# Roadmap pubblica

La roadmap segue gate tecnici verificabili per il sistema personale AI-native. Non sono date promesse; un gate resta aperto finché i risultati e i limiti non sono riproducibili e documentati.

| Fase | Stato | Criteri / prossime prove |
| --- | --- | --- |
| M0 — Semantic hybrid recall | Chiuso sul fixture sintetico | Recall@1 186/200 (93%) e warm p95 25,67 ms sulle cinque card correnti. Non equivale a un benchmark indipendente o a qualità generalizzabile. |
| M1 — Immutable LFS | Chiuso per il gate registrato | Kernel Rust, oggetti cifrati, commit firmati e policy Biscuit; 10.000 terminazioni di processo senza perdita o duplicazione nel test. Power-loss, anchor anti-rollback e servizio persistente restano fuori. |
| M2 — Isolation & throughput | In corso | Bridge iceoryx2 reale tra processi verificata. Il target >100.000 msg/s non è raggiunto; isolamento del peer e Wasmtime non ancora verificati. |
| M3 — Sistema integrato | Non iniziato | Integrare LFS, bus, semantic service e task runtime dietro API stabili; definire bootstrap, gestione chiavi, aggiornamenti, recovery e client utente. |

## M2: lavoro aperto

1. Compilare, testare e sottoporre a Clippy il boundary Wasmtime; aggiungere fixture di componenti ammessi e rifiutati, memory growth, fuel exhaustion e import non autorizzati.
2. Vincolare l'esecuzione a interfacce WIT versionate, verificare gli export attesi e integrare il sandbox con il control plane senza esporre memoria host o capability ambientali.
3. Completare authority con handle opachi e Biscuit attenuati; validare route, expiry, replay e identità dei peer su processi separati.
4. Aggiungere lane bulk streaming con blocchi da 256 KiB, massimo 8 crediti in volo, cancellazione e backpressure che non blocchi la lane fast sotto i 4 KiB.
5. Implementare writer temporaneo per file grandi, cifratura chunked, verifica progressiva e commit sull'hash BLAKE3 del plaintext canonico.
6. Ripetere benchmark pipelined con distribuzioni di latenza e slow-reader fault injection; chiudere il gate solo con throughput e isolamento misurati contro i criteri M2.

```mermaid
flowchart LR
  M0[M0 recall: chiuso sul fixture] --> M1[M1 LFS: gate crash chiuso]
  M1 --> M2[M2 isolation e throughput: aperto]
  M2 --> M3[M3 integrazione: non iniziata]
```

Contributi utili: test avversariali per componenti Wasm, benchmark indipendenti, review delle capability e dei confini IPC, riproduzione su host Windows/Linux, documentazione dei limiti. La precedente preview desktop Debian/KDE non è il criterio di completamento di questa roadmap.
