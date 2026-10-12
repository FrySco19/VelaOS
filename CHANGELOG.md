# Changelog

## Convergenza nativa e substrato storage — 2026-10-12

- Audit della separazione fra preview x86 Glass, guest seL4/Microkit e runtime host; roadmap nativa senza Linux nel prodotto.
- Aggiunto motore storage Rust `no_std`, senza heap né dipendenze, con due snapshot bounded, flush ordinati e riconciliazione dopo errori I/O.
- Verificati 2.565 tagli di scrittura, perdita della cache non sincronizzata e riapertura di un file reale da un secondo processo; compilazione freestanding x86_64/AArch64 e Clippy superati.
- Il componente non è ancora collegato a un driver guest, agli oggetti immutabili LFS o a Glass. Nessuna ricevuta crittografica o chiusura del gate consumer dichiarata.

## Runtime persistente e port seL4 — 2026-10-11

- Collegata una sessione Rust autenticata a SAVE, card autorizzate, selezione esatta, CONFIRM e KEEP, con ticket privati monouso.
- Verificati undici test M4, inclusa la persistenza dopo la riapertura e il rifiuto di SAVE/recall con permessi scaduti; il test della sessione usa una classifica di fixture, non inferenza M0.
- Passato anche il test separato con vero E5 e LanceDB Vector + BM25 fino a CONFIRM, KEEP e riapertura durevole; aggiunto alla CI dopo bootstrap degli asset verificati. L'interfaccia umana resta aperta.
- Misurato e scartato mmap/512: 3,94 GB di working set attivo. Il default streaming/512 riduce il campione a 589 MB, conservando i risultati di entrambi i profili; commit attivo 4,55 GB ancora non risolto.
- Riallineati planner e cache al massimo richiesto di 250 ms, eliminando l'accettazione incoerente di 400 ms.
- Prefisso del compiler reso invariabile ai cambi di focus; stato strutturato e controllo nativo restano separati dall'output modello. Test HTTP ostile verifica che SAVE senza focus non produca step eseguibili.
- I gestori seL4 rifiutano frame malformati prima degli effetti e propagano risposte inattese come dinieghi. Il boot aggiunge 101 frame PCB errati e prove downstream, separati dal gate di prompt injection.
- Pubblicati i prototipi seL4 dei gate B–G: domini separati, grant IPC, Wasm limitato e flusso fisso con risultato 42. Storage guest ancora volatile.
- Aggiunta CI di build e avvio QEMU con scadenza e marker obbligatori, separata dalla qualificazione delle prestazioni.
- Aggiornati i limiti: UI di produzione, integrazione semantica, budget globale di memoria e port durevole restano aperti. Nessuna release consumer completa dichiarata.

## Architettura AI-native — 2026-10-10

- Reso esplicito il pivot del repository dal prototipo desktop Debian/KDE al sistema personale AI-native basato sui gate M0–M2.
- M0 chiuso sul test sintetico congelato: recall@1 186/200 (93%), warm p95 25,67 ms; qualità di produzione non dimostrata.
- M1: registrato il gate storage Rust su 10.000 terminazioni di processo; i limiti di power-loss e rollback restano aperti.
- M2: aggiunto il boundary Wasmtime con WIT tipizzato, memoria configurabile 16–64 MiB, fuel, massimo due task e linker senza WASI/capability host. Build, test base e Clippy passano in GitHub Actions; integrazione e isolamento avversariale restano aperti.
- La bridge iceoryx2 multi-processo misura 62.222 msg/s e 17 µs p99 round-trip nel test sequenziale da 4 KiB; throughput target e isolamento restano aperti.
- Registrati i target M2 ancora da raggiungere: >100.000 msg/s, p99 <100 µs, stream lento da 10 GB con regressione fast-lane <=20% e budget globale <1,5 GB RAM a riposo.
- Aggiornati README, architettura, roadmap e descrizione GitHub per separare prove, limiti e lavoro in corso.

## 2.0 preview pubblica — 2026-10-09

Storico della precedente preview desktop Debian/KDE; non descrive l'architettura corrente M0–M2.

- Pubblicati componenti nativi Plasma, sorgenti, script di build, test e generatori grafici.
- Documentazione, schermate reali, architettura, roadmap e strumenti per i contributi.
- Ripristino permessi Unix per overlay estratti su Windows.
- Build interrotta per componenti mancanti; manifest e verifica della ISO.
- Risolta la selezione conflittuale dei pacchetti GRUB BIOS/UEFI.
- Corretto il caricamento iniziale del benvenuto in Plasma Wayland.
- Percorsi portabili nei tool principali; stato locale ignorato da Git.
- Preview pubblica derivata dalla R2 con accesso SSH di sviluppo disabilitato.
- Conservato il prototipo storico web in un archivio separato con licenza MIT.

Questa release non introduce un modello preinstallato e non certifica installer o hardware fisico.
