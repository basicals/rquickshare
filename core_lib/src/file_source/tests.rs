use std::fs::{self, OpenOptions};
use std::io::{Read, Seek, SeekFrom};
use std::os::unix::ffi::OsStringExt;
use std::os::unix::fs::symlink;

use super::*;

fn source(directory: &Path, name: &str, bytes: &[u8]) -> PathBuf {
    let path = directory.join(name);
    fs::write(&path, bytes).unwrap();
    path
}

fn adopt(path: &Path, name: &str) -> OpenedFile {
    OpenedFile::adopt(File::open(path).unwrap(), name.to_owned()).unwrap()
}

fn acquire(path: &Path, limits: SelectionLimits) -> io::Result<OpenedSelection> {
    OpenedSelection::acquire(&[path.to_owned()], limits)
}

#[test]
fn given_portable_names_when_validated_then_unicode_and_nested_paths_are_accepted() {
    // Arrange / Act / Assert.
    for name in [
        "file",
        "with spaces.png",
        "файл.txt",
        "folder/child.txt",
        "100%.txt",
        &"a".repeat(255),
    ] {
        assert!(validate_name(name).is_ok(), "{name:?}");
    }
}

#[test]
fn given_unsafe_names_when_validated_then_every_unsafe_form_is_rejected() {
    // Arrange.
    let long_component = "a".repeat(256);
    let long_name = "a/".repeat(8192) + "a";
    let names = [
        "",
        "/root",
        ".",
        "..",
        "a/../b",
        "a/./b",
        "a//b",
        "a/",
        "a\\b",
        "C:/b",
        "a\0b",
        "a\nb",
        "a\rb",
        "a\tb",
        "a\u{7f}b",
        &long_component,
        &long_name,
    ];
    // Act / Assert.
    for name in names {
        assert!(validate_name(name).is_err(), "{name:?}");
    }
}

#[test]
fn given_name_at_total_length_limit_when_validated_then_limit_is_inclusive() {
    // Arrange.
    let name = "a/".repeat(8191) + "aa";
    // Act / Assert.
    assert_eq!(name.len(), 16384);
    assert!(validate_name(&name).is_ok());
}

#[test]
fn given_protocol_size_boundaries_when_validated_then_only_representable_sizes_are_accepted() {
    // Arrange / Act / Assert.
    for size in [0, 1, i64::MAX as u64 - 1, i64::MAX as u64] {
        assert_eq!(source_size(size).unwrap(), size);
    }
    for size in [i64::MAX as u64 + 1, u64::MAX] {
        assert!(source_size(size).is_err());
    }
}

#[test]
fn given_total_size_boundaries_when_adding_then_overflow_and_budget_excess_are_rejected() {
    // Arrange / Act / Assert.
    for (total, size, limit, expected) in [
        (0, 0, 1, Some(0)),
        (0, 1, 1, Some(1)),
        (1, 1, 2, Some(2)),
        (1, 2, 2, None),
        (u64::MAX - 1, 1, u64::MAX, Some(u64::MAX)),
        (u64::MAX, 1, u64::MAX, None),
    ] {
        assert_eq!(checked_total(total, size, limit).ok(), expected);
    }
}

#[test]
fn given_zero_budget_when_constructing_selection_then_each_zero_field_is_rejected() {
    // Arrange.
    let limits = SelectionLimits::default();
    let directory = tempfile::tempdir().unwrap();
    let path = source(directory.path(), "file", b"a");
    // Act / Assert.
    for value in [
        SelectionLimits {
            max_files: 0,
            ..limits
        },
        SelectionLimits {
            max_entries: 0,
            ..limits
        },
        SelectionLimits {
            max_depth: 0,
            ..limits
        },
        SelectionLimits {
            max_bytes: 0,
            ..limits
        },
    ] {
        assert!(value.validate().is_err());
        assert!(OpenedSelection::from_files(vec![adopt(&path, "file")], value).is_err());
        assert!(acquire(&path, value).is_err());
    }
}

#[test]
fn given_invalid_manifest_names_when_descriptor_is_adopted_then_no_name_bypasses_validation() {
    // Arrange.
    let directory = tempfile::tempdir().unwrap();
    let path = source(directory.path(), "file", b"a");
    let long_component = "a".repeat(256);
    let long_name = "a/".repeat(8192) + "a";
    // Act / Assert.
    for name in [
        "",
        "/root",
        ".",
        "..",
        "a/../b",
        "a/./b",
        "a//b",
        "a/",
        "a\\b",
        "C:/b",
        "a\0b",
        "a\nb",
        "a\rb",
        "a\tb",
        "a\u{7f}b",
        &long_component,
        &long_name,
    ] {
        assert!(OpenedFile::adopt(File::open(&path).unwrap(), name.to_owned()).is_err());
    }
}

#[test]
fn given_inheritable_descriptor_when_adopted_then_close_on_exec_is_enforced() {
    // Arrange.
    let directory = tempfile::tempdir().unwrap();
    let path = source(directory.path(), "file", b"a");
    let file = File::open(path).unwrap();
    fcntl_setfd(&file, FdFlags::empty()).unwrap();
    // Act.
    let opened = OpenedFile::adopt(file, "file".into()).unwrap();
    // Assert.
    assert!(fcntl_getfd(&opened.file)
        .unwrap()
        .contains(FdFlags::CLOEXEC));
}

#[test]
fn given_default_budgets_when_inspected_then_all_resource_caps_are_explicit() {
    // Arrange / Act.
    let limits = SelectionLimits::default();
    // Assert.
    assert_eq!(
        (
            limits.max_files,
            limits.max_entries,
            limits.max_depth,
            limits.max_bytes
        ),
        (256, 4096, 64, 1 << 40)
    );
}

#[test]
fn given_read_only_regular_file_when_adopted_then_metadata_and_bytes_are_preserved() {
    // Arrange.
    let directory = tempfile::tempdir().unwrap();
    let path = source(directory.path(), "original", b"original bytes");
    // Act.
    let file = adopt(&path, "folder/display.txt");
    let mut bytes = [0; 32];
    let count = file.read_chunk_at(0, &mut bytes).unwrap();
    // Assert.
    assert_eq!(file.relative_name(), "folder/display.txt");
    assert_eq!(file.size(), 14);
    assert_eq!(&bytes[..count], b"original bytes");
}

#[test]
fn given_named_tmpfs_files_when_adopted_then_filesystem_sizes_bytes_and_clones_are_preserved() {
    // Arrange: require real seal-capable tmpfs rather than silently testing a disk file.
    let directory = tempfile::tempdir_in("/dev/shm").unwrap();
    for size in [0, 1, crate::memory_source::MAX_MEMORY_FILE_BYTES + 1] {
        let expected = vec![0x5a; size];
        let path = source(directory.path(), "ordinary", &expected);
        let reader = File::open(&path).unwrap();
        assert_eq!(reader.metadata().unwrap().nlink(), 1);
        assert_eq!(
            rustix::fs::fcntl_get_seals(&reader).unwrap(),
            rustix::fs::SealFlags::SEAL
        );

        // Act.
        let file = OpenedFile::adopt(reader, "folder/display.txt".into()).unwrap();
        let selection =
            OpenedSelection::from_files(vec![file], SelectionLimits::default()).unwrap();
        let duplicate = selection.try_clone().unwrap();
        let file = &duplicate.files()[0];
        let mut actual = vec![0; size];

        // Assert: the anonymous-memory size cap/charge must not apply to a named file.
        assert_eq!(file.size(), size as u64);
        assert_eq!(file.memory_bytes(), 0);
        assert_eq!(file.relative_name(), "folder/display.txt");
        assert_eq!(file.read_chunk_at(0, &mut actual).unwrap(), size);
        assert_eq!(actual, expected);
        assert_eq!(file.read_chunk_at(size as u64, &mut []).unwrap(), 0);
    }
}

#[test]
fn given_mutable_unlinked_tmpfs_file_when_adopted_then_anonymous_guard_is_preserved() {
    // Arrange.
    let directory = tempfile::tempdir_in("/dev/shm").unwrap();
    let path = source(directory.path(), "ordinary", b"original bytes");
    let reader = File::open(&path).unwrap();
    let adopted = OpenedFile::adopt(reader.try_clone().unwrap(), "display.txt".into()).unwrap();
    fs::remove_file(&path).unwrap();
    assert_eq!(reader.metadata().unwrap().nlink(), 0);
    assert_eq!(
        rustix::fs::fcntl_get_seals(&reader).unwrap(),
        rustix::fs::SealFlags::SEAL
    );

    // Act / Assert: a held source remains readable; fresh anonymous adoption fails closed.
    let mut bytes = [0; 14];
    assert_eq!(adopted.read_chunk_at(0, &mut bytes).unwrap(), bytes.len());
    assert_eq!(&bytes, b"original bytes");
    assert_eq!(
        OpenedFile::adopt(reader, "disk-looking-name".into())
            .unwrap_err()
            .kind(),
        ErrorKind::InvalidInput
    );
}

#[test]
fn given_writable_descriptors_when_adopted_then_both_write_access_modes_are_rejected() {
    // Arrange.
    let directory = tempfile::tempdir().unwrap();
    let path = source(directory.path(), "original", b"bytes");
    // Act / Assert.
    for read in [false, true] {
        let file = OpenOptions::new()
            .read(read)
            .write(true)
            .open(&path)
            .unwrap();
        assert!(OpenedFile::adopt(file, "display".into()).is_err());
    }
    assert_eq!(fs::read(path).unwrap(), b"bytes");
}

#[test]
fn given_path_only_descriptor_when_adopted_then_it_cannot_be_used_as_read_authority() {
    // Arrange.
    let directory = tempfile::tempdir().unwrap();
    let path = source(directory.path(), "original", b"bytes");
    let fd = openat2(
        CWD,
        &path,
        OFlags::PATH | OFlags::CLOEXEC,
        Mode::empty(),
        ResolveFlags::NO_SYMLINKS,
    )
    .unwrap();
    // Act / Assert.
    assert!(OpenedFile::adopt(fd.into(), "display".into()).is_err());
}

#[test]
fn given_directory_or_special_descriptor_when_adopted_then_non_regular_types_are_rejected() {
    // Arrange.
    let directory = tempfile::tempdir().unwrap();
    // Act / Assert.
    for file in [
        File::open(directory.path()).unwrap(),
        File::open("/dev/null").unwrap(),
    ] {
        assert!(OpenedFile::adopt(file, "display".into()).is_err());
    }
}

#[test]
fn given_empty_regular_file_when_adopted_then_zero_byte_read_completes() {
    // Arrange.
    let directory = tempfile::tempdir().unwrap();
    let path = source(directory.path(), "empty", b"");
    // Act.
    let file = adopt(&path, "empty");
    // Assert.
    assert_eq!(file.size(), 0);
    assert_eq!(file.read_chunk_at(0, &mut []).unwrap(), 0);
}

#[test]
fn given_acquired_file_when_path_is_replaced_then_original_handle_is_used() {
    // Arrange.
    let directory = tempfile::tempdir().unwrap();
    let path = source(directory.path(), "original", b"original");
    let file = adopt(&path, "original");
    // Act.
    fs::rename(&path, directory.path().join("moved")).unwrap();
    fs::write(&path, b"replacement").unwrap();
    let mut bytes = [0; 8];
    file.read_chunk_at(0, &mut bytes).unwrap();
    // Assert.
    assert_eq!(&bytes, b"original");
    assert_eq!(fs::read(path).unwrap(), b"replacement");
}

#[test]
fn given_acquired_file_when_unlinked_then_the_held_source_remains_readable() {
    // Arrange.
    let directory = tempfile::tempdir().unwrap();
    let path = source(directory.path(), "original", b"bytes");
    let file = adopt(&path, "original");
    // Act.
    fs::remove_file(path).unwrap();
    let mut bytes = [0; 5];
    file.read_chunk_at(0, &mut bytes).unwrap();
    // Assert.
    assert_eq!(&bytes, b"bytes");
}

#[test]
fn given_shared_file_description_when_other_holder_seeks_then_positioned_reads_are_independent() {
    // Arrange.
    let directory = tempfile::tempdir().unwrap();
    let path = source(directory.path(), "original", b"abcdef");
    let file = File::open(path).unwrap();
    let mut other = file.try_clone().unwrap();
    let opened = OpenedFile::adopt(file, "original".into()).unwrap();
    // Act.
    other.seek(SeekFrom::Start(5)).unwrap();
    let mut bytes = [0; 2];
    opened.read_chunk_at(1, &mut bytes).unwrap();
    // Assert.
    assert_eq!(&bytes, b"bc");
    assert_eq!(other.stream_position().unwrap(), 5);
    opened.read_chunk_at(1, &mut bytes).unwrap();
    assert_eq!(&bytes, b"bc");
}

#[test]
fn given_source_size_changes_when_reading_then_growth_and_truncation_fail() {
    // Arrange / Act / Assert.
    for replacement in [b"a".as_slice(), b"abcdefghi".as_slice()] {
        let directory = tempfile::tempdir().unwrap();
        let path = source(directory.path(), "original", b"abcdef");
        let file = adopt(&path, "original");
        fs::write(&path, replacement).unwrap();
        assert_eq!(
            file.read_chunk_at(0, &mut [0; 2]).unwrap_err().kind(),
            ErrorKind::InvalidData
        );
    }
}

#[test]
fn given_same_size_write_when_reading_then_no_immutable_snapshot_is_claimed() {
    // Arrange.
    let directory = tempfile::tempdir().unwrap();
    let path = source(directory.path(), "original", b"before");
    let file = adopt(&path, "original");
    // Act.
    fs::write(path, b"after!").unwrap();
    let mut bytes = [0; 6];
    file.read_chunk_at(0, &mut bytes).unwrap();
    // Assert.
    assert_eq!(&bytes, b"after!");
}

#[test]
fn given_offset_and_buffer_boundaries_when_reading_then_only_advertised_bytes_are_returned() {
    // Arrange.
    let directory = tempfile::tempdir().unwrap();
    let path = source(directory.path(), "original", b"abcdef");
    let file = adopt(&path, "original");
    // Act / Assert.
    for (offset, length, expected) in [
        (0, 2, b"ab".as_slice()),
        (4, 2, b"ef"),
        (5, 4, b"f"),
        (6, 4, b""),
    ] {
        let mut bytes = vec![0xff; length];
        let count = file.read_chunk_at(offset, &mut bytes).unwrap();
        assert_eq!(&bytes[..count], expected);
        assert!(bytes[count..].iter().all(|byte| *byte == 0xff));
    }
    assert_eq!(
        file.read_chunk_at(7, &mut [0; 1]).unwrap_err().kind(),
        ErrorKind::InvalidInput
    );
    assert_eq!(
        file.read_chunk_at(u64::MAX, &mut [0; 1])
            .unwrap_err()
            .kind(),
        ErrorKind::InvalidInput
    );
    assert_eq!(
        file.read_chunk_at(0, &mut []).unwrap_err().kind(),
        ErrorKind::InvalidInput
    );
}

#[test]
fn given_empty_root_list_or_empty_folder_when_acquired_then_empty_selection_is_rejected() {
    // Arrange.
    let directory = tempfile::tempdir().unwrap();
    // Act / Assert.
    assert!(OpenedSelection::acquire(&[], SelectionLimits::default()).is_err());
    assert!(acquire(directory.path(), SelectionLimits::default()).is_err());
    assert!(OpenedSelection::from_files(vec![], SelectionLimits::default()).is_err());
}

#[test]
fn given_nested_folder_when_acquired_then_tree_names_sorting_and_totals_are_preserved() {
    // Arrange.
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("root");
    fs::create_dir_all(root.join("nested")).unwrap();
    source(&root, "z file", b"zz");
    source(&root, "a.txt", b"aaa");
    source(&root.join("nested"), "файл", b"unicode");
    // Act.
    let selection = acquire(&root, SelectionLimits::default()).unwrap();
    // Assert.
    assert_eq!(
        selection
            .files()
            .iter()
            .map(OpenedFile::relative_name)
            .collect::<Vec<_>>(),
        ["root/a.txt", "root/nested/файл", "root/z file"]
    );
    assert_eq!(selection.total_bytes(), 12);
}

#[test]
fn given_regular_root_when_acquired_then_source_does_not_depend_on_absolute_name() {
    // Arrange.
    let directory = tempfile::tempdir().unwrap();
    let path = source(directory.path(), "with spaces %.txt", b"bytes");
    // Act.
    let selection = acquire(&path, SelectionLimits::default()).unwrap();
    // Assert.
    assert_eq!(selection.files()[0].relative_name(), "with spaces %.txt");
    assert_eq!(selection.total_bytes(), 5);
}

#[test]
fn given_missing_or_unnamed_root_when_acquired_then_errors_are_private_and_atomic() {
    // Arrange.
    let directory = tempfile::tempdir().unwrap();
    let good = source(directory.path(), "private-good", b"bytes");
    let missing = directory.path().join("private-missing");
    // Act / Assert.
    for roots in [vec![good.clone(), missing], vec![PathBuf::from("/")]] {
        let error = OpenedSelection::acquire(&roots, SelectionLimits::default()).unwrap_err();
        assert!(!error.to_string().contains("private"));
        assert!(!error
            .to_string()
            .contains(directory.path().to_str().unwrap()));
    }
    assert_eq!(fs::read(good).unwrap(), b"bytes");
}

#[test]
fn given_symlink_root_or_ancestor_when_acquired_then_resolution_is_rejected() {
    // Arrange.
    let directory = tempfile::tempdir().unwrap();
    let path = source(directory.path(), "real", b"bytes");
    let linked = directory.path().join("link");
    symlink(&path, &linked).unwrap();
    let ancestor = directory.path().join("ancestor");
    symlink(directory.path(), &ancestor).unwrap();
    // Act / Assert.
    for value in [linked, ancestor.join("real")] {
        assert!(acquire(&value, SelectionLimits::default()).is_err());
    }
}

#[test]
fn given_symlink_descendant_when_acquired_then_no_outside_bytes_are_selected() {
    // Arrange.
    let directory = tempfile::tempdir().unwrap();
    let outside = source(directory.path(), "secret", b"secret bytes");
    let root = directory.path().join("root");
    fs::create_dir(&root).unwrap();
    source(&root, "valid", b"valid bytes");
    symlink(outside, root.join("escape")).unwrap();
    // Act / Assert.
    assert!(acquire(&root, SelectionLimits::default()).is_err());
}

#[test]
fn given_symlink_to_selected_subtree_when_acquired_then_in_root_aliases_are_also_rejected() {
    // Arrange.
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("root");
    fs::create_dir(&root).unwrap();
    source(&root, "valid", b"valid bytes");
    symlink("valid", root.join("alias")).unwrap();
    // Act / Assert.
    assert!(acquire(&root, SelectionLimits::default()).is_err());
}

#[test]
fn given_fifo_root_or_descendant_when_acquired_then_no_blocking_read_is_attempted() {
    // Arrange.
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("root");
    fs::create_dir(&root).unwrap();
    let fifo = root.join("fifo");
    rustix::fs::mknodat(
        CWD,
        &fifo,
        rustix::fs::FileType::Fifo,
        Mode::RUSR | Mode::WUSR,
        0,
    )
    .unwrap();
    // Act / Assert.
    assert!(acquire(&fifo, SelectionLimits::default()).is_err());
    assert!(acquire(&root, SelectionLimits::default()).is_err());
}

#[test]
fn given_pinned_root_when_ancestor_path_is_replaced_then_original_subtree_is_used() {
    // Arrange.
    let directory = tempfile::tempdir().unwrap();
    let root_path = directory.path().join("root");
    fs::create_dir(&root_path).unwrap();
    source(&root_path, "file", b"original");
    let root = AcquiredRoot::open(&root_path).unwrap();
    // Act.
    fs::rename(&root_path, directory.path().join("moved")).unwrap();
    fs::create_dir(&root_path).unwrap();
    source(&root_path, "file", b"attacker");
    let mut builder = SelectionBuilder::new(SelectionLimits::default()).unwrap();
    root.collect(&mut builder).unwrap();
    let selection = builder.finish().unwrap();
    let mut bytes = [0; 8];
    selection.files()[0].read_chunk_at(0, &mut bytes).unwrap();
    // Assert.
    assert_eq!(&bytes, b"original");
}

#[test]
fn given_replaced_descendant_when_opened_relative_to_root_then_symlink_escape_fails() {
    // Arrange.
    let directory = tempfile::tempdir().unwrap();
    let root_path = directory.path().join("root");
    fs::create_dir_all(root_path.join("child")).unwrap();
    source(&root_path.join("child"), "file", b"original");
    let root = File::open(&root_path).unwrap();
    let outside = directory.path().join("outside");
    fs::create_dir(&outside).unwrap();
    source(&outside, "file", b"secret");
    // Act.
    fs::rename(root_path.join("child"), root_path.join("moved")).unwrap();
    symlink(outside, root_path.join("child")).unwrap();
    // Assert.
    assert!(open_child(&root, Path::new("child/file")).is_err());
}

#[test]
fn given_relative_escape_when_opened_from_root_then_absolute_and_parent_paths_fail() {
    // Arrange.
    let directory = tempfile::tempdir().unwrap();
    let root = File::open(directory.path()).unwrap();
    // Act / Assert.
    for path in ["../outside", "/etc/passwd"] {
        assert!(open_child(&root, Path::new(path)).is_err());
    }
}

#[test]
fn given_file_budget_when_acquiring_then_exact_limit_succeeds_and_excess_fails() {
    // Arrange.
    let directory = tempfile::tempdir().unwrap();
    let a = source(directory.path(), "a", b"a");
    let b = source(directory.path(), "b", b"b");
    let limits = SelectionLimits {
        max_files: 1,
        ..SelectionLimits::default()
    };
    // Act / Assert.
    assert!(OpenedSelection::acquire(std::slice::from_ref(&a), limits).is_ok());
    assert!(OpenedSelection::acquire(&[a, b], limits).is_err());
}

#[test]
fn given_entry_budget_when_walking_then_roots_and_empty_directories_are_counted() {
    // Arrange.
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("root");
    fs::create_dir_all(root.join("empty")).unwrap();
    source(&root, "file", b"a");
    // Act / Assert.
    assert!(acquire(
        &root,
        SelectionLimits {
            max_entries: 3,
            ..SelectionLimits::default()
        }
    )
    .is_ok());
    assert!(acquire(
        &root,
        SelectionLimits {
            max_entries: 2,
            ..SelectionLimits::default()
        }
    )
    .is_err());
}

#[test]
fn given_depth_budget_when_walking_then_exact_depth_succeeds_and_excess_fails() {
    // Arrange.
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("root");
    fs::create_dir_all(root.join("nested")).unwrap();
    source(&root.join("nested"), "file", b"a");
    // Act / Assert.
    assert!(acquire(
        &root,
        SelectionLimits {
            max_depth: 2,
            ..SelectionLimits::default()
        }
    )
    .is_ok());
    assert!(acquire(
        &root,
        SelectionLimits {
            max_depth: 1,
            ..SelectionLimits::default()
        }
    )
    .is_err());
}

#[test]
fn given_byte_budget_when_acquiring_then_zero_files_and_exact_sum_are_handled() {
    // Arrange.
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("root");
    fs::create_dir(&root).unwrap();
    source(&root, "a", b"aa");
    source(&root, "b", b"bbb");
    source(&root, "empty", b"");
    // Act / Assert.
    assert!(acquire(
        &root,
        SelectionLimits {
            max_bytes: 5,
            ..SelectionLimits::default()
        }
    )
    .is_ok());
    assert!(acquire(
        &root,
        SelectionLimits {
            max_bytes: 4,
            ..SelectionLimits::default()
        }
    )
    .is_err());
}

#[test]
fn given_duplicate_destinations_when_building_selection_then_collisions_fail() {
    // Arrange.
    let directory = tempfile::tempdir().unwrap();
    let path = source(directory.path(), "file", b"bytes");
    // Act / Assert.
    assert!(OpenedSelection::from_files(
        vec![adopt(&path, "same"), adopt(&path, "same")],
        SelectionLimits::default()
    )
    .is_err());
    assert!(OpenedSelection::acquire(&[path.clone(), path], SelectionLimits::default()).is_err());
}

#[test]
fn given_same_named_roots_when_acquired_then_colliding_tree_destinations_fail() {
    // Arrange.
    let directory = tempfile::tempdir().unwrap();
    let a = directory.path().join("a/root");
    let b = directory.path().join("b/root");
    fs::create_dir_all(&a).unwrap();
    fs::create_dir_all(&b).unwrap();
    source(&a, "same", b"a");
    source(&b, "different", b"b");
    // Act / Assert.
    assert!(OpenedSelection::acquire(&[a, b], SelectionLimits::default()).is_err());
}

#[test]
fn given_file_directory_prefix_collision_when_building_selection_then_both_orders_fail() {
    // Arrange.
    let directory = tempfile::tempdir().unwrap();
    let path = source(directory.path(), "file", b"bytes");
    // Act / Assert.
    for names in [["a", "a/b"], ["a/b", "a"], ["a/b", "a/b/c"]] {
        let files = names.iter().map(|name| adopt(&path, name)).collect();
        assert!(OpenedSelection::from_files(files, SelectionLimits::default()).is_err());
    }
    assert!(OpenedSelection::from_files(
        vec![adopt(&path, "a"), adopt(&path, "ab/c")],
        SelectionLimits::default()
    )
    .is_ok());
}

#[test]
fn given_non_utf8_or_control_source_name_when_acquired_then_metadata_is_rejected() {
    // Arrange / Act / Assert.
    for name in [
        std::ffi::OsString::from_vec(vec![0xff]),
        std::ffi::OsString::from("bad\nname"),
    ] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join(name);
        fs::write(&path, b"bytes").unwrap();
        assert!(acquire(&path, SelectionLimits::default()).is_err());
        assert!(acquire(directory.path(), SelectionLimits::default()).is_err());
    }
}

#[test]
fn given_private_selection_when_debugged_then_paths_names_and_contents_are_absent() {
    // Arrange.
    let directory = tempfile::tempdir().unwrap();
    let path = source(directory.path(), "PRIVATE-NAME", b"PRIVATE-CONTENTS");
    let selection = acquire(&path, SelectionLimits::default()).unwrap();
    // Act.
    let debug = format!("{selection:?} {:?}", selection.files()[0]);
    // Assert.
    assert!(!debug.contains("PRIVATE"));
    assert!(!debug.contains(directory.path().to_str().unwrap()));
    assert!(debug.contains("file_count: 1"));
}

#[test]
fn given_selection_when_dropped_then_original_files_are_never_deleted() {
    // Arrange.
    let directory = tempfile::tempdir().unwrap();
    let path = source(directory.path(), "original", b"bytes");
    let selection = acquire(&path, SelectionLimits::default()).unwrap();
    // Act.
    drop(selection);
    // Assert.
    assert_eq!(fs::read(path).unwrap(), b"bytes");
}

#[test]
fn given_received_handles_when_bounded_then_count_and_bytes_are_enforced_without_paths() {
    // Arrange.
    let directory = tempfile::tempdir().unwrap();
    let path = source(directory.path(), "file", b"ab");
    // Act / Assert.
    for (count, bytes, succeeds) in [(2, 4, true), (1, 4, false), (2, 3, false)] {
        let selection = OpenedSelection::from_files(
            vec![adopt(&path, "a"), adopt(&path, "b")],
            SelectionLimits {
                max_files: count,
                max_bytes: bytes,
                ..SelectionLimits::default()
            },
        );
        assert_eq!(selection.is_ok(), succeeds);
    }
}

#[test]
fn given_received_manifest_depth_when_bounded_then_exact_depth_succeeds_and_excess_fails() {
    // Arrange.
    let directory = tempfile::tempdir().unwrap();
    let path = source(directory.path(), "file", b"bytes");
    // Act / Assert.
    for (name, limit, accepted) in [
        ("file", 1, true),
        ("a/file", 1, true),
        ("a/b/file", 1, false),
        ("a/b/file", 2, true),
    ] {
        let selected = OpenedSelection::from_files(
            vec![adopt(&path, name)],
            SelectionLimits {
                max_depth: limit,
                ..SelectionLimits::default()
            },
        );
        assert_eq!(selected.is_ok(), accepted);
    }
}

#[test]
fn given_cloned_handle_when_original_position_changes_then_owned_bytes_remain_accessible() {
    // Arrange.
    let directory = tempfile::tempdir().unwrap();
    let path = source(directory.path(), "file", b"abc");
    let opened = adopt(&path, "file");
    // Act.
    let mut cloned = opened.clone_handle().unwrap();
    let mut bytes = Vec::new();
    cloned.read_to_end(&mut bytes).unwrap();
    let mut first = [0; 1];
    opened.read_chunk_at(0, &mut first).unwrap();
    // Assert.
    assert_eq!(bytes, b"abc");
    assert_eq!(&first, b"a");
}
