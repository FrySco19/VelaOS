//! Bounded streaming primitives. Credentials stay in native trusted endpoints, never guests.
use crate::Endpoint;
use anyhow::{Result, ensure};
use iceoryx2::prelude::ZeroCopySend;
use std::collections::BTreeSet;
use subtle::ConstantTimeEq;

pub const BULK_BLOCK: usize = 256 * 1024;
pub const MAX_CREDITS: usize = 8;

#[repr(C)]
#[derive(Debug, Default, Clone, Copy, ZeroCopySend)]
pub struct ChunkHeader {
    pub sequence: u64,
    pub offset: u64,
    pub length: u32,
    pub reserved: u32,
    pub mac: [u8; 32],
}
#[repr(C)]
#[derive(Debug, Default, Clone, Copy, ZeroCopySend)]
pub struct CreditAck {
    pub sequence: u64,
    pub mac: [u8; 32],
}

impl Endpoint {
    fn chunk_mac(&self, sequence: u64, offset: u64, bytes: &[u8]) -> [u8; 32] {
        let mut h = blake3::Hasher::new_keyed(&self.key);
        h.update(b"vela.stream.chunk.v1");
        h.update(&self.route);
        h.update(&self.schema.tag().to_le_bytes());
        h.update(&sequence.to_le_bytes());
        h.update(&offset.to_le_bytes());
        h.update(&(bytes.len() as u64).to_le_bytes());
        h.update(bytes);
        *h.finalize().as_bytes()
    }
    pub fn seal_chunk(&self, sequence: u64, offset: u64, bytes: &[u8]) -> Result<ChunkHeader> {
        self.live()?;
        ensure!(
            !bytes.is_empty() && bytes.len() <= BULK_BLOCK,
            "invalid chunk length"
        );
        Ok(ChunkHeader {
            sequence,
            offset,
            length: bytes.len() as u32,
            reserved: 0,
            mac: self.chunk_mac(sequence, offset, bytes),
        })
    }
    pub fn verify_chunk(
        &self,
        h: &ChunkHeader,
        sequence: u64,
        offset: u64,
        bytes: &[u8],
    ) -> Result<()> {
        self.live()?;
        ensure!(
            h.sequence == sequence
                && h.offset == offset
                && h.reserved == 0
                && h.length as usize == bytes.len()
                && !bytes.is_empty()
                && bytes.len() <= BULK_BLOCK,
            "chunk binding mismatch"
        );
        ensure!(
            bool::from(h.mac.ct_eq(&self.chunk_mac(sequence, offset, bytes))),
            "chunk MAC mismatch"
        );
        Ok(())
    }
    pub fn credit_ack(&self, sequence: u64) -> Result<CreditAck> {
        self.live()?;
        let mut h = blake3::Hasher::new_keyed(&self.key);
        h.update(b"vela.stream.credit.v1");
        h.update(&self.route);
        h.update(&sequence.to_le_bytes());
        Ok(CreditAck {
            sequence,
            mac: *h.finalize().as_bytes(),
        })
    }
    pub fn verify_credit(&self, ack: &CreditAck) -> Result<()> {
        let expected = self.credit_ack(ack.sequence)?;
        ensure!(
            bool::from(ack.mac.ct_eq(&expected.mac)),
            "credit authentication failed"
        );
        Ok(())
    }
}

/// A credit is spent before loaning bytes, never refunded on failure. ACKs free capacity only.
pub struct CreditWindow {
    pending: BTreeSet<u64>,
    next: u64,
    charged: u64,
    quota: u64,
    cancelled: bool,
}
impl CreditWindow {
    pub fn new(quota: u64) -> Result<Self> {
        ensure!(quota > 0, "zero stream quota");
        Ok(Self {
            pending: BTreeSet::new(),
            next: 0,
            charged: 0,
            quota,
            cancelled: false,
        })
    }
    pub fn available(&self) -> bool {
        !self.cancelled && self.pending.len() < MAX_CREDITS
    }
    pub fn issue(&mut self, length: usize) -> Result<(u64, u64)> {
        ensure!(self.available(), "stream cancelled or window full");
        ensure!(length > 0 && length <= BULK_BLOCK, "invalid block size");
        let end = self
            .charged
            .checked_add(length as u64)
            .ok_or_else(|| anyhow::anyhow!("quota overflow"))?;
        ensure!(end <= self.quota, "stream quota exhausted");
        let seq = self.next;
        self.next = self
            .next
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("sequence overflow"))?;
        let offset = self.charged;
        self.charged = end;
        self.pending.insert(seq);
        Ok((seq, offset))
    }
    pub fn acknowledge(&mut self, sequence: u64) -> Result<()> {
        ensure!(
            !self.cancelled && self.pending.remove(&sequence),
            "unknown/replayed credit"
        );
        Ok(())
    }
    pub fn drained(&self) -> bool {
        self.pending.is_empty()
    }
    pub fn charged(&self) -> u64 {
        self.charged
    }
    pub fn cancel(&mut self) {
        self.cancelled = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn window_quota_replay_and_cancellation() -> Result<()> {
        let mut w = CreditWindow::new((MAX_CREDITS * BULK_BLOCK) as u64)?;
        for i in 0..MAX_CREDITS {
            assert_eq!(w.issue(BULK_BLOCK)?.0, i as u64);
        }
        assert!(!w.available());
        assert!(w.issue(1).is_err());
        assert!(w.acknowledge(99).is_err());
        w.acknowledge(0)?;
        assert!(w.acknowledge(0).is_err());
        assert!(w.issue(1).is_err());
        w.cancel();
        assert!(w.acknowledge(1).is_err());
        assert!(w.issue(1).is_err());
        Ok(())
    }
    #[test]
    fn authenticated_chunks_and_credits_are_not_interchangeable() -> Result<()> {
        use crate::{PolicyGate, Schema};
        use biscuit_auth::{KeyPair, builder::BiscuitBuilder};
        let root = KeyPair::new();
        let token = BiscuitBuilder::new()
            .code("right(\"alice\",\"BulkStream\",\"read\");")?
            .build(&root)?
            .to_vec()?;
        let gate = PolicyGate::new(root.public());
        let route = gate.issue(
            &token,
            "alice",
            Schema::BulkStreamV1,
            std::time::Duration::from_secs(120),
        )?;
        let e = gate.endpoint(&route, "alice", Schema::BulkStreamV1)?;
        let header = e.seal_chunk(3, 12, b"bytes")?;
        e.verify_chunk(&header, 3, 12, b"bytes")?;
        assert!(e.verify_chunk(&header, 3, 12, b"ByTes").is_err());
        assert!(e.verify_chunk(&header, 4, 12, b"bytes").is_err());
        let mut ack = e.credit_ack(3)?;
        e.verify_credit(&ack)?;
        ack.mac = header.mac;
        assert!(e.verify_credit(&ack).is_err());
        Ok(())
    }
}
