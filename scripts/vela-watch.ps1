param([Parameter(Mandatory=$true)][string]$StatePath,[switch]$CleanupNow)
$ErrorActionPreference='Stop'
$state=Get-Content -LiteralPath $StatePath -Raw | ConvertFrom-Json
function Cleanup-Vela {
    foreach($entry in @($state.Processes)){
        $p=Get-Process -Id $entry.Id -ErrorAction SilentlyContinue
        if($p -and $p.Path -eq $entry.Path -and $p.StartTime.ToUniversalTime().Ticks.ToString() -eq $entry.Ticks){Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue}
    }
    # Only containers labelled by this launcher instance are stopped.
    $ids=& $state.Wsl -d Debian -- docker ps -aq --filter "label=vela.soft.run=$($state.RunId)"
    foreach($id in $ids){if($id -match '^[0-9a-f]{12,64}$'){& $state.Wsl -d Debian -- docker stop -t 5 $id | Out-Null}}
    if($state.Keeper){$p=Get-Process -Id $state.Keeper.Id -ErrorAction SilentlyContinue;if($p -and $p.Path -eq $state.Keeper.Path -and $p.StartTime.ToUniversalTime().Ticks.ToString() -eq $state.Keeper.Ticks){Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue}}
}
if(!$CleanupNow){
    while($true){
        $parent=Get-Process -Id $state.ParentId -ErrorAction SilentlyContinue
        if(!$parent -or $parent.StartTime.ToUniversalTime().Ticks.ToString() -ne $state.ParentTicks){break}
        Start-Sleep -Milliseconds 500
    }
    $state=Get-Content -LiteralPath $StatePath -Raw | ConvertFrom-Json
}
Cleanup-Vela
