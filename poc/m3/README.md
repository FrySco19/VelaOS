# M3 — finite semantic compiler (gate open)

Gemma 4 E2B selects one of five intent labels. Rust expands a pinned NetConfig template and checks the current Biscuit capability. RESTORE always has `Risk::Confirm`. The model cannot supply a caller, capability, method name, handle, confirmation, executable command or object bytes.

The exact-input proposal cache binds request, trusted focus, caller, prompt, registry epoch and policy epoch. It stores neither capabilities nor authorized plans. Every hit performs fresh authorization. At most 4096 bounded proposals are retained for at most 120 seconds.

## Development inference boundary

`LocalCompiler` uses an authenticated loopback-only llama.cpp development endpoint. This is not yet a published Neural Bus inference service. Production bus ingress, protected identity provisioning and a trusted confirmation client remain work to finish. The model is pinned Gemma 4 E2B Q4_0, SHA256 `fa401b55b07ee70a54c6dae3903c783a6e65064312529ea57175cb5f8dec6634`.

`slm-worker` owns the Windows CUDA development backend. It verifies the pinned server executable and GGUF using a 64 KiB streaming buffer, clears inherited llama/CUDA tuning, requests 999 GPU layers without automatic CPU fitting, disables mmap with `--load-mode none`, enables KV-cache offload, and caps the context at 512 tokens. The 512-token limit includes the system prompt; larger requests may be rejected and must not be silently truncated. The existing compiler still permits at most 8192 input bytes, which is a separate bound, not a guarantee that they fit this context. A server executable hash alone does not authenticate its dependent DLLs: the bootstrap directory must remain protected.

The default profile has one slot, batch 512, microbatch 128, no host prompt-cache pool, two context checkpoints, and **unloads the model after five idle seconds**. Layer count alone left 2152.50 MiB of input/per-layer embeddings in CPU buffers in the earlier measured backend. The explicit `.*=CUDA0` override moves those weights too: logs showed a 3494.28 MiB CUDA model buffer and no CPU model buffer. Repacking remains enabled. No Rust tensor copy is introduced.

The 512-token mmap trial retained a sampled peak working set of 3,942,907,904 bytes. The replacement streaming trial peaked at 589,180,928 bytes (about 85% lower resident host memory), with p95 121.65 ms and maximum 245.77 ms on one 300-case round, 295/300 composed routes correct. Active private commit remained 4,545,949,696 bytes; post-idle commit was 842,022,912 bytes and working set 469,155,840 bytes. These are worker-only samples on Windows, not whole-OS RAM proof or a resolved active-commit limit. Both trials, including the rejected mmap profile, are retained in the JSON reports. Twenty-round consistency and full-system qualification are separate requirements.

An inference request automatically reloads a sleeping backend. Ready-model latency and post-idle wake latency must be measured separately; unloading does not preserve the prior warm-latency guarantee. [Memory results](memory-results.json) distinguish active process commit, working set and sleeping process memory. They do not qualify the full VelaOS RAM budget or establish an active-memory cap. This executable is a development adapter; production inference still needs authenticated Neural Bus ingress, protected provisioning and process-crash supervision. `LocalWorker::Drop` terminates its owned child during normal unwinding; an abrupt parent termination is not covered by that destructor.

Bootstrap invocation: `slm-worker PINNED_SERVER PINNED_MODEL PROTECTED_KEY_FILE LOOPBACK_ADDRESS`. The key is read by the backend from the protected bootstrap file, never placed as an argument value or delivered by the planner. The launcher does not provision production identity or weaken the LFS/data-plane boundary.

The [pinned upstream loader](https://github.com/ggml-org/llama.cpp/blob/b11539/src/llama-model-loader.cpp) frees temporary upload buffers after synchronizing uploads. The [pinned server](https://github.com/ggml-org/llama.cpp/blob/b11539/tools/server/server-context.cpp) supports unloading/reloading on idle and reports sleeping state through its development properties endpoint. These mechanisms do not establish a hard active-process committed-memory limit on Windows/WDDM.

## Evaluation protocol

`freeze_gate` authors 300 synthetic cases without calling inference and refuses to overwrite the frozen file. Repeated wording under different trusted focus is intentional. The corpus is compositional and is a development set, not an independent human holdout. Its BLAKE3 is `9de8095512a3de39ed3948de6cab2fed853ef688719837b5f4a5b656d3f83916`.

The first three reports retain failed trials: 194, 276 and 278 raw classifications correct out of 300. The prompt was improved after inspecting these trials; labels and queries were not changed. A later pass must therefore not be presented as a blind generalization result.

The final runner records both raw model accuracy and composed Router accuracy. Rust changes a Save proposal to Clarify when trusted focus is missing. Early reports scored only raw accuracy; later reports explicitly distinguish both metrics. Router accuracy does not imply equivalent model-only accuracy.

Twenty repetitions require 6000 schema-valid outputs, more than 95% composed routing accuracy per round, identical normalized proposals and every ready-model route-cache-miss sample below 250 ms. Reports keep `m3_gate_closed=false`: production service wiring, warm-cache qualification and integrated memory measurements are separate requirements.

`cache_gate` measures 6000 real cache hits with fresh Biscuit checks. No model or object-byte transfer is involved. Build/test commands are development tooling, not user-facing LFS paths.

## Recorded qualification — 2026-10-11

The readable result summaries link losslessly compressed `.json.gz` archives containing every case. SHA256 hashes refer to the complete uncompressed reports; archive round trips were checked before publication. The runners write full JSON when reproduced.

The [20 all-cold rounds](gate-20-rounds.json) retain all 6000 live model outputs. Every output matches the constrained schema; composed routing scores are 295–296/300 per round (98.33–98.67%), while raw model scores are 277–278/300. Route-cache-miss p95 is 140.089 ms and maximum 246.959 ms. The full stress result fails consistency: input 213, an ambiguous request, produces different validated intents across rounds. This is retained as a failure, not hidden by a cache.

The separate [integrated cache protocol](router-cache-results.json) makes 300 live model calls followed by 5700 actual proposal-cache hits with fresh capability checks. It scores 296/300 in all 20 rounds and retains identical hashes of the complete validated plans. Maximum cold latency is 171.351 ms; warm p95 is 0.185 ms and maximum 0.524 ms. That bounded cache protocol passes. It does not establish cold model determinism after cache expiry or epoch changes, and does not close the complete M3 integration gate.

## Architectural decision

Finite manifest templates replace unconstrained model-generated DAGs in this prototype. The host controls the complete method graph and resource scope, while the model performs only semantic selection. Expanding domains requires versioned manifests and new tests. The admitted M1 schema remains only `vela.net.config.v1` with SSID and DNS; VPN and arbitrary files are not implemented.
