# Intent Engine Zero-App v0 — Context → Glass candidates

**Repo:** `FrySco19/VelaOS` (GitHub; non Origin)  
**Branch:** `intent/zero-app-v0` (da `main` @ `6f05ff0ca06f13f978ae9e6d14ad1281417b9ee9`)  
**Riferimento storico (esistente, non duplicato):** `grok/product-intent-shell` @ `f0942be5cd108a9782443bf6316475f39e457ebb` — path `poc/intent`  
**ISO VB §7:** `vela/vbox-iso` @ `60a5b8b9ec1e566f20c450366e8fa16c10ea34dd` — filosofia: IA parte dell’OS, non chat; il modello non inventa stato/privilegi.  
**Confine:** Intent Engine Zero-App propone superfici Glass dal contesto reale. Output = dati etichettati **Proposal**. Mai esecuzione, mai grant, mai `UserSelection` forgiata. Nessun push/merge su `main` senza ok Francesco. Nessun shell chrome, nessun Policy admit, nessun Context observer loop.

Questo documento descrive la pipeline già presente in `poc/intent` su `grok/product-intent-shell` @ `f0942be`. Non copia né reimplementa quel crate: registra come i segnali di contesto diventano candidati Glass.

## Pipeline

```text
segnale contesto (utterance + corpus locale)
  → retriever lanes: BM25 (+ title stand-in | M0 reale)
  → [opzionale] LlmProposer (solo handle già nel corpus)
  → Reciprocal Rank Fusion (k = 60, stesso K di poc/m0 bridge_recall.py)
  → IntentProposal { candidates[], rationale }
  → Glass view (sola lettura / click-by-index)
  → UserSelection re-derivata dalla shell sul click reale
  → SelectionGatedPdp (fuori da Intent)
```

Fonte comportamento: `poc/intent/src/lib.rs` (`IntentEngine::propose_candidates`) su `f0942be`.

### Step

1. **Corpus** — metadati locali (`Doc { handle, version, oid, title, body }`). Nessun contenuto oggetto; nessun diritto.
2. **Lanes** — BM25 (Okapi su title+body); TitleOverlap (stand-in deterministico del dense M0); opzionale `M0Ranking` / `M0Process`; opzionale LLM.
3. **RRF** — `score(d) = Σ 1/(k + rank)`, k=60, rank 1-based. Lane che non conoscono un handle lo ignorano.
4. **Proposal** — `Candidate { handle, version, title, score, sources }` presi dal corpus al momento della proposta. `IntentProposal.auto_resolve()` è sempre `false`.
5. **Glass** — mostra titolo/versione/score/sources (handle hex solo audit). Click: solo `{token, action, candidate: index}`; la pagina non può mandare handle/version/selection. La shell re-deriva `UserSelection{handle, expected_version}` dalla riga mostrata.

## Etichette Proposal

| Campo | Significato | Autorità? |
|-------|-------------|-----------|
| `utterance` | Query utente | no |
| `candidates[].handle` | Handle LFS opaco (32 B) | no — dato |
| `candidates[].version` | Versione corpus al momento della proposta → `expected_version` futuro | no |
| `candidates[].title` | Display | no |
| `candidates[].score` | Score RRF fuso | no |
| `candidates[].sources` | Lane che hanno rankato (`Bm25`, `Title`, `M0`, `Llm`) — audit/display only | no |
| `rationale` | Testo libero LLM (non parsato, non trusted) | no |

`Candidate::as_selection()` costruisce un `UserSelection` **plain data**, liberamente costruibile → **NON** autorità. Solo `UserSelection` mintato da `TrustedSelectionChannel` entra nel PDP.

## Limiti

| Limite | Valore | Perché |
|--------|--------|--------|
| `RRF_K` | 60 | Allineato a M0 |
| `MAX_CANDIDATES` | 8 | Lista corta su Glass |
| `MAX_LLM_PROPOSALS` | 8 | LLM può solo aggiungere handle già nel corpus |
| `MAX_UTTERANCE` (Glass) | 512 | Hardening host demo |
| `MAX_BODY` (Glass) | 16 KiB | Idem |
| Corpus vuoto / utterance vuota | `IntentProposal` vuota | Nessuna proposta |
| Handle sconosciuto (LLM/M0) | Droppato | Modello non inventa oggetti |
| Versione LLM | Mai | Versioni sempre dal corpus |
| Execute / grant / admit / sign | Assenti in `vela-intent` | `tests/no_authority.rs`; zero deps crypto/policy |
| Route Glass `/select` o simili | Assenti | Solo `/submit` e `/click` |

## Fuori confine

- Non scrive shell chrome (toolbar, dock, Spaces) — confine Shell.
- Non ammette Policy / non firma frame — confine Policy.
- Non osserva il bus Context — confine Context; qui consuma solo segnali già materiali (utterance + corpus).
- Non apre PR su `main` senza ok Francesco.
