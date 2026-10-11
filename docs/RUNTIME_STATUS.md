# Stato di implementazione — 11 ottobre 2026

Il runtime Rust M2–M4 è pubblicato nella PR di sviluppo. M4 resta aperto: non esiste ancora un sistema completo o installabile. Il checkout storico e la VM precedente non sono stati usati come prova della nuova architettura.

| Verifica | Risultato misurato | Confine |
| --- | --- | --- |
| Wasm, capability e streaming | 12 test M2 superati | Componenti Wasm reali; peer nativi fidati sull'host di sviluppo |
| Router e cache | 2 test M3 superati | Template finiti; capability ricontrollate a ogni cache hit |
| Restore e recovery | 4 test M4 superati | Storage reale e journal firmato; provider di rete in memoria |
| Tre benchmark dual-lane | >100.000 msg/s, p99 <100 µs, regressione <20% | 10 GB generati, non file LFS bulk durevoli |
| Planner senza route cache, 20 ripetizioni | 6.000 output validi; router 98,33–98,67%; massimo 246,959 ms | Stabilità fallita su una richiesta ambigua; modello da solo 92,33–92,67% |
| Router con cache, 20 ripetizioni | 98,67%, piani identici; massimo cold 171,351 ms, warm 0,524 ms | 300 chiamate modello + 5.700 cache hit entro TTL; non prova stabilità dopo scadenza |
| SLM locale pronto | Working set circa 638 MB nell'ultima lettura | Memoria privata impegnata circa 4,70 GB; non è la RAM dell'intero OS |

I test e Clippy con warning negati passano localmente. I controlli CI non eseguono il modello GPU né qualificano le prestazioni del sistema completo.

## Lavoro necessario per arrivare alla fine di M4

1. Portare l'ingresso del compiler e gli accessi LFS su servizi Neural Bus persistenti con identità protette. I template di piano non devono trasportare byte degli oggetti.
2. Collegare il richiamo ibrido M0 al servizio Rust e alla selezione esplicita di OID/handle, conservando gli epoch fissati e filtrando per capability.
3. Implementare il provider reale in un dominio di prova isolato, con verifica degli effetti e gestione delle applicazioni parziali. Una scrittura di due proprietà di rete non va dichiarata atomicamente applicata senza prova del provider.
4. Collegare un client autenticato per la conferma e un supervisor persistente per il timer e il recovery. Predisporre un'autorità di rollback attenuata e legata agli oggetti esatti, utilizzabile anche se il permesso utente iniziale scade.
5. Eseguire SAVE → RECALL → selezione esatta → CONFIRM → APPLY → KEEP/rollback, i 100 casi di injection end-to-end e i guasti dei servizi reali. Le regressioni del parser non sostituiscono queste prove.
6. Misurare RAM a riposo e prestazioni sull'intero runtime. Il target <1,5 GB non può essere ricavato dal solo working set del modello o ignorando la memoria impegnata.

M5 segue il gate M4 e riguarda il port seL4/QEMU. Nessuna chiusura di M4 o avanzamento ufficiale a M5 è dichiarato da questo documento.
