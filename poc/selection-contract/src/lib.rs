//! Allocation-free approval wire contract shared by host and Microkit adapters.
//! A selection is evidence to verify, never an authorization token.
#![no_std]
#![forbid(unsafe_code)]

use ed25519_dalek::{Signature, VerifyingKey};
use sha2::{Digest, Sha256};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerificationError {
    PlanTooLarge,
    WrongSession,
    StaleRevision,
    PlanChanged,
    MissingAttestation,
    InvalidAttestation,
}

/// Cryptographic conformance only: policy, trusted display, expiry and durable
/// replay consumption must be enforced by the caller before granting authority.
pub fn verify_selection(
    selection: &UserSelection,
    plan: &[u8],
    expected_session: &[u8; 32],
    expected_revision: u64,
    public_key: &[u8; 32],
) -> Result<(), VerificationError> {
    if plan.len() > MAX_PLAN_BYTES {
        return Err(VerificationError::PlanTooLarge);
    }
    if selection.session_binding != *expected_session {
        return Err(VerificationError::WrongSession);
    }
    if selection.revision != expected_revision {
        return Err(VerificationError::StaleRevision);
    }
    let digest: [u8; 32] = Sha256::digest(plan).into();
    if digest != selection.plan_sha256 {
        return Err(VerificationError::PlanChanged);
    }
    if selection.signature == [0; 64] {
        return Err(VerificationError::MissingAttestation);
    }
    let key =
        VerifyingKey::from_bytes(public_key).map_err(|_| VerificationError::InvalidAttestation)?;
    if key.is_weak() {
        return Err(VerificationError::InvalidAttestation);
    }
    key.verify_strict(
        &selection.signing_bytes(),
        &Signature::from_bytes(&selection.signature),
    )
    .map_err(|_| VerificationError::InvalidAttestation)
}

pub const MAX_PLAN_BYTES: usize = 4096;
pub const VERSION: u16 = 1;
pub const DOMAIN: &[u8] = b"VelaOS.Glass.UserSelection.v1\0";
pub const SIGNING_SIZE: usize = DOMAIN.len() + 2 + 8 + 32 + 8 + 32;
pub const WIRE_SIZE: usize = 2 + 32 + 8 + 32 + 64;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UserSelection {
    pub session_binding: [u8; 32],
    pub revision: u64,
    pub plan_sha256: [u8; 32],
    pub signature: [u8; 64],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecodeError {
    Length,
    Version,
}

impl UserSelection {
    pub fn signing_bytes(&self) -> [u8; SIGNING_SIZE] {
        let mut result = [0; SIGNING_SIZE];
        let mut offset = 0;
        for part in [
            DOMAIN,
            &VERSION.to_le_bytes(),
            b"approve\0",
            &self.session_binding,
            &self.revision.to_le_bytes(),
            &self.plan_sha256,
        ] {
            result[offset..offset + part.len()].copy_from_slice(part);
            offset += part.len();
        }
        result
    }

    pub fn encode(&self) -> [u8; WIRE_SIZE] {
        let mut result = [0; WIRE_SIZE];
        result[..2].copy_from_slice(&VERSION.to_le_bytes());
        result[2..34].copy_from_slice(&self.session_binding);
        result[34..42].copy_from_slice(&self.revision.to_le_bytes());
        result[42..74].copy_from_slice(&self.plan_sha256);
        result[74..].copy_from_slice(&self.signature);
        result
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        if bytes.len() != WIRE_SIZE {
            return Err(DecodeError::Length);
        }
        if bytes[..2] != VERSION.to_le_bytes() {
            return Err(DecodeError::Version);
        }
        let mut selection = Self {
            session_binding: [0; 32],
            revision: 0,
            plan_sha256: [0; 32],
            signature: [0; 64],
        };
        selection.session_binding.copy_from_slice(&bytes[2..34]);
        let mut revision = [0; 8];
        revision.copy_from_slice(&bytes[34..42]);
        selection.revision = u64::from_le_bytes(revision);
        selection.plan_sha256.copy_from_slice(&bytes[42..74]);
        selection.signature.copy_from_slice(&bytes[74..]);
        Ok(selection)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_compact::{KeyPair, Seed};

    #[test]
    fn compact_signature_is_verified_by_no_std_dalek() {
        let key = KeyPair::from_seed(Seed::new([0x5a; 32]));
        let plan = b"exact note bytes";
        let mut selected = selection();
        selected.plan_sha256 = Sha256::digest(plan).into();
        let signature = key.sk.sign(selected.signing_bytes(), None);
        selected.signature.copy_from_slice(signature.as_ref());
        let mut public = [0; 32];
        public.copy_from_slice(key.pk.as_ref());
        assert_eq!(
            verify_selection(
                &selected,
                plan,
                &selected.session_binding,
                selected.revision,
                &public
            ),
            Ok(())
        );
        let mut altered = selected.clone();
        altered.signature[0] ^= 1;
        assert_eq!(
            verify_selection(
                &altered,
                plan,
                &selected.session_binding,
                selected.revision,
                &public
            ),
            Err(VerificationError::InvalidAttestation)
        );
        assert_eq!(
            verify_selection(
                &selected,
                b"other bytes",
                &selected.session_binding,
                selected.revision,
                &public
            ),
            Err(VerificationError::PlanChanged)
        );
        assert_eq!(
            verify_selection(&selected, plan, &[0; 32], selected.revision, &public),
            Err(VerificationError::WrongSession)
        );
        assert_eq!(
            verify_selection(
                &selected,
                plan,
                &selected.session_binding,
                selected.revision + 1,
                &public
            ),
            Err(VerificationError::StaleRevision)
        );
    }
    fn selection() -> UserSelection {
        UserSelection {
            session_binding: [3; 32],
            revision: 0x0102030405060708,
            plan_sha256: [5; 32],
            signature: [7; 64],
        }
    }
    #[test]
    fn round_trip_and_explicit_little_endian() {
        let value = selection();
        let wire = value.encode();
        assert_eq!(&wire[34..42], &[8, 7, 6, 5, 4, 3, 2, 1]);
        assert_eq!(UserSelection::decode(&wire), Ok(value));
    }
    #[test]
    fn rejects_truncation_extension_and_unknown_version() {
        let mut wire = selection().encode();
        for length in 0..WIRE_SIZE {
            assert_eq!(
                UserSelection::decode(&wire[..length]),
                Err(DecodeError::Length)
            );
        }
        assert_eq!(
            UserSelection::decode(&[0; WIRE_SIZE + 1]),
            Err(DecodeError::Length)
        );
        wire[0] = 2;
        assert_eq!(UserSelection::decode(&wire), Err(DecodeError::Version));
    }
    #[test]
    fn signed_context_changes_but_signature_is_excluded() {
        let base = selection();
        let mut changed = base.clone();
        changed.signature[0] ^= 1;
        assert_eq!(base.signing_bytes(), changed.signing_bytes());
        changed.revision += 1;
        assert_ne!(base.signing_bytes(), changed.signing_bytes());
        changed = base.clone();
        changed.session_binding[0] ^= 1;
        assert_ne!(base.signing_bytes(), changed.signing_bytes());
        changed = base.clone();
        changed.plan_sha256[0] ^= 1;
        assert_ne!(base.signing_bytes(), changed.signing_bytes());
    }
}
