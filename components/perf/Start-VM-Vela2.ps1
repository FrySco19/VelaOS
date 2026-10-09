<#
 Vela OS 2 - crea/configura la macchina virtuale "Vela-2" in VirtualBox 7.2 con le impostazioni
 consigliate per velocita' e fluidita', collega la ISO e la avvia.
 Tocca SOLO la VM "Vela-2". Non cancella mai dischi o altre VM.

 Uso (PowerShell):
   .\Start-VM-Vela2.ps1                       # usa la ISO Vela piu' recente in questa cartella
   .\Start-VM-Vela2.ps1 -Iso C:\percorso\Vela-2.iso
   .\Start-VM-Vela2.ps1 -Graphics safe        # se con la grafica 3D vedi difetti o schermo nero
   .\Start-VM-Vela2.ps1 -EjectIso             # dopo l'installazione: avvia dal disco, senza ISO
   .\Start-VM-Vela2.ps1 -NoStart              # configura soltanto
#>
param(
  [ValidatePattern('^[A-Za-z0-9][A-Za-z0-9_.-]{0,63}$')][string]$Name = 'Vela-2',
  [string]$Iso = '',
  [ValidateSet('accel','safe')][string]$Graphics = 'accel',
  [int]$MemoryMB = 0,
  [int]$Cpus = 0,
  [int]$DiskGB = 40,
  [switch]$EjectIso,
  [switch]$NoStart,
  [switch]$Headless
)
$ErrorActionPreference = 'Stop'
$VM = $Name
$VBM = 'C:\Program Files\Oracle\VirtualBox\VBoxManage.exe'
if (-not (Test-Path -LiteralPath $VBM)) {
  if ($env:VBOX_MSI_INSTALL_PATH -and (Test-Path -LiteralPath (Join-Path $env:VBOX_MSI_INSTALL_PATH 'VBoxManage.exe'))) {
    $VBM = Join-Path $env:VBOX_MSI_INSTALL_PATH 'VBoxManage.exe'
  } else { throw 'VirtualBox non trovato. Installa VirtualBox 7.2.' }
}
function Say($m) { Write-Host "[Vela-2] $m" }
function Warn($m) { Write-Host "[Vela-2] ATTENZIONE: $m" -ForegroundColor Yellow }
function VBox([string[]]$a, [string]$err) {
  $out = & $VBM @a 2>&1
  if ($LASTEXITCODE -ne 0) { throw "$err`n$($out -join "`n")" }
  return $out
}
function VBoxTry([string[]]$a) { $out = & $VBM @a 2>&1; if ($LASTEXITCODE -ne 0) { Warn ("impostazione non applicata: " + ($a -join ' ')) }; return $out }

# --- versione VirtualBox
$ver = (& $VBM --version) -join ''
Say "VirtualBox $ver"
if ($ver -notmatch '^7\.(2|[3-9])') { Warn "Vela 2 e' testato con VirtualBox 7.2. Versione trovata: $ver" }

# --- ISO
if (-not $EjectIso) {
  if (-not $Iso) {
    $f = Get-ChildItem -LiteralPath $PSScriptRoot -Filter '*.iso' -ErrorAction SilentlyContinue |
         Where-Object { $_.Name -match 'vela' } | Sort-Object LastWriteTime -Descending | Select-Object -First 1
    if (-not $f) { throw "Nessuna ISO di Vela trovata in $PSScriptRoot. Indica il percorso con -Iso." }
    $Iso = $f.FullName
  }
  if (-not (Test-Path -LiteralPath $Iso)) { throw "ISO non trovata: $Iso" }
  $Iso = (Resolve-Path -LiteralPath $Iso).Path
  Say "ISO: $Iso"
}

# --- risorse in base al PC
$cs = Get-CimInstance Win32_ComputerSystem
$hostGB = [math]::Round($cs.TotalPhysicalMemory / 1GB)
$hostThreads = [Environment]::ProcessorCount
if ($MemoryMB -le 0) { if ($hostGB -ge 15) { $MemoryMB = 6144 } elseif ($hostGB -ge 8) { $MemoryMB = 4096 } else { $MemoryMB = 3072 } }
if ($Cpus -le 0) { if ($hostThreads -ge 8) { $Cpus = 4 } elseif ($hostThreads -ge 4) { $Cpus = 2 } else { $Cpus = 1 } }
Say "PC: $hostGB GB RAM, $hostThreads thread  ->  VM: $MemoryMB MB, $Cpus CPU"

# --- Hyper-V / sicurezza basata su virtualizzazione: rende VirtualBox molto piu' lento
if ($cs.HypervisorPresent) {
  Warn "Windows sta usando Hyper-V (o 'Integrita' della memoria' / WSL2 / Sandbox). VirtualBox gira in modalita' lenta (icona tartaruga verde)."
  Warn "Per la massima velocita': Sicurezza di Windows > Sicurezza dispositivi > Isolamento core > Integrita' della memoria: Disattivata, poi riavvia il PC."
}

# --- VM esistente?
$vms = (& $VBM list vms) -join "`n"
$exists = $vms -match ('"' + [regex]::Escape($VM) + '"')
if ($exists) {
  $info = VBox @('showvminfo', $VM, '--machinereadable') 'Impossibile leggere la VM Vela-2.'
  $state = ($info | Where-Object { $_ -like 'VMState=*' }) -replace 'VMState=|"', ''
  if ($state -in @('running','paused','starting','stopping','saving','restoring')) {
    throw "La VM Vela-2 e' accesa (stato: $state). Spegnila e rilancia lo script."
  }
  if ($state -eq 'saved') { throw "La VM Vela-2 ha uno stato salvato. In VirtualBox: tasto destro su Vela-2 > Scarta lo stato salvato, poi rilancia." }
  Say 'VM Vela-2 trovata: aggiorno le impostazioni.'
} else {
  Say 'Creo la VM Vela-2.'
  VBox @('createvm', '--name', $VM, '--ostype', 'Debian_64', '--register') 'Impossibile creare la VM Vela-2.' | Out-Null
  # firmware solo alla creazione: cambiarlo dopo l'installazione renderebbe il disco non avviabile
  VBox @('modifyvm', $VM, '--firmware=efi') 'Impossibile impostare EFI.' | Out-Null
  $info = VBox @('showvminfo', $VM, '--machinereadable') 'Impossibile leggere la VM Vela-2.'
}
$cfg = ($info | Where-Object { $_ -like 'CfgFile=*' }) -replace '^CfgFile=|"', ''
$vmDir = Split-Path -Parent $cfg

# --- sistema e CPU
VBox @('modifyvm', $VM, "--memory=$MemoryMB", "--cpus=$Cpus", '--ioapic=on', '--rtc-use-utc=on',
       '--paravirt-provider=kvm', '--nested-paging=on', '--large-pages=on', '--hwvirtex=on') 'Impossibile impostare memoria/CPU.' | Out-Null
VBoxTry @('modifyvm', $VM, '--x86-pae=on', '--x86-long-mode=on', '--x86-hpet=off', '--x86-x2apic=on', '--x86-vtx-vpid=on') | Out-Null

# --- grafica
if ($Graphics -eq 'accel') {
  VBox @('modifyvm', $VM, '--graphicscontroller=vmsvga', '--vram=256', '--accelerate-3d=on', '--monitor-count=1') 'Impossibile impostare la grafica.' | Out-Null
  Say 'Grafica: VMSVGA, 256 MB, accelerazione 3D attiva.'
} else {
  VBox @('modifyvm', $VM, '--graphicscontroller=vboxsvga', '--vram=128', '--accelerate-3d=off', '--monitor-count=1') 'Impossibile impostare la grafica.' | Out-Null
  Say 'Grafica: modalita'' sicura (VBoxSVGA, senza 3D). Vela usa effetti leggeri.'
}

# --- integrazione: mouse, appunti, trascinamento, audio, rete
VBox @('modifyvm', $VM, '--mouse=usbtablet', '--keyboard=ps2', '--usb-ohci=on',
       '--clipboard-mode=bidirectional', '--drag-and-drop=bidirectional') 'Impossibile impostare mouse/appunti.' | Out-Null
VBoxTry @('modifyvm', $VM, '--audio-driver=default', '--audio-controller=hda', '--audio-enabled=on', '--audio-out=on', '--audio-in=off') | Out-Null
VBox @('modifyvm', $VM, '--nic1=nat', '--nic-type1=82540EM', '--cable-connected1=on') 'Impossibile impostare la rete.' | Out-Null
VBoxTry @('modifyvm', $VM, '--recording=off') | Out-Null

# --- disco (SATA/AHCI, SSD + TRIM)
$info = VBox @('showvminfo', $VM, '--machinereadable') 'Impossibile leggere la VM Vela-2.'
if (-not ($info -match '^storagecontrollername\d+="SATA"$')) {
  VBox @('storagectl', $VM, '--name', 'SATA', '--add', 'sata', '--controller', 'IntelAhci', '--portcount', '2', '--hostiocache', 'off', '--bootable', 'on') 'Impossibile creare il controller SATA.' | Out-Null
}
$disk = Join-Path $vmDir 'Vela-2.vdi'
$attached = ($info | Where-Object { $_ -like '"SATA-0-0"=*' }) -replace '^"SATA-0-0"=|"', ''
if (-not $attached -or $attached -eq 'none') {
  if (-not (Test-Path -LiteralPath $disk)) {
    Say "Creo il disco virtuale ($DiskGB GB, cresce solo quando serve)."
    VBox @('createmedium', 'disk', '--filename', $disk, '--size', [string]($DiskGB * 1024), '--format', 'VDI', '--variant', 'Standard') 'Impossibile creare il disco.' | Out-Null
  }
  VBox @('storageattach', $VM, '--storagectl', 'SATA', '--port', '0', '--device', '0', '--type', 'hdd', '--medium', $disk, '--nonrotational', 'on', '--discard', 'on') 'Impossibile collegare il disco.' | Out-Null
}

# --- ISO (porta 1)
if ($EjectIso) {
  VBoxTry @('storageattach', $VM, '--storagectl', 'SATA', '--port', '1', '--device', '0', '--type', 'dvddrive', '--medium', 'emptydrive') | Out-Null
  VBox @('modifyvm', $VM, '--boot1=disk', '--boot2=none', '--boot3=none', '--boot4=none') 'Impossibile impostare l''avvio.' | Out-Null
  Say 'ISO rimossa: Vela-2 si avvia dal disco.'
} else {
  VBox @('storageattach', $VM, '--storagectl', 'SATA', '--port', '1', '--device', '0', '--type', 'dvddrive', '--medium', $Iso) 'Impossibile collegare la ISO.' | Out-Null
  VBox @('modifyvm', $VM, '--boot1=dvd', '--boot2=disk', '--boot3=none', '--boot4=none') 'Impossibile impostare l''avvio.' | Out-Null
}

& $VBM showvminfo $VM --machinereadable | Out-File -Encoding utf8 (Join-Path $vmDir 'Vela-2-settings.txt')
Say "Impostazioni salvate in $(Join-Path $vmDir 'Vela-2-settings.txt')"
if ($NoStart) { Say 'Configurazione completata (VM non avviata).'; exit 0 }

# --- avvio
$type = 'gui'; if ($Headless) { $type = 'headless' }
VBox @('startvm', $VM, '--type', $type) 'Impossibile avviare Vela-2.' | Out-Null
Say 'Vela-2 avviata.'

# --- controllo modalita' di esecuzione nel log (VT-x/AMD-V oppure Hyper-V lento)
$log = Join-Path $vmDir 'Logs\VBox.log'
$t0 = Get-Date
while (((Get-Date) - $t0).TotalSeconds -lt 20) {
  if (Test-Path -LiteralPath $log) {
    $txt = Get-Content -LiteralPath $log -Raw -ErrorAction SilentlyContinue
    if ($txt -match 'HM: HMR3Init: .*(AMD-V|VT-x)' -or $txt -match 'NEM:' -or $txt -match 'VMSVGA3d') { break }
  }
  Start-Sleep -Milliseconds 500
}
if (Test-Path -LiteralPath $log) {
  $txt = Get-Content -LiteralPath $log -Raw -ErrorAction SilentlyContinue
  if ($txt -match 'fall back to NEM|NEM: .*Initialized|NEMR3Init') {
    Warn 'VirtualBox sta usando Hyper-V (modalita'' lenta). Disattiva Integrita'' della memoria / Hyper-V e riavvia il PC per avere Vela veloce.'
  } elseif ($txt -match 'HM: HMR3Init: .*(AMD-V|VT-x)') {
    Say 'Virtualizzazione hardware attiva (AMD-V/VT-x): ottimo.'
  }
}
Say 'Appunti condivisi e trascinamento: gia'' impostati su Bidirezionale.'
