//! Signed, exact-plan user selections crossing from Glass to the PCB.
//!
//! The browser is not a trust boundary. A native trusted-input provider must
//! attest the displayed plan digest; the host preview intentionally has none.
use ed25519_compact::PublicKey;
use sha2::{Digest, Sha256};

pub use vela_selection_contract::{MAX_PLAN_BYTES, UserSelection};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SelectionDenial {
    PlanTooLarge,
    PlanChanged,
    StaleRevision,
    WrongSession,
    MissingAttestation,
    InvalidAttestation,
    PcbUnavailable,
}

impl SelectionDenial {
    pub fn code(&self) -> &'static str {
        match self {
            Self::PlanTooLarge => "PLAN_TOO_LARGE",
            Self::PlanChanged => "PLAN_CHANGED",
            Self::StaleRevision => "STALE_REVISION",
            Self::WrongSession => "WRONG_SESSION",
            Self::MissingAttestation => "MISSING_ATTESTATION",
            Self::InvalidAttestation => "INVALID_ATTESTATION",
            Self::PcbUnavailable => "PCB_UNAVAILABLE",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PcbReply {
    SelectionForwarded { request_id: u64 },
    Denied(SelectionDenial),
}

/// Host-to-PCB seam. Implementations must forward the signed event and exact
/// plan bytes to the PCB; they must not mint authority in the Glass process.
pub trait TrustedSelectionChannel {
    fn request_token(&mut self, selection: &UserSelection, exact_plan_bytes: &[u8]) -> PcbReply;
}

/// Default for the web preview until a native trusted-input/PCB transport exists.
#[derive(Default)]
pub struct DisconnectedPcbChannel;

impl TrustedSelectionChannel for DisconnectedPcbChannel {
    fn request_token(&mut self, selection: &UserSelection, _exact_plan_bytes: &[u8]) -> PcbReply {
        if selection.signature == [0; 64] {
            PcbReply::Denied(SelectionDenial::MissingAttestation)
        } else {
            PcbReply::Denied(SelectionDenial::PcbUnavailable)
        }
    }
}

pub fn plan_sha256(exact_plan_bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(exact_plan_bytes).into()
}

/// PCB-side contract verifier used by conformance tests and guest integration.
/// Call before creating any policy selection or requesting a token.
pub fn verify_user_selection(
    selection: &UserSelection,
    exact_plan_bytes: &[u8],
    expected_session_binding: &[u8; 32],
    expected_revision: u64,
    trusted_input_key: &PublicKey,
) -> Result<(), SelectionDenial> {
    use vela_selection_contract::{VerificationError, verify_selection};
    let mut key = [0; 32];
    key.copy_from_slice(trusted_input_key.as_ref());
    verify_selection(selection, exact_plan_bytes, expected_session_binding,
        expected_revision, &key).map_err(|error| match error {
        VerificationError::PlanTooLarge => SelectionDenial::PlanTooLarge,
        VerificationError::WrongSession => SelectionDenial::WrongSession,
        VerificationError::StaleRevision => SelectionDenial::StaleRevision,
        VerificationError::PlanChanged => SelectionDenial::PlanChanged,
        VerificationError::MissingAttestation => SelectionDenial::MissingAttestation,
        VerificationError::InvalidAttestation => SelectionDenial::InvalidAttestation,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HexError;
pub fn decode_hex<const N: usize>(hex: &str) -> Result<[u8; N], HexError> {
    if hex.len() != N * 2 {
        return Err(HexError);
    }
    let mut bytes = [0; N];
    for (index, pair) in hex.as_bytes().chunks_exact(2).enumerate() {
        let high = from_hex(pair[0]).ok_or(HexError)?;
        let low = from_hex(pair[1]).ok_or(HexError)?;
        bytes[index] = high << 4 | low;
    }
    Ok(bytes)
}

pub fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(HEX[(byte >> 4) as usize] as char);
        encoded.push(HEX[(byte & 0x0f) as usize] as char);
    }
    encoded
}

pub fn decode_variable_hex(hex: &str) -> Result<Vec<u8>, HexError> {
    if hex.len() & 1 != 0 || hex.len() > MAX_PLAN_BYTES * 2 {
        return Err(HexError);
    }
    hex.as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let high = from_hex(pair[0]).ok_or(HexError)?;
            let low = from_hex(pair[1]).ok_or(HexError)?;
            Ok(high << 4 | low)
        })
        .collect()
}

fn from_hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_compact::{KeyPair, Seed};

    fn signed_selection(plan: &[u8], revision: u64, session: [u8; 32]) -> (KeyPair, UserSelection) {
        let key = KeyPair::from_seed(Seed::new([0x5a; 32]));
        let mut selection = UserSelection {
            session_binding: session,
            revision,
            plan_sha256: plan_sha256(plan),
            signature: [0; 64],
        };
        let signature = key.sk.sign(selection.signing_bytes(), None);
        selection.signature.copy_from_slice(signature.as_ref());
        (key, selection)
    }

    #[test]
    fn pcb_accepts_only_the_exact_signed_plan_and_context() {
        let plan = br#"["VelaOS plan v1","Create a note","draft bytes"]"#;
        let session = [0x31; 32];
        let (key, selection) = signed_selection(plan, 9, session);
        assert_eq!(
            verify_user_selection(&selection, plan, &session, 9, &key.pk),
            Ok(())
        );
    }

    #[test]
    fn pcb_denies_hash_or_plan_tampering_after_the_click() {
        let plan = br#"["VelaOS plan v1","Create a note","draft bytes"]"#;
        let session = [0x31; 32];
        let (key, selection) = signed_selection(plan, 9, session);

        let mut changed_hash = selection.clone();
        changed_hash.plan_sha256[0] ^= 1;
        assert_eq!(
            verify_user_selection(&changed_hash, plan, &session, 9, &key.pk),
            Err(SelectionDenial::PlanChanged)
        );

        let changed_plan = br#"["VelaOS plan v1","Create a note","different draft"]"#;
        assert_eq!(
            verify_user_selection(&selection, changed_plan, &session, 9, &key.pk),
            Err(SelectionDenial::PlanChanged)
        );
    }

    #[test]
    fn pcb_denies_wrong_key_session_and_revision() {
        let plan = b"exact plan";
        let session = [0x31; 32];
        let (key, selection) = signed_selection(plan, 9, session);
        let other_key = KeyPair::from_seed(Seed::new([0x6b; 32]));

        assert_eq!(
            verify_user_selection(&selection, plan, &session, 10, &key.pk),
            Err(SelectionDenial::StaleRevision)
        );
        assert_eq!(
            verify_user_selection(&selection, plan, &[0x32; 32], 9, &key.pk),
            Err(SelectionDenial::WrongSession)
        );
        assert_eq!(
            verify_user_selection(&selection, plan, &session, 9, &other_key.pk),
            Err(SelectionDenial::InvalidAttestation)
        );
    }

    #[test]
    fn changing_both_payload_and_digest_does_not_repair_the_signature() {
        let plan = b"approved bytes";
        let session = [0x31; 32];
        let (key, mut selection) = signed_selection(plan, 9, session);
        let replacement = b"different bytes";
        selection.plan_sha256 = plan_sha256(replacement);
        assert_eq!(
            verify_user_selection(&selection, replacement, &session, 9, &key.pk),
            Err(SelectionDenial::InvalidAttestation)
        );
    }
}
