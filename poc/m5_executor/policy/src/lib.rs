//! Host-side policy decision point (PDP) for the M5 Executor bridge (Gate 1, M-G1a).
//!
//! The PDP runs in a trusted **std host** process, never inside the Microkit
//! guest and never inside a Wasm task. It pins one Biscuit root public key,
//! verifies an (optionally attenuated) Biscuit for exactly one request, and only
//! on ALLOW signs the unchanged Executor wire format v2 (`VBRG` header + `VEX2`
//! claims + payload) with the PCB Ed25519 signing key.
//!
//! Authority separation (see `BRIDGE_CONTRACT.md`, "Host PDP (M-G1a)"):
//! * the Biscuit root key authorises *who may ask for what*;
//! * the PCB Ed25519 key authenticates *the exact bytes the Executor runs*;
//! * neither is a seL4 kernel capability. The Biscuit root key must differ from
//!   the PCB signing key ([`PolicyDecisionPoint::new`] rejects reuse).
//!
//! No LLM, prompt or model output is an input to [`PolicyDecisionPoint::admit`];
//! a planner may only *propose* an [`AdmitRequest`].

use biscuit_auth::builder::{AuthorizerBuilder, BlockBuilder, Term};
use biscuit_auth::{AuthorizerLimits, Biscuit, PublicKey as BiscuitPublicKey};
use ed25519_compact::SecretKey;
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// The Executor wire protocol, compiled from the guest source without changes.
/// Linking it here keeps the PDP byte-for-byte identical to what the guest verifies.
#[path = "../../app/src/bridge.rs"]
#[allow(clippy::missing_safety_doc)]
pub mod bridge;

/// M-G1b: read grants bound to an explicit user selection (Candidate != Authority).
pub mod grant;

/// Shared `no_std` LFS receipt wire + ledger (also compiled into the guest).
#[path = "../../app/src/ledger_wire.rs"]
pub mod ledger_wire;

/// M-G1b end-to-end admission: UserSelection -> Biscuit grant -> signed frame.
pub mod pipeline;

/// M-G1c LFS writer + PCB notary orchestration (State Ledger).
pub mod ledger;

/// M-CLK-3: in-guest monotonic deadline logic (same file the QEMU PCB compiles).
#[path = "../../app/src/guest_clock.rs"]
pub mod guest_clock;

/// M-CLK-1: injectable trusted clock (wall for minting, monotonic for TTL).
pub mod clock;

/// M-LFS-1..3: durable append-only + fsync ledger store and persistent epoch.
pub mod durable;

/// Same bound as M1/M2 authority code.
pub const MAX_TOKEN_BYTES: usize = 16 * 1024;
/// Largest Wasm payload that fits in one 4 KiB v2 frame.
pub const MAX_PAYLOAD_BYTES: usize = bridge::PAGE_SIZE - bridge::PAYLOAD_OFFSET;
/// The only operation the Executor bridge currently performs.
pub const OPERATION_EXECUTE: &str = "execute";

const LIMITS: AuthorizerLimits = AuthorizerLimits {
    max_facts: 256,
    max_iterations: 32,
    max_time: Duration::from_millis(250),
};

/// Stable deny codes. `PolicyDenied` is the Biscuit-policy outcome (POLICY_DENIED).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DenyCode {
    /// Request shape invalid (identifiers, empty/oversized payload, token size).
    Malformed,
    /// Token does not parse or does not verify under the pinned Biscuit root.
    Authentication,
    /// Biscuit verified but authorizer/checks denied (missing right, out of
    /// scope, quota, expiry, run limits).
    PolicyDenied,
    /// Request exceeds host-enforced Executor bounds (fuel).
    Scope,
    /// Request id is zero or not strictly greater than the last admitted id.
    Replay,
    /// M-G1b: grant lifetime outside 1..=120 s, or grant past `not_after`.
    Expired,
    /// M-G1b: selection/grant `expected_version` differs from the object version.
    VersionConflict,
}

impl DenyCode {
    /// Numeric code from CONTRACTS_v0 C5.
    pub fn contract_code(self) -> u16 {
        match self {
            Self::Malformed => 1,
            Self::Authentication => 2,
            Self::Scope => 3,
            Self::Replay => 4,
            Self::Expired => 7,
            Self::PolicyDenied => 10,
            Self::VersionConflict => 12,
        }
    }
}

#[derive(Clone, Debug)]
pub struct AdmitRequest {
    /// Biscuit token bytes (authority block + optional attenuation blocks).
    pub token: Vec<u8>,
    pub caller: String,
    pub resource: String,
    /// Must be [`OPERATION_EXECUTE`] for the v2 bridge; anything else is denied
    /// by policy unless the token grants it (and then the frame still only executes).
    pub operation: String,
    pub request_id: u32,
    pub payload: Vec<u8>,
    pub fuel: u64,
}

#[derive(Debug)]
pub enum AdmitResult {
    /// Signed v2 frame, ready to publish on the PCB-writable page.
    Allow { frame: Box<[u8; bridge::PAGE_SIZE]> },
    /// Nothing was signed.
    Deny { code: DenyCode, reason: String },
}

impl AdmitResult {
    pub fn is_allow(&self) -> bool {
        matches!(self, Self::Allow { .. })
    }
    pub fn deny_code(&self) -> Option<DenyCode> {
        match self {
            Self::Deny { code, .. } => Some(*code),
            Self::Allow { .. } => None,
        }
    }
}

/// Optional scope narrowing appended by a holder of a parent token.
#[derive(Clone, Debug, Default)]
pub struct Attenuation {
    pub caller: Option<String>,
    pub resource: Option<String>,
    pub operation: Option<String>,
    pub max_payload_bytes: Option<u32>,
    pub max_fuel: Option<u64>,
    pub expires_at: Option<SystemTime>,
}

pub struct PolicyDecisionPoint {
    root: BiscuitPublicKey,
    signer: SecretKey,
    last_admitted: u32,
    frames_signed: u64,
}

#[derive(Debug, PartialEq, Eq)]
pub struct KeyReuse;

impl PolicyDecisionPoint {
    /// `root` is the pinned Biscuit root; `signer` is the PCB Ed25519 key whose
    /// public half is the Executor trust anchor. They must be distinct keys.
    pub fn new(root: BiscuitPublicKey, signer: SecretKey) -> Result<Self, KeyReuse> {
        if root.to_bytes() == *signer.public_key() {
            return Err(KeyReuse);
        }
        Ok(Self {
            root,
            signer,
            last_admitted: 0,
            frames_signed: 0,
        })
    }

    /// Number of frames signed so far (a deny must never increase it).
    pub fn frames_signed(&self) -> u64 {
        self.frames_signed
    }
    pub fn last_admitted(&self) -> u32 {
        self.last_admitted
    }

    pub fn admit(&mut self, request: &AdmitRequest) -> AdmitResult {
        self.admit_at(request, clock::SystemClock::wall_now())
    }

    /// Deterministic-clock variant used by tests and replay tooling.
    pub fn admit_at(&mut self, request: &AdmitRequest, now: SystemTime) -> AdmitResult {
        match self.decide(request, now) {
            Ok(()) => {
                let frame = bridge::signed_frame(
                    &self.signer,
                    request.request_id,
                    &request.payload,
                    request.fuel,
                );
                self.last_admitted = request.request_id;
                self.frames_signed += 1;
                AdmitResult::Allow {
                    frame: Box::new(frame),
                }
            }
            Err((code, reason)) => AdmitResult::Deny { code, reason },
        }
    }

    fn decide(&self, r: &AdmitRequest, now: SystemTime) -> Result<(), (DenyCode, String)> {
        let deny = |code, reason: &str| Err((code, reason.to_string()));
        if !valid_name(&r.caller) || !valid_name(&r.resource) || !valid_name(&r.operation) {
            return deny(DenyCode::Malformed, "invalid identifier");
        }
        if r.token.is_empty() || r.token.len() > MAX_TOKEN_BYTES {
            return deny(DenyCode::Malformed, "token size out of bounds");
        }
        if r.payload.is_empty() || r.payload.len() > MAX_PAYLOAD_BYTES {
            return deny(DenyCode::Malformed, "payload size out of bounds");
        }
        if r.fuel == 0 || r.fuel > bridge::MAX_FUEL {
            return deny(DenyCode::Scope, "fuel outside executor bounds");
        }
        if r.request_id == 0 || r.request_id <= self.last_admitted {
            return deny(DenyCode::Replay, "request id not fresh");
        }
        let token = Biscuit::from(&r.token, self.root)
            .map_err(|e| (DenyCode::Authentication, format!("token rejected: {e}")))?;
        let mut params = HashMap::new();
        params.insert("caller".to_string(), Term::Str(r.caller.clone()));
        params.insert("resource".to_string(), Term::Str(r.resource.clone()));
        params.insert("operation".to_string(), Term::Str(r.operation.clone()));
        params.insert("bytes".to_string(), Term::Integer(r.payload.len() as i64));
        params.insert("fuel".to_string(), Term::Integer(r.fuel as i64));
        params.insert("now".to_string(), Term::Date(unix_secs(now)));
        let source = "caller({caller}); resource({resource}); operation({operation}); \
             payload_bytes({bytes}); fuel({fuel}); time({now}); \
             allow if caller($c), resource($r), operation($o), right($c, $r, $o);";
        AuthorizerBuilder::new()
            .set_limits(LIMITS)
            .code_with_params(source, params, HashMap::new())
            .and_then(|b| b.build(&token))
            .and_then(|mut a| a.authorize())
            .map(|_| ())
            .map_err(|e| (DenyCode::PolicyDenied, format!("POLICY_DENIED: {e}")))
    }
}

/// Append one attenuation block to `parent`. Attenuation can only narrow:
/// checks added here must pass in addition to every earlier check, and facts
/// in this block are not trusted by the PDP allow policy.
pub fn attenuate(
    parent: &[u8],
    root: BiscuitPublicKey,
    a: &Attenuation,
) -> Result<Vec<u8>, biscuit_auth::error::Token> {
    let token = Biscuit::from(parent, root)?;
    let mut params = HashMap::new();
    let mut source = String::new();
    for (key, value) in [
        ("caller", &a.caller),
        ("resource", &a.resource),
        ("operation", &a.operation),
    ] {
        if let Some(value) = value {
            params.insert(format!("att_{key}"), Term::Str(value.clone()));
            source.push_str(&format!("check if {key}({{att_{key}}});\n"));
        }
    }
    if let Some(max) = a.max_payload_bytes {
        params.insert("att_bytes".into(), Term::Integer(max as i64));
        source.push_str("check if payload_bytes($b), $b <= {att_bytes};\n");
    }
    if let Some(max) = a.max_fuel {
        params.insert(
            "att_fuel".into(),
            Term::Integer(max.min(i64::MAX as u64) as i64),
        );
        source.push_str("check if fuel($f), $f <= {att_fuel};\n");
    }
    if let Some(expiry) = a.expires_at {
        params.insert("att_exp".into(), Term::Date(unix_secs(expiry)));
        source.push_str("check if time($t), $t <= {att_exp};\n");
    }
    let block = BlockBuilder::new().code_with_params(&source, params, HashMap::new())?;
    token.append(block)?.to_vec()
}

fn valid_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
}

fn unix_secs(t: SystemTime) -> u64 {
    t.duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
