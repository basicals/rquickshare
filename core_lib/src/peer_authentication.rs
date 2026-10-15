//! Session-bound PairedKey verification. This module grants no receive permission.

use std::fmt;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use p256::ecdsa::{signature::Verifier, Signature, VerifyingKey};
use p256::elliptic_curve::{subtle::ConstantTimeEq, zeroize::Zeroize};
use p256::pkcs8::DecodePublicKey;
use sha2::{Digest, Sha256};

use crate::sharing_nearby::{paired_key_result_frame::Status, PairedKeyEncryptionFrame};

const MAX_SIGNATURE: usize = 72;
const MAX_PUBLIC_KEY: usize = 128;
const MAX_LOCAL_CERTIFICATES: usize = 9;
const PUBLIC_TOLERANCE_SECONDS: i64 = 30 * 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Role {
    Sender,
    Receiver,
}

impl Role {
    fn remote_prefix(self) -> u8 {
        match self {
            Self::Sender => 2,
            Self::Receiver => 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Outcome {
    Success,
    Unable,
    Fail,
}

impl Outcome {
    fn combine(self, other: Self) -> Self {
        match (self, other) {
            (Self::Fail, _) | (_, Self::Fail) => Self::Fail,
            (Self::Unable, _) | (_, Self::Unable) => Self::Unable,
            _ => Self::Success,
        }
    }

    pub(crate) fn wire(self) -> i32 {
        match self {
            Self::Success => Status::Success,
            Self::Unable => Status::Unable,
            Self::Fail => Status::Fail,
        }
        .into()
    }

    fn remote(raw: Option<i32>) -> Self {
        match raw {
            Some(value) if value == Status::Success as i32 => Self::Success,
            Some(value) if value == Status::Fail as i32 => Self::Fail,
            _ => Self::Unable,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AuthenticationError;

impl fmt::Display for AuthenticationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Peer authentication failed")
    }
}

impl std::error::Error for AuthenticationError {}

struct Secret32([u8; 32]);

impl Secret32 {
    fn parse(raw: &[u8]) -> Result<Self, AuthenticationError> {
        raw.try_into().map(Self).map_err(|_| AuthenticationError)
    }
}

impl Drop for Secret32 {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

/// Trusted provider input, never populated from a peer's CertificateInfo frame.
#[derive(Clone)]
pub(crate) struct CertificateMaterial {
    pub(crate) public_key: Vec<u8>,
    pub(crate) secret: Vec<u8>,
    pub(crate) not_before: i64,
    pub(crate) not_after: i64,
}

impl Drop for CertificateMaterial {
    fn drop(&mut self) {
        self.secret.zeroize();
    }
}

#[derive(Default)]
pub(crate) struct Credentials {
    pub(crate) peer: Option<CertificateMaterial>,
    pub(crate) local: Vec<CertificateMaterial>,
}

pub(crate) trait CertificateSource: Send + Sync {
    fn snapshot(&self) -> Result<Credentials, AuthenticationError>;
}

pub(crate) trait Clock: Send + Sync {
    fn seconds(&self) -> Option<i64>;
}

struct AnonymousSource;

impl CertificateSource for AnonymousSource {
    fn snapshot(&self) -> Result<Credentials, AuthenticationError> {
        Ok(Credentials::default())
    }
}

struct SystemClock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SourceState {
    Unbound,
    Empty,
    Available,
    Unavailable,
}

impl Clock for SystemClock {
    fn seconds(&self) -> Option<i64> {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()?
            .as_secs()
            .try_into()
            .ok()
    }
}

struct Certificate {
    key: Option<VerifyingKey>,
    secret: Secret32,
    lower: i64,
    upper: i64,
}

impl Certificate {
    fn parse(material: &CertificateMaterial, peer: bool) -> Result<Self, AuthenticationError> {
        if material.not_before >= material.not_after || material.public_key.len() > MAX_PUBLIC_KEY {
            return Err(AuthenticationError);
        }
        let tolerance = if peer { PUBLIC_TOLERANCE_SECONDS } else { 0 };
        let lower = material
            .not_before
            .checked_sub(tolerance)
            .ok_or(AuthenticationError)?;
        let upper = material
            .not_after
            .checked_add(tolerance)
            .ok_or(AuthenticationError)?;
        let key = if peer {
            Some(
                VerifyingKey::from_public_key_der(&material.public_key)
                    .map_err(|_| AuthenticationError)?,
            )
        } else {
            None
        };
        Ok(Self {
            key,
            secret: Secret32::parse(&material.secret)?,
            lower,
            upper,
        })
    }

    fn valid(&self, now: i64) -> bool {
        self.lower <= now && now < self.upper
    }

    fn hash(&self, token: &Secret32) -> [u8; 6] {
        // Google Nearby common.cc: HKDF-SHA256(token, certificate secret, empty info, 6).
        let mut output = [0; 6];
        hkdf::Hkdf::<Sha256>::new(Some(&self.secret.0), &token.0)
            .expand(&[], &mut output)
            .expect("Six bytes is within the HKDF-SHA256 output bound");
        output
    }
}

pub(crate) struct PeerAuthentication {
    role: Role,
    source: Arc<dyn CertificateSource>,
    clock: Arc<dyn Clock>,
    token: Option<Secret32>,
    peer: Option<Certificate>,
    local: Vec<Certificate>,
    proof_digest: Option<[u8; 32]>,
    received_hash: Option<[u8; 6]>,
    signature: Option<Outcome>,
    combined: Outcome,
    remote_result: Option<Option<i32>>,
    source_state: SourceState,
}

impl fmt::Debug for PeerAuthentication {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PeerAuthentication")
            .field("role", &self.role)
            .field("outcome", &self.combined)
            .field("source", &self.source_state)
            .finish_non_exhaustive()
    }
}

impl PeerAuthentication {
    pub(crate) fn anonymous(role: Role) -> Self {
        Self::new(role, Arc::new(AnonymousSource), Arc::new(SystemClock))
    }

    pub(crate) fn new(
        role: Role,
        source: Arc<dyn CertificateSource>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            role,
            source,
            clock,
            token: None,
            peer: None,
            local: Vec::new(),
            proof_digest: None,
            received_hash: None,
            signature: None,
            combined: Outcome::Success,
            remote_result: None,
            source_state: SourceState::Unbound,
        }
    }

    pub(crate) fn bind_token(&mut self, bytes: &[u8]) -> Result<(), AuthenticationError> {
        if self.token.is_some() || self.combined == Outcome::Fail {
            return self.reject();
        }
        let token = match Secret32::parse(bytes) {
            Ok(token) => token,
            Err(_) => return self.reject(),
        };
        self.token = Some(token);
        let material = match self.source.snapshot() {
            Ok(material) => material,
            Err(_) => {
                self.source_state = SourceState::Unavailable;
                self.combined = Outcome::Unable;
                return Ok(());
            }
        };
        self.source_state = if material.peer.is_none() && material.local.is_empty() {
            SourceState::Empty
        } else {
            SourceState::Available
        };
        if material.local.len() > MAX_LOCAL_CERTIFICATES {
            return self.reject();
        }
        let peer = material
            .peer
            .as_ref()
            .map(|peer| Certificate::parse(peer, true))
            .transpose();
        let local = material
            .local
            .iter()
            .map(|local| Certificate::parse(local, false))
            .collect::<Result<Vec<_>, _>>();
        match (peer, local) {
            (Ok(peer), Ok(local)) => {
                self.peer = peer;
                self.local = local;
                Ok(())
            }
            _ => self.reject(),
        }
    }

    fn reject<T>(&mut self) -> Result<T, AuthenticationError> {
        self.combined = Outcome::Fail;
        Err(AuthenticationError)
    }

    pub(crate) fn outgoing_frame(
        &mut self,
    ) -> Result<PairedKeyEncryptionFrame, AuthenticationError> {
        if self.combined == Outcome::Fail {
            return Err(AuthenticationError);
        }
        let hash = match (&self.token, &self.peer) {
            (Some(token), Some(peer)) => {
                if !self.clock.seconds().is_some_and(|now| peer.valid(now)) {
                    return self.reject();
                }
                peer.hash(token).to_vec()
            }
            _ => crate::utils::gen_random(6),
        };
        Ok(PairedKeyEncryptionFrame {
            signed_data: Some(crate::utils::gen_random(72)),
            secret_id_hash: Some(hash),
            ..Default::default()
        })
    }

    pub(crate) fn verify(&mut self, frame: &PairedKeyEncryptionFrame) -> Outcome {
        if self.combined == Outcome::Fail {
            return Outcome::Fail;
        }
        let fields = [
            frame.signed_data(),
            frame.optional_signed_data(),
            frame.secret_id_hash(),
        ];
        if fields[0].len() > MAX_SIGNATURE || fields[1].len() > MAX_SIGNATURE || fields[2].len() > 6
        {
            self.combined = Outcome::Fail;
            return Outcome::Fail;
        }
        let mut digest = Sha256::new();
        for field in fields {
            digest.update((field.len() as u64).to_be_bytes());
            digest.update(field);
        }
        let digest: [u8; 32] = digest.finalize().into();
        if self.proof_digest.is_some_and(|previous| previous != digest) {
            self.combined = Outcome::Fail;
            return Outcome::Fail;
        }
        self.proof_digest = Some(digest);
        self.received_hash = fields[2].try_into().ok();
        let (signature, hash) = match self.token.as_ref() {
            None => (Outcome::Unable, Outcome::Unable),
            Some(token) => {
                let now = self.clock.seconds();
                let signature = match self.peer.as_ref() {
                    None => Outcome::Unable,
                    Some(peer) => {
                        if !now.is_some_and(|now| peer.valid(now)) {
                            Outcome::Fail
                        } else {
                            let mut payload = [0; 33];
                            payload[0] = self.role.remote_prefix();
                            payload[1..].copy_from_slice(&token.0);
                            let valid = fields[..2].iter().any(|raw| {
                                Signature::from_der(raw).ok().is_some_and(|signature| {
                                    peer.key
                                        .as_ref()
                                        .is_some_and(|key| key.verify(&payload, &signature).is_ok())
                                })
                            });
                            payload.zeroize();
                            if valid {
                                Outcome::Success
                            } else {
                                Outcome::Fail
                            }
                        }
                    }
                };
                let matched = now.is_some_and(|now| {
                    self.local.iter().any(|certificate| {
                        certificate.valid(now)
                            && bool::from(certificate.hash(token).as_slice().ct_eq(fields[2]))
                    })
                });
                (
                    signature,
                    if matched {
                        Outcome::Success
                    } else {
                        Outcome::Unable
                    },
                )
            }
        };
        self.signature = Some(
            self.signature
                .map_or(signature, |previous| previous.combine(signature)),
        );
        self.combined = self.combined.combine(signature).combine(hash);
        if self.combined == Outcome::Fail {
            Outcome::Fail
        } else {
            signature
        }
    }

    pub(crate) fn finish(&mut self, remote: Option<i32>) -> Result<Outcome, AuthenticationError> {
        if self
            .remote_result
            .is_some_and(|previous| previous != remote)
        {
            return self.reject();
        }
        self.remote_result = Some(remote);
        let now = self.clock.seconds();
        if let Some(peer) = &self.peer {
            if !now.is_some_and(|now| peer.valid(now)) {
                return self.reject();
            }
        }
        let local_still_valid = match (&self.token, &self.received_hash, now) {
            (Some(token), Some(hash), Some(now)) => self.local.iter().any(|certificate| {
                certificate.valid(now) && bool::from(certificate.hash(token).ct_eq(hash))
            }),
            _ => false,
        };
        if !local_still_valid {
            self.combined = self.combined.combine(Outcome::Unable);
        }
        self.combined = self
            .combined
            .combine(self.signature.unwrap_or(Outcome::Unable))
            .combine(Outcome::remote(remote));
        if self.combined == Outcome::Fail {
            Err(AuthenticationError)
        } else {
            Ok(self.combined)
        }
    }
}

#[cfg(test)]
pub(crate) mod tests;
