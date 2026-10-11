#![no_std]
#![no_main]
#![feature(asm_cfg)]

#[path = "../bridge.rs"]
mod bridge;

use core::sync::atomic::{Ordering, fence};
use sel4_microkit::{
    Channel, Handler, NullHandler, debug_println, memory_region_symbol, protection_domain,
};

#[protection_domain]
fn init() -> impl Handler {
    let page = memory_region_symbol!(shared_page_addr: *mut u8);
    unsafe {
        bridge::write_mock_request(page.as_ptr());
    }
    fence(Ordering::Release);
    debug_println!(
        "VelaOS C-D bridge: PCB published request {} with mock token",
        1
    );
    Channel::new(0).notify();
    NullHandler::new()
}
