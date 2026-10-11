//! Exact-input cache of compiler proposals. Never stores capabilities or authorization.
use crate::{Router, ValidatedPlan};
use anyhow::{Result, ensure};
use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};

pub struct ProposalCache {
    entries: BTreeMap<[u8; 32], (Instant, String)>,
    capacity: usize,
    ttl: Duration,
}
impl ProposalCache {
    pub fn new(capacity: usize, ttl: Duration) -> Result<Self> {
        ensure!(
            (1..=4096).contains(&capacity) && !ttl.is_zero() && ttl <= Duration::from_secs(120),
            "cache limits"
        );
        Ok(Self {
            entries: BTreeMap::new(),
            capacity,
            ttl,
        })
    }
    fn key(request: &str, caller: &str, focus: bool, policy_epoch: &str) -> [u8; 32] {
        let mut hash = blake3::Hasher::new();
        for part in [
            crate::REGISTRY_EPOCH,
            crate::PROMPT,
            policy_epoch,
            caller,
            request,
        ] {
            hash.update(&(part.len() as u64).to_le_bytes());
            hash.update(part.as_bytes());
        }
        hash.update(&[focus as u8]);
        *hash.finalize().as_bytes()
    }
    pub fn remember(
        &mut self,
        request: &str,
        caller: &str,
        focus: bool,
        policy_epoch: &str,
        proposal: &str,
    ) -> Result<()> {
        ensure!(proposal.len() <= 128, "oversized proposal");
        serde_json::from_str::<crate::CompilerOutput>(proposal)?;
        self.entries
            .retain(|_, (created, _)| created.elapsed() < self.ttl);
        let key = Self::key(request, caller, focus, policy_epoch);
        if !self.entries.contains_key(&key) && self.entries.len() == self.capacity {
            let oldest = *self
                .entries
                .iter()
                .min_by_key(|(_, (created, _))| *created)
                .unwrap()
                .0;
            self.entries.remove(&oldest);
        }
        self.entries
            .insert(key, (Instant::now(), proposal.to_owned()));
        Ok(())
    }
    /// Every hit rechecks the supplied current capability against the trusted router.
    pub fn resolve(
        &self,
        router: &Router,
        request: &str,
        caller: &str,
        focus: bool,
        policy_epoch: &str,
        current_token: &[u8],
    ) -> Result<Option<ValidatedPlan>> {
        let Some((created, proposal)) =
            self.entries
                .get(&Self::key(request, caller, focus, policy_epoch))
        else {
            return Ok(None);
        };
        if created.elapsed() >= self.ttl {
            return Ok(None);
        }
        router
            .validate(proposal, request, caller, focus, current_token)
            .map(Some)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use biscuit_auth::{KeyPair, builder::BiscuitBuilder};
    #[test]
    fn cache_hit_never_reuses_authority_and_context_changes_miss() -> Result<()> {
        let root = KeyPair::new();
        let full = BiscuitBuilder::new().code("right(\"alice\",\"NetConfig\",\"read\"); right(\"alice\",\"NetConfig\",\"write\");")?.build(&root)?.to_vec()?;
        let read = BiscuitBuilder::new()
            .code("right(\"alice\",\"NetConfig\",\"read\");")?
            .build(&root)?
            .to_vec()?;
        let router = Router::new(root.public());
        let mut cache = ProposalCache::new(1, Duration::from_secs(1))?;
        cache.remember(
            "restore CafeGuest",
            "alice",
            true,
            "policy1",
            "{\"intent\":\"restore\"}",
        )?;
        assert!(
            cache
                .resolve(
                    &router,
                    "restore CafeGuest",
                    "alice",
                    true,
                    "policy1",
                    &full
                )?
                .is_some()
        );
        assert!(
            cache
                .resolve(
                    &router,
                    "restore CafeGuest",
                    "alice",
                    true,
                    "policy1",
                    &read
                )
                .is_err()
        );
        for (request, caller, focus, epoch) in [
            ("restore Home", "alice", true, "policy1"),
            ("restore CafeGuest", "bob", true, "policy1"),
            ("restore CafeGuest", "alice", false, "policy1"),
            ("restore CafeGuest", "alice", true, "policy2"),
        ] {
            assert!(
                cache
                    .resolve(&router, request, caller, focus, epoch, &full)?
                    .is_none()
            );
        }
        cache.entries.values_mut().next().unwrap().0 = Instant::now() - Duration::from_secs(2);
        assert!(
            cache
                .resolve(
                    &router,
                    "restore CafeGuest",
                    "alice",
                    true,
                    "policy1",
                    &full
                )?
                .is_none()
        );
        Ok(())
    }
}
