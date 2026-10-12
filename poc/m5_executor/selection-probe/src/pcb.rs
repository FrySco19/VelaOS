#![no_std]
#![no_main]
#![feature(asm_cfg)]

use sel4_microkit::{
    Channel, ChannelSet, Handler, debug_println, memory_region_symbol, protection_domain,
};
use vela_selection_contract::{UserSelection, VerificationError, WIRE_SIZE, verify_selection};
use vela_selection_probe::*;
mod key {
    include!(env!("VELA_TRUSTED_INPUT_PUBLIC_RS"));
}

const TRUSTED_INPUT_CHANNEL: usize = 1;
const EXECUTOR_CHANNEL: usize = 2;
struct Pcb {
    input: Page,
    output: Page,
    busy: bool,
    accepted: u32,
    denied: u32,
}
impl Pcb {
    fn reply(&self, accepted: bool) {
        self.input.clear();
        self.input.write(&[u8::from(accepted)]);
        self.input.set(ACK);
        Channel::new(TRUSTED_INPUT_CHANNEL).notify();
    }
}
impl Pcb {
    fn on_channel(&mut self, channel: Channel) {
        if channel == Channel::new(EXECUTOR_CHANNEL) {
            if !self.busy || self.output.get() != ACK {
                return;
            }
            self.output.clear();
            self.output.set(EMPTY);
            self.busy = false;
            self.reply(true);
            return;
        }
        if channel != Channel::new(TRUSTED_INPUT_CHANNEL) || self.busy || self.input.get() != READY
        {
            return;
        }
        let bytes = self.input.read::<WIRE_SIZE>();
        let selection = UserSelection::decode(&bytes).expect("fixture wire format");
        let result = verify_selection(
            &selection,
            PLAN,
            &SESSION,
            REVISION,
            &key::TRUSTED_PUBLIC_KEY,
        );
        match result {
            Ok(()) => {
                // Fixture's one-shot policy, not a production Biscuit authority.
                assert_eq!(self.accepted, 0, "replayed approval");
                assert_eq!(self.output.get(), EMPTY);
                self.accepted += 1;
                let mut token = [0; 40];
                token[..8].copy_from_slice(b"VELATEST");
                token[8..].copy_from_slice(&selection.plan_sha256);
                self.output.write(&token);
                self.busy = true;
                debug_println!("SELECTION PCB: VALID Ed25519; emitted bounded TEST authorization");
                self.output.set(READY);
                Channel::new(EXECUTOR_CHANNEL).notify();
            }
            Err(reason) => {
                self.denied += 1;
                assert_eq!(self.output.get(), EMPTY);
                self.output.clear();
                match reason {
                    VerificationError::InvalidAttestation => {
                        debug_println!("SELECTION PCB: REJECT bad signature; authority=0")
                    }
                    VerificationError::PlanChanged => {
                        debug_println!("SELECTION PCB: REJECT altered hash; authority=0")
                    }
                    _ => panic!("unexpected fixture denial"),
                }
                if self.denied == 2 {
                    assert_eq!(self.accepted, 1);
                }
                self.reply(false);
            }
        }
    }
}

impl Handler for Pcb {
    type Error = core::convert::Infallible;
    fn notified(&mut self, channels: ChannelSet) -> Result<(), Self::Error> {
        for channel in channels.iter() {
            self.on_channel(channel);
        }
        Ok(())
    }
}

#[protection_domain]
fn init() -> impl Handler {
    let input = unsafe { Page::new(memory_region_symbol!(selection_input_addr: *mut u8).as_ptr()) };
    let output =
        unsafe { Page::new(memory_region_symbol!(selection_output_addr: *mut u8).as_ptr()) };
    assert_eq!(input.get(), EMPTY);
    assert_eq!(output.get(), EMPTY);
    debug_println!("SELECTION PCB: ready; no_std shared verifier ed25519-dalek");
    Channel::new(TRUSTED_INPUT_CHANNEL).notify();
    Pcb {
        input,
        output,
        busy: false,
        accepted: 0,
        denied: 0,
    }
}
