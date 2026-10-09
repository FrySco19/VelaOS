# Verifiche — 9 ottobre 2026

Queste verifiche riguardano la build integrata R2 e i sorgenti da cui deriva la preview pubblica. La preparazione della preview pubblica rimuove l'accesso SSH di sviluppo e aggiorna la documentazione; la sua verifica aggiuntiva è descritta nelle note della release.

| Prova riprodotta | Risultato | Limite |
| --- | --- | --- |
| Terminale | 401 passati, 4 saltati | Ambiente Debian, locale italiano; dipendenze facoltative assenti |
| Assistente | 52 passati | Server modello simulato |
| Browser/native host | 13 controlli passati | Server modello simulato; script autonomo, non suite pytest |
| Controlli statici dei componenti | PASS | Non equivalgono a un audit di sicurezza |
| Manifest e quattro KCM nel filesystem ISO | Presenti | Presenza non implica copertura di tutti i flussi |
| VirtualBox 7.2.20 | Avvio Plasma Wayland | Non prova compatibilità hardware fisico |
| Benvenuto R2 | Prima pagina visibile al primo avvio | Altri percorsi di onboarding da ampliare |
| Impostazioni Vela | Quattro pagine aperte | Non tutti i controlli eseguiti |
| Tema | Cambio chiaro/scuro osservato | Nessuna misura di fluidità |
| Store | Catalogo Flathub caricato | Installazione di ogni app non testata |
| Guest Additions | Rilevate, resize 1440×900 | Appunti e file host/guest non testati |
| Esecutore installato | Ping, rifiuto senza conferma, `printf VELA_INTEGRATION_OK` eseguito dopo token | Prova di protocollo reale, senza modello |
| Servizi utente R2 | Nessun servizio fallito a fine collaudo | Snapshot di un singolo ambiente |

Le schermate pubblicate sono prove visive del collaudo in VM. I registri originali con percorsi privati della macchina non sono distribuiti.

## Riprodurre i test di protocollo

```sh
sudo apt install python3-pytest locales
# Assicurarsi che it_IT.UTF-8 sia generato tramite sudo dpkg-reconfigure locales.
LANG=it_IT.UTF-8 python3 -m pytest -q components/terminal/tests
python3 -m pytest -q components/assistant/tests
PYTHONPATH="$PWD/components/settings-store/overlay/usr/lib/vela/python" python3 components/apps/tests/test_host.py
```

La CI esegue queste suite con server simulati e controlli di sintassi. Non avvia un desktop né installa una ISO. La build completa, installer, hardware fisico e inferenza reale devono avere verifiche separate prima di una release stabile.
