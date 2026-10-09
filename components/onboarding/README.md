# Benvenuto e guida

Percorso di primo avvio, impostazioni iniziali, guida, scorciatoie e suggerimenti. STYLE_GUIDE_COPY.md descrive lo stile dei testi. La prima pagina viene caricata dopo l’inizializzazione della finestra Wayland.

## Struttura

`packages.list` dichiara i pacchetti, quando presente; `overlay/` contiene i file installati; `hook.chroot.sh` prepara il componente durante la build, quando presente. Le directory `tests/` contengono i controlli pertinenti.

## Verifiche

Vedere [il resoconto pubblico](../../docs/VERIFICATION.md) per risultati riprodotti e limiti. I test con un server simulato non dimostrano inferenza reale. La presenza di un componente nell’overlay non prova tutti i suoi flussi.

[Build](../../docs/BUILD.md) · [Architettura](../../docs/ARCHITECTURE.md) · [Contribuire](../../CONTRIBUTING.md)
