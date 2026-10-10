# Recorded M1 storage validation — 2026-10-10

**The specified 10,000-process-kill storage gate passed.** This closes the tested storage/crash-consistency gate, not the complete OS or an externally exposed LFS service. The M0 evaluation was still open at this storage validation; it subsequently [passed the 200-query gate](../m0/GATE.md).

## Functional and code checks

Six Rust test groups passed: canonical identity/dedup/replay/pagination; capabilities/attenuation/expiry/handle tampering; encrypted blob corruption; signed-log tampering; password derivation/schema rejection; multi-chunk encryption/dedup/associated-data authentication. Capability tests include forged and oversized tokens and a rule-expansion token exhausting the configured Datalog fact budget.

Formatting and Clippy with warnings denied passed. [GitHub Rust checks on fbe952c](https://github.com/FrySco19/VelaOS/actions/runs/38059830277) passed, including an independent 200-kill regression smoke test. [Repository source checks](https://github.com/FrySco19/VelaOS/actions/runs/38059830203) also passed. These links identify the checked code revision; later report-only commits do not change the kernel.

## Full local process-kill run

`crash-report.json` is the actual machine-readable result. Windows AMD64, Rust/Cargo 1.96.1, optimized build, eight independent harness threads, 1,250 trials each, distinct requests accumulated in groups of 32. Duration: **382.3448178 seconds**.

| Kill phase | Trials |
| --- | ---: |
| Before blob staging | 2,015 |
| Blob durable, no committed log operation | 1,982 |
| Open log transaction / race against its commit | 2,038 |
| Log durable, index not applied | 2,061 |
| Index applied, before returning from the put hook | 1,904 |
| **Total** | **10,000** |

Of the 2,038 transaction/race trials, 1,036 recovered an already durable operation and 1,002 recovered without that operation; retry completed it exactly once. All trials verified independently expected commit counts, the original acknowledged baseline, idempotent retry replies, object counts and handle reads of every committed object in the current group.

Observed lost commits: **0**. Duplicate commits: **0**. Invalid reads: **0**.

Test executable SHA-256: `41e91b9df2712f3c90002f0c0ffcc278329b70b2e72a54733c02f3657404c893`. Synthetic secrets and data only; production keys are not embedded. The executable is a local verification artifact, not an OS release.

## Limits

An earlier full attempt failed because Biscuit's implicit 1 ms execution budget expired under concurrent load. It was not counted as a successful gate. Explicit finite budgets and fail-closed adversarial-token checks were then validated before the successful repetition.

This is process termination on the current host, not physical power-loss certification. Corpus sizes and the sole NetConfig schema are limited. Semantic LanceDB recall, the service envelope/transport, iceoryx2 channels, Wasmtime hosting, production key management and an external anti-rollback head anchor remain unimplemented. The Rust kernel's internal read buffer is not zero-copy IPC. See README and DECISIONS for the precise architectural boundaries.

