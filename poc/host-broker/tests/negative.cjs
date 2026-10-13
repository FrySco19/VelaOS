// Test-only signing with the enrolled development seed. Never human consent.
const fs=require('node:fs'), net=require('node:net'), crypto=require('node:crypto');
const {chromium}=require('playwright');
const path=require('node:path');
const sha=b=>crypto.createHash('sha256').update(b).digest();
const u64=n=>{const b=Buffer.alloc(8);b.writeBigUInt64LE(BigInt(n));return b;};
const wasm=Buffer.from([0,97,115,109,1,0,0,0,1,12,2,96,2,127,127,1,127,96,1,127,1,127,2,13,1,3,108,102,115,5,119,114,105,116,101,0,0,3,2,1,1,5,4,1,1,1,1,7,16,2,6,109,101,109,111,114,121,2,0,3,114,117,110,0,1,10,10,1,8,0,65,0,32,0,16,0,11]);
function frame(kind,id,payload) {const h=Buffer.alloc(32);h.write('VELA');h.writeUInt16LE(1,4);h.writeUInt16LE(kind,6);h.writeUInt16LE(payload.length,8);h.writeBigUInt64LE(BigInt(id),12);return Buffer.concat([h,payload]);}
async function malformedApproval(seed,mode,id) {
 const note=Buffer.from('Rejected guest tamper test'),session=crypto.randomBytes(32);
 const manifest=Buffer.concat([Buffer.from([1,0]),u64(1),u64(id),sha(Buffer.from('/lfs/notes/note1.txt')).subarray(0,16),sha(note),sha(wasm)]);
 const proposal=Buffer.concat([u64(1),u64(id),session,Buffer.from([note.length,0]),note]);
 const hash=sha(manifest);
 const key=crypto.createPrivateKey({key:Buffer.concat([Buffer.from('302e020100300506032b657004220420','hex'),fs.readFileSync(seed)]),type:'pkcs8',format:'der'});
 const sig=crypto.sign(null,Buffer.concat([Buffer.from('VelaOS.Glass.UserSelection.v1\0'),Buffer.from([1,0]),Buffer.from('approve\0'),session,u64(id),hash]),key);
 if(mode==='signature')sig[0]^=1;else hash[0]^=1;
 const approval=Buffer.concat([Buffer.from([1,0]),session,u64(id),hash,sig]);
 return new Promise((resolve,reject)=>{
  const s=net.connect(7991,'127.0.0.1'),received=[];
  s.setTimeout(1500);s.on('connect',()=>s.write(Buffer.concat([frame(1,id,proposal),frame(2,id,approval)])));
  s.on('data',b=>received.push(b));s.on('error',reject);
  s.on('timeout',()=>{s.destroy();const bytes=Buffer.concat(received);if(bytes.length)reject(new Error('Unexpected guest output on tamper'));else resolve();});
 });
}
(async()=>{
 const root=process.argv[2],seed=process.argv[3];if(!root||!seed)throw new Error('Expected evidence dir and development seed');
 const disk=path.join(root,'disk.img'),before=sha(fs.readFileSync(disk));
 const browser=await chromium.launch({channel:'msedge',headless:true});
 try{
  const page=await browser.newPage();await page.goto('http://127.0.0.1:7882/studio');
  const denial=await page.evaluate(()=>new Promise((resolve,reject)=>{const ws=new WebSocket('ws://127.0.0.1:7990/');ws.onopen=()=>ws.send(JSON.stringify({type:'Proposal',operation:'write-note',intent:'tamper test',note:'changed note',note_sha256:'0'.repeat(64)}));ws.onerror=()=>reject(new Error('transport'));ws.onmessage=e=>{resolve(JSON.parse(e.data));ws.close();};}));
  if(denial.type!=='Denied'||denial.reason!=='Proposal hash mismatch')throw new Error(JSON.stringify(denial));
  await malformedApproval(seed,'signature',Date.now()+1);
  await malformedApproval(seed,'hash',Date.now()+2);
  if(!before.equals(sha(fs.readFileSync(disk))))throw new Error('Disk changed after rejected requests');
  const result={broker_hash_rejected:true,guest_tampered_signature_no_receipt:true,guest_tampered_hash_no_receipt:true,disk_unchanged:true};
  fs.writeFileSync(path.join(root,'negative-result.json'),JSON.stringify(result,null,2));console.log(JSON.stringify(result));
 }finally{await browser.close();}
})().catch(e=>{console.error(e);process.exitCode=1;});
