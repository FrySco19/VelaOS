# Phase 2/3 verification — 2026-10-12

Baseline: 278a0df (Phase 1). Actual manifest: app/Cargo.toml.

A fresh AArch64 Docker build completed for both no_std domains. Both ELF W^X checks passed (three PT_LOAD segments each). Microkit generated the two-domain loader; QEMU booted it and completed every IPC assertion. Docker image: af16b378fce102bddc25ace12c5289a72144f16adba3d14fc42a67920201c6aa.

Host verification: one Rust test passed, covering signature and payload/header tampering, wrong key, replay, signed quota rejection, invalid offsets and snapshot isolation. No host tests were substituted for guest execution.

| Guest case | Reply status | Execution attempts |
| --- | ---: | ---: |
| Signed return 7, request 1 | 0; result 7 | 1 |
| Payload changed to return 8 | 2 | 1 |
| Corrupted signature | 2 | 1 |
| Invalid payload offset | 1 | 1 |
| Signed excessive fuel quota | 3 | 1 |
| Replay of request 1 | 4 | 1 |
| Signed infinite loop, fuel 64 | 6 | 2 |
| Signed WASI import | 5 | 2 |
| Signed return 7, request 8 | 0; result 7 | 3 |

The counter proves rejected authentication/scope/replay frames did not reach instantiation. The WASI module was rejected before instantiation. Request 8 confirms continued service after failures. Original engine self-tests also verified fuel exhaustion and the 64 KiB memory-growth bound.

The harness required the exact completion marker, rejected panic/FAULT output, and accepted timeout exit 124 only after all assertions passed. The timeout stops an idle Microkit guest; it is not the success criterion.

See [serial output](phase-2-3-boot.log). Production limitations, including boot-local replay and the test-only signing key embedded in PCB ELF, are recorded in [README](README.md). No physical-hardware or total-OS memory measurement was performed.

## Reproducibility follow-up — current source

Repeated build and QEMU after adding PCB request/ack correlation assertions. The Rust verify-bridge runner required each reply marker exactly once, checked every rejection and the final result, rejected guest faults, and accepted exit 124 only after completion. Result: PASS. Host descriptor/crypto tests: 2 passed, 0 failed. This verifies the single-flight protected-call lifecycle; no notification queue is claimed. See BRIDGE_CONTRACT.md for the authority and LFS gates still open.
