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

No whole-system RAM qualification was performed. New worker-only 512-token
trials rejected mmap's 3.94 GB peak working set and selected streaming loading,
which peaked at 589 MB resident memory. Active private commit remained 4.55 GB;
post-idle private commit was 842 MB. One streaming round had 295/300 composed
routes correct, 121.65 ms p95 and 245.77 ms maximum. This single ready-model round
does not prove repeated consistency, cold wake latency or the <1.5 GB OS target.
The planner/cache gates now enforce 250 ms rather than the inconsistent 400 ms.

The next consumer-readiness work is protected persistent identity/UI ingress,
durable service integration in the chosen boot platform, error/restart handling
and measured resource budgets. These remain open in the public documentation.
