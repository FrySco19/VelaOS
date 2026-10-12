//! M-G1b "Candidate != Authority": host-side read grants bound to an explicit
//! user selection.
//!
//! Recall (embedding + BM25 -> RRF) returns [`Candidate`]s. A candidate is
//! **data only**: it carries no token, no right, and there is no function that
//! turns a candidate (or any planner/LLM text) into authority. The only path to
//! an `LFS_READ` grant is [`GrantIssuer::issue_read_grant_at`] with a
//! [`UserSelection`] produced by the trusted UI input path
//! ([`TrustedSelectionChannel`]), for exactly the selected handle and version.
//!
//! The emitted grant is the PDP-held delegation token (authority block from the
//! pinned Biscuit root: `right(caller, "lfs", "read")`) plus ONE attenuation
//! block that narrows it to one operation, one caller, one handle, one object
//! version and a `not_after` no later than [`MAX_READ_GRANT_TTL`] (120 s).
//! [`verify_read_grant_at`] is the matching reader-side check.
//!
//! Std host only. Nothing here touches the Executor wire v2, QEMU or `app/`.

use crate::{DenyCode, LIMITS, unix_secs, valid_name};
use biscuit_auth::builder::{AuthorizerBuilder, BlockBuilder, Term};
use biscuit_auth::error::{FailedCheck, Logic, Token as TokenError};
use biscuit_auth::{Biscuit, PublicKey as BiscuitPublicKey};
use std::collections::{HashMap, HashSet};
use std::time::{Duration, SystemTime};

/// Opaque LFS object handle (32 bytes, same width as envelope v0 `objects[].handle`).
pub type Handle = [u8; 32];

/// Upper bound on a read grant lifetime (concept + M1 handle lifetime `1..=120` s).
pub const MAX_READ_GRANT_TTL: Duration = Duration::from_secs(120);
/// Biscuit resource / operation names used by the delegation and the grant.
pub const LFS_RESOURCE: &str = "lfs";
pub const LFS_READ: &str = "read";

/// Fixed check order of the grant attenuation block (block 1). The verifier maps
/// failed checks back to C5 codes by this position, never by parsing text.
const CHECK_OPERATION: u32 = 0;
const CHECK_CALLER: u32 = 1;
const CHECK_HANDLE: u32 = 2;
const CHECK_VERSION: u32 = 3;
const CHECK_NOT_AFTER: u32 = 4;
const GRANT_BLOCK: u32 = 1;

/// A recall result. Data only: holding, listing or ranking candidates confers
/// nothing. Deliberately has no token and no "authorized" flag.
#[derive(Clone, Debug, PartialEq)]
pub struct Candidate {
    pub handle: Handle,
    pub version: u64,
    /// Display/ranking metadata (e.g. RRF score); never consulted by policy.
    pub score: f32,
}

/// An explicit human choice: "read THIS object at THIS version".
///
/// Fields are private and the only constructor is [`TrustedSelectionChannel::select`],
/// which in the product is owned by the trusted shell/compositor input path, never
/// by the planner. (In-process Rust privacy is a structural guard, not an isolation
/// boundary; the real boundary is which domain holds the channel.)
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UserSelection {
    handle: Handle,
    expected_version: u64,
    selection_id: u64,
}

impl UserSelection {
    pub fn handle(&self) -> &Handle {
        &self.handle
    }
    pub fn expected_version(&self) -> u64 {
        self.expected_version
    }
    pub fn selection_id(&self) -> u64 {
        self.selection_id
    }
}

/// Trusted user-input path. Each selection gets a fresh id and is single-use.
#[derive(Debug, Default)]
pub struct TrustedSelectionChannel {
    next_id: u64,
}

impl TrustedSelectionChannel {
    pub fn new() -> Self {
        Self::default()
    }
    /// Event `UserSelection{handle, expected_version}` from a real user gesture.
    pub fn select(&mut self, handle: Handle, expected_version: u64) -> UserSelection {
        self.next_id += 1;
        UserSelection {
            handle,
            expected_version,
            selection_id: self.next_id,
        }
    }
}

/// What a planner may submit. `planner_note` is untrusted free text (LLM output,
/// injected document text, ...). It is carried for audit display only and is
/// never parsed, matched or turned into Datalog.
#[derive(Clone, Debug)]
pub struct GrantRequest<'a> {
    pub caller: String,
    /// The handle the planner wants to read.
    pub requested: Handle,
    /// Candidates shown to the user for this recall.
    pub candidates: &'a [Candidate],
    /// `None` unless the trusted channel produced a selection.
    pub selection: Option<UserSelection>,
    pub ttl: Duration,
    pub planner_note: String,
}

#[derive(Debug)]
pub enum GrantResult {
    /// Attenuated Biscuit: delegation + one narrowing block.
    Grant {
        token: Vec<u8>,
        not_after: SystemTime,
    },
    /// No token was minted.
    Deny { code: DenyCode, reason: String },
}

impl GrantResult {
    pub fn deny_code(&self) -> Option<DenyCode> {
        match self {
            Self::Deny { code, .. } => Some(*code),
            Self::Grant { .. } => None,
        }
    }
}

/// Host PDP component that turns (candidate + explicit selection) into a grant.
pub struct GrantIssuer {
    root: BiscuitPublicKey,
    /// Root-signed `right(caller, "lfs", "read")` delegation. Secret to the PDP:
    /// whoever holds it can attenuate it, so it never leaves this struct.
    delegation: Vec<u8>,
    consumed: HashSet<u64>,
    grants_issued: u64,
}

impl GrantIssuer {
    pub fn new(root: BiscuitPublicKey, delegation: Vec<u8>) -> Result<Self, TokenError> {
        Biscuit::from(&delegation, root)?;
        Ok(Self {
            root,
            delegation,
            consumed: HashSet::new(),
            grants_issued: 0,
        })
    }

    /// A deny must never increase this.
    pub fn grants_issued(&self) -> u64 {
        self.grants_issued
    }

    pub fn issue_read_grant(&mut self, r: &GrantRequest) -> GrantResult {
        self.issue_read_grant_at(r, crate::clock::SystemClock::wall_now())
    }

    pub fn issue_read_grant_at(&mut self, r: &GrantRequest, now: SystemTime) -> GrantResult {
        match self.decide(r) {
            Err((code, reason)) => GrantResult::Deny { code, reason },
            Ok(sel) => match self.mint(&r.caller, &sel, now + r.ttl) {
                Ok(token) => {
                    self.consumed.insert(sel.selection_id);
                    self.grants_issued += 1;
                    GrantResult::Grant {
                        token,
                        not_after: now + r.ttl,
                    }
                }
                Err(e) => GrantResult::Deny {
                    code: DenyCode::PolicyDenied,
                    reason: format!("POLICY_DENIED: mint failed: {e}"),
                },
            },
        }
    }

    fn decide(&self, r: &GrantRequest) -> Result<UserSelection, (DenyCode, String)> {
        let deny = |code, reason: &str| Err((code, reason.to_string()));
        if !valid_name(&r.caller) {
            return deny(DenyCode::Malformed, "invalid caller");
        }
        // Inversion of control: without an explicit user selection nothing
        // else is even looked at -- not the candidates, not the planner note.
        let Some(sel) = r.selection.clone() else {
            return deny(
                DenyCode::PolicyDenied,
                "POLICY_DENIED: candidate is not authority; no UserSelection",
            );
        };
        if r.requested != sel.handle {
            return deny(
                DenyCode::Scope,
                "SCOPE: requested handle != selected handle",
            );
        }
        let Some(candidate) = r.candidates.iter().find(|c| c.handle == sel.handle) else {
            return deny(
                DenyCode::Scope,
                "SCOPE: selected handle was not offered as a candidate",
            );
        };
        if candidate.version != sel.expected_version {
            return deny(
                DenyCode::VersionConflict,
                "VERSION_CONFLICT: candidate version != expected_version",
            );
        }
        if sel.expected_version > i64::MAX as u64 {
            return deny(DenyCode::Malformed, "version out of range");
        }
        if r.ttl.is_zero() || r.ttl > MAX_READ_GRANT_TTL {
            return deny(DenyCode::Expired, "EXPIRED: ttl must be in 1..=120 s");
        }
        if self.consumed.contains(&sel.selection_id) {
            return deny(DenyCode::Replay, "REPLAY: selection already consumed");
        }
        Ok(sel)
    }

    fn mint(
        &self,
        caller: &str,
        sel: &UserSelection,
        not_after: SystemTime,
    ) -> Result<Vec<u8>, TokenError> {
        let token = Biscuit::from(&self.delegation, self.root)?;
        let mut params = HashMap::new();
        params.insert("op".to_string(), Term::Str(LFS_READ.into()));
        params.insert("c".to_string(), Term::Str(caller.into()));
        params.insert("h".to_string(), Term::Bytes(sel.handle.to_vec()));
        params.insert("v".to_string(), Term::Integer(sel.expected_version as i64));
        params.insert("exp".to_string(), Term::Date(unix_secs(not_after)));
        // Order must match the CHECK_* constants.
        let source = "check if operation({op});\n\
                      check if caller({c});\n\
                      check if handle({h});\n\
                      check if object_version({v});\n\
                      check if time($t), $t <= {exp};\n";
        let block = BlockBuilder::new().code_with_params(source, params, HashMap::new())?;
        token.append(block)?.to_vec()
    }
}

/// Reader-side check (LFS/PDP) for one read of `(handle, version)` by `caller`.
/// Error codes follow CONTRACTS_v0 C5.
pub fn verify_read_grant_at(
    token: &[u8],
    root: BiscuitPublicKey,
    caller: &str,
    handle: &Handle,
    version: u64,
    now: SystemTime,
) -> Result<(), DenyCode> {
    if !valid_name(caller)
        || version > i64::MAX as u64
        || token.is_empty()
        || token.len() > crate::MAX_TOKEN_BYTES
    {
        return Err(DenyCode::Malformed);
    }
    let token = Biscuit::from(token, root).map_err(|_| DenyCode::Authentication)?;
    // A bare delegation (no narrowing block) or extra holder blocks are not a grant.
    if token.block_count() != 2 {
        return Err(DenyCode::PolicyDenied);
    }
    let mut params = HashMap::new();
    params.insert("caller".to_string(), Term::Str(caller.into()));
    params.insert("op".to_string(), Term::Str(LFS_READ.into()));
    params.insert("res".to_string(), Term::Str(LFS_RESOURCE.into()));
    params.insert("h".to_string(), Term::Bytes(handle.to_vec()));
    params.insert("v".to_string(), Term::Integer(version as i64));
    params.insert("now".to_string(), Term::Date(unix_secs(now)));
    let source = "caller({caller}); operation({op}); handle({h}); object_version({v}); time({now}); \
                  allow if caller($c), operation($o), right($c, {res}, $o);";
    let result = AuthorizerBuilder::new()
        .set_limits(LIMITS)
        .code_with_params(source, params, HashMap::new())
        .and_then(|b| b.build(&token))
        .and_then(|mut a| a.authorize());
    match result {
        Ok(_) => Ok(()),
        Err(TokenError::FailedLogic(Logic::Unauthorized { checks, .. }))
        | Err(TokenError::FailedLogic(Logic::NoMatchingPolicy { checks })) => {
            Err(classify(&checks))
        }
        Err(_) => Err(DenyCode::PolicyDenied),
    }
}

fn classify(checks: &[FailedCheck]) -> DenyCode {
    let failed: Vec<u32> = checks
        .iter()
        .filter_map(|c| match c {
            FailedCheck::Block(b) if b.block_id == GRANT_BLOCK => Some(b.check_id),
            _ => None,
        })
        .collect();
    if failed.contains(&CHECK_NOT_AFTER) {
        DenyCode::Expired
    } else if failed.contains(&CHECK_HANDLE)
        || failed.contains(&CHECK_CALLER)
        || failed.contains(&CHECK_OPERATION)
    {
        DenyCode::Scope
    } else if failed.contains(&CHECK_VERSION) {
        DenyCode::VersionConflict
    } else {
        DenyCode::PolicyDenied
    }
}
