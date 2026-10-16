use std::{io::Write, os::unix::fs::symlink};

use super::*;
use crate::{
    receive::ReceivePolicy,
    sessions::details::{DetailsStatus, FileDetail, PayloadKind},
};

pub(crate) fn fixture() -> (tempfile::TempDir, Arc<ReceivedFiles>, TransferDetails) {
    let directory = tempfile::tempdir().unwrap();
    let root = ReceivePolicy::new(Some(directory.path().to_owned()))
        .snapshot()
        .pin()
        .unwrap();
    let path = directory.path().join("nested/actual (1).txt");
    let mut file = root.create_file(&path).unwrap();
    file.write_all(b"received").unwrap();
    let mut pending = PendingFiles::new(root, vec![101], ReceiptBudget::default()).unwrap();
    pending.record(101, &path, &file, 8).unwrap();
    let receipt = pending.finish().unwrap();
    let details = TransferDetails {
        status: DetailsStatus::Ready,
        kind: PayloadKind::Files,
        sender: "phone".into(),
        total_bytes: 8,
        transferred_bytes: 8,
        files: vec![FileDetail {
            name: "nested/actual (1).txt".into(),
            size: 8,
        }]
        .into(),
        ..Default::default()
    };
    (directory, receipt, details)
}

#[test]
fn given_partly_published_manifest_when_freezing_receipt_then_only_recorded_members_and_one_reserved_slot_survive(
) {
    // Arrange
    let directory = tempfile::tempdir().unwrap();
    let root = ReceivePolicy::new(Some(directory.path().to_owned()))
        .snapshot()
        .pin()
        .unwrap();
    let budget = ReceiptBudget::default();
    let mut pending = PendingFiles::new(root.clone(), vec![101, 102], budget.clone()).unwrap();
    assert_eq!(budget.0.load(Ordering::Acquire), 1);
    let path = directory.path().join("b.bin");
    let (mut file, publication) = root.stage_file(&path).unwrap();
    file.write_all(b"done").unwrap();
    publication.publish(&file, 4).unwrap();
    publication.sync().unwrap();
    pending.record(102, &path, &file, 4).unwrap();
    // Act / Assert
    let receipt = pending.finish_partial().unwrap();
    assert_eq!(receipt.len(), 1);
    assert_eq!(receipt.details()[0].name, "b.bin");
    assert!(receipt.open(1).is_err());
    let details = TransferDetails {
        files: vec![
            crate::sessions::details::FileDetail {
                name: "a.bin".into(),
                size: 1,
            },
            crate::sessions::details::FileDetail {
                name: "b.bin".into(),
                size: 4,
            },
        ]
        .into(),
        ..Default::default()
    };
    assert!(receipt.subset_of(&details));
    assert!(!receipt.valid_for(&details));
    let mismatched = TransferDetails {
        files: vec![crate::sessions::details::FileDetail {
            name: "b.bin".into(),
            size: 5,
        }]
        .into(),
        ..details
    };
    assert!(!receipt.subset_of(&mismatched));
    assert_eq!(budget.0.load(Ordering::Acquire), 1);
    drop(receipt);
    assert_eq!(budget.0.load(Ordering::Acquire), 0);
    assert!(PendingFiles::new(root, vec![101], budget.clone())
        .unwrap()
        .finish_partial()
        .is_none());
    assert_eq!(budget.0.load(Ordering::Acquire), 0);
}
#[test]
fn given_actual_completed_records_when_root_moves_then_original_authority_and_manifest_order_are_preserved(
) {
    // Arrange
    let directory = tempfile::tempdir().unwrap();
    let original = directory.path().join("original");
    std::fs::create_dir(&original).unwrap();
    let root = ReceivePolicy::new(Some(original.clone()))
        .snapshot()
        .pin()
        .unwrap();
    let a = original.join("a.txt");
    let b = original.join("nested/b (1).txt");
    let mut first = root.create_file(&a).unwrap();
    first.write_all(b"a").unwrap();
    let second = root.create_file(&b).unwrap();
    let mut pending = PendingFiles::new(root, vec![101, 102], ReceiptBudget::default()).unwrap();
    pending.record(102, &b, &second, 0).unwrap();
    assert!(pending.record(102, &b, &second, 0).is_err());
    pending.record(101, &a, &first, 1).unwrap();
    let receipt = pending.finish().unwrap();
    // Act
    std::fs::rename(&original, directory.path().join("moved")).unwrap();
    std::fs::create_dir(&original).unwrap();
    std::fs::write(original.join("a.txt"), b"replacement").unwrap();
    // Assert
    use std::io::Read;
    let mut bytes = Vec::new();
    receipt.open(0).unwrap().read_to_end(&mut bytes).unwrap();
    assert_eq!(bytes, b"a");
    let file = receipt.open(1).unwrap();
    assert_eq!(file.metadata().unwrap().len(), 0);
    assert_eq!(
        rustix::fs::fcntl_getfl(&file).unwrap() & rustix::fs::OFlags::ACCMODE,
        rustix::fs::OFlags::RDONLY
    );
    assert!(rustix::io::fcntl_getfd(&file)
        .unwrap()
        .contains(rustix::io::FdFlags::CLOEXEC));
    assert!(receipt.open(2).is_err());
    assert!(receipt.open(u32::MAX).is_err());
    assert!(!format!("{receipt:?}").contains("original"));
    assert!(!format!("{receipt:?}").contains("b (1)"));
}
#[test]
fn given_received_target_when_deleted_edited_replaced_or_symlinked_then_no_other_object_is_returned(
) {
    // Arrange / Act / Assert
    for change in [
        "deleted",
        "edited",
        "same-size-edited",
        "replaced",
        "symlink",
        "parent-symlink",
        "fifo",
    ] {
        let (directory, receipt, details) = fixture();
        assert!(receipt.valid_for(&details));
        let path = directory.path().join("nested/actual (1).txt");
        match change {
            "deleted" => std::fs::remove_file(&path).unwrap(),
            "edited" => std::fs::write(&path, b"different-length").unwrap(),
            "same-size-edited" => {
                let modified = std::fs::metadata(&path).unwrap().modified().unwrap();
                std::fs::write(&path, b"modified").unwrap();
                std::fs::OpenOptions::new()
                    .write(true)
                    .open(&path)
                    .unwrap()
                    .set_modified(modified + std::time::Duration::from_secs(1))
                    .unwrap();
            }
            "replaced" => {
                std::fs::rename(&path, directory.path().join("old")).unwrap();
                std::fs::write(&path, b"received").unwrap();
            }
            "symlink" => {
                std::fs::rename(&path, directory.path().join("old")).unwrap();
                symlink(directory.path().join("old"), &path).unwrap();
            }
            "parent-symlink" => {
                std::fs::rename(
                    directory.path().join("nested"),
                    directory.path().join("old"),
                )
                .unwrap();
                symlink(
                    directory.path().join("old"),
                    directory.path().join("nested"),
                )
                .unwrap();
            }
            "fifo" => {
                std::fs::remove_file(&path).unwrap();
                rustix::fs::mknodat(
                    rustix::fs::CWD,
                    &path,
                    rustix::fs::FileType::Fifo,
                    rustix::fs::Mode::from_raw_mode(0o600),
                    0,
                )
                .unwrap();
            }
            _ => unreachable!(),
        }
        assert!(receipt.open(0).is_err(), "{change}");
    }
}
#[test]
fn given_receipt_budget_when_snapshots_outlive_history_then_only_final_drop_releases_the_slot() {
    // Arrange
    let directory = tempfile::tempdir().unwrap();
    let root = ReceivePolicy::new(Some(directory.path().to_owned()))
        .snapshot()
        .pin()
        .unwrap();
    let path = directory.path().join("empty");
    let file = root.create_file(&path).unwrap();
    let budget = ReceiptBudget::default();
    let make = || {
        let mut pending = PendingFiles::new(root.clone(), vec![101], budget.clone())?;
        pending.record(101, &path, &file, 0).unwrap();
        pending.finish()
    };
    // Act / Assert
    let mut receipts: Vec<_> = (0..RECEIPT_LIMIT).map(|_| make().unwrap()).collect();
    assert!(make().is_none());
    let retained = receipts.pop().unwrap();
    let clone = retained.clone();
    drop(retained);
    assert!(make().is_none());
    drop(clone);
    assert!(make().is_some());
    drop(receipts);
    assert_eq!(budget.0.load(Ordering::Acquire), 0);
    assert!(PendingFiles::new(root.clone(), vec![], budget.clone()).is_none());
    assert!(PendingFiles::new(root, vec![101; FILE_LIMIT + 1], budget).is_none());
}
