//! Nearby V2 legacy GATT header, built by the owner of the exact slot bytes.
//! Layout/hash rules: google/nearby connections/implementation/mediums/ble.
use rand::TryRngCore;
use sha2::{Digest, Sha256};

pub(super) type Header = [u8; 17];
const SERVICE: &[u8] = b"NearbySharing";

fn mix(mut value: u64) -> u64 {
    value ^= value >> 33;
    value = value.wrapping_mul(0xff51afd7ed558ccd);
    value ^= value >> 33;
    value = value.wrapping_mul(0xc4ceb9fe1a85ec53);
    value ^ (value >> 33)
}
// MurmurHash3 x64/128, seed zero; only the low64 are used by Nearby's bloom.
fn low64(bytes: &[u8]) -> u64 {
    const C1: u64 = 0x87c37b91114253d5;
    const C2: u64 = 0x4cf5ad432745937f;
    let (mut h1, mut h2) = (0u64, 0u64);
    let (chunks, tail) = bytes.as_chunks::<16>();
    for chunk in chunks {
        let mut k1 = u64::from_le_bytes(chunk[..8].try_into().expect("fixed block"));
        let mut k2 = u64::from_le_bytes(chunk[8..].try_into().expect("fixed block"));
        k1 = k1.wrapping_mul(C1).rotate_left(31).wrapping_mul(C2);
        h1 ^= k1;
        h1 = h1
            .rotate_left(27)
            .wrapping_add(h2)
            .wrapping_mul(5)
            .wrapping_add(0x52dce729);
        k2 = k2.wrapping_mul(C2).rotate_left(33).wrapping_mul(C1);
        h2 ^= k2;
        h2 = h2
            .rotate_left(31)
            .wrapping_add(h1)
            .wrapping_mul(5)
            .wrapping_add(0x38495ab5);
    }
    let (mut k1, mut k2) = (0u64, 0u64);
    for (i, byte) in tail.iter().take(8).enumerate() {
        k1 |= u64::from(*byte) << (8 * i);
    }
    for (i, byte) in tail.iter().skip(8).enumerate() {
        k2 |= u64::from(*byte) << (8 * i);
    }
    if tail.len() > 8 {
        h2 ^= k2.wrapping_mul(C2).rotate_left(33).wrapping_mul(C1);
    }
    if !tail.is_empty() {
        h1 ^= k1.wrapping_mul(C1).rotate_left(31).wrapping_mul(C2);
    }
    h1 ^= bytes.len() as u64;
    h2 ^= bytes.len() as u64;
    h1 = h1.wrapping_add(h2);
    h2 = h2.wrapping_add(h1);
    mix(h1).wrapping_add(mix(h2))
}
fn add(bloom: &mut [u8], bytes: &[u8]) {
    let hash = low64(bytes);
    let (a, b) = (hash as i32, (hash >> 32) as i32);
    for i in 1i32..=5 {
        let hash = a.wrapping_add(i.wrapping_mul(b));
        let hash = if hash < 0 { !hash } else { hash } as usize;
        let bit = hash % 80;
        bloom[bit / 8] |= 1 << (bit % 8);
    }
}
fn build(slot: &[u8], salt: &[u8; 128]) -> Header {
    let mut header = [0; 17];
    header[0] = 0x41; // V2, no extended advertisement, one GATT slot, PSM zero.
    add(&mut header[1..11], salt);
    add(&mut header[1..11], SERVICE);
    let seed = Sha256::digest(salt);
    let mut hash = Sha256::new();
    hash.update(&seed[..4]);
    hash.update(slot);
    header[11..15].copy_from_slice(&hash.finalize()[..4]);
    header
}
pub(super) fn new(slot: &[u8]) -> std::io::Result<Header> {
    let mut salt = [0; 128];
    rand::rngs::OsRng
        .try_fill_bytes(&mut salt)
        .map_err(|_| std::io::Error::other("Receiver header entropy unavailable"))?;
    Ok(build(slot, &salt))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn given_independent_cpp_vectors_when_hashed_then_all_blocks_tails_and_nearby_bloom_bits_match()
    {
        // Oracle: public-domain aappleby/smhasher MurmurHash3.cpp x64_128, seed0.
        let vectors: serde_json::Value =
            serde_json::from_str(include_str!("receiver_header-vectors.json")).unwrap();
        for row in vectors.as_array().unwrap() {
            let input = hex::decode(row["input"].as_str().unwrap()).unwrap();
            assert_eq!(low64(&input), row["low"].as_u64().unwrap());
            let mut bloom = [0; 10];
            add(&mut bloom, &input);
            assert_eq!(hex::encode(bloom), row["bloom"].as_str().unwrap());
        }
    }
    #[test]
    fn given_slot_and_salt_when_header_is_built_then_legacy_layout_service_and_cache_hash_are_stable_and_bounded(
    ) {
        let salt = [7; 128];
        let header = build(b"current slot", &salt);
        assert_eq!(header[0], 0x41);
        assert_eq!(&header[15..], &[0, 0]);
        assert!(3 + 4 + 2 + 2 + header.len() <= 31); // Flags, UUID AD, service-data AD.
        assert_eq!(header, build(b"current slot", &salt));
        assert_ne!(&header[11..15], &build(b"changed slot", &salt)[11..15]);
        assert_ne!(header, build(b"current slot", &[8; 128]));
        let mut service = [0; 10];
        add(&mut service, SERVICE);
        for (actual, required) in header[1..11].iter().zip(service) {
            assert_eq!(actual & required, required);
        }
        assert_eq!(&Sha256::digest(SERVICE)[..3], &[0xfc, 0x9f, 0x5e]);
    }
}
