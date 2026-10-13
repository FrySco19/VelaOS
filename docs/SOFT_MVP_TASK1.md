# Vela Soft: Task 1 verification

Implemented on `vela/soft-mvp`: bounded `ListRequest` (6) and
`ListResponse` (7), broker-initiated synchronization on WebSocket connection,
native PCB and Executor routing, and Glass timeline hydration.

The enrolled broker signs the read request. The guest authenticates the stored
receipt and signs the response, binding it to the transaction and a fresh nonce.
The broker verifies both signatures before publishing metadata to Glass.
Synchronization failure is reported rather than represented as an empty store.

## Verified

- Protocol tests: 3 passed.
- Durable note integration test: passed, including signed enumeration, empty
  store, request tampering, response tampering, and nonce/transaction mismatch.
- Broker and Glass builds: passed.
- AArch64 Microkit image build: passed.
- Real browser, broker, VirtIO and guest test:
  `node poc/host-broker/tests/hydration.cjs .local/v1`
  returned `authenticated: true`, `page_reload: true`, `note_count: 1`,
  `receipt_unchanged: true`.

Local evidence is in `.local/v1/hydration-result.json` and
`.local/v1/hydrated-timeline.png`. These artifacts are not release assets.

## Scope and remaining work

The current disk format holds one current note (at most 1024 UTF-8 bytes).
This increment does not reconstruct overwritten versions or implement a
multi-document index. Task 2 must add immutable document versions and reads.

Legacy timestamps derive from the signed request identifier originally created
from host milliseconds; they are not a trusted guest RTC timestamp. Historical
receipts do not record human-consent provenance, so Glass reports it as
`Unrecorded`.

The hydration run used the native-only broker without simulated human approval.
It is a read test, not a new authenticated write. Hardware FIDO2 enrollment and
approval remain Task 3; development enrollment roots are still in use.
Two-week daily-driver stability and a production-ready MVP are not established.
