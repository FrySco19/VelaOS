#![no_std]
#![no_main]
#![feature(asm_cfg)]
use ed25519_dalek::SigningKey;
use sel4_microkit::{
    Channel, ChannelSet, Handler, debug_println, memory_region_symbol, protection_domain,
};
use vela_note_action::{CAP_SIZE, Capability, MAX_LEASE_NS, MAX_NOTE, Operation};
use vela_protocol::{Frame, FrameRef, MAX_FRAME, MessageType};
use vela_selection_contract::UserSelection;
use vela_v1_domains::{Slot, now_ns};
mod key {
    include!(env!("VELA_TRUSTED_INPUT_PUBLIC_RS"));
}
struct Pending {
    op: Operation,
    session: [u8; 32],
}
struct Pcb {
    input: Slot,
    output: Slot,
    executor_reply: Slot,
    reply: Slot,
    pending: Option<Pending>,
    busy: bool,
    key: SigningKey,
    last: u64,
}
impl Pcb {
    fn process(&mut self, bytes: &[u8]) -> Result<(), &'static str> {
        let frame = FrameRef::decode(bytes).map_err(|_| "FRAME")?;
        if frame.header.sequence != 0 {
            return Err("CHUNKS_UNSUPPORTED");
        }
        match frame.header.kind {
            MessageType::Proposal => {
                if self.busy || self.pending.is_some() {
                    return Err("BUSY");
                }
                let b = frame.payload;
                if b.len() < 50 {
                    return Err("PROPOSAL_LENGTH");
                }
                let epoch = u64::from_le_bytes(b[..8].try_into().unwrap());
                let id = u64::from_le_bytes(b[8..16].try_into().unwrap());
                let n = u16::from_le_bytes(b[48..50].try_into().unwrap()) as usize;
                if epoch != 1
                    || id <= self.last
                    || id != frame.header.transaction
                    || n > MAX_NOTE
                    || b.len() != 50 + n
                {
                    return Err("CONTEXT");
                }
                let op = Operation::new(epoch, id, &b[50..]).map_err(|_| "NOTE")?;
                self.pending = Some(Pending {
                    op,
                    session: b[16..48].try_into().unwrap(),
                });
                debug_println!("V1 PCB: Proposal staged; authority=0");
            }
            MessageType::Approval => {
                if self.busy || !self.output.empty() {
                    return Err("BUSY");
                }
                let pending = self.pending.take().ok_or("NO_PROPOSAL")?;
                if frame.header.transaction != pending.op.request {
                    return Err("TRANSACTION");
                }
                let approval = UserSelection::decode(frame.payload).map_err(|_| "SELECTION")?;
                let cap = Capability::issue(
                    &pending.op,
                    &approval,
                    &pending.session,
                    &key::TRUSTED_PUBLIC_KEY,
                    now_ns(),
                    MAX_LEASE_NS,
                    &self.key,
                )
                .map_err(|_| "CRYPTO")?;
                let mut payload = [0; 18 + CAP_SIZE + MAX_NOTE];
                payload[..8].copy_from_slice(&pending.op.epoch.to_le_bytes());
                payload[8..16].copy_from_slice(&pending.op.request.to_le_bytes());
                payload[16..18].copy_from_slice(&(pending.op.note().len() as u16).to_le_bytes());
                payload[18..18 + CAP_SIZE].copy_from_slice(&cap.0);
                payload[18 + CAP_SIZE..18 + CAP_SIZE + pending.op.note().len()]
                    .copy_from_slice(pending.op.note());
                let admitted = Frame::new(
                    MessageType::Approval,
                    pending.op.request,
                    0,
                    &payload[..18 + CAP_SIZE + pending.op.note().len()],
                )
                .map_err(|_| "FRAME")?;
                let mut wire = [0; MAX_FRAME];
                let wire = admitted.encode(&mut wire).map_err(|_| "ENCODE")?;
                self.output.publish(wire)?;
                self.last = pending.op.request;
                self.busy = true;
                debug_println!("V1 PCB: VALID Ed25519; scoped signed capability issued");
                Channel::new(2).notify();
            }
            MessageType::Cancel => {
                if self.busy {
                    return Err("COMMIT_IN_PROGRESS");
                }
                self.pending = None;
            }
            _ => return Err("KIND"),
        }
        Ok(())
    }
}
impl Handler for Pcb {
    type Error = core::convert::Infallible;
    fn notified(&mut self, _: ChannelSet) -> Result<(), Self::Error> {
        if self.executor_reply.ready() && self.reply.empty() {
            let mut bytes = [0; MAX_FRAME];
            if let Ok(n) = self.executor_reply.take(&mut bytes) {
                self.reply.publish(&bytes[..n]).expect("reply ownership");
                Channel::new(1).notify();
            }
            self.executor_reply.release();
            self.busy = false;
            Channel::new(2).notify();
        }
        if self.input.ready() {
            let mut bytes = [0; MAX_FRAME];
            let result = self
                .input
                .take(&mut bytes)
                .and_then(|n| self.process(&bytes[..n]));
            self.input.release();
            Channel::new(1).notify();
            if let Err(reason) = result {
                debug_println!("V1 PCB: DENIED {}; authority=0", reason);
            }
        }
        Ok(())
    }
}
#[protection_domain(heap_size=64*1024)]
fn init() -> impl Handler {
    // Development enrollment roots are generated locally, never fixed fixtures.
    // Embedded secret material is not production TPM-backed key custody.
    let key = SigningKey::from_bytes(include_bytes!(env!("VELA_PCB_SEED")));
    debug_println!("V1 PCB: real Ed25519 policy gate ready");
    Pcb {
        input: Slot(memory_region_symbol!(ingress_pcb_addr:*mut u8).as_ptr()),
        output: Slot(memory_region_symbol!(pcb_exec_addr:*mut u8).as_ptr()),
        executor_reply: Slot(memory_region_symbol!(exec_reply_addr:*mut u8).as_ptr()),
        reply: Slot(memory_region_symbol!(ingress_reply_addr:*mut u8).as_ptr()),
        pending: None,
        busy: false,
        key,
        last: 0,
    }
}
