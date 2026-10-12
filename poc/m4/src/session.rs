//! Trusted native session. Ranking proposes candidates; a human selects an exact OID.
//! This is a host API, not a wire protocol or a completed consumer interface.
use crate::{
    provider::{PersistentProvider, oid_ok},
    supervisor::{AuthenticatedClient, Supervisor, Ticket},
};
use anyhow::{Result, ensure};
use serde_json::Value;
use vela_lfs_m1::{Committed, Listed};

/// Finite UI state; confirmation secrets never appear in model output or metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionPhase {
    Browsing,
    Prepared,
    AwaitingKeep,
}

pub struct Session {
    client: AuthenticatedClient,
    candidates: Vec<String>,
    confirmation: Option<Ticket>,
    keep: Option<Ticket>,
}
impl Session {
    pub fn new(client: AuthenticatedClient) -> Self {
        Self {
            client,
            candidates: Vec::new(),
            confirmation: None,
            keep: None,
        }
    }
    pub fn phase(&self) -> SessionPhase {
        if self.keep.is_some() {
            SessionPhase::AwaitingKeep
        } else if self.confirmation.is_some() {
            SessionPhase::Prepared
        } else {
            SessionPhase::Browsing
        }
    }
    pub fn save(
        &self,
        provider: &PersistentProvider,
        idempotency: &str,
        value: &Value,
    ) -> Result<Committed> {
        let (caller, token) = self.client.credentials();
        provider.save_authorized(token, caller, idempotency, value)
    }
    pub fn cards(
        &mut self,
        provider: &PersistentProvider,
        after_commit: u64,
    ) -> Result<Vec<Listed>> {
        ensure!(
            self.phase() == SessionPhase::Browsing,
            "finish pending operation first"
        );
        // Clear old results even when fresh authorization fails.
        self.candidates.clear();
        let (caller, token) = self.client.credentials();
        let cards = provider.cards_authorized(token, caller, after_commit)?;
        self.candidates = cards.iter().map(|card| card.oid.clone()).collect();
        Ok(cards)
    }
    /// Accept a bounded untrusted M0 ranking, intersecting it with authorized cards.
    /// Ranking does not select a target or confer read/write authority.
    pub fn rank(&mut self, ordered_oids: &[String]) -> Result<()> {
        ensure!(self.phase() == SessionPhase::Browsing, "pending operation");
        ensure!(
            !ordered_oids.is_empty() && ordered_oids.len() <= 100,
            "invalid ranking size"
        );
        let mut accepted = Vec::with_capacity(ordered_oids.len());
        for oid in ordered_oids {
            oid_ok(oid)?;
            ensure!(
                self.candidates.contains(oid),
                "ranking contains unauthorized object"
            );
            ensure!(!accepted.contains(oid), "duplicate candidate");
            accepted.push(oid.clone());
        }
        self.candidates = accepted;
        Ok(())
    }
    /// The trusted host supplies the pinned M0 adapter. Only authorized card
    /// metadata crosses this callback; failure invalidates all cached candidates.
    pub fn recall_with(
        &mut self,
        provider: &PersistentProvider,
        query: &str,
        ranker: impl FnOnce(&[Listed], &str) -> Result<Vec<String>>,
    ) -> Result<Vec<String>> {
        ensure!(
            !query.trim().is_empty() && query.len() <= 4096,
            "invalid recall query"
        );
        let cards = self.cards(provider, 0)?;
        let ranked = match ranker(&cards, query) {
            Ok(ranked) => ranked,
            Err(error) => {
                self.candidates.clear();
                return Err(error);
            }
        };
        if let Err(error) = self.rank(&ranked) {
            self.candidates.clear();
            return Err(error);
        }
        Ok(self.candidates.clone())
    }
    /// Called by the trusted human-selection event, never automatically by the SLM.
    pub fn select(&mut self, supervisor: &Supervisor, oid: &str) -> Result<()> {
        ensure!(self.phase() == SessionPhase::Browsing, "pending operation");
        oid_ok(oid)?;
        ensure!(
            self.candidates.iter().any(|candidate| candidate == oid),
            "select an authorized candidate"
        );
        self.confirmation = Some(supervisor.prepare(&self.client, oid)?);
        self.candidates.clear();
        Ok(())
    }
    pub fn confirm(&mut self, supervisor: &Supervisor) -> Result<()> {
        let ticket = self
            .confirmation
            .take()
            .ok_or_else(|| anyhow::anyhow!("no confirmation pending"))?;
        self.keep = Some(supervisor.confirm(&self.client, ticket)?);
        Ok(())
    }
    pub fn keep(&mut self, supervisor: &Supervisor) -> Result<()> {
        let ticket = self
            .keep
            .take()
            .ok_or_else(|| anyhow::anyhow!("no keep pending"))?;
        supervisor.keep(&self.client, ticket)
    }
    // Dropping the session does not finalize effects: the persistent supervisor
    // retains the deadline and performs rollback independently of this client.
}
