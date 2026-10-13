// Uses the real Glass page, real broker, and real QEMU. No WebSocket mocks.
const { chromium } = require('playwright');
const fs = require('node:fs');
const path = require('node:path');
(async () => {
 const output=process.argv[2];if(!output)throw new Error('Expected evidence directory');
 const browser=await chromium.launch({channel:'msedge',headless:true});
 const page=await browser.newPage({viewport:{width:1280,height:800}});
 const responses=[],errors=[],sent=[];
 page.on('pageerror',e=>errors.push(e.message));
 page.on('websocket',ws=>{ws.on('framereceived',frame=>{try{responses.push(JSON.parse(frame.payload.toString()));}catch{}});ws.on('framesent',frame=>{sent.push(JSON.parse(frame.payload.toString()));});});
 try{
  await page.addInitScript(()=>window.addEventListener('vela:completed',event=>{window.completedReceipt=event.detail;}));
  await page.goto('http://127.0.0.1:7882/studio');
  await page.fill('#intent','Appunta le idee per il bridge IPC');
  await page.press('#intent','Enter');
  await page.waitForSelector('#draft:visible');
  await page.fill('#draft','Vela real browser → Ed25519 → Microkit → Wasm → durable VirtIO note. '+new Date().toISOString());
  const note=await page.inputValue('#draft');
  await page.click('#approve');
  await page.waitForFunction(()=>Boolean(window.completedReceipt),{},{timeout:45000});
  const receipt=responses.find(r=>r.type==='Receipt');
  if(!receipt||!receipt.verified||errors.length)throw new Error(JSON.stringify({responses,errors}));
  const expectedKeys=['intent','note','note_sha256','operation','type'];
  if(sent.length!==1||JSON.stringify(Object.keys(sent[0]).sort())!==JSON.stringify(expectedKeys)||sent[0].note!==note||sent[0].operation!=='write-note'||sent[0].type!=='Proposal')throw new Error('Protocol changed');
  if(await page.locator('#approval').isVisible())throw new Error('Approval prompt did not disappear');
  if(await page.locator('#timeline .entry').count()!==1)throw new Error('Missing timeline entry');
  await page.click('[data-context="Sistema"]');
  await page.waitForFunction(()=>document.querySelectorAll('#timeline .entry').length===0);
  if(await page.locator('#timeline .entry').count()!==0)throw new Error('Context filter failed');
  await page.click('[data-context="Tutto"]');
  await page.waitForFunction(()=>document.querySelectorAll('#timeline .entry').length===1);
  if(await page.locator('#timeline .entry').count()!==1)throw new Error('All-context filter failed');
  fs.writeFileSync(path.join(output,'ui-result.json'),JSON.stringify({note,phase:'Completed',receipt,errors,timeline_entries:1,context_filters:true,prompt_dismissed:true,protocol_unchanged:true},null,2));
  await page.screenshot({path:path.join(output,'glass-receipt.png')});
  console.log(JSON.stringify({phase:'Completed',request:receipt.request,consent:receipt.consent,receipt_bytes:receipt.receipt_hex.length/2}));
 }catch(e){console.error(JSON.stringify({responses,errors,error:e.message}));process.exitCode=1;}
 finally{await browser.close();}
})();
