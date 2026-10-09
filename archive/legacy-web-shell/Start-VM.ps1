$ErrorActionPreference = 'Stop'
$vboxPath = 'C:\Program Files\Oracle\VirtualBox\VBoxManage.exe'
$isoPath = Join-Path $PSScriptRoot 'Vela-Banchina-preview.iso'
if (!(Test-Path -LiteralPath $isoPath)) { throw 'Costruisci prima Vela-Banchina-preview.iso.' }
$vmName = 'Vela-MVP'
$existing = & $vboxPath list vms
if (!($existing -match '^"Vela-MVP"')) {
    & $vboxPath createvm --name $vmName --ostype Debian_64 --register
    & $vboxPath modifyvm $vmName --memory 4096 --cpus 2 --graphicscontroller vmsvga --vram 128 --accelerate-3d off --firmware bios --boot1 dvd --nic1 nat
    & $vboxPath storagectl $vmName --name SATA --add sata --controller IntelAhci
}
& $vboxPath storageattach $vmName --storagectl SATA --port 0 --device 0 --type dvddrive --medium $isoPath
if ($LASTEXITCODE -ne 0) { throw 'Impossibile collegare la ISO; controlla che la VM sia spenta.' }
& $vboxPath modifyvm $vmName --recording off
if ($LASTEXITCODE -ne 0) { throw 'Impossibile preparare la VM; controlla che sia spenta.' }
& $vboxPath startvm $vmName --type gui

