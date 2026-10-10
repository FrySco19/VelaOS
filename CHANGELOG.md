# Changelog

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
