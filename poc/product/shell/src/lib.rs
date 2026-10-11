//! VelaOS Zero-App Shell runtime (host model, `std`).
//!
//! ```text
//! Idle --submit(utterance)--> Interpreting --IntentEngine--> Glass(plan preview)
//! Glass --click Avvia | Rivedi piano--> Authorizing
//!       (emits UserSelection{handle, expected_version} -> TrustedSelectionChannel
//!        -> SelectionGatedPdp::admit_task_at)                --> Result | Error
//! Glass --Annulla--> Idle (undo: nothing was authorized)
//! Result | Error --Chiudi--> Idle
//! ```
//!
//! Invariants (tested in `tests/`):
//! * The ONLY way to reach the PDP is [`ShellRuntime::click`] with `Avvia` /
//!   `RivediPiano` on a candidate index that the glass UI is currently showing.
//!   There is no API that takes a `TaskRequest`, a policy `UserSelection` or a
//!   handle from the caller; the PDP and the trusted selection channel are private.
//! * The selection is re-derived from the displayed candidate (handle + version
//!   shown to the user); planner/LLM text is forwarded only as `planner_note`.
//! * Rendering is "ephemeral glass": [`ShellRuntime::render`] produces a text
//!   view model of the current state; nothing persists after `Chiudi`/`Annulla`.
//!
//! Host model only: in the product the click arrives from the compositor's
//! trusted input path in its own protection domain. In-process Rust privacy is a
//! structural guard, not an isolation boundary.
#![forbid(unsafe_code)]

use std::time::{Duration, SystemTime};
use vela_executor_host::HostExecutor;
use vela_executor_policy::DenyCode;
use vela_executor_policy::grant::{Candidate as PolicyCandidate, MAX_READ_GRANT_TTL, TrustedSelectionChannel};
use vela_executor_policy::pipeline::{RETURN_21_MODULE, SelectionGatedPdp, Stage, TaskOutcome, TaskRequest};
use vela_intent::{IntentEngine, IntentProposal, UserSelection};

/// Default grant TTL requested by the shell (clamped to <= 120 s).
pub const DEFAULT_TTL: Duration = Duration::from_secs(60);
/// Fuel for the demo task.
pub const DEFAULT_FUEL: u64 = 10_000;

/// Coarse state names, recorded in [`ShellRuntime::trace`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Idle,
    Interpreting,
    Glass,
    Authorizing,
    Result,
    Error,
}

/// Which button the user pressed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    /// "Avvia": authorize and dispatch the admitted frame to the Executor.
    Avvia,
    /// "Rivedi piano": authorize (read grant + admission) but do NOT dispatch;
    /// the user inspects the authorized plan first.
    RivediPiano,
}

/// UI gestures. Only these drive the runtime.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Click {
    Avvia { candidate: usize },
    RivediPiano { candidate: usize },
    /// Undo from the glass: back to idle, nothing authorized.
    Annulla,
    /// Dismiss a result/error.
    Chiudi,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Outcome {
    /// Avvia: admitted and run in the host Executor model.
    Executed { request_id: u32, value: u32, not_after: SystemTime },
    /// Rivedi piano: admitted, frame not dispatched.
    Reviewed { request_id: u32, not_after: SystemTime },
}

#[derive(Clone, Debug, PartialEq)]
pub enum ShellError {
    /// No candidate for the utterance.
    NothingFound,
    /// PDP denied (code, stage, reason).
    Denied { code: DenyCode, stage: Stage, reason: String },
    /// Executor rejected an admitted frame (status code).
    Executor { status: u64 },
}

#[derive(Clone, Debug, PartialEq)]
pub enum ShellState {
    Idle,
    Glass(IntentProposal),
    Result { selection: UserSelection, action: Action, outcome: Outcome },
    Error { selection: Option<UserSelection>, error: ShellError },
}

impl ShellState {
    pub fn phase(&self) -> Phase {
        match self {
            Self::Idle => Phase::Idle,
            Self::Glass(_) => Phase::Glass,
            Self::Result { .. } => Phase::Result,
            Self::Error { .. } => Phase::Error,
        }
    }
}

/// A gesture that is not valid in the current state. The state is unchanged.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiReject {
    WrongState(Phase),
    NotOffered(usize),
}

pub struct ShellRuntime {
    caller: String,
    engine: IntentEngine,
    pdp: SelectionGatedPdp,
    channel: TrustedSelectionChannel,
    executor: HostExecutor,
    state: ShellState,
    trace: Vec<Phase>,
    emitted: Vec<UserSelection>,
    next_request_id: u32,
    ttl: Duration,
    clock: Box<dyn Fn() -> SystemTime>,
}

impl ShellRuntime {
    /// `executor` must pin the same PCB key the PDP signs with.
    pub fn new(caller: impl Into<String>, engine: IntentEngine, pdp: SelectionGatedPdp, executor: HostExecutor) -> Self {
        Self {
            caller: caller.into(),
            engine,
            pdp,
            channel: TrustedSelectionChannel::new(),
            executor,
            state: ShellState::Idle,
            trace: vec![Phase::Idle],
            emitted: Vec::new(),
            next_request_id: 1,
            ttl: DEFAULT_TTL,
            clock: Box::new(SystemTime::now),
        }
    }

    /// Requested grant TTL; values above 120 s are clamped (the PDP enforces it too).
    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.ttl = ttl.min(MAX_READ_GRANT_TTL);
        self
    }

    pub fn with_clock(mut self, clock: Box<dyn Fn() -> SystemTime>) -> Self {
        self.clock = clock;
        self
    }

    pub fn state(&self) -> &ShellState {
        &self.state
    }
    pub fn trace(&self) -> &[Phase] {
        &self.trace
    }
    /// Every `UserSelection` the shell has emitted toward the PDP, in order.
    pub fn emitted_selections(&self) -> &[UserSelection] {
        &self.emitted
    }
    pub fn frames_signed(&self) -> u64 {
        self.pdp.frames_signed()
    }
    pub fn grants_issued(&self) -> u64 {
        self.pdp.grants_issued()
    }
    /// Metadata access for the (local) engine, e.g. a new object version arriving.
    pub fn engine_mut(&mut self) -> &mut IntentEngine {
        &mut self.engine
    }

    fn enter(&mut self, s: ShellState) {
        self.trace.push(s.phase());
        self.state = s;
    }

    /// Idle -> Interpreting -> Glass (or Error::NothingFound). Never authorizes.
    pub fn submit(&mut self, utterance: &str) -> Result<&ShellState, UiReject> {
        if self.state != ShellState::Idle {
            return Err(UiReject::WrongState(self.state.phase()));
        }
        self.trace.push(Phase::Interpreting);
        let proposal = self.engine.propose_candidates(utterance);
        if proposal.is_empty() {
            self.enter(ShellState::Error { selection: None, error: ShellError::NothingFound });
        } else {
            self.enter(ShellState::Glass(proposal));
        }
        Ok(&self.state)
    }

    /// The single path toward the PDP.
    pub fn click(&mut self, click: Click) -> Result<&ShellState, UiReject> {
        match (click, &self.state) {
            (Click::Annulla, ShellState::Glass(_)) | (Click::Chiudi, ShellState::Result { .. } | ShellState::Error { .. }) => {
                self.enter(ShellState::Idle);
                Ok(&self.state)
            }
            (Click::Avvia { candidate }, ShellState::Glass(p)) => {
                let p = p.clone();
                self.authorize(p, candidate, Action::Avvia)
            }
            (Click::RivediPiano { candidate }, ShellState::Glass(p)) => {
                let p = p.clone();
                self.authorize(p, candidate, Action::RivediPiano)
            }
            _ => Err(UiReject::WrongState(self.state.phase())),
        }
    }

    fn authorize(&mut self, proposal: IntentProposal, index: usize, action: Action) -> Result<&ShellState, UiReject> {
        let Some(shown) = proposal.candidates.get(index) else {
            return Err(UiReject::NotOffered(index));
        };
        // The event: what the user saw and clicked.
        let selection = shown.as_selection();
        self.emitted.push(selection);
        self.trace.push(Phase::Authorizing);
        let policy_selection = self.channel.select(selection.handle, selection.expected_version);
        let candidates: Vec<PolicyCandidate> = proposal
            .candidates
            .iter()
            .map(|c| PolicyCandidate { handle: c.handle, version: c.version, score: c.score })
            .collect();
        let request_id = self.next_request_id;
        self.next_request_id += 1;
        let task = TaskRequest {
            caller: self.caller.clone(),
            requested: selection.handle,
            candidates: &candidates,
            selection: Some(policy_selection),
            ttl: self.ttl,
            planner_note: proposal.rationale.clone(),
            request_id,
            module: RETURN_21_MODULE.to_vec(),
            fuel: DEFAULT_FUEL,
        };
        let now = (self.clock)();
        let next = match self.pdp.admit_task_at(&task, now) {
            TaskOutcome::Denied { code, stage, reason } => ShellState::Error {
                selection: Some(selection),
                error: ShellError::Denied { code, stage, reason },
            },
            TaskOutcome::Admitted(a) => match action {
                Action::RivediPiano => ShellState::Result {
                    selection,
                    action,
                    outcome: Outcome::Reviewed { request_id: a.request_id, not_after: a.not_after },
                },
                Action::Avvia => {
                    let reply = self.executor.call(&a.frame);
                    let status = reply.status as u64;
                    if status == 0 {
                        ShellState::Result {
                            selection,
                            action,
                            outcome: Outcome::Executed { request_id: reply.request_id, value: reply.value, not_after: a.not_after },
                        }
                    } else {
                        ShellState::Error { selection: Some(selection), error: ShellError::Executor { status } }
                    }
                }
            },
        };
        self.enter(next);
        Ok(&self.state)
    }

    /// Ephemeral glass view model of the current state (text; the real UI renders it).
    pub fn render(&self) -> String {
        match &self.state {
            ShellState::Idle => "( Vela )  Chiedi qualcosa...".into(),
            ShellState::Glass(p) => {
                let mut s = format!("[glass] \"{}\"\n", p.utterance);
                for (i, c) in p.candidates.iter().enumerate() {
                    s += &format!("  {i}. {} (v{}, rrf {:.4}, {:?})  [Avvia] [Rivedi piano]\n", c.title, c.version, c.score, c.sources);
                }
                if !p.rationale.is_empty() {
                    s += &format!("  nota IA (non verificata): {}\n", p.rationale);
                }
                s += "  [Annulla]";
                s
            }
            ShellState::Result { action, outcome, .. } => format!("[result] {action:?}: {outcome:?}  [Chiudi]"),
            ShellState::Error { error, .. } => format!("[error] {error:?}  [Chiudi]"),
        }
    }
}
