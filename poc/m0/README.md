# VelaOS — M0 hybrid recall spike

This isolated experiment starts the LFS + Neural Bus architecture pivot. The existing desktop implementation is legacy work, not the implementation of this architecture. Python is used only for the explicitly permitted M0 retrieval spike; new low-level LFS and bus components will be Rust.

## Run

Install Python 3.12 and uv. From this experiment's checkout, run:

```powershell
uv run --locked --script hybrid_recall.py --download-model
```

The first run downloads the pinned embedding model. Subsequent model inference and retrieval run locally:

```powershell
$env:HF_HUB_OFFLINE = '1'
uv run --locked --script hybrid_recall.py
```

uv may need network access to install dependencies initially. The script refuses missing or modified model files rather than replacing the model. `--output` selects the JSON experiment report; `--repeat` changes the number of warm timing samples. The cache and temporary database are physical experiment artifacts, not an LFS object namespace.

## What is implemented

Five authored dummy semantic cards are embedded with multilingual-e5-small, indexed in LanceDB, and queried through cosine similarity and native BM25. LanceDB's native reciprocal rank fusion combines the two rankings with K=60. Identifiers remain whole whitespace tokens. English stopwords are removed; stemming is disabled.

The script checks native fusion against independently calculated RRF scores, canonical content OIDs, model epoch filtering, and typed prefiltering in both branches. Five vectors use an exact scan; this is not an ANN scalability benchmark. Labels describe expected test results and do not participate in retrieval.

## Observed result

The saved `results.json` records six successful cases: exact SSID, IP address, UUID, an Italian semantic paraphrase, a mixed exact/semantic query, and an Italian media query.

For `ZEPHYR-731 connessione protetta del portatile in caffetteria`:

| Candidate | Cosine similarity | BM25 | Hybrid RRF |
| --- | ---: | ---: | ---: |
| Espresso wireless session | 0.832129 (rank 1) | No hit | 0.016393 (rank 2) |
| Recorded configuration, exact SSID | 0.792807 (rank 2) | 1.454061 (rank 1) | 0.032522 (rank 1) |

The lexical branch rescues the exact identifier even though the embedding prefers the semantic match. This is an observed result for this fixture, not a guarantee that RRF always prioritizes exact terms.

On this Windows host, CPU execution with four threads measured a warm p50 of 39.78 ms and p95 of 47.12 ms over 24 repeated samples. Each sample includes embedding plus three searches for diagnostics. An earlier successful run measured p95 58.49 ms; a subsequent run with explicit assertions for the semantic-first/lexical-recovery contrast passed all six cases and measured p95 35.74 ms (`verification.json`). These small measurements fluctuate and do not establish a production latency guarantee. Model setup and downloading are excluded.

An initial tokenizer configuration also exposed a false lexical match: Italian `in` matched English `in` in the media card. Removing English stopwords fixed these fixtures. Multilingual tokenization, identifier edge cases, and a larger independent evaluation remain necessary.

## Architectural boundaries

1. **Content identity:** OIDs hash RFC 8785 canonical plaintext using BLAKE3. Embeddings and authored card metadata do not define identity.
2. **Deterministic authority:** this spike has no planner or execution authority. Labels never authorize actions.
3. **Two planes:** retrieval returns indexed metadata only. No LLM moves bytes; no shared-memory bus is simulated or claimed.
4. **Exact handles:** results are ranked candidates with `auto_resolve=false`. The metadata OID lookup is not an expiring handle or `lfs.open` implementation.
5. **Reversible changes:** no system configuration is modified. The signed commit log and rollback behavior are deferred to the specified milestones.
6. **Model epochs:** each indexed row carries a pinned model epoch; queries prefilter that epoch. All five model artifacts are checked against fixed SHA-256 digests.

Typed filtering here is not Biscuit capability enforcement. This spike does not implement redb, encryption, Wasmtime, iceoryx2, handles, a commit log, or an LFS service. Dummy card text is deterministically rendered from authored fixture metadata, not a production schema extractor.

## M0 gate closed on the full evaluation

The specification requires recall@1 >0.9 on 200 paraphrases and p95 below 50 ms. The original six-query report retains `m0_roadmap_gate_passed=false`: it did not establish the gate. The subsequent frozen 200-paraphrase evaluation passed with 186/200 (93%) and warm p95 25.67 ms over 600 requests. See [official closure, limitations and reproduction](GATE.md) and [full results](gate-results.json). These are synthetic paraphrases of the current five cards, not independent human evaluation. The [Neural Bus bridge](../m2/README.md) now connects ranked M0 OIDs to exact M1 handles and real typed IPC.

## Pinned sources

- [Embedding model and usage at the pinned revision](https://huggingface.co/intfloat/multilingual-e5-small/blob/614241f622f53c4eeff9890bdc4f31cfecc418b3/README.md): multilingual model, 384 dimensions, query/passage prefixes, mean pooling and normalized vectors; MIT license.
- [LanceDB full-text search documentation](https://github.com/lancedb/docs/blob/main/docs/search/full-text-search.mdx).
- [LanceDB native RRF implementation](https://github.com/lancedb/lancedb/blob/main/python/python/lancedb/rerankers/rrf.py).

Runtime packages are pinned in the script and its uv lockfile. The implementation uses LanceDB 0.40.0's native BM25 fallback allowed by the specification, not a separate Tantivy service. No model weights are included in the source checkout.

