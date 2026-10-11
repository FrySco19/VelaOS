# VelaOS

<p align="center"><img src="docs/images/banner.svg" alt="VelaOS: un sistema operativo personale AI-native e local-first" width="100%"></p>

**Stiamo costruendo da zero una nuova architettura per un sistema operativo personale AI-native, local-first e verificabile**, non aggiornando la vecchia distro Debian/KDE. Il richiamo semantico propone candidati; capability e servizi Rust decidono l'autorità; LFS conserva oggetti immutabili; Neural Bus trasporta i byte su corsie tipizzate. M0–M2 sono le fondamenta in sviluppo: non esiste ancora un OS integrato o installabile.

**English:** We are building a new AI-native personal OS architecture from the ground up, not continuing the old Debian/KDE desktop distribution. M0–M2 develop semantic retrieval, capability-secured immutable storage, typed IPC, and isolated Wasm tasks. The project is not yet an integrated or installable OS.

[![M1 Rust checks](https://github.com/FrySco19/VelaOS/actions/workflows/lfs-m1.yml/badge.svg)](https://github.com/FrySco19/VelaOS/actions/workflows/lfs-m1.yml)
[![M2 Rust checks](https://github.com/FrySco19/VelaOS/actions/workflows/neural-bus-m2.yml/badge.svg)](https://github.com/FrySco19/VelaOS/actions/workflows/neural-bus-m2.yml)

## Stato dei gate

| Fase | Stato | Evidenza e limiti |
| --- | --- | --- |
| M0 — richiamo ibrido | Gate superato sul fixture sintetico | 186/200 parafrasi (93% recall@1), p95 25,67 ms. Non misura qualità su dati personali reali. [Risultati e limiti](poc/m0/GATE.md) |
| M1 — storage LFS | Gate crash superato | Kernel Rust con oggetti cifrati, commit firmati e capability Biscuit; gate registrato su 10.000 terminazioni di processo. Non è ancora un servizio di storage distribuito. [Implementazione e validazione](poc/m1/README.md) |
| M2 — isolamento e throughput | Gate completo aperto | Tre test dual-lane tra processi reali superano 100.000 msg/s, p99 <100 µs e regressione concorrente <20% durante 10 GB di byte generati. Wasm reale esegue letture autorizzate e trap avversariali. Restano isolamento dei peer nativi, servizio LFS persistente e file bulk durevoli. [Stato M2](poc/m2/README.md) |
| M3 — intent router | Implementazione e qualificazione in corso | Gemma locale, cinque template vincolati, policy Rust e cache con nuova verifica delle capability. Corpus sintetico congelato; trial falliti conservati. [Protocollo e limiti](poc/m3/README.md) |
| M4 — ripristino | Coordinatore implementato; gate aperto | Journal firmato, conferma vincolata, verifica dello stato e rollback testati su provider in memoria. Provider reale, client, supervisor e percorso end-to-end ancora da integrare. [Stato M4](poc/m4/README.md) |

I numeri descrivono test specifici, non garanzie di prodotto. La vecchia misura M2 da 17 µs è ping/ack sequenziale; i nuovi test dual-lane sono pipelined. Il trasferimento dallo storage al loan IPC effettua una copia. Il target di sistema è meno di 1,5 GB di RAM a riposo, ancora da misurare sull'intero runtime. Il progetto non è pronto per uso come sistema principale.

## Architettura in breve

```mermaid
flowchart LR
  Query[Richiesta] --> M0[M0: recall ibrido]
  M0 -->|candidati, mai autorità| Policy[Selezione esplicita e Biscuit]
  Policy -->|handle esatto| M1[M1: LFS Rust]
  M1 --> M2[M2: Neural Bus iceoryx2]
  Wasm[Componente Wasm tipizzato] -. boundary in sviluppo .-> M2
```

Il modello non concede permessi e non risolve direttamente gli oggetti. Il codice di policy valida le capability e l'handle prima di ogni accesso. Il componente Wasm usa un contratto WIT tipizzato e un linker senza WASI; l'integrazione end-to-end è ancora aperta. Il piano M2 prevede una corsia rapida sotto 4 KiB e streaming bulk a blocchi da 256 KiB con 8 crediti in volo, senza bloccare la corsia rapida. [Architettura e confini](docs/ARCHITECTURE.md) · [Roadmap dei gate](docs/ROADMAP.md) · [Limiti noti](docs/KNOWN_ISSUES.md).

## Codice e riproduzione

| Percorso | Contenuto |
| --- | --- |
| [`poc/m0/`](poc/m0/README.md) | Esperimento di semantic hybrid recall; Python è confinato a questo spike. |
| [`poc/m1/`](poc/m1/README.md) | Kernel storage Rust: canonicalizzazione, cifratura, commit firmati e handle capability-bound. |
| [`poc/m2/`](poc/m2/README.md) | Bridge Neural Bus Rust/iceoryx2 e boundary Wasmtime in sviluppo. |
| [`poc/m3/`](poc/m3/README.md) | Compiler semantico locale, policy dei template, cache e corpus di valutazione. |
| [`poc/m4/`](poc/m4/README.md) | Coordinatore di restore con journal firmato, ticket e rollback. |
| [`archive/`](archive/README.md) | Prototipo storico della shell web. |
| `components/`, `build/` | Sorgenti e pipeline della precedente preview Debian/KDE; non implementano l'architettura M0-M2. |

I controlli usano Rust 1.96.1 e i manifest dei singoli crate. Wasmtime compila ed esegue i test reali anche sull'host Windows di sviluppo. La guida [BUILD.md](docs/BUILD.md) riguarda la vecchia preview desktop e lo dichiara esplicitamente.

## Contribuire

Issue, review di sicurezza, test riproducibili e documentazione dei limiti sono particolarmente utili. Vedi [CONTRIBUTING.md](CONTRIBUTING.md), [Security](SECURITY.md) e [Code of Conduct](CODE_OF_CONDUCT.md). Le licenze del codice e degli artefatti storici sono descritte in [LICENSE](LICENSE), [LICENSES](LICENSES/README.md) e [THIRD_PARTY.md](THIRD_PARTY.md).

## Progetto precedente

La preview Debian 13/KDE Plasma, con assistente desktop locale, resta nel repository come linea storica e non è il sistema costruito dai gate M0-M2. Le relative schermate, note e procedure non costituiscono prove dell'architettura AI-native. Consulta [le note storiche della release](RELEASE_NOTES.md) e la [documentazione desktop](docs/BUILD.md).
