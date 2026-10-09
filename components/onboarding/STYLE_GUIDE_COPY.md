# Vela OS 2 — Guida allo stile dei testi (copy)

terminale, installer (Calamares), schermata di accesso, notifiche, file `.desktop`, messaggi d'errore.
Chi scrive testi segue questa guida; onboarding rivede tutto e apre richieste in `coordination/requests.md`.

## 1. Perché esiste
the initial tester ha bocciato la v1 per frasi come «Meno rumore, più possibilità», «Il tuo Porto.»,
«Ogni progetto, il suo respiro.», «Un pensiero. Un punto di partenza.», «Il tuo contesto resta tuo.»,
«Una scrivania calma, uno spazio per ogni progetto.», «A MODO TUO». Il suo commento: «ma cos'è sta roba».
Sono slogan: suonano bene e non dicono niente. Un sistema operativo non fa pubblicità a sé stesso.

**Regola d'oro: ogni testo deve dire cosa c'è o cosa succede, nel modo più breve possibile.**
Se un testo si potrebbe togliere senza perdere informazioni, toglilo.

## 2. Tono
- Semplice, diretto, cortese. Come le interfacce di Apple e Microsoft in italiano.
- Si dà del **tu** («Scegli una lingua», non «Scelga» né «Scegliete»).
- Nessuno slogan, metafora, gioco di parole, frase a effetto, punto esclamativo, emoji.
- Nessun nome di fantasia per le parti del sistema (niente «Porto», «Banchina», «Cima»): si usano i nomi comuni
  del glossario (Dock, Barra in alto, App).
- Non vantarsi («potente», «intelligente», «veloce», «elegante», «rivoluzionario»): mostra la funzione, non l'aggettivo.
- Non scusarsi in modo teatrale («Ops!», «Qualcosa è andato storto :(»). Spiega cosa è successo e cosa fare.

## 3. Forma
| Elemento | Regola | Sì | No |
|---|---|---|---|
| Pulsanti | Verbo all'imperativo, 1–2 parole, nessun punto | Installa, Continua, Annulla, Apri, Salta | OK!, Procediamo, Avanti tutta |
| Titoli di pagina / finestra | Sostantivo o verbo, maiuscola solo all'inizio (sentence case) | Lingua e tastiera | Lingua E Tastiera, LINGUA |
| Descrizioni | Una frase, max ~15 parole, punto finale | Puoi cambiarla dopo in Impostazioni. | Personalizza la tua esperienza come preferisci! |
| Voci di menu | Verbo o sostantivo, niente punto; «…» solo se apre una finestra che chiede altro | Rinomina…, Nuova cartella | Clicca qui per rinominare |
| Campi di testo (suggerimento grigio) | Esempio o cosa scrivere | Cerca app e file | Scrivi qualcosa… di bello |
| Interruttori | Descrivono lo stato «acceso» | Ricorda le pagine visitate | Non dimenticare le pagine |
| Errori | Cosa è successo + cosa fare. Niente codici a schermo (eventualmente in «Dettagli») | Impossibile connettersi alla rete «Casa». Controlla la password. | Error 0x80070005, Operazione fallita |
| Conferme distruttive | Il pulsante ripete l'azione | «Eliminare 3 file?» → Elimina / Annulla | Sì / No |
| Stato vuoto | Cosa manca + come aggiungerlo | Nessuna app installata. Cercane una qui sopra. | Qui è tutto tranquillo… |
| Caricamento | Cosa sta facendo | Caricamento delle app… | Un attimo di magia… |

- Maiuscole: solo prima parola e nomi propri (Vela, Wi-Fi, Bluetooth, Flathub, Ollama). Le parti del sistema nel
  glossario si scrivono come indicato (Dock, Impostazioni, Store, Cestino con maiuscola quando sono nomi di app/luoghi).
- Accenti corretti: è, perché, più, già, può, sarà. Mai «e'», «perche'».
- Apostrofo: usare ' (diritto), coerente con le traduzioni KDE.
- Puntini: carattere «…» (U+2026), non tre punti.
- Virgolette nel testo: « » per citare nomi («Casa»), mai "".
- Numeri e unità: spazio tra numero e unità (4 GB, 60 s), virgola decimale (1,5 GB), ore 24h (20:45).
- Scorciatoie: nomi dei tasti in italiano dove KDE li traduce: Ctrl, Alt, Maiusc, Meta (tasto Windows/Super) — scrivere
  «Meta» e, solo la prima volta in una pagina, «Meta (tasto Windows)». Combinazioni con «+»: Ctrl+C.
- Percorsi nelle impostazioni: « > » con spazi: «Impostazioni > Privacy e memoria». Non «→», «/», «»».
- **Vela è femminile** (la vela): «Installa Vela», «Vela è installata», «Prova Vela senza installarla», «Vela viene copiata sul disco».
- «Internet» con la maiuscola.
- L'assistente può parlare in prima persona solo nei suoi messaggi di stato («Sto rispondendo…»); il resto dell'interfaccia no.
- Lunghezza: i testi italiani sono ~30% più lunghi dell'inglese; lasciare spazio e non troncare con «…».

## 4. Glossario IT / EN
Usare **sempre** questi termini. Se KDE usa un termine diverso nelle sue traduzioni e non possiamo cambiarlo, va bene
(non si toccano i file .po di KDE), ma nei testi Vela si usa la colonna «Vela IT».

| Vela IT | EN | Note / da evitare |
|---|---|---|
| Vela | Vela | Nome del sistema. Mai «VelaOS», «Vela OS 2» nell'interfaccia (solo in Informazioni sul sistema: «Vela 2»). |
| Scrivania | Desktop | La superficie con lo sfondo. Coincide con la cartella `~/Scrivania`. KDE a volte dice «desktop»: accettato nei testi KDE. |
| Spazi | Spaces | I desktop virtuali di KDE. «Spazio 1», «Nuovo spazio». Non «area di lavoro», non «scrivanie». |
| Panoramica | Overview | Vista di tutte le finestre e degli spazi (KWin Overview). |
| Dock | Dock | La barra delle app in basso. Non «Banchina», «Porto». |
| Barra in alto | Top bar | Orologio, menu, area di notifica. Non «Cima». |
| App | Apps | Singolare e plurale invariati: «1 app», «12 app». Non «applicativi», «programmi» (ok «programma» solo nel terminale). |
| Tutte le app | All apps | Il lanciatore / griglia delle app. |
| Ricerca | Search | Funzione; il campo dice «Cerca app, file e impostazioni». Non «Spotlight». |
| Assistente | Assistant | L'IA di sistema. Non «Copilot», «agente», «AI buddy». Per l'IA in generale: «IA» (mai «AI» nei testi italiani). |
| Intelligenza artificiale | Artificial intelligence | Nome della pagina di Impostazioni. |
| Modello locale | Local model | Gira su questo computer (Ollama). |
| Servizio online / API | Online service / API | Modello remoto compatibile OpenAI. «Chiave API» (non «API key», «token»). |
| Impostazioni | Settings | L'app (KDE «Impostazioni di sistema» → nome mostrato «Impostazioni»). Non «Preferenze», «Configurazione». |
| Store | Store | Plasma Discover rinominato. «Installa», «Rimuovi», «Aggiorna». Non «Negozio», «Discover», «Software». |
| Aggiornamenti | Updates | |
| File | Files | Il gestore file (Dolphin). |
| Cestino | Trash | «Sposta nel Cestino», «Svuota Cestino». Non «Spazzatura», «Elimina» quando in realtà sposta. |
| Elimina | Delete | Solo quando cancella definitivamente. |
| Cartella Inizio / Home | Home | Nella barra laterale KDE: «Home». Accettato. |
| Documenti, Download, Immagini, Musica, Video, Scrivania, Modelli, Pubblici | | Nomi xdg-user-dirs italiani. |
| Appunti | Clipboard | «Cronologia degli appunti». Non «clipboard». |
| Copia, Taglia, Incolla, Annulla, Ripeti, Seleziona tutto | Copy, Cut, Paste, Undo, Redo, Select all | «Annulla» = Undo e Cancel (come Apple/KDE). |
| Screenshot | Screenshot | Parola d'uso comune. Non «istantanea», «cattura schermo». App: «Screenshot» (Spectacle). |
| Terminale | Terminal | Il terminale classico. |
| Terminale IA | AI Terminal | Il terminale con assistente. |
| Browser | Browser | |
| Notifiche | Notifications | «Centro notifiche», «Non disturbare». |
| Centro di controllo | Control centre | Pannello rapido Wi-Fi/audio/luminosità. |
| Wi-Fi | Wi-Fi | Sempre con trattino e maiuscole. Non «WiFi», «wifi», «Wireless». |
| Rete | Network | |
| Bluetooth | Bluetooth | |
| Audio | Sound | Non «Suono». |
| Schermo | Display | «Schermi» per la pagina con più monitor. |
| Tastiera | Keyboard | «Layout di tastiera» → «Lingua della tastiera» nei testi Vela (più chiaro). |
| Lingua e area geografica | Language & region | |
| Aspetto | Appearance | «Chiaro», «Scuro», «Automatico», «Colore principale» (accent). Non «accento». La pagina di Impostazioni si chiama «Aspetto» (settings-store): nei rimandi scrivere «Impostazioni > Aspetto». |
| Sfondo | Wallpaper | Non «carta da parati», «wallpaper». |
| Accessibilità | Accessibility | «Testo più grande», «Contrasto elevato», «Riduci animazioni», «Lettore schermo» (Orca). |
| Privacy | Privacy | Parola comune in italiano. Pagina: «Privacy e memoria». |
| Memoria (dell'assistente) | Memory | Indice personale. Specificare «memoria dell'assistente» se ambiguo con la RAM. |
| Cronologia delle azioni | Action log | Registro delle azioni dell'IA. Non «log». |
| Istantanee di sistema / Ripristina | System snapshots / Restore | Snapshot Btrfs (app «Istantanee di sistema»). «Istantanea» al singolare. |
| Accedi, Esci, Blocca, Sospendi, Riavvia, Spegni | Sign in, Log out, Lock, Sleep, Restart, Shut down | Non «Logout», «Arresta il sistema». |
| Utenti | Users | «Account» per account online. |
| Installa Vela | Install Vela | Pulsante nella sessione live. Installer: «Installazione di Vela». |
| Prova Vela | Try Vela | Sessione live. |
| Guida e scorciatoie | Help & shortcuts | Pagina di aiuto. |
| Suggerimenti | Tips | Notifiche dei primi giorni. |
| Benvenuto | Welcome | L'app del primo avvio. |

## 5. Esempi: prima → dopo
| v1 / da evitare | Vela 2 |
|---|---|
| Meno rumore, più possibilità | *(eliminare: nessun testo)* |
| Il tuo Porto. | Tutte le app |
| Ogni progetto, il suo respiro. | Spazi: tieni separate le finestre di ogni progetto. |
| Un pensiero. Un punto di partenza. | Cerca app, file e impostazioni |
| Il tuo contesto resta tuo. | Cosa può vedere l'assistente |
| Una scrivania calma, uno spazio per ogni progetto. | *(eliminare)* |
| A MODO TUO | Aspetto |
| Benvenuto nel futuro del tuo computer! | Benvenuto in Vela |
| Scopri la potenza dell'IA | Configura l'assistente |
| Ops! Qualcosa è andato storto. | Impossibile salvare il file. Lo spazio su disco è esaurito. |
| Caricamento della magia… | Caricamento… |

## 6. Lista di controllo per chi scrive un testo
1. Dice cosa c'è o cosa succede? Se no, riscrivilo o toglilo.
2. È la forma più breve che resta chiara?
3. Usa i termini del glossario?
4. Il pulsante è un verbo che descrive l'azione?
5. Nessun inglese se esiste la parola italiana del glossario? (Eccezioni: Wi-Fi, Bluetooth, Store, Dock, Browser, App, Screenshot, Download, Privacy, API.)
6. Nessuna frase in `components/qa/lib/banned_phrases.txt`?
7. La versione inglese (se l'app la offre) è altrettanto semplice?

## 7. Inglese
Quando l'utente sceglie English: stesso tono (Apple/Microsoft US English), sentence case, «you», nessuno slogan.
Ogni componente Vela che mostra testi deve avere entrambe le lingue e scegliere in base a `LANGUAGE`/`LANG`
(o `language` in `ai.json` per le app IA). Italiano è il predefinito.
