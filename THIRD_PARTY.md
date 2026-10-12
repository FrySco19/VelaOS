# Componenti di terze parti

VelaOS si basa su software esistente. Copyright e licenze dei rispettivi autori restano validi.

## Runtime Neural e anteprima seL4 corrente

I crate Rust M0–M4 e il port M5 sono identificati dai rispettivi `Cargo.toml` e
lockfile. Le loro licenze esplicite prevalgono sul default del repository. Il
port usa Microkit SDK 2.3.0 verificato con SHA-256 e `rust-sel4` alla revisione
`d441a880938209f661201704dfbee7b91c5aa905`.

Il manifest ufficiale Microkit 2.3.0 fissa il kernel seL4 a
`6e7c3b733d296cfd88d5fbf635c96e447a882374` e Microkit a
`8780fab8699f5aeec109b21325ff37c741736b24`. La preview include archivi sorgente
corrispondenti di kernel, Microkit e Rust/seL4 insieme alle immagini e checksum;
conservano licenze e intestazioni originali. Vedere il
[manifest upstream](https://github.com/seL4/microkit-manifest/blob/2.3.0/default.xml)
e [licenze Microkit](https://github.com/seL4/microkit/blob/2.3.0/LICENSE.md).
I componenti non sono tutti sotto una singola licenza Vela: il kernel e i servizi
upstream mantengono i rispettivi copyright/SPDX.

I pesi SLM, gli asset di embedding e i DLL CUDA non sono inclusi nelle immagini
seL4 o negli archivi sorgente Vela. Il bootstrap nativo verifica gli asset
fissati; le condizioni dei rispettivi fornitori restano applicabili.

## Preview desktop storica Debian/KDE

La tabella e le note ISO seguenti descrivono la precedente preview desktop,
non i contenuti dell'attuale loader QEMU Neural/seL4.

| Componente | Origine e note |
| --- | --- |
| Debian 13, kernel e pacchetti | [Debian](https://www.debian.org/); licenze dei singoli pacchetti in `/usr/share/doc/<pacchetto>/copyright` nella ISO |
| KDE Plasma, Frameworks, Discover, Konsole | [KDE](https://kde.org/); GPL/LGPL e altre licenze a seconda del componente |
| Qt/PySide6 | [Qt](https://www.qt.io/); conservare le condizioni della specifica versione distribuita |
| Firefox ESR | [Mozilla](https://www.mozilla.org/firefox/); non viene dichiarata proprietà del marchio Firefox |
| Flatpak/Flathub | [Flatpak](https://flatpak.org/), [Flathub](https://flathub.org/); ciascuna app ha licenza propria e non è inclusa solo perché compare nel catalogo |
| Calamares | [Calamares](https://calamares.io/); configurazione basata sui pacchetti Debian |
| Gemma 4 E2B IT QAT Q4_0 | [Google, modello ufficiale](https://huggingface.co/google/gemma-4-E2B-it-qat-q4_0-gguf); Apache-2.0, revisione e SHA256 in `components/local-ai/assets.json`, licenza inclusa nella ISO |
| llama.cpp | [ggml-org](https://github.com/ggml-org/llama.cpp/tree/b11539); runtime MIT, build e hash fissati nel manifesto, licenza inclusa nella ISO |
| Guest Additions | Pacchetti [Debian Fast Track](https://fasttrack.debian.net/); mantenere le licenze del pacchetto |
| Inter | [Inter](https://github.com/rsms/inter), SIL Open Font License 1.1; font/risorse derivati conservano le relative condizioni |
| Risorse Vela | Generatori e metadati in `components/theme`; vedere LICENSES/README.md |

La ISO include anche pacchetti da `contrib`, `non-free` e `non-free-firmware`: il progetto Vela è aperto, ma non tutti i pacchetti della distribuzione sono sotto GPL. Il manifesto `/live/filesystem.packages` identifica nomi e versioni distribuite. Le rispettive informazioni copyright sono presenti nel filesystem live.

## Sorgenti delle dipendenze

Questo repository contiene i sorgenti originali Vela e i sorgenti dei KCM presenti nell'overlay. Per i pacchetti Debian, ottenere la versione esatta del pacchetto sorgente dal repository di provenienza con `apt-get source`, abilitando le corrispondenti righe `deb-src`; se non più sul mirror corrente, consultare [snapshot.debian.org](https://snapshot.debian.org/). Per Fast Track consultare il suo repository sorgenti. Riferimenti generici non sostituiscono i sorgenti corrispondenti richiesti dalle singole licenze: per distribuzioni successive conservare i pacchetti sorgente e verificare gli obblighi prima di pubblicare binari.

Le licenze originali non devono essere eliminate dalle copie o dalle redistribuzioni. Eventuali materiali di terze parti aggiunti con una pull request devono avere provenienza e condizioni verificabili.
