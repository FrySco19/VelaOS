# Protocollo di memoria M3/M4

Stato: piano di residenza e misura; nessun risultato RAM/VRAM del sistema integrato.

## Modello e piattaforma di riferimento

Il modello candidato resta quello già fissato in [assets.json](../../components/local-ai/assets.json): Gemma 4 E2B IT QAT Q4_0, revisione `675cff42a74c774d6cb76f76d8eacb49b48c9b93`, SHA-256 `fa401b55b07ee70a54c6dae3903c783a6e65064312529ea57175cb5f8dec6634`. Dimensione artefatto **3.349.516.256 byte**. Non viene sostituito silenziosamente e il benchmark della preview desktop non chiude M3.

Osservazione locale 2026-10-10: NVIDIA RTX 2060, 6144 MiB di VRAM; occupazione istantanea esterna al test 792 MiB. Quest'ultima non è una disponibilità garantita. Un file da 3,35 GB non equivale alla somma esatta delle allocazioni GPU: tensor layout, KV, workspace e driver vanno misurati.

Il precedente pacchetto runtime b11539 è per la preview desktop, non certifica il futuro servizio Rust o il backend CUDA. Tutto il nuovo controller di memoria e l'adapter Neural Bus sono Rust. L'inferenza può utilizzare il motore nativo esistente come dipendenza confinata; non aggiunge API generiche ai task Wasm. Versione, backend, compatibilità modello e build riproducibile diventano parte del manifest prima del gate.

## Stati di residenza

| Stato | Comportamento |
| --- | --- |
| Probing | Rileva device, memoria disponibile e allocazioni richieste senza caricare copie multiple. |
| Loading | Una istanza e trasferimenti limitati; nessuna elaborazione interattiva prima di Ready. |
| ReadyGpu | Pesi supportati e KV sulla GPU; una inferenza attiva, contesto 4096 token, slot unico. |
| Busy | Lease di compute; quote CPU e staging, cancellazione fra unità di lavoro. |
| Pressure | Sospende nuove inferenze, rilascia cache opzionali; non tocca il pool fast-lane. |
| Unloaded | Scaricamento esplicito in caso di pressione; route deterministiche già valide restano disponibili. |
| Unavailable | RAM/VRAM insufficienti o device perso: errore visibile, nessuna promessa di limite rispettato. |

ReadyGpu è il profilo predefinito sugli host compatibili, senza cloud né account. Non scarichiamo e ricarichiamo il modello per ogni intento: il costo di caricamento si misura separatamente dal cold plan, che significa route-cache miss con modello pronto.

Un dispositivo senza VRAM dedicata non può rispettare questo profilo per magia. La memoria unificata conta nel budget fisico, anche se chiamata «GPU». La via CPU usa lo stesso modello solo se ammessa esplicitamente dal profilo hardware e riqualificata; non nasconde una violazione del target consumer. Fino alla prova, supporto completo <1,5 GB su tali dispositivi resta non dimostrato.

## Cosa significa offloading

Pesi e KV su VRAM dove il backend lo supporta; nessuna seconda istanza embedding/planner dello stesso modello; staging host limitato; niente locking permanente dell'intero artefatto in RAM. Cache del prompt, history e output sono limitati e con retention corta. I log non accumulano copie dei prompt completi.

La mappatura dell'artefatto e l'offload non garantiscono il rilascio delle pagine residenti o delle copie host. Misureremo memoria privata, pagine di backing residenti, memoria condivisa, staging e memoria del driver. Un'opzione di runtime che dichiara GPU layers non prova da sola il consumo complessivo.

Il servizio riceve il modello come risorsa LFS autorizzata, con hash verificato. L'adapter può usare backing privato necessario al motore, senza introdurre percorsi utente, servizi POSIX o accesso ambientale per i guest.

Il controller acquisisce una lease GPU usando il bisogno misurato di pesi + KV + workspace + margine. Cambiare cache precision o layout richiede verifica di correttezza e qualità; non si applica quantizzazione ulteriore silenziosa. Device lost invalida lease e richieste in volo; il planner non dispone di authority riutilizzabile.

## Budget iniziale, MB decimali

| Area | Massimo progettuale a riposo |
| --- | ---: |
| Base sistema e client | 550 |
| Host runtime SLM, driver attribuibile, staging residente | 450 |
| Ricerca semantica e cache LFS | 150 |
| Wasmtime, executor, control plane | 100 |
| Pool bus e metadati | 50 |
| Margine | 150 |
| Totale | **1450** |

I due task Wasm possono usare 64 MiB ciascuno durante lavoro: il budget a riposo non è il tetto sotto carico. Pool bulk iniziale 2 MiB per stream, fino a quattro stream ammessi nel profilo iniziale, con metadati/crypto separatamente quotati. L'admission controller deve sommare tutte le quote, non moltiplicare pool per ogni destinatario senza limite.

La tabella è un vincolo di progetto, non una misura. La priorità è lasciare almeno 50 MB rispetto al limite **1.500.000.000 byte**. Non confondiamo GB con GiB. Riservare spazio di indirizzamento Wasm non equivale a consumare la stessa RAM fisica; registriamo entrambi.

## Prova richiesta

1. Avvio del sistema di riferimento completo; client presente, LFS/indice pronti, modello in ReadyGpu. Nessun confronto con il consumo totale del PC Windows usato per sviluppo.
2. Misura baseline e incremento attribuibile dopo loading; documentare picco startup, cold e warm. Il picco viene riportato anche se il gate riguarda idle.
3. Cinque minuti senza task; campioni ogni secondo. Gate: massimo consumo fisico attribuibile del sistema idle <1,5 GB, pagine condivise contate una volta; VRAM riportata separatamente.
4. Una inferenza, due task e quattro stream; riportare picchi, backpressure, latenza fast-lane e ritorno alla baseline. Venti cicli di carico/scarico per individuare ritenzione e leak.
5. Pressione VRAM esterna e device loss; nessun fallback non dichiarato, nessuna espansione illimitata in RAM. Unloading è visibile e la successiva latenza di startup resta separata.

Per il PoC host misuriamo l'incremento dei servizi Vela e la memoria dei processi, ma non lo chiamiamo consumo di un OS completo. Il gate consumer globale richiede una piattaforma integrata. La soglia cold <400 ms potrebbe non essere raggiunta dal modello/hardware attuale: non verrà allentata per far passare M3.

Fonti: [runtime fissato e controlli GPU/KV](https://github.com/ggml-org/llama.cpp/blob/b11539/tools/server/README.md), [modello alla revisione fissata](https://huggingface.co/google/gemma-4-E2B-it-qat-q4_0-gguf/blob/675cff42a74c774d6cb76f76d8eacb49b48c9b93/README.md), [pool Wasmtime e spazio virtuale](https://docs.wasmtime.dev/api/wasmtime/struct.PoolingAllocationConfig.html).
