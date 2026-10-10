# Decisioni M3 → M4 — 2026-10-10

1. L'OK dell'utente autorizza i deliverable progettuali e il percorso verso M4. Non equivale a un risultato di test; la regola della specifica impone gate consecutivi. Questo pacchetto non modifica codice eseguibile.
2. Documento separato dal checkout M2 in uso: base `main` 4a4de8f, branch di progetto dedicato. Nessuna modifica a sandbox, authority o dual-lane dell'altro agente e nessun messaggio a task esterni.
3. Roadmap allineata al diagramma originale: M3 Planner/gate/cache; M4 E2E/red-team/rollback. Stato progettato, non iniziato in runtime; M2 resta aperto fino al report finale.
4. Rust controlla parsing, schemi, risk policy, cache, quote e codifica binaria. Prompt e grammatica sono strumenti di qualità; non concedono autorità.
5. Conservare Gemma 4 E2B fissato come candidato, senza sostituzioni. GPU-resident è il profilo di riferimento; compatibilità, RAM reale e <400 ms vanno provati. Nessun modello è scaricato/caricato da questa PR.
6. Il modello pesa circa 3,35 GB: non può essere mantenuto interamente in RAM sotto un budget totale di 1,5 GB. VRAM dedicata e copie host misurate sono prerequisiti del profilo; dispositivi a memoria unificata richiedono una qualifica distinta.
7. Cache iniziale esatta, nessun riuso di capability, conferme o handle. ANN è solo proposta di template fino alla calibrazione richiesta dal PDF.
8. M4 parte dal solo schema NetConfig attualmente ammesso. Richieste VPN o nuovi tipi non vengono trasformate in capacità già implementate.
9. Conferma e Keep sono input autenticati del client; journal precedente all'effetto, rollback dedicato e recovery del provider sono deterministici. Un planner ostile non può completare queste barriere.
10. I target sono quelli del PDF, senza abbassarli per far passare il modello scelto. Bootstrap, device loading e latenza cold plan sono misure separate e visibili.

## Evidenze e limiti

Letti roadmap/architettura su main, manifest locale del modello e sorgenti sandbox/authority nel checkout M2. Ispezionata visivamente la pagina 14 del PDF originale. GPU letta con interrogazione read-only. Consultate fonti primarie Wasmtime e llama.cpp; nessuna inferenza, benchmark M3, streaming M2, restore attivo o RAM dell'OS integrato eseguito.

Le informazioni fornite dall'utente su altri contributi rimangono contesto; non si inventano risultati CI o commit non verificati. PR #5 è verificata merged. I report M0/M1 precedenti restano gli unici gate chiusi richiamati da questo pacchetto.

## Passaggio operativo, senza scavalcare i gate

Prima: chiudere M2 e stabilizzare Envelope/route/authority/WIT. Poi implementare M3 nell'ordine registry → validator → grammatica/prompt → modello locale → cache → audit → 300 intent. Solo dopo gate M3: servizi LFS/provider/client → SAVE → RECALL → prepare/confirm/apply/rollback → 100 injection e prove di recovery M4. Ogni fase consegna risultati reali prima della successiva.
