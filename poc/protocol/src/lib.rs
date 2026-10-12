#![no_std]
#![forbid(unsafe_code)]

pub const MAGIC: [u8; 4] = *b"VELA";
pub const VERSION: u16 = 1;
pub const MAX_FRAME: usize = 2048;
pub const HEADER_SIZE: usize = 32;
pub const MAX_PAYLOAD: usize = MAX_FRAME - HEADER_SIZE;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u16)]
pub enum MessageType {
    Proposal = 1,
    Approval = 2,
    Receipt = 3,
    Ack = 4,
    Cancel = 5,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Magic,
    Version,
    Kind,
    Length,
    Reserved,
    Transaction,
    OutputTooSmall,
    Backpressure,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Header {
    pub magic: [u8; 4],
    pub version: u16,
    pub kind: MessageType,
    pub payload_len: u16,
    pub transaction: u64,
    pub sequence: u32,
}

impl Header {
    pub fn decode(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() < HEADER_SIZE {
            return Err(Error::Length);
        }
        if bytes[..4] != MAGIC {
            return Err(Error::Magic);
        }
        if bytes[4..6] != VERSION.to_le_bytes() {
            return Err(Error::Version);
        }
        let kind = match u16::from_le_bytes([bytes[6], bytes[7]]) {
            1 => MessageType::Proposal,
            2 => MessageType::Approval,
            3 => MessageType::Receipt,
            4 => MessageType::Ack,
            5 => MessageType::Cancel,
            _ => return Err(Error::Kind),
        };
        let payload_len = u16::from_le_bytes([bytes[8], bytes[9]]);
        if usize::from(payload_len) > MAX_PAYLOAD {
            return Err(Error::Length);
        }
        if bytes[10..12] != [0; 2] || bytes[24..32] != [0; 8] {
            return Err(Error::Reserved);
        }
        let transaction = u64::from_le_bytes(bytes[12..20].try_into().map_err(|_| Error::Length)?);
        if transaction == 0 {
            return Err(Error::Transaction);
        }
        let sequence = u32::from_le_bytes(bytes[20..24].try_into().map_err(|_| Error::Length)?);
        Ok(Self {
            magic: MAGIC,
            version: VERSION,
            kind,
            payload_len,
            transaction,
            sequence,
        })
    }
}

// No repr(C) wire casting: on-wire fields are explicitly little-endian.
pub struct Frame {
    pub header: Header,
    pub payload: [u8; MAX_PAYLOAD],
}

impl Frame {
    pub fn new(
        kind: MessageType,
        transaction: u64,
        sequence: u32,
        payload: &[u8],
    ) -> Result<Self, Error> {
        if payload.len() > MAX_PAYLOAD {
            return Err(Error::Length);
        }
        if transaction == 0 {
            return Err(Error::Transaction);
        }
        let mut frame = Self {
            header: Header {
                magic: MAGIC,
                version: VERSION,
                kind,
                payload_len: payload.len() as u16,
                transaction,
                sequence,
            },
            payload: [0; MAX_PAYLOAD],
        };
        frame.payload[..payload.len()].copy_from_slice(payload);
        Ok(frame)
    }
    pub fn encode<'a>(&self, out: &'a mut [u8]) -> Result<&'a [u8], Error> {
        if self.header.magic != MAGIC {
            return Err(Error::Magic);
        }
        if self.header.version != VERSION {
            return Err(Error::Version);
        }
        let n = usize::from(self.header.payload_len);
        if n > MAX_PAYLOAD {
            return Err(Error::Length);
        }
        if self.header.transaction == 0 {
            return Err(Error::Transaction);
        }
        let len = HEADER_SIZE + n;
        if out.len() < len {
            return Err(Error::OutputTooSmall);
        }
        out[..HEADER_SIZE].fill(0);
        out[..4].copy_from_slice(&MAGIC);
        out[4..6].copy_from_slice(&VERSION.to_le_bytes());
        out[6..8].copy_from_slice(&(self.header.kind as u16).to_le_bytes());
        out[8..10].copy_from_slice(&self.header.payload_len.to_le_bytes());
        out[12..20].copy_from_slice(&self.header.transaction.to_le_bytes());
        out[20..24].copy_from_slice(&self.header.sequence.to_le_bytes());
        out[HEADER_SIZE..len].copy_from_slice(&self.payload[..n]);
        Ok(&out[..len])
    }
}

#[derive(Debug)]
pub struct FrameRef<'a> {
    pub header: Header,
    pub payload: &'a [u8],
}
impl<'a> FrameRef<'a> {
    pub fn decode(bytes: &'a [u8]) -> Result<Self, Error> {
        let header = Header::decode(bytes)?;
        if bytes.len() != HEADER_SIZE + usize::from(header.payload_len) {
            return Err(Error::Length);
        }
        Ok(Self {
            header,
            payload: &bytes[HEADER_SIZE..],
        })
    }
}

// Terminal errors: caller closes/resets the transport session. No magic scanning.
// Ready bytes stay immutable until consume(); this preserves downstream credits.
pub struct Decoder {
    bytes: [u8; MAX_FRAME],
    used: usize,
    expected: usize,
    failed: bool,
}
impl Default for Decoder {
    fn default() -> Self {
        Self::new()
    }
}
impl Decoder {
    pub const fn new() -> Self {
        Self {
            bytes: [0; MAX_FRAME],
            used: 0,
            expected: HEADER_SIZE,
            failed: false,
        }
    }
    pub fn ready(&self) -> bool {
        !self.failed && self.used >= HEADER_SIZE && self.used == self.expected
    }
    pub fn push(&mut self, byte: u8) -> Result<(), Error> {
        if self.failed {
            return Err(Error::Length);
        }
        if self.ready() {
            return Err(Error::Backpressure);
        }
        if self.used >= MAX_FRAME {
            self.failed = true;
            return Err(Error::Length);
        }
        self.bytes[self.used] = byte;
        self.used += 1;
        if self.used == HEADER_SIZE {
            match Header::decode(&self.bytes[..HEADER_SIZE]) {
                Ok(h) => self.expected = HEADER_SIZE + usize::from(h.payload_len),
                Err(e) => {
                    self.failed = true;
                    return Err(e);
                }
            }
        }
        Ok(())
    }
    pub fn wire(&self) -> Option<&[u8]> {
        self.ready().then_some(&self.bytes[..self.used])
    }
    pub fn consume(&mut self) -> Result<(), Error> {
        if !self.ready() {
            return Err(Error::Backpressure);
        }
        self.bytes[..self.used].fill(0);
        self.used = 0;
        self.expected = HEADER_SIZE;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn maximum_wire_and_borrowed_payload() {
        let frame = Frame::new(MessageType::Proposal, 9, 3, &[7; MAX_PAYLOAD]).unwrap();
        let mut wire = [0; MAX_FRAME];
        let bytes = frame.encode(&mut wire).unwrap();
        assert_eq!(bytes.len(), MAX_FRAME);
        let parsed = FrameRef::decode(bytes).unwrap();
        assert_eq!(parsed.header.transaction, 9);
        assert_eq!(parsed.header.sequence, 3);
        assert_eq!(parsed.payload.as_ptr(), bytes[HEADER_SIZE..].as_ptr());
        assert_eq!(parsed.payload, &[7; MAX_PAYLOAD]);
    }
    #[test]
    fn fragmented_coalesced_and_backpressure() {
        let f = Frame::new(MessageType::Approval, 1, 0, b"abc").unwrap();
        let mut wire = [0; MAX_FRAME];
        let bytes = f.encode(&mut wire).unwrap();
        let mut decoder = Decoder::new();
        for _ in 0..2 {
            for (i, byte) in bytes.iter().enumerate() {
                decoder.push(*byte).unwrap();
                assert_eq!(decoder.ready(), i + 1 == bytes.len());
            }
            assert_eq!(decoder.push(0), Err(Error::Backpressure));
            assert_eq!(
                FrameRef::decode(decoder.wire().unwrap()).unwrap().payload,
                b"abc"
            );
            decoder.consume().unwrap();
        }
    }
    #[test]
    fn invalid_headers_and_trailing_bytes_fail_closed() {
        let f = Frame::new(MessageType::Receipt, 1, 0, &[]).unwrap();
        let mut wire = [0; MAX_FRAME];
        f.encode(&mut wire).unwrap();
        for (offset, expected) in [
            (0, Error::Magic),
            (4, Error::Version),
            (6, Error::Kind),
            (10, Error::Reserved),
            (24, Error::Reserved),
        ] {
            let mut bad = wire;
            bad[offset] = 0xff;
            assert_eq!(FrameRef::decode(&bad[..HEADER_SIZE]).unwrap_err(), expected);
        }
        assert_eq!(
            FrameRef::decode(&wire[..HEADER_SIZE + 1]).unwrap_err(),
            Error::Length
        );
        wire[8..10].copy_from_slice(&u16::MAX.to_le_bytes());
        assert_eq!(FrameRef::decode(&wire).unwrap_err(), Error::Length);
    }
}
