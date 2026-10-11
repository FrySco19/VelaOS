#![no_std]
#![no_main]
#![feature(asm_cfg)]

use sel4::{CPtr, MessageInfo, cap_type};

const GRANT_SLOT: usize = 1;

#[repr(C, align(4096))]
pub struct ChildStack([u8; 4096]);

#[unsafe(no_mangle)]
pub static mut CHILD_STACK: ChildStack = ChildStack([0; 4096]);

#[unsafe(no_mangle)]
pub extern "C" fn _start(ipc_buffer_addr: usize) -> ! {
    unsafe {
        let stack_bottom = core::ptr::addr_of_mut!(CHILD_STACK) as *mut u8;
        core::ptr::write_volatile(stack_bottom, 0);
        sel4::set_ipc_buffer(&mut *(ipc_buffer_addr as *mut sel4::IpcBuffer));
    }

    let endpoint = CPtr::from_bits(GRANT_SLOT as sel4::Word).cast::<cap_type::Endpoint>();
    sel4::debug_println!("Gate C: recipient entered isolated PD");
    let response = endpoint.call(MessageInfo::new(0, 0, 0, 0));
    assert_eq!(response.label(), 0x5645_4c41);
    sel4::debug_println!("Gate C: recipient received authorized reply");

    loop {
        core::hint::spin_loop();
    }
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {
        core::hint::spin_loop();
    }
}
