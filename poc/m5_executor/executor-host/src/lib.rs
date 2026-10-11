//! Host-side model of the M5 Executor (`poc/m5_executor/app/src/main.rs`).
//!
//! The wire format, descriptor parsing and Ed25519 admission verification are
//! the *unchanged guest code*: `app/src/bridge.rs` is compiled in via `#[path]`
//! exactly as the keygen tests do. The Wasmi execution path cannot be linked
//! from the guest binary (it depends on `sel4-microkit`), so
//! [`run_authenticated`] mirrors it line for line; `tests/guest_parity.rs`
//! fails if the guest configuration drifts from this mirror.
//!
//! Obligations are numbered in `poc/m5_executor/EXECUTOR_CONTRACT.md` (E1..E9).
//! This crate is a test harness: it is not the Executor, adds no capability and
//! is never linked into a guest image.

#[path = "../../app/src/bridge.rs"]
pub mod bridge;

use bridge::{PAGE_SIZE, Status};
use ed25519_compact::PublicKey;
use wasmi::{Config, Engine, Linker, Module, Store, StoreLimits, StoreLimitsBuilder, TrapCode};

/// Interpreter value-stack ceiling (guest: `set_max_stack_height(16 * 1024)`).
pub const MAX_STACK_HEIGHT: usize = 16 * 1024;
/// Call-depth ceiling (guest: `set_max_recursion_depth(64)`).
pub const MAX_RECURSION_DEPTH: usize = 64;
/// Largest payload a v2 frame can carry: the only bound on parse/validate work (E3, E8).
pub const MAX_PAYLOAD_BYTES: usize = PAGE_SIZE - bridge::PAYLOAD_OFFSET;

/// The four IPC reply registers of label 0x702 (E5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Reply {
    /// regs[0]: C5 code 0..=6.
    pub status: Status,
    /// regs[1]: the i32 returned by `run`, as u32 bits; 0 on any error.
    pub value: u32,
    /// regs[2]: request ID. Trusted only when `status` is not Malformed and the
    /// frame authenticated (see EXECUTOR_CONTRACT E6).
    pub request_id: u32,
    /// regs[3]: number of instantiation attempts in this session.
    pub attempts: u64,
}

/// Session state of one Executor boot (E7: replay state is boot-local today).
pub struct HostExecutor {
    root: PublicKey,
    last_consumed: u32,
    attempts: u64,
}

impl HostExecutor {
    pub fn new(root: PublicKey) -> Self {
        Self { root, last_consumed: 0, attempts: 0 }
    }
    pub fn last_consumed(&self) -> u32 {
        self.last_consumed
    }
    pub fn attempts(&self) -> u64 {
        self.attempts
    }

    /// Mirror of `ExecutorHandler::protected` for a correctly labelled call.
    pub fn call(&mut self, shared: &[u8; PAGE_SIZE]) -> Reply {
        self.protected(0, bridge::EXECUTE_LABEL, 0, shared)
    }

    /// Mirror of `ExecutorHandler::protected` including channel/label/count checks.
    pub fn protected(
        &mut self,
        channel: usize,
        label: u64,
        count: usize,
        shared: &[u8; PAGE_SIZE],
    ) -> Reply {
        if channel != 0 || label != bridge::EXECUTE_LABEL || count != 0 {
            return self.reply(Status::Malformed, 0, 0);
        }
        // E1: owned snapshot before any parse/verify; only the snapshot executes.
        // SAFETY: `shared` is a valid, readable 4 KiB buffer for this call.
        let snapshot = unsafe { bridge::snapshot(shared.as_ptr()) };
        let verified = match bridge::verify_with_key(&snapshot, self.last_consumed, self.root) {
            Ok(verified) => verified,
            Err(status) => {
                let id = u32::from_le_bytes(snapshot[8..12].try_into().unwrap());
                return self.reply(status, 0, id);
            }
        };
        // E7: consumed before instantiation, including executions that fail.
        self.last_consumed = verified.descriptor.request_id;
        let d = verified.descriptor;
        let payload = &snapshot[d.payload_offset..d.payload_offset + d.payload_len];
        let (status, value) =
            match run_authenticated(payload, verified.fuel, verified.memory, &mut self.attempts) {
                Ok(value) => (Status::Success, value),
                Err(status) => (status, 0),
            };
        self.reply(status, value, d.request_id)
    }

    fn reply(&self, status: Status, value: i32, request_id: u32) -> Reply {
        Reply { status, value: value as u32, request_id, attempts: self.attempts }
    }
}

/// Line-for-line mirror of `run_authenticated` in `app/src/main.rs`.
pub fn run_authenticated(
    payload: &[u8],
    fuel: u64,
    memory: usize,
    attempts: &mut u64,
) -> Result<i32, Status> {
    let mut config = Config::default();
    config
        .consume_fuel(true)
        .set_max_stack_height(MAX_STACK_HEIGHT)
        .set_max_recursion_depth(MAX_RECURSION_DEPTH);
    let engine = Engine::new(&config);
    let module = Module::new(&engine, payload).map_err(|_| Status::Wasm)?;
    if module.imports().next().is_some() {
        return Err(Status::Wasm);
    }
    let limits = StoreLimitsBuilder::new()
        .memory_size(memory)
        .memories(1)
        .instances(1)
        .tables(0)
        .table_elements(0)
        .build();
    let mut store = Store::new(&engine, limits);
    store.limiter(|limits| limits);
    store.set_fuel(fuel).map_err(|_| Status::Wasm)?;
    let linker = Linker::<StoreLimits>::new(&engine); // NO WASI / host imports.
    *attempts += 1;
    let map_trap = |error: wasmi::Error| {
        if error.as_trap_code() == Some(TrapCode::OutOfFuel) {
            Status::Fuel
        } else {
            Status::Wasm
        }
    };
    let instance = linker
        .instantiate_and_start(&mut store, &module)
        .map_err(map_trap)?;
    let run = instance
        .get_typed_func::<(), i32>(&store, "run")
        .map_err(|_| Status::Wasm)?;
    run.call(&mut store, ()).map_err(map_trap)
}

/// Returns true when the module validates under the guest's Wasmi feature set.
/// Tests use it to prove a rejection came from a limit/linker rule, not a bad fixture.
pub fn validates(payload: &[u8]) -> bool {
    let mut config = Config::default();
    config.consume_fuel(true);
    Module::new(&Engine::new(&config), payload).is_ok()
}

/// Number of imports a module declares (0 for invalid modules).
pub fn import_count(payload: &[u8]) -> usize {
    Module::new(&Engine::default(), payload)
        .map(|m| m.imports().count())
        .unwrap_or(0)
}
