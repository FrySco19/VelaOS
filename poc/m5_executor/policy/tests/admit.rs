//! M-G1a host tests: Biscuit decision -> (only on ALLOW) Executor v2 signed frame.

use biscuit_auth::{KeyPair as RootKeys, builder::BiscuitBuilder, builder::BlockBuilder};
use ed25519_compact::{KeyPair as PcbKeys, Seed};
use std::time::{Duration, SystemTime};
use vela_executor_policy::{
    AdmitRequest, AdmitResult, Attenuation, DenyCode, PolicyDecisionPoint, attenuate, bridge,
};

const CALLER: &str = "pcb-planner";

fn pcb() -> PcbKeys {
    PcbKeys::from_seed(Seed::new([41; 32]))
}

fn parent(root: &RootKeys, rights: &[(&str, &str)]) -> Vec<u8> {
    let facts: String = rights
        .iter()
        .map(|(res, op)| format!("right(\"{CALLER}\", \"{res}\", \"{op}\");\n"))
        .collect();
    BiscuitBuilder::new()
        .code(&facts)
        .unwrap()
        .build(root)
        .unwrap()
        .to_vec()
        .unwrap()
}

fn scope(now: SystemTime) -> Attenuation {
    Attenuation {
        caller: Some(CALLER.into()),
        resource: Some("NetConfig".into()),
        operation: Some("execute".into()),
        max_payload_bytes: Some(64),
        max_fuel: Some(1000),
        expires_at: Some(now + Duration::from_secs(60)),
    }
}

fn request(token: Vec<u8>, resource: &str, request_id: u32) -> AdmitRequest {
    AdmitRequest {
        token,
        caller: CALLER.into(),
        resource: resource.into(),
        operation: "execute".into(),
        request_id,
        payload: bridge::RETURN_7_MODULE.to_vec(),
        fuel: 1000,
    }
}

fn setup() -> (RootKeys, PolicyDecisionPoint, SystemTime) {
    let root = RootKeys::new();
    let pdp = PolicyDecisionPoint::new(root.public(), pcb().sk).unwrap();
    (root, pdp, SystemTime::now())
}

fn attenuated(root: &RootKeys, now: SystemTime) -> Vec<u8> {
    let parent = parent(root, &[("NetConfig", "execute"), ("Secrets", "execute")]);
    attenuate(&parent, root.public(), &scope(now)).unwrap()
}

fn assert_denied(pdp: &PolicyDecisionPoint, result: AdmitResult, code: DenyCode) {
    match result {
        AdmitResult::Deny { code: got, reason } => {
            assert_eq!(got, code, "unexpected deny code: {reason}")
        }
        AdmitResult::Allow { .. } => panic!("expected deny {code:?}, got allow"),
    }
    assert_eq!(pdp.frames_signed(), 0, "a deny must sign nothing");
}

#[test]
fn in_scope_attenuated_token_yields_frame_verified_by_pcb_key() {
    let (root, mut pdp, now) = setup();
    let result = pdp.admit_at(&request(attenuated(&root, now), "NetConfig", 1), now);
    let AdmitResult::Allow { frame } = result else {
        panic!("expected allow: {result:?}")
    };
    let verified = bridge::verify_with_key(&frame, 0, pcb().pk).expect("executor accepts");
    assert_eq!(verified.descriptor.request_id, 1);
    assert_eq!(verified.fuel, 1000);
    let d = verified.descriptor;
    assert_eq!(
        &frame[d.payload_offset..d.payload_offset + d.payload_len],
        bridge::RETURN_7_MODULE
    );
    // The frame is signed by the PCB key, not by the Biscuit root.
    let other = PcbKeys::from_seed(Seed::new([42; 32]));
    assert_eq!(
        bridge::verify_with_key(&frame, 0, other.pk).unwrap_err(),
        bridge::Status::Authentication
    );
    assert_eq!(pdp.frames_signed(), 1);
    assert_eq!(pdp.last_admitted(), 1);
}

#[test]
fn unattenuated_parent_with_right_is_allowed() {
    let (root, mut pdp, now) = setup();
    let token = parent(&root, &[("NetConfig", "execute")]);
    assert!(
        pdp.admit_at(&request(token, "NetConfig", 1), now)
            .is_allow()
    );
}

#[test]
fn out_of_scope_resource_is_policy_denied() {
    let (root, mut pdp, now) = setup();
    // Parent holds Secrets/execute, but the attenuation narrowed it to NetConfig.
    let result = pdp.admit_at(&request(attenuated(&root, now), "Secrets", 1), now);
    assert_denied(&pdp, result, DenyCode::PolicyDenied);
}

#[test]
fn other_caller_or_operation_is_policy_denied() {
    let (root, mut pdp, now) = setup();
    let mut r = request(attenuated(&root, now), "NetConfig", 1);
    r.caller = "intruder".into();
    let result = pdp.admit_at(&r, now);
    assert_denied(&pdp, result, DenyCode::PolicyDenied);
    let mut r = request(attenuated(&root, now), "NetConfig", 1);
    r.operation = "write".into();
    let result = pdp.admit_at(&r, now);
    assert_denied(&pdp, result, DenyCode::PolicyDenied);
}

#[test]
fn wrong_root_is_authentication_denied() {
    let (_root, mut pdp, now) = setup();
    let forger = RootKeys::new();
    let result = pdp.admit_at(&request(attenuated(&forger, now), "NetConfig", 1), now);
    assert_denied(&pdp, result, DenyCode::Authentication);
    let result = pdp.admit_at(&request(vec![0xA5; 64], "NetConfig", 1), now);
    assert_denied(&pdp, result, DenyCode::Authentication);
}

#[test]
fn expired_token_is_policy_denied() {
    let (root, mut pdp, now) = setup();
    let token = attenuated(&root, now);
    let later = now + Duration::from_secs(61);
    let result = pdp.admit_at(&request(token, "NetConfig", 1), later);
    assert_denied(&pdp, result, DenyCode::PolicyDenied);
}

#[test]
fn missing_right_is_policy_denied() {
    let (root, mut pdp, now) = setup();
    let token = parent(&root, &[("NetConfig", "read")]);
    let result = pdp.admit_at(&request(token, "NetConfig", 1), now);
    assert_denied(&pdp, result, DenyCode::PolicyDenied);
}

#[test]
fn attenuated_byte_and_fuel_quotas_are_enforced() {
    let (root, mut pdp, now) = setup();
    let parent = parent(&root, &[("NetConfig", "execute")]);
    let tight = Attenuation {
        max_payload_bytes: Some(16),
        ..Default::default()
    };
    let token = attenuate(&parent, root.public(), &tight).unwrap();
    let result = pdp.admit_at(&request(token, "NetConfig", 1), now);
    assert_denied(&pdp, result, DenyCode::PolicyDenied);

    let mut r = request(attenuated(&root, now), "NetConfig", 1);
    r.fuel = 1001; // within executor MAX_FUEL, above attenuated max_fuel
    let result = pdp.admit_at(&r, now);
    assert_denied(&pdp, result, DenyCode::PolicyDenied);
}

#[test]
fn attenuation_block_facts_cannot_grant_rights() {
    let (root, mut pdp, now) = setup();
    let parent = parent(&root, &[("NetConfig", "read")]);
    let token = Biscuit_append(
        &root,
        &parent,
        "right(\"pcb-planner\", \"NetConfig\", \"execute\");",
    );
    let result = pdp.admit_at(&request(token, "NetConfig", 1), now);
    assert_denied(&pdp, result, DenyCode::PolicyDenied);
}

#[allow(non_snake_case)]
fn Biscuit_append(root: &RootKeys, token: &[u8], code: &str) -> Vec<u8> {
    biscuit_auth::Biscuit::from(token, root.public())
        .unwrap()
        .append(BlockBuilder::new().code(code).unwrap())
        .unwrap()
        .to_vec()
        .unwrap()
}

#[test]
fn replayed_or_stale_request_ids_are_denied_and_denies_do_not_advance() {
    let (root, mut pdp, now) = setup();
    let token = attenuated(&root, now);
    let result = pdp.admit_at(&request(token.clone(), "NetConfig", 0), now);
    assert_denied(&pdp, result, DenyCode::Replay);
    let result = pdp.admit_at(&request(token.clone(), "Secrets", 7), now);
    assert_denied(&pdp, result, DenyCode::PolicyDenied);
    assert_eq!(pdp.last_admitted(), 0);
    assert!(
        pdp.admit_at(&request(token.clone(), "NetConfig", 5), now)
            .is_allow()
    );
    for id in [5, 4] {
        let r = pdp.admit_at(&request(token.clone(), "NetConfig", id), now);
        assert_eq!(r.deny_code(), Some(DenyCode::Replay));
    }
    assert_eq!(pdp.frames_signed(), 1);
}

#[test]
fn host_bounds_and_malformed_requests_are_denied_before_biscuit() {
    let (root, mut pdp, now) = setup();
    let token = attenuated(&root, now);
    let mut r = request(token.clone(), "NetConfig", 1);
    r.fuel = bridge::MAX_FUEL + 1;
    let result = pdp.admit_at(&r, now);
    assert_denied(&pdp, result, DenyCode::Scope);
    let mut r = request(token.clone(), "NetConfig", 1);
    r.payload.clear();
    let result = pdp.admit_at(&r, now);
    assert_denied(&pdp, result, DenyCode::Malformed);
    let mut r = request(token.clone(), "NetConfig", 1);
    r.payload = vec![0; vela_executor_policy::MAX_PAYLOAD_BYTES + 1];
    let result = pdp.admit_at(&r, now);
    assert_denied(&pdp, result, DenyCode::Malformed);
    let result = pdp.admit_at(&request(vec![0; 16 * 1024 + 1], "NetConfig", 1), now);
    assert_denied(&pdp, result, DenyCode::Malformed);
    // Datalog injection attempt through an identifier never reaches the authorizer.
    let result = pdp.admit_at(&request(token, "NetConfig\"); right(\"x", 1), now);
    assert_denied(&pdp, result, DenyCode::Malformed);
}

#[test]
fn biscuit_root_and_pcb_signing_key_must_differ() {
    let seed = [9u8; 32];
    let biscuit_private =
        biscuit_auth::PrivateKey::from_bytes(&seed, biscuit_auth::builder::Algorithm::Ed25519)
            .unwrap();
    let root = RootKeys::from(&biscuit_private);
    let pcb = PcbKeys::from_seed(Seed::new(seed));
    assert!(PolicyDecisionPoint::new(root.public(), pcb.sk).is_err());
}
