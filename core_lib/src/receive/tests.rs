use super::*;

#[test]
fn given_independent_owners_when_policy_changes_then_admitted_directory_snapshots_do_not_change() {
    // Arrange
    let a = tempfile::tempdir().unwrap();
    let b = tempfile::tempdir().unwrap();
    let owner = ReceivePolicy::new(Some(a.path().to_owned()));
    let separate = ReceivePolicy::new(Some(b.path().to_owned()));
    let admitted = owner.snapshot();
    // Actual RQS constructors and setters share no process-global configuration.
    let first = crate::RQS::new(
        crate::Visibility::Invisible,
        None,
        Some(a.path().to_owned()),
    );
    let second = crate::RQS::new(
        crate::Visibility::Invisible,
        None,
        Some(b.path().to_owned()),
    );
    assert_eq!(
        first.receive_policy.snapshot().pin().unwrap().display(),
        a.path()
    );
    second.set_download_path(Some(a.path().to_owned()));
    assert_eq!(
        first.receive_policy.snapshot().pin().unwrap().display(),
        a.path()
    );
    first.set_download_path(None);
    assert_eq!(
        first.receive_policy.snapshot().path,
        crate::utils::get_download_dir()
    );
    // Act / Assert
    owner.set(Some(b.path().to_owned()));
    let old = admitted.pin().unwrap();
    let next = owner.snapshot().pin().unwrap();
    assert_eq!(old.display(), a.path());
    assert_eq!(next.display(), b.path());
    separate.set(Some(a.path().to_owned()));
    assert_eq!(owner.snapshot().pin().unwrap().display(), b.path());
    assert_eq!(separate.snapshot().pin().unwrap().display(), a.path());
    assert!(!format!("{owner:?} {old:?} {admitted:?}").contains(a.path().to_str().unwrap()));
    assert!(ReceivePolicy::new(Some(a.path().join("missing")))
        .snapshot()
        .pin()
        .is_err());
    let regular_file = tempfile::NamedTempFile::new().unwrap();
    assert!(ReceivePolicy::new(Some(regular_file.path().to_owned()))
        .snapshot()
        .pin()
        .is_err());
}

#[cfg(target_os = "linux")]
#[test]
fn given_pinned_root_when_renamed_or_children_are_replaced_then_descriptor_creation_stays_confined_and_exclusive(
) {
    use std::{io::Write, os::unix::fs::symlink};
    // Arrange
    let base = tempfile::tempdir().unwrap();
    let selected = base.path().join("selected");
    let moved = base.path().join("moved");
    let outside = base.path().join("outside");
    std::fs::create_dir(&selected).unwrap();
    std::fs::create_dir(&outside).unwrap();
    let root = ReceivePolicy::new(Some(selected.clone()))
        .snapshot()
        .pin()
        .unwrap();
    // Act
    std::fs::rename(&selected, &moved).unwrap();
    symlink(&outside, &selected).unwrap();
    root.create_file(&selected.join("folder/file"))
        .unwrap()
        .write_all(b"owned")
        .unwrap();
    // Assert
    assert_eq!(std::fs::read(moved.join("folder/file")).unwrap(), b"owned");
    assert!(std::fs::read_dir(&outside).unwrap().next().is_none());
    assert!(root.create_file(&selected.join("folder/file")).is_err());
    assert_eq!(std::fs::read(moved.join("folder/file")).unwrap(), b"owned");
    symlink(&outside, moved.join("escape")).unwrap();
    assert!(root
        .create_file(&selected.join("escape/forbidden"))
        .is_err());
    for relative in ["", ".", "..", "../escape", "folder/../escape", "/absolute"] {
        assert!(
            root.create_file(&selected.join(relative)).is_err(),
            "{relative}"
        );
    }
    assert!(std::fs::read_dir(&outside).unwrap().next().is_none());
}
