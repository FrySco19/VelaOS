# M0 gate closed — 2026-10-10

**Passed:** recall@1 = **186/200 = 0.93**, strictly above 0.90; warm request p95 = **25.668985 ms**, below 50 ms. This closes the M0 retrieval spike gate on the explicitly defined five-card synthetic corpus. It is not evidence of production retrieval quality on arbitrary personal data.

The 200 unique paraphrases were authored from the existing five cards and the specification's recall use cases, 40 per card, in Italian and English. They were frozen before the first run. Corpus SHA-256: `e5ab8d1fdb1e4182778314524f7918cf724d0aab9dcf730afa3ff43c5ad244b0`. No queries, labels, model, card text, tokenizer or ranking parameters were tuned after observing the results.

This is an autonomously authored synthetic evaluation, not an independent human benchmark. The cards, model and RRF(60) are those of the original spike. No cross-encoder is used, as explicitly permitted for M0. Results are in `gate-results.json`; every failure is retained.

| Card | Correct / 40 |
| --- | ---: |
| Exact recorded profile | 38 |
| Protected cafe profile | 38 |
| Home profile | 37 |
| Troubleshooting guide | 33 |
| Sunset media | 40 |

There are **14 failures**, including queries with conflicting semantic/identifier cues and DNS terms. The guide subset achieves only 82.5%. Overall success does not imply exact keywords always override semantics, nor that every subgroup passes 90%. These are explicit regression cases for future retrieval work, not removed examples.

Latency measures 600 sequential requests across three deterministically shuffled rounds. Each request recomputes its embedding and runs one native hybrid query including result materialization. p50 = 20.519500 ms; maximum = 164.658600 ms. Loading, index construction and cold start are excluded; these are warm latency metrics, not a concurrency/load guarantee. No embedding result cache or precomputed query vectors are used.

Run in the original pinned M0 environment:

```powershell
python freeze_paraphrases.py
$env:HF_HUB_OFFLINE = '1'
python evaluate_gate.py
```

The historical six-query spike still reports its own gate as unevaluated; `evaluate_gate.py` and this recorded 200-query result are the official gate evaluation. The Neural Bus must treat recall results as candidates and require deterministic selection, capability verification and an exact LFS handle before delivering bytes. Scores grant no authority.
