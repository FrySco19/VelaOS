#![no_std]
#![no_main]
#![feature(asm_cfg)]

use sel4_microkit::{
    Channel, ChannelSet, Handler, debug_println, memory_region_symbol, protection_domain,
};
use sha2::{Digest, Sha256};
use vela_selection_probe::*;

struct Sink {
    page: Page,
    count: u32,
}
impl Sink {
    fn on_channel(&mut self, channel: Channel) {
        assert_eq!(channel, Channel::new(0));
        if self.page.get() != READY {
            return;
        }
        let token = self.page.read::<40>();
        assert_eq!(&token[..8], b"VELATEST");
        let hash: [u8; 32] = Sha256::digest(PLAN).into();
        assert_eq!(&token[8..], &hash);
        self.count += 1;
        assert_eq!(self.count, 1);
        debug_println!("SELECTION EXECUTOR SINK: TEST authorization received once; hash verified");
        self.page.clear();
        self.page.set(ACK);
        Channel::new(0).notify();
    }
}

impl Handler for Sink {
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
    let pointer = memory_region_symbol!(selection_output_addr: *mut u8).as_ptr();
    debug_println!("SELECTION EXECUTOR SINK: ready; no Wasm dispatch for TEST token");
    Sink {
        page: unsafe { Page::new(pointer) },
        count: 0,
    }
}
