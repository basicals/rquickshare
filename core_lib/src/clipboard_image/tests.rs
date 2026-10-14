use super::*;
fn fixture(
    width: u32,
    height: u32,
    color: png::ColorType,
    depth: png::BitDepth,
    bytes: &[u8],
    animated: bool,
) -> Vec<u8> {
    let mut data = Vec::new();
    let mut encoder = png::Encoder::new(&mut data, width, height);
    encoder.set_color(color);
    encoder.set_depth(depth);
    encoder
        .add_text_chunk("private".into(), "private metadata".into())
        .unwrap();
    if animated {
        encoder.set_animated(1, 0).unwrap();
    }
    if color == png::ColorType::Indexed {
        encoder.set_palette(vec![10, 20, 30]);
        encoder.set_trns(vec![40]);
    }
    let mut writer = encoder.write_header().unwrap();
    writer.write_image_data(bytes).unwrap();
    writer.finish().unwrap();
    data
}
#[test]
fn given_static_color_formats_when_captured_then_pixels_dimensions_and_metadata_free_source_are_preserved(
) {
    // Arrange / Act / Assert
    use png::{BitDepth::*, ColorType::*};
    for (color, depth, input, rgba) in [
        (Grayscale, Eight, vec![17], [17, 17, 17, 255]),
        (GrayscaleAlpha, Eight, vec![17, 42], [17, 17, 17, 42]),
        (Rgb, Eight, vec![10, 20, 30], [10, 20, 30, 255]),
        (Rgba, Eight, vec![10, 20, 30, 40], [10, 20, 30, 40]),
        (
            Rgba,
            Sixteen,
            vec![10, 255, 20, 255, 30, 255, 40, 255],
            [10, 20, 30, 40],
        ),
        (Indexed, Eight, vec![0], [10, 20, 30, 40]),
        (Grayscale, One, vec![128], [255, 255, 255, 255]),
    ] {
        let input = fixture(1, 1, color, depth, &input, false);
        let captured = capture(&input, &CancellationToken::new()).unwrap();
        assert_eq!(
            (
                captured.width,
                captured.height,
                captured.preview_width,
                captured.preview_height
            ),
            (1, 1, 1, 1)
        );
        assert_eq!(captured.preview_rgba, rgba);
        assert_eq!(captured.source.files()[0].relative_name(), "Clipboard.png");
        let file = &captured.source.files()[0];
        assert_eq!(file.memory_bytes(), file.size());
        let mut png = vec![0; file.size() as usize];
        assert_eq!(file.read_chunk_at(0, &mut png).unwrap(), png.len());
        assert!(!png
            .windows(b"private metadata".len())
            .any(|value| value == b"private metadata"));
        let recaptured = capture(&png, &CancellationToken::new()).unwrap();
        assert_eq!(recaptured.preview_rgba, rgba);
        assert!(!format!("{captured:?}").contains("metadata"));
    }
}
#[test]
fn given_corrupt_truncated_or_animated_png_when_captured_then_no_source_is_created() {
    // Arrange
    let data = fixture(
        1,
        1,
        png::ColorType::Rgba,
        png::BitDepth::Eight,
        &[1, 2, 3, 4],
        false,
    );
    let animated = fixture(
        1,
        1,
        png::ColorType::Rgba,
        png::BitDepth::Eight,
        &[1, 2, 3, 4],
        true,
    );
    let mut corrupt = data.clone();
    let idat = corrupt.windows(4).position(|v| v == b"IDAT").unwrap();
    corrupt[idat + 4] ^= 1;
    let mut deflate = corrupt.clone();
    let length = u32::from_be_bytes(deflate[idat - 4..idat].try_into().unwrap()) as usize;
    let mut crc_value = u32::MAX;
    for byte in &deflate[idat..idat + 4 + length] {
        crc_value ^= u32::from(*byte);
        for _ in 0..8 {
            crc_value = (crc_value >> 1) ^ (0xedb88320 & (0u32.wrapping_sub(crc_value & 1)));
        }
    }
    deflate[idat + 4 + length..idat + 8 + length].copy_from_slice(&(!crc_value).to_be_bytes());
    let mut crc = data.clone();
    crc[29] ^= 1;
    // Act / Assert
    for invalid in [
        vec![],
        b"not PNG".to_vec(),
        data[..32].to_vec(),
        data[..data.len() - 1].to_vec(),
        corrupt,
        deflate,
        crc,
        animated,
    ] {
        assert!(capture(&invalid, &CancellationToken::new()).is_err());
    }
    let token = CancellationToken::new();
    token.cancel();
    assert_eq!(capture(&data, &token).unwrap_err(), ImageError::Cancelled);
}
#[test]
fn given_image_and_output_bounds_when_validated_then_limits_are_inclusive_without_truncation() {
    // Arrange / Act / Assert
    for (w, h, valid) in [
        (0, 1, false),
        (1, 0, false),
        (1, 1, true),
        (8000, 8000, true),
        (8000, 8001, false),
        (u32::MAX, u32::MAX, false),
    ] {
        assert_eq!(dimensions(w, h).is_ok(), valid);
    }
    let mut oversized_header = Vec::new();
    png::Encoder::new(&mut oversized_header, 8000, 8001)
        .write_header()
        .unwrap();
    assert_eq!(
        capture(&oversized_header, &CancellationToken::new()).unwrap_err(),
        ImageError::Bounds
    );
    assert_eq!(
        capture(
            &vec![0; MAX_MEMORY_FILE_BYTES + 1],
            &CancellationToken::new()
        )
        .unwrap_err(),
        ImageError::Bounds
    );
    let mut writer = BoundedPng(vec![0; MAX_MEMORY_FILE_BYTES - 1]);
    writer.write_all(&[9]).unwrap();
    assert!(writer.write_all(&[10]).is_err());
    assert_eq!(writer.0.len(), MAX_MEMORY_FILE_BYTES);
    assert_eq!(writer.0.last(), Some(&9));
    // Both aspect-ratio branches preserve original dimensions and exact sampled pixels.
    for (width, height, pw, ph) in [(1024, 2, 512, 1), (2, 1024, 1, 512), (513, 513, 512, 512)] {
        let mut pixels = Vec::new();
        for y in 0..height {
            for x in 0..width {
                pixels.extend_from_slice(&[x as u8, y as u8, 30, 40]);
            }
        }
        let input = fixture(
            width,
            height,
            png::ColorType::Rgba,
            png::BitDepth::Eight,
            &pixels,
            false,
        );
        let output = capture(&input, &CancellationToken::new()).unwrap();
        assert_eq!(
            (
                output.width,
                output.height,
                output.preview_width,
                output.preview_height
            ),
            (width, height, pw, ph)
        );
        assert_eq!(output.preview_rgba.len(), (pw * ph * 4) as usize);
        for y in 0..ph {
            for x in 0..pw {
                let offset = ((y * pw + x) * 4) as usize;
                assert_eq!(
                    &output.preview_rgba[offset..offset + 4],
                    &[(x * width / pw) as u8, (y * height / ph) as u8, 30, 40]
                );
            }
        }
    }
}
