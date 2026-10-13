//! Bounded signed catalog pages and exact immutable-version reads.
use crate::{Error, MAX_NOTE, RECEIPT_SIZE, Receipt, digest, document_record};
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use vela_storage_core::{ARCHIVE_SLOTS, BlockDevice, MAX_BYTES, ObjectStore};
pub const REQUEST_SIZE: usize = 112;
pub const RESPONSE_SIZE: usize = 32 + 8 + 1 + 32 + RECEIPT_SIZE + 2 + MAX_NOTE + 64;
fn signing(read: bool, response: bool, id: u64, body: &[u8], out: &mut [u8; 1500]) -> usize {
    let domain = match (read, response) {
        (false, false) => b"Vela.catalog.list.request.v2".as_slice(),
        (true, false) => b"Vela.catalog.read.request.v2",
        (false, true) => b"Vela.catalog.list.response.v2",
        (true, true) => b"Vela.catalog.read.response.v2",
    };
    let n = domain.len() + 8 + body.len();
    out[..domain.len()].copy_from_slice(domain);
    out[domain.len()..domain.len() + 8].copy_from_slice(&id.to_le_bytes());
    out[domain.len() + 8..n].copy_from_slice(body);
    n
}
pub fn request_bytes(
    read: bool,
    id: u64,
    nonce: &[u8; 32],
    target: &[u8; 16],
) -> ([u8; 1500], usize) {
    let mut body = [0; 48];
    body[..32].copy_from_slice(nonce);
    body[32..].copy_from_slice(target);
    let mut out = [0; 1500];
    let n = signing(read, false, id, &body, &mut out);
    (out, n)
}
pub fn verify_request(
    read: bool,
    id: u64,
    p: &[u8],
    key: &VerifyingKey,
) -> Result<([u8; 32], [u8; 16]), Error> {
    if id == 0 || p.len() != REQUEST_SIZE || key.is_weak() {
        return Err(Error::Approval);
    }
    let nonce = p[..32].try_into().unwrap();
    let target = p[32..48].try_into().unwrap();
    let (bytes, n) = request_bytes(read, id, &nonce, &target);
    key.verify_strict(
        &bytes[..n],
        &Signature::from_bytes(p[48..].try_into().unwrap()),
    )
    .map_err(|_| Error::Approval)?;
    Ok((nonce, target))
}
pub fn response<D: BlockDevice>(
    store: &mut ObjectStore<D>,
    key: &SigningKey,
    read: bool,
    id: u64,
    nonce: &[u8; 32],
    target: &[u8; 16],
    out: &mut [u8; RESPONSE_SIZE],
) -> Result<usize, Error> {
    out[..32].copy_from_slice(nonce);
    let mut record = [0; MAX_BYTES];
    let mut next = ARCHIVE_SLOTS;
    let mut len = 0;
    if read {
        len = store
            .read(*target, &mut record)
            .map_err(|_| Error::Storage)?;
    } else {
        if target[8..].iter().any(|b| *b != 0) {
            return Err(Error::Bounds);
        }
        let cursor = u64::from_le_bytes(target[..8].try_into().unwrap());
        if cursor > ARCHIVE_SLOTS {
            return Err(Error::Bounds);
        }
        for slot in cursor..ARCHIVE_SLOTS {
            if let Some(v) = store.entry(slot).map_err(|_| Error::Storage)? {
                len = store
                    .read(v.object, &mut record)
                    .map_err(|_| Error::Storage)?;
                next = slot + 1;
                break;
            }
        }
    }
    out[32..40].copy_from_slice(&next.to_le_bytes());
    out[40] = u8::from(len != 0);
    let mut body = 41;
    if len != 0 {
        let (root, parent, r, note) = document_record(&record[..len], &key.verifying_key())?;
        if read && r.0[90..106] != *target {
            return Err(Error::Receipt);
        }
        out[41..57].copy_from_slice(&root);
        out[57..73].copy_from_slice(&parent);
        out[73..73 + RECEIPT_SIZE].copy_from_slice(&r.0);
        out[73 + RECEIPT_SIZE..75 + RECEIPT_SIZE]
            .copy_from_slice(&(note.len() as u16).to_le_bytes());
        out[75 + RECEIPT_SIZE..75 + RECEIPT_SIZE + note.len()].copy_from_slice(note);
        body = 75 + RECEIPT_SIZE + note.len();
    }
    let mut signed = [0; 1500];
    let n = signing(read, true, id, &out[..body], &mut signed);
    out[body..body + 64].copy_from_slice(&key.sign(&signed[..n]).to_bytes());
    Ok(body + 64)
}
pub struct Entry<'a> {
    pub root: [u8; 16],
    pub parent: [u8; 16],
    pub receipt: Receipt,
    pub note: &'a str,
}
pub fn verify_response<'a>(
    read: bool,
    id: u64,
    nonce: &[u8; 32],
    p: &'a [u8],
    key: &VerifyingKey,
) -> Result<(u64, Option<Entry<'a>>), Error> {
    if p.len() < 105 || p.len() > RESPONSE_SIZE || p[..32] != *nonce || key.is_weak() {
        return Err(Error::Receipt);
    }
    let body = p.len() - 64;
    let mut signed = [0; 1500];
    let n = signing(read, true, id, &p[..body], &mut signed);
    key.verify_strict(
        &signed[..n],
        &Signature::from_bytes(p[body..].try_into().unwrap()),
    )
    .map_err(|_| Error::Receipt)?;
    let next = u64::from_le_bytes(p[32..40].try_into().unwrap());
    if next > ARCHIVE_SLOTS {
        return Err(Error::Receipt);
    }
    if p[40] == 0 && body == 41 {
        return Ok((next, None));
    }
    if p[40] != 1 || body < 75 + RECEIPT_SIZE {
        return Err(Error::Receipt);
    }
    let len =
        u16::from_le_bytes(p[73 + RECEIPT_SIZE..75 + RECEIPT_SIZE].try_into().unwrap()) as usize;
    if len > MAX_NOTE || body != 75 + RECEIPT_SIZE + len {
        return Err(Error::Receipt);
    }
    let root = p[41..57].try_into().unwrap();
    let parent = p[57..73].try_into().unwrap();
    let r = Receipt(p[73..73 + RECEIPT_SIZE].try_into().unwrap());
    r.authenticate(key)?;
    let note = core::str::from_utf8(&p[75 + RECEIPT_SIZE..body]).map_err(|_| Error::Receipt)?;
    let id_expected =
        if root == crate::object_id() && parent == [0; 16] && r.0[90..106] == crate::object_id() {
            crate::object_id()
        } else {
            crate::version_id(
                &root,
                &parent,
                u64::from_le_bytes(r.0[10..18].try_into().unwrap()),
                &digest(note.as_bytes()),
            )
        };
    if r.0[26..58] != digest(note.as_bytes())
        || r.0[90..106] != id_expected
        || u64::from_le_bytes(r.0[18..26].try_into().unwrap()) == 0
    {
        return Err(Error::Receipt);
    }
    Ok((
        next,
        Some(Entry {
            root,
            parent,
            receipt: r,
            note,
        }),
    ))
}
