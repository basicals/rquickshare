use super::*;
use crate::hdl::info::{InternalFileInfo, TransferMetadata};
use crate::utils::RemoteDeviceInfo;

fn prepared() -> InnerState {
    let mut state = InnerState {
        state: State::WaitingForUserConsent,
        remote_device_info: Some(RemoteDeviceInfo {
            name: "Nearby phone".into(),
            device_type: DeviceType::Phone,
        }),
        pin_code: Some("0123".into()),
        transfer_metadata: Some(TransferMetadata {
            files: Some(vec!["folder/item".into()]),
            total_bytes: 3,
            ..Default::default()
        }),
        ..Default::default()
    };
    state.transferred_files.insert(
        1,
        InternalFileInfo {
            payload_id: 1,
            file_url: "/private/downloads/item".into(),
            parent_folder: Some("folder".into()),
            bytes_transferred: 0,
            total_size: 3,
            file: None,
        },
    );
    state
}
#[test]
fn given_prepared_files_when_projected_then_only_actual_allowlisted_details_are_exposed() {
    // Arrange
    let mut state = prepared();
    state.transfer_metadata.as_mut().unwrap().text_payload = Some("private clipboard".into());
    state.transfer_metadata.as_mut().unwrap().destination = Some("/private/downloads".into());
    // Act
    let details = TransferDetails::from_state(&state);
    // Assert
    assert_eq!(details.status, DetailsStatus::Ready);
    assert_eq!(details.sender, "Nearby phone");
    assert_eq!(details.pin, "0123");
    assert_eq!(
        details.files.as_ref(),
        vec![FileDetail {
            name: "folder/item".into(),
            size: 3
        }]
    );
    for secret in ["private", "clipboard", "Nearby", "0123", "folder", "item"] {
        assert!(!format!("{details:?}").contains(secret));
        assert!(!format!("{:?}", details.files).contains(secret));
    }
    for (device, expected) in [
        (DeviceType::Unknown, DeviceKind::Unknown),
        (DeviceType::Phone, DeviceKind::Phone),
        (DeviceType::Tablet, DeviceKind::Tablet),
        (DeviceType::Laptop, DeviceKind::Laptop),
    ] {
        state.remote_device_info.as_mut().unwrap().device_type = device;
        assert_eq!(TransferDetails::from_state(&state).device, expected);
    }
    state.state = State::ReceivingFiles;
    state.transfer_metadata.as_mut().unwrap().ack_bytes = 2;
    let progress = TransferDetails::from_state(&state);
    assert_eq!((progress.pin.as_str(), progress.transferred_bytes), ("", 2));
}
#[test]
fn given_invalid_names_sender_or_pin_when_projected_then_no_partial_metadata_is_returned() {
    // Arrange / Act / Assert
    for name in [
        "",
        "/item",
        "../item",
        "folder/./item",
        "folder//item",
        "a\\b",
        "a\n",
        "a\0",
    ] {
        let mut state = prepared();
        state.transferred_files.get_mut(&1).unwrap().parent_folder = Some(name.into());
        if name.is_empty() {
            state.transferred_files.get_mut(&1).unwrap().file_url = "/".into();
        }
        assert_eq!(
            TransferDetails::from_state(&state).status,
            DetailsStatus::Unsupported,
            "{name:?}"
        );
    }
    for pin in ["", "123", "12345", "１２３４", "1a34", "12\n4"] {
        let mut state = prepared();
        state.pin_code = Some(pin.into());
        assert_eq!(
            TransferDetails::from_state(&state),
            TransferDetails::unsupported()
        );
    }
    for name in ["".to_owned(), "a".repeat(257), "peer\n".into()] {
        let mut state = prepared();
        state.remote_device_info.as_mut().unwrap().name = name;
        assert_eq!(
            TransferDetails::from_state(&state),
            TransferDetails::unsupported()
        );
    }
}
#[test]
fn given_size_and_manifest_boundaries_when_projected_then_complete_bounded_files_are_required() {
    // Arrange / Act / Assert
    for count in [1, 255, 256, 257] {
        let mut state = prepared();
        let prototype = state.transferred_files.remove(&1).unwrap();
        for id in 0..count {
            state.transferred_files.insert(
                id,
                InternalFileInfo {
                    payload_id: id,
                    file_url: prototype.file_url.clone(),
                    parent_folder: None,
                    bytes_transferred: 0,
                    total_size: 1,
                    file: None,
                },
            );
        }
        state.transfer_metadata.as_mut().unwrap().total_bytes = count as u64;
        let details = TransferDetails::from_state(&state);
        assert_eq!(
            details.status,
            if count <= 256 {
                DetailsStatus::Ready
            } else {
                DetailsStatus::Unsupported
            }
        );
        assert_eq!(
            details.files.len(),
            if count <= 256 { count as usize } else { 0 }
        );
    }
    for length in [1023, 1024, 1025] {
        let mut state = prepared();
        let file = state.transferred_files.get_mut(&1).unwrap();
        file.parent_folder = None;
        file.file_url = "a".repeat(length).into();
        assert_eq!(
            TransferDetails::from_state(&state).status,
            if length <= NAME_LIMIT {
                DetailsStatus::Ready
            } else {
                DetailsStatus::Unsupported
            }
        );
    }
    for (size, total, progress, expected) in [
        (-1, 3, 0, false),
        (0, 0, 0, true),
        (3, 3, 3, true),
        (3, 3, 4, false),
        (3, 2, 0, false),
    ] {
        let mut state = prepared();
        state.transferred_files.get_mut(&1).unwrap().total_size = size;
        let metadata = state.transfer_metadata.as_mut().unwrap();
        metadata.total_bytes = total;
        metadata.ack_bytes = progress;
        assert_eq!(
            TransferDetails::from_state(&state).status == DetailsStatus::Ready,
            expected
        );
    }
    let mut state = prepared();
    for id in 0..65 {
        state.transferred_files.insert(
            id,
            InternalFileInfo {
                payload_id: id,
                file_url: "a".repeat(1024).into(),
                parent_folder: None,
                bytes_transferred: 0,
                total_size: 1,
                file: None,
            },
        );
    }
    state.transfer_metadata.as_mut().unwrap().total_bytes = 65;
    assert_eq!(
        TransferDetails::from_state(&state),
        TransferDetails::unsupported()
    );
    state.transferred_files.remove(&64);
    state.transfer_metadata.as_mut().unwrap().total_bytes = 64;
    assert_eq!(
        TransferDetails::from_state(&state).status,
        DetailsStatus::Ready
    );
}
#[test]
fn given_unprepared_or_nonfile_payload_when_projected_then_readiness_is_truthful() {
    // Arrange / Act / Assert
    assert_eq!(
        TransferDetails::from_state(&InnerState::default()).status,
        DetailsStatus::Unavailable
    );
    let mut state = prepared();
    state.transferred_files.clear();
    assert_eq!(
        TransferDetails::from_state(&state).status,
        DetailsStatus::Unavailable
    );
    for payload in [
        crate::hdl::TextPayloadInfo::Text(1),
        crate::hdl::TextPayloadInfo::Url(1),
        crate::hdl::TextPayloadInfo::Wifi {
            payload_id: 1,
            ssid: "secret".into(),
            security_type: crate::sharing_nearby::wifi_credentials_metadata::SecurityType::default(
            ),
        },
    ] {
        let mut state = prepared();
        state.text_payload = Some(payload);
        assert_eq!(
            TransferDetails::from_state(&state),
            TransferDetails::unsupported()
        );
    }
}

#[test]
fn given_text_metadata_boundaries_when_projected_then_consent_is_bounded_and_payload_free() {
    // Arrange / Act / Assert
    for (payload, ty, kind) in [
        (
            crate::hdl::TextPayloadInfo::Text(1),
            crate::hdl::TextPayloadType::Text,
            PayloadKind::Text,
        ),
        (
            crate::hdl::TextPayloadInfo::Url(1),
            crate::hdl::TextPayloadType::Url,
            PayloadKind::Url,
        ),
    ] {
        for size in [0, 1, 131072, 131073] {
            let mut state = prepared();
            state.transferred_files.clear();
            state.text_payload = Some(payload.clone());
            let metadata = state.transfer_metadata.as_mut().unwrap();
            metadata.files = None;
            metadata.text_type = Some(ty.clone());
            metadata.total_bytes = size;
            metadata.text_payload = Some("private-body".into());
            let details = TransferDetails::from_state(&state);
            assert_eq!(details.status == DetailsStatus::Ready, size <= 131072);
            assert!(details.files.is_empty());
            assert!(!format!("{details:?}").contains("private-body"));
            if size <= 131072 {
                assert_eq!(details.kind, kind);
                assert_eq!(details.total_bytes, size);
                assert!(details.valid(true));
                let mut changed = details.clone();
                changed.kind = PayloadKind::Files;
                assert!(!details.same_description(&changed));
            }
            state.transfer_metadata.as_mut().unwrap().files = Some(vec!["mixed".into()]);
            assert_eq!(
                TransferDetails::from_state(&state),
                TransferDetails::unsupported()
            );
        }
    }
}
