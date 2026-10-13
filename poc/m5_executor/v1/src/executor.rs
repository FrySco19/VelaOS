#![no_std]
#![no_main]
#![feature(asm_cfg)]
use ed25519_dalek::{SigningKey, VerifyingKey};
use sel4_microkit::{
    Channel, ChannelSet, Handler, debug_println, memory_region_symbol, protection_domain,
};
use vela_note_action::{CAP_SIZE, Capability, MAX_NOTE, Operation, RECEIPT_SIZE, execute_timed};
use vela_protocol::{Frame, FrameRef, MAX_FRAME, MessageType};
use vela_storage_core::ObjectStore;
use vela_v1_domains::{Slot, now_ns};
mod storage_core {
    pub use vela_storage_core::*;
}
#[path = "../../app/src/virtio_block.rs"]
mod virtio_block;
#[unsafe(no_mangle)]
static mut lfs_dma_phys: u64 = 0;
struct Executor {
    input: Slot,
    reply: Slot,
    store: ObjectStore<virtio_block::VirtioBlock>,
    pcb: VerifyingKey,
    lfs: SigningKey,
}
impl Executor {
    fn run(&mut self, bytes: &[u8]) -> Result<(), &'static str> {
        let frame = FrameRef::decode(bytes).map_err(|_| "FRAME")?;
        if frame.header.kind != MessageType::Approval || frame.header.sequence != 0 {
            return Err("KIND");
        }
        let b = frame.payload;
        if b.len() < 18 + CAP_SIZE {
            return Err("LENGTH");
        }
        let epoch = u64::from_le_bytes(b[..8].try_into().unwrap());
        let id = u64::from_le_bytes(b[8..16].try_into().unwrap());
        let n = u16::from_le_bytes(b[16..18].try_into().unwrap()) as usize;
        if n > MAX_NOTE || b.len() != 18 + CAP_SIZE + n || id != frame.header.transaction {
            return Err("CONTEXT");
        }
        let op = Operation::new(epoch, id, &b[18 + CAP_SIZE..]).map_err(|_| "NOTE")?;
        let cap = Capability(b[18..18 + CAP_SIZE].try_into().unwrap());
        let receipt = execute_timed(&mut self.store, &op, &cap, now_ns, &self.pcb, &self.lfs)
            .map_err(|_| "WASM_OR_DURABILITY")?;
        debug_println!(
            "V1 EXECUTOR: Wasm executed; VirtIO flush/readback complete; signed receipt generation={}",
            self.store.version().unwrap().generation
        );
        let mut payload = [0; CAP_SIZE + RECEIPT_SIZE];
        payload[..CAP_SIZE].copy_from_slice(&cap.0);
        payload[CAP_SIZE..].copy_from_slice(&receipt.0);
        let response = Frame::new(MessageType::Receipt, id, 0, &payload).map_err(|_| "FRAME")?;
        let mut wire = [0; MAX_FRAME];
        let wire = response.encode(&mut wire).map_err(|_| "ENCODE")?;
        self.reply.publish(wire)?;
        Ok(())
    }
}
impl Handler for Executor {
    type Error = core::convert::Infallible;
    fn notified(&mut self, _: ChannelSet) -> Result<(), Self::Error> {
        if self.input.ready() && self.reply.empty() {
            let mut bytes = [0; MAX_FRAME];
            let result = self
                .input
                .take(&mut bytes)
                .and_then(|n| self.run(&bytes[..n]));
            self.input.release();
            Channel::new(0).notify();
            if let Err(reason) = result {
                debug_println!("V1 EXECUTOR: DENIED {}; no success receipt", reason);
            }
        }
        Ok(())
    }
}
#[protection_domain(heap_size=2*1024*1024)]
fn init() -> impl Handler {
    let mmio = memory_region_symbol!(lfs_mmio_addr:*mut u8).as_ptr();
    let dma = memory_region_symbol!(lfs_dma_addr:*mut u8).as_ptr();
    let physical = unsafe { core::ptr::read_volatile(&raw const lfs_dma_phys) };
    let device = unsafe { virtio_block::VirtioBlock::new_selected(mmio.add(0xe00), dma, physical) }
        .expect("dedicated disk");
    let mut store = ObjectStore::open(device).expect("LFS recovery");
    let lfs = SigningKey::from_bytes(include_bytes!(env!("VELA_LFS_SEED")));
    let mut recovered = [0; MAX_NOTE];
    let n = vela_note_action::recover(&mut store, &lfs.verifying_key(), &mut recovered)
        .expect("signed persisted state");
    if store.version().is_some() {
        debug_println!(
            "V1 LFS: AUTHENTICATED AFTER REBOOT note={}",
            core::str::from_utf8(&recovered[..n]).expect("UTF8")
        );
    }
    debug_println!(
        "V1 LFS: recovered generation={}",
        store.version().map_or(0, |v| v.generation)
    );
    Executor {
        input: Slot(memory_region_symbol!(pcb_exec_addr:*mut u8).as_ptr()),
        reply: Slot(memory_region_symbol!(exec_reply_addr:*mut u8).as_ptr()),
        store,
        pcb: VerifyingKey::from_bytes(include_bytes!(env!("VELA_PCB_PUBLIC"))).expect("PCB root"),
        lfs,
    }
}
