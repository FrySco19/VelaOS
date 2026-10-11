//! Control-plane capability checks and typed data-plane primitives. No LLM invocation.
pub mod authority;
pub mod sandbox;
pub mod streaming;

use anyhow::{Result, ensure};
use biscuit_auth::{AuthorizerLimits, Biscuit, PublicKey, builder::AuthorizerBuilder};
use iceoryx2::prelude::ZeroCopySend;
use rand::{RngCore, rngs::OsRng};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use subtle::ConstantTimeEq;
use zeroize::Zeroizing;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Schema {
    NetConfigV1,
    Bulk4KiB,
    BulkStreamV1,
}
impl Schema {
    pub fn tag(self) -> u32 {
        match self {
            Self::NetConfigV1 => 1,
            Self::Bulk4KiB => 2,
            Self::BulkStreamV1 => 3,
        }
    }
    fn right(self) -> &'static str {
        match self {
            Self::NetConfigV1 => "NetConfig",
            Self::Bulk4KiB => "Bulk4KiB",
            Self::BulkStreamV1 => "BulkStream",
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Route {
    pub channel: String,
    pub schema: Schema,
    pub source: String,
    pub destination: String,
    caller: String,
    id: [u8; 32],
    deadline_ms: u64,
    mac: [u8; 32],
}
pub struct PolicyGate {
    root: PublicKey,
    key: Zeroizing<[u8; 32]>,
    started: Instant,
}
impl PolicyGate {
    pub fn new(root: PublicKey) -> Self {
        let mut key = Zeroizing::new([0; 32]);
        OsRng.fill_bytes(key.as_mut());
        Self {
            root,
            key,
            started: Instant::now(),
        }
    }
    /// Called once by the control plane. Closed manifests: LFS -> viewer only.
    pub fn issue(
        &self,
        token: &[u8],
        caller: &str,
        schema: Schema,
        ttl: Duration,
    ) -> Result<Route> {
        ensure!(
            !caller.is_empty()
                && caller.len() <= 128
                && caller
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b)),
            "invalid caller"
        );
        ensure!(
            token.len() <= 16384 && ttl.as_millis() > 0 && ttl <= Duration::from_secs(120),
            "token or route TTL exceeds bound"
        );
        let token = Biscuit::from(token, self.root)?;
        AuthorizerBuilder::new().set_limits(AuthorizerLimits {max_facts:256,max_iterations:32,max_time:Duration::from_millis(250)})
            .time().code(format!("operation(\"read\"); resource(\"{}\"); allow if right(\"{caller}\",\"{}\",\"read\");",schema.right(),schema.right()))?
            .build(&token)?.authorize()?;
        let mut nonce = [0u8; 32];
        OsRng.fill_bytes(&mut nonce);
        let mut route = Route {
            channel: format!("vela-{}", blake3::hash(&nonce).to_hex()),
            schema,
            source: "lfs".into(),
            destination: "viewer".into(),
            caller: caller.into(),
            id: nonce,
            deadline_ms: u64::try_from(self.started.elapsed().as_millis())?
                + u64::try_from(ttl.as_millis())?,
            mac: [0; 32],
        };
        route.mac = self.mac(&route)?;
        Ok(route)
    }
    fn mac(&self, r: &Route) -> Result<[u8; 32]> {
        Ok(*blake3::keyed_hash(
            &self.key,
            &serde_json::to_vec(&(
                "vela.bus.route.v1",
                &r.channel,
                r.schema,
                &r.source,
                &r.destination,
                &r.caller,
                r.id,
                r.deadline_ms,
            ))?,
        )
        .as_bytes())
    }
    pub fn validate(&self, r: &Route, caller: &str, schema: Schema) -> Result<()> {
        ensure!(
            r.caller == caller
                && r.schema == schema
                && r.source == "lfs"
                && r.destination == "viewer",
            "route binding mismatch"
        );
        ensure!(
            self.started.elapsed().as_millis() < u128::from(r.deadline_ms),
            "route expired"
        );
        ensure!(bool::from(r.mac.ct_eq(&self.mac(r)?)), "route MAC mismatch");
        Ok(())
    }
    /// Secret endpoint credential is delivered only after validating an issued route.
    pub fn endpoint(&self, r: &Route, caller: &str, schema: Schema) -> Result<Endpoint> {
        self.validate(r, caller, schema)?;
        let valid_for_ms = r
            .deadline_ms
            .checked_sub(u64::try_from(self.started.elapsed().as_millis())?)
            .ok_or_else(|| anyhow::anyhow!("route expired"))?;
        Ok(Endpoint {
            route: r.id,
            schema: r.schema,
            key: *blake3::keyed_hash(&self.key, &r.id).as_bytes(),
            expires_ms: u64::try_from(SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis())?
                + valid_for_ms,
            valid_for_ms,
            started: Instant::now(),
        })
    }
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Endpoint {
    route: [u8; 32],
    schema: Schema,
    key: [u8; 32],
    expires_ms: u64,
    valid_for_ms: u64,
    #[serde(skip, default = "Instant::now")]
    started: Instant,
}
impl Drop for Endpoint {
    fn drop(&mut self) {
        zeroize::Zeroize::zeroize(&mut self.key);
    }
}

/// Fixed-layout schema: no Vec, String, references or pointers in shared memory.
#[repr(C)]
#[derive(Debug, ZeroCopySend)]
pub struct Frame {
    pub sequence: u64,
    pub schema: u32,
    pub length: u32,
    route: [u8; 32],
    mac: [u8; 32],
    pub bytes: [u8; 4096],
}
impl Frame {
    pub fn empty() -> Self {
        Self {
            sequence: 0,
            schema: 0,
            length: 0,
            route: [0; 32],
            mac: [0; 32],
            bytes: [0; 4096],
        }
    }
}
impl Endpoint {
    fn live(&self) -> Result<()> {
        ensure!(
            self.started.elapsed().as_millis() < u128::from(self.valid_for_ms)
                && SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis()
                    < u128::from(self.expires_ms),
            "endpoint expired"
        );
        Ok(())
    }
    /// Fill a loaned shared-memory frame. No allocation or planner in the hot path.
    pub fn stamp(&self, frame: &mut Frame, seq: u64, length: usize) -> Result<()> {
        self.live()?;
        ensure!(length <= 4096, "typed payload too large");
        frame.sequence = seq;
        frame.schema = self.schema.tag();
        frame.length = length as u32;
        frame.route = self.route;
        frame.mac = self.frame_mac(frame);
        Ok(())
    }
    fn frame_mac(&self, f: &Frame) -> [u8; 32] {
        let mut hash = blake3::Hasher::new_keyed(&self.key);
        hash.update(b"vela.bus.frame.v1");
        hash.update(&f.route);
        hash.update(&f.sequence.to_le_bytes());
        hash.update(&f.schema.to_le_bytes());
        hash.update(&f.length.to_le_bytes());
        hash.update(&f.bytes);
        *hash.finalize().as_bytes()
    }
    pub fn verify<'a>(&self, frame: &'a Frame, expected_seq: u64) -> Result<&'a [u8]> {
        self.live()?;
        ensure!(
            frame.route == self.route
                && frame.schema == self.schema.tag()
                && frame.length <= 4096
                && frame.sequence == expected_seq,
            "frame type/route/sequence mismatch"
        );
        ensure!(
            bool::from(frame.mac.ct_eq(&self.frame_mac(frame))),
            "frame authentication failed"
        );
        Ok(&frame.bytes[..frame.length as usize])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use biscuit_auth::{
        KeyPair,
        builder::{BiscuitBuilder, BlockBuilder},
    };
    #[test]
    fn capabilities_routes_and_frames_fail_closed() -> Result<()> {
        let root = KeyPair::new();
        let gate = PolicyGate::new(root.public());
        let token = BiscuitBuilder::new()
            .code("right(\"alice\",\"NetConfig\",\"read\");")?
            .build(&root)?
            .to_vec()?;
        let route = gate.issue(&token, "alice", Schema::NetConfigV1, Duration::from_secs(1))?;
        assert!(
            gate.issue(&token, "bob", Schema::NetConfigV1, Duration::from_secs(1))
                .is_err()
        );
        assert!(
            gate.issue(&token, "alice", Schema::Bulk4KiB, Duration::from_secs(1))
                .is_err()
        );
        let denied = Biscuit::from(&token, root.public())?
            .append(BlockBuilder::new().code("check if operation(\"write\");")?)?
            .to_vec()?;
        assert!(
            gate.issue(
                &denied,
                "alice",
                Schema::NetConfigV1,
                Duration::from_secs(1)
            )
            .is_err()
        );
        let endpoint = gate.endpoint(&route, "alice", Schema::NetConfigV1)?;
        let mut frame = Frame::empty();
        frame.bytes[..3].copy_from_slice(b"abc");
        endpoint.stamp(&mut frame, 7, 3)?;
        assert_eq!(endpoint.verify(&frame, 7)?, b"abc");
        assert!(endpoint.verify(&frame, 8).is_err());
        frame.bytes[0] ^= 1;
        assert!(endpoint.verify(&frame, 7).is_err());
        let mut expired_endpoint = endpoint.clone();
        expired_endpoint.valid_for_ms = 0;
        assert!(expired_endpoint.stamp(&mut frame, 8, 3).is_err());
        let mut bad = route.clone();
        bad.destination = "unauthorized".into();
        assert!(gate.validate(&bad, "alice", Schema::NetConfigV1).is_err());
        bad = route.clone();
        bad.mac[0] ^= 1;
        assert!(gate.validate(&bad, "alice", Schema::NetConfigV1).is_err());
        let mut expired = route;
        expired.deadline_ms = 0;
        expired.mac = gate.mac(&expired)?;
        assert!(
            gate.validate(&expired, "alice", Schema::NetConfigV1)
                .is_err()
        );
        Ok(())
    }
}
