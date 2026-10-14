//! Anonymous immutable source bytes. Only an owned descriptor authorizes reopening.
use std::{
    fs::File,
    io::{self, Write},
    os::fd::AsRawFd,
};

use rustix::fs::{fcntl_add_seals, memfd_create, open, MemfdFlags, Mode, OFlags, SealFlags};

use crate::file_source::OpenedFile;

pub const MAX_MEMORY_FILE_BYTES: usize = 32 * 1024 * 1024;
pub const MEMORY_RESERVATION_BYTES: u64 = 128 * 1024 * 1024;
pub(crate) const IMMUTABLE_SEALS: SealFlags = SealFlags::WRITE
    .union(SealFlags::GROW)
    .union(SealFlags::SHRINK)
    .union(SealFlags::SEAL);

pub fn create(bytes: &[u8], name: String) -> io::Result<OpenedFile> {
    if bytes.is_empty() || bytes.len() > MAX_MEMORY_FILE_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Memory source size exceeds bounds",
        ));
    }
    let mut writer = File::from(memfd_create(
        "rquickshare-source",
        MemfdFlags::CLOEXEC | MemfdFlags::ALLOW_SEALING,
    )?);
    writer.write_all(bytes)?;
    fcntl_add_seals(&writer, IMMUTABLE_SEALS)?;
    let reader = open(
        format!("/proc/self/fd/{}", writer.as_raw_fd()),
        OFlags::RDONLY | OFlags::CLOEXEC,
        Mode::empty(),
    )?;
    OpenedFile::adopt(File::from(reader), name)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn given_memory_bounds_when_created_then_exact_immutable_private_bytes_are_owned() {
        // Arrange / Act / Assert
        for size in [0, 1, MAX_MEMORY_FILE_BYTES, MAX_MEMORY_FILE_BYTES + 1] {
            let bytes = vec![0x63; size];
            let result = create(&bytes, "private.png".into());
            if size == 0 || size > MAX_MEMORY_FILE_BYTES {
                assert_eq!(result.unwrap_err().kind(), io::ErrorKind::InvalidInput);
                continue;
            }
            let source = result.unwrap();
            assert_eq!(source.memory_bytes(), size as u64);
            assert_eq!(source.size(), size as u64);
            let reader = source.clone_handle().unwrap();
            assert_eq!(
                rustix::fs::fcntl_get_seals(&reader).unwrap(),
                IMMUTABLE_SEALS
            );
            assert_eq!(
                rustix::fs::fcntl_getfl(&reader).unwrap() & OFlags::ACCMODE,
                OFlags::RDONLY
            );
            assert_eq!(
                rustix::io::fcntl_getfd(&reader).unwrap(),
                rustix::io::FdFlags::CLOEXEC
            );
            assert!(reader.set_len(0).is_err());
            assert!(rustix::io::pwrite(&reader, b"changed", 0).is_err());
            assert!(!format!("{source:?}").contains("private"));
            let mut captured = vec![0; size];
            drop(reader);
            assert_eq!(source.read_chunk_at(0, &mut captured).unwrap(), size);
            assert_eq!(captured, bytes);
            let reader = source.clone_handle().unwrap();
            drop(source);
            assert_eq!(reader.metadata().unwrap().len(), size as u64);
        }
    }
    #[test]
    fn given_untrusted_memory_handles_when_adopted_then_only_complete_seals_and_bounded_size_pass()
    {
        // Arrange / Act / Assert
        for (seals, size, accepted) in [
            (SealFlags::empty(), 1, false),
            (
                SealFlags::WRITE | SealFlags::GROW | SealFlags::SHRINK,
                1,
                false,
            ),
            (IMMUTABLE_SEALS, 0, false),
            (IMMUTABLE_SEALS, MAX_MEMORY_FILE_BYTES as u64 + 1, false),
            (IMMUTABLE_SEALS, 1, true),
        ] {
            let writer = File::from(memfd_create("untrusted", MemfdFlags::ALLOW_SEALING).unwrap());
            writer.set_len(size).unwrap();
            fcntl_add_seals(&writer, seals).unwrap();
            let reader = File::from(
                open(
                    format!("/proc/self/fd/{}", writer.as_raw_fd()),
                    OFlags::RDONLY,
                    Mode::empty(),
                )
                .unwrap(),
            );
            let result = OpenedFile::adopt(reader, "disk-looking-name".into());
            assert_eq!(result.is_ok(), accepted);
            if let Ok(source) = result {
                assert_eq!(source.memory_bytes(), size);
            }
        }
    }
}
