const {chromium}=require('playwright');
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto');
(async()=>{
 const output=process.argv[2];if(!output)throw new Error('Expected evidence directory');
 const browser=await chromium.launch({channel:'msedge',headless:true});
 try{
  const page=await browser.newPage({viewport:{width:1440,height:1000}});let frames=0;
  page.on('websocket',ws=>ws.on('framesent',()=>frames++));
  await page.goto('http://127.0.0.1:7882/studio');
  await page.screenshot({path:path.join(output,'redesign-idle.png')});
  await page.fill('#intent','Appunta un progetto di bridge');await page.press('#intent','Enter');
  await page.waitForSelector('#approval:visible');
  await page.fill('#draft','é'.repeat(600));
  await page.waitForFunction(()=>document.getElementById('planDigest').textContent.includes('oltre il limite'));
  if(!await page.isDisabled('#approve'))throw new Error('Oversize payload can be approved');
  await page.fill('#draft','Payload esatto <script> — nessuna interpretazione HTML.');
  const expected=crypto.createHash('sha256').update(await page.inputValue('#draft')).digest('hex');
  await page.waitForFunction(hash=>document.getElementById('planDigest').textContent===hash,expected);
  await page.screenshot({path:path.join(output,'redesign-proposal.png')});
  await page.click('#reject');
  if(frames!==0||await page.locator('#timeline .entry').count()!==0)throw new Error('Reject emitted a transaction');
  // Isolated UI fault injection: transport ACK cannot complete a transaction.
  // This context is separate from the real-broker E2E test.
  const ackPage=await browser.newPage();
  await ackPage.addInitScript(()=>{
   window.WebSocket=class {constructor(){setTimeout(()=>this.onopen?.(),0);}send(){setTimeout(()=>this.onmessage?.({data:JSON.stringify({type:'Ack'})}),0);}close(){}};
  });
  await ackPage.goto('http://127.0.0.1:7882/studio');await ackPage.fill('#intent','Appunta ACK test');await ackPage.press('#intent','Enter');
  await ackPage.waitForFunction(()=>!document.getElementById('approve').disabled);await ackPage.click('#approve');
  await ackPage.waitForFunction(()=>document.getElementById('status').textContent.includes('conferma di trasporto'));
  if(await ackPage.locator('#timeline .entry').count()!==0||!await ackPage.locator('#approval').isVisible()||!await ackPage.isDisabled('#draft'))throw new Error('ACK completed or unlocked the transaction');
  await page.setViewportSize({width:390,height:844});
  if(await page.evaluate(()=>document.documentElement.scrollWidth>window.innerWidth))throw new Error('Mobile horizontal overflow');
  await page.screenshot({path:path.join(output,'redesign-mobile.png')});
  const result={reject_sends_no_frames:true,utf8_limit:true,exact_sha256:true,ack_is_not_receipt:true,payload_locked_while_loading:true,mobile_no_overflow:true};
  fs.writeFileSync(path.join(output,'redesign-ui-result.json'),JSON.stringify(result,null,2));console.log(JSON.stringify(result));
 }finally{await browser.close();}
})().catch(e=>{console.error(e);process.exitCode=1;});
