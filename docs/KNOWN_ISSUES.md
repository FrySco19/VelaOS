# Problemi noti e limiti

- Gemma 4 E2B e llama.cpp sono inclusi. Le richieste CPU a contesto vuoto possono essere lente; le richieste con prefisso già elaborato sono distinte nel rapporto. GPU, voce, visione e telefoni non sono collaudati.
- Il modello può sbagliare strumenti o argomenti: anteprima, conferma e verifiche restano necessarie. Il benchmark sintetico non misura intelligenza generale.
- Calamares è configurato, ma installazione su disco e dual boot non sono collaudati.
- Mancano prove su hardware fisico per Wi-Fi, Bluetooth, audio, sospensione, GPU e periferiche.
- Non esiste ancora un percorso convalidato di aggiornamento atomico e ripristino dopo aggiornamenti falliti.
- La latenza CPU dell'assistente è misurata nel rapporto della release; consumi, prestazioni generali e accessibilità richiedono prove dedicate.
- Le funzionalità applicative sono integrate ma non tutte verificate da inizio a fine.
- Gli strumenti QA storici richiedono adattamento dei percorsi e dell'ambiente.
- La build usa mirror correnti; pacchetti, toolchain e asset non sono ancora fissati per riproducibilità bit per bit.
- I KCM precompilati nell'overlay dipendono dall'ABI Debian 13; ricostruirli quando cambia la piattaforma.

Un problema non testato non viene dichiarato risolto. Segnalare regressioni con versione, macchina, passi, risultato atteso e log privi di dati personali.
