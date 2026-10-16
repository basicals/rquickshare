//! Only bounded validated pixels cross from the isolated decoder into GTK.
use std::{fmt, path::PathBuf, process::Stdio, sync::Arc, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    process::Command,
    sync::Semaphore,
};
use tokio_util::sync::CancellationToken;

pub const MAX_ENCODED: usize = 256 * 1024;
pub const MAX_DIMENSION: u32 = 2048;
pub const PREVIEW_EDGE: u32 = 256;
pub const MAX_PIXELS_BYTES: usize = PREVIEW_EDGE as usize * PREVIEW_EDGE as usize * 4;
pub const MAX_OUTPUT: usize = 16 + MAX_PIXELS_BYTES;
pub const MEMORY_BYTES: u64 = 512 * 1024 * 1024;
pub const CPU_SECONDS: u64 = 5;
pub const DESCRIPTORS: u64 = 128;
pub const DEADLINE: Duration = Duration::from_secs(10);
const MAGIC: &[u8; 8] = b"RQSP\0\0\0\x01";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Rejected,
    Unavailable,
    Cancelled,
    Busy,
}

pub fn admit(bytes: &[u8]) -> Result<(), Error> {
    if bytes.is_empty()
        || bytes.len() > MAX_ENCODED
        || !(bytes.starts_with(b"\x89PNG\r\n\x1a\n") || bytes.starts_with(b"\xff\xd8\xff"))
    {
        return Err(Error::Rejected);
    }
    Ok(())
}

pub fn dimensions(width: u32, height: u32) -> Result<(), Error> {
    if width == 0 || height == 0 || width > MAX_DIMENSION || height > MAX_DIMENSION {
        Err(Error::Rejected)
    } else {
        Ok(())
    }
}

pub fn scaled(width: u32, height: u32) -> Result<(u32, u32), Error> {
    dimensions(width, height)?;
    let longest = width.max(height);
    if longest <= PREVIEW_EDGE {
        return Ok((width, height));
    }
    Ok((
        (width * PREVIEW_EDGE / longest).max(1),
        (height * PREVIEW_EDGE / longest).max(1),
    ))
}

pub struct Pixels {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}
impl fmt::Debug for Pixels {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Pixels")
            .field("width", &self.width)
            .field("height", &self.height)
            .finish_non_exhaustive()
    }
}
impl Pixels {
    /// The helper validates decoder layout before reading or resampling pixels.
    pub fn from_rgba(width: u32, height: u32, stride: usize, bytes: &[u8]) -> Result<Self, Error> {
        dimensions(width, height)?;
        let row = width as usize * 4;
        if stride < row
            || stride > MAX_DIMENSION as usize * 4
            || stride.checked_mul(height as usize) != Some(bytes.len())
        {
            return Err(Error::Rejected);
        }
        let (pw, ph) = scaled(width, height)?;
        let length = pw as usize * ph as usize * 4;
        let mut rgba = Vec::new();
        rgba.try_reserve_exact(length)
            .map_err(|_| Error::Unavailable)?;
        for y in 0..ph {
            let sy = y as usize * height as usize / ph as usize;
            for x in 0..pw {
                let sx = x as usize * width as usize / pw as usize;
                let at = sy * stride + sx * 4;
                rgba.extend_from_slice(&bytes[at..at + 4]);
            }
        }
        Ok(Self {
            width: pw,
            height: ph,
            rgba,
        })
    }
    pub fn encode(&self) -> Result<Vec<u8>, Error> {
        self.validate()?;
        let mut output = Vec::with_capacity(16 + self.rgba.len());
        output.extend_from_slice(MAGIC);
        output.extend_from_slice(&self.width.to_be_bytes());
        output.extend_from_slice(&self.height.to_be_bytes());
        output.extend_from_slice(&self.rgba);
        Ok(output)
    }
    fn validate(&self) -> Result<(), Error> {
        if self.width == 0
            || self.height == 0
            || self.width > PREVIEW_EDGE
            || self.height > PREVIEW_EDGE
            || self.width as usize * self.height as usize * 4 != self.rgba.len()
        {
            Err(Error::Rejected)
        } else {
            Ok(())
        }
    }
    pub fn parse(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() < 16 || bytes.len() > MAX_OUTPUT || &bytes[..8] != MAGIC {
            return Err(Error::Rejected);
        }
        let width = u32::from_be_bytes(bytes[8..12].try_into().map_err(|_| Error::Rejected)?);
        let height = u32::from_be_bytes(bytes[12..16].try_into().map_err(|_| Error::Rejected)?);
        // Validate before cloning any body received from the helper.
        if width == 0
            || height == 0
            || width > PREVIEW_EDGE
            || height > PREVIEW_EDGE
            || width as usize * height as usize * 4 != bytes.len() - 16
        {
            return Err(Error::Rejected);
        }
        Ok(Self {
            width,
            height,
            rgba: bytes[16..].to_vec(),
        })
    }
}

#[derive(Clone)]
pub struct Decoder {
    executable: PathBuf,
    permit: Arc<Semaphore>,
}
impl fmt::Debug for Decoder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Decoder").finish_non_exhaustive()
    }
}
impl Decoder {
    pub fn new(executable: PathBuf) -> Self {
        Self {
            executable,
            permit: Arc::new(Semaphore::new(1)),
        }
    }
    pub fn installed() -> Result<Self, Error> {
        let binary = std::env::current_exe().map_err(|_| Error::Unavailable)?;
        let parent = binary.parent().ok_or(Error::Unavailable)?;
        let helper = if parent.file_name().is_some_and(|name| name == "bin") {
            parent
                .parent()
                .ok_or(Error::Unavailable)?
                .join("libexec/rquickshare-preview-helper")
        } else {
            parent.join("rquickshare-preview-helper")
        };
        Ok(Self::new(helper))
    }
    pub async fn decode(&self, bytes: Vec<u8>, cancel: CancellationToken) -> Result<Pixels, Error> {
        if cancel.is_cancelled() {
            return Err(Error::Cancelled);
        }
        admit(&bytes)?;
        let _permit = self
            .permit
            .clone()
            .try_acquire_owned()
            .map_err(|_| Error::Busy)?;
        let mut child = Command::new(&self.executable)
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("XDG_DATA_DIRS", "/usr/local/share:/usr/share")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(|_| Error::Unavailable)?;
        let stdin = child.stdin.take().ok_or(Error::Unavailable)?;
        let stdout = child.stdout.take().ok_or(Error::Unavailable)?;
        let result = {
            let exchange = async {
                let write = async {
                    let mut stdin = stdin;
                    stdin
                        .write_all(&bytes)
                        .await
                        .map_err(|_| Error::Unavailable)?;
                    stdin.shutdown().await.map_err(|_| Error::Unavailable)?;
                    drop(stdin);
                    Ok::<_, Error>(())
                };
                let read = async {
                    let mut output = Vec::new();
                    stdout
                        .take((MAX_OUTPUT + 1) as u64)
                        .read_to_end(&mut output)
                        .await
                        .map_err(|_| Error::Unavailable)?;
                    if output.len() > MAX_OUTPUT {
                        return Err(Error::Rejected);
                    }
                    Ok(output)
                };
                let (_, output) = tokio::try_join!(write, read)?;
                let status = child.wait().await.map_err(|_| Error::Unavailable)?;
                if !status.success() {
                    return Err(Error::Unavailable);
                }
                Pixels::parse(&output)
            };
            tokio::select! {
                biased;
                _ = cancel.cancelled() => Err(Error::Cancelled),
                result = tokio::time::timeout(DEADLINE,exchange) => result.unwrap_or(Err(Error::Unavailable)),
            }
        };
        if result.is_err() {
            // Owned child only. kill also waits; after a completed child this is harmless.
            let _ = child.kill().await;
            let _ = child.wait().await;
        }
        result
    }
}

#[cfg(test)]
mod tests;
