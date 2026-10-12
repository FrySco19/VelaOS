use anyhow::{Context, Result, anyhow, ensure};
use std::sync::atomic::{AtomicUsize, Ordering};
use wasmtime::component::{Component, Linker};
use wasmtime::{Config, Engine, Store, StoreLimits, StoreLimitsBuilder};

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
    host: Option<HostContext>,
}

/// Trusted construction only: the guest cannot supply its principal, root or native LFS.
pub struct HostContext {
    pub principal: String,
    pub authority: std::sync::Arc<crate::authority::TaskAuthority>,
    pub lfs: std::sync::Arc<vela_lfs_m1::Lfs>,
}

impl vela::task::object_access::Host for TaskState {
    fn read(&mut self, handle: Vec<u8>, maximum: u32) -> std::result::Result<Vec<u8>, String> {
        let result = (|| -> Result<Vec<u8>> {
            ensure!(
                maximum > 0 && maximum <= 65536,
                "guest read window exceeds 64 KiB"
            );
            let host = self
                .host
                .as_ref()
                .ok_or_else(|| anyhow!("no object authority for this task"))?;
            let opaque = crate::authority::OpaqueHandle::from_wire(&handle)?;
            let bytes = host
                .authority
                .open_lfs_via_bus(&host.lfs, &opaque, &host.principal)?;
            ensure!(
                bytes.len() <= maximum as usize,
                "object exceeds guest read window"
            );
            Ok(bytes.to_vec())
        })();
        result.map_err(|_| "ObjectAccessDenied".to_owned())
    }
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
        self.run_inner(component_bytes, input, None)
    }

    pub fn run_with_host(
        &self,
        component_bytes: &[u8],
        input: &str,
        host: HostContext,
    ) -> Result<String> {
        self.run_inner(component_bytes, input, Some(host))
    }

    fn run_inner(
        &self,
        component_bytes: &[u8],
        input: &str,
        host: Option<HostContext>,
    ) -> Result<String> {
        ensure!(
            component_bytes.len() <= 4 * 1024 * 1024,
            "Wasm component exceeds 4 MiB"
        );
        ensure!(input.len() <= MAX_INPUT_BYTES, "Wasm input exceeds 1 MiB");
        let _permit = TaskPermit::acquire()?;
        let component = Component::from_binary(&self.engine, component_bytes)
            .context("compile Vela task component")?;
        let limits = StoreLimitsBuilder::new()
            .memory_size(self.memory_bytes)
            .memories(1)
            .instances(4)
            .tables(0)
            .build();
        let mut store = Store::new(&self.engine, TaskState { limits, host });
        store.limiter(|state| &mut state.limits);
        store.set_fuel(TASK_FUEL)?;

        // No WASI or ambient capabilities. Only explicit host-held object authority may be linked.
        let mut linker = Linker::<TaskState>::new(&self.engine);
        if store.data().host.is_some() {
            Task::add_to_linker::<_, wasmtime::component::HasSelf<_>>(&mut linker, |state| state)?;
        }
        let task = Task::instantiate(&mut store, &component, &linker)
            .context("instantiate Vela task; imports must match explicit host authority")?;
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
    static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn component(body: &str, pages: u32, forbidden_import: bool) -> Result<Vec<u8>> {
        let import = if forbidden_import {
            "(import \"ambient\" (func))"
        } else {
            ""
        };
        Ok(wat::parse_str(format!(
            r#"(component {import}
          (core module $m
            (memory (export "memory") {pages})
            (func (export "realloc") (param i32 i32 i32 i32) (result i32) i32.const 128)
            (func (export "run") (param i32 i32) (result i32) {body}))
          (core instance $i (instantiate $m))
          (func (export "run") (param "input" string) (result (result string (error string)))
            (canon lift (core func $i "run") (memory $i "memory") (realloc (func $i "realloc")))))"#
        ))?)
    }

    #[test]
    fn executes_real_component_and_traps_runaway_guest() -> Result<()> {
        let _guard = TEST_LOCK.lock().unwrap();
        let sandbox = Sandbox::new()?;
        assert_eq!(
            sandbox.run(&component("i32.const 0", 1, false)?, "hello")?,
            ""
        );
        let runaway = component("(loop $forever br $forever) i32.const 0", 1, false)?;
        let error = sandbox.run(&runaway, "hello").unwrap_err();
        assert!(format!("{error:#}").contains("fuel"));
        Ok(())
    }

    #[test]
    fn rejects_ambient_import_and_initial_memory_over_limit() -> Result<()> {
        let _guard = TEST_LOCK.lock().unwrap();
        let sandbox = Sandbox::new()?;
        assert!(
            sandbox
                .run(&component("i32.const 0", 1, true)?, "")
                .is_err()
        );
        assert!(
            sandbox
                .run(&component("i32.const 0", 1025, false)?, "")
                .is_err()
        );
        let oob = component("i32.const 65536 i32.load drop i32.const 0", 1, false)?;
        assert!(sandbox.run(&oob, "").is_err());
        Ok(())
    }

    fn read_component(handle: &[u8]) -> Result<Vec<u8>> {
        let data = handle
            .iter()
            .map(|b| format!("\\{b:02x}"))
            .collect::<String>();
        Ok(wat::parse_str(format!(
            r#"(component
          (import "vela:task/object-access@1.0.0" (instance $objects
            (export "read" (func (param "handle" (list u8)) (param "maximum" u32)
              (result (result (list u8) (error string)))))))
          (alias export $objects "read" (func $read))
          (core module $mem
            (memory (export "memory") 1)
            (data (i32.const 32) "{data}")
            (func (export "realloc") (param i32 i32 i32 i32) (result i32) i32.const 128))
          (core instance $memory (instantiate $mem))
          (core func $lower (canon lower (func $read) (memory $memory "memory")
            (realloc (func $memory "realloc"))))
          (core instance $host
            (export "memory" (memory $memory "memory"))
            (export "read" (func $lower)))
          (core module $code
            (import "host" "memory" (memory 1))
            (import "host" "read" (func $read (param i32 i32 i32 i32)))
            (func (export "run") (param i32 i32) (result i32)
              i32.const 32 i32.const 32 i32.const 65536 i32.const 0 call $read i32.const 0))
          (core instance $code (instantiate $code (with "host" (instance $host))))
          (func (export "run") (param "input" string) (result (result string (error string)))
            (canon lift (core func $code "run") (memory $memory "memory")
              (realloc (func $memory "realloc")))))"#
        ))?)
    }

    #[test]
    fn real_wasm_reads_only_host_authorized_lfs_object() -> Result<()> {
        let _guard = TEST_LOCK.lock().unwrap();
        use crate::authority::{CapabilityOperation, CapabilityRequest, TaskAuthority};
        use biscuit_auth::{KeyPair, builder::BiscuitBuilder};
        use std::sync::Arc;
        let root = KeyPair::new();
        let token = BiscuitBuilder::new().code(
            "right(\"alice\",\"NetConfig\",\"read\"); right(\"alice\",\"NetConfig\",\"write\");")?
            .build(&root)?.to_vec()?;
        let dir = tempfile::tempdir()?;
        let mut lfs =
            vela_lfs_m1::Lfs::unlock(&dir.path().join("wasm.redb"), [83; 32], root.public())?;
        let object = lfs.put(
            &token,
            "alice",
            "wasm-read",
            &serde_json::json!({
            "schema":"vela.net.config.v1", "ssid":"WasmHostTest", "dns":"9.9.9.9"}),
        )?;
        let exact = lfs.handle(&token, "alice", &object.oid, 120)?;
        let authority = Arc::new(TaskAuthority::new(root.public(), 1024)?);
        let opaque = authority.issue(
            &token,
            CapabilityRequest {
                caller: "alice".into(),
                resource: "NetConfig".into(),
                operations: vec![CapabilityOperation::Read],
                byte_quota: 1024,
                ttl: std::time::Duration::from_secs(120),
            },
            Some(exact),
        )?;
        let lfs = Arc::new(lfs);
        let host = || HostContext {
            principal: "alice".into(),
            authority: authority.clone(),
            lfs: lfs.clone(),
        };
        let sandbox = Sandbox::new()?;
        let guest = read_component(&opaque.to_wire())?;
        let output = sandbox.run_with_host(&guest, "read", host())?;
        assert!(output.contains("WasmHostTest"));
        assert_eq!(
            blake3::hash(output.as_bytes()).to_hex().as_str(),
            object.oid
        );
        assert!(
            sandbox
                .run_with_host(&read_component(&[0; 32])?, "read", host())
                .is_err()
        );
        let foreign = HostContext {
            principal: "bob".into(),
            authority: authority.clone(),
            lfs: lfs.clone(),
        };
        assert!(sandbox.run_with_host(&guest, "read", foreign).is_err());
        assert!(authority.revoke(&opaque)?);
        assert!(sandbox.run_with_host(&guest, "read", host()).is_err());
        assert!(sandbox.run(&guest, "read").is_err());
        Ok(())
    }

    #[test]
    fn memory_limit_is_bounded_to_specification() -> Result<()> {
        let _guard = TEST_LOCK.lock().unwrap();
        assert!(Sandbox::with_memory_limit(MIN_MEMORY_BYTES - 1).is_err());
        assert!(Sandbox::with_memory_limit(MAX_MEMORY_BYTES + 1).is_err());
        Sandbox::with_memory_limit(MIN_MEMORY_BYTES)?;
        Sandbox::with_memory_limit(MAX_MEMORY_BYTES)?;
        Ok(())
    }

    #[test]
    fn concurrent_task_slots_are_strictly_bounded() -> Result<()> {
        let _guard = TEST_LOCK.lock().unwrap();
        let first = TaskPermit::acquire()?;
        let second = TaskPermit::acquire()?;
        assert!(TaskPermit::acquire().is_err());
        drop(first);
        let replacement = TaskPermit::acquire()?;
        drop((second, replacement));
        Ok(())
    }
}
