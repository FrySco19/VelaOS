# Componenti di terze parti

VelaOS si basa su software esistente. Copyright e licenze dei rispettivi autori restano validi.

| Componente | Origine e note |
| --- | --- |
| Debian 13, kernel e pacchetti | [Debian](https://www.debian.org/); licenze dei singoli pacchetti in `/usr/share/doc/<pacchetto>/copyright` nella ISO |
| KDE Plasma, Frameworks, Discover, Konsole | [KDE](https://kde.org/); GPL/LGPL e altre licenze a seconda del componente |
| Qt/PySide6 | [Qt](https://www.qt.io/); conservare le condizioni della specifica versione distribuita |
| Firefox ESR | [Mozilla](https://www.mozilla.org/firefox/); non viene dichiarata proprietà del marchio Firefox |
| Flatpak/Flathub | [Flatpak](https://flatpak.org/), [Flathub](https://flathub.org/); ciascuna app ha licenza propria e non è inclusa solo perché compare nel catalogo |
| Calamares | [Calamares](https://calamares.io/); configurazione basata sui pacchetti Debian |
| Guest Additions | Pacchetti [Debian Fast Track](https://fasttrack.debian.net/); mantenere le licenze del pacchetto |
| Inter | [Inter](https://github.com/rsms/inter), SIL Open Font License 1.1; font/risorse derivati conservano le relative condizioni |
| Risorse Vela | Generatori e metadati in `components/theme`; vedere LICENSES/README.md |

La ISO include anche pacchetti da `contrib`, `non-free` e `non-free-firmware`: il progetto Vela è aperto, ma non tutti i pacchetti della distribuzione sono sotto GPL. Il manifesto `/live/filesystem.packages` identifica nomi e versioni distribuite. Le rispettive informazioni copyright sono presenti nel filesystem live.

## Sorgenti delle dipendenze

Questo repository contiene i sorgenti originali Vela e i sorgenti dei KCM presenti nell'overlay. Per i pacchetti Debian, ottenere la versione esatta del pacchetto sorgente dal repository di provenienza con `apt-get source`, abilitando le corrispondenti righe `deb-src`; se non più sul mirror corrente, consultare [snapshot.debian.org](https://snapshot.debian.org/). Per Fast Track consultare il suo repository sorgenti. Riferimenti generici non sostituiscono i sorgenti corrispondenti richiesti dalle singole licenze: per distribuzioni successive conservare i pacchetti sorgente e verificare gli obblighi prima di pubblicare binari.

Le licenze originali non devono essere eliminate dalle copie o dalle redistribuzioni. Eventuali materiali di terze parti aggiunti con una pull request devono avere provenienza e condizioni verificabili.
