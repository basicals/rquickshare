use super::*;
use crate::location_nearby_connections::{
    offline_frame, payload_transfer_frame::payload_header, v1_frame, OfflineFrame,
    PayloadTransferFrame, V1Frame,
};

fn packet(
    id: i64,
    size: usize,
    offset: usize,
    body: &[u8],
    last: bool,
) -> (PayloadHeader, PayloadChunk) {
    (
        PayloadHeader {
            id: Some(id),
            r#type: Some(payload_header::PayloadType::Bytes.into()),
            total_size: Some(size as i64),
            ..Default::default()
        },
        PayloadChunk {
            offset: Some(offset as i64),
            flags: Some(i32::from(last)),
            body: Some(body.to_vec()),
        },
    )
}

pub(crate) fn frame(id: i64, size: usize, offset: usize, body: &[u8], last: bool) -> OfflineFrame {
    let (header, chunk) = packet(id, size, offset, body, last);
    OfflineFrame {
        version: Some(offline_frame::Version::V1.into()),
        v1: Some(V1Frame {
            r#type: Some(v1_frame::FrameType::PayloadTransfer.into()),
            payload_transfer: Some(PayloadTransferFrame {
                packet_type: Some(1),
                payload_header: Some(header),
                payload_chunk: Some(chunk),
                ..Default::default()
            }),
            ..Default::default()
        }),
    }
}

#[test]
fn given_valid_interleaved_payloads_when_completed_then_exact_bytes_and_budget_are_released() {
    for id in [i64::MIN, -1, 0, 1, i64::MAX] {
        for separate_final in [false, true] {
            // Arrange
            let mut assembler = BytePayloads::default();
            let (header, first) = packet(id, 3, 0, b"a", false);
            // Act / Assert
            assert!(assembler.push(&header, &first).unwrap().is_none());
            let (other, empty) = packet(id.wrapping_add(1), 0, 0, b"", true);
            assert_eq!(assembler.push(&other, &empty).unwrap(), Some(vec![]));
            assert_eq!(assembler.reserved, 3);
            let (_, next) = packet(id, 3, 1, b"bc", !separate_final);
            let result = assembler.push(&header, &next).unwrap();
            let result = if separate_final {
                assert!(result.is_none());
                let (_, last) = packet(id, 3, 3, b"", true);
                assembler.push(&header, &last).unwrap()
            } else {
                result
            };
            assert_eq!(result, Some(b"abc".to_vec()));
            assert!(assembler.is_empty());
        }
    }
    for size in [0, 1, MAX_RESERVED_BYTES - 1, MAX_RESERVED_BYTES] {
        let mut assembler = BytePayloads::default();
        let bytes = vec![0x45; size];
        let (header, chunk) = packet(0, size, 0, &bytes, true);
        assert_eq!(assembler.push(&header, &chunk).unwrap(), Some(bytes));
        assert!(assembler.is_empty());
    }
}

#[test]
fn given_missing_or_invalid_headers_when_pushed_then_admission_fails_without_retained_memory() {
    for case in [
        "id",
        "size",
        "offset",
        "negative-size",
        "large-size",
        "negative-offset",
        "start-offset",
        "continue-offset",
    ] {
        // Arrange
        let mut assembler = BytePayloads::default();
        let (mut header, mut chunk) = packet(44, 2, 0, b"a", false);
        match case {
            "id" => header.id = None,
            "size" => header.total_size = None,
            "offset" => chunk.offset = None,
            "negative-size" => header.total_size = Some(-1),
            "large-size" => header.total_size = Some(MAX_RESERVED_BYTES as i64 + 1),
            "negative-offset" => chunk.offset = Some(-1),
            "start-offset" => chunk.offset = Some(1),
            "continue-offset" => {
                assembler.push(&header, &chunk).unwrap();
            }
            _ => unreachable!(),
        }
        // Act / Assert
        assert!(assembler.push(&header, &chunk).is_err(), "{case}");
        assert!(assembler.is_empty(), "{case}");
    }
}

#[test]
fn given_bound_payload_when_size_body_or_final_length_changes_then_all_pending_ownership_is_cleared(
) {
    for (size, body, final_chunk) in [
        (2, b"b".as_slice(), false),
        (4, b"b", false),
        (3, b"bcd", false),
        (3, b"b", true),
    ] {
        // Arrange
        let mut assembler = BytePayloads::default();
        let (other, pending) = packet(99, 10, 0, b"", false);
        assembler.push(&other, &pending).unwrap();
        let (header, first) = packet(11, 3, 0, b"a", false);
        assembler.push(&header, &first).unwrap();
        let (header, chunk) = packet(11, size, 1, body, final_chunk);
        // Act / Assert
        assert!(assembler.push(&header, &chunk).is_err());
        assert!(assembler.is_empty());
    }
    for (size, body) in [(0, b"a".as_slice()), (2, b"a"), (1, b"ab")] {
        let mut assembler = BytePayloads::default();
        let (header, chunk) = packet(1, size, 0, body, true);
        assert!(assembler.push(&header, &chunk).is_err());
        assert!(assembler.is_empty());
    }
}

#[test]
fn given_sparse_reservations_when_aggregate_limit_is_reached_then_completion_releases_admission_budget(
) {
    for extra in [0, 1, 2] {
        // Arrange: zero received bytes still reserve almost the whole budget.
        let mut assembler = BytePayloads::default();
        let (large, pending) = packet(1, MAX_RESERVED_BYTES - 1, 0, b"", false);
        assembler.push(&large, &pending).unwrap();
        let (small, pending) = packet(2, extra, 0, b"", false);
        // Act / Assert
        assert_eq!(assembler.push(&small, &pending).is_ok(), extra <= 1);
        if extra > 1 {
            assert!(assembler.is_empty());
        } else {
            assert_eq!(assembler.reserved, MAX_RESERVED_BYTES - 1 + extra);
            let (_, final_chunk) = packet(2, extra, 0, &vec![0; extra], true);
            assert_eq!(
                assembler.push(&small, &final_chunk).unwrap(),
                Some(vec![0; extra])
            );
            assert_eq!(assembler.reserved, MAX_RESERVED_BYTES - 1);
            let (replacement, pending) = packet(3, 1, 0, b"", false);
            assembler.push(&replacement, &pending).unwrap();
            assert_eq!(assembler.reserved, MAX_RESERVED_BYTES);
            let (overflow, pending) = packet(4, 1, 0, b"", false);
            assert!(assembler.push(&overflow, &pending).is_err());
            assert!(assembler.is_empty());
        }
        let (full, pending) = packet(5, MAX_RESERVED_BYTES, 0, b"", false);
        assembler.push(&full, &pending).unwrap();
        assert_eq!(assembler.reserved, MAX_RESERVED_BYTES);
    }
}

#[test]
fn given_empty_incomplete_payloads_when_count_limit_is_reached_then_slots_are_bounded_and_reusable()
{
    for count in [MAX_PENDING - 1, MAX_PENDING, MAX_PENDING + 1] {
        // Arrange / Act
        let mut assembler = BytePayloads::default();
        for id in 0..count {
            let (header, chunk) = packet(id as i64, 0, 0, b"", false);
            assert_eq!(assembler.push(&header, &chunk).is_ok(), id < MAX_PENDING);
        }
        // Assert
        if count > MAX_PENDING {
            assert!(assembler.is_empty());
        } else {
            assert_eq!(assembler.pending.len(), count);
            let (header, chunk) = packet(0, 0, 0, b"", true);
            assert_eq!(assembler.push(&header, &chunk).unwrap(), Some(vec![]));
            let (header, chunk) = packet(100, 0, 0, b"", false);
            assembler.push(&header, &chunk).unwrap();
            assert_eq!(assembler.pending.len(), count);
        }
    }
}

#[test]
fn given_completed_id_when_reused_then_no_completed_buffer_or_reservation_accumulates() {
    // Arrange
    let mut assembler = BytePayloads::default();
    // Act / Assert: both repeated and distinct ids start fresh after completion.
    for id in [0; 32].into_iter().chain(1..=32) {
        for bytes in [b"first".as_slice(), b"different"] {
            let (header, chunk) = packet(id, bytes.len(), 0, bytes, true);
            assert_eq!(
                assembler.push(&header, &chunk).unwrap(),
                Some(bytes.to_vec())
            );
            assert!(assembler.is_empty());
        }
    }
}

#[test]
fn given_private_bytes_and_id_when_debugged_or_rejected_then_diagnostics_do_not_expose_them() {
    // Arrange
    let mut assembler = BytePayloads::default();
    let (header, chunk) = packet(987654321, 16, 0, b"private-body", false);
    assembler.push(&header, &chunk).unwrap();
    // Act / Assert
    assert_eq!(
        format!("{assembler:?}"),
        "BytePayloads { pending: 1, reserved: 16 }"
    );
    let error = assembler.push(&header, &chunk).unwrap_err();
    assert_eq!(error.to_string(), "Unexpected byte payload offset");
    assert!(assembler.is_empty());
    drop(assembler);
}
