use std::net::Ipv4Addr;
use std::path::{Path, PathBuf};

use anyhow::anyhow;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use bytes::Bytes;
use get_if_addrs::get_if_addrs;
use hkdf::Hkdf;
use num_bigint::{BigUint, ToBigInt};
use p256::elliptic_curve::rand_core::OsRng;
use p256::{PublicKey, SecretKey};
use rand::{Rng, RngCore};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use tokio::io::{AsyncRead, AsyncReadExt};
use ts_rs::TS;

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize, TS)]
#[ts(export)]
#[allow(dead_code)]
pub enum DeviceType {
    Unknown = 0,
    Phone = 1,
    Tablet = 2,
    Laptop = 3,
}

#[allow(dead_code)]
impl DeviceType {
    pub fn from_raw_value(value: u8) -> Self {
        match value {
            0 => DeviceType::Unknown,
            1 => DeviceType::Phone,
            2 => DeviceType::Tablet,
            3 => DeviceType::Laptop,
            _ => DeviceType::Unknown,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, TS)]
#[ts(export)]
pub struct RemoteDeviceInfo {
    pub name: String,
    pub device_type: DeviceType,
}

impl RemoteDeviceInfo {
    pub fn serialize(&self) -> Vec<u8> {
        // 1 byte: Version(3 bits)|Visibility(1 bit)|Device Type(3 bits)|Reserved(1 bit)
        let mut endpoint_info: Vec<u8> = vec![((self.device_type.clone() as u8) << 1) & 0b111];

        // 16 bytes: unknown random bytes
        endpoint_info.extend((0..16).map(|_| rand::rng().random_range(0..=255)));

        // Device name in UTF-8 prefixed with 1-byte length
        let mut name_chars = self.name.as_bytes().to_vec();
        if name_chars.len() > 255 {
            name_chars.truncate(255);
        }
        endpoint_info.push(name_chars.len() as u8);
        endpoint_info.extend(name_chars);

        endpoint_info
    }
}

pub fn gen_mdns_name(endpoint_id: [u8; 4]) -> String {
    let mut name_b = Vec::new();

    let pcp: [u8; 1] = [0x23];
    name_b.extend_from_slice(&pcp);

    name_b.extend_from_slice(&endpoint_id);

    let service_id: [u8; 3] = [0xFC, 0x9F, 0x5E];
    name_b.extend_from_slice(&service_id);

    let unknown_bytes: [u8; 2] = [0x00, 0x00];
    name_b.extend_from_slice(&unknown_bytes);

    URL_SAFE_NO_PAD.encode(&name_b)
}

pub fn gen_mdns_endpoint_info(device_type: u8, device_name: &str) -> String {
    let mut record = Vec::new();

    // 1 byte: Version(3 bits)|Visibility(1 bit)|Device Type(3 bits)|Reserved(1 bits)
    // Device types: unknown=0, phone=1, tablet=2, laptop=3
    record.push(device_type << 1);

    let unknown_bytes = rand::rng().random::<[u8; 16]>();
    record.extend_from_slice(&unknown_bytes);

    let device_name_bytes = device_name.as_bytes();
    let mut name_len = device_name_bytes.len().min(u8::MAX as usize);
    while !device_name.is_char_boundary(name_len) {
        name_len -= 1;
    }

    record.push(name_len as u8);
    record.extend_from_slice(&device_name_bytes[..name_len]);

    URL_SAFE_NO_PAD.encode(&record)
}

pub fn parse_mdns_endpoint_info(encoded_str: &str) -> Result<(DeviceType, String), anyhow::Error> {
    let decoded_bytes = URL_SAFE_NO_PAD.decode(encoded_str)?;

    // Compact Quick Share records contain only the 17-byte binary prefix.
    if decoded_bytes.len() < 17 {
        return Err(anyhow!(
            "Invalid data length: expected at least 17 bytes, got {}",
            decoded_bytes.len()
        ));
    }

    let device_type = (decoded_bytes[0] >> 1) & 0x7;

    if decoded_bytes.len() == 17 {
        return Ok((DeviceType::from_raw_value(device_type), String::new()));
    }

    // Standard records append a one-byte UTF-8 name length followed by the name.
    let name_length = decoded_bytes[17] as usize;
    let expected_length = 18 + name_length;
    if expected_length > decoded_bytes.len() {
        return Err(anyhow!(
            "Invalid name length: declared {name_length} bytes, payload has {} bytes available",
            decoded_bytes.len().saturating_sub(18)
        ));
    }

    let device_name_bytes = &decoded_bytes[18..expected_length];
    let device_name = String::from_utf8(device_name_bytes.to_vec())?;

    Ok((DeviceType::from_raw_value(device_type), device_name))
}

pub async fn stream_read_exact<S: AsyncRead + Unpin>(
    socket: &mut S,
    buf: &mut [u8],
) -> Result<(), anyhow::Error> {
    match socket.read_exact(buf).await {
        Ok(_) => Ok(()),
        Err(e) => Err(e.into()),
    }
}

pub fn gen_ecdsa_keypair() -> (SecretKey, PublicKey) {
    let secret_key = SecretKey::random(&mut OsRng);
    let public_key = secret_key.public_key();

    (secret_key, public_key)
}

pub fn encode_point(unsigned: Bytes) -> Result<Vec<u8>, anyhow::Error> {
    let big_int = BigUint::from_bytes_be(&unsigned)
        .to_bigint()
        .ok_or_else(|| anyhow!("Failed to convert to bigint"))?;

    Ok(big_int.to_signed_bytes_be())
}

pub fn normalize_p256_coordinate(raw: &[u8]) -> Result<[u8; 32], anyhow::Error> {
    if raw.is_empty() {
        return Err(anyhow!("P-256 coordinate is empty"));
    }

    let unsigned = if raw.len() == 33 {
        if raw[0] != 0 {
            return Err(anyhow!(
                "P-256 coordinate has an invalid 33-byte signed representation"
            ));
        }
        &raw[1..]
    } else if raw.len() <= 32 {
        raw
    } else {
        return Err(anyhow!("P-256 coordinate is too long: {} bytes", raw.len()));
    };

    let mut normalized = [0_u8; 32];
    let offset = 32 - unsigned.len();
    normalized[offset..].copy_from_slice(unsigned);
    Ok(normalized)
}

pub fn hkdf_extract_expand(
    salt: &[u8],
    input: &[u8],
    info: &[u8],
    output_len: usize,
) -> Result<Vec<u8>, anyhow::Error> {
    let hkdf = Hkdf::<Sha256>::new(Some(salt), input);
    let mut okm = vec![0u8; output_len];
    hkdf.expand(info, &mut okm)
        .map_err(|e| anyhow!("HKDF expand failed: {}", e))?;
    Ok(okm)
}

pub fn to_four_digit_string(bytes: &Vec<u8>) -> String {
    let k_hash_modulo = 9973;
    let k_hash_base_multiplier = 31;

    let mut hash = 0;
    let mut multiplier = 1;
    for &byte in bytes {
        let byte = byte as i8 as i32;
        hash = (hash + byte * multiplier) % k_hash_modulo;
        multiplier = (multiplier * k_hash_base_multiplier) % k_hash_modulo;
    }

    format!("{:04}", hash.abs())
}

pub fn gen_random(size: usize) -> Vec<u8> {
    let mut data = vec![0; size];
    rand::rng().fill_bytes(&mut data);

    data
}

pub fn get_download_dir() -> PathBuf {
    if let Some(user_dirs) = directories::UserDirs::new() {
        if let Some(dd) = user_dirs.download_dir() {
            return dd.to_path_buf();
        }

        return user_dirs.home_dir().to_path_buf();
    }

    Path::new("/").to_path_buf()
}

fn is_virtual_interface(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    [
        "docker",
        "veth",
        "br-",
        "virbr",
        "tun",
        "tap",
        "wg",
        "tailscale",
        "warp",
    ]
    .iter()
    .any(|prefix| name.starts_with(prefix))
}

/// Pick a LAN IPv4 suitable for advertising a Wi-Fi bandwidth-upgrade endpoint.
///
/// Virtual/tunnel interfaces are intentionally ignored so a VPN, Docker bridge,
/// or WireGuard-style adapter is not advertised to a nearby Android device.
fn select_lan_ipv4<I>(interfaces: I) -> Option<[u8; 4]>
where
    I: IntoIterator<Item = (String, std::net::IpAddr)>,
{
    let mut fallback = None;

    for (name, address) in interfaces {
        if is_virtual_interface(&name) {
            continue;
        }

        let std::net::IpAddr::V4(ip) = address else {
            continue;
        };
        if ip.is_loopback() || ip.is_link_local() {
            continue;
        }

        if ip.is_private() {
            return Some(ip.octets());
        }

        fallback.get_or_insert(ip.octets());
    }

    fallback
}

pub fn local_lan_ipv4() -> Option<[u8; 4]> {
    let interfaces = get_if_addrs().ok()?;
    select_lan_ipv4(interfaces.into_iter().map(|interface| {
        let address = interface.ip();
        (interface.name, address)
    }))
}

pub fn is_not_self_ip(ip_address: &Ipv4Addr) -> bool {
    if let Ok(if_addrs) = get_if_addrs() {
        for if_addr in if_addrs {
            if if_addr.ip() == *ip_address {
                return false;
            }
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encoded_endpoint(device_type: DeviceType, suffix: &[u8]) -> String {
        let mut payload = vec![(device_type as u8) << 1];
        payload.extend_from_slice(&[0_u8; 16]);
        payload.extend_from_slice(suffix);
        URL_SAFE_NO_PAD.encode(payload)
    }

    #[test]
    fn test_gen_and_parse_mdns_info() {
        let device_name = "a_device_name";
        let device_type = DeviceType::Laptop;

        let info = gen_mdns_endpoint_info(device_type.clone() as u8, device_name);
        let parse_info = parse_mdns_endpoint_info(&info).unwrap();

        assert_eq!(parse_info.1, device_name);
        assert_eq!(parse_info.0, device_type);
    }

    #[test]
    fn test_gen_mdns_info_truncates_multibyte_name_on_utf8_boundary() {
        let info = gen_mdns_endpoint_info(DeviceType::Laptop as u8, &"é".repeat(200));
        let (device_type, device_name) = parse_mdns_endpoint_info(&info).unwrap();

        assert_eq!(device_type, DeviceType::Laptop);
        assert_eq!(device_name.len(), 254);
        assert_eq!(device_name, "é".repeat(127));
    }

    #[test]
    fn test_parse_compact_17_byte_mdns_info() {
        let info = encoded_endpoint(DeviceType::Phone, &[]);
        let parse_info = parse_mdns_endpoint_info(&info).unwrap();

        assert_eq!(parse_info.0, DeviceType::Phone);
        assert_eq!(parse_info.1, "");
    }

    #[test]
    fn test_parse_standard_mdns_info_with_empty_name() {
        let info = encoded_endpoint(DeviceType::Tablet, &[0]);
        let parse_info = parse_mdns_endpoint_info(&info).unwrap();

        assert_eq!(parse_info.0, DeviceType::Tablet);
        assert_eq!(parse_info.1, "");
    }

    #[test]
    fn test_parse_mdns_info_rejects_payload_shorter_than_prefix() {
        let info = URL_SAFE_NO_PAD.encode([0_u8; 16]);
        let err = parse_mdns_endpoint_info(&info).unwrap_err();

        assert!(err.to_string().contains("expected at least 17 bytes"));
    }

    #[test]
    fn test_parse_mdns_info_rejects_truncated_name() {
        let info = encoded_endpoint(DeviceType::Laptop, &[5, b'a']);
        let err = parse_mdns_endpoint_info(&info).unwrap_err();

        assert!(err.to_string().contains("declared 5 bytes"));
    }

    #[test]
    fn test_parse_mdns_info_rejects_invalid_base64() {
        assert!(parse_mdns_endpoint_info("%%%not-base64%%%").is_err());
    }

    #[test]
    fn test_parse_mdns_info_rejects_invalid_utf8_name() {
        let info = encoded_endpoint(DeviceType::Laptop, &[1, 0xff]);

        assert!(parse_mdns_endpoint_info(&info).is_err());
    }

    #[test]
    fn given_remote_mdns_txt_prefix_and_device_bits_when_parsed_then_only_complete_endpoint_data_is_admitted(
    ) {
        // Arrange: a genuine compact TXT prefix with no length/name suffix.
        let mut compact = vec![(DeviceType::Laptop as u8) << 1];
        compact.extend_from_slice(&[0_u8; 16]);

        // Act/Assert: every prefix shorter than 17 bytes is rejected,
        // including the zero-byte and one-byte boundary cases.
        for bytes in 0..17 {
            let encoded = URL_SAFE_NO_PAD.encode(&compact[..bytes]);
            assert!(
                parse_mdns_endpoint_info(&encoded).is_err(),
                "accepted an incomplete {bytes}-byte mDNS identity"
            );
        }

        let decoded = parse_mdns_endpoint_info(&URL_SAFE_NO_PAD.encode(&compact)).unwrap();
        assert_eq!(decoded, (DeviceType::Laptop, String::new()));

        let mut empty_name = compact.clone();
        empty_name.push(0);
        let decoded = parse_mdns_endpoint_info(&URL_SAFE_NO_PAD.encode(&empty_name)).unwrap();
        assert_eq!(decoded, (DeviceType::Laptop, String::new()));

        // The length field is present, but its declared name is absent.
        let mut missing_name = compact;
        missing_name.push(1);
        assert!(parse_mdns_endpoint_info(&URL_SAFE_NO_PAD.encode(&missing_name)).is_err());
        missing_name.push(b'x');
        let decoded = parse_mdns_endpoint_info(&URL_SAFE_NO_PAD.encode(&missing_name)).unwrap();
        assert_eq!(decoded, (DeviceType::Laptop, "x".into()));

        // Reserved bits and unsupported device types cannot bypass the
        // production three-bit type normalization.
        for raw in [0_u8, 2, 4, 6, 8, 14, 16, u8::MAX] {
            let mut txt = vec![raw];
            txt.extend_from_slice(&[0_u8; 16]);
            let parsed = parse_mdns_endpoint_info(&URL_SAFE_NO_PAD.encode(txt)).unwrap();
            assert_eq!(
                parsed,
                (DeviceType::from_raw_value((raw >> 1) & 7), String::new()),
                "incorrect type normalization from raw header {raw}"
            );
        }
    }

    #[test]
    fn given_remote_mdns_txt_name_boundaries_when_decoded_then_truncation_and_invalid_utf8_are_rejected(
    ) {
        // Arrange: remote TXT records may contain up to 255 UTF-8 name bytes,
        // independently of our own stricter local device-name budget.
        let names = [
            String::new(),
            "x".into(),
            "x".repeat(254),
            "x".repeat(255),
            "é".repeat(127),
            format!("{}x", "é".repeat(127)),
            "🧩".repeat(63),
        ];

        for name in names {
            let mut txt = vec![(DeviceType::Phone as u8) << 1];
            txt.extend_from_slice(&[0_u8; 16]);
            txt.push(name.len() as u8);
            txt.extend_from_slice(name.as_bytes());

            // Act/Assert: exact declared boundaries must decode the full name.
            let parsed = parse_mdns_endpoint_info(&URL_SAFE_NO_PAD.encode(&txt)).unwrap();
            assert_eq!(parsed, (DeviceType::Phone, name.clone()));

            // Removing the last byte must never accept a partial name.
            if !name.is_empty() {
                txt.pop();
                assert!(
                    parse_mdns_endpoint_info(&URL_SAFE_NO_PAD.encode(&txt)).is_err(),
                    "accepted truncated name declared as {} bytes",
                    name.len()
                );
            }
        }

        for bytes in [
            vec![0xff_u8],
            vec![0xc3],
            vec![0x80],
            vec![0xc0, 0xaf],
            vec![0xed, 0xa0, 0x80],
        ] {
            let mut txt = vec![(DeviceType::Phone as u8) << 1];
            txt.extend_from_slice(&[0_u8; 16]);
            txt.push(bytes.len() as u8);
            txt.extend(bytes);
            assert!(
                parse_mdns_endpoint_info(&URL_SAFE_NO_PAD.encode(txt)).is_err(),
                "accepted invalid UTF-8 in mDNS TXT identity"
            );
        }
    }

    #[test]
    fn test_normalize_p256_coordinate_left_pads_short_values() {
        let normalized = normalize_p256_coordinate(&[0x12, 0x34]).unwrap();

        assert_eq!(&normalized[..30], &[0_u8; 30]);
        assert_eq!(&normalized[30..], &[0x12, 0x34]);
    }

    #[test]
    fn test_normalize_p256_coordinate_accepts_signed_33_byte_values() {
        let mut raw = vec![0_u8];
        raw.extend([0x80_u8; 32]);

        let normalized = normalize_p256_coordinate(&raw).unwrap();

        assert_eq!(normalized, [0x80_u8; 32]);
    }

    #[test]
    fn test_normalize_p256_coordinate_rejects_invalid_lengths() {
        assert!(normalize_p256_coordinate(&[]).is_err());
        assert!(normalize_p256_coordinate(&[1_u8; 33]).is_err());
        assert!(normalize_p256_coordinate(&[0_u8; 34]).is_err());
    }

    #[test]
    fn virtual_interface_filter_rejects_tunnels() {
        for name in [
            "docker0",
            "veth1234",
            "br-abcd",
            "virbr0",
            "tun0",
            "tap0",
            "wg0",
            "tailscale0",
            "warp0",
        ] {
            assert!(
                is_virtual_interface(name),
                "{name} should be treated as virtual"
            );
        }

        for name in ["wlan0", "wlp3s0", "eth0", "enp4s0"] {
            assert!(
                !is_virtual_interface(name),
                "{name} should be eligible for LAN selection"
            );
        }
    }

    #[test]
    fn virtual_interface_filter_is_case_insensitive() {
        for name in ["Docker0", "Wg0", "TAILSCALE0", "Warp0"] {
            assert!(is_virtual_interface(name));
        }
    }

    #[test]
    fn lan_selector_prefers_private_physical_ipv4() {
        let selected = select_lan_ipv4([
            (
                "docker0".to_owned(),
                "172.17.0.1".parse::<std::net::IpAddr>().unwrap(),
            ),
            (
                "eth0".to_owned(),
                "203.0.113.10".parse::<std::net::IpAddr>().unwrap(),
            ),
            (
                "wlan0".to_owned(),
                "192.168.1.25".parse::<std::net::IpAddr>().unwrap(),
            ),
        ]);

        assert_eq!(selected, Some([192, 168, 1, 25]));
    }

    #[test]
    fn lan_selector_uses_first_public_ipv4_as_fallback() {
        let selected = select_lan_ipv4([
            (
                "eth0".to_owned(),
                "203.0.113.10".parse::<std::net::IpAddr>().unwrap(),
            ),
            (
                "eth1".to_owned(),
                "198.51.100.20".parse::<std::net::IpAddr>().unwrap(),
            ),
        ]);

        assert_eq!(selected, Some([203, 0, 113, 10]));
    }

    #[test]
    fn lan_selector_ignores_ipv6_loopback_link_local_and_tunnels() {
        let selected = select_lan_ipv4([
            (
                "lo".to_owned(),
                "127.0.0.1".parse::<std::net::IpAddr>().unwrap(),
            ),
            (
                "eth0".to_owned(),
                "169.254.10.20".parse::<std::net::IpAddr>().unwrap(),
            ),
            (
                "wlan0".to_owned(),
                "2001:db8::1".parse::<std::net::IpAddr>().unwrap(),
            ),
            (
                "wg0".to_owned(),
                "10.0.0.5".parse::<std::net::IpAddr>().unwrap(),
            ),
        ]);

        assert_eq!(selected, None);
    }
}
