use anyhow::{Context, Result, anyhow, ensure};
use std::sync::atomic::{AtomicUsize, Ordering};
use wasmtime::{Config, Engine, Store, StoreLimits, StoreLimitsBuilder};
use wasmtime::component::{Component, Linker};

wasmtime::component::bindgen!({
    path: "wit",
    world: "task",
});

pub const MIN_MEMORY_BYTES: usize = 16 * 1024 * 1024;
pub const DEFAULT_MEMORY_BYTES: usize = MIN_MEMORY_BYTES;
pub const MAX_MEMORY_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_TASKS: usize = 2;
const TASK_FUEL: u64 = 10_000_000;
const MAX_INPUT_BYTES: usize = 1024 * 1024;

static ACTIVE_TASKS: AtomicUsize = AtomicUsize::new(0);

struct TaskPermit;

impl TaskPermit {
    fn acquire() -> Result<Self> {
        ACTIVE_TASKS
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |active| {
                (active < MAX_TASKS).then_some(active + 1)
            })
            .map(|_| Self)
            .map_err(|_| anyhow!("maximum concurrent Wasm tasks reached"))
    }
}

impl Drop for TaskPermit {
    fn drop(&mut self) {
        ACTIVE_TASKS.fetch_sub(1, Ordering::AcqRel);
    }
}

struct TaskState {
    limits: StoreLimits,
}

pub struct Sandbox {
    engine: Engine,
    memory_bytes: usize,
}

impl Sandbox {
    pub fn new() -> Result<Self> {
        Self::with_memory_limit(DEFAULT_MEMORY_BYTES)
    }

    pub fn with_memory_limit(memory_bytes: usize) -> Result<Self> {
        ensure!(
            (MIN_MEMORY_BYTES..=MAX_MEMORY_BYTES).contains(&memory_bytes),
            "Wasm memory limit must be between 16 MiB and 64 MiB"
        );

        let mut config = Config::new();
        config.wasm_component_model(true);
        config.consume_fuel(true);

        Ok(Self {
            engine: Engine::new(&config).context("create Wasmtime engine")?,
            memory_bytes,
        })
    }

    pub fn run(&self, component_bytes: &[u8], input: &str) -> Result<String> {
        ensure!(input.len() <= MAX_INPUT_BYTES, "Wasm input exceeds 1 MiB");
        let _permit = TaskPermit::acquire()?;
        let component = Component::from_binary(&self.engine, component_bytes)
            .context("compile Vela task component")?;
        let limits = StoreLimitsBuilder::new()
            .memory_size(self.memory_bytes)
            .memories(1)
            .instances(1)
            .tables(0)
            .build();
        let mut store = Store::new(&self.engine, TaskState { limits });
        store.limiter(|state| &mut state.limits);
        store.set_fuel(TASK_FUEL)?;

        // An empty linker intentionally exposes no WASI or ambient host capabilities.
        let linker = Linker::<TaskState>::new(&self.engine);
        let task = Task::instantiate(&mut store, &component, &linker)
            .context("instantiate Vela task; imports are not permitted")?;
        let output = task
            .call_run(&mut store, input)
            .context("execute Vela task")?
            .map_err(|message| anyhow!("Vela task returned an error: {message}"))?;
        ensure!(output.len() <= MAX_INPUT_BYTES, "Wasm output exceeds 1 MiB");
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_limit_is_bounded_to_specification() -> Result<()> {
        assert!(Sandbox::with_memory_limit(MIN_MEMORY_BYTES - 1).is_err());
        assert!(Sandbox::with_memory_limit(MAX_MEMORY_BYTES + 1).is_err());
        Sandbox::with_memory_limit(MIN_MEMORY_BYTES)?;
        Sandbox::with_memory_limit(MAX_MEMORY_BYTES)?;
        Ok(())
    }

    #[test]
    fn concurrent_task_slots_are_strictly_bounded() -> Result<()> {
        let first = TaskPermit::acquire()?;
        let second = TaskPermit::acquire()?;
        assert!(TaskPermit::acquire().is_err());
        drop(first);
        let replacement = TaskPermit::acquire()?;
        drop((second, replacement));
        Ok(())
    }
}