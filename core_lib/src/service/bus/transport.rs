//! Safe ancillary receipt and framing limits before zbus allocates/decodes the body.

use std::io::{self, IoSliceMut};
use std::mem::MaybeUninit;
use std::os::fd::OwnedFd;
use std::sync::Arc;

use rustix::net::{recvmsg, RecvAncillaryBuffer, RecvAncillaryMessage, RecvFlags, ReturnFlags};
use tokio::net::{unix::OwnedReadHalf, unix::OwnedWriteHalf, UnixStream};
use zbus::connection::socket::{ReadHalf, Socket, Split};
use zbus::fdo::ConnectionCredentials;
use zbus::Message;

pub(super) const MAX_FRAME: usize = 64 * 1024;
pub(super) const MAX_FDS: usize = super::super::uploads::BATCH_FILES;

fn invalid() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        "Native bus receive budget or framing violation",
    )
}

pub(super) struct BoundedSocket(pub UnixStream);

impl Socket for BoundedSocket {
    type ReadHalf = BoundedRead;
    type WriteHalf = OwnedWriteHalf;

    fn split(self) -> Split<Self::ReadHalf, Self::WriteHalf> {
        let (read, write) = self.0.into_split();
        Split::new(
            BoundedRead {
                read: Arc::new(read),
                handshake_bytes: 0,
            },
            write,
        )
    }
}

#[derive(Debug)]
pub(super) struct BoundedRead {
    read: Arc<OwnedReadHalf>,
    handshake_bytes: usize,
}

async fn receive(read: &OwnedReadHalf, buffer: &mut [u8]) -> io::Result<(usize, Vec<OwnedFd>)> {
    let stream = read.as_ref();
    stream
        .async_io(tokio::io::Interest::READABLE, || {
            let mut storage = [MaybeUninit::uninit(); rustix::cmsg_space!(ScmRights(MAX_FDS))];
            let mut ancillary = RecvAncillaryBuffer::new(&mut storage);
            let mut slices = [IoSliceMut::new(buffer)];
            // This kernel flag sets CLOEXEC before the descriptors enter this process.
            let received = recvmsg(stream, &mut slices, &mut ancillary, RecvFlags::CMSG_CLOEXEC)?;
            let mut fds = Vec::new();
            let mut unexpected = false;
            for message in ancillary.drain() {
                match message {
                    RecvAncillaryMessage::ScmRights(handles) => fds.extend(handles),
                    _ => unexpected = true,
                }
            }
            if unexpected || received.flags.contains(ReturnFlags::CTRUNC) || fds.len() > MAX_FDS {
                return Err(invalid());
            }
            if received.bytes == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "Native bus closed",
                ));
            }
            Ok((received.bytes, fds))
        })
        .await
}

/// Checks only primary framing lengths; zbus still validates the full D-Bus message.
fn check_header(header: &[u8; 16]) -> io::Result<()> {
    let integer = |bytes: &[u8]| -> io::Result<u64> {
        let value: [u8; 4] = bytes.try_into().map_err(|_| invalid())?;
        Ok(match header[0] {
            b'l' => u32::from_le_bytes(value),
            b'B' => u32::from_be_bytes(value),
            _ => return Err(invalid()),
        } as u64)
    };
    let fields = integer(&header[12..16])?;
    let body = integer(&header[4..8])?;
    let header_length = 16 + fields;
    let padded = (header_length + 7) & !7;
    if padded
        .checked_add(body)
        .is_none_or(|size| size > MAX_FRAME as u64)
    {
        return Err(invalid());
    }
    Ok(())
}

#[derive(Debug)]
struct FrameRead {
    read: Arc<OwnedReadHalf>,
    header: [u8; 16],
    filled: usize,
    fds: usize,
}

#[async_trait::async_trait]
impl ReadHalf for FrameRead {
    async fn recvmsg(&mut self, buffer: &mut [u8]) -> io::Result<(usize, Vec<OwnedFd>)> {
        let result = receive(&self.read, buffer).await?;
        self.fds += result.1.len();
        if self.fds > MAX_FDS {
            return Err(invalid());
        }
        let count = (16 - self.filled).min(result.0);
        self.header[self.filled..self.filled + count].copy_from_slice(&buffer[..count]);
        self.filled += count;
        if self.filled == 16 {
            check_header(&self.header)?;
        }
        Ok(result)
    }
    fn can_pass_unix_fd(&self) -> bool {
        true
    }
}

#[async_trait::async_trait]
impl ReadHalf for BoundedRead {
    async fn recvmsg(&mut self, buffer: &mut [u8]) -> io::Result<(usize, Vec<OwnedFd>)> {
        // This method is used for EXTERNAL authentication, before binary framing.
        let result = receive(&self.read, buffer).await?;
        self.handshake_bytes = self
            .handshake_bytes
            .checked_add(result.0)
            .ok_or_else(invalid)?;
        if self.handshake_bytes > MAX_FRAME || !result.1.is_empty() {
            return Err(invalid());
        }
        Ok(result)
    }
    async fn receive_message(
        &mut self,
        sequence: u64,
        bytes: &mut Vec<u8>,
        fds: &mut Vec<OwnedFd>,
    ) -> zbus::Result<Message> {
        if !fds.is_empty() || bytes.len() > MAX_FRAME {
            return Err(invalid().into());
        }
        let mut frame = FrameRead {
            read: self.read.clone(),
            header: [0; 16],
            filled: bytes.len().min(16),
            fds: 0,
        };
        frame.header[..frame.filled].copy_from_slice(&bytes[..frame.filled]);
        if frame.filled == 16 {
            check_header(&frame.header)?;
        }
        // Delegate parsing and receive-position assignment to the SDK. FrameRead's
        // recvmsg validates the primary header before its default parser resizes.
        frame.receive_message(sequence, bytes, fds).await
    }
    fn can_pass_unix_fd(&self) -> bool {
        true
    }
    async fn peer_credentials(&mut self) -> io::Result<ConnectionCredentials> {
        let credentials = rustix::net::sockopt::socket_peercred(self.read.as_ref().as_ref())?;
        Ok(ConnectionCredentials::default()
            .set_unix_user_id(credentials.uid.as_raw())
            .set_process_id(credentials.pid.as_raw_nonzero().get() as u32))
    }
}

#[cfg(test)]
mod tests;
