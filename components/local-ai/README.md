# Included local inference

Gemma 4 E2B QAT Q4_0 and llama.cpp are assembled from pinned upstream releases.
They are distributed with the ISO, not stored as large Git blobs. The build must
fail when the artifacts are missing or their hashes differ. See assets.json.

The user service listens only on 127.0.0.1:11435, supports the existing compatible
chat protocol and unloads the model after five minutes idle. CPU inference is
the reference configuration; GPU/mobile performance is not implied by it.
The service has no download or cloud fallback. Text tool use is the initial scope.
An automatically generated per-session credential is kept in the user's runtime
directory with mode 0600; applications obtain it through the shared configuration.
