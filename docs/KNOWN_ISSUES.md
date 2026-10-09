# Problemi noti e limiti

- Il modello e il runtime locale non sono preinstallati. L'inferenza reale, il download, l'accelerazione GPU e la qualità delle risposte non sono convalidati.
- Calamares è configurato, ma installazione su disco e dual boot non sono collaudati.
- Mancano prove su hardware fisico per Wi-Fi, Bluetooth, audio, sospensione, GPU e periferiche.
- Non esiste ancora un percorso convalidato di aggiornamento atomico e ripristino dopo aggiornamenti falliti.
- Prestazioni, consumi, latenza e accessibilità richiedono misurazioni e prove dedicate.
- Le funzionalità applicative sono integrate ma non tutte verificate da inizio a fine.
- Gli strumenti QA storici richiedono adattamento dei percorsi e dell'ambiente.
- La build usa mirror correnti; pacchetti, toolchain e asset non sono ancora fissati per riproducibilità bit per bit.
- I KCM precompilati nell'overlay dipendono dall'ABI Debian 13; ricostruirli quando cambia la piattaforma.

Un problema non testato non viene dichiarato risolto. Segnalare regressioni con versione, macchina, passi, risultato atteso e log privi di dati personali.
