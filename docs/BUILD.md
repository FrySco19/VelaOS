# Build e riproduzione

> Le istruzioni Debian/live-build qui sotto ricostruiscono la precedente preview desktop VelaOS 2. Non costruiscono l'architettura AI-native M0–M2 e non producono una release corrente. Per il progetto attivo, vedere i README di [M0](../poc/m0/README.md), [M1](../poc/m1/README.md) e [M2](../poc/m2/README.md).

## Prototipi correnti M0–M2

M1 e M2 sono crate Rust indipendenti. Dal checkout, con Rust 1.90 o successivo:

```powershell
cargo test --manifest-path poc/m1/Cargo.toml --locked
cargo test --manifest-path poc/m2/Cargo.toml --all-features --locked
cargo clippy --manifest-path poc/m2/Cargo.toml --all-features --all-targets --locked -- -D warnings
```

M0 è uno spike Python con ambiente e modello fissati, descritto in `poc/m0/README.md`. Il build Wasmtime in M2 non è stato ancora confermato localmente al 2026-10-10; un comando di test non è prova di successo finché non termina e restituisce exit code zero.

## Preview desktop storica

Queste istruzioni ricostruiscono la linea Debian 13/KDE Plasma conservata nel repository per riferimento.

## Ambiente

Debian 13 amd64, accesso root/sudo, connessione ai mirror Debian, Fast Track, Hugging Face e GitHub, almeno 40 GB liberi consigliati. Usare un filesystem Linux (ext4 anche in WSL), non compilare direttamente su una cartella NTFS montata. Il risultato dipende dalle versioni correnti dei mirror: non è ancora una build riproducibile bit per bit.

```sh
sudo apt update
sudo apt install git python3 live-build debootstrap squashfs-tools xorriso isolinux syslinux-common grub-pc-bin grub-efi-amd64-bin mtools dosfstools rsync sudo
git clone https://github.com/FrySco19/VelaOS.git
cd VelaOS
sudo sh build/build.sh
```

Il risultato è in `build/out/`, con checksum SHA256. La build controlla componenti, launcher e moduli nativi nel filesystem compresso. `--release` aumenta la compressione; non certifica la stabilità del sistema.

La build prepara automaticamente Gemma 4 E2B e llama.cpp, controllando gli hash
in `components/local-ai/assets.json`. Gli input scaricati sono conservati in
`.local/ai-cache/`; il modello e il runtime non entrano nella cronologia Git.
Per preparare la cache separatamente: `python3 build/prepare-local-ai.py`.
La ISO include già questi file e non scarica il modello al primo avvio.

Le ISO oltre il limite per singolo asset GitHub vengono pubblicate in parti.
`python3 tools/split-release.py percorso.iso cartella-release` crea
parti e manifest; `tools/join-release.ps1` le riunisce su Windows verificando
ogni checksum e quello finale. Non sovrascrive una ISO esistente.

## Ricostruire i moduli nativi

L'overlay include quattro KCM precompilati per l'ABI Debian 13. Sono inclusi anche i sorgenti C++/QML. Per ricostruirli prima della ISO:

```sh
sudo apt install cmake extra-cmake-modules g++ qt6-base-dev qt6-declarative-dev libkf6kcmutils-dev libkf6i18n-dev libkf6coreaddons-dev gettext python3-polib
sh components/settings-store/build-kcm.sh
```

Per i generatori grafici installare le dipendenze dichiarate nei relativi script (Python, Pillow, NumPy, librsvg, font Inter e strumenti GRUB). Le risorse generate sono già incluse; non serve rigenerarle per una prima build.

## Build riservata agli sviluppatori

```sh
sudo sh build/build.sh --test
```

`--test` aggiunge SSH, autenticazione di sviluppo e una chiave locale. Non distribuire questa ISO come release pubblica. Le chiavi sono generate in `.local/ssh/` e ignorate da Git. Una release deve partire da una build pulita senza `--test`; `quick.sh --no-test` non rimuove tutti gli elementi aggiunti da una build test.

`tools/devvm.sh create` installa su un disco virtuale dedicato e sostituisce l'eventuale disco di sviluppo precedente. `tools/devvm-install.sh` è distruttivo sul dispositivo indicato: usare soltanto una VM sacrificabile. Non invocarlo su un disco fisico.

## Ripresa di build e stato locale

Le build usano `coordination/build.lock`. Output, cache, VM e chiavi non sono versionati. `VELA_BUILD_DIR`, `VELA_OUT`, `VELA_SSH_KEY` e `VELA_RUN` permettono di scegliere altri percorsi. Una ripresa con `lb build` può ripristinare la cache bootstrap e sovrascrivere un chroot già modificato: diagnosticare la fase fallita o ripartire da zero; non considerare i vecchi marker prova di integrazione.

Gli strumenti QA storici contengono ancora default `/workspace/vela/...`: configurarli per il proprio ambiente. Sono inclusi per continuità e non sono tutti coperti dalla CI. I percorsi correnti dei tool principali sono locali al checkout.
