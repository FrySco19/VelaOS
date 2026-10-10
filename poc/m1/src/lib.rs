//! Internal M1 storage engine. No public bus or zero-copy transport is implemented here.
use anyhow::{Result, anyhow, ensure};
use biscuit_auth::{AuthorizerLimits, Biscuit, PublicKey, builder::AuthorizerBuilder};
use chacha20poly1305::{
    XChaCha20Poly1305, XNonce,
    aead::{Aead, KeyInit, Payload},
};
use ed25519_dalek::{Signature, Signer, SigningKey};
use rand::{RngCore, rngs::OsRng};
use redb::{
    Database, Durability, ReadableDatabase, ReadableTable, ReadableTableMetadata, TableDefinition,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    path::Path,
    time::{Instant, SystemTime, UNIX_EPOCH},
};
use subtle::ConstantTimeEq;
use zeroize::Zeroizing;

const BLOBS: TableDefinition<&str, &[u8]> = TableDefinition::new("encrypted_blobs");
const LOG: TableDefinition<u64, &[u8]> = TableDefinition::new("signed_commit_log");
const IDEM: TableDefinition<&str, &[u8]> = TableDefinition::new("idempotency");
const FORMAT: TableDefinition<&str, &[u8]> = TableDefinition::new("format");
const MAX_BYTES: usize = 8 * 1024 * 1024;
pub const EPOCH: &str = "m1-extractor-netconfig-v1:no-embedding";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Kind {
    NetConfig,
}
impl Kind {
    fn name(self) -> &'static str {
        "NetConfig"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Card {
    pub title: String,
    pub summary: String,
    pub model_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Committed {
    pub oid: String,
    pub commit_id: u64,
    pub title: String,
}
#[derive(Clone, Serialize, Deserialize)]
struct Binding {
    oid: String,
    kind: Kind,
    result: Committed,
}
#[derive(Clone, Serialize, Deserialize)]
struct Entry {
    seq: u64,
    oid: String,
    kind: Kind,
    caller: String,
    idem: String,
    time: u64,
    blob_digest: String,
    card: Card,
}
#[derive(Clone, Serialize, Deserialize)]
struct Signed {
    seq: u64,
    previous: String,
    sealed: Sealed,
    signature: Vec<u8>,
}
#[derive(Clone, Serialize, Deserialize)]
struct Sealed {
    nonce: [u8; 24],
    ciphertext: Vec<u8>,
}
#[derive(Clone, Serialize, Deserialize)]
struct Blob {
    wrapped_key: Sealed,
    order: Vec<String>,
    chunks: BTreeMap<String, Sealed>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Handle {
    oid: String,
    caller: String,
    kind: Kind,
    exp: u64,
    boot: [u8; 16],
    deadline_ms: u64,
    mac: [u8; 32],
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Listed {
    pub oid: String,
    pub commit_id: u64,
    pub time: u64,
    pub card: Card,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    BeforeBlob,
    BlobDurable,
    BeforeLogCommit,
    LogDurable,
    IndexApplied,
}

pub fn now() -> Result<u64> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs())
}
pub fn oid(value: &Value) -> Result<String> {
    Ok(blake3::hash(&serde_jcs::to_vec(value)?)
        .to_hex()
        .to_string())
}
pub fn derive_collection_key(password: &[u8], salt: &[u8; 16]) -> Result<Zeroizing<[u8; 32]>> {
    let mut key = Zeroizing::new([0u8; 32]);
    argon2::Argon2::default()
        .hash_password_into(password, salt, key.as_mut())
        .map_err(|e| anyhow!("key derivation failed: {e}"))?;
    Ok(key)
}
fn seal(key: &[u8; 32], bytes: &[u8], aad: &[u8]) -> Result<Sealed> {
    let mut nonce = [0u8; 24];
    OsRng.fill_bytes(&mut nonce);
    let ciphertext = XChaCha20Poly1305::new(key.into())
        .encrypt(XNonce::from_slice(&nonce), Payload { msg: bytes, aad })
        .map_err(|_| anyhow!("encryption failed"))?;
    Ok(Sealed { nonce, ciphertext })
}
fn decrypt(key: &[u8; 32], sealed: &Sealed, aad: &[u8]) -> Result<Zeroizing<Vec<u8>>> {
    Ok(Zeroizing::new(
        XChaCha20Poly1305::new(key.into())
            .decrypt(
                XNonce::from_slice(&sealed.nonce),
                Payload {
                    msg: &sealed.ciphertext,
                    aad,
                },
            )
            .map_err(|_| anyhow!("authentication failed"))?,
    ))
}
fn canonical<T: Serialize>(v: &T) -> Result<Vec<u8>> {
    Ok(serde_jcs::to_vec(v)?)
}
fn encode_blob(collection: &[u8; 32], object: &str, plain: &[u8]) -> Result<Vec<u8>> {
    let mut dek = Zeroizing::new([0u8; 32]);
    OsRng.fill_bytes(dek.as_mut());
    let mut blob = Blob {
        wrapped_key: seal(collection, &*dek, format!("wrap:{object}").as_bytes())?,
        order: vec![],
        chunks: BTreeMap::new(),
    };
    for c in fastcdc::v2020::FastCDC::new(plain, 2048, 8192, 32768) {
        let bytes = &plain[c.offset..c.offset + c.length];
        let hash = blake3::hash(bytes).to_hex().to_string();
        if !blob.chunks.contains_key(&hash) {
            blob.chunks.insert(
                hash.clone(),
                seal(&dek, bytes, format!("chunk:{object}:{hash}").as_bytes())?,
            );
        }
        blob.order.push(hash);
    }
    canonical(&blob)
}
fn signing_bytes(s: &Signed) -> Result<Vec<u8>> {
    canonical(&("vela.lfs.commit.v1", s.seq, &s.previous, &s.sealed))
}
fn safe_name(s: &str) -> Result<()> {
    ensure!(
        !s.is_empty()
            && s.len() <= 128
            && s.bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"-_.".contains(&c)),
        "invalid caller/idempotency identifier"
    );
    Ok(())
}
fn extract(value: &Value) -> Result<Card> {
    let map = value
        .as_object()
        .ok_or_else(|| anyhow!("typed object required"))?;
    ensure!(
        map.len() == 3
            && map.contains_key("schema")
            && map.contains_key("ssid")
            && map.contains_key("dns"),
        "schema fields mismatch"
    );
    ensure!(
        value["schema"] == "vela.net.config.v1",
        "unsupported schema"
    );
    let ssid = value["ssid"]
        .as_str()
        .ok_or_else(|| anyhow!("SSID must be text"))?;
    ensure!(
        !ssid.is_empty() && ssid.len() <= 32 && !ssid.chars().any(char::is_control),
        "invalid SSID"
    );
    let dns = value["dns"]
        .as_str()
        .ok_or_else(|| anyhow!("DNS must be text"))?;
    dns.parse::<std::net::IpAddr>()?;
    Ok(Card {
        title: format!("Network configuration — {ssid}"),
        summary: format!("SSID {ssid}; DNS {dns}"),
        model_id: EPOCH.into(),
    })
}

pub struct Lfs {
    db: Database,
    collection: Zeroizing<[u8; 32]>,
    log_key: Zeroizing<[u8; 32]>,
    handle_key: Zeroizing<[u8; 32]>,
    signing: SigningKey,
    root: PublicKey,
    index: BTreeMap<String, Entry>,
    head: u64,
    previous: String,
    boot: [u8; 16],
    started: Instant,
}
impl Lfs {
    /// Host backing is an adapter parameter, never a logical object address.
    pub fn unlock(backing: &Path, collection: [u8; 32], root: PublicKey) -> Result<Self> {
        let signing =
            SigningKey::from_bytes(&blake3::derive_key("vela.lfs.signing.v1", &collection));
        let mut boot = [0u8; 16];
        OsRng.fill_bytes(&mut boot);
        let mut lfs = Self {
            db: Database::builder()
                .set_cache_size(8 * 1024 * 1024)
                .create(backing)?,
            log_key: Zeroizing::new(blake3::derive_key(
                "vela.lfs.log-encryption.v1",
                &collection,
            )),
            handle_key: Zeroizing::new(blake3::derive_key("vela.lfs.handle-mac.v1", &collection)),
            collection: Zeroizing::new(collection),
            signing,
            root,
            index: BTreeMap::new(),
            head: 0,
            previous: String::new(),
            boot,
            started: Instant::now(),
        };
        let existing = {
            let tx = lfs.db.begin_read()?;
            match tx.open_table(FORMAT) {
                Ok(table) => Some(
                    table
                        .get("unlock-check")?
                        .ok_or_else(|| anyhow!("missing unlock verifier"))?
                        .value()
                        .to_vec(),
                ),
                Err(redb::TableError::TableDoesNotExist(_)) => {
                    ensure!(
                        tx.list_tables()?.next().is_none(),
                        "format missing from nonempty backing"
                    );
                    None
                }
                Err(e) => return Err(e.into()),
            }
        };
        if let Some(bytes) = existing {
            let sealed: Sealed = serde_json::from_slice(&bytes)?;
            ensure!(
                &*decrypt(&lfs.collection, &sealed, b"vela.lfs.format.v1")? == b"vela-lfs-m1-v1",
                "wrong format"
            );
        } else {
            let mut tx = lfs.db.begin_write()?;
            tx.set_durability(Durability::Immediate)?;
            {
                tx.open_table(BLOBS)?;
                tx.open_table(LOG)?;
                tx.open_table(IDEM)?;
                let bytes = canonical(&seal(
                    &lfs.collection,
                    b"vela-lfs-m1-v1",
                    b"vela.lfs.format.v1",
                )?)?;
                tx.open_table(FORMAT)?
                    .insert("unlock-check", bytes.as_slice())?;
            }
            tx.commit()?;
        }
        lfs.replay()?;
        Ok(lfs)
    }
    fn authorize(&self, token: &[u8], caller: &str, kind: Kind, op: &str) -> Result<()> {
        safe_name(caller)?;
        ensure!(token.len() <= 16 * 1024, "capability exceeds size limit");
        let token = Biscuit::from(token, self.root)?;
        let source = format!(
            "caller(\"{caller}\"); resource(\"{kind}\"); operation(\"{op}\"); now({time}); allow if right(\"{caller}\", \"{kind}\", \"{op}\");",
            kind = kind.name(),
            time = now()?
        );
        AuthorizerBuilder::new()
            .set_limits(AuthorizerLimits {
                max_facts: 256,
                max_iterations: 32,
                max_time: std::time::Duration::from_millis(250),
            })
            .time()
            .code(source)?
            .build(&token)?
            .authorize()?;
        Ok(())
    }
    fn blob_bytes(&self, object: &str) -> Result<Vec<u8>> {
        let tx = self.db.begin_read()?;
        let t = tx.open_table(BLOBS)?;
        Ok(t.get(object)?
            .ok_or_else(|| anyhow!("missing durable blob"))?
            .value()
            .to_vec())
    }
    fn plaintext(&self, e: &Entry) -> Result<Zeroizing<Vec<u8>>> {
        let bytes = self.blob_bytes(&e.oid)?;
        ensure!(
            blake3::hash(&bytes).to_hex().as_str() == e.blob_digest,
            "blob digest mismatch"
        );
        let blob: Blob = serde_json::from_slice(&bytes)?;
        let dek = decrypt(
            &self.collection,
            &blob.wrapped_key,
            format!("wrap:{}", e.oid).as_bytes(),
        )?;
        let key: Zeroizing<[u8; 32]> = Zeroizing::new(dek.as_slice().try_into()?);
        let mut plain = Zeroizing::new(Vec::new());
        for hash in blob.order {
            let chunk = blob
                .chunks
                .get(&hash)
                .ok_or_else(|| anyhow!("missing chunk"))?;
            let chunk = decrypt(&key, chunk, format!("chunk:{}:{hash}", e.oid).as_bytes())?;
            ensure!(
                blake3::hash(&chunk).to_hex().as_str() == hash,
                "chunk hash mismatch"
            );
            ensure!(plain.len() + chunk.len() <= MAX_BYTES, "oversized blob");
            plain.extend_from_slice(&chunk);
        }
        ensure!(
            blake3::hash(&plain).to_hex().as_str() == e.oid,
            "plaintext OID mismatch"
        );
        Ok(plain)
    }
    fn replay(&mut self) -> Result<()> {
        let tx = self.db.begin_read()?;
        let log = tx.open_table(LOG)?;
        let idem = tx.open_table(IDEM)?;
        let mut index = BTreeMap::new();
        let mut previous = String::new();
        let mut head = 0;
        let mut expected_idem = BTreeMap::new();
        for row in log.iter()? {
            let (seq, bytes) = row?;
            let s: Signed = serde_json::from_slice(bytes.value())?;
            ensure!(
                seq.value() == head + 1 && s.seq == seq.value() && s.previous == previous,
                "broken commit chain"
            );
            self.signing
                .verifying_key()
                .verify_strict(&signing_bytes(&s)?, &Signature::from_slice(&s.signature)?)?;
            let plain = decrypt(
                &self.log_key,
                &s.sealed,
                format!("entry:{}:{}", s.seq, s.previous).as_bytes(),
            )?;
            let e: Entry = serde_json::from_slice(&plain)?;
            ensure!(
                e.seq == s.seq && e.card.model_id == EPOCH,
                "invalid commit epoch or sequence"
            );
            let value: Value = serde_json::from_slice(&self.plaintext(&e)?)?;
            ensure!(
                extract(&value)? == e.card,
                "card differs from deterministic extractor"
            );
            let result = Committed {
                oid: e.oid.clone(),
                commit_id: e.seq,
                title: e.card.title.clone(),
            };
            expected_idem.insert(
                format!("{}:{}", e.caller, e.idem),
                Binding {
                    oid: e.oid.clone(),
                    kind: e.kind,
                    result,
                },
            );
            index.entry(e.oid.clone()).or_insert(e);
            previous = blake3::hash(bytes.value()).to_hex().to_string();
            head = s.seq;
        }
        ensure!(
            idem.len()? == expected_idem.len() as u64,
            "idempotency/log divergence"
        );
        for (key, expected) in expected_idem {
            let stored: Sealed = serde_json::from_slice(
                idem.get(key.as_str())?
                    .ok_or_else(|| anyhow!("missing idempotency binding"))?
                    .value(),
            )?;
            let actual: Binding =
                serde_json::from_slice(&decrypt(&self.log_key, &stored, key.as_bytes())?)?;
            ensure!(
                canonical(&actual)? == canonical(&expected)?,
                "idempotency binding mismatch"
            );
        }
        self.index = index;
        self.head = head;
        self.previous = previous;
        Ok(())
    }
    pub fn put(
        &mut self,
        token: &[u8],
        caller: &str,
        idem: &str,
        value: &Value,
    ) -> Result<Committed> {
        self.put_with_probe(token, caller, idem, value, |_| {})
    }
    /// Fault-observation hook for the process-kill harness. It grants no authority.
    pub fn put_with_probe(
        &mut self,
        token: &[u8],
        caller: &str,
        idem: &str,
        value: &Value,
        mut probe: impl FnMut(Phase),
    ) -> Result<Committed> {
        self.authorize(token, caller, Kind::NetConfig, "write")?;
        safe_name(idem)?;
        let card = extract(value)?;
        let plain = Zeroizing::new(canonical(value)?);
        ensure!(plain.len() <= MAX_BYTES, "object too large");
        let object = oid(value)?;
        let binding_key = format!("{caller}:{idem}");
        {
            let tx = self.db.begin_read()?;
            let table = tx.open_table(IDEM)?;
            if let Some(v) = table.get(binding_key.as_str())? {
                let sealed: Sealed = serde_json::from_slice(v.value())?;
                let bound: Binding = serde_json::from_slice(&decrypt(
                    &self.log_key,
                    &sealed,
                    binding_key.as_bytes(),
                )?)?;
                ensure!(
                    bound.oid == object && bound.kind == Kind::NetConfig,
                    "idempotency key reused with different request"
                );
                return Ok(bound.result);
            }
        }
        probe(Phase::BeforeBlob);
        let blob_bytes = {
            let tx = self.db.begin_read()?;
            let t = tx.open_table(BLOBS)?;
            t.get(object.as_str())?.map(|v| v.value().to_vec())
        };
        let blob_bytes = match blob_bytes {
            Some(bytes) => bytes,
            None => {
                let bytes = encode_blob(&self.collection, &object, &plain)?;
                let mut tx = self.db.begin_write()?;
                tx.set_durability(Durability::Immediate)?;
                {
                    tx.open_table(BLOBS)?
                        .insert(object.as_str(), bytes.as_slice())?;
                }
                tx.commit()?;
                bytes
            }
        };
        probe(Phase::BlobDurable);
        let e = Entry {
            seq: self.head + 1,
            oid: object.clone(),
            kind: Kind::NetConfig,
            caller: caller.into(),
            idem: idem.into(),
            time: now()?,
            blob_digest: blake3::hash(&blob_bytes).to_hex().to_string(),
            card,
        };
        // Reject corrupt orphan blobs before referencing them in a committed operation.
        ensure!(
            self.plaintext(&e)?.as_slice() == plain.as_slice(),
            "existing blob does not match put"
        );
        let mut signed = Signed {
            seq: e.seq,
            previous: self.previous.clone(),
            sealed: seal(
                &self.log_key,
                &canonical(&e)?,
                format!("entry:{}:{}", e.seq, self.previous).as_bytes(),
            )?,
            signature: vec![],
        };
        signed.signature = self
            .signing
            .sign(&signing_bytes(&signed)?)
            .to_bytes()
            .to_vec();
        let encoded = canonical(&signed)?;
        let result = Committed {
            oid: object.clone(),
            commit_id: e.seq,
            title: e.card.title.clone(),
        };
        let binding = Binding {
            oid: object.clone(),
            kind: e.kind,
            result: result.clone(),
        };
        let bound = canonical(&seal(
            &self.log_key,
            &canonical(&binding)?,
            binding_key.as_bytes(),
        )?)?;
        let mut tx = self.db.begin_write()?;
        tx.set_durability(Durability::Immediate)?;
        {
            tx.open_table(LOG)?.insert(e.seq, encoded.as_slice())?;
            tx.open_table(IDEM)?
                .insert(binding_key.as_str(), bound.as_slice())?;
        }
        probe(Phase::BeforeLogCommit);
        tx.commit()?;
        // Set authoritative head before the replayable index update.
        self.head = e.seq;
        self.previous = blake3::hash(&encoded).to_hex().to_string();
        probe(Phase::LogDurable);
        self.index.entry(object).or_insert(e);
        probe(Phase::IndexApplied);
        Ok(result)
    }
    pub fn list(
        &self,
        token: &[u8],
        caller: &str,
        after_commit: u64,
        limit: usize,
    ) -> Result<Vec<Listed>> {
        self.authorize(token, caller, Kind::NetConfig, "list")?;
        ensure!((1..=100).contains(&limit), "page limit out of range");
        let mut rows: Vec<_> = self
            .index
            .values()
            .filter(|e| e.seq > after_commit)
            .map(|e| Listed {
                oid: e.oid.clone(),
                commit_id: e.seq,
                time: e.time,
                card: e.card.clone(),
            })
            .collect();
        rows.sort_by_key(|e| e.commit_id);
        rows.truncate(limit);
        Ok(rows)
    }
    pub fn handle(
        &self,
        token: &[u8],
        caller: &str,
        object: &str,
        ttl_seconds: u64,
    ) -> Result<Handle> {
        self.authorize(token, caller, Kind::NetConfig, "read")?;
        ensure!((1..=120).contains(&ttl_seconds), "invalid handle lifetime");
        ensure!(self.index.contains_key(object), "object not committed");
        let mut h = Handle {
            oid: object.into(),
            caller: caller.into(),
            kind: Kind::NetConfig,
            exp: now()?
                .checked_add(ttl_seconds)
                .ok_or_else(|| anyhow!("clock overflow"))?,
            boot: self.boot,
            deadline_ms: u64::try_from(self.started.elapsed().as_millis())?
                .checked_add(ttl_seconds * 1000)
                .ok_or_else(|| anyhow!("clock overflow"))?,
            mac: [0; 32],
        };
        h.mac = self.handle_mac(&h)?;
        Ok(h)
    }
    fn handle_mac(&self, h: &Handle) -> Result<[u8; 32]> {
        Ok(*blake3::keyed_hash(
            &self.handle_key,
            &canonical(&(
                "vela.lfs.handle.read.v1",
                &h.oid,
                &h.caller,
                h.kind,
                h.exp,
                h.boot,
                h.deadline_ms,
            ))?,
        )
        .as_bytes())
    }
    /// Internal typed read; M2 must deliver this buffer through iceoryx2, never an LLM.
    pub fn open(&self, token: &[u8], caller: &str, handle: &Handle) -> Result<Zeroizing<Vec<u8>>> {
        self.authorize(token, caller, handle.kind, "read")?;
        ensure!(
            handle.caller == caller
                && handle.boot == self.boot
                && now()? < handle.exp
                && self.started.elapsed().as_millis() < u128::from(handle.deadline_ms),
            "handle caller/expiry/session mismatch"
        );
        ensure!(
            bool::from(handle.mac.ct_eq(&self.handle_mac(handle)?)),
            "invalid handle MAC"
        );
        self.plaintext(
            self.index
                .get(&handle.oid)
                .ok_or_else(|| anyhow!("object not committed"))?,
        )
    }
    pub fn commit_count(&self) -> u64 {
        self.head
    }
    pub fn object_count(&self) -> usize {
        self.index.len()
    }
}

/// Dedicated synthetic fixture credential. Never use in a deployed service.
#[cfg(any(test, feature = "fault-fixtures"))]
pub mod fixture {
    use super::*;
    use biscuit_auth::{
        KeyPair, PrivateKey,
        builder::{Algorithm, BiscuitBuilder},
    };
    pub const COLLECTION: [u8; 32] = [73; 32];
    pub fn root() -> KeyPair {
        KeyPair::from(&PrivateKey::from_bytes(&[91; 32], Algorithm::Ed25519).unwrap())
    }
    pub fn token(caller: &str, operations: &[&str]) -> Result<Vec<u8>> {
        safe_name(caller)?;
        let mut builder = BiscuitBuilder::new();
        for op in operations {
            ensure!(
                ["read", "write", "list"].contains(op),
                "invalid fixture right"
            );
            builder = builder.code(format!("right(\"{caller}\", \"NetConfig\", \"{op}\");"))?;
        }
        Ok(builder.build(&root())?.to_vec()?)
    }
    pub fn payload(i: u64) -> Value {
        serde_json::json!({"schema":"vela.net.config.v1","ssid":format!("M1-{i}"),"dns":"9.9.9.9"})
    }
}

#[cfg(test)]
mod tests;
