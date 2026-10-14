$ErrorActionPreference='Stop'
$root=$PSScriptRoot
$data=Join-Path $env:LOCALAPPDATA 'Vela/data'
New-Item -ItemType Directory -Force -Path $data | Out-Null
$mutex=New-Object Threading.Mutex($false,'Local\VelaNative')
$owned=@()
try {
 if(!$mutex.WaitOne(0)){throw 'Vela is already running'}
 foreach($f in @('broker.seed','pcb.public','lfs.public','fido.json')){if(!(Test-Path -LiteralPath (Join-Path $data $f))){throw "Enrollment missing: $f. Provision a matching guest/host identity before launching."}}
 foreach($port in @(7990,7991,7882)){if(Get-NetTCPConnection -LocalPort $port -State Listen -ErrorAction SilentlyContinue){throw "Port $port already in use"}}
 $disk=Join-Path $data 'disk.img'
 if(!(Test-Path -LiteralPath $disk)){$f=[IO.File]::Open($disk,[IO.FileMode]::CreateNew);try{$f.SetLength(131072);$f.Flush($true)}finally{$f.Dispose()}}
 if((Get-Item -LiteralPath $disk).Length -ne 131072){throw 'Unexpected disk size'}
 $qa=@('-machine','virt,virtualization=on,gic-version=2','-cpu','cortex-a53','-m','2G','-display','none','-monitor','none','-serial',('file:"'+(Join-Path $data 'boot.log')+'"'),'-net','none','-kernel',('"'+(Join-Path $root 'v1-loader.img')+'"'),'-global','virtio-mmio.force-legacy=false','-chardev','socket,id=host,host=127.0.0.1,port=7991,server=on,wait=off','-device','virtio-serial-device,id=v0','-device','virtconsole,chardev=host,bus=v0.0')
 for($i=0;$i -lt 7;$i++){$qa+=@('-device','virtio-serial-device')}
 $qa+=@('-drive',('file="'+$disk+'",if=none,id=lfs,format=raw,cache=none'),'-device','virtio-blk-device,drive=lfs')
 $owned+=Start-Process (Join-Path $root 'qemu/qemu-system-aarch64.exe') -WindowStyle Hidden -PassThru -ArgumentList $qa
 function Wait-Port($port){for($i=0;$i -lt 100;$i++){$c=New-Object Net.Sockets.TcpClient;try{$t=$c.ConnectAsync('127.0.0.1',$port);if($t.Wait(100) -and $c.Connected){return}}catch{}finally{$c.Dispose()};Start-Sleep -Milliseconds 100};throw "Service $port did not start"}
 Wait-Port 7991
 $owned+=Start-Process (Join-Path $root 'vela-glass.exe') -WindowStyle Hidden -PassThru -ArgumentList @('--port','7882')
 $ba=@('serve');foreach($name in @('broker.seed','pcb.public','lfs.public','fido.json')){$ba+='"'+(Join-Path $data $name)+'"'}
 $owned+=Start-Process (Join-Path $root 'vela-host-broker.exe') -WindowStyle Hidden -PassThru -ArgumentList $ba
 Wait-Port 7882;Wait-Port 7990
 Start-Process 'http://127.0.0.1:7882/studio'
 while($true){foreach($p in $owned){$p.Refresh();if($p.HasExited){throw 'A Vela component stopped'}};Start-Sleep -Seconds 1}
} catch {Write-Error $_ -ErrorAction Continue;Read-Host 'Press Enter to close'} finally {
 foreach($p in $owned){try{if(!$p.HasExited){$p.Kill();$p.WaitForExit()}}catch{}}
 $mutex.Dispose()
}
