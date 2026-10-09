# Assistente opzionale

Pannello nativo Python/PySide6/QML, strumenti locali, integrazione D-Bus e KRunner. Usa la configurazione condivisa e un modello scelto dall’utente; nessun modello viene dichiarato operativo solo perché il pannello si apre.

## Struttura

`packages.list` dichiara i pacchetti, quando presente; `overlay/` contiene i file installati; `hook.chroot.sh` prepara il componente durante la build, quando presente. Le directory `tests/` contengono i controlli pertinenti.

## Verifiche

Vedere [il resoconto pubblico](../../docs/VERIFICATION.md) per risultati riprodotti e limiti. I test con un server simulato non dimostrano inferenza reale. La presenza di un componente nell’overlay non prova tutti i suoi flussi.

[Build](../../docs/BUILD.md) · [Architettura](../../docs/ARCHITECTURE.md) · [Contribuire](../../CONTRIBUTING.md)
