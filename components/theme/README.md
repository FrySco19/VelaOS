# Identità visiva

Token di colore, temi Plasma chiari e scuri, look-and-feel, icone, sfondi, decorazioni, Plymouth e menu di avvio. I generatori in src/ producono le risorse incluse nell’overlay; mantenere i metadati e le licenze delle risorse.

## Struttura

`packages.list` dichiara i pacchetti, quando presente; `overlay/` contiene i file installati; `hook.chroot.sh` prepara il componente durante la build, quando presente. Le directory `tests/` contengono i controlli pertinenti.

## Verifiche

Vedere [il resoconto pubblico](../../docs/VERIFICATION.md) per risultati riprodotti e limiti. I test con un server simulato non dimostrano inferenza reale. La presenza di un componente nell’overlay non prova tutti i suoi flussi.

[Build](../../docs/BUILD.md) · [Architettura](../../docs/ARCHITECTURE.md) · [Contribuire](../../CONTRIBUTING.md)
