# Banchina — stato e verifica, 8 ottobre 2026

Questa consegna implementa la nuova esperienza della shell e parte degli interventi di sicurezza e file manager. È un'anteprima live, non Vela OS 1.0. Non è stata pubblicata online.

## Identità dell'immagine

Vela-Banchina-preview.iso, 1.351.352.320 byte, volume VELA_PREVIEW.

SHA256: `726d14a9a838423065dbe344c490447b8ec4acdb71c98ed5e38e3a0be69ceb02`.

JavaScript nella ISO e nel sorgente: SHA256 `e96aa1ae5047fe61ff0aeffcf245ab79334bf7fc7b5a0d17a63f937421e453c2`. Confrontato tramite estrazione SquashFS. Licenze Vela e note terze parti presenti in /vela-info sul supporto; licenze xterm.js e addon-fit dentro la shell.

## Modifiche

- Cima: menu dell'app attiva; menu nascosti sul desktop vuoto; spazi al centro; orologio spostato in basso. Rimane un menu per le app interne, non un menu globale delle app native.
- Banchina larga: Porto, icone appoggiate, punto corallo, pulsanti con titoli e indicazione della finestra attiva. Anteprime schematiche al passaggio.
- Porto: ricerca delle app disponibili, griglia e lista, informazioni e alimentazione. Browser, Terminale IA e controlli hardware non collegati rimossi. Assistente mostrato solo se il servizio rileva un modello Ollama.
- Finestre: area pallini 24 px e glifi al passaggio, controlli spostabili a destra, geometria ripristinata dopo ingrandimento, bordi per ridimensionare, layout sul verde e snap ai bordi. Alt+Tab nella shell, Alt+F4 per chiudere. Le interazioni mouse con bordi/layout/drag non sono state tutte collaudate in VM.
- Desktop: saluto attenuato quando ci sono finestre. Spazi: miniature 16:10 ridisegnate, tinta distinta, creazione e spostamento finestre tramite drag o comando. Non sono anteprime live dei contenuti e non sono persistenti.
- Terminale: fish su PTY Linux dentro la finestra Vela, con dock, tema, snap e spazi. Eliminato lo stacco visivo di xterm/Openbox. Non è il compositore Wayland finale.
- File: tre cartelle reali e sottocartelle, briciole, crea/rinomina/duplica/cestino reversibile, viste icone/elenco/layout colonne, ricerca nel percorso aperto, anteprima testo e immagini; supporto PDF implementato, non certificato visivamente.
- Tema: cambio dalla ricerca aggiorna anche il selettore delle Impostazioni aperte. Corretto anche Invio che poteva riaprire la ricerca dopo il comando.
- Impostazioni: pagine Aspetto, Accessibilità, Privacy e Informazioni effettive. Preferenze salvate nel profilo della sessione, non su disco persistente.
- Servizio: cookie di sessione casuale HttpOnly/SameSite=Strict, Host e Origin verificati. Non isola da processi locali dello stesso utente. Azioni file limitate alle cartelle abilitate.
- Build: SquashFS zstd livello 10; boot noeject; licenza MIT del progetto e note terze parti. Script shell normalizzati LF e .gitattributes. Build cold e Docker ancora non certificati.

## VM — nuova ISO esatta

VirtualBox 7.2.20, 4 GB RAM, 2 CPU, VMSVGA 128 MB, 3D disattivato, NAT, nessun disco dati o cartella condivisa. Schermate finali a 1920×1014, risoluzione della finestra VM osservata. Guest Additions operativi per autenticazione e Guest Control; clipboard e test sistematico di resize ancora non certificati.

| Prova | Risultato | Evidenza in evidence-banchina/ |
|---|---|---|
| Avvio EFI automatico | 37.57 s dal kernel alla shell | boot-efi.json, efi-desktop.png |
| Avvio BIOS automatico | 32.10 s dal kernel alla shell | boot-bios.json, bios-desktop.png |
| RAM EFI a riposo | 651.39 MiB, uptime 62.26 s | idle-efi.json |
| RAM con 10 finestre, incluso terminale | 779.55 MiB, uptime 187.94 s | ten-windows.json, ten-windows.png, ten-window-list.png |
| Terminale reale | pwd restituisce /home/vela/Documents; echo Vela restituisce Vela | terminal-final.png |
| Snap metà sinistra/destra | terminale e Documenti affiancati | snap-terminal-left.png, banchina-dual.png |
| Alt+Tab | striscia della shell, rilascio Alt attiva la finestra selezionata | alt-tab.png |
| F3 e cambio Studio/Personale | spazi e finestre ripristinati | spaces.png, studio.png |
| Porto da tastiera | griglia/lista e alimentazione | porto.png |
| Spegnimento EFI via servizio | VM PoweredOff senza controlvm poweroff | poweroff-efi-ack.json, poweroff-efi-state.txt, efi-vbox.log |
| Spegnimento BIOS dal Porto | dialogo → Conferma → VM PoweredOff senza forzature | poweroff-confirm.png, poweroff-bios-state.txt, bios-vbox.log |

Entrambe le righe di boot contengono noeject: efi-cmdline.txt e bios-cmdline.txt. Dopo lo spegnimento EFI Guest Control segnala l'errore di chiusura sessione perché il guest è già spento; stato VM e log confermano l'esito. Non è stato eseguito un audit completo del filesystem durante lo smontaggio. Nessun registratore VirtualBox attivato.

Un avvio per firmware; non sono distribuzioni statistiche. Il tempo usa /proc/uptime dopo due frame della shell ed esclude il firmware. RAM = MemTotal − MemAvailable del guest. Risoluzione e carico host differiscono dai test precedenti, quindi non attribuire il miglioramento a una singola modifica. Target 15 s non raggiunto; 60 fps e latenza <100 ms non misurati.

## Browser e servizio

Nel browser Windows su una home di prova isolata in work/demo-home:

1. Porto → Documenti → nuova cartella → rinomina → cestino → Annulla: elementi reali verificati.
2. Ingrandisci → ripristina: recuperate posizione 16vw/9vh e dimensioni originali, senza salto in alto a sinistra.
3. Scuro → ricerca attiva tema chiaro: tema chiaro, selettore light selezionato, ricerca chiusa. Stato DOM osservato.
4. Riduci movimento → ricaricamento: preferenza mantenuta. Non prova persistenza al reboot live.
5. Nuovo spazio → Progetto → Invio; ricerca a sinistra → F3; anteprima PNG reale: verificati.

Sessione simulata da agente che conosce il codice; non uno studio con utenti nuovi. Tempi dei singoli compiti non misurati.

Verifica automatica del servizio con home temporanea: rifiuto richieste senza cookie, Host/Origin esterni e risalita percorsi; crea, rinomina, duplica, anteprima testo, cestino e ripristino. Esito PASS in service-test.log. Non è una scansione di sicurezza completa. PDF, drag mouse, snap dai bordi, lettore di schermo, clipboard e multi-monitor restano da verificare.

## Prossimi blocchi

Installer, home persistente, password scelta dall'utente, onboarding lingua/tastiera, browser reale, rete/audio, gestione privilegi per installazione, compositor Wayland, snapshot e aggiornamenti. La pagina di presentazione è preparata localmente; nessuna pubblicazione eseguita.
