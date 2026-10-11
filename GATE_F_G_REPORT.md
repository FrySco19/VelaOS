# Gate F/G — Timing Preparation and Residual Risk Report

**Status:** Gate F and Gate G remain in progress.

**Platform:** seL4/Microkit 2.3.0, `qemu_virt_aarch64`, QEMU AArch64, pinned
Rust toolchain.

**Scope:** One fixed Gate E demonstration flow in the five-PD image.

## Executive summary

The five-domain image builds and boots. Its fixed Orchestrator → PCB → Bus →
Executor → LFS flow still completes and returns Wasm result `42`. A host-side
monotonic-clock observer now records serial-marker intervals around each
protected call and the Wasm invocation.

One diagnostic run reported a 9.933 ms end-to-end serial-marker interval.
This is **not** a guest-only latency measurement, a percentile, or qualification
against the `<250 ms` target. The interval includes debug output, QEMU
emulation, serial transport, and host scheduling. The run has no repeated
samples, workload variance, or target-hardware measurement.

No total-system RAM measurement was performed. Artifact sizes, statically
declared stack/heap bounds, and a loader-reported root-server physical-memory
range are recorded below as distinct quantities; none establishes the complete
OS idle or active footprint.

## Gate F: observed timing

The first attempt read `CNTVCT_EL0`/`CNTFRQ_EL0` directly from an application
protection domain. QEMU faulted the Orchestrator because the pinned kernel does
not permit those EL0 system-register reads. That implementation was removed.
The present method timestamps arrival of the guest's `BEGIN` and `END` debug
lines using the QEMU host's monotonic clock.

Run command (from Windows PowerShell):

```powershell
wsl.exe -e bash -lc "cd /mnt/c/Users/tella/Documents/Codex/2026-10-09/in-c-users-tella-documents-codex/outputs/VelaOS-runtime && docker build -f poc/m5_sel4/Dockerfile -t velaos-gate-f poc/m5_sel4"
wsl.exe -e bash -lc "docker run --rm --entrypoint timeout velaos-gate-f 12s make benchmark"
```

The Docker build exited successfully after compiling the five Microkit PDs and
Gate C root-task proof. QEMU emitted every expected Gate D/E success marker and
all five complete timing-marker pairs before the 12-second timeout terminated
the still-running guest. The timeout exit status is expected for this smoke
run; it is not the application's test result.

| Interval | Single observed sample |
| --- | ---: |
| Wasm invocation (module setup, bounded probes, and sample call) | 1.236 ms |
| Bus → Executor protected call | 3.826 ms |
| Bus → LFS protected call | 2.077 ms |
| PCB → Bus protected call | 7.669 ms |
| Orchestrator end-to-end protected call | 9.933 ms |

These intervals are nested and must not be added together. This diagnostic
does not measure cold boot, a production task, queueing under concurrency,
unloaded-model wake-up, or real hardware. Each includes marker printing and
host/QEMU observation overhead. Therefore it neither proves nor disproves the
target-hardware `<250 ms` requirement.

### Gate F completion still required

- Use a permitted, calibrated clock source with guest-side or hardware-side
  timestamps and quantify measurement overhead.
- Run enough repeated and varied trials to report p50/p95/p99/max, including
  load, contention, cold-start, and failure cases.
- Define the exact intent-to-execution interval and separate model wake-up
  from routing before target qualification.
- Preserve the QEMU smoke test, but do not use it as a substitute for the
  physical-hardware and system-level M4 latency evidence.

## Memory and artifact accounting

### Observed and declared quantities

| Quantity | Value | Meaning and limitation |
| --- | ---: | --- |
| QEMU guest RAM configuration | 2 GiB | Capacity assigned to the emulated guest, **not** measured consumption. |
| Microkit loader image | 6,339,412 bytes (about 6.05 MiB) | File size on disk; not equal to resident or private RAM. |
| Executor ELF | 2,158,472 bytes | File size on disk; includes code/data image, not a runtime working-set measurement. |
| Other four PD ELFs | 61,568–63,000 bytes each | File sizes on disk only. |
| Five declared PD stacks | 320 KiB total | Five `0x10_000` stack declarations; not the complete memory requirement. |
| Executor heap | 2 MiB | Explicit Microkit heap bound for the executor PD. |
| Wasm linear memory limit | 64 KiB | Store-level guest memory cap exercised by the QEMU probe. |
| Loader root-server physical range | `0x60258000–0x60621000` = 3,969,024 bytes (3.785 MiB) | One boot-time range printed by the loader; not the sum of kernel, all PDs, device buffers, allocator slack, and runtime state. |

The ELF and loader file sizes are not RAM usage. The root-server range is a
loader diagnostic for one reserved physical range, not a whole-system memory
accounting result. The 2 GiB QEMU setting is a capacity ceiling. These values
must not be presented as evidence that the `<1.5 GB total OS idle RAM` target
has passed.

### Whole-system memory evidence still required

- Account for kernel image/data, kernel objects, all PD code/data/stack/heap,
  page tables, IPC buffers, shared frames, and boot-time allocator slack.
- Include drivers, LFS backing/cache, logging, queues, and all dynamic/runtime
  allocations in active and idle states.
- Capture a reproducible whole-system high-water/idle measurement on the
  actual target configuration and report the method, units, and sample window.
- Repeat M4's GPU/SLM memory qualification separately; QEMU does not validate
  CUDA buffers, VRAM offload, or host private-commit accounting.

## Gate G: residual security review

This is a bounded engineering review of the current proof, not an independent
security audit or formal verification.

| Area | Current evidence | Residual risk / missing proof |
| --- | --- | --- |
| Protection domains | Five statically declared Microkit PDs build with separate ELFs, CSpaces, and VSpaces. | Negative tests for unintended capability reachability, fault containment/restart, and adversarial resource exhaustion are absent. |
| Authorization | PCB accepts one fixed message label on its statically configured endpoint. | No Biscuit parsing/signature verification, claims/policy evaluation, grant profiles, timed leases, deny cases, or revocation/use-after-revoke integration in Gate E. |
| Dynamic capability proof | A separate root-task experiment creates a child CSpace/VSpace, passes a badged endpoint cap, verifies its badge, then deletes the grant. | It is not integrated with the five-PD flow; it uses a fixed badge and lacks token, lease, deny, and adversarial lifecycle tests. |
| Bus and IPC | One fixed protected-call path returns the expected result. | No untrusted ingress, schema fuzzing, payload/queue stress limits, replay handling, concurrency, or denial-of-service evaluation. |
| Wasm | Wasmi runs without WASI or host imports; QEMU probes fuel exhaustion and a 64 KiB memory cap. | No untrusted component admission/signature path, epoch deadline, cancellation, concurrent stores, or comprehensive malformed-module/fault corpus. |
| LFS | LFS handler validates and records result `42` in volatile state. | No durable storage, authenticated object selection, crash consistency, key lifecycle, or rollback integration in this flow. |
| Timing/debug | Host observer measures serial marker arrival; QEMU confirms direct EL0 counter reads trap. | Serial debug is observable and perturbs timing. No guest-calibrated production clock or independent timing channel has been qualified. |
| Platform | One AArch64 QEMU virtual platform and one emulated CPU boot successfully. | No secure boot, physical hardware, DMA/device isolation, GPU, multiprocessor, side-channel, or supply-chain qualification. |

## Gate disposition

Gates A–E retain their previously documented scope and limitations. Gate F has
working measurement **preparation** and one successful QEMU observation, but is
not closed. Gate G has this residual-risk/resource-accounting report, but it
does not replace a formal security review. Neither Gate F nor Gate G is closed,
and neither the `<250 ms` latency target nor `<1.5 GB` total idle-RAM target is
qualified by this work.
