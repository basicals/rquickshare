use std::fs::File;
use std::io::{self, ErrorKind};
use std::os::unix::fs::FileExt;

trait ChunkSource {
    fn size(&self) -> io::Result<u64>;
    fn read_at(&self, buffer: &mut [u8], offset: u64) -> io::Result<usize>;
}

impl ChunkSource for File {
    fn size(&self) -> io::Result<u64> {
        Ok(self.metadata()?.len())
    }
    fn read_at(&self, buffer: &mut [u8], offset: u64) -> io::Result<usize> {
        FileExt::read_at(self, buffer, offset)
    }
}

/// Reads only the advertised range, without modifying the shared file position.
pub(crate) fn read_file_chunk(
    file: &File,
    expected_size: u64,
    offset: u64,
    buffer: &mut [u8],
) -> io::Result<usize> {
    read_chunk(file, expected_size, offset, buffer)
}

fn read_chunk(
    file: &impl ChunkSource,
    expected_size: u64,
    offset: u64,
    buffer: &mut [u8],
) -> io::Result<usize> {
    let remaining = expected_size.checked_sub(offset).ok_or_else(|| {
        io::Error::new(
            ErrorKind::InvalidInput,
            "File offset exceeds advertised size",
        )
    })?;
    if file.size()? != expected_size {
        return Err(io::Error::new(
            ErrorKind::InvalidData,
            "Source size changed during transfer",
        ));
    }
    if remaining == 0 {
        return Ok(0);
    }
    if buffer.is_empty() {
        return Err(io::Error::new(ErrorKind::InvalidInput, "Empty read buffer"));
    }
    let count = remaining.min(buffer.len() as u64) as usize;
    let read = loop {
        match file.read_at(&mut buffer[..count], offset) {
            Err(error) if error.kind() == ErrorKind::Interrupted => continue,
            result => break result?,
        }
    };
    if read == 0 {
        return Err(io::Error::new(
            ErrorKind::UnexpectedEof,
            "Source ended before advertised size",
        ));
    }
    Ok(read)
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;

    struct Storage {
        size: io::Result<u64>,
        bytes: Vec<u8>,
        error: Option<ErrorKind>,
        interrupted: Cell<bool>,
        calls: Cell<usize>,
    }

    impl ChunkSource for Storage {
        fn size(&self) -> io::Result<u64> {
            self.size
                .as_ref()
                .copied()
                .map_err(|error| io::Error::from(error.kind()))
        }
        fn read_at(&self, buffer: &mut [u8], offset: u64) -> io::Result<usize> {
            self.calls.set(self.calls.get() + 1);
            if self.interrupted.replace(false) {
                return Err(ErrorKind::Interrupted.into());
            }
            if let Some(error) = self.error {
                return Err(error.into());
            }
            let bytes = self.bytes.get(offset as usize..).unwrap_or_default();
            let count = buffer.len().min(bytes.len());
            buffer[..count].copy_from_slice(&bytes[..count]);
            Ok(count)
        }
    }

    fn storage() -> Storage {
        Storage {
            size: Ok(3),
            bytes: b"abc".to_vec(),
            error: None,
            interrupted: Cell::new(false),
            calls: Cell::new(0),
        }
    }

    #[test]
    fn given_premature_eof_when_reading_then_zero_progress_is_an_error() {
        // Arrange.
        let source = Storage {
            bytes: vec![],
            ..storage()
        };
        // Act / Assert.
        assert_eq!(
            read_chunk(&source, 3, 0, &mut [0; 4]).unwrap_err().kind(),
            ErrorKind::UnexpectedEof
        );
    }

    #[test]
    fn given_interrupted_storage_when_reading_then_operation_retries_without_sleep() {
        // Arrange.
        let source = Storage {
            interrupted: Cell::new(true),
            ..storage()
        };
        let mut bytes = [0; 3];
        // Act.
        let count = read_chunk(&source, 3, 0, &mut bytes).unwrap();
        // Assert.
        assert_eq!(count, 3);
        assert_eq!(&bytes, b"abc");
        assert_eq!(source.calls.get(), 2);
    }

    #[test]
    fn given_storage_errors_when_reading_then_permission_io_and_disconnect_errors_propagate() {
        // Arrange / Act / Assert.
        for kind in [
            ErrorKind::PermissionDenied,
            ErrorKind::Other,
            ErrorKind::UnexpectedEof,
            ErrorKind::NotConnected,
        ] {
            let source = Storage {
                error: Some(kind),
                ..storage()
            };
            assert_eq!(
                read_chunk(&source, 3, 0, &mut [0; 4]).unwrap_err().kind(),
                kind
            );
            assert_eq!(source.calls.get(), 1);
        }
    }

    #[test]
    fn given_metadata_error_when_reading_then_no_data_read_is_attempted() {
        // Arrange.
        let source = Storage {
            size: Err(ErrorKind::PermissionDenied.into()),
            ..storage()
        };
        // Act / Assert.
        assert_eq!(
            read_chunk(&source, 3, 0, &mut [0; 4]).unwrap_err().kind(),
            ErrorKind::PermissionDenied
        );
        assert_eq!(source.calls.get(), 0);
    }

    #[test]
    fn given_offset_beyond_end_when_reading_then_no_storage_call_is_attempted() {
        // Arrange.
        let source = storage();
        // Act / Assert.
        assert_eq!(
            read_chunk(&source, 3, 4, &mut [0; 4]).unwrap_err().kind(),
            ErrorKind::InvalidInput
        );
        assert_eq!(source.calls.get(), 0);
    }

    #[test]
    fn given_complete_or_empty_source_when_reading_then_no_storage_call_is_attempted() {
        // Arrange / Act / Assert.
        for (size, offset) in [(3, 3), (0, 0)] {
            let source = Storage {
                size: Ok(size),
                ..storage()
            };
            assert_eq!(read_chunk(&source, size, offset, &mut []).unwrap(), 0);
            assert_eq!(source.calls.get(), 0);
        }
    }

    #[test]
    fn given_short_storage_read_when_reading_then_only_actual_bytes_are_returned() {
        // Arrange.
        let source = Storage {
            bytes: b"a".to_vec(),
            ..storage()
        };
        let mut bytes = [0xff; 8];
        // Act.
        let count = read_chunk(&source, 3, 0, &mut bytes).unwrap();
        // Assert.
        assert_eq!(count, 1);
        assert_eq!(bytes, [b'a', 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
    }

    #[test]
    fn given_large_storage_source_when_reading_then_buffer_and_remaining_limits_are_applied() {
        // Arrange.
        let source = Storage {
            bytes: b"abcdef".to_vec(),
            ..storage()
        };
        // Act / Assert.
        for (offset, length, expected) in [(0, 2, b"ab".as_slice()), (1, 2, b"bc"), (2, 8, b"c")] {
            let mut bytes = vec![0xff; length];
            let count = read_chunk(&source, 3, offset, &mut bytes).unwrap();
            assert_eq!(&bytes[..count], expected);
            assert!(bytes[count..].iter().all(|byte| *byte == 0xff));
        }
    }
}
