# Stato di implementazione — 11 ottobre 2026

Aggiornamento nativo 12 ottobre: vedere [audit di convergenza](NATIVE_CONVERGENCE.md).
La preview Glass x86, il guest Microkit e il runtime host rimangono separati.
Il nuovo `poc/storage-core` è un componente Rust `no_std` verificato, non ancora
un driver guest o LFS persistente integrato. Nessun gate consumer viene chiuso.

Il runtime Rust M2–M4 è pubblicato nella PR di sviluppo. **M4 è condizionatamente chiuso come milestone d'implementazione; M5 ha un prototipo parziale, non una release consumer.** La chiusura non certifica le soglie globali o un sistema completo/installabile. Richiamo ibrido reale e persistenza sono provati nella sessione nativa; UI protetta, servizi persistenti, qualificazione completa e RAM dell'intero OS restano aperti. Il profilo mmap è stato misurato e scartato per la residenza RAM; il default usa caricamento streaming con contesto 512 e KV offload. Il checkout storico e la VM precedente non sono stati usati come prova della nuova architettura.

| Verifica | Risultato misurato | Confine |
| --- | --- | --- |
| Wasm, capability e streaming | 12 test M2 superati | Componenti Wasm reali; peer nativi fidati sull'host di sviluppo |
| Router, cache e adapter | 4 test M3 superati | Template finiti, permessi ricontrollati e modello ostile incapace di inventare focus attraverso l'adapter HTTP |
| Restore e recovery | 11 test M4 standard + 1 test M0 reale superati | Provider persistente M1, journal firmato, watchdog e sessione nativa SAVE/RECALL/selezione/CONFIRM/KEEP; ingresso UI di produzione non collegato |
| Tre benchmark dual-lane | >100.000 msg/s, p99 <100 µs, regressione <20% | 10 GB generati, non file LFS bulk durevoli |
| Planner senza route cache, 20 ripetizioni | 6.000 output validi; router 98,33–98,67%; massimo 246,959 ms | Stabilità fallita su una richiesta ambigua; modello da solo 92,33–92,67% |
| Router con cache, 20 ripetizioni | 98,67%, piani identici; massimo cold 171,351 ms, warm 0,524 ms | 300 chiamate modello + 5.700 cache hit entro TTL; non prova stabilità dopo scadenza |
| Adapter typed-focus v2, 20 ripetizioni | 6.000 output validi; 296/300 instradati per round; p95 163,4565 ms, massimo 264,2297 ms | Unico superamento su route-cache miss al round 16/caso 295; gate <250 ms fallito, non è una misura QEMU |
| Autorizzazione cache isolata, 6.000 hit | Nuova verifica Biscuit per hit; p95 0,257 ms, massimo 0,601 ms | Nessun modello o byte oggetto; non autorizza a riusare o saltare capability |
| SLM con nuovo launcher Rust | Riposo: commit 599 MB, working set 490 MB; risveglio con inferenza 2.945 ms | Scaricamento automatico dopo 5s; picco campionato attivo: commit 4,55 GB, working set 610 MB; non è la RAM dell'intero OS. Il risveglio è separato dal giro di latenza sotto |
| Nuovo profilo SLM, un giro da 300 richieste a modello pronto | 300 output validi; Router 295/300; p95 155,918 ms, massimo 307,335 ms | Non chiude il gate: massimo oltre 250 ms, nessuna prova di consistenza su 20 giri; il risveglio a freddo non è incluso |

I test e Clippy con warning negati passano localmente. I controlli CI non eseguono il modello GPU né qualificano le prestazioni del sistema completo.

Il nuovo adapter mantiene invariabile il prefisso di sistema e trasporta il focus
nel messaggio strutturato. Il Router continua a usare il booleano fidato nativo:
il modello non può inventare una selezione. Il primo giro v2 ha ottenuto 296/300,
p95 120,35 ms e massimo 129,40 ms. Il precedente giro v1 da 6.000 richieste aveva
classificazioni identiche e 295/300 in ogni round, ma sette campioni oltre 250 ms
(massimo 258,47 ms): il suo gate rimane fallito e i dati sono conservati.

La più recente ripetizione v2 ha prodotto 6.000/6.000 output validi e 296/300
route composte corrette in ciascuno dei 20 round, con classificazioni normalizzate
identiche. Ha p95 163,4565 ms e un solo massimo oltre soglia, 264,2297 ms
(round 16, caso 295). Quel campione è un **miss** della route cache mentre il
modello era pronto; il tempo include inferenza locale e Router/Biscuit, non è un
cache hit né una misura QEMU. Il protocollo integrato distinto ha 5.700 cache hit
con verifica fresca, ma il suo massimo warm registrato è 14,5466 ms e non soddisfa
il relativo limite di 5 ms. In una prova breve e isolata di 6.000 hit, l'intero
percorso cache più nuova verifica Biscuit ha p95 0,257 ms e massimo 0,601 ms.
L'evidenza non giustifica l'omissione del controllo di autorità e non consente di
dichiarare superato il gate stretto dei 250 ms. I dati v2 completi restano
conservati nei due rapporti JSON locali dell'esecuzione.

## Condizioni pendenti per la chiusura completa di M4

La chiusura condizionale consente di iniziare la progettazione di M5 (port seL4/QEMU), ma non dichiara superato il gate di sistema né autorizza una release M5. Restano da soddisfare le condizioni seguenti; regressioni o risultati fuori soglia riaprono M4:

1. Portare l'ingresso del compiler e gli accessi LFS su servizi Neural Bus persistenti con identità protette. I template di piano non devono trasportare byte degli oggetti.
2. Portare il collegamento M0/sessione Rust, ora verificato con embedding reali offline, nel servizio persistente di produzione e nell'interfaccia umana, mantenendo epoch e filtro capability.
3. Implementato il provider persistente nel dominio di prova isolato: oggetti M1 reali, verifica degli OID, head firmato e gestione del precedente stato durante applicazioni interrotte. La pubblicazione transazionale riguarda il solo head; nessuna atomicità tra database o modifica di interfacce fisiche è dichiarata.
4. Implementati supervisor persistente, watchdog da 120s e gestori del client autenticato con ticket CONFIRM monouso e ticket KEEP separato. Il grant di recovery è emesso da una radice dedicata, ristretto a transazione/dominio/OID esatti e revocato nel ledger al completamento; non eredita la scadenza del token utente. Resta da collegare l'ingresso UI autenticato al servizio Neural Bus di produzione.
5. Eseguire SAVE → RECALL → selezione esatta → CONFIRM → APPLY → KEEP/rollback, i 100 casi di injection end-to-end e i guasti dei servizi reali. Le regressioni del parser non sostituiscono queste prove.
6. Misurare RAM a riposo e prestazioni sull'intero runtime. Il target <1,5 GB non può essere ricavato dal solo working set del modello o ignorando la memoria impegnata.

M5 riguarda il port seL4/QEMU. Il prototipo ora avvia cinque domini e completa un intent fisso con Wasm e risultato 42; lo storage nel guest è volatile, senza SLM né verifica Biscuit integrata. Il port completo e la release restano subordinati alle condizioni pendenti di M4. Vedere il [design](M5_DESIGN.md), il [port riproducibile](../poc/m5_sel4/README.md) e il [rapporto dei limiti](../GATE_F_G_REPORT.md).

### Sessione nativa e pubblicazione

La nuova sessione mantiene fino a 100 OID autorizzati e ticket non serializzabili.
SAVE e lettura delle card usano il token del client, senza delegare i permessi del
provider. Ranking estranei o duplicati vengono rifiutati; la selezione umana resta
esplicita e il supervisor ricontrolla la capability. Il test attraversa storage,
conferma, trasferimento M2 e riapertura durevole. Usa una classifica di fixture:
non esegue il modello di embedding M0. Il test separato `live_m0_session_to_durable_keep`
è invece passato in 14,78 secondi con E5 fissato e LanceDB reali, senza rete:
salvataggio, card autorizzate, ricerca ibrida, selezione esatta, conferma, KEEP e
riapertura durevole. Conferma e selezione sono eventi fidati del test; non una UI.

La nuova CI seL4 ricostruisce l'immagine fissata e controlla i marker Wasm/intent
entro 60 secondi, terminando QEMU in modo controllato. Non misura la RAM del guest
e non qualifica i benchmark attraverso tempi della seriale. La ricostruzione
locale del container e lo smoke sul nuovo guest sono passati.

### Verifica del provider e del supervisor

I moduli sono nel crate M4: inserirli nel crate M2 creerebbe una dipendenza circolare, perché richiedono il coordinatore M4 e il Router M3, che già dipendono da M2. I cinque nuovi test verificano riapertura dei database, scope errato, permesso utente scaduto, sessione errata, recovery, revoca dopo KEEP, watchdog e interruzione prima dello staging. Le scadenze sono indotte nei test; non sono state attese per 120 secondi reali e i test di riapertura non sostituiscono prove di kill del processo o perdita di alimentazione.

Il bootstrap fidato deve fornire chiavi persistenti protette, capability del provider, LFS sbloccato e database privati. Usare `ProviderConfig` e una cache metadata da 1 MiB. I gestori del supervisor sono API native fidate, non endpoint accessibili al planner. Il watchdog ha uno stack da 2 MiB e controlla ogni 250 ms; I/O in corso può ritardare il rollback. Letture delle proprietà limitate a 4 KiB, metadata a 4 KiB e record del grant a 12 KiB. Il journal cresce con le operazioni: questi limiti non costituiscono una misura della RAM complessiva.

### Ottimizzazione della memoria SLM

Il nuovo launcher `poc/m3/src/worker.rs` gestisce il backend CUDA b11539, verifica il modello fissato con buffer SHA256 da 64 KiB e applica limiti al contesto, al batch e alle cache. `--gpu-layers all` lasciava 2152,50 MiB di embedding nel buffer CPU: il posizionamento esplicito `.*=CUDA0` elimina quel buffer modello CPU, come verificato dai log (3494,28 MiB di pesi su CUDA). Il loader upstream libera già i buffer di upload: il dato storico di 4,70 GB non dimostra un leak.

Lo scaricamento dopo 5s senza richieste riduce il commit del worker a 599.150.592 byte e, dopo un secondo ciclo di inferenza/scaricamento, a 594.862.080 byte. Il launcher impegna altri 741.376 byte nell'ultima misura. Il picco attivo campionato resta 4.551.987.200 byte: non è implementato un cap di commit attivo sotto 1,5 GB. Il working set attivo campionato è invece 609.738.752 byte. Questi valori appartengono al profilo precedente (`--load-mode none`, contesto 1024); il solo contatore di commit privato non prova che l'intero GGUF sia stato allocato nell'heap CPU, dato che i log attestano anche 3.494,28 MiB di buffer modello CUDA. Non attribuiamo automaticamente il commit residuo a un leak o a una particolare allocazione del driver senza ulteriori prove.

La nuova prova mmap/512 ha raggiunto 3.942.907.904 byte di working set, pur con buffer modello CUDA. È stata sostituita da `--load-mode none`, mantenendo 999 layer GPU, KV-cache offload e contesto 512. La prova streaming ha raggiunto 589.180.928 byte residenti (circa 85% in meno nel confronto dei campioni) e commit attivo 4.545.949.696 byte. Dopo sette secondi di inattività: commit 842.022.912 byte e working set 469.155.840 byte. Il problema del commit attivo non è dichiarato risolto.

Sul singolo giro da 300 richieste, streaming/512 ha ottenuto 295/300 corrette con Router, p95 121,65 ms e massimo 245,77 ms; il risveglio è escluso dal punteggio. Il giro mmap aveva p95 124,41 ms e massimo 248,90 ms. Si conservano [mmap](../poc/m3/mmap-512-memory.json), [streaming](../poc/m3/stream-512-memory.json) e le classificazioni complete: sono campioni del worker Windows, non RAM dell'intero OS. I gate del planner e della cache sono stati riallineati al target massimo di 250 ms: il precedente limite di 400 ms non è più accettato. Venti ripetizioni e qualificazione integrata restano separate dal singolo giro.

Il risveglio automatico è stato verificato con una vera richiesta di inferenza (2.945,447 ms). Il costo di riattivazione è un trade-off deliberato e accettato per il profilo a memoria inattiva ridotta: pre-caricare il modello eviterebbe quel costo ma manterrebbe occupate risorse che lo scaricamento dopo 5s rilascia. Il valore di 2.945 ms non è incluso nel giro delle 300 richieste, eseguito a modello pronto. In quel giro il massimo di 307,335 ms supera comunque il limite di 250 ms; il test con zero checkpoint non ha risolto il superamento e ha peggiorato il p95, quindi non è stato adottato. Non dichiariamo superato il gate di latenza né attribuiamo la coda massima esclusivamente al risveglio a freddo. I [risultati memoria](../poc/m3/memory-results.json) e il [giro completo da 300 casi](../poc/m3/memory-planner-gate.json) includono anche prove fallite e limiti. Il target RAM dell'intero OS resta da misurare e l'ingresso di produzione deve ancora usare il Neural Bus: la chiusura condizionale non sostituisce queste prove.
