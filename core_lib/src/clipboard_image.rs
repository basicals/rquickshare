//! Validated static PNG capture. Full-sized encoded bytes never reach a GUI decoder.
use std::io::{self, Cursor, Write};

use tokio_util::sync::CancellationToken;

use crate::{
    file_source::{OpenedSelection, SelectionLimits},
    memory_source::{self, MAX_MEMORY_FILE_BYTES},
};

pub const MAX_PIXELS: u64 = 64_000_000;
const MAX_DECODED: usize = 256_000_000;
const PREVIEW_EDGE: u32 = 512;

pub struct CapturedImage {
    pub source: OpenedSelection,
    pub width: u32,
    pub height: u32,
    pub preview_width: u32,
    pub preview_height: u32,
    pub preview_rgba: Vec<u8>,
}
impl std::fmt::Debug for CapturedImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CapturedImage")
            .field("width", &self.width)
            .field("height", &self.height)
            .finish_non_exhaustive()
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ImageError {
    InvalidPng,
    Bounds,
    Cancelled,
    Resource,
}
fn check_cancel(cancel: &CancellationToken) -> Result<(), ImageError> {
    if cancel.is_cancelled() {
        Err(ImageError::Cancelled)
    } else {
        Ok(())
    }
}
fn dimensions(width: u32, height: u32) -> Result<(), ImageError> {
    if width == 0 || height == 0 || u64::from(width) * u64::from(height) > MAX_PIXELS {
        Err(ImageError::Bounds)
    } else {
        Ok(())
    }
}
struct BoundedPng(Vec<u8>);
impl Write for BoundedPng {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self
            .0
            .len()
            .checked_add(bytes.len())
            .is_none_or(|size| size > MAX_MEMORY_FILE_BYTES)
        {
            return Err(io::Error::other("Encoded image exceeds bounds"));
        }
        self.0
            .try_reserve_exact(bytes.len())
            .map_err(|_| io::Error::other("Image allocation failed"))?;
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
pub fn capture(bytes: &[u8], cancel: &CancellationToken) -> Result<CapturedImage, ImageError> {
    check_cancel(cancel)?;
    if bytes.is_empty() || bytes.len() > MAX_MEMORY_FILE_BYTES {
        return Err(ImageError::Bounds);
    }
    let mut decoder = png::Decoder::new_with_limits(
        Cursor::new(bytes),
        png::Limits {
            bytes: 64 * 1024 * 1024,
        },
    );
    decoder.ignore_checksums(false);
    decoder.set_ignore_text_chunk(true);
    decoder.set_ignore_iccp_chunk(true);
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let header = decoder
        .read_header_info()
        .map_err(|_| ImageError::InvalidPng)?;
    dimensions(header.width, header.height)?;
    check_cancel(cancel)?;
    let mut reader = decoder.read_info().map_err(|_| ImageError::InvalidPng)?;
    if reader.info().animation_control.is_some() || reader.info().frame_control.is_some() {
        return Err(ImageError::InvalidPng);
    }
    let size = reader
        .output_buffer_size()
        .filter(|size| *size <= MAX_DECODED)
        .ok_or(ImageError::Bounds)?;
    let mut pixels = Vec::new();
    pixels
        .try_reserve_exact(size)
        .map_err(|_| ImageError::Resource)?;
    pixels.resize(size, 0);
    let output = reader
        .next_frame(&mut pixels)
        .map_err(|_| ImageError::InvalidPng)?;
    reader.finish().map_err(|_| ImageError::InvalidPng)?;
    if reader.info().animation_control.is_some() || output.bit_depth != png::BitDepth::Eight {
        return Err(ImageError::InvalidPng);
    }
    dimensions(output.width, output.height)?;
    pixels.truncate(output.buffer_size());
    check_cancel(cancel)?;
    let (preview_width, preview_height, preview_rgba) = preview(&pixels, &output, cancel)?;
    let mut encoded = BoundedPng(Vec::new());
    let mut encoder = png::Encoder::new(&mut encoded, output.width, output.height);
    encoder.set_color(output.color_type);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(|_| ImageError::Resource)?;
    writer
        .write_image_data(&pixels)
        .map_err(|_| ImageError::Resource)?;
    writer.finish().map_err(|_| ImageError::Resource)?;
    check_cancel(cancel)?;
    let file = memory_source::create(&encoded.0, "Clipboard.png".into())
        .map_err(|_| ImageError::Resource)?;
    check_cancel(cancel)?;
    let source = OpenedSelection::from_files(vec![file], SelectionLimits::default())
        .map_err(|_| ImageError::Resource)?;
    Ok(CapturedImage {
        source,
        width: output.width,
        height: output.height,
        preview_width,
        preview_height,
        preview_rgba,
    })
}
fn preview(
    pixels: &[u8],
    output: &png::OutputInfo,
    cancel: &CancellationToken,
) -> Result<(u32, u32, Vec<u8>), ImageError> {
    let (width, height) = (output.width, output.height);
    let (pw, ph) = if width >= height {
        let pw = width.min(PREVIEW_EDGE);
        (
            pw,
            ((u64::from(height) * u64::from(pw) / u64::from(width)) as u32).max(1),
        )
    } else {
        let ph = height.min(PREVIEW_EDGE);
        (
            ((u64::from(width) * u64::from(ph) / u64::from(height)) as u32).max(1),
            ph,
        )
    };
    let channels = match output.color_type {
        png::ColorType::Grayscale => 1,
        png::ColorType::GrayscaleAlpha => 2,
        png::ColorType::Rgb => 3,
        png::ColorType::Rgba => 4,
        png::ColorType::Indexed => return Err(ImageError::InvalidPng),
    };
    let mut rgba = Vec::new();
    rgba.try_reserve_exact((pw * ph * 4) as usize)
        .map_err(|_| ImageError::Resource)?;
    for y in 0..ph {
        check_cancel(cancel)?;
        let sy = (u64::from(y) * u64::from(height) / u64::from(ph)) as usize;
        for x in 0..pw {
            let sx = (u64::from(x) * u64::from(width) / u64::from(pw)) as usize;
            let index = sy * output.line_size + sx * channels;
            let pixel = pixels
                .get(index..index + channels)
                .ok_or(ImageError::InvalidPng)?;
            match channels {
                1 => rgba.extend_from_slice(&[pixel[0], pixel[0], pixel[0], 255]),
                2 => rgba.extend_from_slice(&[pixel[0], pixel[0], pixel[0], pixel[1]]),
                3 => rgba.extend_from_slice(&[pixel[0], pixel[1], pixel[2], 255]),
                4 => rgba.extend_from_slice(pixel),
                _ => unreachable!(),
            }
        }
    }
    Ok((pw, ph, rgba))
}
#[cfg(test)]
mod tests;
