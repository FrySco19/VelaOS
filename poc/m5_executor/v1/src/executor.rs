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
mod broker_key {
    include!(env!("VELA_TRUSTED_INPUT_PUBLIC_RS"));
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
        if matches!(
            frame.header.kind,
            MessageType::ListRequest | MessageType::ReadRequest
        ) && frame.header.sequence == 0
        {
            let root =
                VerifyingKey::from_bytes(&broker_key::TRUSTED_PUBLIC_KEY).map_err(|_| "ROOT")?;
            let read = frame.header.kind == MessageType::ReadRequest;
            let (nonce, target) = vela_note_action::catalog::verify_request(
                read,
                frame.header.transaction,
                frame.payload,
                &root,
            )
            .map_err(|_| "READ_AUTHORITY")?;
            let mut payload = [0; vela_note_action::catalog::RESPONSE_SIZE];
            let n = vela_note_action::catalog::response(
                &mut self.store,
                &self.lfs,
                read,
                frame.header.transaction,
                &nonce,
                &target,
                &mut payload,
            )
            .map_err(|_| "INDEX")?;
            let response = Frame::new(
                if read {
                    MessageType::ReadResponse
                } else {
                    MessageType::ListResponse
                },
                frame.header.transaction,
                0,
                &payload[..n],
            )
            .map_err(|_| "FRAME")?;
            let mut wire = [0; MAX_FRAME];
            self.reply
                .publish(response.encode(&mut wire).map_err(|_| "ENCODE")?)?;
            debug_println!("SOFT LFS: authenticated current index returned");
            return Ok(());
        }
        if frame.header.kind != MessageType::Approval || frame.header.sequence != 0 {
            return Err("KIND");
        }
        let b = frame.payload;
        if b.len() < 50 + CAP_SIZE {
            return Err("LENGTH");
        }
        let epoch = u64::from_le_bytes(b[..8].try_into().unwrap());
        let id = u64::from_le_bytes(b[8..16].try_into().unwrap());
        let n = u16::from_le_bytes(b[16..18].try_into().unwrap()) as usize;
        if n > MAX_NOTE || b.len() != 50 + CAP_SIZE + n || id != frame.header.transaction {
            return Err("CONTEXT");
        }
        let op = Operation::document(
            epoch,
            id,
            b[18..34].try_into().unwrap(),
            b[34..50].try_into().unwrap(),
            &b[50 + CAP_SIZE..],
        )
        .map_err(|_| "NOTE")?;
        let cap = Capability(b[50..50 + CAP_SIZE].try_into().unwrap());
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
    let store = ObjectStore::open_archive(device).expect("LFS recovery");
    let lfs = SigningKey::from_bytes(include_bytes!(env!("VELA_LFS_SEED")));
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
