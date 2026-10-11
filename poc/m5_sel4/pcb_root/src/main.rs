#![no_std]
#![no_main]
#![feature(asm_cfg)]

extern crate alloc;

use core::ptr;

use object::{Object, ObjectSymbol};
use sel4::ObjectBlueprint;
use sel4_root_task::{Never, debug_println, root_task};

mod child_vspace;
mod object_allocator;

use child_vspace::create_child_vspace;
use object_allocator::ObjectAllocator;

const CHILD_ELF_CONTENTS: &[u8] = include_bytes!(env!("RECIPIENT_ELF"));
const RECIPIENT_CNODE_SIZE_BITS: usize = 2;
const GRANT_SLOT: usize = 1;
const GRANT_BADGE: sel4::Word = 0x5645_4c41;
const GRANULE_SIZE: usize = sel4::FrameObjectType::GRANULE.bytes();

#[root_task(heap_size = 1024 * 64)]
fn main(bootinfo: &sel4::BootInfoPtr) -> sel4::Result<Never> {
    let mut allocator = ObjectAllocator::new(bootinfo);
    let free_page_addr = init_free_page_addr(bootinfo);
    let child_image = object::File::parse(CHILD_ELF_CONTENTS).unwrap();
    let (child_vspace, ipc_buffer_addr, ipc_buffer_cap) = create_child_vspace(
        &mut allocator,
        &child_image,
        sel4::init_thread::slot::VSPACE.cap(),
        free_page_addr,
        sel4::init_thread::slot::ASID_POOL.cap(),
    );

    let recipient_cnode =
        allocator.allocate_variable_sized::<sel4::cap_type::CNode>(RECIPIENT_CNODE_SIZE_BITS);
    let endpoint = allocator.allocate_fixed_sized::<sel4::cap_type::Endpoint>();
    let endpoint_source = sel4::init_thread::slot::CNODE.cap().absolute_cptr(endpoint);
    let grant_slot = recipient_cnode.absolute_cptr_from_bits_with_depth(
        GRANT_SLOT as sel4::CPtrBits,
        RECIPIENT_CNODE_SIZE_BITS,
    );
    grant_slot
        .mint(&endpoint_source, sel4::CapRights::write_only(), GRANT_BADGE)
        .expect("failed to mint recipient endpoint capability");

    let reply_object = allocator
        .allocate(ObjectBlueprint::Reply)
        .cast::<sel4::cap_type::Reply>();

    let recipient_tcb = allocator.allocate_fixed_sized::<sel4::cap_type::Tcb>();
    let recipient_sched_context =
        allocator.allocate_variable_sized::<sel4::cap_type::SchedContext>(8);
    bootinfo
        .sched_control()
        .index(0)
        .cap()
        .sched_control_configure_flags(recipient_sched_context, 10_000, 10_000, 0, 0, 0)
        .expect("failed to configure recipient scheduling context");
    recipient_tcb
        .tcb_configure(
            recipient_cnode,
            sel4::CNodeCapData::new(0, sel4::WORD_SIZE - RECIPIENT_CNODE_SIZE_BITS),
            child_vspace,
            ipc_buffer_addr as sel4::Word,
            ipc_buffer_cap,
        )
        .expect("failed to configure recipient TCB");
    recipient_tcb
        .tcb_set_sched_params(
            sel4::init_thread::slot::TCB.cap(),
            100,
            100,
            recipient_sched_context,
            sel4::CPtr::from_bits(0).cast(),
        )
        .expect("failed to assign recipient scheduling context");

    let child_stack_top = child_image
        .symbol_by_name("CHILD_STACK")
        .expect("recipient ELF is missing CHILD_STACK")
        .address()
        + GRANULE_SIZE as u64;
    let mut context = sel4::UserContext::default();
    *context.pc_mut() = child_image.entry().try_into().unwrap();
    *context.sp_mut() = child_stack_top.try_into().unwrap();
    *context.c_param_mut(0) = ipc_buffer_addr as sel4::Word;
    recipient_tcb
        .tcb_write_all_registers(true, &mut context)
        .expect("failed to start recipient PD");
    recipient_tcb
        .tcb_resume()
        .expect("failed to resume recipient PD");

    debug_println!("Gate C: minted badged send capability into live recipient CSpace");
    let (request, badge) = endpoint.recv(reply_object);
    assert_eq!(
        badge, GRANT_BADGE,
        "recipient did not use the derived grant"
    );
    reply_object.send(sel4::MessageInfo::new(badge, 0, 0, request.length()));
    debug_println!("Gate C: recipient used the grant; badge verified");

    grant_slot
        .delete()
        .expect("failed to revoke the recipient endpoint capability");
    recipient_tcb
        .tcb_suspend()
        .expect("failed to suspend recipient after revocation");
    debug_println!("Gate C: revoked the recipient endpoint capability");

    sel4::init_thread::suspend_self()
}

#[repr(C, align(4096))]
struct FreePagePlaceholder([u8; GRANULE_SIZE]);

static mut FREE_PAGE_PLACEHOLDER: FreePagePlaceholder = FreePagePlaceholder([0; GRANULE_SIZE]);

fn init_free_page_addr(bootinfo: &sel4::BootInfoPtr) -> usize {
    let addr = ptr::addr_of!(FREE_PAGE_PLACEHOLDER) as usize;
    get_user_image_frame_slot(bootinfo, addr)
        .cap()
        .frame_unmap()
        .unwrap();
    addr
}

fn get_user_image_frame_slot(
    bootinfo: &sel4::BootInfoPtr,
    addr: usize,
) -> sel4::init_thread::Slot<sel4::cap_type::Granule> {
    unsafe extern "C" {
        static __executable_start: usize;
    }
    let user_image_addr = ptr::addr_of!(__executable_start) as usize;
    bootinfo
        .user_image_frames()
        .index(addr / GRANULE_SIZE - user_image_addr / GRANULE_SIZE)
}
