# Nota di stato corrente — 2026-10-10

Il progetto attivo è l'architettura di un sistema personale AI-native descritta nel [README](README.md). M0 e M1 hanno gate locali registrati come superati nei rispettivi limiti; M2 è in sviluppo. Non esiste ancora una release installabile che integri recall, LFS, Neural Bus e Wasmtime.

La preview Debian 13/KDE Plasma descritta sotto è una release storica del prototipo desktop. Non è l'implementazione né una dimostrazione dei gate M0–M2. Le verifiche e le limitazioni di quella ISO valgono solo per quell'artefatto.

# VelaOS 2.1 — preview desktop storica con assistente locale

Desktop Debian 13/KDE Plasma 6 con Gemma 4 E2B e llama.cpp inclusi. L'assistente parte in locale, senza account o download del modello al primo avvio. Versione di sviluppo, da provare in una VM dedicata con 8 GB di RAM. Accesso SSH di sviluppo disabilitato, senza chiavi predisposte.

Include anteprima/conferma delle operazioni, organizzazione delle cartelle con registro e ripristino, lettura/cambio dell'uscita audio e installazione di VLC, GIMP, LibreOffice e Krita dal catalogo utente Flathub. I comandi espliciti riconosciuti evitano l'inferenza; gli altri passano al modello. Nessun fallback cloud automatico. Il download delle applicazioni necessita di rete.

Le richieste CPU con contesto vuoto possono essere lente. Il benchmark reale, le prove della VM e i test di protocollo sono distinti nel rapporto `docs/VERIFICATION_LOCAL.md`. Il benchmark sintetico non misura intelligenza generale; il modello può chiedere chiarimenti o sbagliare argomenti.

Non sono convalidati installer completo, dual boot, hardware fisico, sospensione, GPU o telefoni. Non è una release stabile per sostituire il sistema principale. La sessione live senza persistenza perde dati e registri allo spegnimento.

Scaricare tutte le parti della ISO, `ISO-PARTS.json` e `join-release.ps1` nella stessa cartella. Lo script riunisce i file e controlla gli hash di ogni parte e della ISO completa. Quindi avviare `Start-VM-Vela2.ps1` con `-MemoryMB 8192`. Sorgenti, licenze, pin del modello e script di build sono pubblici; il modello non entra nella cronologia Git.
