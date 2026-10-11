#![no_std]
#![no_main]
#![feature(asm_cfg)]

use sel4_microkit::{debug_println, protection_domain, Handler, NullHandler};

#[protection_domain]
fn init() -> impl Handler {
    debug_println!("VelaOS Gate A: seL4 Boot Successful");
    NullHandler::new()
}
