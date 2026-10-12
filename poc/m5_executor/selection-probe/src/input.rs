#![no_std]
#![no_main]
#![feature(asm_cfg)]

use ed25519_compact::{KeyPair, Seed};
use sel4_microkit::{
    Channel, ChannelSet, Handler, debug_println, memory_region_symbol, protection_domain,
};
use sha2::{Digest, Sha256};
use vela_selection_contract::UserSelection;
use vela_selection_probe::*;

struct Input {
    page: Page,
    key: KeyPair,
    step: u8,
}
impl Input {
    fn on_channel(&mut self, channel: Channel) {
        assert_eq!(channel, Channel::new(0));
        if self.step == 4 {
            return;
        }
        if self.step > 0 {
            assert_eq!(self.page.get(), ACK);
            let reply = self.page.read::<1>()[0];
            assert_eq!(reply, if self.step == 1 { 1 } else { 0 });
            self.page.clear();
            self.page.set(EMPTY);
        }
        if self.step == 3 {
            debug_println!(
                "SELECTION PROBE PASS: valid approval, bad signature and altered hash over IPC"
            );
            self.step = 4;
            return;
        }
        assert_eq!(self.page.get(), EMPTY);
        let mut selection = UserSelection {
            session_binding: SESSION,
            revision: REVISION,
            plan_sha256: Sha256::digest(PLAN).into(),
            signature: [0; 64],
        };
        let signature = self.key.sk.sign(selection.signing_bytes(), None);
        selection.signature.copy_from_slice(signature.as_ref());
        match self.step {
            0 => debug_println!("SELECTION INPUT: publish signed valid UserSelection"),
            1 => {
                selection.signature[0] ^= 1;
                debug_println!("SELECTION INPUT: publish bad signature");
            }
            2 => {
                selection.plan_sha256[0] ^= 1;
                debug_println!("SELECTION INPUT: publish altered hash");
            }
            _ => unreachable!(),
        }
        self.page.write(&selection.encode());
        self.page.set(READY);
        self.step += 1;
        Channel::new(0).notify();
    }
}

impl Handler for Input {
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
    debug_println!("SELECTION INPUT: TEST FIXTURE; no physical human click");
    let seed = include_bytes!(env!("VELA_TRUSTED_INPUT_FIXTURE_SEED"));
    let key = KeyPair::from_seed(Seed::new(*seed));
    let pointer = memory_region_symbol!(selection_input_addr: *mut u8).as_ptr();
    Input {
        page: unsafe { Page::new(pointer) },
        key,
        step: 0,
    }
}
