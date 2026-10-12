//! Shared fixtures for the M-G1b pipeline and LFS receipt tests.
#![allow(dead_code)]

use biscuit_auth::{KeyPair as RootKeys, builder::BiscuitBuilder};
use ed25519_compact::{KeyPair as EdKeys, Seed};
use std::time::{Duration, SystemTime};
use vela_executor_policy::grant::{Candidate, Handle, TrustedSelectionChannel};
use vela_executor_policy::pipeline::{
    IndexEntry, MockPlanner, RETURN_21_MODULE, SelectionGatedPdp, TaskRequest,
};

pub const USER: &str = "vela-user";
pub const NOTES: Handle = [0xA1; 32];
pub const PHOTOS: Handle = [0xB2; 32];
pub const SECRETS: Handle = [0xC3; 32];
pub const INTENT: &str = "riassumi le note progetto VelaOS";

pub fn index() -> Vec<IndexEntry> {
    vec![
        IndexEntry { handle: NOTES, version: 3, title: "Note progetto VelaOS (bozza)" },
        IndexEntry { handle: PHOTOS, version: 1, title: "Foto vacanze note" },
        IndexEntry { handle: SECRETS, version: 9, title: "Chiavi private" },
    ]
}

pub fn candidates() -> Vec<Candidate> {
    MockPlanner::propose(INTENT, &index())
}

pub fn pcb() -> EdKeys {
    EdKeys::from_seed(Seed::new([41; 32]))
}
pub fn lfs_keys() -> EdKeys {
    EdKeys::from_seed(Seed::new([53; 32]))
}

pub fn delegation(root: &RootKeys, resource: &str, op: &str) -> Vec<u8> {
    BiscuitBuilder::new()
        .code(format!("right(\"{USER}\", \"{resource}\", \"{op}\");"))
        .unwrap()
        .build(root)
        .unwrap()
        .to_vec()
        .unwrap()
}

pub fn gated(root: &RootKeys) -> SelectionGatedPdp {
    SelectionGatedPdp::new(
        root.public(),
        delegation(root, "lfs", "read"),
        delegation(root, "executor", "execute"),
        pcb().sk,
    )
    .unwrap()
}

pub struct Fixture {
    pub root: RootKeys,
    pub channel: TrustedSelectionChannel,
    pub candidates: Vec<Candidate>,
    pub now: SystemTime,
}

pub fn fixture() -> Fixture {
    Fixture {
        root: RootKeys::new(),
        channel: TrustedSelectionChannel::new(),
        candidates: candidates(),
        now: SystemTime::now(),
    }
}

pub fn task<'a>(c: &'a [Candidate], requested: Handle, request_id: u32) -> TaskRequest<'a> {
    TaskRequest {
        caller: USER.into(),
        requested,
        candidates: c,
        selection: None,
        ttl: Duration::from_secs(60),
        planner_note: String::new(),
        request_id,
        module: RETURN_21_MODULE.to_vec(),
        fuel: 10_000,
    }
}
