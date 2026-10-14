param([Parameter(Mandatory)][string]$QemuDirectory,[Parameter(Mandatory)][string]$GuestImage,[Parameter(Mandatory)][string]$GuestSha256,[Parameter(Mandatory)][string]$Iscc)
$ErrorActionPreference='Stop'
$root=$PSScriptRoot
$stage=Join-Path $root 'dist/v1-stage'
if(Test-Path -LiteralPath $stage){throw 'Stage exists: choose a clean packaging workspace to avoid stale artifacts'}
if((Get-FileHash -LiteralPath $GuestImage -Algorithm SHA256).Hash -ne $GuestSha256){throw 'Guest image hash mismatch'}
if(!(Test-Path -LiteralPath (Join-Path $QemuDirectory 'qemu-system-aarch64.exe'))){throw 'Complete native Windows QEMU directory required'}
if(!(Test-Path -LiteralPath (Join-Path $QemuDirectory 'COPYING'))){throw 'QEMU license missing: provide complete redistributable distribution and corresponding source offer'}
$env:PATH=(Join-Path $env:USERPROFILE '.cargo/bin')+';'+$env:PATH
foreach($crate in @('poc/host-broker','poc/product/glass')){& cargo build --release --locked --manifest-path (Join-Path $root "$crate/Cargo.toml");if($LASTEXITCODE -ne 0){throw "Build failed: $crate"}}
New-Item -ItemType Directory -Force -Path $stage,(Join-Path $stage 'qemu') | Out-Null
Copy-Item -LiteralPath (Join-Path $root 'poc/host-broker/target/release/vela-host-broker.exe') -Destination $stage
Copy-Item -LiteralPath (Join-Path $root 'poc/product/glass/target/release/vela-glass.exe') -Destination $stage
Copy-Item -LiteralPath $GuestImage -Destination (Join-Path $stage 'v1-loader.img')
Copy-Item -Path (Join-Path $QemuDirectory '*') -Destination (Join-Path $stage 'qemu') -Recurse
Copy-Item -LiteralPath (Join-Path $root 'packaging/vela-native.ps1') -Destination $stage
# Never package .local, private seeds, credentials or existing user disks.
& $Iscc "/DStage=$stage" "/DOutput=$(Join-Path $root 'dist')" (Join-Path $root 'vela-setup.iss')
if($LASTEXITCODE -ne 0){throw 'Installer compiler failed'}
Get-FileHash -LiteralPath (Join-Path $root 'dist/Vela-Soft-Setup.exe') -Algorithm SHA256
