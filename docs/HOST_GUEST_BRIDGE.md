# Glass host -> Microkit PCB: implementation proposal

Status: proposed, not implemented. Baseline: approval-core / `25e0412`.
The native shared-buffer cryptographic probe passes; Glass currently has no
trusted native signer and fails closed. Signing, transport and policy are three
separate integration tasks. No browser event is automatically trusted input.

## Transport decision

Use a dedicated VirtIO serial port over virtio-mmio on the existing QEMU `virt`
AArch64 machine. QEMU is the outer virtual machine/device model; Windows is
outside Microkit. The seL4 VMM libraries for guests *inside* Microkit are not
required for this topology. WSL is development infrastructure, not a Vela service.

```text
Glass renderer
  -> native approval broker on Windows
  -> byte stream on loopback (WSL forwarding, explicitly qualified)
  -> QEMU chardev socket
  -> virtio-serial-device / virtserialport
  -> Rust no_std ingress PD
  -> bounded page + Microkit notification channel 1
  -> PCB shared verifier + deterministic policy
  -> authorized route to Executor
```

Keep the existing debug UART dedicated to serial logs. A dedicated VirtIO port
avoids mixing binary approvals with logs or QEMU monitor commands. Host-visible
shared RAM is deferred: it increases writable aliases and TOCTOU complexity.

QEMU options to validate with the installed version and the new guest driver:

```text
-chardev socket,id=vela_ingress,host=127.0.0.1,port=7991,server=on,wait=off
-device virtio-serial-device,id=vela_serial
-device virtserialport,bus=vela_serial.0,chardev=vela_ingress,name=vela.approval
```

With Docker under WSL, run QEMU with host networking so its loopback listener is
not exposed on the LAN. Windows-to-WSL localhost forwarding must be tested, not
assumed. If unavailable, use a local authenticated relay with explicit ACLs;
never resolve it by binding an unauthenticated listener on all host interfaces.

The current QEMU device inventory advertises the serial device/port. That is not
a passing guest-driver test. Pin the MMIO/IRQ mapping from the actual QEMU device
tree, fail on device-ID mismatch, and review the Microkit mapping manifest.

## Native host approval broker

Keep the Ed25519 signing key out of HTML, JavaScript and the renderer process.
The native broker owns the pending immutable operation, renders its approval
surface and independently recomputes its digest. Glass sends drafts/proposals.

Basic consent is labelled "approval on this PC". Medium/high-risk operations
require a fresh Windows Hello/FIDO2 assertion whose challenge binds the pending
operation and guest session. Verify that assertion before signing UserSelection.
Windows Hello/FIDO2 credentials do not automatically produce Ed25519 signatures:
the broker verifies their assertion and then uses its separate enrolled Ed25519
key to attest the event. Define ACLs and encrypted custody for that key; do not
claim kernel-compromise resistance or native seL4 input guarantees on Windows.

Use an authenticated named-pipe host API where available. A web-preview adapter
requires restrictive origin/session checks; neither origin nor a browser nonce
alone proves a human gesture. Never reintroduce a JavaScript signing-key hook.

## Wire protocol and bounds

Introduce a closed `vela.host_guest.v1` envelope, maximum 2048 bytes per frame:
magic, version, message kind, bounded payload length, request ID and sequence.
Use explicit little-endian encoding, no native struct casting. Treat TCP as a
stream: accumulate bounded partial frames and parse coalesced frames correctly.
No dynamic queue growth, resynchronization scan or caller-selected IPC endpoint.

Initial message kinds:

- `HELLO` / `CHALLENGE`: authenticate the enrolled host key and establish a fresh
  guest session. A nonce alone does not authenticate the guest; production also
  needs an enrolled guest identity and trusted launch/key provisioning.
- `STAGE_BEGIN` / `STAGE_CHUNK` / `STAGE_END`: transfer an immutable operation
  manifest and exact note bytes into bounded temporary storage. Maximum 4096
  bytes for the initial note workflow; chunks at most 1024 bytes. No persistent
  user mutation, Wasm execution or authorization is implied by staging.
- `APPROVE`: carry the existing 138-byte UserSelection encoding. Its session
  binding/revision must match the guest's pending staged operation, and its
  SHA-256 must match the exact canonical manifest already staged.
- `CANCEL`, `ACK`, `DENIED`, `RESULT`, `RECEIPT`: distinct typed responses.

The manifest must bind operation, exact object scope/version, note digest, caller,
request identity and policy context. A signed arbitrary hash is insufficient.
Content chunks travel on the typed data plane; the model neither forwards nor
alters bytes during authorization. Header IDs must match the signed manifest;
framing metadata is not independent authority.

No permanent user write until fresh policy authorization. Validate payload sizes,
digest, signature, current session, one-shot state, object scope and policy before
dispatch. A cryptographic-only reply is `SELECTION_VERIFIED`, not `AUTHORIZED`.

## Guest ingress and ownership

Add `poc/m5_executor/host-ingress/` as a Rust no_std PD implementing the dedicated
VirtIO serial device: negotiated features, fixed RX/TX queues, bounded descriptor
validation, MMIO/IRQ and ring-buffer parsing. It owns the device mappings; PCB
does not. Limit the first implementation to one session and one pending approval.

Reserve four 4 KiB pages (16 KiB) for transport queues/frames plus one 4 KiB PCB
control page. ELF, stacks and driver metadata are additional and must be counted
in the existing ingress/driver budget. Notify on completed frames, not every byte.

Use EMPTY -> READY -> CONSUMED/ACK ownership. The producer cannot reuse a slot
until a matching acknowledgement; notification is only a wakeup. The PCB copies
the complete bounded selection before verification. Approved bytes must then be
private/immutable or independently revalidated at consumption: a cooperative
ownership flag alone cannot constrain a malicious writer with an RW mapping.

The pinned Rust Microkit API delivers `ChannelSet`: iterate all notified channels
and drain ready descriptors. Correlate replies with request ID, revision and
session. Duplicate notifications must not duplicate authorization or execution.

## Freshness, reboot and result recovery

The test's constant session must be removed before external ingress. Provision a
fresh guest boot/session challenge through a reviewed entropy path (for QEMU,
a bounded VirtIO RNG driver is a candidate dependency, not already implemented).
Do not fall back to timestamps, build-time constants or host-supplied unsigned
nonces when entropy is unavailable. Refuse external approvals instead.

Bind signatures to that session and the pending revision. Use the guest monotonic
clock for short deadlines. Disconnect invalidates pending approvals; guest reboot
invalidates all session-bound approvals. Durable request consumption and the LFS
transaction journal are required before automatic recovery of applied effects.

If the host loses a reply after application, query the transaction by its bound
identity. Never silently re-sign or rerun it. Return success only after execution
and the matching LFS receipt are verified. Host shows disconnected/unknown until
reconciliation, preserving the user's draft.

## Implementation sequence and acceptance

1. Add and test the VirtIO ingress PD with unsigned echo frames only. No authority.
2. Replace the selection-probe fixture sender with externally supplied frames;
   retain the existing 46-marker regression suite and mock authorization sink.
3. Add fresh-session establishment and an explicitly labelled host test signer.
   Run valid, altered-hash/signature, wrong-session, replay, truncation, oversized,
   split/coalesced, duplicate notification and disconnect tests.
4. Add the native approval surface and WebAuthn verification. Browser/model
   attempts cannot request a signature without that approval path.
5. Connect deterministic policy, one-shot consumption, Executor and durable LFS.
   Verify the user's actual note and receipt after a separate QEMU cold boot.

Pass only with real host-generated bytes reaching the guest PCB, no authorization
on negative cases, bounded memory/queues, and no payload mixing with debug UART.
Transport qualification alone does not close trusted-input or policy gates.

Sources: [QEMU character backends](https://www.qemu.org/docs/master/system/qemu-manpage.html),
[QEMU device emulation](https://www.qemu.org/docs/master/system/device-emulation.html),
[Windows WebAuthn](https://learn.microsoft.com/en-us/windows/win32/webauthn/-webauthn-portal).
