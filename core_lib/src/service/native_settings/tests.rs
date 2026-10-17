use std::cell::Cell;
use std::os::unix::fs::{symlink, PermissionsExt};

use super::*;

#[test]
fn given_schema_one_when_name_is_explicitly_saved_then_original_bytes_are_backed_up_before_schema_two(
) {
    let f = Fixture::new();
    let first = f.store.save(f.edit(&f.store.load().unwrap(), 1)).unwrap();
    let mut old: serde_json::Value = serde_json::from_slice(&f.bytes()).unwrap();
    old["schema"] = serde_json::json!(1);
    old["values"].as_object_mut().unwrap().remove("device_name");
    let bytes = serde_json::to_vec(&old).unwrap();
    std::fs::write(f.store.directory.join(FILE), &bytes).unwrap();
    let before = f.store.load().unwrap();
    assert_eq!(before.schema, 1);
    assert_eq!(before.base.revision, first.base.revision);
    assert!(before.values.as_ref().unwrap().device_name.is_none());
    assert_eq!(f.bytes(), bytes);
    let mut edit = f.edit(&before, 2);
    edit.device_name = Some(crate::identity::DeviceName::new("Рабочий ПК").unwrap());
    let saved = f.store.save(edit).unwrap();
    assert_eq!(saved.schema, 2);
    assert_eq!(saved.base.revision, 2);
    assert_eq!(
        saved
            .values
            .as_ref()
            .unwrap()
            .device_name
            .as_ref()
            .unwrap()
            .as_str(),
        "Рабочий ПК"
    );
    assert_eq!(
        std::fs::read(
            f.store
                .directory
                .join(format!("backup-{}.json", hash(&[&bytes])))
        )
        .unwrap(),
        bytes
    );
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&f.bytes()).unwrap()["schema"],
        2
    );
    old["values"]["device_name"] = serde_json::Value::Null;
    let injected = serde_json::to_vec(&old).unwrap();
    std::fs::write(f.store.directory.join(FILE), &injected).unwrap();
    assert!(f.store.load().unwrap().values.is_none());
    assert_eq!(f.bytes(), injected);
}

#[test]
fn given_named_save_when_retried_changed_or_reset_then_receipt_binds_name_without_extra_revision() {
    for value in ["Laptop", "Рабочий ПК", &format!("{}a", "é".repeat(118))] {
        let f = Fixture::new();
        let before = f.store.load().unwrap();
        let edit = || {
            let mut edit = f.edit(&before, 1);
            edit.device_name = Some(crate::identity::DeviceName::new(value).unwrap());
            edit
        };
        let saved = f.store.save(edit()).unwrap();
        let bytes = f.bytes();
        assert_eq!(f.store.save(edit()).unwrap().base, saved.base);
        assert_eq!(f.bytes(), bytes);
        let mut changed = edit();
        changed.device_name = None;
        assert!(matches!(
            f.store.save(changed),
            Err(StoreError::ReplayConflict)
        ));
        assert_eq!(f.bytes(), bytes);
        assert_eq!(
            f.store
                .load()
                .unwrap()
                .values
                .unwrap()
                .device_name
                .unwrap()
                .as_str(),
            value
        );
        let reset = f.store.save(f.edit(&saved, 2)).unwrap();
        assert_eq!(reset.base.revision, 2);
        assert!(reset.values.unwrap().device_name.is_none());
    }
}

#[test]
fn given_invalid_persisted_name_when_loaded_then_explicit_recovery_preserves_corrupt_bytes() {
    let f = Fixture::new();
    f.store.save(f.edit(&f.store.load().unwrap(), 1)).unwrap();
    let original: serde_json::Value = serde_json::from_slice(&f.bytes()).unwrap();
    for value in [
        serde_json::json!(""),
        serde_json::json!(" trailing "),
        serde_json::json!("a\n"),
        serde_json::json!("a\u{202e}b"),
        serde_json::json!("a".repeat(238)),
        serde_json::json!("é".repeat(119)),
        serde_json::json!(false),
        serde_json::json!(1),
        serde_json::json!([]),
        serde_json::json!({}),
    ] {
        let mut document = original.clone();
        document["values"]["device_name"] = value;
        let bytes = serde_json::to_vec(&document).unwrap();
        std::fs::write(f.store.directory.join(FILE), &bytes).unwrap();
        let bad = f.store.load().unwrap();
        assert!(bad.values.is_none());
        assert!(matches!(
            f.store.save(f.edit(&bad, 2)),
            Err(StoreError::RecoveryRequired)
        ));
        assert_eq!(f.bytes(), bytes);
    }
}

struct Fixture {
    root: tempfile::TempDir,
    store: Store,
}
impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let store = Store::new(root.path().join("native"), root.path().join("legacy"));
        Self { root, store }
    }
    fn edit(&self, snapshot: &Snapshot, request: u32) -> Edit {
        Edit {
            base: snapshot.base.clone(),
            request: format!("{request:032x}"),
            visibility: 1,
            port: Some(0),
            card_mode: CardMode::Full,
            device_name: None,
            directory: DirectoryIntent::Default,
            recover: false,
        }
    }
    fn bytes(&self) -> Vec<u8> {
        std::fs::read(self.store.directory.join(FILE)).unwrap()
    }
    fn private_native(&self, bytes: &[u8]) {
        std::fs::create_dir(&self.store.directory).unwrap();
        std::fs::set_permissions(
            &self.store.directory,
            std::fs::Permissions::from_mode(0o700),
        )
        .unwrap();
        std::fs::write(self.store.directory.join(FILE), bytes).unwrap();
        std::fs::set_permissions(
            self.store.directory.join(FILE),
            std::fs::Permissions::from_mode(0o600),
        )
        .unwrap();
    }
}

#[test]
fn given_missing_or_legacy_source_when_explicitly_saved_then_native_revision_and_exact_backup_preserve_original(
) {
    for legacy in [
        None,
        Some(br#"{"visibility":2,"port":1234,"unknown":{"secret":"preserve"}}"#.as_slice()),
        Some(br#"{"visibility":"bad"}"#.as_slice()),
    ] {
        // Arrange
        let f = Fixture::new();
        if let Some(bytes) = legacy {
            std::fs::write(&f.store.legacy, bytes).unwrap();
        }
        let before = f.store.load().unwrap();
        assert!(!f.store.directory.exists()); // Reads never create state.
        assert_eq!(before.base.revision, 0);
        assert_eq!(
            before.source,
            if legacy.is_some() {
                Source::Legacy
            } else {
                Source::Defaults
            }
        );
        assert_eq!(
            before.visibility_recovered,
            legacy.is_some_and(|bytes| bytes.windows(3).any(|s| s == b"bad"))
        );
        // Act
        let saved = f.store.save(f.edit(&before, 1)).unwrap();
        // Assert
        assert_eq!(saved.base.revision, 1);
        assert_eq!(saved.source, Source::Native);
        assert!(!saved.visibility_recovered);
        assert_eq!(saved.values.as_ref().unwrap().visibility, 1);
        assert_eq!(saved.values.as_ref().unwrap().port, Some(0));
        assert_eq!(saved.values.as_ref().unwrap().card_mode, CardMode::Full);
        assert_eq!(saved.base, f.store.load().unwrap().base);
        assert_eq!(
            std::fs::metadata(&f.store.directory).unwrap().mode() & 0o777,
            0o700
        );
        assert_eq!(
            std::fs::metadata(f.store.directory.join(FILE))
                .unwrap()
                .mode()
                & 0o777,
            0o600
        );
        if let Some(bytes) = legacy {
            assert_eq!(std::fs::read(&f.store.legacy).unwrap(), bytes);
            assert_eq!(
                std::fs::read(
                    f.store
                        .directory
                        .join(format!("backup-{}.json", hash(&[bytes])))
                )
                .unwrap(),
                bytes
            );
        }
    }
}

#[test]
fn given_commit_receipt_when_retry_conflict_or_external_edit_occurs_then_only_exact_latest_intent_is_acknowledged(
) {
    // Arrange
    let f = Fixture::new();
    let before = f.store.load().unwrap();
    let saved = f.store.save(f.edit(&before, 1)).unwrap();
    let bytes = f.bytes();
    // Act / Assert
    assert_eq!(f.store.save(f.edit(&before, 1)).unwrap().base, saved.base);
    assert_eq!(f.bytes(), bytes);
    let mut changed = f.edit(&before, 1);
    changed.visibility = 0;
    assert!(matches!(
        f.store.save(changed),
        Err(StoreError::ReplayConflict)
    ));
    assert!(matches!(
        f.store.save(f.edit(&before, 2)),
        Err(StoreError::Conflict)
    ));
    let newer = f.store.save(f.edit(&saved, 2)).unwrap();
    assert_eq!(newer.base.revision, 2);
    assert!(matches!(
        f.store.save(f.edit(&before, 1)),
        Err(StoreError::Conflict)
    ));
    std::fs::write(f.store.directory.join(FILE), b"externally changed").unwrap();
    assert!(matches!(
        f.store.save(f.edit(&newer, 3)),
        Err(StoreError::Conflict)
    ));
    assert_eq!(f.bytes(), b"externally changed");

    let legacy = Fixture::new();
    std::fs::write(&legacy.store.legacy, b"{}").unwrap();
    let old = legacy.store.load().unwrap();
    std::fs::write(&legacy.store.legacy, br#"{"visibility":1}"#).unwrap();
    assert!(matches!(
        legacy.store.save(legacy.edit(&old, 1)),
        Err(StoreError::Conflict)
    ));
    assert!(!legacy.store.directory.join(FILE).exists());
}

#[test]
fn given_corrupt_schema_or_values_when_recovered_then_explicit_replacement_backs_up_original_and_rejects_unsafe_inputs(
) {
    let valid = Fixture::new();
    valid
        .store
        .save(valid.edit(&valid.store.load().unwrap(), 1))
        .unwrap();
    let document: serde_json::Value = serde_json::from_slice(&valid.bytes()).unwrap();
    let mut cases = vec![b"{".to_vec(), b"null".to_vec(), b"{}".to_vec()];
    for pointer in [
        "/schema",
        "/revision",
        "/values/visibility",
        "/values/port",
        "/values/card_mode",
        "/receipt/request",
        "/receipt/fingerprint",
    ] {
        let mut changed = document.clone();
        *changed.pointer_mut(pointer).unwrap() = serde_json::json!("invalid");
        cases.push(serde_json::to_vec(&changed).unwrap());
    }
    for (pointer, value) in [
        ("/schema", serde_json::json!(3)),
        ("/revision", serde_json::json!(0)),
        ("/values/visibility", serde_json::json!(3)),
        ("/values/port", serde_json::json!(65536)),
    ] {
        let mut changed = document.clone();
        *changed.pointer_mut(pointer).unwrap() = value;
        cases.push(serde_json::to_vec(&changed).unwrap());
    }
    let mut unknown = document;
    unknown["surprise"] = serde_json::json!(true);
    cases.push(serde_json::to_vec(&unknown).unwrap());
    for bytes in cases {
        // Arrange
        let f = Fixture::new();
        f.private_native(&bytes);
        let corrupt = f.store.load().unwrap();
        assert!(corrupt.values.is_none());
        // Act / Assert
        assert!(matches!(
            f.store.save(f.edit(&corrupt, 2)),
            Err(StoreError::RecoveryRequired)
        ));
        assert_eq!(f.bytes(), bytes);
        let mut edit = f.edit(&corrupt, 2);
        edit.recover = true;
        let saved = f.store.save(edit).unwrap();
        assert_eq!(saved.base.revision, 1);
        assert_eq!(
            std::fs::read(
                f.store
                    .directory
                    .join(format!("backup-{}.json", hash(&[&bytes])))
            )
            .unwrap(),
            bytes
        );
        for invalid in [
            "",
            "x",
            "ABCDEF01234567890123456789012345",
            "0000000000000000000000000000000g",
        ] {
            let mut edit = f.edit(&saved, 3);
            edit.request = invalid.into();
            assert!(matches!(f.store.save(edit), Err(StoreError::InvalidInput)));
        }
        let mut edit = f.edit(&saved, 3);
        edit.visibility = 3;
        assert!(matches!(f.store.save(edit), Err(StoreError::InvalidInput)));
    }
    let f = Fixture::new();
    f.private_native(&vec![b' '; MAX_SETTINGS_BYTES + 1]);
    assert!(matches!(f.store.load(), Err(StoreError::Capacity)));
    let f = Fixture::new();
    f.private_native(&vec![b' '; MAX_SETTINGS_BYTES]);
    assert!(f.store.load().unwrap().values.is_none());
}

#[test]
fn given_non_utf8_directory_capability_when_saved_retained_or_replaced_then_identity_and_bytes_remain_authoritative(
) {
    // Arrange
    let f = Fixture::new();
    let target = f
        .root
        .path()
        .join(std::ffi::OsString::from_vec(vec![b'd', 0xff]));
    std::fs::create_dir(&target).unwrap();
    let file = File::open(&target).unwrap();
    let before = f.store.load().unwrap();
    let mut edit = f.edit(&before, 1);
    edit.directory = DirectoryIntent::Replace(file.try_clone().unwrap());
    // Act
    let saved = f.store.save(edit).unwrap();
    let record = saved.values.as_ref().unwrap().directory.as_ref().unwrap();
    // Assert
    assert_eq!(record.path(), target);
    assert_eq!(
        record.open().unwrap().metadata().unwrap().ino(),
        file.metadata().unwrap().ino()
    );
    let moved = f.root.path().join("moved");
    std::fs::rename(&target, &moved).unwrap();
    let mut retry = f.edit(&before, 1);
    retry.directory = DirectoryIntent::Replace(file);
    assert_eq!(f.store.save(retry).unwrap().base, saved.base); // Fingerprint is FD identity, not renamed display path.
    std::fs::create_dir(&target).unwrap();
    assert!(matches!(record.open(), Err(StoreError::Directory)));
    let mut edit = f.edit(&saved, 2);
    edit.directory = DirectoryIntent::Retain;
    let retained = f.store.save(edit).unwrap();
    assert_eq!(
        retained.values.as_ref().unwrap().directory.as_ref(),
        Some(record)
    );
    let mut bad = f.edit(&retained, 3);
    bad.directory = DirectoryIntent::Replace(File::open(f.store.directory.join(FILE)).unwrap());
    assert!(matches!(f.store.save(bad), Err(StoreError::Directory)));
    assert!(!format!("{record:?}").contains("moved"));
}

#[test]
fn given_unsafe_storage_objects_when_loaded_then_no_symlink_hardlink_fifo_or_public_file_is_trusted(
) {
    for variant in 0..6 {
        // Arrange
        let f = Fixture::new();
        f.private_native(b"{}");
        let path = f.store.directory.join(FILE);
        match variant {
            0 => {
                std::fs::set_permissions(&f.store.directory, std::fs::Permissions::from_mode(0o755))
                    .unwrap()
            }
            1 => std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap(),
            2 => {
                std::fs::hard_link(&path, f.root.path().join("other")).unwrap();
            }
            3 => {
                std::fs::remove_file(&path).unwrap();
                symlink(f.root.path().join("absent"), &path).unwrap();
            }
            4 => {
                std::fs::remove_file(&path).unwrap();
                rustix::fs::mknodat(
                    rustix::fs::CWD,
                    &path,
                    rustix::fs::FileType::Fifo,
                    Mode::from_raw_mode(0o600),
                    0,
                )
                .unwrap();
            }
            5 => {
                std::fs::remove_file(&path).unwrap();
                std::fs::create_dir(&path).unwrap();
            }
            _ => unreachable!(),
        }
        // Act / Assert (FIFO open is nonblocking).
        assert!(matches!(f.store.load(), Err(StoreError::UnsafeStorage)));
    }
    let f = Fixture::new();
    symlink(f.root.path(), &f.store.directory).unwrap();
    assert!(matches!(f.store.load(), Err(StoreError::UnsafeStorage)));
    let f = Fixture::new();
    symlink(f.root.path().join("absent"), &f.store.legacy).unwrap();
    assert!(matches!(f.store.load(), Err(StoreError::Read)));
}

#[test]
fn given_backup_quota_or_corrupt_existing_backup_when_imported_then_original_is_not_overwritten() {
    for quota in [false, true] {
        // Arrange
        let f = Fixture::new();
        std::fs::write(&f.store.legacy, b"{}").unwrap();
        let before = f.store.load().unwrap();
        let dir = f.store.open_directory(true).unwrap().unwrap();
        if quota {
            for index in 0..BACKUP_LIMIT {
                f.store
                    .replace(&dir, &format!("backup-{index}.json"), b"prior")
                    .unwrap();
            }
        } else {
            f.store
                .replace(
                    &dir,
                    &format!("backup-{}.json", hash(&[b"{}"])),
                    b"different",
                )
                .unwrap();
        }
        // Act / Assert
        assert!(matches!(
            f.store.save(f.edit(&before, 1)),
            Err(StoreError::BackupCapacity | StoreError::UnsafeStorage)
        ));
        assert_eq!(std::fs::read(&f.store.legacy).unwrap(), b"{}");
        assert!(!f.store.directory.join(FILE).exists());
    }
}

struct FaultIo {
    fail: Cell<Option<u8>>,
}

struct SourceEditingIo(PathBuf);
impl PersistenceIo for SourceEditingIo {
    fn write(&self, file: &mut File, bytes: &[u8]) -> std::io::Result<()> {
        SystemIo.write(file, bytes)
    }
    fn sync_file(&self, file: &File) -> std::io::Result<()> {
        SystemIo.sync_file(file)
    }
    fn rename(&self, directory: &File, from: &str, to: &str) -> std::io::Result<()> {
        SystemIo.rename(directory, from, to)?;
        if to.starts_with("backup-") {
            std::fs::write(&self.0, br#"{"visibility":2}"#)?;
        }
        Ok(())
    }
    fn sync_directory(&self, directory: &File) -> std::io::Result<()> {
        SystemIo.sync_directory(directory)
    }
}

#[test]
fn given_legacy_source_when_changed_during_backup_then_final_cas_rejects_import_without_native_commit(
) {
    // Arrange
    let f = Fixture::new();
    std::fs::write(&f.store.legacy, b"{}").unwrap();
    let before = f.store.load().unwrap();
    let store = Store::with_io(
        f.store.directory.clone(),
        f.store.legacy.clone(),
        SourceEditingIo(f.store.legacy.clone()),
    );
    // Act / Assert
    assert!(matches!(
        store.save(f.edit(&before, 1)),
        Err(StoreError::Conflict)
    ));
    assert!(!store.directory.join(FILE).exists());
    assert_eq!(
        std::fs::read(&store.legacy).unwrap(),
        br#"{"visibility":2}"#
    );
    assert_eq!(
        std::fs::read(
            store
                .directory
                .join(format!("backup-{}.json", hash(&[b"{}"])))
        )
        .unwrap(),
        b"{}"
    );
}
impl FaultIo {
    fn step(&self, step: u8) -> std::io::Result<()> {
        if self.fail.get() == Some(step) {
            Err(std::io::ErrorKind::StorageFull.into())
        } else {
            Ok(())
        }
    }
}
impl PersistenceIo for FaultIo {
    fn write(&self, file: &mut File, bytes: &[u8]) -> std::io::Result<()> {
        self.step(0)?;
        SystemIo.write(file, bytes)
    }
    fn sync_file(&self, file: &File) -> std::io::Result<()> {
        self.step(1)?;
        SystemIo.sync_file(file)
    }
    fn rename(&self, directory: &File, from: &str, to: &str) -> std::io::Result<()> {
        self.step(2)?;
        SystemIo.rename(directory, from, to)
    }
    fn sync_directory(&self, directory: &File) -> std::io::Result<()> {
        self.step(3)?;
        SystemIo.sync_directory(directory)
    }
}
#[test]
fn given_write_sync_or_rename_failure_when_committing_then_precommit_preserves_old_bytes_and_postcommit_requires_reconciliation(
) {
    for step in 0..4 {
        // Arrange
        let f = Fixture::new();
        let mut initial = f.edit(&f.store.load().unwrap(), 1);
        initial.device_name = Some(crate::identity::DeviceName::new("Before").unwrap());
        let first = f.store.save(initial).unwrap();
        let named_edit = || {
            let mut edit = f.edit(&first, 2);
            edit.device_name = Some(crate::identity::DeviceName::new("After").unwrap());
            edit
        };
        let old = f.bytes();
        let store = Store::with_io(
            f.store.directory.clone(),
            f.store.legacy.clone(),
            FaultIo {
                fail: Cell::new(Some(step)),
            },
        );
        // Act
        let result = store.save(named_edit());
        // Assert
        if step < 3 {
            assert!(matches!(result, Err(StoreError::Write)));
            assert_eq!(f.bytes(), old);
            assert_eq!(store.load().unwrap().base, first.base);
        } else {
            assert!(matches!(result, Err(StoreError::Indeterminate)));
            assert_ne!(f.bytes(), old);
            assert_eq!(store.load().unwrap().base.revision, 2);
            assert!(matches!(
                store.save(named_edit()),
                Err(StoreError::Indeterminate)
            ));
        }
        assert_eq!(
            store
                .load()
                .unwrap()
                .values
                .unwrap()
                .device_name
                .unwrap()
                .as_str(),
            if step < 3 { "Before" } else { "After" }
        );
        store.io.fail.set(None);
        assert_eq!(store.save(named_edit()).unwrap().base.revision, 2);
        assert_eq!(
            std::fs::read_dir(&store.directory)
                .unwrap()
                .filter_map(|entry| entry.ok())
                .filter(|entry| entry.file_name().as_bytes().starts_with(b".pending-"))
                .count(),
            0
        );
    }
}

#[test]
fn given_visibility_partial_edit_when_saved_replayed_or_competing_then_other_fields_and_cas_remain_intact(
) {
    for mode in 0..=2 {
        // Arrange
        let f = Fixture::new();
        let root = tempfile::tempdir().unwrap();
        let mut initial = f.edit(&f.store.load().unwrap(), 1);
        initial.directory = DirectoryIntent::Replace(File::open(root.path()).unwrap());
        initial.device_name = Some(crate::identity::DeviceName::new("Retained").unwrap());
        let before = f.store.save(initial).unwrap();
        let request = format!("{:032x}", 2);
        // Act
        let (saved, fresh) = f
            .store
            .save_visibility(before.base.clone(), request.clone(), mode)
            .unwrap();
        let bytes = f.bytes();
        let (replayed, fresh_retry) = f
            .store
            .save_visibility(before.base.clone(), request.clone(), mode)
            .unwrap();
        // Assert
        assert!(fresh);
        assert!(!fresh_retry);
        assert_eq!(replayed.base, saved.base);
        assert_eq!(f.bytes(), bytes);
        let mut expected = before.values.clone().unwrap();
        expected.visibility = mode;
        assert_eq!(saved.values, Some(expected));
        assert!(matches!(
            f.store
                .save_visibility(before.base.clone(), request.clone(), (mode + 1) % 3),
            Err(StoreError::ReplayConflict)
        ));
        let next = f.store.save(f.edit(&saved, 3)).unwrap();
        assert!(matches!(
            f.store.save_visibility(before.base, request, mode),
            Err(StoreError::Conflict)
        ));
        assert_eq!(f.store.load().unwrap().base, next.base);
    }
}

#[test]
fn given_invalid_or_corrupt_visibility_intent_when_saved_then_original_settings_are_preserved() {
    // Arrange
    let f = Fixture::new();
    let before = f.store.load().unwrap();
    // Act / Assert
    for mode in [3, u8::MAX] {
        assert!(matches!(
            f.store
                .save_visibility(before.base.clone(), format!("{:032x}", 1), mode),
            Err(StoreError::InvalidInput)
        ));
    }
    for bad in ["", "abc", &"g".repeat(32), &"A".repeat(32), &"0".repeat(33)] {
        assert!(matches!(
            f.store.save_visibility(before.base.clone(), bad.into(), 1),
            Err(StoreError::InvalidInput)
        ));
    }
    let mut bad = before.base.clone();
    bad.token = "x".repeat(64);
    assert!(matches!(
        f.store.save_visibility(bad, format!("{:032x}", 1), 1),
        Err(StoreError::InvalidInput)
    ));
    assert!(!f.store.directory.join(FILE).exists());
    let saved = f.store.save(f.edit(&before, 1)).unwrap();
    std::fs::write(f.store.directory.join(FILE), b"corrupt").unwrap();
    assert!(matches!(
        f.store
            .save_visibility(saved.base, format!("{:032x}", 2), 0),
        Err(StoreError::RecoveryRequired)
    ));
    assert_eq!(f.bytes(), b"corrupt");
}

#[test]
fn given_visibility_disk_failure_when_saved_then_precommit_is_unchanged_and_uncertain_replay_is_not_fresh(
) {
    for step in 0..4 {
        // Arrange
        let f = Fixture::new();
        let before = f.store.save(f.edit(&f.store.load().unwrap(), 1)).unwrap();
        let bytes = f.bytes();
        let store = Store::with_io(
            f.store.directory.clone(),
            f.store.legacy.clone(),
            FaultIo {
                fail: Cell::new(Some(step)),
            },
        );
        let edit = || store.save_visibility(before.base.clone(), format!("{:032x}", 2), 2);
        // Act / Assert
        let result = edit();
        if step < 3 {
            assert!(matches!(result, Err(StoreError::Write)));
            assert_eq!(f.bytes(), bytes);
        } else {
            assert!(matches!(result, Err(StoreError::Indeterminate)));
            assert_eq!(store.load().unwrap().values.unwrap().visibility, 2);
            store.io.fail.set(None);
            let (recovered, fresh) = edit().unwrap();
            assert!(!fresh);
            assert_eq!(recovered.base.revision, before.base.revision + 1);
        }
    }
}
