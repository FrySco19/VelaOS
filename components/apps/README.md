# Applicazioni e browser

Pacchetti applicativi, estensione Firefox, native host per il protocollo dell’assistente e gestione degli spazi. L’accesso al contenuto delle pagine segue i permessi del browser. Il modello non è preinstallato.

## Struttura

`packages.list` dichiara i pacchetti, quando presente; `overlay/` contiene i file installati; `hook.chroot.sh` prepara il componente durante la build, quando presente. Le directory `tests/` contengono i controlli pertinenti.

## Verifiche

Vedere [il resoconto pubblico](../../docs/VERIFICATION.md) per risultati riprodotti e limiti. I test con un server simulato non dimostrano inferenza reale. La presenza di un componente nell’overlay non prova tutti i suoi flussi.

[Build](../../docs/BUILD.md) · [Architettura](../../docs/ARCHITECTURE.md) · [Contribuire](../../CONTRIBUTING.md)
