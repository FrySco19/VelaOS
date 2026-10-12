//! M-G1b end-to-end admission: "Candidate != Authority" wired to the Executor.
//!
//! ```text
//! mock AI intent -> MockPlanner::propose -> [Candidate] (data only)
//!                -> TrustedSelectionChannel -> UserSelection{handle, expected_version}
//!                -> GrantIssuer: attenuated Biscuit LFS_READ grant (TTL <= 120 s)
//!                -> verify_read_grant_at (reader-side check, defence in depth)
//!                -> authority_digest = BLAKE3(grant) bound into the Wasm payload
//!                -> execute token attenuated to (caller, executor, execute, bytes, fuel, not_after)
//!                -> PolicyDecisionPoint::admit_at -> bridge::signed_frame (wire v2, unchanged)
//! ```
//!
//! The planner's only output type is [`Candidate`]. [`SelectionGatedPdp`] owns the
//! PDP signer, the read delegation and the execute delegation privately; there is
//! no API that signs a frame without passing through a [`UserSelection`] grant.
//! Without a selection the outcome is `POLICY_DENIED` (C5 10): no grant, no frame,
//! no Wasm.
//!
//! Std host only (`biscuit-auth` is std-only). Nothing here links into `app/`.

use crate::grant::{
    Candidate, GrantIssuer, GrantRequest, GrantResult, Handle, UserSelection,
    verify_read_grant_at,
};
use crate::ledger_wire::GRANT_SECTION;
use crate::{AdmitRequest, AdmitResult, Attenuation, DenyCode, KeyReuse, PolicyDecisionPoint, attenuate, bridge};
use biscuit_auth::PublicKey as BiscuitPublicKey;
use ed25519_compact::SecretKey;
use std::time::{Duration, SystemTime};

/// Biscuit resource of the execute delegation.
pub const EXECUTOR_RESOURCE: &str = "executor";

/// `run() -> i32` returning 21: the e2e task (distinct from the bridge's 7).
pub const RETURN_21_MODULE: &[u8] = &[
    0, 97, 115, 109, 1, 0, 0, 0, 1, 5, 1, 96, 0, 1, 127, 3, 2, 1, 0, 7, 7, 1, 3, b'r', b'u', b'n',
    0, 0, 10, 6, 1, 4, 0, 65, 21, 11,
];

/// One entry of the mock local index the planner ranks over.
#[derive(Clone, Debug)]
pub struct IndexEntry {
    pub handle: Handle,
    pub version: u64,
    pub title: &'static str,
}

/// Mock AI planner. It can only *rank*: its output type has no token and no
/// right. Deterministic so tests and the QEMU bundle are reproducible.
pub struct MockPlanner;

impl MockPlanner {
    pub fn propose(intent: &str, index: &[IndexEntry]) -> Vec<Candidate> {
        let words: Vec<String> = intent
            .split_whitespace()
            .map(|w| w.to_lowercase())
            .filter(|w| w.len() > 2)
            .collect();
        let mut out: Vec<Candidate> = index
            .iter()
            .filter_map(|e| {
                let title = e.title.to_lowercase();
                let hits = words.iter().filter(|w| title.contains(w.as_str())).count();
                (hits > 0).then(|| Candidate {
                    handle: e.handle,
                    version: e.version,
                    score: hits as f32 / words.len().max(1) as f32,
                })
            })
            .collect();
        out.sort_by(|a, b| b.score.total_cmp(&a.score));
        out
    }
}

/// BLAKE3-256 of the serialized Biscuit grant (CONTRACTS_v0 C1 `authority_digest`).
pub fn authority_digest(grant: &[u8]) -> [u8; 32] {
    *blake3::hash(grant).as_bytes()
}

/// Append exactly one `vela.grant` custom section carrying `digest`.
/// The admission signature covers the payload, hence the digest.
pub fn bind_grant(module: &[u8], digest: &[u8; 32]) -> Vec<u8> {
    let content_len = 1 + GRANT_SECTION.len() + 32;
    assert!(content_len < 128 && GRANT_SECTION.len() < 128);
    let mut out = module.to_vec();
    out.push(0); // custom section id
    out.push(content_len as u8);
    out.push(GRANT_SECTION.len() as u8);
    out.extend_from_slice(GRANT_SECTION);
    out.extend_from_slice(digest);
    out
}

/// Which gate produced a deny.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    /// No/invalid UserSelection or grant constraints (GrantIssuer).
    Grant,
    /// Reader-side re-verification of the minted grant failed.
    GrantVerify,
    /// Execute-token attenuation could not be built.
    ExecuteToken,
    /// Executor-admission Biscuit check / request bounds (PolicyDecisionPoint).
    Admission,
}

/// A task proposed for execution on behalf of the user.
#[derive(Clone, Debug)]
pub struct TaskRequest<'a> {
    pub caller: String,
    /// Handle the planner wants the task to read.
    pub requested: Handle,
    /// Candidates the shell displayed for this intent.
    pub candidates: &'a [Candidate],
    /// `None` unless the trusted input path produced a selection.
    pub selection: Option<UserSelection>,
    pub ttl: Duration,
    /// Untrusted planner/LLM text; audit only, never parsed.
    pub planner_note: String,
    pub request_id: u32,
    /// Import-free Wasm module (without grant section; it is bound here).
    pub module: Vec<u8>,
    pub fuel: u64,
}

#[derive(Debug)]
pub struct Admission {
    pub request_id: u32,
    pub frame: Box<[u8; bridge::PAGE_SIZE]>,
    /// Attenuated Biscuit LFS_READ grant (delegation + 1 narrowing block).
    pub grant: Vec<u8>,
    pub authority_digest: [u8; 32],
    pub payload: Vec<u8>,
    pub not_after: SystemTime,
    pub selection_id: u64,
    pub selected_version: u64,
}

#[derive(Debug)]
pub enum TaskOutcome {
    Admitted(Box<Admission>),
    Denied {
        code: DenyCode,
        stage: Stage,
        reason: String,
    },
}

impl TaskOutcome {
    pub fn deny_code(&self) -> Option<DenyCode> {
        match self {
            Self::Denied { code, .. } => Some(*code),
            Self::Admitted(_) => None,
        }
    }
    pub fn admitted(self) -> Option<Box<Admission>> {
        match self {
            Self::Admitted(a) => Some(a),
            Self::Denied { .. } => None,
        }
    }
}

#[derive(Debug)]
pub enum SetupError {
    KeyReuse,
    Token(String),
}

impl From<KeyReuse> for SetupError {
    fn from(_: KeyReuse) -> Self {
        Self::KeyReuse
    }
}

/// PCB host-side PDP gated by explicit user selection.
pub struct SelectionGatedPdp {
    root: BiscuitPublicKey,
    issuer: GrantIssuer,
    pdp: PolicyDecisionPoint,
    exec_delegation: Vec<u8>,
}

impl SelectionGatedPdp {
    /// `read_delegation`: root-signed `right(caller, "lfs", "read")`.
    /// `exec_delegation`: root-signed `right(caller, "executor", "execute")`.
    /// Both stay private to this struct (whoever holds them can attenuate).
    pub fn new(
        root: BiscuitPublicKey,
        read_delegation: Vec<u8>,
        exec_delegation: Vec<u8>,
        signer: SecretKey,
    ) -> Result<Self, SetupError> {
        let issuer = GrantIssuer::new(root, read_delegation)
            .map_err(|e| SetupError::Token(e.to_string()))?;
        biscuit_auth::Biscuit::from(&exec_delegation, root)
            .map_err(|e| SetupError::Token(e.to_string()))?;
        Ok(Self {
            root,
            issuer,
            pdp: PolicyDecisionPoint::new(root, signer)?,
            exec_delegation,
        })
    }

    pub fn frames_signed(&self) -> u64 {
        self.pdp.frames_signed()
    }
    pub fn grants_issued(&self) -> u64 {
        self.issuer.grants_issued()
    }

    pub fn admit_task(&mut self, r: &TaskRequest) -> TaskOutcome {
        self.admit_task_at(r, crate::clock::SystemClock::wall_now())
    }

    pub fn admit_task_at(&mut self, r: &TaskRequest, now: SystemTime) -> TaskOutcome {
        let deny = |code, stage, reason: String| TaskOutcome::Denied {
            code,
            stage,
            reason,
        };
        // 1. Candidate != Authority: only a UserSelection can mint the grant.
        let grant_request = GrantRequest {
            caller: r.caller.clone(),
            requested: r.requested,
            candidates: r.candidates,
            selection: r.selection.clone(),
            ttl: r.ttl,
            planner_note: r.planner_note.clone(),
        };
        let (grant, not_after) = match self.issuer.issue_read_grant_at(&grant_request, now) {
            GrantResult::Grant { token, not_after } => (token, not_after),
            GrantResult::Deny { code, reason } => return deny(code, Stage::Grant, reason),
        };
        let Some(selection) = r.selection.as_ref() else {
            return deny(
                DenyCode::PolicyDenied,
                Stage::Grant,
                "POLICY_DENIED: no UserSelection".into(),
            );
        };
        // 2. Reader-side verification of exactly what was minted.
        if let Err(code) = verify_read_grant_at(
            &grant,
            self.root,
            &r.caller,
            selection.handle(),
            selection.expected_version(),
            now,
        ) {
            return deny(code, Stage::GrantVerify, "minted grant failed verification".into());
        }
        // 3. Bind the grant into the signed payload.
        let digest = authority_digest(&grant);
        let payload = bind_grant(&r.module, &digest);
        // 4. Execute token: narrowed to this caller/op/size/fuel and the grant's not_after.
        let attenuation = Attenuation {
            caller: Some(r.caller.clone()),
            resource: Some(EXECUTOR_RESOURCE.into()),
            operation: Some(crate::OPERATION_EXECUTE.into()),
            max_payload_bytes: Some(payload.len() as u32),
            max_fuel: Some(r.fuel),
            expires_at: Some(not_after),
        };
        let exec_token = match attenuate(&self.exec_delegation, self.root, &attenuation) {
            Ok(t) => t,
            Err(e) => {
                return deny(
                    DenyCode::PolicyDenied,
                    Stage::ExecuteToken,
                    format!("POLICY_DENIED: {e}"),
                );
            }
        };
        // 5. Existing M-G1a admission; only ALLOW signs the unchanged v2 frame.
        let admit = AdmitRequest {
            token: exec_token,
            caller: r.caller.clone(),
            resource: EXECUTOR_RESOURCE.into(),
            operation: crate::OPERATION_EXECUTE.into(),
            request_id: r.request_id,
            payload: payload.clone(),
            fuel: r.fuel,
        };
        match self.pdp.admit_at(&admit, now) {
            AdmitResult::Allow { frame } => TaskOutcome::Admitted(Box::new(Admission {
                request_id: r.request_id,
                frame,
                grant,
                authority_digest: digest,
                payload,
                not_after,
                selection_id: selection.selection_id(),
                selected_version: selection.expected_version(),
            })),
            AdmitResult::Deny { code, reason } => deny(code, Stage::Admission, reason),
        }
    }
}
