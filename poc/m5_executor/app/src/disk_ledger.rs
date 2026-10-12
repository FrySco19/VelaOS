//! Authenticated snapshot of bounded receipt history and signed admission note.
//! Whole snapshot is Ed25519 authenticated; storage CRC is not authentication.
use crate::storage_core::{MAX_BYTES, ObjectStore};
use crate::{bridge, ledger_wire as lw, virtio_block::VirtioBlock};
use ed25519_compact::{KeyPair, PublicKey, Signature};
use sel4_microkit::{debug_println, memory_region_symbol};
const SLOT: [u8; 16] = [0xa1; 16];
pub struct DiskLedger {
    ledger: lw::Ledger<8>,
    store: ObjectStore<VirtioBlock>,
    note: [u8; 128],
    note_len: usize,
    poisoned: bool,
}
#[unsafe(no_mangle)]
static mut virtio_dma_phys: u64 = 0;

impl DiskLedger {
    pub fn new(key: &KeyPair) -> Self {
        let driver = unsafe {
            VirtioBlock::new(
                memory_region_symbol!(virtio_mmio_addr: *mut u8).as_ptr(),
                memory_region_symbol!(virtio_dma_addr: *mut u8).as_ptr(),
                core::ptr::read_volatile(&raw const virtio_dma_phys),
            )
        }
        .expect("VirtIO initialization");
        debug_println!("VelaOS DISK: VirtIO modern queue ready; FLUSH negotiated");
        let mut store = ObjectStore::open(driver).expect("recover storage");
        let mut ledger = lw::Ledger::new();
        let mut bytes = [0; MAX_BYTES];
        let n = store.read(SLOT, &mut bytes).expect("read snapshot");
        let mut note = [0; 128];
        let mut note_len = 0;
        if n != 0 {
            assert!(n >= 202 && &bytes[..4] == b"VLD1");
            let count = u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize;
            note_len = u16::from_le_bytes(bytes[8..10].try_into().unwrap()) as usize;
            assert!(count <= 8 && note_len <= 128 && n == 138 + count * 128 + 64);
            key.pk
                .verify(
                    &bytes[..n - 64],
                    &Signature::new(bytes[n - 64..n].try_into().unwrap()),
                )
                .expect("snapshot signature");
            note.copy_from_slice(&bytes[10..138]);
            for i in 0..count {
                let p = &bytes[138 + i * 128..138 + (i + 1) * 128];
                let u64at = |i| u64::from_le_bytes(p[i..i + 8].try_into().unwrap());
                let body = lw::ReceiptBody {
                    epoch: u64at(0),
                    request_id: u64at(8),
                    status: u32::from_le_bytes(p[16..20].try_into().unwrap()),
                    value: u32::from_le_bytes(p[20..24].try_into().unwrap()),
                    authority_digest: p[24..56].try_into().unwrap(),
                    seq: u64at(56),
                };
                ledger
                    .recover(
                        lw::Receipt {
                            body,
                            signature: p[64..128].try_into().unwrap(),
                        },
                        &key.pk,
                    )
                    .expect("receipt history signature");
            }
            debug_println!(
                "VelaOS DISK: cold boot recovered signed note bytes={} receipts={} generation={}",
                note_len,
                ledger.len(),
                store.version().unwrap().generation
            );
            debug_println!(
                "VelaOS DISK: recovered note verified: {}",
                core::str::from_utf8(&note[..note_len]).expect("note UTF-8")
            );
        } else {
            debug_println!("VelaOS DISK: blank dedicated disk; no prior commit");
        }
        Self {
            ledger,
            store,
            note,
            note_len,
            poisoned: false,
        }
    }
    pub fn len(&self) -> usize {
        self.ledger.len()
    }
    pub fn commit(
        &mut self,
        key: &KeyPair,
        pcb: PublicKey,
        frame: &[u8; bridge::PAGE_SIZE],
        c: &lw::Commit,
    ) -> Result<lw::CommitOutcome, lw::LfsStatus> {
        if self.poisoned {
            return Err(lw::LfsStatus::QuotaExceeded);
        }
        let outcome = self.ledger.commit(&key.sk, pcb, frame, c)?;
        if matches!(outcome, lw::CommitOutcome::Replayed(_)) {
            return Ok(outcome);
        }
        let d = bridge::parse_descriptor(frame).map_err(|_| lw::LfsStatus::Malformed)?;
        let payload = &frame[d.payload_offset..d.payload_offset + d.payload_len];
        let Some(note) = note_section(payload) else {
            self.poisoned = true;
            return Err(lw::LfsStatus::Malformed);
        };
        if note.len() > 128 {
            self.poisoned = true;
            return Err(lw::LfsStatus::Malformed);
        }
        self.note.fill(0);
        self.note[..note.len()].copy_from_slice(note);
        self.note_len = note.len();
        let mut bytes = [0; MAX_BYTES];
        bytes[..4].copy_from_slice(b"VLD1");
        bytes[4..8].copy_from_slice(&(self.ledger.len() as u32).to_le_bytes());
        bytes[8..10].copy_from_slice(&(self.note_len as u16).to_le_bytes());
        bytes[10..138].copy_from_slice(&self.note);
        for (i, r) in self.ledger.receipts().enumerate() {
            let message = r.body.message();
            let pos = 138 + i * 128;
            bytes[pos..pos + 64].copy_from_slice(&message[message.len() - 64..]);
            bytes[pos + 64..pos + 128].copy_from_slice(&r.signature);
        }
        let n = 138 + self.ledger.len() * 128;
        let sig = key.sk.sign(&bytes[..n], None);
        bytes[n..n + 64].copy_from_slice(sig.as_ref());
        let generation = self.store.version().map_or(0, |v| v.generation);
        if self.store.save(SLOT, generation, &bytes[..n + 64]).is_err() {
            self.poisoned = true;
            return Err(lw::LfsStatus::QuotaExceeded);
        }
        debug_println!(
            "VelaOS DISK: data+signed history flushed generation={}; receipt release allowed",
            generation + 1
        );
        debug_println!(
            "VelaOS DISK: note bytes={} stored on block backend",
            self.note_len
        );
        Ok(outcome)
    }
}
fn leb(b: &[u8]) -> Option<(usize, usize)> {
    let mut r = 0usize;
    for (i, &v) in b.iter().take(5).enumerate() {
        if i == 4 && v & 0xf0 != 0 {
            return None;
        }
        r |= ((v & 127) as usize) << (7 * i);
        if v & 128 == 0 {
            return Some((r, i + 1));
        }
    }
    None
}
fn note_section(m: &[u8]) -> Option<&[u8]> {
    if m.get(..8)? != b"\0asm\x01\0\0\0" {
        return None;
    }
    let mut at = 8;
    let mut found = None;
    while at < m.len() {
        let id = m[at];
        at += 1;
        let (size, n) = leb(&m[at..])?;
        at += n;
        let end = at.checked_add(size)?;
        let section = m.get(at..end)?;
        if id == 0 {
            let (len, n) = leb(section)?;
            let name = section.get(n..n.checked_add(len)?)?;
            if name == b"vela.note" {
                if found.is_some() {
                    return None;
                }
                found = Some(section.get(n + len..)?);
            }
        }
        at = end;
    }
    found
}
