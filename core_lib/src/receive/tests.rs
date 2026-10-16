use super::*;

#[cfg(target_os = "linux")]
#[test]
fn given_unpublished_receive_when_zero_or_partial_content_is_dropped_then_no_final_or_temporary_name_remains(
) {
    use std::io::Write;
    for body in [b"".as_slice(), b"partial"] {
        // Arrange
        let directory = tempfile::tempdir().unwrap();
        let root = ReceivePolicy::new(Some(directory.path().to_owned()))
            .snapshot()
            .pin()
            .unwrap();
        let target = directory.path().join("incoming.bin");
        let (mut file, publication) = root.stage_file(&target).unwrap();
        // Act / Assert
        file.write_all(body).unwrap();
        assert!(std::fs::read_dir(directory.path())
            .unwrap()
            .next()
            .is_none());
        drop(publication);
        drop(file);
        assert!(std::fs::read_dir(directory.path())
            .unwrap()
            .next()
            .is_none());
    }
}

#[cfg(target_os = "linux")]
#[test]
fn given_completed_receive_when_published_after_root_move_then_exact_owned_content_is_visible_once()
{
    use std::{io::Write, os::unix::fs::symlink};
    for body in [b"".as_slice(), b"complete"] {
        // Arrange
        let base = tempfile::tempdir().unwrap();
        let original = base.path().join("original");
        let moved = base.path().join("moved");
        let outside = base.path().join("outside");
        std::fs::create_dir(&original).unwrap();
        std::fs::create_dir(&outside).unwrap();
        let root = ReceivePolicy::new(Some(original.clone()))
            .snapshot()
            .pin()
            .unwrap();
        let (mut file, mut publication) = root.stage_file(&original.join("file")).unwrap();
        file.write_all(body).unwrap();
        std::fs::rename(&original, &moved).unwrap();
        symlink(&outside, &original).unwrap();
        // Act / Assert
        assert!(publication.publish(&file, body.len() as u64 + 1).is_err());
        assert!(!moved.join("file").exists());
        publication.publish(&file, body.len() as u64).unwrap();
        publication.sync().unwrap();
        assert_eq!(std::fs::read(moved.join("file")).unwrap(), body);
        assert!(publication.publish(&file, body.len() as u64).is_err());
        // Real fsync failure after linking must never remove the already complete file.
        publication.parent = File::from(
            rustix::fs::open(
                &moved,
                rustix::fs::OFlags::PATH
                    | rustix::fs::OFlags::DIRECTORY
                    | rustix::fs::OFlags::CLOEXEC,
                rustix::fs::Mode::empty(),
            )
            .unwrap(),
        );
        assert!(publication.sync().is_err());
        assert_eq!(std::fs::read(moved.join("file")).unwrap(), body);
        drop(file);
        drop(publication);
        assert_eq!(std::fs::read(moved.join("file")).unwrap(), body);
        assert!(std::fs::read_dir(outside).unwrap().next().is_none());
    }
}

#[cfg(target_os = "linux")]
#[test]
fn given_late_destination_collision_or_unsafe_parent_when_staging_or_publishing_then_user_objects_are_never_overwritten(
) {
    use std::{io::Write, os::unix::fs::symlink};
    for case in ["file", "symlink", "directory", "parent-symlink"] {
        // Arrange
        let base = tempfile::tempdir().unwrap();
        let root = ReceivePolicy::new(Some(base.path().to_owned()))
            .snapshot()
            .pin()
            .unwrap();
        let target = base.path().join("file");
        let outside = tempfile::tempdir().unwrap();
        if case == "parent-symlink" {
            symlink(outside.path(), base.path().join("escape")).unwrap();
            assert!(root.stage_file(&base.path().join("escape/file")).is_err());
            assert!(std::fs::read_dir(outside.path()).unwrap().next().is_none());
            continue;
        }
        let (mut file, publication) = root.stage_file(&target).unwrap();
        file.write_all(b"incoming").unwrap();
        match case {
            "file" => std::fs::write(&target, b"user-owned").unwrap(),
            "symlink" => symlink(outside.path().join("untouched"), &target).unwrap(),
            _ => std::fs::create_dir(&target).unwrap(),
        }
        // Act / Assert
        assert!(publication.publish(&file, 8).is_err());
        drop(publication);
        drop(file);
        if case == "file" {
            assert_eq!(std::fs::read(&target).unwrap(), b"user-owned");
        }
        if case == "symlink" {
            assert!(target.symlink_metadata().unwrap().file_type().is_symlink());
        }
        if case == "directory" {
            assert!(target.is_dir());
        }
        assert!(std::fs::read_dir(outside.path()).unwrap().next().is_none());
        assert_eq!(std::fs::read_dir(base.path()).unwrap().count(), 1);
    }
}

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
