"""Local desktop integration. All mutations use explicit capabilities, never shell text."""
import configparser, os, pathlib, shlex, shutil, subprocess, threading, uuid

CATALOG = {
 'firefox-esr': ('Firefox', 'Navigazione web', 'compass'),
 'chromium': ('Chromium', 'Navigazione web', 'compass'),
 'vlc': ('VLC', 'Video e musica', 'note'),
 'libreoffice': ('LibreOffice', 'Documenti, fogli e presentazioni', 'note'),
 'gimp': ('GIMP', 'Modifica immagini', 'note'),
 'mousepad': ('Mousepad', 'Editor di testo', 'note'),
}
PANELS = {
 'display': ['xfce4-display-settings'], 'keyboard': ['xfce4-keyboard-settings'],
 'mouse': ['xfce4-mouse-settings'], 'appearance': ['xfce4-appearance-settings'],
 'audio': ['pavucontrol'], 'network': ['nm-connection-editor'],
 'packages': ['sudo', '-n', 'synaptic'],
}
JOBS = {}; LOCK = threading.Lock()

def env():
 e = os.environ.copy(); e.setdefault('DISPLAY', ':0'); return e

def run(args, timeout=5):
 return subprocess.run(args, capture_output=True, text=True, timeout=timeout, env=env())

def desktop_apps():
 found = {}
 for folder in [pathlib.Path('/usr/share/applications'), pathlib.Path.home()/'.local/share/applications']:
  for p in folder.glob('*.desktop'):
   try:
    c = configparser.ConfigParser(interpolation=None, strict=False); c.read(p, encoding='utf-8')
    d = c['Desktop Entry']; key = p.stem
    if d.get('Hidden','false')=='true': found.pop(key,None); continue
    if d.get('Type')!='Application' or d.get('NoDisplay','false')=='true' or d.get('Terminal','false')=='true': continue
    args = shlex.split(d.get('Exec',''))
    if not args or not shutil.which(args[0]) or (d.get('TryExec') and not shutil.which(d['TryExec'])): continue
    name = d.get('Name[it]', d.get('Name','Applicazione'))
    found[key] = {'id':key, 'name':name, 'icon':'compass' if 'firefox' in key or 'chromium' in key else 'note', 'args':[a.replace('%%','%') for a in args if not a.startswith('%')]}
   except (KeyError, ValueError, OSError, configparser.Error): continue
 return found

def catalog():
 result = []
 for package,(name,description,icon) in CATALOG.items():
  installed = False
  if shutil.which('dpkg-query'):
   r = run(['dpkg-query','-W','-f=${db:Status-Status}',package])
   installed = r.returncode==0 and r.stdout.strip()=='installed'
  result.append(dict(package=package,name=name,description=description,icon=icon,installed=installed))
 return result

def install(package):
 if package not in CATALOG: raise ValueError('Questa applicazione non è nel catalogo.')
 with LOCK:
  if any(j['state']=='running' for j in JOBS.values()): raise ValueError('Attendi che l’installazione in corso finisca.')
  key=uuid.uuid4().hex; JOBS[key]={'state':'running','message':'Aggiorno il catalogo…','package':package,'log':''}
 def work():
  try:
   for args,message in [(['sudo','-n','apt-get','update'],'Aggiorno il catalogo…'), (['sudo','-n','apt-get','install','-y',package],'Scarico e installo l’app…')]:
    JOBS[key]['message']=message
    e=env(); e['DEBIAN_FRONTEND']='noninteractive'
    with subprocess.Popen(args,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True,env=e) as p:
     for line in p.stdout: JOBS[key]['log']=(JOBS[key]['log']+line)[-16000:]
     if p.wait()!=0: raise ValueError('Installazione non riuscita. Controlla la connessione e lo spazio disponibile; i dettagli sono qui sotto.')
   JOBS[key].update(state='done',message='Installazione completata. L’app è disponibile in Applicazioni.')
  except (OSError,ValueError) as ex: JOBS[key].update(state='error',message=str(ex))
 threading.Thread(target=work,daemon=True).start(); return {'job':key}

def get(path, query):
 if path=='/api/native-windows':
  windows=[]
  if shutil.which('wmctrl'):
   for line in run(['wmctrl','-l']).stdout.splitlines():
    cols=line.split(None,3)
    if len(cols)==4 and 'Vela — anteprima della shell' not in cols[3]: windows.append({'id':cols[0],'title':cols[3]})
  return {'windows':windows}
 if path=='/api/apps': return {'apps':[{k:v for k,v in app.items() if k!='args'} for app in desktop_apps().values()]}
 if path=='/api/store': return {'items':catalog()}
 if path=='/api/store-job':
  job=JOBS.get(query.get('id',[''])[0])
  if not job: raise ValueError('Installazione non disponibile.')
  return dict(job)
 if path=='/api/system':
  disk=shutil.disk_usage(pathlib.Path.home())
  network=run(['nmcli','-t','-f','DEVICE,TYPE,STATE','device']).stdout.strip() if shutil.which('nmcli') else ''
  return {'network':network, 'disk_free_gib':round(disk.free/2**30,1),'disk_total_gib':round(disk.total/2**30,1), 'panels':{k:bool(shutil.which(v[0]) and (k!='packages' or shutil.which('synaptic'))) for k,v in PANELS.items()}}
 return None

def post(path,data):
 if path=='/api/native-window':
  key=data.get('id'); available={w['id'] for w in get('/api/native-windows',{})['windows']}
  if key not in available: raise ValueError('Questa finestra è chiusa.')
  if data.get('action') not in ('activate','close'): raise ValueError('Azione non disponibile.')
  run(['wmctrl','-i','-a' if data['action']=='activate' else '-c',key]); return {'ok':True}
 if path=='/api/launch':
  app=desktop_apps().get(data.get('id'))
  if not app: raise ValueError('L’app non è più installata. Aggiorna l’elenco.')
  args=app['args']
  if pathlib.Path(args[0]).name=='chromium': args=args+['--user-data-dir='+str(pathlib.Path.home()/'.config/vela-browser'),'--new-window']
  subprocess.Popen(args,env=env(),stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL,start_new_session=True)
  return {'ok':True}
 if path=='/api/system-panel':
  args=PANELS.get(data.get('panel'))
  if not args or not shutil.which(args[0]): raise ValueError('Questo pannello non è ancora disponibile.')
  subprocess.Popen(args,env=env(),stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL,start_new_session=True); return {'ok':True}
 if path=='/api/store-install': return install(data.get('package'))
 if path=='/api/clipboard':
  if not shutil.which('xclip'): raise ValueError('Gli appunti di sistema non sono disponibili.')
  if 'text' in data:
   value=data['text']
   if not isinstance(value,str) or len(value)>60000: raise ValueError('Seleziona un testo più breve.')
   p=subprocess.Popen(['xclip','-selection','clipboard'],stdin=subprocess.PIPE,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL,env=env()); p.stdin.write(value.encode()); p.stdin.close()
   return {'ok':True}
  r=run(['xclip','-selection','clipboard','-o']); return {'text':r.stdout[:60000] if r.returncode==0 else ''}
 return None
