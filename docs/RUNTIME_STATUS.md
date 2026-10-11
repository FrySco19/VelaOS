# Stato di implementazione — 11 ottobre 2026

Il runtime Rust M2–M4 è pubblicato nella PR di sviluppo. M4 resta aperto: non esiste ancora un sistema completo o installabile. Il checkout storico e la VM precedente non sono stati usati come prova della nuova architettura.

| Verifica | Risultato misurato | Confine |
| --- | --- | --- |
| Wasm, capability e streaming | 12 test M2 superati | Componenti Wasm reali; peer nativi fidati sull'host di sviluppo |
| Router e cache | 2 test M3 superati | Template finiti; capability ricontrollate a ogni cache hit |
| Restore e recovery | 9 test M4 superati | Provider persistente di proprietà nel dominio isolato M1; journal firmato e watchdog; ingresso UI di produzione non collegato |
| Tre benchmark dual-lane | >100.000 msg/s, p99 <100 µs, regressione <20% | 10 GB generati, non file LFS bulk durevoli |
| Planner senza route cache, 20 ripetizioni | 6.000 output validi; router 98,33–98,67%; massimo 246,959 ms | Stabilità fallita su una richiesta ambigua; modello da solo 92,33–92,67% |
| Router con cache, 20 ripetizioni | 98,67%, piani identici; massimo cold 171,351 ms, warm 0,524 ms | 300 chiamate modello + 5.700 cache hit entro TTL; non prova stabilità dopo scadenza |
| SLM con nuovo launcher Rust | Riposo: commit 599 MB, working set 490 MB; risveglio con inferenza 2.945 ms | Scaricamento automatico dopo 5s; picco campionato attivo: commit 4,55 GB, working set 610 MB; non è la RAM dell'intero OS. Il risveglio è separato dal giro di latenza sotto |
| Nuovo profilo SLM, un giro da 300 richieste a modello pronto | 300 output validi; Router 295/300; p95 155,918 ms, massimo 307,335 ms | Non chiude il gate: massimo oltre 250 ms, nessuna prova di consistenza su 20 giri; il risveglio a freddo non è incluso |

I test e Clippy con warning negati passano localmente. I controlli CI non eseguono il modello GPU né qualificano le prestazioni del sistema completo.

## Lavoro necessario per arrivare alla fine di M4

1. Portare l'ingresso del compiler e gli accessi LFS su servizi Neural Bus persistenti con identità protette. I template di piano non devono trasportare byte degli oggetti.
2. Collegare il richiamo ibrido M0 al servizio Rust e alla selezione esplicita di OID/handle, conservando gli epoch fissati e filtrando per capability.
3. Implementato il provider persistente nel dominio di prova isolato: oggetti M1 reali, verifica degli OID, head firmato e gestione del precedente stato durante applicazioni interrotte. La pubblicazione transazionale riguarda il solo head; nessuna atomicità tra database o modifica di interfacce fisiche è dichiarata.
4. Implementati supervisor persistente, watchdog da 120s e gestori del client autenticato con ticket CONFIRM monouso e ticket KEEP separato. Il grant di recovery è emesso da una radice dedicata, ristretto a transazione/dominio/OID esatti e revocato nel ledger al completamento; non eredita la scadenza del token utente. Resta da collegare l'ingresso UI autenticato al servizio Neural Bus di produzione.
5. Eseguire SAVE → RECALL → selezione esatta → CONFIRM → APPLY → KEEP/rollback, i 100 casi di injection end-to-end e i guasti dei servizi reali. Le regressioni del parser non sostituiscono queste prove.
6. Misurare RAM a riposo e prestazioni sull'intero runtime. Il target <1,5 GB non può essere ricavato dal solo working set del modello o ignorando la memoria impegnata.

M5 segue il gate M4 e riguarda il port seL4/QEMU. Nessuna chiusura di M4 o avanzamento ufficiale a M5 è dichiarato da questo documento.

### Verifica del provider e del supervisor

I moduli sono nel crate M4: inserirli nel crate M2 creerebbe una dipendenza circolare, perché richiedono il coordinatore M4 e il Router M3, che già dipendono da M2. I cinque nuovi test verificano riapertura dei database, scope errato, permesso utente scaduto, sessione errata, recovery, revoca dopo KEEP, watchdog e interruzione prima dello staging. Le scadenze sono indotte nei test; non sono state attese per 120 secondi reali e i test di riapertura non sostituiscono prove di kill del processo o perdita di alimentazione.

Il bootstrap fidato deve fornire chiavi persistenti protette, capability del provider, LFS sbloccato e database privati. Usare `ProviderConfig` e una cache metadata da 1 MiB. I gestori del supervisor sono API native fidate, non endpoint accessibili al planner. Il watchdog ha uno stack da 2 MiB e controlla ogni 250 ms; I/O in corso può ritardare il rollback. Letture delle proprietà limitate a 4 KiB, metadata a 4 KiB e record del grant a 12 KiB. Il journal cresce con le operazioni: questi limiti non costituiscono una misura della RAM complessiva.

### Ottimizzazione della memoria SLM

Il nuovo launcher `poc/m3/src/worker.rs` gestisce il backend CUDA b11539, verifica il modello fissato con buffer SHA256 da 64 KiB e applica limiti al contesto, al batch e alle cache. `--gpu-layers all` lasciava 2152,50 MiB di embedding nel buffer CPU: il posizionamento esplicito `.*=CUDA0` elimina quel buffer modello CPU, come verificato dai log (3494,28 MiB di pesi su CUDA). Il loader upstream libera già i buffer di upload: il dato storico di 4,70 GB non dimostra un leak.

Lo scaricamento dopo 5s senza richieste riduce il commit del worker a 599.150.592 byte e, dopo un secondo ciclo di inferenza/scaricamento, a 594.862.080 byte. Il launcher impegna altri 741.376 byte nell'ultima misura. Il picco attivo campionato resta 4.551.987.200 byte: non è implementato un cap di commit attivo sotto 1,5 GB. Il working set attivo campionato è invece 609.738.752 byte. Non attribuiamo automaticamente il commit residuo a un leak o a una particolare allocazione del driver senza ulteriori prove.

Il risveglio automatico è stato verificato con una vera richiesta di inferenza (2.945,447 ms). Il costo di riattivazione è un trade-off deliberato e accettato per il profilo a memoria inattiva ridotta: pre-caricare il modello eviterebbe quel costo ma manterrebbe occupate risorse che lo scaricamento dopo 5s rilascia. Il valore di 2.945 ms non è incluso nel giro delle 300 richieste, eseguito a modello pronto. In quel giro il massimo di 307,335 ms supera comunque il limite di 250 ms; il test con zero checkpoint non ha risolto il superamento e ha peggiorato il p95, quindi non è stato adottato. Non dichiariamo superato il gate di latenza né attribuiamo la coda massima esclusivamente al risveglio a freddo. I [risultati memoria](../poc/m3/memory-results.json) e il [giro completo da 300 casi](../poc/m3/memory-planner-gate.json) includono anche prove fallite e limiti. Il target RAM dell'intero OS e il gate M4 restano aperti; l'ingresso di produzione deve ancora usare il Neural Bus.
