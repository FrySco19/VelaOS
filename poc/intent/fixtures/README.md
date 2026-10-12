# ciao demo fixtures

`ciao-demo-signals.jsonl` holds three observational envelopes from Francesco's PC
(clock tick, host session, `%USERPROFILE%\.cargo`).

The Intent path that reads them is **FALLBACK CONTEXT-SIGNALS**.
BM25 on the utterance «ciao» has no lexical hit, and no LLM is called.
Rows are proposals only: they do not grant, admit, or select.

Re-render `poc/intent/demo-out/` (proposal JSON plus the PNG pair):

```text
cargo run --manifest-path poc/intent/Cargo.toml --bin ciao-fallback -- poc/intent/fixtures/ciao-demo-signals.jsonl
```
