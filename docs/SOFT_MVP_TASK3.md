# Task 3: USB FIDO security gate

The production broker uses Mozilla authenticator 0.5.0 (USB HID transport only)
with crypto_rust and P-256 signature verification. There is no Windows Hello,
console approval or unavailable-device approval fallback. The old direct
`send` transport harness is removed from the executable.

`enroll-fido <credential.json>` registers an ES256 CTAP2 credential and requires
an additional fresh assertion to prove possession before publishing enrollment.
`--ctap1` selects U2F compatibility enrollment. Existing enrollment is never
silently replaced. PIN-required devices fail closed under the touch-only policy.

Authorization verifies the enrolled public key, RP hash, exact assertion
signature, User Presence flag and counter. A fresh random challenge binds the
session, revision, document, parent, exact note bytes and Wasm manifest through
the UserSelection signing bytes. The counter journal is flushed before a permit
is issued. Zero-counter devices still require fresh cryptographic assertions;
no clone-detection guarantee is claimed for such devices.

Ed25519 seed loading and signing occur after this check. The seed remains a
local development file; FIDO gates this process, it does not hardware-encrypt the
seed or protect it from a compromised host. USB-device enrollment and the
signed UP claim are trusted; manufacturer attestation is not validated, and the
broker cannot independently prove a particular capacitive sensor implementation.

The literal no-signed-guest-frame-before-touch policy also gates timeline list
and version reads. Glass explicitly sends ListRequest instead of triggering an
unauthorized automatic guest query. Each list ceremony authorizes its bounded
catalog pages. Read and write ceremonies require their own fresh assertion.
Timeout/refusal preserves the editable draft. Nothing is automatically retried.

## Verified on Windows

- Cargo check, build and three cryptographic negative tests passed.
- Real broker/WebSocket integration with a TCP guest spy rejected unregistered
  list, read and write requests: zero guest connections and zero guest bytes.
- The removed legacy `send` command was rejected.

## Hardware gate remains open

The user has no USB security key yet. Successful registration, actual touch,
CTAP1 hardware compatibility and a positive hardware-to-QEMU write are NOT
verified. Existing native storage and Task 2 positive results remain historical
evidence; they are not FIDO2 evidence.

When a key is available, run the broker's `enroll-fido` command for
`.local/host-bridge/fido.json`, then perform the interactive CRUD browser test.
Production runtime is configured for hardware-fido2 and currently reports
`fidoEnrolled: false`; authorization is denied rather than simulated.
