# Roadmap pubblica

La roadmap segue gate tecnici verificabili per il sistema personale AI-native. Non sono date promesse; un gate resta aperto finché i risultati e i limiti non sono riproducibili e documentati.

| Fase | Stato | Criteri / prossime prove |
| --- | --- | --- |
| M0 — Semantic hybrid recall | Chiuso sul fixture sintetico | Recall@1 186/200 (93%) e warm p95 25,67 ms sulle cinque card correnti. Non equivale a un benchmark indipendente o a qualità generalizzabile. |
| M1 — Immutable LFS | Chiuso per il gate registrato | Kernel Rust, oggetti cifrati, commit firmati e policy Biscuit; 10.000 terminazioni di processo senza perdita o duplicazione nel test. Power-loss, anchor anti-rollback e servizio persistente restano fuori. |
| M2 — Isolation & throughput | In corso | Bridge iceoryx2 reale verificata; Wasmtime compila e supera i test base in GitHub Actions. Restano aperti isolamento avversariale, throughput e streaming. |
| M3 — Sistema integrato | Non iniziato | Integrare LFS, bus, semantic service e task runtime dietro API stabili; definire bootstrap, gestione chiavi, aggiornamenti, recovery e client utente. Obiettivo di sistema: meno di 1,5 GB RAM a riposo, da misurare sul sistema integrato. |
| M5 — Bridge PCB–Executor seL4 | Fasi 2/3 verificate in QEMU; gate aperto | Pagina condivisa 4 KiB, richiesta firmata Ed25519, ack via protected call, Wasm ricevuto eseguito con risultato 7. Rifiuti di firma, bounds, scope, replay, fuel e WASI verificati. Biscuit nel PCB, TTL/replay persistente, LFS end-to-end e misure sul target restano aperti. |

## M2: lavoro aperto

1. Compilare, testare e sottoporre a Clippy il boundary Wasmtime; aggiungere fixture di componenti ammessi e rifiutati, memory growth, fuel exhaustion e import non autorizzati.
2. Vincolare l'esecuzione a interfacce WIT versionate, verificare gli export attesi e integrare il sandbox con il control plane senza esporre memoria host o capability ambientali.
3. Completare authority con handle opachi e Biscuit attenuati; validare route, expiry, replay e identità dei peer su processi separati.
4. Aggiungere lane bulk streaming con blocchi da 256 KiB, massimo 8 crediti in volo, cancellazione e backpressure che non blocchi la lane fast sotto i 4 KiB.
5. Implementare writer temporaneo per file grandi, cifratura chunked, verifica progressiva e commit sull'hash BLAKE3 del plaintext canonico.
6. Chiudere il gate prestazionale con >100.000 messaggi/s da 4 KiB e p99 sotto 100 µs, riportando separatamente round-trip e latenza unidirezionale.
7. Provare uno stream di almeno 10 GB con un lettore lento: massimo 20% di regressione sulla corsia rapida, memoria limitata ai buffer/crediti e nessuna crescita proporzionale alla dimensione dell'oggetto.
8. Misurare il budget RAM dell'intero sistema a riposo; il target è <1,5 GB, non ancora raggiunto né misurato per l'architettura corrente.

I parametri iniziali del bulk sono 256 KiB per blocco e 8 crediti in volo (2 MiB per stream). Sono obiettivi di progetto; la lane bulk e il protocollo di crediti non sono ancora implementati.

## M5 — bridge PCB–Executor

Il [prototipo](../poc/m5_executor/README.md) usa Wasmi `no_std`, una pagina condivisa di 4 KiB e una chiamata Microkit protetta con ack e identificatore di richiesta. L'Executor autentica uno snapshot privato e interpreta gli stessi byte con fuel e limiti di memoria, senza WASI. Il risultato 7 ritorna al PCB nei registri IPC. Il runner Rust ripete build e boot e richiede tutti i marker dei casi negativi; i test host verificano descrittori e crittografia.

Il [contratto del bridge](../poc/m5_executor/BRIDGE_CONTRACT.md) definisce ownership, gestione replay, distinzione fra token e capability kernel e i criteri ancora aperti: Biscuit e attenuazione nel PCB, scadenza con clock fidato, replay persistente, revoca/cancellazione e recupero crash, persistenza dei risultati LFS e misure RAM/latenza dell'intero sistema. Non è una chiusura di M5.

```mermaid
flowchart LR
  M0[M0 recall: chiuso sul fixture] --> M1[M1 LFS: gate crash chiuso]
  M1 --> M2[M2 isolation e throughput: aperto]
  M2 --> M3[M3 integrazione: non iniziata]
```

Contributi utili: test avversariali per componenti Wasm, benchmark indipendenti, review delle capability e dei confini IPC, riproduzione su host Windows/Linux, documentazione dei limiti. La precedente preview desktop Debian/KDE non è il criterio di completamento di questa roadmap.
