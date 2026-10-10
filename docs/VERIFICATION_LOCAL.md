# Verifiche della preview 2.1 — 10 ottobre 2026

> **Verifica storica della preview desktop Debian/KDE.** I test del modello qui descritti non sono il gate semantico M0 né un test del runtime Wasmtime. Per lo stato dell'architettura corrente vedere [README](../README.md).

## Modello reale, distinto dai test simulati

Gemma 4 E2B IT QAT Q4_0, revisione e SHA256 nel
[manifesto degli input](../components/local-ai/assets.json), llama.cpp `b11539`.
Riferimento CPU: AMD Ryzen 5 2600X (6 core / 12 thread), host con circa 16 GB RAM,
Debian 13 in WSL2, server con 4 thread e contesto 4096. La RTX 2060 dell'host
non è stata usata. Cache del prefisso attiva; nessuna azione eseguita dal benchmark.

**113/120 casi corretti (94.2%).**
Mediana 3.1 s, 95º percentile 13.6 s,
prima richiesta della sessione 34.9 s.
Sono 10 scenari con 12 parafrasi sintetiche ciascuno: non utenti reali,
non una misura di intelligenza generale e non un confronto con altri modelli.
Le richieste a contesto vuoto hanno raggiunto circa 72 secondi; la mediana
non promette che ogni richiesta risponda in 3 secondi.

| Scenario | Corretti | Mediana |
| --- | --- | --- |
| `organize_folder` | 9/12 | 3.1 s |
| `diagnose_audio` | 8/12 | 2.7 s |
| `set_volume` | 12/12 | 2.6 s |
| `install_catalog_app:vlc` | 12/12 | 3.1 s |
| `install_catalog_app:gimp` | 12/12 | 3.1 s |
| `open_settings` | 12/12 | 2.7 s |
| `create_folder` | 12/12 | 2.9 s |
| `no_tool` | 36/36 | 8.6 s |

Nei 84 casi che richiedevano un'azione, 77 hanno prodotto lo strumento e gli
argomenti attesi. I sette errori sono sei richieste di consenso aggiuntive e una
risposta che indicava `diagnose_audio` invece di chiamarlo. Non sono stati osservati
argomenti sbagliati nelle chiamate prodotte in questa sessione. Nei 36 casi senza
strumenti, nessuna chiamata è stata prodotta; sono richiesti chiarimenti per la
cartella non specificata e rifiuti per app fuori catalogo/comandi liberi.
Questo campione non dimostra che il modello sia immune da errori o prompt injection.

La revisione delle risposte negative ha rilevato due risposte (111 e 112) che,
pur rifiutando il comando distruttivo e senza produrre chiamate, menzionavano
erroneamente la disponibilità di `run_command`. Il punteggio misura strumenti
e argomenti, non la correttezza di ogni frase. Il backend blocca comunque
questo strumento nel pannello assistente.

Il percorso diretto dei comandi espliciti riconosciuti dal pannello è provato
separatamente e non aumenta artificialmente il punteggio del modello.
[Risposte e tempi di tutti i casi](reports/benchmark-gemma4-e2b.jsonl),
[riepilogo leggibile](reports/benchmark-gemma4-e2b.summary.json).

## Prove del software

| Verifica | Esito | Limite |
| --- | --- | --- |
| Terminale | 402 passati, 3 saltati | Ambiente Debian e locale italiano |
| Assistente | 115 passati | Protocolli simulati e fixture reali, distinti dall'inferenza |
| Browser/native host | 13 controlli passati | Server modello simulato |
| Spostamenti | Anteprima, conferma, byte ripristinati, nuovo controller | Cartelle di prova nella Home |
| Interruzione improvvisa | Processo terminato fra creazione del collegamento e rimozione del file; recupero riuscito | Filesystem locale di prova |
| Cancellazione | Processo reale interrotto; rifiuto se annullato prima dell'avvio | Non prova ogni backend esterno |
| Permessi revocati | Proposte bloccate alla conferma | Test del controller |
| Audio in VM | Stato letto, uscita cambiata e ripristinata | Uscita temporanea; ascolto non verificato |
| Catalogo in VM | VLC installato dopo controllo origine e permessi | Base 2.0 con strumenti aggiornati; rete necessaria |
| KCM | Quattro moduli ricompilati dai sorgenti | ABI Debian 13 |

L'installazione VLC ha richiesto una ripresa dopo il limite iniziale di 600 secondi;
il limite dell'operazione è stato portato a 1800 secondi, con interruzione disponibile.
Il risultato finale è stato verificato tramite Flatpak.

La riduzione degli strumenti pertinenti al dominio ha abbassato il contesto di
una richiesta cartella da 1696 a 768 token nelle prove iniziali: elaborazione del
prompt da 65,3 a 29,7 secondi. Le istruzioni sono state successivamente ampliate;
i tempi finali sono quelli del benchmark sopra. Il server ha rilasciato il modello
dopo cinque minuti inattivo: RSS osservata circa 170 MiB, contro circa 2,9 GiB
con modello caricato. RSS non equivale al consumo totale dell'intero sistema.

## Collaudo della ISO finale

ISO `VelaOS-2.1-local-preview.iso`, 5 295 636 480 byte.
SHA256: `5be8cd425182c55ba21c992d87089bedba533a81b3c7e04da723a3c326ca496f`.
La copia Windows e la suddivisione in tre parti producono lo stesso checksum.

Avvio UEFI/Plasma verificato in una VM dedicata, 8192 MB RAM e 4 CPU, con cavo
di rete scollegato prima dell'avvio. Il servizio locale è abilitato e parte
automaticamente; modello e runtime sono già presenti. Flathub è configurato
senza download preliminare. Nessun ascolto SSH TCP rilevato.

La prova sul modello reale ha verificato hash dei pesi, credenziale 0600,
rifiuto HTTP 401 senza credenziale, proposta su una cartella temporanea,
assenza di modifiche prima della conferma, spostamento confermato e ripristino
dei byte originali da un nuovo controller. [Risultato grezzo](reports/iso-offline-session.json).
La proposta ha richiesto 147.3 s includendo l'attesa di una richiesta audio
contemporanea. La prima richiesta audio nel pannello ha richiesto circa 85.6 s
di elaborazione (796 token di ingresso, 10 di uscita). Il caricamento iniziale
del servizio ha richiesto circa 83 s in questa sessione live compressa.
Questi tempi della VM sono distinti dal benchmark WSL con prefisso in cache.

La diagnosi audio, il cambio a un'uscita temporanea PipeWire e il ripristino
dell'uscita originale sono stati verificati nella nuova ISO. L'udibilità
effettiva dell'audio non è stata verificata. [Schermate della nuova ISO](SCREENSHOTS.md).
L'installazione dal catalogo richiede rete e viene collaudata separatamente.

Dopo la prova offline è stata ricollegata la rete della VM. Il percorso del
catalogo incluso nella nuova ISO ha verificato origine e permessi, installato
`org.videolan.VLC` per l'utente e verificato il risultato tramite Flatpak.
La configurazione Flathub già presente non ha richiesto interventi manuali.
L'avvio di VLC ha mostrato la sua finestra iniziale di privacy; riproduzione
multimediale e udibilità non sono state collaudate.
[Risultato dell'installazione](reports/iso-catalog-session.json).

## Ripetere

```sh
LANG=it_IT.UTF-8 python3 -m pytest -q components/terminal/tests
python3 -m pytest -q components/assistant/tests
PYTHONPATH="$PWD/components/settings-store/overlay/usr/lib/vela/python" python3 components/apps/tests/test_host.py
python3 tools/benchmark_local.py --output benchmark.jsonl
```

Su una sessione live dedicata, con rete della VM scollegata:

```sh
python3 tools/verify_local_session.py
```

Quest'ultima prova crea soltanto una cartella temporanea nella Home, chiede una
proposta al modello reale, conferma gli spostamenti su quei file, riavvia il
controller e ripristina i byte originali. Verifica inoltre hash del modello,
credenziale di sessione e rifiuto dell'inferenza senza autenticazione.

Installer, dual boot, aggiornamenti/ripristino del sistema, hardware fisico,
GPU, accessibilità e telefoni restano da collaudare. VelaOS resta una preview.
