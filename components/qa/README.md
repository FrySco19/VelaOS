# Verifiche

Strumenti host e VM per test statici, sistema, interazioni e confronto dei flussi. Non è un’applicazione installata nella ISO. Le prove in VM richiedono QEMU/SSH e configurazione dei percorsi locali.

## Verifiche

Vedere [il resoconto pubblico](../../docs/VERIFICATION.md) per risultati riprodotti e limiti. I test con un server simulato non dimostrano inferenza reale. La presenza di un componente nell’overlay non prova tutti i suoi flussi.

## Uso portabile (senza `/workspace/vela`)

La repo root è derivata dalla posizione degli script (override: `QA_REPO`);
le evidenze vanno fuori dalla checkout (`QA_OUT`, default
`~/.local/state/vela-qa/<build>`; override libero). Comandi riprodotti in un
clone fresco, senza VM (solo suite `static`; `system`/`complaints`/`parity`
richiedono QEMU/SSH):

```bash
bash -n components/qa/run.sh components/qa/lib/qa.sh   # syntax check
QA_BUILD=probe bash components/qa/run.sh probe static  # static suite
```

[Build](../../docs/BUILD.md) · [Architettura](../../docs/ARCHITECTURE.md) · [Contribuire](../../CONTRIBUTING.md)
