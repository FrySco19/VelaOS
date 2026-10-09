<#
 Vela OS 2 — final gate in VirtualBox (user's PC). Touches ONLY the VM "Vela-2".
 usage (PowerShell):  .\vbox-gate.ps1 -Iso <path-to-iso> [-User vela] [-Pass vela] [-Phase boot|test|off|all]
 Steps: (boot) create/configure via perf's Start-VM-Vela2.ps1, start; (test) screenshots over time, guest additions,
 resize hint, host<->guest clipboard via guestcontrol; (off) ACPI power button then poweroff fallback.
 Evidence: <dir of ISO>\qa-evidence\
#>
param([Parameter(Mandatory=$true)][string]$Iso, [string]$User='vela', [string]$Pass='vela', [string]$Phase='all', [int]$BootWait=150)
$ErrorActionPreference = 'Continue'
$VBM = 'C:\Program Files\Oracle\VirtualBox\VBoxManage.exe'
$VM = 'Vela-2'
$Dir = Split-Path -Parent $Iso
$Ev = Join-Path $Dir 'qa-evidence'; New-Item -ItemType Directory -Force -Path $Ev | Out-Null
$Log = Join-Path $Ev 'gate.log'
function L($m) { $t = (Get-Date).ToString('HH:mm:ss'); "$t $m" | Tee-Object -FilePath $Log -Append }
function Shot($name) { & $VBM controlvm $VM screenshotpng (Join-Path $Ev "$name.png") 2>&1 | Out-Null; L "screenshot $name" }
function GuestRun([string]$cmd) {
  & $VBM guestcontrol $VM run --exe /bin/sh --username $User --password $Pass --wait-stdout --wait-stderr --timeout 20000 -- -c "U=`$(id -u); export XDG_RUNTIME_DIR=/run/user/`$U WAYLAND_DISPLAY=wayland-0 DISPLAY=:0 DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/`$U/bus; $cmd" 2>&1
}
$vms = (& $VBM list vms) -join "`n"
if ($Phase -in @('boot','all')) {
  L "== BOOT  iso=$Iso"
  if ((Get-FileHash $Iso -Algorithm SHA256).Hash) { L ("sha256 " + (Get-FileHash $Iso -Algorithm SHA256).Hash) }
  $perf = Join-Path $Dir 'Start-VM-Vela2.ps1'
  if (-not (Test-Path $perf)) { L "ERROR: $perf missing"; exit 1 }
  $t0 = Get-Date
  & powershell -ExecutionPolicy Bypass -File $perf -Iso $Iso 2>&1 | Tee-Object -FilePath $Log -Append
  & $VBM showvminfo $VM --machinereadable | Out-File (Join-Path $Ev 'showvminfo.txt')
  foreach ($s in 10,20,30,45,60,90,120) {
    while (((Get-Date) - $t0).TotalSeconds -lt $s) { Start-Sleep -Milliseconds 500 }
    Shot ("boot-{0:d3}s" -f $s)
  }
}
if ($Phase -in @('test','all')) {
  L "== TEST"
  $ga = & $VBM guestproperty get $VM '/VirtualBox/GuestAdd/Version' 2>&1; L "GuestAdd/Version: $ga"
  $gar = & $VBM guestproperty get $VM '/VirtualBox/GuestAdd/RunLevel' 2>&1; L "GuestAdd/RunLevel: $gar"
  & $VBM guestproperty enumerate $VM 2>&1 | Out-File (Join-Path $Ev 'guestproperties.txt')
  Shot 'desktop-1'
  # resize (P-DISP-1)
  & $VBM controlvm $VM setvideomodehint 1600 900 32 2>&1 | Out-Null; Start-Sleep 5; Shot 'resize-1600x900'
  & $VBM controlvm $VM setvideomodehint 1280 800 32 2>&1 | Out-Null; Start-Sleep 5; Shot 'resize-1280x800'
  Add-Type -AssemblyName System.Drawing
  foreach ($n in 'resize-1600x900','resize-1280x800') { $p = Join-Path $Ev "$n.png"; if (Test-Path $p) { $i=[System.Drawing.Image]::FromFile($p); L "$n actual=$($i.Width)x$($i.Height)"; $i.Dispose() } }
  # clipboard (R4d)
  & $VBM controlvm $VM clipboard mode bidirectional 2>&1 | Out-Null
  $n1 = 'vela-host-' + (Get-Random); Set-Clipboard -Value $n1; Start-Sleep 2
  $g = GuestRun 'timeout 5 wl-paste -n 2>/dev/null || timeout 5 xclip -o -sel clip 2>/dev/null'
  L "host->guest expected=$n1 got=[$g]"; if ("$g" -match $n1) { L 'PASS R4d host->guest' } else { L 'FAIL R4d host->guest' }
  $n2 = 'vela-guest-' + (Get-Random)
  GuestRun "printf $n2 | (wl-copy 2>/dev/null || xclip -sel clip) ; sleep 1" | Out-Null; Start-Sleep 3
  $h = Get-Clipboard; L "guest->host expected=$n2 got=[$h]"; if ("$h" -match $n2) { L 'PASS R4d guest->host' } else { L 'FAIL R4d guest->host' }
  $info = GuestRun 'lsmod | grep -E "vbox|vmwgfx"; loginctl show-session $(loginctl list-sessions --no-legend | awk "NR==1{print \$1}") -p Type; free -m; systemd-analyze; qdbus org.kde.KWin /Compositor org.kde.kwin.Compositing.active 2>/dev/null || qdbus6 org.kde.KWin /Compositor org.kde.kwin.Compositing.active 2>/dev/null'
  $info | Out-File (Join-Path $Ev 'guest-info.txt'); L "guest info saved"
  Shot 'desktop-2'
}
if ($Phase -in @('off','all')) {
  L "== OFF"
  & $VBM controlvm $VM acpipowerbutton 2>&1 | Out-Null
  $ok = $false; foreach ($i in 1..30) { Start-Sleep 1; if ((& $VBM showvminfo $VM --machinereadable) -match 'VMState="poweroff"') { $ok = $true; break } }
  if ($ok) { L "PASS P-POWER-1 ACPI shutdown in $i s" } else { L "FAIL P-POWER-1 ACPI shutdown >30 s, forcing poweroff"; & $VBM controlvm $VM poweroff 2>&1 | Out-Null }
}
L "== done; evidence in $Ev"
