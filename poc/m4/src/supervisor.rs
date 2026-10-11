use crate::{
    Operation, Phase, RestoreCoordinator, now,
    provider::{PersistentProvider, Scope, oid_ok},
};
use anyhow::{Result, anyhow, ensure};
use biscuit_auth::{KeyPair, PublicKey, builder::BiscuitBuilder};
use rand::{RngCore, rngs::OsRng};
use redb::{Durability, ReadableDatabase, TableDefinition};
use serde::{Deserialize, Serialize};
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
use vela_control_plane_m3::Router;
use zeroize::{Zeroize, Zeroizing};

const GRANT: TableDefinition<u8, &[u8]> = TableDefinition::new("m4_exact_recovery_grant_v1");
const TIMER: Duration = Duration::from_secs(120);

/// Provisioned by the protected recovery broker, not by a user/session token.
pub struct RecoveryIssuer {
    key: KeyPair,
}
impl RecoveryIssuer {
    pub fn new(key: KeyPair) -> Self {
        Self { key }
    }
    pub fn public_key(&self) -> PublicKey {
        self.key.public()
    }
    fn issue(&self, scope: &Scope) -> Result<Zeroizing<Vec<u8>>> {
        scope.validate()?;
        let token = BiscuitBuilder::new().code(format!(
            "rollback(\"{}\",\"{}\",\"{}\",\"{}\"); check if actor(\"m4_supervisor\"), operation(\"rollback\"), transaction(\"{}\"), domain(\"{}\"), from_oid(\"{}\"), to_oid(\"{}\");",
            scope.id, scope.domain, scope.from, scope.to, scope.id, scope.domain, scope.from, scope.to,
        ))?.build(&self.key)?.to_vec()?;
        ensure!(token.len() <= 8192, "oversized recovery grant");
        // No inherited user expiration; provider ledger revokes scope on KEEP/revert.
        Ok(Zeroizing::new(token))
    }
}

/// Construct only in the trusted authenticated UI ingress, never from SLM output.
#[derive(Clone)]
pub struct AuthenticatedClient {
    caller: String,
    token: Zeroizing<Vec<u8>>,
    session: [u8; 32],
}
impl AuthenticatedClient {
    pub fn connect(root: PublicKey, caller: String, token: Vec<u8>) -> Result<Self> {
        ensure!(token.len() <= 16384, "oversized client capability");
        Router::new(root).validate(
            "{\"intent\":\"restore\"}",
            "Authenticate restore client",
            &caller,
            true,
            &token,
        )?;
        let mut session = [0; 32];
        OsRng.fill_bytes(&mut session);
        Ok(Self {
            caller,
            token: Zeroizing::new(token),
            session,
        })
    }
    fn authorize(&self, root: PublicKey) -> Result<()> {
        Router::new(root).validate(
            "{\"intent\":\"restore\"}",
            "Authenticated restore action",
            &self.caller,
            true,
            &self.token,
        )?;
        Ok(())
    }
}

/// Not Clone/Serialize. CONFIRM consumes this ticket; KEEP receives a new one.
pub struct Ticket {
    operation: String,
    secret: [u8; 32],
    session: [u8; 32],
}
impl Drop for Ticket {
    fn drop(&mut self) {
        self.secret.zeroize();
        self.session.zeroize();
    }
}
fn ticket(client: &AuthenticatedClient, operation: &str) -> Ticket {
    let mut secret = [0; 32];
    OsRng.fill_bytes(&mut secret);
    Ticket {
        operation: operation.into(),
        secret,
        session: client.session,
    }
}
fn ticket_hash(ticket: &Ticket) -> String {
    let mut hash = blake3::Hasher::new();
    hash.update(b"vela.ui.ticket.v1");
    hash.update(ticket.operation.as_bytes());
    hash.update(&ticket.session);
    hash.update(&ticket.secret);
    hash.finalize().to_hex().to_string()
}

#[derive(Serialize, Deserialize)]
struct StoredGrant {
    scope: Scope,
    token: Vec<u8>,
}
impl Drop for StoredGrant {
    fn drop(&mut self) {
        self.token.zeroize();
    }
}

struct Core {
    journal: RestoreCoordinator,
    provider: Arc<PersistentProvider>,
    issuer: RecoveryIssuer,
    user_root: PublicKey,
    deadline: Option<Instant>,
}
impl Core {
    fn store_grant(&self, grant: &StoredGrant) -> Result<()> {
        let bytes = Zeroizing::new(serde_json::to_vec(grant)?);
        ensure!(bytes.len() <= 12288, "oversized recovery record");
        let mut tx = self.journal.db.begin_write()?;
        tx.set_durability(Durability::Immediate)?;
        {
            tx.open_table(GRANT)?.insert(0, bytes.as_slice())?;
        }
        tx.commit()?;
        Ok(())
    }
    fn grant(&self, op: &Operation) -> Result<StoredGrant> {
        let tx = self.journal.db.begin_read()?;
        let table = tx.open_table(GRANT)?;
        let raw = table
            .get(0)?
            .ok_or_else(|| anyhow!("missing recovery grant"))?;
        ensure!(raw.value().len() <= 12288, "oversized recovery record");
        let grant: StoredGrant = serde_json::from_slice(raw.value())?;
        ensure!(
            grant.scope.id == op.id
                && grant.scope.domain == self.provider.domain()
                && grant.scope.from == op.target
                && grant.scope.to == op.previous,
            "recovery grant binding mismatch"
        );
        Ok(grant)
    }
    fn prepare(&mut self, client: &AuthenticatedClient, target: &str) -> Result<Ticket> {
        client.authorize(self.user_root)?;
        oid_ok(target)?;
        ensure!(
            self.journal.current.as_ref().is_none_or(|op| matches!(
                op.phase,
                Phase::Finalized | Phase::RolledBack | Phase::Cancelled
            )),
            "unfinished operation"
        );
        let previous = self.provider.active_oid()?;
        let mut random = [0; 32];
        OsRng.fill_bytes(&mut random);
        let id = blake3::hash(&random).to_hex().to_string();
        random.zeroize();
        let scope = Scope {
            id: id.clone(),
            domain: self.provider.domain().into(),
            from: target.into(),
            to: previous.clone(),
        };
        let grant = StoredGrant {
            token: self.issuer.issue(&scope)?.to_vec(),
            scope,
        };
        self.provider.check_rollback(&grant.scope, &grant.token)?;
        self.store_grant(&grant)?;
        let confirmation = ticket(client, &id);
        let op = Operation {
            id,
            caller: client.caller.clone(),
            target: target.into(),
            previous,
            confirmation_hash: ticket_hash(&confirmation),
            prepared_until: now()?
                .checked_add(120)
                .ok_or_else(|| anyhow!("clock overflow"))?,
            keep_until: 0,
            phase: Phase::Prepared,
        };
        self.journal.append(op)?; // Intent and grant are durable BEFORE provider staging.
        self.deadline = Some(Instant::now() + TIMER);
        if let Err(error) = self.provider.stage(&grant.scope) {
            self.recover()?;
            return Err(error);
        }
        Ok(confirmation)
    }
    fn check_ticket(
        &self,
        client: &AuthenticatedClient,
        ticket: &Ticket,
        phase: Phase,
    ) -> Result<Operation> {
        client.authorize(self.user_root)?;
        let op = self
            .journal
            .current
            .clone()
            .ok_or_else(|| anyhow!("no pending operation"))?;
        ensure!(
            op.phase == phase
                && op.caller == client.caller
                && op.id == ticket.operation
                && ticket.session == client.session
                && op.confirmation_hash == ticket_hash(ticket),
            "ticket/session/phase mismatch"
        );
        ensure!(
            self.deadline
                .is_some_and(|deadline| Instant::now() < deadline)
                && now()?
                    < if phase == Phase::Prepared {
                        op.prepared_until
                    } else {
                        op.keep_until
                    },
            "timer expired"
        );
        Ok(op)
    }
    fn confirm(&mut self, client: &AuthenticatedClient, confirmation: Ticket) -> Result<Ticket> {
        let mut op = self.check_ticket(client, &confirmation, Phase::Prepared)?;
        let grant = self.grant(&op)?;
        let keep = ticket(client, &op.id);
        op.phase = Phase::Applying;
        op.confirmation_hash = ticket_hash(&keep);
        op.keep_until = now()?
            .checked_add(120)
            .ok_or_else(|| anyhow!("clock overflow"))?;
        self.journal.append(op.clone())?; // Durable, single-use CONFIRM before effect.
        self.deadline = Some(Instant::now() + TIMER);
        if let Err(error) = self.provider.activate(&grant.scope) {
            self.rollback()?;
            return Err(error);
        }
        op.phase = Phase::AwaitingKeep;
        self.journal.append(op)?;
        Ok(keep)
    }
    fn keep(&mut self, client: &AuthenticatedClient, keep: Ticket) -> Result<()> {
        let mut op = self.check_ticket(client, &keep, Phase::AwaitingKeep)?;
        let grant = self.grant(&op)?;
        ensure!(
            self.provider.active_oid()? == op.target,
            "effect changed before KEEP"
        );
        op.phase = Phase::Finalized;
        self.journal.append(op)?; // Durable KEEP decision before grant retirement.
        self.provider.keep(&grant.scope)?;
        self.deadline = None;
        Ok(())
    }
    fn rollback(&mut self) -> Result<()> {
        let mut op = self
            .journal
            .current
            .clone()
            .ok_or_else(|| anyhow!("no pending operation"))?;
        let grant = self.grant(&op)?;
        let prepared = op.phase == Phase::Prepared;
        op.phase = Phase::RollingBack;
        self.journal.append(op.clone())?;
        let result = if self.provider.unstaged(&grant.scope)? {
            Ok(())
        } else {
            self.provider.revert(&grant.scope, &grant.token)
        };
        if let Err(error) = result {
            op.phase = Phase::RecoveryRequired;
            self.journal.append(op)?;
            return Err(error);
        }
        op.phase = if prepared {
            Phase::Cancelled
        } else {
            Phase::RolledBack
        };
        self.journal.append(op)?;
        self.deadline = None;
        Ok(())
    }
    fn recover(&mut self) -> Result<()> {
        match self.journal.current.as_ref().map(|op| op.phase) {
            Some(
                Phase::Prepared
                | Phase::Applying
                | Phase::AwaitingKeep
                | Phase::RollingBack
                | Phase::RecoveryRequired,
            ) => self.rollback(),
            Some(Phase::Finalized) => {
                let op = self.journal.current.as_ref().unwrap();
                // A subsequent preparation may replace the single grant slot
                // before its own journal append. A durable KEEP is authenticated
                // by the signed journal and needs no old rollback capability.
                let scope = Scope {
                    id: op.id.clone(),
                    domain: self.provider.domain().into(),
                    from: op.target.clone(),
                    to: op.previous.clone(),
                };
                self.provider.keep(&scope)
            }
            _ => Ok(()),
        }
    }
    fn tick(&mut self) -> Result<()> {
        if let Some(op) = &self.journal.current {
            let limit = if op.phase == Phase::Prepared {
                op.prepared_until
            } else {
                op.keep_until
            };
            if matches!(
                op.phase,
                Phase::Prepared | Phase::Applying | Phase::AwaitingKeep
            ) && (self.deadline.is_none_or(|d| Instant::now() >= d) || now()? >= limit)
            {
                return self.rollback();
            }
            if op.phase == Phase::RecoveryRequired {
                return self.rollback();
            }
        }
        Ok(())
    }
}

/// Trusted Neural Bus UI dispatcher calls these typed handlers. No WASI or SLM API.
/// Owns the actual background timer, rather than requiring client polling.
pub struct Supervisor {
    core: Arc<Mutex<Core>>,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
    health: Arc<Mutex<Option<String>>>,
}
impl Supervisor {
    pub fn start(
        journal: RestoreCoordinator,
        provider: Arc<PersistentProvider>,
        issuer: RecoveryIssuer,
        user_root: PublicKey,
    ) -> Result<Self> {
        let mut core = Core {
            journal,
            provider,
            issuer,
            user_root,
            deadline: None,
        };
        core.recover()?; // Never extend a timer across restart.
        let core = Arc::new(Mutex::new(core));
        let stop = Arc::new(AtomicBool::new(false));
        let health = Arc::new(Mutex::new(None));
        let (weak, flag, status) = (Arc::downgrade(&core), stop.clone(), health.clone());
        let worker = thread::Builder::new()
            .name("vela-rollback".into())
            .stack_size(2 * 1024 * 1024)
            .spawn(move || {
                while !flag.load(Ordering::Acquire) {
                    let Some(core) = weak.upgrade() else { break };
                    let outcome = core
                        .lock()
                        .map_err(|_| anyhow!("supervisor poisoned"))
                        .and_then(|mut c| c.tick());
                    if let Ok(mut health) = status.lock() {
                        *health = outcome
                            .err()
                            .map(|e| e.to_string().chars().take(256).collect());
                    }
                    thread::sleep(Duration::from_millis(250));
                }
            })?;
        Ok(Self {
            core,
            stop,
            worker: Some(worker),
            health,
        })
    }
    pub fn prepare(&self, client: &AuthenticatedClient, oid: &str) -> Result<Ticket> {
        self.core
            .lock()
            .map_err(|_| anyhow!("supervisor poisoned"))?
            .prepare(client, oid)
    }
    pub fn confirm(&self, client: &AuthenticatedClient, ticket: Ticket) -> Result<Ticket> {
        self.core
            .lock()
            .map_err(|_| anyhow!("supervisor poisoned"))?
            .confirm(client, ticket)
    }
    pub fn keep(&self, client: &AuthenticatedClient, ticket: Ticket) -> Result<()> {
        self.core
            .lock()
            .map_err(|_| anyhow!("supervisor poisoned"))?
            .keep(client, ticket)
    }
    pub fn health(&self) -> Result<Option<String>> {
        Ok(self
            .health
            .lock()
            .map_err(|_| anyhow!("supervisor poisoned"))?
            .clone())
    }
}
impl Drop for Supervisor {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        // Clean shutdown also attempts recovery; failure remains durable in the journal.
        if let Ok(mut core) = self.core.lock() {
            let _ = core.recover();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;
    use redb::Database;
    use vela_lfs_m1::Lfs;
    fn value(ssid: &str) -> serde_json::Value {
        serde_json::json!({"schema":"vela.net.config.v1","ssid":ssid,"dns":"9.9.9.9"})
    }
    #[test]
    fn restart_between_rollback_intent_and_provider_staging() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let root = KeyPair::new();
        let issuer = RecoveryIssuer::new(KeyPair::new());
        let provider = provider(dir.path(), &root, issuer.public_key())?;
        let previous = provider.active_oid()?;
        let target = provider.admit(&value("After"))?;
        let scope = Scope {
            id: "d".repeat(64),
            domain: provider.domain().into(),
            from: target.clone(),
            to: previous.clone(),
        };
        let signing = SigningKey::from_bytes(&[47; 32]);
        let journal =
            RestoreCoordinator::unlock(&dir.path().join("supervisor.redb"), signing.clone())?;
        let mut core = Core {
            journal,
            provider: provider.clone(),
            issuer,
            user_root: root.public(),
            deadline: None,
        };
        core.store_grant(&StoredGrant {
            token: core.issuer.issue(&scope)?.to_vec(),
            scope: scope.clone(),
        })?;
        core.journal.append(Operation {
            id: scope.id,
            caller: "alice".into(),
            target,
            previous: previous.clone(),
            confirmation_hash: "0".repeat(64),
            prepared_until: now()? + 120,
            keep_until: 0,
            phase: Phase::RollingBack,
        })?;
        drop(core);
        let journal = RestoreCoordinator::unlock(&dir.path().join("supervisor.redb"), signing)?;
        let supervisor = Supervisor::start(
            journal,
            provider.clone(),
            RecoveryIssuer::new(KeyPair::new()),
            root.public(),
        )?;
        assert_eq!(provider.active_oid()?, previous);
        assert_eq!(
            supervisor
                .core
                .lock()
                .unwrap()
                .journal
                .operation()
                .unwrap()
                .phase,
            Phase::RolledBack
        );
        Ok(())
    }
    fn user_token(root: &KeyPair, expired: bool) -> Result<Vec<u8>> {
        let code = if expired {
            "right(\"alice\",\"NetConfig\",\"read\"); right(\"alice\",\"NetConfig\",\"write\"); check if time($t), $t < 2000-01-01T00:00:00Z;"
        } else {
            "right(\"alice\",\"NetConfig\",\"read\"); right(\"alice\",\"NetConfig\",\"write\");"
        };
        Ok(BiscuitBuilder::new().code(code)?.build(root)?.to_vec()?)
    }
    fn provider(
        dir: &std::path::Path,
        root: &KeyPair,
        recovery: PublicKey,
    ) -> Result<Arc<PersistentProvider>> {
        let token = BiscuitBuilder::new().code("right(\"m4_provider\",\"NetConfig\",\"read\"); right(\"m4_provider\",\"NetConfig\",\"write\");")?.build(root)?.to_vec()?;
        Ok(Arc::new(PersistentProvider::new(
            Lfs::unlock(&dir.join("objects.redb"), [76; 32], root.public())?,
            Database::builder()
                .set_cache_size(1024 * 1024)
                .create(dir.join("provider.redb"))?,
            crate::provider::ProviderConfig {
                domain: "a".repeat(64),
                signing: SigningKey::from_bytes(&[24; 32]),
                lfs_root: root.public(),
                rollback_root: recovery,
                service_token: token,
            },
            &value("Before"),
        )?))
    }
    #[test]
    fn actual_persistence_scoped_recovery_and_user_expiration() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let root = KeyPair::new();
        let issuer = RecoveryIssuer::new(KeyPair::new());
        let pubkey = issuer.public_key();
        let provider = provider(dir.path(), &root, pubkey)?;
        let previous = provider.snapshot()?.0;
        let target = provider.admit(&value("After"))?;
        let scope = Scope {
            id: "b".repeat(64),
            domain: provider.domain().into(),
            from: target.clone(),
            to: previous.clone(),
        };
        let grant = issuer.issue(&scope)?;
        provider.stage(&scope)?;
        provider.activate(&scope)?;
        assert_eq!(provider.snapshot()?.1, value("After"));
        assert!(
            AuthenticatedClient::connect(root.public(), "alice".into(), user_token(&root, true)?)
                .is_err()
        );
        let mut forged = scope.clone();
        forged.to = target;
        assert!(provider.revert(&forged, &grant).is_err());
        provider.revert(&scope, &grant)?;
        assert_eq!(provider.snapshot()?.1, value("Before"));
        provider.revert(&scope, &grant)?; // Recovery receipt, no second side effect.
        drop(provider);
        let reopened = self::provider(dir.path(), &root, pubkey)?;
        assert_eq!(reopened.snapshot()?.0, previous);
        Ok(())
    }
    #[test]
    fn supervisor_confirm_timer_restart_and_keep() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let root = KeyPair::new();
        let issuer = RecoveryIssuer::new(KeyPair::new());
        let pubkey = issuer.public_key();
        let provider = provider(dir.path(), &root, pubkey)?;
        let target = provider.admit(&value("After"))?;
        let client =
            AuthenticatedClient::connect(root.public(), "alice".into(), user_token(&root, false)?)?;
        let wrong_session =
            AuthenticatedClient::connect(root.public(), "alice".into(), user_token(&root, false)?)?;
        let signing = SigningKey::from_bytes(&[47; 32]);
        let journal =
            RestoreCoordinator::unlock(&dir.path().join("supervisor.redb"), signing.clone())?;
        let mut core = Core {
            journal,
            provider: provider.clone(),
            issuer,
            user_root: root.public(),
            deadline: None,
        };
        let confirm = core.prepare(&client, &target)?;
        assert!(
            core.check_ticket(&wrong_session, &confirm, Phase::Prepared)
                .is_err()
        );
        let keep = core.confirm(&client, confirm)?;
        assert_eq!(provider.snapshot()?.1, value("After"));
        core.deadline = Some(Instant::now());
        core.tick()?;
        assert!(core.keep(&client, keep).is_err());
        assert_eq!(provider.snapshot()?.1, value("Before"));
        let confirm = core.prepare(&client, &target)?;
        let _keep = core.confirm(&client, confirm)?;
        drop(core);
        let journal = RestoreCoordinator::unlock(&dir.path().join("supervisor.redb"), signing)?;
        // Recovery does not use issuer private key or the original user token.
        let supervisor = Supervisor::start(
            journal,
            provider.clone(),
            RecoveryIssuer::new(KeyPair::new()),
            root.public(),
        )?;
        assert_eq!(provider.snapshot()?.1, value("Before"));
        assert!(supervisor.health()?.is_none());
        drop(supervisor);
        Ok(())
    }
    #[test]
    fn background_watchdog_and_keep_revokes_exact_grant() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let root = KeyPair::new();
        let issuer = RecoveryIssuer::new(KeyPair::new());
        let provider = provider(dir.path(), &root, issuer.public_key())?;
        let before = provider.snapshot()?.0;
        let after = provider.admit(&value("After"))?;
        let client =
            AuthenticatedClient::connect(root.public(), "alice".into(), user_token(&root, false)?)?;
        let journal = RestoreCoordinator::unlock(
            &dir.path().join("supervisor.redb"),
            SigningKey::from_bytes(&[47; 32]),
        )?;
        let supervisor = Supervisor::start(journal, provider.clone(), issuer, root.public())?;
        let confirm = supervisor.prepare(&client, &after)?;
        let keep = supervisor.confirm(&client, confirm)?;
        let grant = {
            let core = supervisor.core.lock().unwrap();
            core.grant(core.journal.operation().unwrap())?
        };
        supervisor.keep(&client, keep)?;
        assert!(provider.revert(&grant.scope, &grant.token).is_err());
        assert_eq!(provider.snapshot()?.0, after);
        let confirm = supervisor.prepare(&client, &before)?;
        let keep = supervisor.confirm(&client, confirm)?;
        let mut expired = client.clone();
        expired.token = Zeroizing::new(user_token(&root, true)?);
        assert!(supervisor.keep(&expired, keep).is_err());
        supervisor.core.lock().unwrap().deadline = Some(Instant::now());
        let deadline = Instant::now() + Duration::from_secs(5);
        while provider.snapshot()?.0 != after && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(20));
        }
        assert_eq!(provider.snapshot()?.0, after);
        assert!(supervisor.health()?.is_none());
        Ok(())
    }
    #[test]
    fn staged_partial_application_survives_reopen_without_publishing_target() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let root = KeyPair::new();
        let issuer = RecoveryIssuer::new(KeyPair::new());
        let pubkey = issuer.public_key();
        let current = provider(dir.path(), &root, pubkey)?;
        let before = current.active_oid()?;
        let target = current.admit(&value("After"))?;
        let scope = Scope {
            id: "c".repeat(64),
            domain: current.domain().into(),
            from: target,
            to: before.clone(),
        };
        let grant = issuer.issue(&scope)?;
        current.stage(&scope)?;
        assert!(current.revert(&scope, &user_token(&root, false)?).is_err());
        drop(current);
        let reopened = provider(dir.path(), &root, pubkey)?;
        assert_eq!(reopened.active_oid()?, before);
        reopened.revert(&scope, &grant)?;
        assert_eq!(reopened.active_oid()?, before);
        Ok(())
    }
}
