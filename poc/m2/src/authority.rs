use anyhow::{Result, anyhow, ensure};
use biscuit_auth::builder::BlockBuilder;
use biscuit_auth::{Biscuit, PublicKey, builder::AuthorizerBuilder};
use rand::{RngCore, rngs::OsRng};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeSet, HashMap},
    fmt,
    sync::{Arc, Mutex, RwLock},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use zeroize::Zeroizing;

const MAX_TOKEN_BYTES: usize = 16 * 1024;
pub const MAX_CAPABILITY_TTL: Duration = Duration::from_secs(120);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct OpaqueHandle([u8; 32]);

impl fmt::Debug for OpaqueHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("OpaqueHandle([REDACTED])")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CapabilityOperation {
    Read,
    Write,
}

impl CapabilityOperation {
    fn as_str(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
        }
    }
}

pub struct CapabilityRequest {
    pub caller: String,
    pub resource: String,
    pub operations: Vec<CapabilityOperation>,
    pub byte_quota: u64,
    pub ttl: Duration,
}

struct Grant {
    caller: String,
    resource: String,
    operations: BTreeSet<CapabilityOperation>,
    byte_quota: u64,
    bytes_used: u64,
    expires_at_unix: u64,
    deadline: Instant,
    token: Zeroizing<Vec<u8>>,
    parent_token: Zeroizing<Vec<u8>>,
    lfs_handle: Option<vela_lfs_m1::Handle>,
    revoked: bool,
}

pub struct TaskAuthority {
    root: PublicKey,
    max_byte_quota: u64,
    grants: RwLock<HashMap<OpaqueHandle, Arc<Mutex<Grant>>>>,
}

impl TaskAuthority {
    /// Construct this manager inside the trusted host service, never in a Wasm guest.
    pub fn new(root: PublicKey, max_byte_quota: u64) -> Result<Self> {
        ensure!(max_byte_quota > 0, "host byte quota must be nonzero");
        Ok(Self {
            root,
            max_byte_quota,
            grants: RwLock::new(HashMap::new()),
        })
    }

    /// Attenuate a host-issued Biscuit and bind its rights to one host-held LFS handle.
    /// The returned random identifier is the only value intended for a Wasm task.
    pub fn issue(
        &self,
        parent_token: &[u8],
        request: CapabilityRequest,
        lfs_handle: Option<vela_lfs_m1::Handle>,
    ) -> Result<OpaqueHandle> {
        validate_name(&request.caller, "caller")?;
        validate_name(&request.resource, "resource")?;
        ensure!(
            !parent_token.is_empty() && parent_token.len() <= MAX_TOKEN_BYTES,
            "capability token exceeds size limit"
        );
        ensure!(
            request.byte_quota > 0 && request.byte_quota <= self.max_byte_quota,
            "byte quota exceeds host policy"
        );
        ensure!(
            !request.ttl.is_zero()
                && request.ttl <= MAX_CAPABILITY_TTL
                && request.ttl.as_secs() > 0,
            "capability lifetime must be between 1 and 120 seconds"
        );
        let operations: BTreeSet<_> = request.operations.iter().copied().collect();
        ensure!(
            !operations.is_empty() && operations.len() == request.operations.len(),
            "capability operations must be nonempty and unique"
        );
        ensure!(
            !operations.contains(&CapabilityOperation::Read) || lfs_handle.is_some(),
            "read capability must bind an exact M1 handle"
        );

        let parent = Biscuit::from(parent_token, self.root)?;
        for operation in &operations {
            authorize_biscuit(
                &parent,
                &request.caller,
                &request.resource,
                *operation,
                None,
                None,
            )?;
        }

        let now = unix_now()?;
        let expires_at_unix = now
            .checked_add(request.ttl.as_secs())
            .ok_or_else(|| anyhow!("capability expiration overflow"))?;
        let mut attenuation = String::new();
        for operation in &operations {
            attenuation.push_str(&format!(
                " check if right(\"{}\",\"{}\",\"{}\");",
                request.caller,
                request.resource,
                operation.as_str()
            ));
        }
        attenuation.push_str(&format!(
            " check if byte_quota({}); check if capability_expires({});",
            request.byte_quota, expires_at_unix
        ));
        let attenuated = parent.append(BlockBuilder::new().code(&attenuation)?)?;
        let token = Zeroizing::new(attenuated.to_vec()?);
        let mut nonce = [0u8; 32];
        OsRng.fill_bytes(&mut nonce);
        let handle = OpaqueHandle(nonce);
        let grant = Grant {
            caller: request.caller,
            resource: request.resource,
            operations,
            byte_quota: request.byte_quota,
            bytes_used: 0,
            expires_at_unix,
            deadline: Instant::now() + request.ttl,
            token,
            parent_token: Zeroizing::new(parent_token.to_vec()),
            lfs_handle,
            revoked: false,
        };
        let mut grants = self
            .grants
            .write()
            .map_err(|_| anyhow!("capability registry poisoned"))?;
        ensure!(!grants.contains_key(&handle), "opaque handle collision");
        grants.insert(handle, Arc::new(Mutex::new(grant)));
        Ok(handle)
    }

    /// Verify rights and permanently charge the requested bytes before host I/O begins.
    /// Failed or cancelled I/O does not refund quota; this keeps concurrent use bounded.
    pub fn authorize_and_charge(
        &self,
        handle: &OpaqueHandle,
        caller: &str,
        operation: CapabilityOperation,
        bytes: u64,
    ) -> Result<()> {
        let grant = self.lookup(handle)?;
        let mut grant = grant
            .lock()
            .map_err(|_| anyhow!("capability grant poisoned"))?;
        self.check_and_charge(&mut grant, caller, operation, bytes)
    }

    /// Open the bound M1 object inside the host and charge its measured size before
    /// returning plaintext. This method is not exposed to Wasm.
    pub fn open_lfs(
        &self,
        lfs: &vela_lfs_m1::Lfs,
        handle: &OpaqueHandle,
        caller: &str,
    ) -> Result<Zeroizing<Vec<u8>>> {
        let grant = self.lookup(handle)?;
        let mut grant = grant
            .lock()
            .map_err(|_| anyhow!("capability grant poisoned"))?;
        self.check_and_charge(&mut grant, caller, CapabilityOperation::Read, 0)?;
        let lfs_handle = grant
            .lfs_handle
            .as_ref()
            .ok_or_else(|| anyhow!("opaque capability has no LFS handle"))?;
        let plaintext = lfs.open(&grant.parent_token, caller, lfs_handle)?;
        self.check_and_charge(
            &mut grant,
            caller,
            CapabilityOperation::Read,
            u64::try_from(plaintext.len())?,
        )?;
        Ok(plaintext)
    }

    pub fn revoke(&self, handle: &OpaqueHandle) -> Result<bool> {
        let grant = self
            .grants
            .write()
            .map_err(|_| anyhow!("capability registry poisoned"))?
            .remove(handle);
        if let Some(grant) = grant {
            grant
                .lock()
                .map_err(|_| anyhow!("capability grant poisoned"))?
                .revoked = true;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn lookup(&self, handle: &OpaqueHandle) -> Result<Arc<Mutex<Grant>>> {
        self.grants
            .read()
            .map_err(|_| anyhow!("capability registry poisoned"))?
            .get(handle)
            .cloned()
            .ok_or_else(|| anyhow!("unknown or revoked opaque handle"))
    }

    fn check_and_charge(
        &self,
        grant: &mut Grant,
        caller: &str,
        operation: CapabilityOperation,
        bytes: u64,
    ) -> Result<()> {
        ensure!(!grant.revoked, "opaque capability revoked");
        ensure!(caller == grant.caller, "opaque handle caller mismatch");
        ensure!(
            Instant::now() < grant.deadline && unix_now()? < grant.expires_at_unix,
            "capability expired"
        );
        ensure!(
            grant.operations.contains(&operation),
            "operation is outside attenuated capability"
        );
        let used = grant
            .bytes_used
            .checked_add(bytes)
            .ok_or_else(|| anyhow!("byte quota overflow"))?;
        ensure!(used <= grant.byte_quota, "capability byte quota exceeded");

        let token = Biscuit::from(&*grant.token, self.root)?;
        authorize_biscuit(
            &token,
            caller,
            &grant.resource,
            operation,
            Some(grant.byte_quota),
            Some(grant.expires_at_unix),
        )?;
        grant.bytes_used = used;
        Ok(())
    }
}

fn authorize_biscuit(
    token: &Biscuit,
    caller: &str,
    resource: &str,
    operation: CapabilityOperation,
    quota: Option<u64>,
    expires_at: Option<u64>,
) -> Result<()> {
    let mut source = format!(
        "caller(\"{caller}\"); resource(\"{resource}\"); operation(\"{}\"); now({}); allow if right(\"{caller}\",\"{resource}\",\"{}\");",
        operation.as_str(),
        unix_now()?,
        operation.as_str()
    );
    if let Some(quota) = quota {
        source.push_str(&format!(" byte_quota({quota});"));
    }
    if let Some(expires_at) = expires_at {
        source.push_str(&format!(" capability_expires({expires_at});"));
    }
    AuthorizerBuilder::new()
        .set_limits(biscuit_auth::AuthorizerLimits {
            max_facts: 256,
            max_iterations: 32,
            max_time: Duration::from_millis(250),
        })
        .time()
        .code(source)?
        .build(token)?
        .authorize()?;
    Ok(())
}

fn validate_name(value: &str, label: &str) -> Result<()> {
    ensure!(
        !value.is_empty()
            && value.len() <= 128
            && value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"-_.".contains(&byte)),
        "invalid {label} identifier"
    );
    Ok(())
}

fn unix_now() -> Result<u64> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;
    use biscuit_auth::{KeyPair, builder::BiscuitBuilder};
    use vela_lfs_m1::Lfs;

    fn parent_token(root: &KeyPair, rights: &[&str]) -> Result<Vec<u8>> {
        let facts = rights
            .iter()
            .map(|right| format!("right(\"alice\",\"NetConfig\",\"{right}\");"))
            .collect::<String>();
        Ok(BiscuitBuilder::new().code(&facts)?.build(root)?.to_vec()?)
    }

    fn request(operations: Vec<CapabilityOperation>, byte_quota: u64) -> CapabilityRequest {
        CapabilityRequest {
            caller: "alice".into(),
            resource: "NetConfig".into(),
            operations,
            byte_quota,
            ttl: Duration::from_secs(30),
        }
    }

    #[test]
    fn attenuates_rights_and_enforces_cumulative_quota() -> Result<()> {
        let root = KeyPair::new();
        let token = parent_token(&root, &["read", "write"])?;
        let storage_dir = tempfile::tempdir()?;
        let mut lfs = Lfs::unlock(
            &storage_dir.path().join("authority.redb"),
            [73; 32],
            root.public(),
        )?;
        let object = lfs.put(
            &token,
            "alice",
            "authority-fixture",
            &serde_json::json!({
                "schema": "vela.net.config.v1",
                "ssid": "AuthorityTest",
                "dns": "1.1.1.1"
            }),
        )?;
        let lfs_handle = lfs.handle(&token, "alice", &object.oid, 120)?;
        let authority = TaskAuthority::new(root.public(), 1024)?;
        let opaque = authority.issue(
            &token,
            request(vec![CapabilityOperation::Read], 5),
            Some(lfs_handle),
        )?;

        let serialized = serde_json::to_string(&opaque)?;
        assert!(!serialized.contains(&object.oid));
        assert!(authority.open_lfs(&lfs, &opaque, "alice").is_err());

        assert!(
            authority
                .authorize_and_charge(&opaque, "alice", CapabilityOperation::Read, 4)
                .is_ok()
        );
        assert!(
            authority
                .authorize_and_charge(&opaque, "alice", CapabilityOperation::Write, 0)
                .is_err()
        );
        assert!(
            authority
                .authorize_and_charge(&opaque, "bob", CapabilityOperation::Read, 0)
                .is_err()
        );
        assert!(
            authority
                .authorize_and_charge(&opaque, "alice", CapabilityOperation::Read, 2)
                .is_err()
        );
        assert!(
            authority
                .authorize_and_charge(&opaque, "alice", CapabilityOperation::Read, 1)
                .is_ok()
        );
        assert!(
            authority
                .authorize_and_charge(&opaque, "alice", CapabilityOperation::Read, 1)
                .is_err()
        );
        Ok(())
    }

    #[test]
    fn host_open_charges_measured_bytes_before_returning_plaintext() -> Result<()> {
        let root = KeyPair::new();
        let token = parent_token(&root, &["read"])?;
        let storage_dir = tempfile::tempdir()?;
        let mut lfs = Lfs::unlock(
            &storage_dir.path().join("authority-open.redb"),
            [73; 32],
            root.public(),
        )?;
        let object = lfs.put(
            &parent_token(&root, &["read", "write"])?,
            "alice",
            "authority-open",
            &serde_json::json!({
                "schema": "vela.net.config.v1",
                "ssid": "HostOpenTest",
                "dns": "9.9.9.9"
            }),
        )?;
        let write_token = parent_token(&root, &["read", "write"])?;
        let lfs_handle = lfs.handle(&write_token, "alice", &object.oid, 120)?;
        let authority = TaskAuthority::new(root.public(), 1024)?;
        let opaque = authority.issue(
            &token,
            request(vec![CapabilityOperation::Read], 1024),
            Some(lfs_handle),
        )?;

        let payload = authority.open_lfs(&lfs, &opaque, "alice")?;
        assert!(!payload.is_empty());
        let remaining = 1024 - u64::try_from(payload.len())?;
        authority.authorize_and_charge(&opaque, "alice", CapabilityOperation::Read, remaining)?;
        assert!(
            authority
                .authorize_and_charge(&opaque, "alice", CapabilityOperation::Read, 1)
                .is_err()
        );
        Ok(())
    }

    #[test]
    fn cannot_grant_right_missing_from_parent_biscuit() -> Result<()> {
        let root = KeyPair::new();
        let token = parent_token(&root, &["read"])?;
        let authority = TaskAuthority::new(root.public(), 1024)?;
        assert!(
            authority
                .issue(&token, request(vec![CapabilityOperation::Write], 5), None,)
                .is_err()
        );
        Ok(())
    }

    #[test]
    fn expires_and_revokes_opaque_handles() -> Result<()> {
        let root = KeyPair::new();
        let token = parent_token(&root, &["write"])?;
        let authority = TaskAuthority::new(root.public(), 1024)?;
        let opaque = authority.issue(&token, request(vec![CapabilityOperation::Write], 5), None)?;
        let grant = authority.lookup(&opaque)?;
        grant
            .lock()
            .map_err(|_| anyhow!("capability grant poisoned"))?
            .deadline = Instant::now();
        assert!(
            authority
                .authorize_and_charge(&opaque, "alice", CapabilityOperation::Write, 1)
                .is_err()
        );
        assert!(authority.revoke(&opaque)?);
        assert!(
            authority
                .authorize_and_charge(&opaque, "alice", CapabilityOperation::Write, 0)
                .is_err()
        );
        Ok(())
    }
}
