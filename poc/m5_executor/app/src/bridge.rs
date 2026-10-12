#![allow(dead_code)]

use ed25519_compact::{PublicKey, SecretKey, Signature};
#[path = "trusted_key.rs"]
pub mod trusted_key;

pub const PAGE_SIZE: usize = 4096;
pub const HEADER_SIZE: usize = 20;
pub const CLAIMS_SIZE: usize = 32;
pub const TOKEN_SIZE: usize = CLAIMS_SIZE + Signature::BYTES;
pub const TOKEN_OFFSET: usize = HEADER_SIZE;
pub const PAYLOAD_OFFSET: usize = HEADER_SIZE + TOKEN_SIZE;
pub const MAX_FUEL: u64 = 10_000_000;
pub const MAX_MEMORY: usize = 64 * 1024;
pub const EXECUTE_LABEL: sel4_label::Label = 0x701;
pub const REPLY_LABEL: sel4_label::Label = 0x702;
// Keep the wire protocol usable in host tests without linking the Microkit ABI.
mod sel4_label {
    pub type Label = u64;
}
const DOMAIN: &[u8] = b"VelaOS.Executor.SignedFrame.v2\0";

pub const RETURN_7_MODULE: &[u8] = &[
    0, 97, 115, 109, 1, 0, 0, 0, 1, 5, 1, 96, 0, 1, 127, 3, 2, 1, 0, 7, 7, 1, 3, b'r', b'u', b'n',
    0, 0, 10, 6, 1, 4, 0, 65, 7, 11,
];
pub const LOOP_MODULE: &[u8] = &[
    0, 97, 115, 109, 1, 0, 0, 0, 1, 5, 1, 96, 0, 1, 127, 3, 2, 1, 0, 7, 7, 1, 3, b'r', b'u', b'n',
    0, 0, 10, 10, 1, 8, 0, 3, 64, 12, 0, 11, 0, 11,
];
pub const WASI_IMPORT_MODULE: &[u8] = &[
    0, 97, 115, 109, 1, 0, 0, 0, 1, 4, 1, 96, 0, 0, 2, 36, 1, 22, b'w', b'a', b's', b'i', b'_',
    b's', b'n', b'a', b'p', b's', b'h', b'o', b't', b'_', b'p', b'r', b'e', b'v', b'i', b'e', b'w',
    b'1', 9, b'p', b'r', b'o', b'c', b'_', b'e', b'x', b'i', b't', 0, 0,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u64)]
pub enum Status {
    Success = 0,
    Malformed = 1,
    Authentication = 2,
    Scope = 3,
    Replay = 4,
    Wasm = 5,
    Fuel = 6,
}

#[derive(Clone, Copy, Debug)]
pub struct Descriptor {
    pub request_id: u32,
    pub token_offset: usize,
    pub payload_offset: usize,
    pub payload_len: usize,
}
#[derive(Clone, Copy, Debug)]
pub struct Verified {
    pub descriptor: Descriptor,
    pub fuel: u64,
    pub memory: usize,
}

fn u16_at(page: &[u8; PAGE_SIZE], offset: usize) -> u16 {
    u16::from_le_bytes(page[offset..offset + 2].try_into().unwrap())
}
fn u32_at(page: &[u8; PAGE_SIZE], offset: usize) -> u32 {
    u32::from_le_bytes(page[offset..offset + 4].try_into().unwrap())
}
pub fn parse_descriptor(page: &[u8; PAGE_SIZE]) -> Result<Descriptor, Status> {
    if &page[..4] != b"VBRG" || u16_at(page, 4) != 2 || u16_at(page, 6) as usize != HEADER_SIZE {
        return Err(Status::Malformed);
    }
    let token_offset = u16_at(page, 12) as usize;
    let token_len = u16_at(page, 14) as usize;
    let payload_offset = u16_at(page, 16) as usize;
    let payload_len = u16_at(page, 18) as usize;
    if token_len != TOKEN_SIZE || payload_len == 0 {
        return Err(Status::Malformed);
    }
    let token_end = token_offset
        .checked_add(token_len)
        .filter(|end| token_offset >= HEADER_SIZE && *end <= PAGE_SIZE)
        .ok_or(Status::Malformed)?;
    let payload_end = payload_offset
        .checked_add(payload_len)
        .filter(|end| payload_offset >= HEADER_SIZE && *end <= PAGE_SIZE)
        .ok_or(Status::Malformed)?;
    if token_offset < payload_end && payload_offset < token_end {
        return Err(Status::Malformed);
    }
    Ok(Descriptor {
        request_id: u32_at(page, 8),
        token_offset,
        payload_offset,
        payload_len,
    })
}

// Authenticate header/offsets, claims and exactly the bytes that will execute.
fn signed_message(
    page: &[u8; PAGE_SIZE],
    d: Descriptor,
    output: &mut [u8; PAGE_SIZE + DOMAIN.len()],
) -> usize {
    let mut cursor = 0;
    for part in [
        DOMAIN,
        &page[..HEADER_SIZE],
        &page[d.token_offset..d.token_offset + CLAIMS_SIZE],
        &page[d.payload_offset..d.payload_offset + d.payload_len],
    ] {
        output[cursor..cursor + part.len()].copy_from_slice(part);
        cursor += part.len();
    }
    cursor
}
pub fn verify_frame(page: &[u8; PAGE_SIZE], last_consumed: u32) -> Result<Verified, Status> {
    verify_with_key(
        page,
        last_consumed,
        PublicKey::new(trusted_key::TRUSTED_PUBLIC_KEY),
    )
}
pub fn verify_with_key(
    page: &[u8; PAGE_SIZE],
    last_consumed: u32,
    root: PublicKey,
) -> Result<Verified, Status> {
    let descriptor = parse_descriptor(page)?;
    let t = descriptor.token_offset;
    let signature = Signature::from_slice(&page[t + CLAIMS_SIZE..t + TOKEN_SIZE])
        .map_err(|_| Status::Authentication)?;
    let mut message = [0; PAGE_SIZE + DOMAIN.len()];
    let length = signed_message(page, descriptor, &mut message);
    root.verify(&message[..length], &signature)
        .map_err(|_| Status::Authentication)?;
    let fuel = u64::from_le_bytes(page[t + 16..t + 24].try_into().unwrap());
    let memory = u32_at(page, t + 24) as usize;
    if &page[t..t + 4] != b"VEX2"
        || u32_at(page, t + 4) != 2
        || u32_at(page, t + 8) != descriptor.request_id
        || u32_at(page, t + 12) != 1
        || u32_at(page, t + 28) as usize != descriptor.payload_len
        || fuel == 0
        || fuel > MAX_FUEL
        || memory == 0
        || memory > MAX_MEMORY
        || descriptor.request_id == 0
    {
        return Err(Status::Scope);
    }
    if descriptor.request_id <= last_consumed {
        return Err(Status::Replay);
    }
    Ok(Verified {
        descriptor,
        fuel,
        memory,
    })
}

pub fn signed_frame(
    secret: &SecretKey,
    request_id: u32,
    payload: &[u8],
    fuel: u64,
) -> [u8; PAGE_SIZE] {
    assert!(!payload.is_empty() && payload.len() <= PAGE_SIZE - PAYLOAD_OFFSET);
    let mut page = [0; PAGE_SIZE];
    page[..4].copy_from_slice(b"VBRG");
    page[4..6].copy_from_slice(&2u16.to_le_bytes());
    page[6..8].copy_from_slice(&(HEADER_SIZE as u16).to_le_bytes());
    page[8..12].copy_from_slice(&request_id.to_le_bytes());
    for (offset, value) in [
        (12, TOKEN_OFFSET),
        (14, TOKEN_SIZE),
        (16, PAYLOAD_OFFSET),
        (18, payload.len()),
    ] {
        page[offset..offset + 2].copy_from_slice(&(value as u16).to_le_bytes());
    }
    let t = TOKEN_OFFSET;
    page[t..t + 4].copy_from_slice(b"VEX2");
    page[t + 4..t + 8].copy_from_slice(&2u32.to_le_bytes()); // Executor audience
    page[t + 8..t + 12].copy_from_slice(&request_id.to_le_bytes());
    page[t + 12..t + 16].copy_from_slice(&1u32.to_le_bytes()); // execute only
    page[t + 16..t + 24].copy_from_slice(&fuel.to_le_bytes());
    page[t + 24..t + 28].copy_from_slice(&(MAX_MEMORY as u32).to_le_bytes());
    page[t + 28..t + 32].copy_from_slice(&(payload.len() as u32).to_le_bytes());
    page[PAYLOAD_OFFSET..PAYLOAD_OFFSET + payload.len()].copy_from_slice(payload);
    let descriptor = parse_descriptor(&page).unwrap();
    let mut message = [0; PAGE_SIZE + DOMAIN.len()];
    let length = signed_message(&page, descriptor, &mut message);
    let signature = secret.sign(&message[..length], None);
    page[t + CLAIMS_SIZE..t + TOKEN_SIZE].copy_from_slice(signature.as_ref());
    page
}

/// Safety: `page` is the mapped 4 KiB PCB-writable page, valid for this call.
pub unsafe fn publish(page: *mut u8, bytes: &[u8; PAGE_SIZE]) {
    for (i, byte) in bytes.iter().enumerate() {
        unsafe {
            page.add(i).write_volatile(*byte);
        }
    }
}
/// Safety: `page` is a mapped readable 4 KiB region. No borrowed shared slice escapes.
pub unsafe fn snapshot(page: *const u8) -> [u8; PAGE_SIZE] {
    let mut bytes = [0; PAGE_SIZE];
    for (i, byte) in bytes.iter_mut().enumerate() {
        *byte = unsafe { page.add(i).read_volatile() };
    }
    bytes
}
