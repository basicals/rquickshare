use std::fs::File;
use std::os::fd::AsFd;

use zbus::connection::socket::WriteHalf;
use zbus::zvariant::Fd;

use super::*;

fn header(big: bool, fields: u32, body: u32) -> [u8; 16] {
    let mut bytes = [0; 16];
    bytes[0] = if big { b'B' } else { b'l' };
    bytes[4..8].copy_from_slice(&if big {
        body.to_be_bytes()
    } else {
        body.to_le_bytes()
    });
    bytes[12..16].copy_from_slice(&if big {
        fields.to_be_bytes()
    } else {
        fields.to_le_bytes()
    });
    bytes
}

#[test]
fn given_primary_lengths_when_checked_then_both_endian_boundaries_are_enforced() {
    // Arrange / Act / Assert
    for big in [false, true] {
        for (fields, body, accepted) in [
            (0, 0, true),
            (0, MAX_FRAME as u32 - 16, true),
            (0, MAX_FRAME as u32 - 15, false),
            (1, MAX_FRAME as u32 - 24, true),
            (1, MAX_FRAME as u32 - 23, false),
            (u32::MAX, 0, false),
            (0, u32::MAX, false),
        ] {
            assert_eq!(check_header(&header(big, fields, body)).is_ok(), accepted);
        }
    }
    let mut bytes = header(false, 0, 0);
    bytes[0] = b'?';
    assert!(check_header(&bytes).is_err());
}

fn fd_message(count: usize) -> Message {
    let source = tempfile::NamedTempFile::new().unwrap();
    let handles: Vec<_> = (0..count)
        .map(|_| File::open(source.path()).unwrap())
        .collect();
    let fds: Vec<_> = handles.iter().map(Fd::from).collect();
    Message::method_call("/test", "Files")
        .unwrap()
        .build(&(fds, "payload"))
        .unwrap()
}

#[tokio::test]
async fn given_fragmented_frame_with_max_fds_when_received_then_sequence_and_cloexec_are_preserved()
{
    // Arrange
    let (left, right) = UnixStream::pair().unwrap();
    let (mut read, _) = BoundedSocket(left).split().take();
    let (_, mut write) = right.into_split();
    let message = fd_message(MAX_FDS);
    let fds: Vec<_> = message.data().fds().iter().map(AsFd::as_fd).collect();
    assert_eq!(fds.len(), MAX_FDS);
    // Act: two SCM_RIGHTS fragments across an incomplete primary header.
    write
        .sendmsg(&message.data()[..8], &fds[..8])
        .await
        .unwrap();
    write
        .sendmsg(&message.data()[8..], &fds[8..])
        .await
        .unwrap();
    let result = read
        .receive_message(77, &mut Vec::new(), &mut Vec::new())
        .await
        .unwrap();
    // Assert
    assert!(result.recv_position() > message.recv_position());
    assert_eq!(result.data().fds().len(), MAX_FDS);
    for fd in result.data().fds() {
        assert!(rustix::io::fcntl_getfd(fd)
            .unwrap()
            .contains(rustix::io::FdFlags::CLOEXEC));
    }
}

#[tokio::test]
async fn given_excess_fds_in_multiple_receives_when_framed_then_aggregate_limit_rejects() {
    // Arrange
    let (left, right) = UnixStream::pair().unwrap();
    let (mut read, _) = BoundedSocket(left).split().take();
    let (_, mut write) = right.into_split();
    let message = fd_message(MAX_FDS + 1);
    let fds: Vec<_> = message.data().fds().iter().map(AsFd::as_fd).collect();
    // Act
    write
        .sendmsg(&message.data()[..8], &fds[..8])
        .await
        .unwrap();
    write
        .sendmsg(&message.data()[8..], &fds[8..])
        .await
        .unwrap();
    // Assert
    assert!(read
        .receive_message(1, &mut Vec::new(), &mut Vec::new())
        .await
        .is_err());
}

#[tokio::test]
async fn given_truncated_ancillary_data_when_received_then_partial_fds_are_not_admitted() {
    // Arrange
    let (left, right) = UnixStream::pair().unwrap();
    let (mut read, _) = BoundedSocket(left).split().take();
    let (_, mut write) = right.into_split();
    let message = fd_message(MAX_FDS + 1);
    let fds: Vec<_> = message.data().fds().iter().map(AsFd::as_fd).collect();
    // Act
    write.sendmsg(message.data(), &fds).await.unwrap();
    // Assert
    assert!(read
        .receive_message(1, &mut Vec::new(), &mut Vec::new())
        .await
        .is_err());
}

#[tokio::test]
async fn given_oversized_or_buffered_primary_header_when_received_then_no_body_is_requested() {
    // Arrange / Act / Assert: writer remains open and supplies no advertised body.
    for buffered in [false, true] {
        let (left, right) = UnixStream::pair().unwrap();
        let (mut read, _) = BoundedSocket(left).split().take();
        let (_, mut write) = right.into_split();
        let bytes = header(false, 0, u32::MAX);
        let mut pending = if buffered { bytes.to_vec() } else { Vec::new() };
        if !buffered {
            write.sendmsg(&bytes[..5], &[]).await.unwrap();
            write.sendmsg(&bytes[5..], &[]).await.unwrap();
        }
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            read.receive_message(1, &mut pending, &mut Vec::new()),
        )
        .await
        .unwrap();
        assert!(result.is_err());
    }
}

#[tokio::test]
async fn given_eof_or_handshake_fd_when_read_then_transport_fails_closed() {
    // Arrange / Act / Assert
    let (left, right) = UnixStream::pair().unwrap();
    let (mut read, _) = BoundedSocket(left).split().take();
    drop(right);
    assert!(read
        .receive_message(1, &mut Vec::new(), &mut Vec::new())
        .await
        .is_err());
    let (left, right) = UnixStream::pair().unwrap();
    let (mut read, _) = BoundedSocket(left).split().take();
    let (_, mut write) = right.into_split();
    let source = tempfile::NamedTempFile::new().unwrap();
    write
        .sendmsg(b"OK\r\n", &[source.as_file().as_fd()])
        .await
        .unwrap();
    assert!(read.recvmsg(&mut [0; 16]).await.is_err());
}
