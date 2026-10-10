# Assistente locale integrato

> **Documento della preview desktop Debian/KDE 2.x.** Descrive l'assistente GUI storico, non un planner o un runtime integrato con LFS, Neural Bus o Wasmtime. Per il progetto corrente vedere [README](../README.md).

La nuova integrazione usa Gemma 4 E2B e llama.cpp inclusi nella distribuzione.
Non richiede account, API cloud o download al primo avvio. L'esecuzione CPU è
il riferimento iniziale; il modello viene rilasciato dopo cinque minuti inattivo.
Nome, revisione, licenze e hash sono in `components/local-ai/assets.json`.

## Percorso delle operazioni

```mermaid
flowchart LR
  Request[Richiesta] --> Model[Modello locale]
  Model --> Validate[Strumento registrato e argomenti validati]
  Validate --> Preview[Anteprima]
  Preview --> Confirm[Conferma]
  Confirm --> Execute[Esecuzione]
  Execute --> Verify[Verifica del risultato]
  Verify --> Receipt[Ricevuta e recupero previsto]
```

I comandi semplici già riconosciuti dal sistema evitano l'inferenza. Le richieste
più complesse passano al modello. Una selezione locale propone soltanto gli
strumenti pertinenti al dominio quando riconoscibile, riducendo il contesto sulla
CPU. Questa selezione non autorizza operazioni. Il modello può proporre strumenti ma non usare comandi
shell liberi. Una scelta strutturata può essere sbagliata: l'anteprima e la conferma
restano necessarie. Non vengono inventate percentuali di fiducia.

### Cartelle

`organize_folder` lavora su una cartella esplicita nella Home, senza ricorsione.
Organizza immagini, video, audio e archivi. Documenti, elementi nascosti,
collegamenti e collisioni restano invariati. Limite iniziale: 1000 elementi.
Il piano elenca ogni spostamento prima della conferma. File cambiati nel frattempo
bloccano l'esecuzione; le destinazioni non vengono sovrascritte.

Il registro locale viene scritto prima di ogni modifica. Le operazioni completate
o interrotte riappaiono come ricevute recuperabili all'avvio del controller.
L'annullamento si ferma se trova file cambiati o nuove collisioni. L'interruzione
durante un gruppo di spostamenti lascia disponibili le informazioni per recuperarlo.
Nella sessione live senza persistenza file e registri sono persi allo spegnimento.

### Audio

`diagnose_audio` legge le uscite e l'uscita predefinita tramite il servizio audio.
`set_audio_output` mostra l'uscita proposta, verifica che lo stato non sia cambiato,
richiede conferma e controlla il risultato. È disponibile il ripristino dell'uscita.
Le applicazioni aperte possono mantenere un'uscita propria; l'ascolto richiede una
prova dell'utente. Non si dichiara un guasto risolto soltanto perché un comando termina.

### Catalogo

`install_catalog_app` accetta inizialmente VLC, GIMP, LibreOffice e Krita. Usa
Flathub nel profilo utente, verifica l'origine, mostra i metadati con i permessi,
richiede conferma e controlla la presenza dell'app installata. Il download delle
app necessita di rete: l'inferenza continua a rimanere locale.
Interrompere un'installazione può lasciare runtime o dati parziali; non viene
promesso un annullamento completo dell'installazione.

## Risorse e privacy

- Un'operazione di modifica alla volta per controller.
- Contesto iniziale del server: 4096 token; una generazione per volta.
- API su loopback, senza interfaccia web, con credenziale di sessione creata automaticamente.
- Nessuna lettura continua dello schermo e nessun fallback cloud automatico.
- La build iniziale è per testo e strumenti. Visione, voce e accelerazione mobile
  richiedono integrazioni e prove separate.
- La configurazione può disattivare l'assistente. Il desktop resta utilizzabile.

## Verifiche

Le prove di protocollo con server simulato restano distinte dall'inferenza reale.
`tools/benchmark_local.py` salva ogni risposta, esito e durata. Il campione di
parafrasi sintetiche non sostituisce una prova con utenti né misura intelligenza
generale. I casi senza strumenti richiedono anche una revisione delle risposte.

Per ripetere il benchmark su una distribuzione avviata:

```sh
python3 tools/benchmark_local.py --output benchmark.jsonl
```

Le misure del modello, le prove della ISO e i limiti effettivi della release
saranno raccolti nel rapporto di verifica della release. Il supporto a Gemma su
telefoni non equivale a VelaOS disponibile come sistema operativo mobile.
