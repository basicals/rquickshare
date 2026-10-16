use super::*;
use std::os::unix::fs::PermissionsExt;

const PNG: &[u8] = include_bytes!("../../assets/preview-fixture.png");

#[test]
fn given_encoded_input_when_admitted_then_only_bounded_png_or_jpeg_signatures_pass() {
    // Arrange / Act / Assert
    for prefix in [b"\x89PNG\r\n\x1a\n".as_slice(), b"\xff\xd8\xff"] {
        for size in [0, 1, 7, 8, MAX_ENCODED - 1, MAX_ENCODED, MAX_ENCODED + 1] {
            let mut bytes = vec![0; size];
            let copied = size.min(prefix.len());
            bytes[..copied].copy_from_slice(&prefix[..copied]);
            assert_eq!(
                admit(&bytes).is_ok(),
                size >= prefix.len() && size <= MAX_ENCODED
            );
        }
    }
    for bytes in [
        b"GIF89a".as_slice(),
        b"<svg/>",
        b"",
        b"private-image-data",
        b"\xff\xd8",
        b"\x89PNG\r\n\x1aX",
    ] {
        assert_eq!(admit(bytes), Err(Error::Rejected));
    }
}

#[test]
fn given_dimensions_when_scaled_then_boundaries_and_aspect_ratio_are_preserved() {
    // Arrange / Act / Assert
    for width in [0, 1, 255, 256, 257, 2047, 2048, 2049, u32::MAX] {
        for height in [0, 1, 255, 256, 257, 2047, 2048, 2049, u32::MAX] {
            let valid =
                width > 0 && height > 0 && width <= MAX_DIMENSION && height <= MAX_DIMENSION;
            assert_eq!(dimensions(width, height).is_ok(), valid);
            assert_eq!(scaled(width, height).is_ok(), valid);
        }
    }
    for (w, h, pw, ph) in [
        (1, 2048, 1, 256),
        (2048, 1, 256, 1),
        (512, 256, 256, 128),
        (256, 512, 128, 256),
        (1, 1, 1, 1),
        (256, 256, 256, 256),
        (257, 257, 256, 256),
    ] {
        assert_eq!(scaled(w, h).unwrap(), (pw, ph));
    }
}

#[test]
fn given_frame_layout_when_validated_then_exact_stride_and_body_bounds_are_required() {
    // Arrange / Act / Assert
    for (w, h, stride, len, valid) in [
        (1, 1, 4, 4, true),
        (1, 2, 8, 16, true),
        (2, 1, 8, 8, true),
        (0, 1, 4, 4, false),
        (1, 0, 4, 0, false),
        (2049, 1, 8196, 8196, false),
        (1, 2049, 4, 8196, false),
        (1, 1, 0, 0, false),
        (2, 1, 7, 7, false),
        (1, 1, 8193, 8193, false),
        (1, 1, 4, 3, false),
        (1, 1, 4, 5, false),
        (1, 2, 4, 4, false),
        (1, 1, usize::MAX, 0, false),
    ] {
        let bytes = vec![0; len];
        assert_eq!(
            Pixels::from_rgba(w, h, stride, &bytes).is_ok(),
            valid,
            "{w}/{h}/{stride}/{len}"
        );
    }
}

#[test]
fn given_pixel_protocol_when_corrupt_or_truncated_then_no_unvalidated_pixels_escape() {
    // Arrange
    let valid = Pixels {
        width: 2,
        height: 1,
        rgba: vec![1, 2, 3, 255, 4, 5, 6, 127],
    }
    .encode()
    .unwrap();
    // Act / Assert
    for length in 0..valid.len() {
        assert!(Pixels::parse(&valid[..length]).is_err());
    }
    for at in 0..8 {
        let mut bad = valid.clone();
        bad[at] ^= 1;
        assert!(Pixels::parse(&bad).is_err());
    }
    for at in [8, 12] {
        for value in [0_u32, 257, u32::MAX] {
            let mut bad = valid.clone();
            bad[at..at + 4].copy_from_slice(&value.to_be_bytes());
            assert!(Pixels::parse(&bad).is_err());
        }
    }
    let mut extra = valid;
    extra.push(0);
    assert!(Pixels::parse(&extra).is_err());
    assert!(Pixels::parse(&vec![0; MAX_OUTPUT + 1]).is_err());
    let maximum = Pixels {
        width: 256,
        height: 256,
        rgba: vec![11; MAX_PIXELS_BYTES],
    };
    let parsed = Pixels::parse(&maximum.encode().unwrap()).unwrap();
    assert_eq!(parsed.rgba, maximum.rgba);
    assert_eq!((parsed.width, parsed.height), (256, 256));
}

#[test]
fn given_rgba_with_padding_or_scaling_when_converted_then_exact_pixels_are_returned() {
    // Arrange / Act / Assert
    let padded = [1, 2, 3, 255, 9, 9, 9, 9, 4, 5, 6, 127, 8, 8, 8, 8];
    let pixels = Pixels::from_rgba(1, 2, 8, &padded).unwrap();
    assert_eq!(pixels.rgba, [1, 2, 3, 255, 4, 5, 6, 127]);
    let decoded = Pixels::parse(&pixels.encode().unwrap()).unwrap();
    assert_eq!((decoded.width, decoded.height), (1, 2));
    assert_eq!(decoded.rgba, pixels.rgba);
    let source: Vec<u8> = (0..512_u32)
        .flat_map(|x| [x as u8, (x / 256) as u8, 1, 255])
        .collect();
    let pixels = Pixels::from_rgba(512, 1, 512 * 4, &source).unwrap();
    let expected: Vec<u8> = (0..256_u32)
        .flat_map(|x| [(x * 2) as u8, (x * 2 / 256) as u8, 1, 255])
        .collect();
    assert_eq!((pixels.width, pixels.height), (256, 1));
    assert_eq!(pixels.rgba, expected);
}

#[test]
fn given_private_pixels_or_helper_path_when_debugged_then_content_and_paths_are_redacted() {
    // Arrange
    let pixels = Pixels {
        width: 1,
        height: 1,
        rgba: vec![83, 69, 67, 82],
    };
    let decoder = Decoder::new(PathBuf::from("/private/secret-helper"));
    // Act / Assert
    assert_eq!(format!("{pixels:?}"), "Pixels { width: 1, height: 1, .. }");
    assert_eq!(format!("{decoder:?}"), "Decoder { .. }");
}

#[test]
fn given_invalid_pixel_objects_when_encoded_then_public_fields_cannot_bypass_output_validation() {
    // Arrange / Act / Assert
    for (w, h, length) in [
        (0, 1, 0),
        (1, 0, 0),
        (257, 1, 1028),
        (1, 257, 1028),
        (1, 1, 3),
        (1, 1, 5),
    ] {
        let pixels = Pixels {
            width: w,
            height: h,
            rgba: vec![0; length],
        };
        assert_eq!(pixels.encode(), Err(Error::Rejected));
    }
}

#[tokio::test]
async fn given_actual_sandbox_helper_when_decoding_fixture_then_exact_rgba_is_verified() {
    // Arrange: the build step provides the actual sibling helper, never a mocked decoder.
    let helper =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/rquickshare-preview-helper");
    let decoder = Decoder::new(helper);
    // Act
    let image = decoder
        .decode(PNG.to_vec(), CancellationToken::new())
        .await
        .expect("real Bubblewrap/Glycin decoder must start; do not disable sandbox to pass");
    // Assert
    assert_eq!((image.width, image.height), (2, 1));
    assert_eq!(image.rgba, [255, 0, 0, 255, 0, 0, 255, 127]);
    let jpeg = decoder
        .decode(
            include_bytes!("../../assets/preview-fixture.jpg").to_vec(),
            CancellationToken::new(),
        )
        .await
        .unwrap();
    assert_eq!((jpeg.width, jpeg.height), (2, 1));
    assert_eq!(jpeg.rgba.len(), 8);
    // JPEG is lossy; bound decoded color error while checking order and alpha exactly.
    for (actual, expected) in jpeg.rgba.iter().zip([255_u8, 0, 0, 255, 0, 0, 255, 255]) {
        assert!(actual.abs_diff(expected) <= 2);
    }
    assert_eq!(jpeg.rgba[3], 255);
    assert_eq!(jpeg.rgba[7], 255);
}

#[tokio::test]
async fn given_cancelled_busy_or_missing_decoder_when_requested_then_no_queued_operation_starts() {
    // Arrange
    let decoder = Decoder::new(PathBuf::from("/nonexistent/rquickshare-helper"));
    let cancel = CancellationToken::new();
    cancel.cancel();
    // Act / Assert
    assert!(matches!(
        decoder.decode(PNG.to_vec(), cancel).await,
        Err(Error::Cancelled)
    ));
    let permit = decoder.permit.clone().acquire_owned().await.unwrap();
    assert!(matches!(
        decoder.decode(PNG.to_vec(), CancellationToken::new()).await,
        Err(Error::Busy)
    ));
    drop(permit);
    assert!(matches!(
        decoder.decode(PNG.to_vec(), CancellationToken::new()).await,
        Err(Error::Unavailable)
    ));
    assert!(matches!(
        decoder.decode(vec![], CancellationToken::new()).await,
        Err(Error::Rejected)
    ));
}

fn process_fixture(body: &str) -> (tempfile::TempDir, PathBuf) {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("fixture");
    std::fs::write(&path, format!("#!/usr/bin/python3\n{body}\n")).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
    (root, path)
}

#[tokio::test]
async fn given_external_process_failures_when_cancelled_or_malformed_then_owned_children_are_reaped(
) {
    // Arrange / Act / Assert: external transport fixtures, not a fake image decoder.
    for (body, expected) in [
        (
            "import sys; sys.stdin.buffer.read(); sys.stdout.buffer.write(b'RQSP')".to_owned(),
            Error::Rejected,
        ),
        (
            format!(
                "import sys; sys.stdin.buffer.read(); sys.stdout.buffer.write(b'x'*{})",
                MAX_OUTPUT + 1
            ),
            Error::Rejected,
        ),
        (
            "import sys; sys.stdin.buffer.read(); sys.exit(3)".to_owned(),
            Error::Unavailable,
        ),
    ] {
        let (_root, path) = process_fixture(&body);
        let result = Decoder::new(path)
            .decode(PNG.to_vec(), CancellationToken::new())
            .await;
        assert_eq!(result.unwrap_err(), expected);
    }
    for expire in [false, true] {
        let socket_root = tempfile::tempdir().unwrap();
        let socket = socket_root.path().join("ready");
        let listener = tokio::net::UnixListener::bind(&socket).unwrap();
        let (_root,path)=process_fixture(&format!("import socket,os,signal\ns=socket.socket(socket.AF_UNIX); s.connect({:?}); s.sendall(str(os.getpid()).encode()); s.close(); signal.pause()",socket.to_str().unwrap()));
        let decoder = Decoder::new(path);
        let cancel = CancellationToken::new();
        let task = {
            let cancel = cancel.clone();
            tokio::spawn(async move { decoder.decode(PNG.to_vec(), cancel).await })
        };
        let (mut ready, _) = tokio::time::timeout(DEADLINE, listener.accept())
            .await
            .unwrap()
            .unwrap();
        let mut pid = String::new();
        ready.read_to_string(&mut pid).await.unwrap();
        let pid: u32 = pid.parse().unwrap();
        if expire {
            tokio::time::pause();
            tokio::time::advance(DEADLINE).await;
        } else {
            cancel.cancel();
        }
        let result = task.await.unwrap();
        if expire {
            tokio::time::resume();
            assert!(matches!(result, Err(Error::Unavailable)));
        } else {
            assert!(matches!(result, Err(Error::Cancelled)));
        }
        assert!(
            !PathBuf::from(format!("/proc/{pid}")).exists(),
            "owned helper must be reaped before completion"
        );
    }
}
