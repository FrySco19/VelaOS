// Desktop capabilities shared with the actual Linux session.
let linuxApps=[], systemState=null;
let externalWindows=[],appSignature='';
const baseDock=renderDock;
renderDock=function(){baseDock();const list=$('.tasklist');externalWindows.forEach(w=>{const b=document.createElement('button');b.className='task';b.innerHTML=svg('compass')+'<span>'+esc(w.title)+'</span>';b.title=w.title;b.onclick=()=>api('native-window',{id:w.id,action:'activate'}).catch(e=>toast(e.message));b.oncontextmenu=e=>{e.preventDefault();showMenu([['Apri finestra',()=>b.click()],['Chiudi finestra',()=>api('native-window',{id:w.id,action:'close'}).catch(err=>toast(err.message))]])};list.append(b)})};
async function refreshNativeWindows(){if(!native)return;try{const r=await api('native-windows');if(JSON.stringify(r.windows)!==JSON.stringify(externalWindows)){externalWindows=r.windows;renderDock()}}catch{}}
document.body.classList.toggle('effects',localStorage.getItem('vela-effects')==='true');
document.body.classList.toggle('desktop-icons',localStorage.getItem('vela-desktop-icons')!=='false');
const iconDefs=document.querySelector('svg defs');
iconDefs.insertAdjacentHTML('beforeend','<symbol id="store" viewBox="0 0 24 24"><path d="M3 8h18v13H3zM7 8V6a5 5 0 0 1 10 0v2M8 13l4 4 4-4M12 10v7"/></symbol>');
async function refreshApps(){
 const r=await api('apps');linuxApps=r.apps;
 const signature=JSON.stringify(r.apps);if(signature===appSignature)return;appSignature=signature;
 Object.keys(apps).filter(id=>id.startsWith('native:')).forEach(id=>delete apps[id]);
 linuxApps.forEach(a=>apps['native:'+a.id]=[a.name,a.icon]);renderDock();
 $('#desktopApps').innerHTML=linuxApps.filter(a=>/firefox|chromium/.test(a.id)).map(a=>`<button class="desktop-app" data-native="${esc(a.id)}">${svg(a.icon)}<span>${esc(a.name)}</span></button>`).join('');
 $('#desktopApps').querySelectorAll('[data-native]').forEach(b=>b.onclick=()=>launchNative(b.dataset.native));
}
async function launchNative(id){try{await api('launch',{id});toast('Apro '+(linuxApps.find(a=>a.id===id)?.name||'l’app')+'…')}catch(e){toast(e.message)}}
async function systemPanel(panel){try{await api('system-panel',{panel});toast('Apro le impostazioni di sistema…')}catch(e){toast(e.message)}}
async function copyText(text){if(!text)return;try{if(native)await api('clipboard',{text});else await navigator.clipboard.writeText(text);toast('Testo copiato.')}catch(e){toast('Non riesco a copiare il testo. '+e.message)}}
async function readClipboard(){return native?(await api('clipboard',{})).text:await navigator.clipboard.readText()}

document.addEventListener('contextmenu',e=>{
 if(e.target.closest('input,textarea,.xterm,.preview'))return;
 if(e.target.closest('#desktop')&&!e.target.closest('.window')){
  e.preventDefault();showMenu([['Apri Documenti',()=>openApp('files')],['Applicazioni',()=>porto()],['Store',()=>openApp('store')],['Terminale',()=>openApp('terminal')],['Nuova cartella',async()=>{openApp('files');await current.fileAction('folder')}],['Personalizza desktop',()=>{openApp('settings');settings(current,current.querySelector('.content'),'Scrivania')}],['Aggiorna applicazioni',()=>refreshApps().catch(err=>toast(err.message))]]);
  const m=$('#systemMenu');m.style.top=Math.min(e.clientY,innerHeight-360)+'px';m.style.left=Math.min(e.clientX,innerWidth-250)+'px';
 }
});
document.addEventListener('keydown',e=>{
 const editable=e.target.closest('input,textarea,[contenteditable],.terminal-host');
 if(!editable&&current?.dataset.id==='files'&&e.ctrlKey&&['c','v','z'].includes(e.key.toLowerCase())){e.preventDefault();current.fileAction({c:'copy',v:'paste',z:'undo'}[e.key.toLowerCase()])}
});

const originalSettings=settings;
settings=function(w,c,page='Aspetto'){
 const pages=['Aspetto','Scrivania','Prestazioni','Schermo','Audio','Rete','Tastiera','Mouse e touchpad','Applicazioni','Archiviazione','Accessibilità','Privacy','Informazioni'];
 originalSettings(w,c,['Aspetto','Accessibilità','Privacy','Informazioni'].includes(page)?page:'Informazioni');
 const sidebar=c.querySelector('.sidebar');sidebar.innerHTML='<input class="settings-search" aria-label="Cerca nelle impostazioni" placeholder="Cerca impostazioni">'+pages.map(p=>`<button data-page="${p}" class="${p===page?'selected':''}">${p}</button>`).join('');
 sidebar.querySelector('input').oninput=e=>sidebar.querySelectorAll('button').forEach(b=>b.hidden=!b.textContent.toLowerCase().includes(e.target.value.toLowerCase()));
 sidebar.querySelectorAll('button').forEach(b=>b.onclick=()=>settings(w,c,b.dataset.page));
 const body=c.querySelector('.settings-body');
 const panelPage={'Schermo':['display','Risoluzione e disposizione degli schermi'],'Audio':['audio','Volume, ingressi e uscite audio'],'Rete':['network','Connessioni Ethernet e Wi-Fi'],'Tastiera':['keyboard','Layout, ripetizione e scorciatoie'],'Mouse e touchpad':['mouse','Velocità, pulsanti e scorrimento']};
 if(panelPage[page]){const [key,text]=panelPage[page];body.innerHTML=`<h2>${page}</h2><div class="card"><h3>${text}</h3><p>Modifica i dispositivi reali della sessione Linux.</p><button class="primary" data-panel="${key}">Apri ${page.toLowerCase()}</button><p class="hint">Il pannello si apre in una finestra del sistema.</p></div>`;body.querySelector('button').onclick=()=>systemPanel(key);if(page==='Rete')api('system').then(r=>{if(c.isConnected&&body.querySelector('.card')){const pre=document.createElement('pre');pre.textContent=r.network||'Nessuna interfaccia rilevata.';body.querySelector('.card').append(pre)}}).catch(e=>toast(e.message))}
 if(page==='Scrivania'||page==='Prestazioni'){
  const controls=page==='Scrivania'?[['desktop-icons','Mostra icone delle applicazioni sul desktop']]:[['effects','Trasparenze e sfocature (richiedono più risorse)']];
  body.innerHTML=`<h2>${page}</h2><div class="card">${controls.map(([key,label])=>`<div class="row"><label for="pref-${key}">${label}</label><input id="pref-${key}" type="checkbox" data-pref="${key}" ${document.body.classList.contains(key)?'checked':''}></div>`).join('')}${page==='Prestazioni'?'<p>La modalità leggera è attiva quando le sfocature sono disabilitate. È la scelta predefinita per VirtualBox.</p><p>Le animazioni si possono ridurre in Accessibilità.</p>':'<p>Le app installate vengono rilevate automaticamente. Usa il tasto destro sulla scrivania per aprire il menu.</p>'}</div>`;
  body.querySelectorAll('[data-pref]').forEach(b=>b.onchange=()=>{document.body.classList.toggle(b.dataset.pref,b.checked);localStorage.setItem('vela-'+b.dataset.pref,b.checked)});
 }
 if(page==='Applicazioni'){
  body.innerHTML='<h2>Applicazioni</h2><div class="card"><button class="primary" id="open-store">Apri Store</button><button id="packages">Gestione completa dei pacchetti</button><p>Installa le app dal catalogo oppure gestisci i pacchetti Debian con Synaptic.</p></div><div class="installed-list"></div>';
  body.querySelector('#open-store').onclick=()=>openApp('store');body.querySelector('#packages').onclick=()=>systemPanel('packages');
  api('apps').then(r=>{if(!body.querySelector('.installed-list'))return;body.querySelector('.installed-list').innerHTML=r.apps.map(a=>`<div class="row"><span>${esc(a.name)}</span><button data-native="${esc(a.id)}">Apri</button></div>`).join('');body.querySelectorAll('[data-native]').forEach(b=>b.onclick=()=>launchNative(b.dataset.native))}).catch(e=>toast(e.message));
 }
 if(page==='Archiviazione'){
  body.innerHTML='<h2>Archiviazione</h2><div class="card" id="disk">Leggo lo spazio disponibile…</div>';
  api('system').then(r=>{if(body.querySelector('#disk'))body.querySelector('#disk').innerHTML=`<h3>${r.disk_free_gib} GB disponibili su ${r.disk_total_gib} GB</h3><p>Questa VM usa una sessione live in memoria. File, app e impostazioni si perdono allo spegnimento.</p><button onclick="openApp('files')">Apri Documenti</button>`}).catch(e=>toast(e.message));
 }
 if(page==='Aspetto')body.insertAdjacentHTML('beforeend','<div class="card"><h3>Applicazioni Linux</h3><p>Font e colori delle applicazioni esterne.</p><button onclick="systemPanel(\'appearance\')">Aspetto del sistema</button></div>');
 if(page==='Informazioni')body.innerHTML='<h2>Vela</h2><div class="card"><h3>Anteprima · Desktop e applicazioni</h3><p>Debian 13 · Firefox, Store e impostazioni di sistema.</p><p>La sessione è live: le installazioni e i dati non vengono conservati dopo lo spegnimento.</p></div>';
};

async function store(w,c){
 c.innerHTML='<div class="store-body"><div class="store-heading"><div><p class="eyebrow">SOFTWARE DEBIAN</p><h2>Store</h2></div><button class="refresh-store">Aggiorna</button></div><input class="store-search" aria-label="Cerca nello Store" placeholder="Cerca un’app"><p>App dai repository configurati di Debian. Le installazioni sono temporanee in questa sessione live.</p><div class="store-grid">Carico il catalogo…</div><div class="install-status" role="status"></div><details><summary>Dettagli installazione</summary><pre class="install-log"></pre></details><button class="all-packages">Tutti i pacchetti · Synaptic</button></div>';
 c.querySelector('.refresh-store').onclick=()=>store(w,c);c.querySelector('.all-packages').onclick=()=>systemPanel('packages');
 try{
  const {items}=await api('store');
  const draw=q=>{const grid=c.querySelector('.store-grid');grid.innerHTML=items.filter(a=>(a.name+' '+a.description).toLowerCase().includes(q.toLowerCase())).map(a=>`<article class="store-card"><span class="icon">${svg(a.icon)}</span><h3>${esc(a.name)}</h3><p>${esc(a.description)}</p><button class="${a.installed?'':'primary'}" data-package="${a.package}" data-installed="${a.installed}">${a.installed?'Apri':'Installa'}</button></article>`).join('');grid.querySelectorAll('[data-package]').forEach(b=>b.onclick=async()=>{try{if(b.dataset.installed==='true'){await refreshApps();const key=b.dataset.package;const app=linuxApps.find(a=>a.id===key||a.id.includes(key)||key==='libreoffice'&&a.id.includes('libreoffice-writer')||key==='firefox-esr'&&a.id.includes('firefox'));if(app)return launchNative(app.id);return toast('Apri l’app da Applicazioni.')}if(!await confirmAction('Installare '+items.find(a=>a.package===b.dataset.package).name+' dai repository Debian?'))return;const r=await api('store-install',{package:b.dataset.package});b.disabled=true;trackInstall(r.job,w,c)}catch(e){toast(e.message)}})};
  draw('');c.querySelector('.store-search').oninput=e=>draw(e.target.value);
 }catch(e){c.querySelector('.store-grid').textContent=e.message}
}
async function trackInstall(id,w,c){
 try{const r=await api('store-job?id='+encodeURIComponent(id));if(!w.isConnected)return;const status=c.querySelector('.install-status'),log=c.querySelector('.install-log');if(status)status.textContent=r.message;if(log)log.textContent=r.log;if(r.state==='running'){setTimeout(()=>trackInstall(id,w,c),1000);return}await refreshApps();if(r.state==='done'){toast(r.message);c.querySelectorAll(`[data-package="${r.package}"]`).forEach(b=>{b.disabled=false;b.dataset.installed='true';b.textContent='Apri'})}else{toast(r.message);c.querySelectorAll('button[disabled]').forEach(b=>b.disabled=false)}}catch(e){toast(e.message)}
}
refreshApps().catch(()=>{});
setInterval(()=>{if(!document.hidden)refreshApps().catch(()=>{})},20000);
setInterval(()=>{if(!document.hidden)refreshNativeWindows()},3000);
