# Local-only hardware proof; restores the committed public anchor in finally.
param([string]$OutputDirectory = "")
$ErrorActionPreference = "Stop"
$Root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Set-Location $Root
if (-not $OutputDirectory) { $OutputDirectory = ".local/hardware-$(Get-Date -Format yyyyMMdd-HHmmss)" }
New-Item -ItemType Directory -Force .local/hardware-keys | Out-Null
$Anchor = "poc/m5_executor/app/src/trusted_key.rs"
$Original = [IO.File]::ReadAllBytes((Join-Path $Root $Anchor))
$OldMode = $env:VELA_HARDWARE
function WslPath([string]$Path) {
    $Full = [IO.Path]::GetFullPath((Join-Path $Root $Path))
    return "/mnt/" + $Full.Substring(0,1).ToLower() + $Full.Substring(2).Replace('\','/')
}
try {
    & cargo run --locked -q --manifest-path poc/m5_executor/keygen/Cargo.toml --bin vela-executor-keygen -- .local/hardware-keys/pcb.seed $Anchor
    if ($LASTEXITCODE -ne 0) { throw "Test key generation failed" }
    $env:VELA_HARDWARE = "1"
    & cargo run --locked -q --manifest-path poc/m5_executor/policy/Cargo.toml --example e2e_bundle -- .local/hardware-keys/pcb.seed .local/hardware-keys/lfs.seed poc/m5_executor/app/generated
    if ($LASTEXITCODE -ne 0) { throw "Biscuit fixture generation failed" }
    & wsl -d Debian -- docker image inspect velaos-executor-e2e:latest *> $null
    if ($LASTEXITCODE -ne 0) {
        & wsl -d Debian -- docker build --secret ("id=pcb_seed,src=" + (WslPath ".local/hardware-keys/pcb.seed")) --secret ("id=lfs_seed,src=" + (WslPath ".local/hardware-keys/lfs.seed")) -f (WslPath "poc/m5_executor/Dockerfile.e2e") -t velaos-executor-e2e (WslPath "poc/m5_executor")
        if ($LASTEXITCODE -ne 0) { throw "Microkit foundation build failed" }
    }
    & wsl -d Debian -- python3 (WslPath "scripts/verify-lfs-hardware.py") --pcb-seed (WslPath ".local/hardware-keys/pcb.seed") --lfs-seed (WslPath ".local/hardware-keys/lfs.seed") --out (WslPath $OutputDirectory)
    if ($LASTEXITCODE -ne 0) { throw "Hardware proof failed; inspect output logs" }
} finally {
    [IO.File]::WriteAllBytes((Join-Path $Root $Anchor), $Original)
    $env:VELA_HARDWARE = $OldMode
}
