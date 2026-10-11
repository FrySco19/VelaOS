use anyhow::{Result, anyhow, ensure};
use biscuit_auth::{AuthorizerLimits, Biscuit, PublicKey, builder::AuthorizerBuilder};
use ed25519_dalek::{Signature, Signer, SigningKey};
use redb::{Database, Durability, ReadableDatabase, ReadableTable, TableDefinition};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{sync::Mutex, time::Duration};
use vela_lfs_m1::Lfs;
use vela_neural_bus_m2::authority::{CapabilityOperation, CapabilityRequest, TaskAuthority};
use zeroize::Zeroizing;

const HEAD: TableDefinition<u8, &[u8]> = TableDefinition::new("m4_provider_head_v1");
const SERVICE: &str = "m4_provider";

pub fn oid_ok(s: &str) -> Result<()> {
    ensure!(
        s.len() == 64
            && s.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
        "invalid OID/domain/operation ID"
    );
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Scope {
    pub id: String,
    pub domain: String,
    pub from: String, // rollback source = target of forward apply
    pub to: String,   // rollback destination = previous active object
}
impl Scope {
    pub fn validate(&self) -> Result<()> {
        for s in [&self.id, &self.domain, &self.from, &self.to] {
            oid_ok(s)?;
        }
        Ok(())
    }
}
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
enum Outcome {
    Kept,
    Reverted,
}
#[derive(Clone, Serialize, Deserialize)]
struct Head {
    active: String,
    pending: Option<Scope>,
    completed: Option<(Scope, Outcome)>,
}
#[derive(Serialize, Deserialize)]
struct SignedHead {
    state: Head,
    signature: Vec<u8>,
}

/// Isolated file-backed property domain, not an OS network-interface driver.
/// LFS stores the actual encrypted values. This private metadata DB stores only OIDs.
pub struct PersistentProvider {
    domain: String,
    lfs: Mutex<Lfs>,
    metadata: Database,
    signing: SigningKey,
    lfs_root: PublicKey,
    rollback_root: PublicKey,
    service_token: Zeroizing<Vec<u8>>,
}
/// Protected bootstrap inputs; never accepted from planner output.
pub struct ProviderConfig {
    pub domain: String,
    pub signing: SigningKey,
    pub lfs_root: PublicKey,
    pub rollback_root: PublicKey,
    pub service_token: Vec<u8>,
}
impl PersistentProvider {
    /// Trusted native ingress only: object bytes never enter the planner.
    pub fn save_authorized(
        &self,
        token: &[u8],
        caller: &str,
        idempotency: &str,
        value: &Value,
    ) -> Result<vela_lfs_m1::Committed> {
        self.lfs
            .lock()
            .map_err(|_| anyhow!("LFS poisoned"))?
            .put(token, caller, idempotency, value)
    }

    /// Only semantic cards leave the native storage boundary.
    pub fn cards_authorized(
        &self,
        token: &[u8],
        caller: &str,
        after_commit: u64,
    ) -> Result<Vec<vela_lfs_m1::Listed>> {
        self.lfs
            .lock()
            .map_err(|_| anyhow!("LFS poisoned"))?
            .list(token, caller, after_commit, 100)
    }
    pub fn new(
        lfs: Lfs,
        metadata: Database,
        config: ProviderConfig,
        initial: &Value,
    ) -> Result<Self> {
        let ProviderConfig {
            domain,
            signing,
            lfs_root,
            rollback_root,
            service_token,
        } = config;
        oid_ok(&domain)?;
        ensure!(service_token.len() <= 16384, "oversized service capability");
        let this = Self {
            domain,
            lfs: Mutex::new(lfs),
            metadata,
            signing,
            lfs_root,
            rollback_root,
            service_token: Zeroizing::new(service_token),
        };
        let exists = {
            let mut tx = this.metadata.begin_write()?;
            tx.set_durability(Durability::Immediate)?;
            let exists = tx.open_table(HEAD)?.get(0)?.is_some();
            tx.commit()?;
            exists
        };
        if !exists {
            let object = this.admit(initial)?;
            this.write(&Head {
                active: object,
                pending: None,
                completed: None,
            })?;
        }
        this.active_oid()?;
        Ok(this)
    }
    fn read(&self) -> Result<Head> {
        let tx = self.metadata.begin_read()?;
        let table = tx.open_table(HEAD)?;
        let bytes = table
            .get(0)?
            .ok_or_else(|| anyhow!("missing provider head"))?;
        ensure!(bytes.value().len() <= 4096, "oversized provider metadata");
        let record: SignedHead = serde_json::from_slice(bytes.value())?;
        self.signing.verifying_key().verify_strict(
            &serde_jcs::to_vec(&("vela.provider.head.v1", &self.domain, &record.state))?,
            &Signature::from_slice(&record.signature)?,
        )?;
        Ok(record.state)
    }
    fn write(&self, state: &Head) -> Result<()> {
        let signed = SignedHead {
            signature: self
                .signing
                .sign(&serde_jcs::to_vec(&(
                    "vela.provider.head.v1",
                    &self.domain,
                    state,
                ))?)
                .to_bytes()
                .to_vec(),
            state: state.clone(),
        };
        let bytes = serde_json::to_vec(&signed)?;
        ensure!(bytes.len() <= 4096, "oversized provider metadata");
        let mut tx = self.metadata.begin_write()?;
        tx.set_durability(Durability::Immediate)?;
        {
            tx.open_table(HEAD)?.insert(0, bytes.as_slice())?;
        }
        tx.commit()?;
        Ok(())
    }
    fn load_locked(&self, lfs: &Lfs, object: &str) -> Result<Value> {
        oid_ok(object)?;
        let exact = lfs.handle(&self.service_token, SERVICE, object, 120)?;
        let broker = TaskAuthority::new(self.lfs_root, 4096)?;
        let handle = broker.issue(
            &self.service_token,
            CapabilityRequest {
                caller: SERVICE.into(),
                resource: "NetConfig".into(),
                operations: vec![CapabilityOperation::Read],
                byte_quota: 4096,
                ttl: Duration::from_secs(120),
            },
            Some(exact),
        )?;
        let bytes = broker.open_lfs_via_bus(lfs, &handle, SERVICE)?;
        let value: Value = serde_json::from_slice(&bytes)?;
        ensure!(
            vela_lfs_m1::oid(&value)? == object,
            "provider effect verification failed"
        );
        Ok(value)
    }
    pub fn admit(&self, value: &Value) -> Result<String> {
        let mut lfs = self.lfs.lock().map_err(|_| anyhow!("provider poisoned"))?;
        let object = vela_lfs_m1::oid(value)?;
        let committed = lfs.put(
            &self.service_token,
            SERVICE,
            &format!("provider-{object}"),
            value,
        )?;
        self.load_locked(&lfs, &committed.oid)?;
        Ok(committed.oid)
    }
    pub fn snapshot(&self) -> Result<(String, Value)> {
        let lfs = self.lfs.lock().map_err(|_| anyhow!("provider poisoned"))?;
        let state = self.read()?;
        Ok((state.active.clone(), self.load_locked(&lfs, &state.active)?))
    }
    /// Control-plane response contains only an OID; verification bytes stay here.
    pub fn active_oid(&self) -> Result<String> {
        let lfs = self.lfs.lock().map_err(|_| anyhow!("provider poisoned"))?;
        let state = self.read()?;
        self.load_locked(&lfs, &state.active)?;
        Ok(state.active)
    }
    pub fn domain(&self) -> &str {
        &self.domain
    }
    pub fn stage(&self, scope: &Scope) -> Result<()> {
        scope.validate()?;
        let lfs = self.lfs.lock().map_err(|_| anyhow!("provider poisoned"))?;
        let mut state = self.read()?;
        ensure!(
            scope.domain == self.domain && state.pending.is_none() && state.active == scope.to,
            "stage conflict"
        );
        self.load_locked(&lfs, &scope.from)?;
        self.load_locked(&lfs, &scope.to)?;
        state.pending = Some(scope.clone());
        self.write(&state)
    }
    pub fn activate(&self, scope: &Scope) -> Result<()> {
        let lfs = self.lfs.lock().map_err(|_| anyhow!("provider poisoned"))?;
        let mut state = self.read()?;
        ensure!(
            state.pending.as_ref() == Some(scope) && state.active == scope.to,
            "apply conflict"
        );
        self.load_locked(&lfs, &scope.from)?;
        // The object was durably staged in M1 first. This transaction publishes
        // only one logical head; no multi-database/device atomicity is claimed.
        state.active = scope.from.clone();
        self.write(&state)?;
        let observed = self.read()?;
        ensure!(observed.active == scope.from, "head verification failed");
        self.load_locked(&lfs, &observed.active)?;
        Ok(())
    }
    pub(super) fn check_rollback(&self, scope: &Scope, capability: &[u8]) -> Result<()> {
        scope.validate()?;
        ensure!(
            scope.domain == self.domain && capability.len() <= 8192,
            "rollback scope invalid"
        );
        let biscuit = Biscuit::from(capability, self.rollback_root)?;
        AuthorizerBuilder::new().set_limits(AuthorizerLimits {
            max_facts: 64, max_iterations: 8, max_time: Duration::from_millis(25),
        }).code(format!(
            "actor(\"m4_supervisor\"); operation(\"rollback\"); transaction(\"{}\"); domain(\"{}\"); from_oid(\"{}\"); to_oid(\"{}\"); allow if rollback(\"{}\",\"{}\",\"{}\",\"{}\");",
            scope.id, scope.domain, scope.from, scope.to, scope.id, scope.domain, scope.from, scope.to,
        ))?.build(&biscuit)?.authorize()?;
        Ok(())
    }
    pub fn revert(&self, scope: &Scope, capability: &[u8]) -> Result<()> {
        self.check_rollback(scope, capability)?;
        let lfs = self.lfs.lock().map_err(|_| anyhow!("provider poisoned"))?;
        let mut state = self.read()?;
        if state.pending.is_none() {
            // Idempotent recovery after head commit but before journal completion.
            ensure!(
                state.active == scope.to
                    && state.completed.as_ref() == Some(&(scope.clone(), Outcome::Reverted)),
                "rollback revoked or not staged"
            );
            self.load_locked(&lfs, &scope.to)?;
            return Ok(());
        }
        ensure!(
            state.pending.as_ref() == Some(scope)
                && (state.active == scope.from || state.active == scope.to),
            "rollback conflict: preserving third-party state"
        );
        self.load_locked(&lfs, &scope.to)?;
        state.active = scope.to.clone();
        state.pending = None;
        state.completed = Some((scope.clone(), Outcome::Reverted));
        self.write(&state)?;
        let observed = self.read()?;
        ensure!(observed.active == scope.to, "rollback verification failed");
        self.load_locked(&lfs, &observed.active)?;
        Ok(())
    }
    pub fn keep(&self, scope: &Scope) -> Result<()> {
        let lfs = self.lfs.lock().map_err(|_| anyhow!("provider poisoned"))?;
        let mut state = self.read()?;
        if state.pending.is_none() {
            ensure!(
                state.active == scope.from
                    && state.completed.as_ref() == Some(&(scope.clone(), Outcome::Kept)),
                "keep receipt mismatch"
            );
            self.load_locked(&lfs, &state.active)?;
            return Ok(());
        }
        ensure!(
            state.pending.as_ref() == Some(scope) && state.active == scope.from,
            "keep conflict"
        );
        self.load_locked(&lfs, &scope.from)?;
        state.pending = None;
        state.completed = Some((scope.clone(), Outcome::Kept));
        self.write(&state)
    }
    pub fn unstaged(&self, scope: &Scope) -> Result<bool> {
        let _guard = self.lfs.lock().map_err(|_| anyhow!("provider poisoned"))?;
        let state = self.read()?;
        Ok(state.pending.is_none()
            && state.active == scope.to
            && state
                .completed
                .as_ref()
                .is_none_or(|(old, _)| old.id != scope.id))
    }
}
