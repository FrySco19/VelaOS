# Native session and QEMU validation — 11 October 2026

This is a tested development prototype, not a finished consumer OS.

## Native Rust workflow

On the Windows development host, all 12 M4 tests passed together (8.91 seconds),
including the normally asset-gated live M0 test. Formatting and Clippy with all
warnings denied passed. The real offline test used the pinned multilingual E5
model, actual LanceDB vector/BM25/RRF ranking and capability-filtered M1 cards.
An exact-keyword saved configuration ranked first. Trusted test events selected
the exact OID, confirmed application, kept it, and reopened durable databases.
The provider transferred bytes through its existing M2 iceoryx2 adapter.

This proves native API integration on a fixture. It does not prove a human UI,
protected cross-process client ingress, personal-data retrieval quality, a real
network device, or the full prompt-injection/process-kill qualification.

## seL4 boot

A fresh Docker build of both the five-domain Microkit image and separate PCB
root-task proof completed. The new 60-second smoke harness passed on the rebuilt
guest, requiring Wasm fuel/memory verification and the completed fixed result-42
flow. An invalid loader failed closed with exit status 1. QEMU was terminated
and reaped by the harness rather than leaving an indefinite guest process.

The CI exports the verified QEMU loader plus its SHA-256 as a downloadable
artifact. It is an AArch64 QEMU prototype image, not a desktop ISO or installer.
The guest still has volatile LFS state and lacks integrated Biscuit, persistent
recovery, SLM, GPU and consumer-device support. Root-task grants remain a separate
proof. Boot checks do not close M5 gates B–G or constitute a security audit.

## Memory and release limits

No new whole-system RAM or SLM latency qualification was performed in this
validation. The current mmap/512-token profile remains unqualified. Previous
worker-only measurements and their failed latency/active-commit criteria remain
in the runtime report; they do not establish the <1.5 GB OS idle target.

The next consumer-readiness work is protected persistent identity/UI ingress,
durable service integration in the chosen boot platform, error/restart handling
and measured resource budgets. These remain open in the public documentation.
