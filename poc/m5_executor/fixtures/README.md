# Executor contract fixtures

Wasm fixtures for `poc/m5_executor/executor-host` contract tests (see `../EXECUTOR_CONTRACT.md`).
They are **host-test inputs only**: the guest PCB sequence (`app/src/bin/pcb.rs`) and the
`verify-bridge` markers do not use them.

- `*.wat` are the sources; each `*.wasm` next to it is the committed binary.
- `max_payload.wasm` (exactly 3980 bytes = `PAGE_SIZE - PAYLOAD_OFFSET`) and `oversized.wasm`
  (3981 bytes, cannot fit in a v2 frame) are generated: the return-7 module plus a custom
  section named `pad`.
- Regenerate: `cargo run --example gen_fixtures --manifest-path poc/m5_executor/executor-host/Cargo.toml`
- The test `fixtures_match_sources` fails if a committed `.wasm` drifts from its source.
- `max_fuel_loop`, `max_fuel_call_loop`, `max_fuel_recursion_churn`: hostile R1 inputs run at
  `MAX_FUEL` by `executor-host/tests/r1_max_fuel.rs`; each must end in status 6 without a native
  stack overflow. (The guest's QEMU MAX_FUEL case uses `bridge::LOOP_MODULE`.)
