#![no_std]
#![no_main]
#![feature(asm_cfg)]

use core::{
    ptr::{read_volatile, write_volatile},
    sync::atomic::{AtomicU32, Ordering},
};
use sel4_microkit::{
    Channel, ChannelSet, Handler, debug_println, memory_region_symbol, protection_domain,
};
use vela_protocol::{FrameRef, MAX_FRAME, MessageType};
use vela_selection_contract::{UserSelection, verify_selection};
use vela_selection_probe::{ACK, EMPTY, PLAN, Page, READY, REVISION, SESSION};
mod key {
    include!(env!("VELA_TRUSTED_INPUT_PUBLIC_RS"));
}

const INPUT_CHANNEL: usize = 1;
const EXECUTOR_CHANNEL: usize = 2;

struct Mailbox(*mut u8);
impl Mailbox {
    fn state(&self) -> &AtomicU32 {
        unsafe { &*self.0.cast::<AtomicU32>() }
    }
    fn snapshot(&self, out: &mut [u8; MAX_FRAME]) -> Result<usize, &'static str> {
        let len = unsafe { u32::from_le(read_volatile(self.0.add(4).cast::<u32>())) } as usize;
        if !(vela_protocol::HEADER_SIZE..=MAX_FRAME).contains(&len) {
            return Err("FRAME_LENGTH");
        }
        for (i, byte) in out[..len].iter_mut().enumerate() {
            *byte = unsafe { read_volatile(self.0.add(8 + i)) };
        }
        Ok(len)
    }
    fn release(&self) {
        for i in 4..4096 {
            unsafe {
                write_volatile(self.0.add(i), 0);
            }
        }
        self.state().store(EMPTY, Ordering::Release);
        Channel::new(INPUT_CHANNEL).notify();
    }
}

struct Pcb {
    input: Mailbox,
    output: Page,
    busy: bool,
    consumed: bool,
}
impl Pcb {
    fn verify(&self, bytes: &[u8]) -> Result<UserSelection, &'static str> {
        let frame = FrameRef::decode(bytes).map_err(|_| "FRAME_DECODE")?;
        if frame.header.kind != MessageType::Approval {
            return Err("MESSAGE_TYPE");
        }
        let selection = UserSelection::decode(frame.payload).map_err(|_| "SELECTION_DECODE")?;
        // Headers are not signed independently: require matching signed context.
        if frame.header.transaction != selection.revision || frame.header.sequence != 0 {
            return Err("TRANSACTION_BINDING");
        }
        verify_selection(
            &selection,
            PLAN,
            &SESSION,
            REVISION,
            &key::TRUSTED_PUBLIC_KEY,
        )
        .map_err(|_| "CRYPTO_OR_CONTEXT")?;
        if self.consumed {
            return Err("REPLAY");
        }
        Ok(selection)
    }
    fn receive(&mut self) {
        if self.busy || self.input.state().load(Ordering::Acquire) != READY {
            return;
        }
        let mut bytes = [0; MAX_FRAME];
        let verified = self
            .input
            .snapshot(&mut bytes)
            .and_then(|len| self.verify(&bytes[..len]));
        match verified {
            Ok(selection) => {
                if self.output.get() != EMPTY {
                    debug_println!("SELECTION PCB: REJECT output busy; authority=0");
                    self.input.release();
                    return;
                }
                // TEST-only one-shot state. Production needs durable consumption
                // and real policy; signature validity alone is not authorization.
                self.consumed = true;
                self.busy = true;
                self.input.release();
                let mut token = [0; 40];
                token[..8].copy_from_slice(b"VELATEST");
                token[8..].copy_from_slice(&selection.plan_sha256);
                self.output.clear();
                self.output.write(&token);
                self.output.set(READY);
                debug_println!(
                    "SELECTION PCB: VALID Ed25519; framed host approval; TEST authority=1"
                );
                Channel::new(EXECUTOR_CHANNEL).notify();
            }
            Err(reason) => {
                self.input.release();
                debug_println!("SELECTION PCB: REJECT {}; authority=0", reason);
            }
        }
    }
}

impl Handler for Pcb {
    type Error = core::convert::Infallible;
    fn notified(&mut self, channels: ChannelSet) -> Result<(), Self::Error> {
        if channels.contains(Channel::new(EXECUTOR_CHANNEL))
            && self.busy
            && self.output.get() == ACK
        {
            self.output.clear();
            self.output.set(EMPTY);
            self.busy = false;
        }
        // Recheck the slot after ACK: combined notifications do not lose work.
        if channels.contains(Channel::new(INPUT_CHANNEL)) || !self.busy {
            self.receive();
        }
        Ok(())
    }
}

#[protection_domain]
fn init() -> impl Handler {
    let input = memory_region_symbol!(ingress_pcb_addr: *mut u8).as_ptr();
    assert!(!input.is_null() && input as usize & 4095 == 0);
    let output =
        unsafe { Page::new(memory_region_symbol!(selection_output_addr: *mut u8).as_ptr()) };
    debug_println!("SELECTION PCB: framed host ingress ready; TEST session/policy");
    Pcb {
        input: Mailbox(input),
        output,
        busy: false,
        consumed: false,
    }
}
