use serde_json::json;

use super::*;

#[test]
fn given_existing_settings_when_parsed_then_valid_preferences_and_fresh_defaults_are_preserved() {
    // Arrange / Act / Assert
    assert_eq!(
        StartupSettings::parse(b"{}").unwrap(),
        StartupSettings::default()
    );
    for (raw, expected) in [
        (0, Visibility::Visible),
        (1, Visibility::Invisible),
        (2, Visibility::Temporarily),
    ] {
        let bytes = serde_json::to_vec(&json!({"visibility": raw, "port": 32100, "download_path": "/tmp/Received files", "autostart": true, "unknown": 5})).unwrap();
        let settings = StartupSettings::parse(&bytes).unwrap();
        assert_eq!(
            settings,
            StartupSettings {
                visibility: expected,
                visibility_recovered: false,
                port: Some(32100),
                download_path: Some("/tmp/Received files".into()),
                device_name: None
            }
        );
        assert_eq!(Visibility::from_raw_value(raw), expected);
    }
}

#[test]
fn given_invalid_visibility_when_parsed_then_hidden_recovery_never_discards_other_preferences() {
    // Arrange / Act / Assert
    for raw in [
        json!(null),
        json!(true),
        json!("Visible"),
        json!(-1),
        json!(0.5),
        json!(3),
        json!(u64::MAX),
        json!([]),
        json!({}),
    ] {
        let bytes = serde_json::to_vec(
            &json!({"visibility": raw, "port": 12345, "download_path": "relative/received"}),
        )
        .unwrap();
        let settings = StartupSettings::parse(&bytes).unwrap();
        assert_eq!(settings.visibility, Visibility::Invisible);
        assert!(settings.visibility_recovered);
        assert_eq!(settings.port, Some(12345));
        assert_eq!(settings.download_path, Some("relative/received".into()));
    }
    for raw in [3, 255, u64::MAX] {
        assert_eq!(Visibility::try_from_raw_value(raw), None);
        assert_eq!(Visibility::from_raw_value(raw), Visibility::Invisible);
    }
    assert!(
        StartupSettings::parse(br#"{"visibility":18446744073709551616}"#)
            .unwrap()
            .visibility_recovered
    );
}

#[test]
fn given_unusable_whole_settings_when_parsed_then_startup_is_rejected_instead_of_reset() {
    // Arrange / Act / Assert
    for bytes in [
        b"".as_slice(),
        b"{broken",
        b"null",
        b"[]",
        b"false",
        b"\xff",
    ] {
        assert_eq!(StartupSettings::parse(bytes), Err(SettingsError::Format));
    }
    let mut exact = b"{}".to_vec();
    exact.resize(MAX_SETTINGS_BYTES, b' ');
    assert_eq!(
        StartupSettings::parse(&exact).unwrap(),
        StartupSettings::default()
    );
    exact.push(b' ');
    assert_eq!(StartupSettings::parse(&exact), Err(SettingsError::Capacity));
}

#[test]
fn given_optional_transport_settings_when_parsed_then_invalid_values_are_rejected_and_boundaries_preserved(
) {
    // Arrange / Act / Assert
    for port in [0, 1, 1023, 1024, 65535] {
        assert_eq!(
            StartupSettings::parse(&serde_json::to_vec(&json!({"port": port})).unwrap())
                .unwrap()
                .port,
            Some(port)
        );
    }
    for port in [
        json!(-1),
        json!(65536),
        json!(u64::MAX),
        json!(0.5),
        json!("1234"),
        json!(true),
    ] {
        assert_eq!(
            StartupSettings::parse(&serde_json::to_vec(&json!({"port": port})).unwrap()),
            Err(SettingsError::Port)
        );
    }
    assert_eq!(
        StartupSettings::parse(br#"{"port":null,"download_path":null}"#).unwrap(),
        StartupSettings::default()
    );
    for path in [
        json!(""),
        json!("bad\0path"),
        json!(1),
        json!(true),
        json!([]),
    ] {
        assert_eq!(
            StartupSettings::parse(&serde_json::to_vec(&json!({"download_path": path})).unwrap()),
            Err(SettingsError::DownloadPath)
        );
    }
}

#[test]
fn given_existing_or_missing_file_when_read_then_bytes_are_unchanged_and_absent_means_fresh_install(
) {
    // Arrange
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join(".settings.json");
    assert_eq!(
        StartupSettings::read(&path).unwrap(),
        StartupSettings::default()
    );
    assert!(!path.exists());
    for bytes in [
        br#"{"visibility":42,"port":1234,"unknown":"keep"}"#.as_slice(),
        b"broken",
    ] {
        std::fs::write(&path, bytes).unwrap();
        // Act
        let result = StartupSettings::read(&path);
        // Assert
        if bytes[0] == b'{' {
            assert!(result.unwrap().visibility_recovered);
        } else {
            assert_eq!(result, Err(SettingsError::Format));
        }
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
    }
    assert_eq!(
        StartupSettings::read(directory.path()),
        Err(SettingsError::Read)
    );
    std::fs::write(&path, vec![b' '; MAX_SETTINGS_BYTES + 1]).unwrap();
    assert_eq!(StartupSettings::read(&path), Err(SettingsError::Capacity));
}

#[cfg(target_os = "linux")]
#[test]
fn given_fifo_settings_when_opened_then_nonblocking_type_check_rejects_without_a_producer() {
    // Arrange
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("fifo");
    rustix::fs::mkfifoat(
        rustix::fs::CWD,
        &path,
        rustix::fs::Mode::RUSR | rustix::fs::Mode::WUSR,
    )
    .unwrap();
    // Act / Assert
    assert_eq!(StartupSettings::read(&path), Err(SettingsError::Read));
    assert!(path.exists());
    let broken = directory.path().join("broken-link");
    std::os::unix::fs::symlink(directory.path().join("missing"), &broken).unwrap();
    assert_eq!(StartupSettings::read(&broken), Err(SettingsError::Read));
}
