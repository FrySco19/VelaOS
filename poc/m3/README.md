# M3 — finite semantic compiler (gate open)

Gemma 4 E2B selects one of five intent labels. Rust expands a pinned NetConfig template and checks the current Biscuit capability. RESTORE always has `Risk::Confirm`. The model cannot supply a caller, capability, method name, handle, confirmation, executable command or object bytes.

The exact-input proposal cache binds request, trusted focus, caller, prompt, registry epoch and policy epoch. It stores neither capabilities nor authorized plans. Every hit performs fresh authorization. At most 4096 bounded proposals are retained for at most 120 seconds.

## Development inference boundary

`LocalCompiler` uses an authenticated loopback-only llama.cpp development endpoint. This is not yet a published Neural Bus inference service. Production bus ingress, protected identity provisioning and a trusted confirmation client remain work to finish. The model is pinned Gemma 4 E2B Q4_0, SHA256 `fa401b55b07ee70a54c6dae3903c783a6e65064312529ea57175cb5f8dec6634`.

The Windows loader streams weights into GPU memory without retaining a resident file mapping. The preflight report separates the first inference/JIT cost from ready-model latency, working set and private committed memory. It does not establish the RAM use of an entire VelaOS system.

## Evaluation protocol

`freeze_gate` authors 300 synthetic cases without calling inference and refuses to overwrite the frozen file. Repeated wording under different trusted focus is intentional. The corpus is compositional and is a development set, not an independent human holdout. Its BLAKE3 is `9de8095512a3de39ed3948de6cab2fed853ef688719837b5f4a5b656d3f83916`.

The first three reports retain failed trials: 194, 276 and 278 raw classifications correct out of 300. The prompt was improved after inspecting these trials; labels and queries were not changed. A later pass must therefore not be presented as a blind generalization result.

The final runner records both raw model accuracy and composed Router accuracy. Rust changes a Save proposal to Clarify when trusted focus is missing. Early reports scored only raw accuracy; later reports explicitly distinguish both metrics. Router accuracy does not imply equivalent model-only accuracy.

Twenty repetitions require 6000 schema-valid outputs, more than 95% composed routing accuracy per round, identical normalized proposals and every ready-model route-cache-miss sample below 400 ms. Reports keep `m3_gate_closed=false`: production service wiring, warm-cache qualification and integrated memory measurements are separate requirements.

`cache_gate` measures 6000 real cache hits with fresh Biscuit checks. No model or object-byte transfer is involved. Build/test commands are development tooling, not user-facing LFS paths.

## Architectural decision

Finite manifest templates replace unconstrained model-generated DAGs in this prototype. The host controls the complete method graph and resource scope, while the model performs only semantic selection. Expanding domains requires versioned manifests and new tests. The admitted M1 schema remains only `vela.net.config.v1` with SSID and DNS; VPN and arbitrary files are not implemented.
