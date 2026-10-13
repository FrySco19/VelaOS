//! Legacy single-object synchronization compatibility.
//! Native Vela Soft uses `catalog` for immutable multi-document list/read.
//! This is not a multi-document index or a reconstruction of discarded versions.
use crate::{Error, MAX_NOTE, RECEIPT_SIZE, Receipt, digest, object_id, recover};
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use vela_storage_core::{BlockDevice, MAX_BYTES, ObjectStore};
pub const REQUEST_DOMAIN: &[u8] = b"VelaOS.ListRequest.v1\0";
const RESPONSE_DOMAIN: &[u8] = b"VelaOS.ListResponse.v1\0";
pub const REQUEST_SIZE: usize = 96;
pub const MAX_RESPONSE: usize = 32 + 2 + RECEIPT_SIZE + 2 + MAX_NOTE + 64;
fn signing(domain: &[u8], id: u64, body: &[u8], out: &mut [u8; 1400]) -> usize {
    let n = domain.len() + 8 + body.len();
    out[..domain.len()].copy_from_slice(domain);
    out[domain.len()..domain.len() + 8].copy_from_slice(&id.to_le_bytes());
    out[domain.len() + 8..n].copy_from_slice(body);
    n
}
pub fn request_signing(id: u64, nonce: &[u8; 32]) -> ([u8; 1400], usize) {
    let mut out = [0; 1400];
    let n = signing(REQUEST_DOMAIN, id, nonce, &mut out);
    (out, n)
}
pub fn verify_request(id: u64, payload: &[u8], root: &VerifyingKey) -> Result<[u8; 32], Error> {
    if id == 0 || payload.len() != REQUEST_SIZE || root.is_weak() {
        return Err(Error::Approval);
    }
    let nonce: [u8; 32] = payload[..32].try_into().unwrap();
    let (bytes, n) = request_signing(id, &nonce);
    root.verify_strict(
        &bytes[..n],
        &Signature::from_bytes(payload[32..].try_into().unwrap()),
    )
    .map_err(|_| Error::Approval)?;
    Ok(nonce)
}
pub fn response<D: BlockDevice>(
    store: &mut ObjectStore<D>,
    key: &SigningKey,
    id: u64,
    nonce: &[u8; 32],
    out: &mut [u8; MAX_RESPONSE],
) -> Result<usize, Error> {
    let mut note = [0; MAX_NOTE];
    let len = recover(store, &key.verifying_key(), &mut note)?;
    out[..32].copy_from_slice(nonce);
    let mut body = 34;
    if store.version().is_some() {
        let mut record = [0; MAX_BYTES];
        let n = store
            .read(object_id(), &mut record)
            .map_err(|_| Error::Storage)?;
        if n != 4 + RECEIPT_SIZE + len {
            return Err(Error::Storage);
        }
        out[32..34].copy_from_slice(&1u16.to_le_bytes());
        out[34..34 + RECEIPT_SIZE].copy_from_slice(&record[4..4 + RECEIPT_SIZE]);
        out[34 + RECEIPT_SIZE..36 + RECEIPT_SIZE].copy_from_slice(&(len as u16).to_le_bytes());
        out[36 + RECEIPT_SIZE..36 + RECEIPT_SIZE + len].copy_from_slice(&note[..len]);
        body = 36 + RECEIPT_SIZE + len;
    } else {
        out[32..34].copy_from_slice(&0u16.to_le_bytes());
    }
    let mut bytes = [0; 1400];
    let n = signing(RESPONSE_DOMAIN, id, &out[..body], &mut bytes);
    out[body..body + 64].copy_from_slice(&key.sign(&bytes[..n]).to_bytes());
    Ok(body + 64)
}
pub struct Entry<'a> {
    pub receipt: Receipt,
    pub note: &'a str,
}
pub fn verify_response<'a>(
    id: u64,
    nonce: &[u8; 32],
    payload: &'a [u8],
    root: &VerifyingKey,
) -> Result<Option<Entry<'a>>, Error> {
    if payload.len() < 98
        || payload.len() > MAX_RESPONSE
        || payload[..32] != *nonce
        || root.is_weak()
    {
        return Err(Error::Receipt);
    }
    let body = payload.len() - 64;
    let mut bytes = [0; 1400];
    let n = signing(RESPONSE_DOMAIN, id, &payload[..body], &mut bytes);
    root.verify_strict(
        &bytes[..n],
        &Signature::from_bytes(payload[body..].try_into().unwrap()),
    )
    .map_err(|_| Error::Receipt)?;
    match u16::from_le_bytes(payload[32..34].try_into().unwrap()) {
        0 if body == 34 => Ok(None),
        1 if body >= 36 + RECEIPT_SIZE => {
            let len = u16::from_le_bytes(
                payload[34 + RECEIPT_SIZE..36 + RECEIPT_SIZE]
                    .try_into()
                    .unwrap(),
            ) as usize;
            if len > MAX_NOTE || body != 36 + RECEIPT_SIZE + len {
                return Err(Error::Receipt);
            }
            let receipt = Receipt(payload[34..34 + RECEIPT_SIZE].try_into().unwrap());
            receipt.authenticate(root)?;
            let note = core::str::from_utf8(&payload[36 + RECEIPT_SIZE..body])
                .map_err(|_| Error::Receipt)?;
            if receipt.0[26..58] != digest(note.as_bytes())
                || receipt.0[90..106] != object_id()
                || u64::from_le_bytes(receipt.0[18..26].try_into().unwrap()) == 0
            {
                return Err(Error::Receipt);
            }
            Ok(Some(Entry { receipt, note }))
        }
        _ => Err(Error::Receipt),
    }
}
