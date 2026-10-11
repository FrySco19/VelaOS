#![allow(dead_code)]

pub const PAGE_SIZE: usize = 4096;
pub const HEADER_SIZE: usize = 20;
pub const MAGIC: &[u8; 4] = b"VBRG";
pub const VERSION: u16 = 1;
pub const MOCK_TOKEN: &[u8] = b"mock-biscuit-token";
pub const MOCK_PAYLOAD: &[u8] = b"phase-one-payload";

const MAGIC_OFFSET: usize = 0;
const VERSION_OFFSET: usize = 4;
const HEADER_LEN_OFFSET: usize = 6;
const REQUEST_ID_OFFSET: usize = 8;
const TOKEN_OFFSET_OFFSET: usize = 12;
const TOKEN_LEN_OFFSET: usize = 14;
const PAYLOAD_OFFSET_OFFSET: usize = 16;
const PAYLOAD_LEN_OFFSET: usize = 18;

#[derive(Clone, Copy)]
pub struct Descriptor {
    pub request_id: u32,
    pub token_len: usize,
    pub payload_len: usize,
}

#[derive(Clone, Copy)]
pub enum ParseError {
    BadMagic,
    UnsupportedVersion,
    InvalidHeader,
    EmptyField,
    OutOfBounds,
    OverlappingFields,
}

pub unsafe fn write_mock_request(page: *mut u8) {
    unsafe {
        write_bytes(page, MAGIC_OFFSET, MAGIC);
        write_u16(page, VERSION_OFFSET, VERSION);
        write_u16(page, HEADER_LEN_OFFSET, HEADER_SIZE as u16);
        write_u32(page, REQUEST_ID_OFFSET, 1);
    }

    let token_offset = HEADER_SIZE;
    let payload_offset = token_offset + MOCK_TOKEN.len();
    unsafe {
        write_u16(page, TOKEN_OFFSET_OFFSET, token_offset as u16);
        write_u16(page, TOKEN_LEN_OFFSET, MOCK_TOKEN.len() as u16);
        write_u16(page, PAYLOAD_OFFSET_OFFSET, payload_offset as u16);
        write_u16(page, PAYLOAD_LEN_OFFSET, MOCK_PAYLOAD.len() as u16);

        write_bytes(page, token_offset, MOCK_TOKEN);
        write_bytes(page, payload_offset, MOCK_PAYLOAD);
    }
}

pub unsafe fn parse_descriptor(page: *const u8) -> Result<Descriptor, ParseError> {
    let mut magic = [0; 4];
    for (index, byte) in magic.iter_mut().enumerate() {
        *byte = unsafe { page.add(MAGIC_OFFSET + index).read_volatile() };
    }
    if &magic != MAGIC {
        return Err(ParseError::BadMagic);
    }

    let version = unsafe { read_u16(page, VERSION_OFFSET) };
    if version != VERSION {
        return Err(ParseError::UnsupportedVersion);
    }

    let header_len = unsafe { read_u16(page, HEADER_LEN_OFFSET) } as usize;
    if header_len != HEADER_SIZE {
        return Err(ParseError::InvalidHeader);
    }

    let request_id = unsafe { read_u32(page, REQUEST_ID_OFFSET) };
    let token_offset = unsafe { read_u16(page, TOKEN_OFFSET_OFFSET) } as usize;
    let token_len = unsafe { read_u16(page, TOKEN_LEN_OFFSET) } as usize;
    let payload_offset = unsafe { read_u16(page, PAYLOAD_OFFSET_OFFSET) } as usize;
    let payload_len = unsafe { read_u16(page, PAYLOAD_LEN_OFFSET) } as usize;

    if token_len == 0 || payload_len == 0 {
        return Err(ParseError::EmptyField);
    }

    let token_end = token_offset
        .checked_add(token_len)
        .filter(|end| token_offset >= header_len && *end <= PAGE_SIZE)
        .ok_or(ParseError::OutOfBounds)?;
    let payload_end = payload_offset
        .checked_add(payload_len)
        .filter(|end| payload_offset >= header_len && *end <= PAGE_SIZE)
        .ok_or(ParseError::OutOfBounds)?;

    if token_offset < payload_end && payload_offset < token_end {
        return Err(ParseError::OverlappingFields);
    }

    Ok(Descriptor {
        request_id,
        token_len,
        payload_len,
    })
}

unsafe fn write_bytes(page: *mut u8, offset: usize, bytes: &[u8]) {
    for (index, byte) in bytes.iter().enumerate() {
        unsafe {
            page.add(offset + index).write_volatile(*byte);
        }
    }
}

unsafe fn write_u16(page: *mut u8, offset: usize, value: u16) {
    unsafe { write_bytes(page, offset, &value.to_le_bytes()) }
}

unsafe fn write_u32(page: *mut u8, offset: usize, value: u32) {
    unsafe { write_bytes(page, offset, &value.to_le_bytes()) }
}

unsafe fn read_u16(page: *const u8, offset: usize) -> u16 {
    let low = unsafe { page.add(offset).read_volatile() };
    let high = unsafe { page.add(offset + 1).read_volatile() };
    u16::from_le_bytes([low, high])
}

unsafe fn read_u32(page: *const u8, offset: usize) -> u32 {
    let mut bytes = [0; 4];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = unsafe { page.add(offset + index).read_volatile() };
    }
    u32::from_le_bytes(bytes)
}
