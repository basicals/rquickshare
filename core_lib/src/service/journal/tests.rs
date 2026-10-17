use std::{
    cell::Cell,
    fs,
    os::unix::fs::{symlink, PermissionsExt},
};

use super::*;
fn record(id: u64, status: &str) -> Record {
    Record {
        id: format!("{:032x}-{id:016x}", 7),
        peer: "Private advertised peer".into(),
        inbound: true,
        status: status.into(),
        kind: "files".into(),
        files: 1,
        total: 100,
        transferred: 40,
        started: 0,
        updated: 0,
    }
}
fn store() -> (tempfile::TempDir, Store) {
    let dir = tempfile::tempdir().unwrap();
    fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let store = Store::open(dir.path().join("state"), SystemIo, 100);
    (dir, store)
}
#[test]
fn given_active_and_terminal_records_when_reopened_then_only_unfinished_work_becomes_interrupted() {
    // Arrange
    let (dir, mut s) = store();
    s.observe(vec![record(1, "active"), record(2, "finished")], 100);
    // Act
    let reopened = Store::open(dir.path().join("state"), SystemIo, 120);
    // Assert
    assert_eq!(reopened.error, 0);
    assert_eq!(reopened.doc.records.len(), 2);
    assert_eq!(reopened.doc.records[0].status, "interrupted");
    assert_eq!(reopened.doc.records[1].status, "finished");
    assert_eq!(reopened.doc.records[0].started, 100);
    assert_eq!(reopened.doc.records[0].updated, 120);
}
#[test]
fn given_retention_limits_when_records_age_or_overflow_then_boundaries_and_backward_time_are_exact()
{
    for count in [0, 1, 999, 1000, 1001] {
        // Arrange / Act
        let (_dir, mut s) = store();
        s.observe((0..count).map(|id| record(id, "finished")).collect(), 100);
        // Assert
        assert_eq!(s.doc.records.len(), (count as usize).min(COMPLETED));
        if count == 1001 {
            assert_eq!(s.doc.records[0].id, record(1, "finished").id);
        }
        assert!(!s.prune(99));
        assert!(!s.prune(100 + MAX_AGE));
        s.prune(101 + MAX_AGE);
        assert!(s.doc.records.is_empty());
    }
}
#[test]
fn given_clear_retry_when_newer_completion_arrives_then_retry_preserves_new_work_and_active_records(
) {
    // Arrange
    let (_dir, mut s) = store();
    s.observe(vec![record(1, "finished"), record(2, "active")], 100);
    let token = s.view().token;
    s.clear(&token, 101).unwrap();
    assert_eq!(
        s.doc.records,
        vec![Record {
            started: 100,
            updated: 100,
            ..record(2, "active")
        }]
    );
    s.observe(vec![record(3, "finished")], 102);
    // Act
    s.clear(&token, 103).unwrap();
    // Assert
    assert_eq!(s.doc.records.len(), 2);
    assert!(s
        .doc
        .records
        .iter()
        .any(|r| r.id == record(3, "finished").id));
    assert_eq!(s.clear(&"a".repeat(64), 104), Err(StoreError::Conflict));
}
#[test]
fn given_corrupt_or_future_document_when_cleared_then_exact_original_is_preserved_before_reset() {
    for raw in [
        b"broken-json".as_slice(),
        br#"{"schema":99,"last_clear":"","records":[]}"#,
    ] {
        // Arrange
        let dir = tempfile::tempdir().unwrap();
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let file = dir.path().join(FILE);
        fs::write(&file, raw).unwrap();
        fs::set_permissions(&file, fs::Permissions::from_mode(0o600)).unwrap();
        let mut s = Store::open(dir.path().into(), SystemIo, 100);
        assert_eq!(s.error, 1);
        let token = s.view().token;
        // Act
        s.clear(&token, 101).unwrap();
        // Assert
        assert_eq!(
            fs::read(
                dir.path()
                    .join(format!("native-transfers-recovery-{token}.json"))
            )
            .unwrap(),
            raw
        );
        let reloaded = Store::open(dir.path().into(), SystemIo, 102);
        assert_eq!(reloaded.error, 0);
        assert!(reloaded.doc.records.is_empty());
    }
}
#[test]
fn given_unsafe_file_types_permissions_or_links_when_loaded_then_original_is_never_replaced() {
    for case in [
        "permissions",
        "symlink",
        "hardlink",
        "directory",
        "oversized",
    ] {
        // Arrange
        let dir = tempfile::tempdir().unwrap();
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let file = dir.path().join(FILE);
        let original = dir.path().join("original");
        fs::write(&original, b"preserve").unwrap();
        match case {
            "symlink" => symlink(&original, &file).unwrap(),
            "hardlink" => fs::hard_link(&original, &file).unwrap(),
            "directory" => fs::create_dir(&file).unwrap(),
            "oversized" => {
                fs::write(&file, vec![0; MAX_BYTES + 1]).unwrap();
                fs::set_permissions(&file, fs::Permissions::from_mode(0o600)).unwrap();
            }
            _ => {
                fs::write(&file, b"preserve").unwrap();
                fs::set_permissions(&file, fs::Permissions::from_mode(0o644)).unwrap();
            }
        }
        // Act / Assert
        let mut s = Store::open(dir.path().into(), SystemIo, 100);
        assert_ne!(s.error, 0);
        assert!(s.clear(&s.view().token, 101).is_err());
        assert_eq!(fs::read(original).unwrap(), b"preserve");
    }
}
struct FaultIo(Cell<Option<&'static str>>);
impl PersistenceIo for FaultIo {
    fn write(&self, f: &mut std::fs::File, b: &[u8]) -> std::io::Result<()> {
        if self.0.get() == Some("write") {
            return Err(std::io::Error::other("fixture"));
        }
        SystemIo.write(f, b)
    }
    fn sync_file(&self, f: &std::fs::File) -> std::io::Result<()> {
        if self.0.get() == Some("file") {
            return Err(std::io::Error::other("fixture"));
        }
        SystemIo.sync_file(f)
    }
    fn rename(&self, d: &std::fs::File, a: &str, b: &str) -> std::io::Result<()> {
        if self.0.get() == Some("rename") {
            return Err(std::io::Error::other("fixture"));
        }
        SystemIo.rename(d, a, b)
    }
    fn sync_directory(&self, d: &std::fs::File) -> std::io::Result<()> {
        if self.0.get() == Some("directory") {
            return Err(std::io::Error::other("fixture"));
        }
        SystemIo.sync_directory(d)
    }
}
#[test]
fn given_each_filesystem_failure_when_retried_then_pending_metadata_is_not_falsely_acknowledged_or_lost(
) {
    for fault in ["write", "file", "rename", "directory"] {
        // Arrange
        let dir = tempfile::tempdir().unwrap();
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let mut s = Store::open(
            dir.path().join("state"),
            FaultIo(Cell::new(Some(fault))),
            100,
        );
        // Act
        s.observe(vec![record(1, "finished")], 100);
        assert_eq!(s.error, 1);
        assert_eq!(s.doc.records.len(), 1);
        s.io.0.set(None);
        s.persist().unwrap();
        // Assert
        let read = Store::open(dir.path().join("state"), SystemIo, 101);
        assert_eq!(read.error, 0);
        assert_eq!(read.doc.records, s.doc.records);
    }
}
#[test]
fn given_records_when_validated_or_debugged_then_bounds_are_strict_and_private_labels_are_redacted()
{
    let row = record(1, "finished");
    assert!(row.valid());
    for case in ["id", "peer", "control", "status", "kind", "files", "time"] {
        let mut r = row.clone();
        match case {
            "id" => r.id = "X".repeat(49),
            "peer" => r.peer = "x".repeat(257),
            "control" => r.peer = "bad\nlabel".into(),
            "status" => r.status = "accepted-automatically".into(),
            "kind" => r.kind = "payload".into(),
            "files" => r.files = 257,
            _ => {
                r.started = 2;
                r.updated = 1;
            }
        };
        assert!(!r.valid(), "{case}");
    }
    assert!(!format!("{row:?}").contains(&row.peer));
    assert!(!format!("{row:?}").contains(&row.id));
    let keys = serde_json::to_value(row)
        .unwrap()
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();
    assert_eq!(
        keys,
        [
            "id",
            "peer",
            "inbound",
            "status",
            "kind",
            "files",
            "total",
            "transferred",
            "started",
            "updated"
        ]
        .map(String::from)
        .into_iter()
        .collect()
    );
}
#[tokio::test]
async fn given_queued_metadata_when_owner_closes_then_all_owned_io_is_joined_and_records_survive() {
    // Arrange
    let dir = tempfile::tempdir().unwrap();
    fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let owner = Owner::with_clock(dir.path().join("state"), || 100);
    for id in 0..10 {
        owner
            .sender
            .send(Command::Observe(vec![record(id, "finished")]))
            .await
            .unwrap();
    }
    // Act
    owner.close().await;
    // Assert
    let saved = Store::open(dir.path().join("state"), SystemIo, 100);
    assert_eq!(saved.error, 0);
    assert_eq!(saved.doc.records.len(), 10);
}

#[test]
fn given_failed_write_when_passively_read_then_only_explicit_retry_persists_pending_metadata() {
    // Arrange
    let directory = tempfile::tempdir().unwrap();
    fs::set_permissions(directory.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let mut store = Store::open(
        directory.path().join("state"),
        FaultIo(Cell::new(Some("write"))),
        100,
    );
    store.observe(vec![record(1, "finished")], 100);
    assert_eq!(store.error, 1);
    store.io.0.set(None);
    // Act / Assert: an implicit write would clear the error and create the file.
    let passive = store.refresh(101, false);
    assert_eq!(passive.error, 1);
    assert_eq!(passive.records.len(), 1);
    assert!(!directory.path().join(FILE).exists());
    let retried = store.refresh(101, true);
    assert_eq!(retried.error, 0);
    assert_eq!(
        Store::open(directory.path().join("state"), SystemIo, 101)
            .view()
            .records,
        retried.records
    );
}
