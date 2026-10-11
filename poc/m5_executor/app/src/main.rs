#![no_std]
#![no_main]
#![feature(asm_cfg)]

mod bridge;

use core::sync::atomic::{Ordering, fence};
use sel4_microkit::{
    Channel, ChannelSet, Handler, debug_println, memory_region_symbol, protection_domain,
};
use wasmi::{Config, Engine, Linker, Module, Store, StoreLimits, StoreLimitsBuilder, TrapCode};

const TASK_FUEL: u64 = 10_000_000;
const FUEL_PROBE: u64 = 64;
const MAX_LINEAR_MEMORY: usize = 64 * 1024;

const RETURN_42_MODULE: &[u8] = &[
    0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, // Wasm header
    0x01, 0x05, 0x01, 0x60, 0x00, 0x01, 0x7f, // () -> i32
    0x03, 0x04, 0x03, 0x00, 0x00, 0x00, // three functions
    0x05, 0x04, 0x01, 0x01, 0x01, 0x02, // one memory, min 1/max 2 pages
    0x07, 0x15, 0x03, 0x03, b'r', b'u', b'n', 0x00, 0x00, 0x04, b'g', b'r', b'o', b'w', 0x00, 0x01,
    0x04, b'b', b'u', b'r', b'n', 0x00, 0x02, 0x0a, 0x16, 0x03, 0x04, 0x00, 0x41, 0x2a,
    0x0b, // run returns 42
    0x06, 0x00, 0x41, 0x01, 0x40, 0x00, 0x0b, // grow one page
    0x08, 0x00, 0x03, 0x40, 0x0c, 0x00, 0x0b, 0x00, 0x0b, // infinite loop
];

const WASI_IMPORT_MODULE: &[u8] = &[
    0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, // Wasm header
    0x01, 0x04, 0x01, 0x60, 0x00, 0x00, // () -> ()
    0x02, 0x24, 0x01, 0x16, b'w', b'a', b's', b'i', b'_', b's', b'n', b'a', b'p', b's', b'h', b'o',
    b't', b'_', b'p', b'r', b'e', b'v', b'i', b'e', b'w', b'1', 0x09, b'p', b'r', b'o', b'c', b'_',
    b'e', b'x', b'i', b't', 0x00, 0x00,
];

#[protection_domain(heap_size = 2 * 1024 * 1024)]
fn init() -> impl Handler {
    let result = run_fixture();
    assert_eq!(result, 42);
    debug_println!("VelaOS Gate D: no-WASI Wasmi executor initialized");
    debug_println!("VelaOS Gate D: imported WASI host call rejected");
    debug_println!("VelaOS Gate D: fuel exhaustion and 64 KiB memory cap verified");
    ExecutorHandler {
        shared_page: memory_region_symbol!(shared_page_addr: *mut u8).as_ptr(),
    }
}

struct ExecutorHandler {
    shared_page: *mut u8,
}

impl Handler for ExecutorHandler {
    type Error = core::convert::Infallible;

    fn notified(&mut self, channels: ChannelSet) -> Result<(), Self::Error> {
        if !channels.contains(Channel::new(0)) {
            debug_println!("VelaOS C-D bridge: ignored unrelated notification");
            return Ok(());
        }

        fence(Ordering::Acquire);
        match unsafe { bridge::parse_descriptor(self.shared_page) } {
            Ok(descriptor) => debug_println!(
                "VelaOS C-D bridge: Executor received descriptor request_id={} token_len={} payload_len={}",
                descriptor.request_id,
                descriptor.token_len,
                descriptor.payload_len
            ),
            Err(_) => debug_println!("VelaOS C-D bridge: rejected malformed descriptor"),
        }
        Ok(())
    }
}

fn run_fixture() -> i32 {
    let mut config = Config::default();
    config
        .consume_fuel(true)
        .set_max_stack_height(16 * 1024)
        .set_max_recursion_depth(64);

    let engine = Engine::new(&config);
    let no_import_linker = Linker::<StoreLimits>::new(&engine);
    let import_module =
        Module::new(&engine, WASI_IMPORT_MODULE).expect("validate WASI import probe");
    let mut import_store = Store::new(&engine, StoreLimitsBuilder::new().build());
    match no_import_linker.instantiate_and_start(&mut import_store, &import_module) {
        Err(_) => {}
        Ok(_) => panic!("empty linker unexpectedly resolved a WASI import"),
    }

    let module = Module::new(&engine, RETURN_42_MODULE).expect("validate embedded Wasm");
    let limits = StoreLimitsBuilder::new()
        .memory_size(MAX_LINEAR_MEMORY)
        .memories(1)
        .tables(0)
        .table_elements(0)
        .instances(1)
        .build();
    let mut store = Store::new(&engine, limits);
    store.limiter(|limits| limits);
    store.set_fuel(TASK_FUEL).expect("enable bounded fuel");

    // Empty linker: imports, including WASI, cannot resolve to host functions.
    let linker = no_import_linker;
    let instance = linker
        .instantiate_and_start(&mut store, &module)
        .expect("instantiate import-free Wasm module");
    let grow = instance
        .get_typed_func::<(), i32>(&store, "grow")
        .expect("missing memory-growth probe");
    assert_eq!(
        grow.call(&mut store, ()).expect("execute memory probe"),
        -1,
        "memory limiter allowed growth beyond 64 KiB"
    );
    let remaining_fuel = store.get_fuel().expect("fuel metering disabled");
    assert!(remaining_fuel < TASK_FUEL, "execution did not consume fuel");

    let run = instance
        .get_typed_func::<(), i32>(&store, "run")
        .expect("missing run export");
    let result = run.call(&mut store, ()).expect("run bounded Wasm task");

    store.set_fuel(FUEL_PROBE).expect("reset probe fuel");
    let burn = instance
        .get_typed_func::<(), i32>(&store, "burn")
        .expect("missing fuel probe");
    match burn.call(&mut store, ()) {
        Err(error) if error.as_trap_code() == Some(TrapCode::OutOfFuel) => {}
        Err(error) => panic!("fuel probe trapped unexpectedly: {error:?}"),
        Ok(_) => panic!("infinite-loop probe did not exhaust fuel"),
    }
    result
}
