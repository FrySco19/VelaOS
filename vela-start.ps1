param([switch]$Hardware,[switch]$NoBrowser,[string]$DataDirectory)
$ErrorActionPreference='Stop'
$root=$PSScriptRoot
Set-Location -LiteralPath $root
$runtime=if($DataDirectory){[IO.Path]::GetFullPath($DataDirectory)}else{Join-Path $root '.local/v1'}
$allowedRuntime=[IO.Path]::GetFullPath((Join-Path $root '.local'))+[IO.Path]::DirectorySeparatorChar
if(!$runtime.StartsWith($allowedRuntime,[StringComparison]::OrdinalIgnoreCase)){throw 'Data directory must be a dedicated workspace .local directory'}
$wsl=Join-Path $env:SystemRoot 'System32/wsl.exe'
$broker=Join-Path $root 'poc/host-broker/target/debug/vela-host-broker.exe'
$glass=Join-Path $root 'poc/product/glass/target/debug/vela-glass.exe'
$watch=Join-Path $root 'scripts/vela-watch.ps1'
$runId=[guid]::NewGuid().ToString('N')
$statePath=Join-Path $runtime "soft-run-$runId.json"
$owned=@();$keeper=$null;$watcher=$null
function Docker-Vela([string[]]$Arguments){$result=& $wsl -d Debian -- docker @Arguments;if($LASTEXITCODE -ne 0){throw "Docker operation failed: $($Arguments[0])"};return $result}
function Identity($process,[string]$ExpectedPath){return @{Id=$process.Id;Path=$ExpectedPath;Ticks=$process.StartTime.ToUniversalTime().Ticks.ToString()}}
$parent=Get-Process -Id $PID
$state=@{RunId=$runId;ParentId=$PID;ParentTicks=$parent.StartTime.ToUniversalTime().Ticks.ToString();Wsl=$wsl;Processes=@();Keeper=$null}
$hash=[Security.Cryptography.SHA256]::Create()
$mutexName='Local\VelaSoft-'+([BitConverter]::ToString($hash.ComputeHash([Text.Encoding]::UTF8.GetBytes($root.ToLowerInvariant())))).Replace('-','')
$hash.Dispose()
$mutex=New-Object Threading.Mutex($false,$mutexName)
try{$acquired=$mutex.WaitOne(0)}catch [Threading.AbandonedMutexException]{$acquired=$true}
if(!$acquired){$mutex.Dispose();throw 'Vela Soft is already running for this workspace'}
function Save-State {
    $state.Processes=@($script:owned);$state.Keeper=$script:keeper
    $temporary=$statePath+'.tmp'
    $state | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $temporary -Encoding UTF8
    Move-Item -LiteralPath $temporary -Destination $statePath -Force
}
function Wait-Port([int]$Port){
    for($i=0;$i -lt 100;$i++){
        $client=New-Object Net.Sockets.TcpClient
        try{$task=$client.ConnectAsync('127.0.0.1',$Port);if($task.Wait(200) -and $client.Connected){return}}catch{}finally{$client.Dispose()}
        Start-Sleep -Milliseconds 100
    }
    throw "Service did not start on port $Port"
}
try {
    foreach($relative in @('.local/host-bridge/broker.seed','.local/v1/pcb.public','.local/v1/lfs.public')){if(!(Test-Path -LiteralPath (Join-Path $root $relative))){throw "Local enrollment missing: $relative"}}
    if(!(Test-Path -LiteralPath $wsl)){throw 'WSL Debian is required by this launcher'}
    if(!(Test-Path -LiteralPath (Join-Path $root '.local/host-bridge/fido.json'))){throw 'Register a USB key with enroll-fido before launching hardware mode'}
    Docker-Vela @('image','inspect','velaos-v1') | Out-Null
    foreach($port in @(7990,7882)){
        foreach($l in @(Get-NetTCPConnection -LocalPort $port -State Listen -ErrorAction SilentlyContinue)){
            $p=Get-Process -Id $l.OwningProcess
            $expected=if($port -eq 7990){$broker}else{$glass}
            if($p.Path -ne $expected){throw "Port $port belongs to another application"}
            Stop-Process -Id $p.Id -Force
        }
    }
    # Replace only the old workspace VM, verifying its actual mounted volume.
    $old=Docker-Vela @('ps','-q','--filter','name=^vela-v1-vm$')
    if($old){
        $inspect=(Docker-Vela @('inspect',$old) | Out-String | ConvertFrom-Json)[0]
        $linuxRuntime='/mnt/'+$runtime.Substring(0,1).ToLower()+$runtime.Substring(2).Replace('\','/')
        if(!(@($inspect.Mounts).Where({$_.Source -eq $linuxRuntime -and $_.Destination -eq '/data'}))){throw 'Existing VM is not owned by this workspace'}
        Docker-Vela @('stop','-t','5',$old)|Out-Null
    }
    if(!(Test-Path -LiteralPath $broker) -or !(Test-Path -LiteralPath $glass)){
        $cargo=Join-Path $env:USERPROFILE '.cargo/bin/cargo.exe'
        if(!(Test-Path -LiteralPath $cargo)){throw 'Rust is required to build missing Windows executables'}
        & $cargo build --locked --manifest-path poc/host-broker/Cargo.toml;if($LASTEXITCODE -ne 0){throw 'Broker build failed'}
        & $cargo build --locked --manifest-path poc/product/glass/Cargo.toml;if($LASTEXITCODE -ne 0){throw 'Glass build failed'}
    }
    New-Item -ItemType Directory -Path $runtime -Force | Out-Null
    $disk=Join-Path $runtime 'disk.img'
    if(!(Test-Path -LiteralPath $disk)){$file=[IO.File]::Open($disk,[IO.FileMode]::CreateNew);try{$file.SetLength(131072);$file.Flush($true)}finally{$file.Dispose()}}
    elseif((Get-Item -LiteralPath $disk).Length -eq 4096){
        if(Test-Path -LiteralPath ($disk+'.pre-archive.bak')){throw 'Migration backup exists; inspect before retrying'}
        Copy-Item -LiteralPath $disk -Destination ($disk+'.pre-archive.bak')
        $file=[IO.File]::Open($disk,[IO.FileMode]::Open);try{$file.SetLength(131072);$file.Flush($true)}finally{$file.Dispose()}
    }
    elseif((Get-Item -LiteralPath $disk).Length -ne 131072){throw 'Unexpected LFS disk size: refusing to mount'}
    Save-State
    $watcher=Start-Process -FilePath powershell.exe -WindowStyle Hidden -PassThru -ArgumentList @('-NoProfile','-ExecutionPolicy','RemoteSigned','-File',('"'+$watch+'"'),'-StatePath',('"'+$statePath+'"'))
    $k=Start-Process -FilePath $wsl -WindowStyle Hidden -PassThru -ArgumentList @('-d','Debian','--','sleep','infinity');$keeper=Identity $k $wsl;Save-State
    $linuxRuntime='/mnt/'+$runtime.Substring(0,1).ToLower()+$runtime.Substring(2).Replace('\','/')
    $dockerArgs=@('run','-d','--rm','--name',"vela-soft-$runId",'--label',"vela.soft.run=$runId",'-p','127.0.0.1:7991:7991','-v',($linuxRuntime+':/data'),'velaos-v1','qemu-system-aarch64','-machine','virt,virtualization=on,gic-version=2','-cpu','cortex-a53','-m','2G','-display','none','-monitor','none','-serial','file:/data/soft-boot.log','-net','none','-kernel','/work/build/v1-loader.img','-global','virtio-mmio.force-legacy=false','-chardev','socket,id=host,host=0.0.0.0,port=7991,server=on,wait=off','-device','virtio-serial-device,id=v0','-device','virtconsole,chardev=host,bus=v0.0')
    for($i=0;$i -lt 7;$i++){$dockerArgs+=@('-device','virtio-serial-device')}
    $dockerArgs+=@('-drive','file=/data/disk.img,if=none,id=lfs,format=raw,cache=none','-device','virtio-blk-device,drive=lfs')
    Docker-Vela $dockerArgs | Out-Null
    Wait-Port 7991
    $g=Start-Process -FilePath $glass -WindowStyle Hidden -PassThru -WorkingDirectory $root -ArgumentList @('--port','7882') -RedirectStandardOutput (Join-Path $runtime 'soft-glass.log') -RedirectStandardError (Join-Path $runtime 'soft-glass-error.log');$owned+=Identity $g $glass;Save-State
    Wait-Port 7882
    $brokerArgs=@('serve','.local/host-bridge/broker.seed','.local/v1/pcb.public','.local/v1/lfs.public','.local/host-bridge/fido.json')
    Write-Host 'Hardware FIDO2 required; no virtual approval fallback.'
    Write-Host 'Lascia aperto il terminale per autorizzare. Ctrl+C o chiusura arrestano Vela.'
    $b=Start-Process -FilePath $broker -NoNewWindow -PassThru -WorkingDirectory $root -ArgumentList $brokerArgs;$owned+=Identity $b $broker;Save-State
    Wait-Port 7990
    if(!$NoBrowser){Start-Process 'http://127.0.0.1:7882/studio'}
    $b.WaitForExit()
    if($b.ExitCode -ne 0){throw "Broker exited with code $($b.ExitCode)"}
} finally {
    if(Test-Path -LiteralPath $statePath){& $watch -StatePath $statePath -CleanupNow}
    if($watcher -and !$watcher.HasExited){Stop-Process -Id $watcher.Id -Force -ErrorAction SilentlyContinue}
    $mutex.ReleaseMutex();$mutex.Dispose()
}
