#![no_std]
#![no_main]
#![feature(asm_cfg)]

mod gate_e;

use sel4_microkit::{Channel, Handler, Infallible, MessageInfo, debug_println, protection_domain};
use wasmi::{Config, Engine, Linker, Module, Store, StoreLimitsBuilder, TrapCode};

const WASM_FUEL_LIMIT: u64 = 10_000_000;
const FUEL_PROBE_LIMIT: u64 = 64;
const MAX_WASM_MEMORY_BYTES: usize = 64 * 1024;

const WASM_TEST_MODULE: &[u8] = &[
    0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, // header
    0x01, 0x05, 0x01, 0x60, 0x00, 0x01, 0x7f, // (type (func (result i32)))
    0x03, 0x04, 0x03, 0x00, 0x00, 0x00, // three functions using type 0
    0x05, 0x04, 0x01, 0x01, 0x01, 0x02, // one memory, min 1 page, max 2
    0x07, 0x15, 0x03, 0x03, b'r', b'u', b'n', 0x00, 0x00, 0x04, b'g', b'r', b'o', b'w', 0x00, 0x01,
    0x04, b'b', b'u', b'r', b'n', 0x00, 0x02, // export run, grow and burn
    0x0a, 0x16, 0x03, 0x04, 0x00, 0x41, 0x2a, 0x0b, // run returns 42
    0x06, 0x00, 0x41, 0x01, 0x40, 0x00, 0x0b, // grow one page
    0x08, 0x00, 0x03, 0x40, 0x0c, 0x00, 0x0b, 0x00, 0x0b, // infinite loop for fuel test
];

#[protection_domain(heap_size = 2 * 1024 * 1024)]
fn init() -> impl Handler {
    debug_println!("VelaOS Gate D: bounded Wasm executor online");
    execute_bounded_module();
    debug_println!("VelaOS Gate D: fuel exhaustion and 64 KiB memory cap verified");
    HandlerImpl
}

struct HandlerImpl;

impl Handler for HandlerImpl {
    type Error = Infallible;

    fn protected(
        &mut self,
        channel: Channel,
        msg_info: MessageInfo,
    ) -> Result<MessageInfo, Self::Error> {
        if !gate_e::accepts(channel, 0, &msg_info, gate_e::EXECUTE_LABEL) {
            return Ok(gate_e::denied());
        }
        debug_println!("VelaOS Gate F: BEGIN wasm_invocation");
        let result = execute_bounded_module();
        debug_println!("VelaOS Gate F: END wasm_invocation");
        assert_eq!(result, gate_e::WASM_RESULT as i32);
        debug_println!("VelaOS Gate E: executor returned Wasm result {}", result);
        Ok(MessageInfo::new(
            gate_e::WASM_RESULT_LABEL_BASE + result as gate_e::MessageLabel,
            0,
        ))
    }
}

fn execute_bounded_module() -> i32 {
    let mut config = Config::default();
    config
        .consume_fuel(true)
        .set_max_stack_height(16 * 1024)
        .set_max_recursion_depth(64);

    let engine = Engine::new(&config);
    let module = Module::new(&engine, WASM_TEST_MODULE).expect("failed to validate Wasm module");
    let limits = StoreLimitsBuilder::new()
        .memory_size(MAX_WASM_MEMORY_BYTES)
        .memories(1)
        .tables(0)
        .table_elements(0)
        .instances(1)
        .build();
    let mut store = Store::new(&engine, limits);
    store.limiter(|limits| limits);
    store
        .set_fuel(WASM_FUEL_LIMIT)
        .expect("failed to set Wasm fuel limit");

    let linker = Linker::<wasmi::StoreLimits>::new(&engine);
    let instance = linker
        .instantiate_and_start(&mut store, &module)
        .expect("failed to instantiate Wasm module without host imports");
    let grow = instance
        .get_typed_func::<(), i32>(&store, "grow")
        .expect("Wasm module is missing grow export");
    let growth_result = grow
        .call(&mut store, ())
        .expect("bounded memory-growth probe failed");
    assert_eq!(
        growth_result, -1,
        "Wasm memory limiter allowed growth beyond one page"
    );
    let fuel_remaining = store.get_fuel().expect("fuel metering was disabled");
    assert!(
        fuel_remaining < WASM_FUEL_LIMIT,
        "Wasm execution did not consume fuel"
    );
    let run = instance
        .get_typed_func::<(), i32>(&store, "run")
        .expect("Wasm module is missing run export");
    let result = run.call(&mut store, ()).expect("bounded Wasm task failed");

    store
        .set_fuel(FUEL_PROBE_LIMIT)
        .expect("failed to set fuel exhaustion probe limit");
    let burn = instance
        .get_typed_func::<(), i32>(&store, "burn")
        .expect("Wasm module is missing burn export");
    match burn.call(&mut store, ()) {
        Err(error) if error.as_trap_code() == Some(TrapCode::OutOfFuel) => {}
        Err(error) => panic!("fuel probe trapped unexpectedly: {error:?}"),
        Ok(_) => panic!("fuel probe completed without exhausting its budget"),
    }
    result
}
