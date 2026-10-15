use std::sync::atomic::{AtomicI64, AtomicUsize, Ordering};

use p256::ecdsa::{signature::Signer, SigningKey};
use p256::pkcs8::EncodePublicKey;

use super::*;

const TOKEN: [u8; 32] = [0x2a; 32];
const SECRET: [u8; 32] = [0x33; 32];

#[derive(Default)]
struct Source {
    peer: Option<CertificateMaterial>,
    local: Vec<CertificateMaterial>,
    failed: bool,
    calls: AtomicUsize,
}

impl CertificateSource for Source {
    fn snapshot(&self) -> Result<Credentials, AuthenticationError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.failed {
            Err(AuthenticationError)
        } else {
            Ok(Credentials {
                peer: self.peer.clone(),
                local: self.local.clone(),
            })
        }
    }
}

struct Time(AtomicI64);
impl Clock for Time {
    fn seconds(&self) -> Option<i64> {
        let now = self.0.load(Ordering::SeqCst);
        (now != i64::MIN).then_some(now)
    }
}

fn material() -> CertificateMaterial {
    CertificateMaterial {
        public_key: key()
            .verifying_key()
            .to_public_key_der()
            .unwrap()
            .as_bytes()
            .to_vec(),
        secret: SECRET.to_vec(),
        not_before: 10_000,
        not_after: 20_000,
    }
}

fn key() -> SigningKey {
    SigningKey::from_slice(&[0x11; 32]).unwrap()
}

fn signature(prefix: u8, token: &[u8; 32], signing_key: &SigningKey) -> Vec<u8> {
    let mut payload = vec![prefix];
    payload.extend_from_slice(token);
    let signature: Signature = signing_key.sign(&payload);
    signature.to_der().as_bytes().to_vec()
}

fn setup(role: Role, peer: bool, local: bool) -> (PeerAuthentication, Arc<Time>) {
    let time = Arc::new(Time(AtomicI64::new(15_000)));
    let source = Arc::new(Source {
        peer: peer.then(material),
        local: if local { vec![material()] } else { vec![] },
        ..Source::default()
    });
    let mut auth = PeerAuthentication::new(role, source, time.clone());
    auth.bind_token(&TOKEN).unwrap();
    (auth, time)
}

fn proof(role: Role) -> PairedKeyEncryptionFrame {
    let certificate = Certificate::parse(&material(), false).unwrap();
    PairedKeyEncryptionFrame {
        signed_data: Some(signature(role.remote_prefix(), &TOKEN, &key())),
        secret_id_hash: Some(certificate.hash(&Secret32(TOKEN)).to_vec()),
        ..Default::default()
    }
}

pub(crate) fn known_fixture(role: Role) -> (PeerAuthentication, PairedKeyEncryptionFrame) {
    let (auth, _) = setup(role, true, true);
    (auth, proof(role))
}

#[test]
fn given_token_boundaries_and_rebinding_when_admitted_then_only_one_full_token_can_be_used() {
    // Arrange / Act / Assert
    for length in [0, 31, 32, 33] {
        let source = Arc::new(Source::default());
        let mut auth = PeerAuthentication::new(
            Role::Sender,
            source.clone(),
            Arc::new(Time(AtomicI64::new(15_000))),
        );
        assert_eq!(auth.bind_token(&vec![42; length]).is_ok(), length == 32);
        assert_eq!(
            source.calls.load(Ordering::SeqCst),
            usize::from(length == 32)
        );
        assert_eq!(auth.bind_token(&TOKEN), Err(AuthenticationError));
        assert_eq!(auth.verify(&proof(Role::Sender)), Outcome::Fail);
        assert!(auth.outgoing_frame().is_err());
        assert!(auth.finish(Some(1)).is_err());
    }
    let mut missing = PeerAuthentication::anonymous(Role::Receiver);
    assert_eq!(missing.verify(&proof(Role::Receiver)), Outcome::Unable);
    assert_eq!(missing.finish(Some(1)), Ok(Outcome::Unable));
}

#[test]
fn given_empty_failed_or_independent_sources_when_verifying_then_no_unverified_success_or_secret_debug_exists(
) {
    for failed in [false, true] {
        // Arrange
        let source = Arc::new(Source {
            failed,
            ..Source::default()
        });
        let mut auth = PeerAuthentication::new(
            Role::Receiver,
            source.clone(),
            Arc::new(Time(AtomicI64::new(15_000))),
        );
        auth.bind_token(&TOKEN).unwrap();
        let (mut known, _) = setup(Role::Receiver, true, true);
        // Act / Assert
        for _ in 0..2 {
            assert_eq!(auth.verify(&proof(Role::Receiver)), Outcome::Unable);
            assert_eq!(auth.finish(Some(1)), Ok(Outcome::Unable));
            assert_eq!(known.verify(&proof(Role::Receiver)), Outcome::Success);
            assert_eq!(known.finish(Some(1)), Ok(Outcome::Success));
        }
        assert_eq!(source.calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            format!("{auth:?}"),
            format!(
                "PeerAuthentication {{ role: Receiver, outcome: Unable, source: {:?}, .. }}",
                if failed {
                    SourceState::Unavailable
                } else {
                    SourceState::Empty
                }
            )
        );
        assert_eq!(
            format!("{known:?}"),
            "PeerAuthentication { role: Receiver, outcome: Success, source: Available, .. }"
        );
        let outgoing = auth.outgoing_frame().unwrap();
        assert_eq!(outgoing.signed_data().len(), 72);
        assert_eq!(outgoing.secret_id_hash().len(), 6);
    }
}

#[test]
fn given_malformed_certificate_material_when_bound_then_invalid_known_identity_cannot_downgrade() {
    let mut variants = vec![];
    for length in [0, 1, 90, 92, 128, 129] {
        let mut value = material();
        value.public_key.resize(length, 0);
        variants.push(value);
    }
    let mut wrong_curve = material();
    wrong_curve.public_key[22] ^= 1;
    variants.push(wrong_curve);
    for length in [0, 31, 33] {
        let mut value = material();
        value.secret.resize(length, 0);
        variants.push(value);
    }
    for (lower, upper) in [(1, 1), (2, 1), (i64::MIN, 1), (1, i64::MAX)] {
        let mut value = material();
        value.not_before = lower;
        value.not_after = upper;
        variants.push(value);
    }
    for bad in variants {
        // Arrange
        let source = Arc::new(Source {
            peer: Some(bad),
            ..Source::default()
        });
        let mut auth = PeerAuthentication::new(
            Role::Receiver,
            source,
            Arc::new(Time(AtomicI64::new(15_000))),
        );
        // Act / Assert
        assert_eq!(auth.bind_token(&TOKEN), Err(AuthenticationError));
        assert_eq!(auth.verify(&proof(Role::Receiver)), Outcome::Fail);
        assert!(auth.finish(Some(1)).is_err());
    }
    for count in [0, 1, 9, 10] {
        for length in [31, 32, 33] {
            let mut local = material();
            local.secret.resize(length, 0);
            let source = Arc::new(Source {
                local: vec![local; count],
                ..Source::default()
            });
            let mut auth = PeerAuthentication::new(
                Role::Sender,
                source,
                Arc::new(Time(AtomicI64::new(15_000))),
            );
            assert_eq!(
                auth.bind_token(&TOKEN).is_ok(),
                count <= 9 && (count == 0 || length == 32)
            );
        }
    }
}

#[test]
fn given_public_and_local_time_boundaries_when_verified_then_inclusive_lower_and_exclusive_upper_hold(
) {
    for now in [
        8199,
        8200,
        8201,
        9999,
        10000,
        19999,
        20000,
        21799,
        21800,
        21801,
        i64::MIN,
    ] {
        // Arrange
        let (mut auth, time) = setup(Role::Receiver, true, true);
        time.0.store(now, Ordering::SeqCst);
        // Act / Assert
        let valid_peer = (8200..21800).contains(&now);
        assert_eq!(
            auth.verify(&proof(Role::Receiver)),
            if valid_peer {
                Outcome::Success
            } else {
                Outcome::Fail
            }
        );
        assert_eq!(
            auth.finish(Some(1)),
            if !valid_peer {
                Err(AuthenticationError)
            } else if (10000..20000).contains(&now) {
                Ok(Outcome::Success)
            } else {
                Ok(Outcome::Unable)
            }
        );
        let (mut outgoing, time) = setup(Role::Receiver, true, true);
        time.0.store(now, Ordering::SeqCst);
        assert_eq!(outgoing.outgoing_frame().is_ok(), valid_peer);
    }
}

#[test]
fn given_both_roles_when_key_token_role_or_der_changes_then_only_actual_matching_signature_verifies(
) {
    for role in [Role::Sender, Role::Receiver] {
        for mutation in [
            "none",
            "role",
            "token",
            "key",
            "empty",
            "truncated",
            "trailing",
            "scalar",
            "oversize",
        ] {
            // Arrange
            let (mut auth, _) = setup(role, true, true);
            let mut frame = proof(role);
            let raw = frame.signed_data.as_mut().unwrap();
            match mutation {
                "role" => *raw = signature(3 - role.remote_prefix(), &TOKEN, &key()),
                "token" => *raw = signature(role.remote_prefix(), &[0x2b; 32], &key()),
                "key" => {
                    *raw = signature(
                        role.remote_prefix(),
                        &TOKEN,
                        &SigningKey::from_slice(&[0x12; 32]).unwrap(),
                    )
                }
                "empty" => raw.clear(),
                "truncated" => {
                    raw.pop();
                }
                "trailing" => raw.push(0),
                "scalar" => *raw = vec![0x30, 6, 2, 1, 0, 2, 1, 0],
                "oversize" => *raw = vec![0; 73],
                _ => (),
            }
            // Act / Assert
            assert_eq!(
                auth.verify(&frame),
                if mutation == "none" {
                    Outcome::Success
                } else {
                    Outcome::Fail
                },
                "{role:?}/{mutation}"
            );
        }
    }
}

#[test]
fn given_primary_and_optional_signature_combinations_when_checked_then_only_same_key_bounded_proofs_work(
) {
    for primary in ["valid", "invalid", "missing", "oversize"] {
        for optional in ["valid", "invalid", "missing", "oversize"] {
            for known in [false, true] {
                // Arrange
                let (mut auth, _) = setup(Role::Receiver, known, true);
                let value = |kind| match kind {
                    "valid" => Some(signature(1, &TOKEN, &key())),
                    "invalid" => Some(vec![0; 72]),
                    "oversize" => Some(vec![0; 73]),
                    _ => None,
                };
                let frame = PairedKeyEncryptionFrame {
                    signed_data: value(primary),
                    optional_signed_data: value(optional),
                    ..proof(Role::Receiver)
                };
                // Act / Assert
                let expected = if primary == "oversize" || optional == "oversize" {
                    Outcome::Fail
                } else if !known {
                    Outcome::Unable
                } else if primary == "valid" || optional == "valid" {
                    Outcome::Success
                } else {
                    Outcome::Fail
                };
                assert_eq!(
                    auth.verify(&frame),
                    expected,
                    "{primary}/{optional}/{known}"
                );
            }
        }
    }
}

#[test]
fn given_independent_openssl_and_hkdf_vectors_when_verified_then_wire_algorithm_matches_reference()
{
    // Arrange: signature came from OpenSSL, expected HKDF from Python stdlib.
    let vector: serde_json::Value = serde_json::from_str(include_str!("test-vector.json")).unwrap();
    let decode = |name| hex::decode(vector[name].as_str().unwrap()).unwrap();
    let token = Secret32::parse(&decode("token_hex")).unwrap();
    let mut cert = material();
    cert.public_key = decode("public_key_spki_hex");
    cert.secret = decode("certificate_secret_hex");
    let certificate = Certificate::parse(&cert, true).unwrap();
    assert_eq!(
        certificate.hash(&token).as_slice(),
        decode("expected_token_hash_hex")
    );
    let source = Arc::new(Source {
        peer: Some(cert.clone()),
        local: vec![cert],
        ..Source::default()
    });
    let mut auth = PeerAuthentication::new(
        Role::Receiver,
        source,
        Arc::new(Time(AtomicI64::new(15_000))),
    );
    auth.bind_token(&token.0).unwrap();
    // Act / Assert
    assert_eq!(
        auth.verify(&PairedKeyEncryptionFrame {
            signed_data: Some(decode("signature_der_hex")),
            secret_id_hash: Some(decode("expected_token_hash_hex")),
            ..Default::default()
        }),
        Outcome::Success
    );
    assert_eq!(auth.finish(Some(1)), Ok(Outcome::Success));
    assert_eq!(
        auth.outgoing_frame().unwrap().secret_id_hash(),
        decode("expected_token_hash_hex")
    );
}

#[test]
fn given_local_hash_presence_lengths_and_mutations_when_checked_then_it_never_replaces_peer_signature(
) {
    for peer in [false, true] {
        for local in [false, true] {
            for change in [
                "none", "wrong", "token", "secret", "missing", "short", "long",
            ] {
                // Arrange
                let (mut auth, _) = setup(Role::Receiver, peer, local);
                let mut frame = proof(Role::Receiver);
                match change {
                    "wrong" => frame.secret_id_hash.as_mut().unwrap()[0] ^= 1,
                    "token" => {
                        frame.secret_id_hash = Some(
                            Certificate::parse(&material(), false)
                                .unwrap()
                                .hash(&Secret32([0x2b; 32]))
                                .to_vec(),
                        )
                    }
                    "secret" => {
                        let mut value = material();
                        value.secret[0] ^= 1;
                        frame.secret_id_hash = Some(
                            Certificate::parse(&value, false)
                                .unwrap()
                                .hash(&Secret32(TOKEN))
                                .to_vec(),
                        );
                    }
                    "missing" => frame.secret_id_hash = None,
                    "short" => frame.secret_id_hash = Some(vec![0; 5]),
                    "long" => frame.secret_id_hash = Some(vec![0; 7]),
                    _ => (),
                }
                // Act / Assert
                let signature = if change == "long" {
                    Outcome::Fail
                } else if peer {
                    Outcome::Success
                } else {
                    Outcome::Unable
                };
                assert_eq!(auth.verify(&frame), signature);
                let expected = if change == "long" {
                    Err(AuthenticationError)
                } else if peer && local && change == "none" {
                    Ok(Outcome::Success)
                } else {
                    Ok(Outcome::Unable)
                };
                assert_eq!(auth.finish(Some(1)), expected);
            }
        }
    }
}

#[test]
fn given_all_local_and_remote_results_when_combined_then_failure_absorbs_and_unable_never_upgrades()
{
    for first in [Outcome::Success, Outcome::Unable, Outcome::Fail] {
        for second in [Outcome::Success, Outcome::Unable, Outcome::Fail] {
            let expected = if first == Outcome::Fail || second == Outcome::Fail {
                Outcome::Fail
            } else if first == Outcome::Unable || second == Outcome::Unable {
                Outcome::Unable
            } else {
                Outcome::Success
            };
            assert_eq!(first.combine(second), expected);
        }
    }
    for peer in [false, true] {
        for local in [false, true] {
            for remote in [
                None,
                Some(-1),
                Some(0),
                Some(1),
                Some(2),
                Some(3),
                Some(4),
                Some(i32::MAX),
            ] {
                // Arrange
                let (mut auth, _) = setup(Role::Receiver, peer, local);
                auth.verify(&proof(Role::Receiver));
                // Act / Assert
                let expected = if remote == Some(2) {
                    Err(AuthenticationError)
                } else if peer && local && remote == Some(1) {
                    Ok(Outcome::Success)
                } else {
                    Ok(Outcome::Unable)
                };
                assert_eq!(auth.finish(remote), expected);
                assert_eq!(auth.finish(remote), expected);
                if expected.is_err() {
                    assert_eq!(auth.finish(Some(1)), Err(AuthenticationError));
                }
            }
        }
    }
    assert_eq!(
        [
            Outcome::Success.wire(),
            Outcome::Fail.wire(),
            Outcome::Unable.wire()
        ],
        [1, 2, 3]
    );
}

#[test]
fn given_exact_conflicting_or_expired_replays_when_processed_then_failure_and_session_identity_cannot_reset(
) {
    // Arrange
    let (mut auth, time) = setup(Role::Receiver, true, true);
    let frame = proof(Role::Receiver);
    // Act / Assert
    for _ in 0..2 {
        assert_eq!(auth.verify(&frame), Outcome::Success);
        assert_eq!(auth.finish(Some(1)), Ok(Outcome::Success));
    }
    time.0.store(21800, Ordering::SeqCst);
    assert_eq!(auth.verify(&frame), Outcome::Fail);
    time.0.store(15000, Ordering::SeqCst);
    assert_eq!(auth.verify(&frame), Outcome::Fail);
    assert!(auth.finish(Some(1)).is_err());
    let (mut auth, _) = setup(Role::Receiver, true, true);
    auth.verify(&frame);
    let mut changed = frame.clone();
    changed.secret_id_hash.as_mut().unwrap()[0] ^= 1;
    assert_eq!(auth.verify(&changed), Outcome::Fail);
    assert_eq!(auth.verify(&frame), Outcome::Fail);
    let (mut auth, _) = setup(Role::Receiver, true, true);
    auth.verify(&frame);
    assert_eq!(auth.finish(Some(1)), Ok(Outcome::Success));
    assert!(auth.finish(Some(3)).is_err());
    let (mut auth, time) = setup(Role::Receiver, true, true);
    auth.verify(&frame);
    time.0.store(20000, Ordering::SeqCst);
    assert_eq!(auth.finish(Some(1)), Ok(Outcome::Unable));
    let (mut auth, time) = setup(Role::Receiver, true, true);
    auth.verify(&frame);
    time.0.store(21800, Ordering::SeqCst);
    assert!(auth.finish(Some(1)).is_err());
}
