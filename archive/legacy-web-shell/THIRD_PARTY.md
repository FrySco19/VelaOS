# Terze parti

La licenza MIT di Vela copre la shell, il servizio e le ricette originali. Non cambia la licenza dei pacchetti Debian contenuti nella ISO.

- Debian 13, kernel Linux, systemd, Chromium, Xorg, Openbox, Python, fish e altri pacchetti: licenze individuali in /usr/share/doc/<pacchetto>/copyright dentro la ISO. Inventario della prima build in evidence/packages.txt. I sorgenti dei pacchetti Debian sono disponibili tramite gli archivi sorgente Debian; la build non include una copia completa di tutti i sorgenti delle dipendenze.
- Inter: SIL Open Font License 1.1, pacchetto fonts-inter.
- DejaVu: licenza dei font DejaVu, pacchetto fonts-dejavu.
- xterm.js 6.0.0: MIT, ottenuto dal registro npm ufficiale @xterm/xterm. Copia della licenza in shell/vendor/XTERM-LICENSE. JavaScript e CSS inclusi per funzionare offline.
- VirtualBox Guest Additions: licenze dei pacchetti virtualbox-guest-utils e virtualbox-guest-x11, archivio Debian Fasttrack.

La ricetta abilita gli archivi main, contrib, non-free e non-free-firmware. Eventuale firmware proprietario mantiene i propri termini; l'abilitazione dell'archivio non significa che ogni firmware sia incluso. Non presentare tutta la ISO come software MIT o interamente libero. Prima di distribuzione pubblica verificare inventario e obblighi relativi ai pacchetti effettivamente inclusi.
- addon-fit 0.11.0 di xterm.js: MIT, licenza in shell/vendor/FIT-LICENSE.
