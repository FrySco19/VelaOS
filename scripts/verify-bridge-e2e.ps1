# VelaOS verify-bridge-e2e for Windows: Rust/cargo on Windows, Docker inside WSL
# (or Docker Desktop). Same two steps as scripts/verify-bridge-e2e.sh.
#
#   powershell -ExecutionPolicy Bypass -File scripts\verify-bridge-e2e.ps1
#
# Optional: -PcbSeed <path>  existing seed matching the committed anchor
#           -Offline         pass --offline to cargo
#           -NativeDocker    use docker.exe (Docker Desktop) instead of WSL docker
param([string]$PcbSeed = "", [switch]$Offline, [switch]$NativeDocker, [switch]$Hardware)
if ($Hardware) {
  & (Join-Path $PSScriptRoot "verify-lfs-hardware.ps1")
  if (-not $?) { exit 1 }
  exit 0
}
$ErrorActionPreference = "Stop"
$Root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Set-Location $Root
$Exec   = "poc/m5_executor"
$Anchor = "$Exec/app/src/trusted_key.rs"
$Out    = ".local/e2e"
$Log    = "$Out/verify-bridge-e2e.log"
$Summary= "$Out/verify-bridge-e2e.summary.txt"
$CargoX = @(); if ($Offline) { $CargoX = @("--offline") }
function Say($m) { Write-Host "`n==> $m" -ForegroundColor Cyan }
function Die($m) { Write-Host "`nFAIL: $m" -ForegroundColor Red; exit 1 }
function Invoke-Docker { if ($NativeDocker) { & docker.exe @args } else { & wsl.exe -e docker @args } }
function Invoke-Cargo { & cargo.exe @CargoX @args; if ($LASTEXITCODE -ne 0) { Die "cargo $($args -join ' ') failed" } }
function ToWsl($p) {
  $full = [IO.Path]::GetFullPath((Join-Path $Root $p))
  $drive = $full.Substring(0,1).ToLower()
  "/mnt/$drive" + $full.Substring(2).Replace('\','/')
}

Say "Preflight"
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) { Die "cargo not found: install Rust from https://rustup.rs (>= 1.90)" }
if (-not $NativeDocker -and -not (Get-Command wsl.exe -ErrorAction SilentlyContinue)) { Die "wsl.exe not found (or use -NativeDocker)" }
Invoke-Docker info *> $null; if ($LASTEXITCODE -ne 0) { Die "Docker daemon not reachable (WSL: 'sudo service docker start'; or start Docker Desktop and use -NativeDocker)" }
Invoke-Docker buildx version *> $null; if ($LASTEXITCODE -ne 0) { Die "Docker buildx/BuildKit missing" }
Write-Host "cargo: $(cargo.exe --version)"
Write-Host "docker: $(Invoke-Docker version --format '{{.Server.Version}}')"
New-Item -ItemType Directory -Force $Out | Out-Null

$LfsSeed = "$Out/m5-lfs.seed"
if (-not $PcbSeed) { $PcbSeed = ".local/m5-executor.seed" }
$Backup = $null
try {
  if (Test-Path $PcbSeed) {
    Say "Using existing PCB seed $PcbSeed (must match committed anchor)"
  } else {
    Say "No PCB seed found -> ephemeral test key for this run only"
    $PcbSeed = "$Out/ephemeral-pcb.seed"
    Remove-Item -Force $PcbSeed -ErrorAction SilentlyContinue
    $Backup = "$Out/trusted_key.rs.committed"
    Copy-Item $Anchor $Backup -Force
    Invoke-Cargo run --locked -q --manifest-path "$Exec/keygen/Cargo.toml" --bin vela-executor-keygen '--' $PcbSeed $Anchor
  }

  Say "Step 1/2: host PDP bundle (intent -> UserSelection -> grant -> signed frames)"
  Invoke-Cargo run --locked -q --manifest-path "$Exec/policy/Cargo.toml" --example e2e_bundle '--' $PcbSeed $LfsSeed "$Exec/app/generated"

  Say "Step 2/2: docker build Dockerfile.e2e + QEMU boot + 37-marker check (first run: 15-40 min)"
  if ($NativeDocker) { $runArgs = @($Exec, $PcbSeed, $LfsSeed, $Log) }
  else { $runArgs = @((ToWsl $Exec), (ToWsl $PcbSeed), (ToWsl $LfsSeed), $Log, "--wsl") }
  $prev = $ErrorActionPreference; $ErrorActionPreference = "Continue"
  & cargo.exe @CargoX run --locked -q --manifest-path "$Exec/keygen/Cargo.toml" --bin verify-bridge-e2e '--' @runArgs 2>&1 |
    ForEach-Object { "$_" } | Tee-Object -FilePath $Summary
  $rc = $LASTEXITCODE
  (Get-Content $Summary) | Set-Content -Encoding UTF8 $Summary  # PS 5.1 Tee writes UTF-16
  $ErrorActionPreference = $prev
} finally {
  if ($Backup -and (Test-Path $Backup)) { Copy-Item $Backup $Anchor -Force; Remove-Item $Backup; Write-Host "(restored committed trust anchor $Anchor)" }
}

Write-Host "`nSerial log : $Root\$($Log.Replace('/','\'))"
Write-Host "Verdict    : $Root\$($Summary.Replace('/','\'))"
if ($rc -eq 0 -and (Select-String -Path $Summary -Pattern '^PASS \(e2e\): 37 markers' -Quiet)) {
  Write-Host "RESULT: PASS (37 markers incl. POLICY_DENIED + MAX_FUEL, no panic/FAULT)" -ForegroundColor Green
} else { Die "verify-bridge-e2e failed (exit $rc). Read the verdict file above, then the serial log." }
