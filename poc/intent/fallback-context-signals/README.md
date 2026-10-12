# FALLBACK CONTEXT-SIGNALS

Host demo port. **Not an LLM.** `llm` is null on every proposal. Candidates are
observational Context envelopes labeled `source=ContextSignal`. They do not
grant, admit, or select.

`clock_tick` is a UX window only. Other kept envelopes (`ts` present and
`consumers` contains `intent`) become proposal rows. The handle on a row is a
display-only SHA-256 prefix of the observed `object.handle`.

## Commands

From the repository root:

```text
python3 poc/intent/fallback-context-signals/consume_ciao.py
python3 poc/intent/fallback-context-signals/consume_scenarios.py
```

`consume_ciao.py` reads `fixtures/ciao-demo-signals.jsonl` and writes:

- `out/proposal-ciao.json`
- `out/01-proposal-rows.png`
- `out/02-glass-open.png`

`consume_scenarios.py` reads `fixtures/scenarios/*.jsonl` (yt-favij,
whatsapp-mamma, ig-barbiere) and writes `out/<scenario>-proposal.json` plus
the same two PNG views. PNG drawing is the host Pillow renderer in
`consume_ciao.py`.

Requires Python 3 and Pillow. Inter is used when present under
`/usr/share/fonts/opentype/inter/` or `/usr/share/fonts/truetype/macos/`.
