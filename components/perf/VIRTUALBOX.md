# Vela 2 in VirtualBox

Serve VirtualBox 7.2 (provato con la 7.2.20). Vela 2 usa una macchina virtuale nuova, chiamata **Vela-2**. Le altre VM restano come sono.

## Il modo più semplice

1. Metti `Start-VM-Vela2.ps1` nella stessa cartella della ISO di Vela.
2. Fai clic destro sul file e scegli **Esegui con PowerShell**.
   In alternativa, apri PowerShell in quella cartella e scrivi:
   `powershell -ExecutionPolicy Bypass -File .\Start-VM-Vela2.ps1`
3. Lo script crea la VM Vela-2 con le impostazioni giuste, collega la ISO e la avvia.

Se rilanci lo script, aggiorna le impostazioni della VM esistente senza cancellare niente. La VM deve essere spenta.

## Dopo l'installazione sul disco

Spegni Vela e lancia:
`powershell -ExecutionPolicy Bypass -File .\Start-VM-Vela2.ps1 -EjectIso`

La ISO viene tolta e Vela si avvia dal disco.

## Impostazioni usate (se preferisci farle a mano)

| Dove, in VirtualBox | Valore |
|---|---|
| Generale › Tipo | Linux, Debian (64-bit) |
| Sistema › Scheda madre › Memoria | 6144 MB (4096 MB se il PC ha 8 GB) |
| Sistema › Scheda madre › Abilita EFI | Sì |
| Sistema › Processore | 4 CPU |
| Sistema › Accelerazione › Paravirtualizzazione | KVM, con Paginazione annidata attiva |
| Schermo › Scheda video | VMSVGA, 256 MB, **Abilita accelerazione 3D: Sì** |
| Archiviazione | Controller SATA, disco VDI da 40 GB con "Unità a stato solido" attivo, ISO nel lettore ottico |
| Audio | Intel HD Audio, uscita attiva, ingresso spento |
| Rete | NAT |
| USB | Controller USB 1.1, mouse "Tavoletta USB" |
| Generale › Avanzate | Appunti condivisi: Bidirezionale. Trascinamento: Bidirezionale |

## Copia e incolla tra Windows e Vela

Funziona da solo: Ctrl+C in Windows e Ctrl+V in Vela, e viceversa. Se non funziona, controlla che nel menu della finestra della VM, **Dispositivi › Appunti condivisi**, sia scelto **Bidirezionale**.

Il trascinamento di file tra Windows e Vela funziona solo nella sessione "Plasma (X11)", che si sceglie nella schermata di accesso. Questo è un limite di VirtualBox.

## Se Vela è lento

- **Icona della tartaruga verde** in basso a destra nella finestra della VM: Windows sta usando Hyper-V e VirtualBox va molto più piano. Apri Sicurezza di Windows › Sicurezza dispositivi › Isolamento core, metti **Integrità della memoria** su Disattivata e riavvia il PC. Anche WSL2, Windows Sandbox e la funzionalità Hyper-V causano lo stesso rallentamento. Lo script ti avvisa quando succede.
- **Accelerazione 3D spenta**: Vela funziona, ma con effetti ridotti. Lo segnala con una notifica. Spegni la VM, apri Impostazioni › Schermo e attiva "Abilita accelerazione 3D".
- Chiudi gli altri programmi pesanti su Windows mentre usi la VM.

## Se lo schermo resta nero o vedi difetti grafici

Alcune combinazioni di scheda video e driver di Windows danno problemi con l'accelerazione 3D. Spegni la VM e lancia:
`powershell -ExecutionPolicy Bypass -File .\Start-VM-Vela2.ps1 -Graphics safe`

Vela parte senza 3D e si adatta da solo con effetti più leggeri. Per tornare alla grafica completa, rilancia lo script senza `-Graphics safe`.

## Accesso nella sessione di prova (live)

Utente **vela**, password **live**. Valgono solo per la sessione di prova: durante l'installazione scegli il tuo nome utente e la tua password.
