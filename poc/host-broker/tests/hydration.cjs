// Real guest index; no localStorage, fake receipts or WebSocket mocks.
const {chromium}=require('playwright');
const fs=require('node:fs'),path=require('node:path');
(async()=>{
 const output=process.argv[2];const browser=await chromium.launch({channel:'msedge',headless:true});
 try{
  const page=await browser.newPage({viewport:{width:1280,height:900}});const replies=[];
  await page.addInitScript(()=>window.addEventListener('vela:hydrated',event=>window.hydration=event.detail));
  page.on('websocket',ws=>ws.on('framereceived',frame=>{replies.push(JSON.parse(frame.payload.toString()));}));
  await page.goto('http://127.0.0.1:7882/studio');await page.waitForFunction(()=>Boolean(window.hydration),{},{timeout:25000});
  const first=replies.find(r=>r.type==='ListResponse');if(!first||!first.verified||first.notes.length!==1)throw new Error('Missing authenticated persisted note');
  const request=first.notes[0].request;
  if(await page.locator('#timeline .entry').count()!==1)throw new Error('Feed missing persisted note');
  await page.reload();await page.waitForFunction(()=>Boolean(window.hydration),{},{timeout:25000});
  const last=replies.filter(r=>r.type==='ListResponse').at(-1);
  if(last.notes[0].request!==request||last.notes[0].receipt_hex!==first.notes[0].receipt_hex)throw new Error('Historical receipt changed on reload');
  await page.screenshot({path:path.join(output,'hydrated-timeline.png')});
  fs.writeFileSync(path.join(output,'hydration-result.json'),JSON.stringify({authenticated:true,page_reload:true,note_count:1,request,receipt_unchanged:true,preview:last.notes[0].preview},null,2));
  console.log(JSON.stringify({authenticated:true,page_reload:true,note_count:1,receipt_unchanged:true}));
 }finally{await browser.close();}
})().catch(e=>{console.error(e);process.exitCode=1;});
