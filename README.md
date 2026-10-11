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

The native [M4 session](poc/m4/README.md) now connects authorized saves,
candidate selection and single-use confirmation to persistent recovery. Its
default integration test uses a ranking fixture. A separate offline test now
passes with real M0 embeddings and hybrid search through durable KEEP/reopen;
a consumer UI remains pending. The [seL4 CI](.github/workflows/sel4-boot.yml) builds the pinned
guest and checks bounded Wasm plus the fixed intent flow with a 60-second limit.

## Project status

| Gate | Status | Scope and evidence |
| --- | --- | --- |
| M0 — semantic recall | **CLOSED** | Hybrid-recall gate passed on a synthetic fixture: 186/200 paraphrases (93% recall@1), p95 25.67 ms. This does not establish quality on personal data. |
| M1 — LFS authority | **CLOSED** | Rust storage prototype with encrypted objects, signed commits, and Biscuit-bound capabilities; crash gate recorded over 10,000 process terminations. Not a distributed or production storage service. |
| M2 — IPC and WASM | **CLOSED** | Dual-lane IPC and WASM prototype tests cover typed transport, authorized reads, and adversarial traps. Current host-native peer isolation and durable bulk-file integration are not complete. |
| M3 — intent routing | **CLOSED** | Constrained local intent router, finite templates, deterministic Rust policy, and capability revalidation at cache hits. It remains a prototype and does not authorize operations by model output. |
| M4 — recovery and SLM footprint | **CONDITIONALLY CLOSED** | Persistent provider, confirmation, rollback and live hybrid-recall integration tests pass. Streaming CUDA loading replaces rejected mmap; measured worker peak working set 589 MB, active private commit 4.55 GB. Full-system qualification remains pending. |
| M5 Gate A — reproducible seL4 boot | **CLOSED** | The pinned Microkit 2.3.0 image built and booted the AArch64 seL4 guest in QEMU. |
| M5 Gate B — domain isolation | **IN PROGRESS** | Five statically declared protection domains with separate ELFs, CSpaces, and VSpaces now build and boot. Capability manifests, restricted authority, negative reachability tests, and crash/restart tests remain. |
| M5 Gate C — PCB capability derivation | **IN PROGRESS** | A separately booted custom seL4 root task creates a live child with isolated CSpace/VSpace, mints a badged send capability into its CSpace, receives and verifies the child’s IPC badge, replies, then deletes the grant and suspends the child. Biscuit validation, policy profiles, timed leases, and deny/revocation tests remain. |
| M5 Gate D — bounded Wasm executor | **IN PROGRESS** | The executor PD runs Wasmi 2.0.0 without `std` or WASI. QEMU verifies a 10M fuel budget, fuel-exhaustion trap, 64 KiB memory cap, and a 16 KiB Wasm stack bound; task IPC, epoch deadlines, concurrency, and cancellation remain. |
| M5 Gate E — bus/LFS integration | **IN PROGRESS** | A fixed intent traverses Orchestrator → PCB → Bus → Executor → LFS using Microkit protected calls; the executor returns 42 and the LFS PD records it in volatile memory. Biscuit authorization, dynamic-grant wiring, durable LFS, and adversarial E2E cases remain. |
| M5 Gate F — benchmark preparation | **IN PROGRESS** | QEMU boots and the serial-marker timing harness records host-observed intervals for the fixed flow. These are single-run diagnostic samples, not guest-only latency or qualification against the 250 ms target. |
| M5 Gate G — security review | **IN PROGRESS** | A residual-risk and resource-accounting report is published; this is not an independent security audit or a security qualification. |

“Closed” for M0–M3 refers to the stated component-gate scope; it does not mean
those components have been integrated into a complete seL4 operating system.
Gate B currently proves the static five-domain boot structure only. M4 remains
conditional until its outstanding system and hardware measurements are
completed. See [runtime status and limitations](docs/RUNTIME_STATUS.md).
Gate C's root-task proof boots as a separate seL4 image; it is not part of the
five-domain Microkit image. It demonstrates live capability derivation and an
IPC round-trip, but not Biscuit validation or the complete policy broker.
Gate D's fixed Wasm probe runs in the five-domain Microkit image with no
WASI or host imports; Gate E connects it to a fixed bus/LFS task flow.
Gate E connects those domains for one deterministic demonstration flow, but
its PCB uses a fixed label rule and its LFS write is volatile, not durable.
Gate F's timing method observes serial-marker intervals on the QEMU host
because the pinned kernel traps EL0 reads of `CNTVCT_EL0`; neither those
host-observed values nor the artifact sizes are whole-system RAM/latency
qualification. See [the Gate F/G report](GATE_F_G_REPORT.md) for the run data,
resource estimates, and residual risks.

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
The measured 512-token streaming profile had 121.65 ms p95 and 245.77 ms maximum
in one new round. The rejected mmap profile retained 3.94 GB resident host pages;
streaming reduced that sample to 589 MB, while active private commit stayed
approximately 4.55 GB. Full repeated and integrated qualification remains open.
The invariant-prefix adapter subsequently scored 296/300 on one round with a
129.40 ms maximum: host focus travels as structured context, and native policy
still prevents the model from manufacturing a selection. Its repeated gate is
recorded separately from these earlier trials.
Idle measurements were process-level, not total-OS
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
nightly. The guest prints Wasm verification and the completed fixed intent to QEMU's serial output.
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
| [`poc/m5_sel4/`](poc/m5_sel4/README.md) | Microkit/Rust/QEMU domain, capability, bounded Wasm and fixed-flow proofs. |
| [`GATE_F_G_REPORT.md`](GATE_F_G_REPORT.md) | Gate F timing observations and Gate G resource/residual-risk review. |
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
