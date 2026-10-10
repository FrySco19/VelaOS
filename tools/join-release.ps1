param([string]$Directory = $PSScriptRoot)
$ErrorActionPreference = 'Stop'
$root = (Resolve-Path -LiteralPath $Directory).Path
$manifestPath = Join-Path $root 'ISO-PARTS.json'
$manifest = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
if ($manifest.iso -ne [IO.Path]::GetFileName($manifest.iso)) { throw 'Invalid ISO filename' }
$target = Join-Path $root $manifest.iso
if (Test-Path -LiteralPath $target) { throw 'The ISO already exists. Choose an empty output folder.' }
foreach ($part in $manifest.parts) {
    if ($part.name -ne [IO.Path]::GetFileName($part.name)) { throw 'Invalid part filename' }
    $path = Join-Path $root $part.name
    if (-not (Test-Path -LiteralPath $path)) { throw "Missing part: $($part.name)" }
    if ((Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant() -ne $part.sha256) {
        throw "Checksum mismatch: $($part.name)"
    }
}
$out = [IO.File]::Open($target, [IO.FileMode]::CreateNew, [IO.FileAccess]::Write)
try {
    foreach ($part in $manifest.parts) {
        $inputStream = [IO.File]::OpenRead((Join-Path $root $part.name))
        try { $inputStream.CopyTo($out) } finally { $inputStream.Dispose() }
    }
} finally { $out.Dispose() }
if ((Get-FileHash -LiteralPath $target -Algorithm SHA256).Hash.ToLowerInvariant() -ne $manifest.sha256) {
    throw 'ISO checksum mismatch. Do not boot this file.'
}
Write-Host "Verified ISO: $target"
