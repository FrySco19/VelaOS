<#
.SYNOPSIS
  Creates (or reuses) a VirtualBox VM for the native VelaOS x86_64 ISO and starts it.
.EXAMPLE
  powershell -ExecutionPolicy Bypass -File .\Start-VelaOS-VirtualBox.ps1
#>
param(
    [string]$Iso = (Join-Path $PSScriptRoot "dist\velaos-x86_64.iso"),
    [string]$Name = "VelaOS-x86_64",
    [int]$MemoryMB = 512,
    [switch]$Headless
)
$ErrorActionPreference = "Stop"
$vbm = "$env:ProgramFiles\Oracle\VirtualBox\VBoxManage.exe"
if (-not (Test-Path $vbm)) { $vbm = (Get-Command VBoxManage -ErrorAction SilentlyContinue).Source }
if (-not $vbm) { throw "VirtualBox not found. Install it from https://www.virtualbox.org/ and retry." }
$Iso = (Resolve-Path $Iso).Path

$exists = (& $vbm list vms) -match "^`"$([regex]::Escape($Name))`" "
if (-not $exists) {
    Write-Host "Creating VM '$Name'..."
    & $vbm createvm --name $Name --ostype Other_64 --register | Out-Null
    & $vbm modifyvm $Name --memory $MemoryMB --vram 32 --cpus 1 --firmware bios `
        --graphicscontroller vboxvga --mouse ps2 --keyboard ps2 --ioapic on `
        --boot1 dvd --boot2 none --boot3 none --boot4 none --audio-enabled off --nic1 none
    & $vbm storagectl $Name --name "IDE" --add ide | Out-Null
}
$running = (& $vbm list runningvms) -match "^`"$([regex]::Escape($Name))`" "
if ($running) { Write-Host "VM '$Name' is already running."; exit 0 }
& $vbm storageattach $Name --storagectl "IDE" --port 1 --device 0 --type dvddrive --medium $Iso
$type = if ($Headless) { "headless" } else { "gui" }
& $vbm startvm $Name --type $type
Write-Host "VelaOS is booting. You should see the GRUB menu, the VelaOS splash, then the desktop."
