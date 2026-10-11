# VelaOS

**VelaOS is an AI-native, local-first operating-system architecture built around
intent rather than a collection of resident applications.** A language model
helps translate a user's request into a constrained plan; deterministic Rust
services, explicit policy, and kernel-enforced capabilities decide what may
actually happen.

The project is transitioning its Rust/Linux proof of concept toward seL4
protection domains. It is not yet a complete, installable, or production-ready
operating system. Gate results describe bounded prototypes and tests, not
product-wide guarantees.

[![M1 Rust checks](https://github.com/FrySco19/VelaOS/actions/workflows/lfs-m1.yml/badge.svg)](https://github.com/FrySco19/VelaOS/actions/workflows/lfs-m1.yml)
[![M2 Rust checks](https://github.com/FrySco19/VelaOS/actions/workflows/neural-bus-m2.yml/badge.svg)](https://github.com/FrySco19/VelaOS/actions/workflows/neural-bus-m2.yml)

## Vision: the zero-app paradigm

Traditional desktop systems organize work around applications that users open,
configure, and leave running. VelaOS instead aims to organize work around user
intent:

1. **The LLM Control Plane** interprets a request and proposes a finite,
   structured workflow. Model output is untrusted input: it cannot grant
   permission, choose arbitrary storage objects, or invoke privileged services
   directly.
2. **The deterministic Rust Data Plane** validates the proposed workflow,
   resolves exact authorized object handles, checks policy and capabilities,
   and performs the operation through typed service interfaces.
3. **Ephemeral interfaces and workflows** are assembled for the task, present
   only the necessary controls, and can disappear when the task completes.
   Persistent system services remain, but the goal is to avoid requiring a
   resident app for every kind of work.

This is an architectural direction, not a claim that today's prototypes have
eliminated desktop applications. The model proposes; trusted policy and
services decide and execute.

## Architecture: seL4, capabilities, and WebAssembly

The target architecture uses **seL4 protection domains** to isolate services
such as the Local File Store (LFS), Neural Bus, WASM executor, Orchestrator, and
Policy and Capability Broker (PCB). Each domain receives only the kernel
capabilities it needs; typed IPC endpoints define how domains communicate.

The PCB verifies offline **Biscuit authorization tokens** against application
policy. A Biscuit token is not itself a seL4 capability: after successful
verification, the PCB may derive narrowly scoped, short-lived native
capabilities for a specific operation. seL4 enforces access based on possession
of those capabilities, while services still validate request semantics.

Untrusted WASM components are intended to run in a bounded Wasmtime executor
**without WASI** and without ambient file, network, process, or device access.
The host exposes only explicitly linked interfaces. Per-invocation fuel,
epoch-based interruption, memory limits, and bounded IPC constrain execution.

The architecture and threat boundaries are described in the [M5 design](docs/M5_DESIGN.md).
Gate A/B's reproducible QEMU protection-domain scaffold and exact commands are in
[`poc/m5_sel4/`](poc/m5_sel4/README.md).

## Project status

| Gate | Status | Scope and evidence |
| --- | --- | --- |
| M0 — semantic recall | **CLOSED** | Hybrid-recall gate passed on a synthetic fixture: 186/200 paraphrases (93% recall@1), p95 25.67 ms. This does not establish quality on personal data. |
| M1 — LFS authority | **CLOSED** | Rust storage prototype with encrypted objects, signed commits, and Biscuit-bound capabilities; crash gate recorded over 10,000 process terminations. Not a distributed or production storage service. |
| M2 — IPC and WASM | **CLOSED** | Dual-lane IPC and WASM prototype tests cover typed transport, authorized reads, and adversarial traps. Current host-native peer isolation and durable bulk-file integration are not complete. |
| M3 — intent routing | **CLOSED** | Constrained local intent router, finite templates, deterministic Rust policy, and capability revalidation at cache hits. It remains a prototype and does not authorize operations by model output. |
| M4 — recovery and SLM footprint | **CONDITIONALLY CLOSED** | Persistent-provider, confirmation, and rollback implementation is present. mmap, 512-token context, and KV-offload configuration is in place; physical hardware/CI memory and latency qualification remains pending. |
| M5 Gate A — reproducible seL4 boot | **CLOSED** | The pinned Microkit 2.3.0 image built and booted the AArch64 seL4 guest in QEMU. |
| M5 Gate B — domain isolation | **IN PROGRESS** | Five statically declared protection domains with separate ELFs, CSpaces, and VSpaces now build and boot. Capability manifests, restricted authority, negative reachability tests, and crash/restart tests remain. |
| M5 Gate C — PCB capability derivation | **NEXT** | Define and test the bounded capability-derivation mechanism; Biscuit verification is application policy, not a kernel capability. |

“Closed” for M0–M3 refers to the stated component-gate scope; it does not mean
those components have been integrated into a complete seL4 operating system.
Gate B currently proves the static five-domain boot structure only. M4 remains
conditional until its outstanding system and hardware measurements are
completed. See [runtime status and limitations](docs/RUNTIME_STATUS.md).

## Non-negotiable engineering targets

These are acceptance targets, **not claims that the current system has already
met them**:

| Metric | Target | Qualification |
| --- | --- | --- |
| Total OS idle RAM | **< 1.5 GB** | Measure the complete runtime at idle; a model process's working set or unloaded footprint is not a system-wide measurement. |
| Intent-to-execution routing latency | **< 250 ms** | Measure end-to-end routing under the pinned workload; report percentiles and maximum separately from cold model wake-up. |

The latest recorded 300-request SLM run had a 155.918 ms p95 and a 307.335 ms
maximum with the model already loaded, so that run did **not** meet the maximum
latency target. Cold wake-up was measured separately at approximately 2.945 s.
The newer mmap/512-token/KV-offload profile still needs hardware/CI
qualification. Earlier idle measurements were process-level, not total-OS
measurements, so the <1.5 GB target is also not yet certified. Detailed
measurements and caveats are recorded in [RUNTIME_STATUS.md](docs/RUNTIME_STATUS.md).

## VelaOS and conventional desktop operating systems

| Axis | VelaOS (target architecture) | Windows | macOS |
| --- | --- | --- | --- |
| Kernel architecture | seL4 microkernel; Gate A boot demonstrated in QEMU, service isolation is next | Windows NT hybrid kernel | XNU hybrid kernel |
| UI paradigm | Intent-centric direction; task-scoped, ephemeral interfaces | Primarily app-centric desktop | Primarily app-centric desktop |
| Permission model | PCB verifies Biscuit authority and derives scoped, ephemeral seL4 capabilities | User Access Control (UAC), access tokens, ACLs, and application sandboxing | Transparency, Consent, and Control (TCC), entitlements, and app sandboxing |
| RAM footprint | **Target:** <1.5 GB total OS idle RAM; not yet measured/qualified for the complete system | Varies by release, device, configuration, and workload; no directly comparable VelaOS baseline | Varies by release, device, configuration, and workload; no directly comparable VelaOS baseline |

This is an architectural comparison, not a claim of feature or performance
parity. The VelaOS column describes the target and current gate evidence, not a
finished product.

## Gate A: reproduce the QEMU boot

From the repository root, with Docker available:

```powershell
docker build -f poc/m5_sel4/Dockerfile -t velaos-gate-a poc/m5_sel4
docker run --rm -it velaos-gate-a
```

The image pins seL4 Microkit 2.3.0, `rust-sel4` v5.0.0, and its matching Rust
nightly. The guest prints the Gate A success message to QEMU's serial output.
Use `Ctrl+A`, then `X` to exit QEMU. This smoke test does not validate Gate B
isolation or physical hardware behavior. See the [Gate A README](poc/m5_sel4/README.md)
for toolchain details and scope.

## Repository map

| Path | Contents |
| --- | --- |
| [`poc/m0/`](poc/m0/GATE.md) | Semantic recall experiment and fixture results. |
| [`poc/m1/`](poc/m1/README.md) | Rust LFS storage and capability-bound operations. |
| [`poc/m2/`](poc/m2/README.md) | Neural Bus, typed IPC, and WASM boundary prototypes. |
| [`poc/m3/`](poc/m3/README.md) | Local intent routing, deterministic policy, and evaluation corpus. |
| [`poc/m4/`](poc/m4/README.md) | Recovery coordinator, persistent provider, and rollback supervisor. |
| [`poc/m5_sel4/`](poc/m5_sel4/README.md) | Gate A boot and initial Gate B Microkit/Rust/QEMU protection-domain scaffold. |
| [`docs/M5_DESIGN.md`](docs/M5_DESIGN.md) | Proposed seL4 protection-domain architecture and M5 acceptance gates. |
| [`docs/RUNTIME_STATUS.md`](docs/RUNTIME_STATUS.md) | Runtime evidence, measurements, and outstanding limitations. |
| [`archive/`](archive/README.md) | Historical web-shell prototype. |

`components/` and `build/` contain the previous Debian/KDE preview and do not
implement the new AI-native architecture.

## Contributing

Reproducible tests, security reviews, and clear reporting of limitations are
welcome. See [CONTRIBUTING.md](CONTRIBUTING.md), [SECURITY.md](SECURITY.md),
and [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md). License information is available
in [LICENSE](LICENSE), [`LICENSES/`](LICENSES/README.md), and
[THIRD_PARTY.md](THIRD_PARTY.md).
