# Provare VelaOS in VirtualBox

Usare una VM dedicata e un disco virtuale vuoto. La preview si può provare in modalità live senza installarla sul disco. Il percorso completo dell'installer non è ancora collaudato.

Configurazione usata per il collaudo: VirtualBox 7.2.20, Linux/Debian 64 bit, EFI, 2 CPU, VMSVGA con 256 MB video, 8 GB RAM, rete NAT e ISO nel lettore ottico. 8 GB per la VM richiedono memoria sufficiente anche per l'host; le prestazioni non sono ancora misurate.

Per la release 2.1: scaricare tutte le parti, `ISO-PARTS.json` e
`join-release.ps1` in una cartella, quindi eseguire lo script per ricostruire e
verificare la ISO. Servono spazio per le parti e per il file ricostruito.

Su Windows mettere ISO e `Start-VM-Vela2.ps1` nella stessa cartella, quindi:

```powershell
.\Start-VM-Vela2.ps1 -Iso ".\VelaOS-2.1-local-preview.iso" -MemoryMB 8192
```

Lo script crea o configura la VM `Vela-2`. Se esiste già una VM con quel nome, leggere lo script e usare il parametro `-Name` per scegliere una VM dedicata. Non interrompere una VM in uso per cambiare la configurazione.

La sessione live usa l'utente `vela` e la password `live`; sono credenziali pubbliche di una sessione temporanea. Non esporre la sessione su una rete non fidata. Nella preview pubblica il servizio SSH è mascherato e le chiavi di sviluppo sono rimosse. I tool QEMU con SSH richiedono una propria build `--test`.

Se la grafica presenta problemi, scegliere il menu di avvio in modalità sicura o la sessione X11. Guest Additions e ridimensionamento a 1440×900 sono stati verificati nella preview precedente; appunti condivisi e tutte le configurazioni GPU non sono stati collaudati.
