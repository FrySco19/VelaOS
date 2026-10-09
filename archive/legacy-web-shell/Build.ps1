$ErrorActionPreference = 'Stop'
$sourcePath = $PSScriptRoot
$linuxPath = (& wsl.exe -d Debian -- wslpath -a $sourcePath).Trim()
if ($LASTEXITCODE -ne 0) { throw 'Debian in WSL non è disponibile.' }
& wsl.exe -d Debian -u root -- sh "$linuxPath/build.sh"
if ($LASTEXITCODE -ne 0) { throw 'La build non è riuscita. Conserva il log per la diagnosi.' }
