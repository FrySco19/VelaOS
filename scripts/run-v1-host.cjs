// Development launcher: Windows Glass + broker + native Microkit QEMU.
// Uses the built velaos-v1 image and locally enrolled development keys.
// Mock consent is NEVER enabled by default.
const fs=require('node:fs'),path=require('node:path'),net=require('node:net');
const {spawn,spawnSync}=require('node:child_process');
const root=path.resolve(__dirname,'..'),dir=path.join(root,'.local','v1');
const wsl=path.join(process.env.SystemRoot||'C:\\Windows','System32','wsl.exe');
const linuxPath=p=>'/mnt/'+p[0].toLowerCase()+p.slice(2).replaceAll('\\','/');
function docker(args){const r=spawnSync(wsl,['-d','Debian','--','docker',...args],{cwd:root,encoding:'utf8',windowsHide:true});if(r.status!==0)throw new Error(r.stderr||r.error?.message);return r.stdout.trim();}
function native(relative,args,label,port){
 const exe=path.join(root,relative);if(!fs.existsSync(exe))throw new Error('Build missing: '+exe);
 const inspect=spawnSync('powershell.exe',['-NoProfile','-Command',`$listener=Get-NetTCPConnection -LocalPort ${port} -State Listen -ErrorAction SilentlyContinue;if($listener){$proc=Get-Process -Id $listener.OwningProcess;@{pid=$proc.Id;path=$proc.Path}|ConvertTo-Json -Compress}`],{encoding:'utf8',windowsHide:true});
 if(inspect.stdout.trim()){const old=JSON.parse(inspect.stdout);if(old.path.toLowerCase()!==exe.toLowerCase())throw new Error('Port belongs to another program: '+port);return old.pid;}
 const log=fs.openSync(path.join(dir,label+'.log'),'a');const child=spawn(exe,args,{cwd:root,detached:true,windowsHide:true,stdio:['ignore',log,log]});child.unref();fs.closeSync(log);return child.pid;
}
async function ready(port){for(let i=0;i<100;i++){if(await new Promise(resolve=>{const s=net.connect(port,'127.0.0.1');s.setTimeout(200);s.once('connect',()=>{s.destroy();resolve(true);});s.once('error',()=>resolve(false));s.once('timeout',()=>{s.destroy();resolve(false);});}))return;await new Promise(r=>setTimeout(r,200));}throw new Error('Port unavailable: '+port);}
(async()=>{
 if(process.platform!=='win32')throw new Error('This host launcher requires Windows + WSL Debian');
 const flags=process.argv.slice(2);if(flags.length)throw new Error('Development consent flags are removed; USB FIDO2 is mandatory');
 fs.mkdirSync(dir,{recursive:true});
 // WSL may shut down its background daemon when its final foreground client
 // exits. Keep a detached client alive for this explicitly started dev stack.
 const keepLog=fs.openSync(path.join(dir,'wsl-keepalive.log'),'a');
 const keeper=spawn(wsl,['-d','Debian','--','sleep','infinity'],{detached:true,windowsHide:true,stdio:['ignore',keepLog,keepLog]});keeper.unref();fs.closeSync(keepLog);
 const required=['pcb.public','lfs.public','pcb.seed','lfs.seed'];for(const f of required)if(!fs.existsSync(path.join(dir,f)))throw new Error('Enrollment missing: '+f);
 const activeBeforeMigration=docker(['ps','--filter','name=^vela-v1-vm$','--format','{{.Names}}']);
 if(activeBeforeMigration && fs.statSync(path.join(dir,'disk.img')).size===4096)throw new Error('Stop vela-v1-vm before archive migration');
 const disk=path.join(dir,'disk.img');if(!fs.existsSync(disk)){const fd=fs.openSync(disk,'wx');fs.ftruncateSync(fd,131072);fs.fsyncSync(fd);fs.closeSync(fd);}else if(fs.statSync(disk).size===4096){fs.copyFileSync(disk,disk+'.pre-archive.bak',fs.constants.COPYFILE_EXCL);const fd=fs.openSync(disk,'r+');fs.ftruncateSync(fd,131072);fs.fsyncSync(fd);fs.closeSync(fd);}else if(fs.statSync(disk).size!==131072)throw new Error('Only dedicated archive disks are accepted');
 const running=docker(['ps','--filter','name=^vela-v1-vm$','--format','{{.Names}}']);
 if(!running){
  const args=['run','-d','--rm','--name','vela-v1-vm','-p','127.0.0.1:7991:7991','-v',linuxPath(dir)+':/data','velaos-v1','qemu-system-aarch64','-machine','virt,virtualization=on,gic-version=2','-cpu','cortex-a53','-m','2G','-display','none','-monitor','none','-serial','file:/data/boot-final.log','-net','none','-kernel','/work/build/v1-loader.img','-global','virtio-mmio.force-legacy=false','-chardev','socket,id=host,host=0.0.0.0,port=7991,server=on,wait=off','-device','virtio-serial-device,id=v0','-device','virtconsole,chardev=host,bus=v0.0'];
  for(let i=0;i<7;i++)args.push('-device','virtio-serial-device');
  args.push('-drive','file=/data/disk.img,if=none,id=lfs,format=raw,cache=none','-device','virtio-blk-device,drive=lfs');docker(args);
 }
 await ready(7991);
 // Reuse verified listeners rather than starting duplicate native processes.
 const brokerPid=native('poc/host-broker/target/debug/vela-host-broker.exe',['serve','.local/host-bridge/broker.seed','.local/v1/pcb.public','.local/v1/lfs.public','.local/host-bridge/fido.json'],'broker-final',7990);
 const glassPid=native('poc/product/glass/target/debug/vela-glass.exe',['--port','7882'],'glass-final',7882);
 await ready(7990);await ready(7882);
 fs.writeFileSync(path.join(dir,'running.json'),JSON.stringify({brokerPid,glassPid,wslKeeperPid:keeper.pid,consentMode:'hardware-fido2',fidoEnrolled:fs.existsSync(path.join(root,'.local/host-bridge/fido.json')),url:'http://127.0.0.1:7882/studio'},null,2));
 console.log('READY http://127.0.0.1:7882/studio ; consent='+ 'hardware-fido2');
})().catch(e=>{console.error(e.message);process.exitCode=1;});
