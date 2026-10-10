# Limiti noti dell'architettura M0–M2

Questa pagina descrive il sistema personale AI-native in sviluppo. La precedente preview Debian/KDE ha un diverso insieme di limiti documentato nelle [note storiche](../RELEASE_NOTES.md) e nelle [verifiche desktop](VERIFICATION.md).

- **M0:** il gate è passato su cinque card sintetiche e 200 parafrasi curate dagli autori; non dimostra qualità su corpus reali, lingue non coperte o query indipendenti.
- **M1:** il gate di 10.000 terminazioni di processo è passato, ma non simula perdita improvvisa di alimentazione. Manca un anchor esterno anti-rollback; il rebuild del catalogo cresce linearmente.
- **M2 throughput:** la misura corrente è sequenziale ping/ack da 4 KiB (62.222 msg/s, p99 round-trip 17 µs); non è throughput pipelined e non supera il target >100.000 msg/s.
- **M2 isolamento:** il fixture gira con processi nativi fidati sotto lo stesso account Windows. Identità del peer IPC, isolamento OS, replay protection e isolamento Wasmtime non sono dimostrati.
- **Wasm:** il modulo sandbox compila e i test base passano nella CI Windows; non sono ancora stati eseguiti test avversariali completi. Non è integrato nel bus o in un servizio di task.
- **Bus bulk:** non sono implementati streaming con crediti/backpressure, comportamento slow-reader, cancellazione robusta o lane indipendente per oggetti MB/GB.
- **Writer massivo:** manca il writer temporaneo con cifratura chunked, verifica progressiva e commit del BLAKE3 canonico.
- **Sistema integrato:** semantic index persistente in Rust, provisioning delle chiavi, service supervision, recovery e client utente non sono disponibili come percorso end-to-end.
- **Budget RAM:** l'obiettivo è meno di 1,5 GB per l'intero sistema a riposo. Non è ancora misurato sull'architettura corrente; i vecchi benchmark della preview desktop non sono un sostituto.

Un requisito senza test non viene dichiarato risolto. Segnalare regressioni con commit, sistema operativo, comando riprodotto, risultato atteso e log privi di dati personali.
