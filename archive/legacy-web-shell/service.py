"""Vela preview service. Loopback, session cookie, bounded file capabilities and user PTYs."""
import base64, codecs, json, mimetypes, os, pathlib, secrets, shutil, signal, subprocess, threading, urllib.request
from http.server import ThreadingHTTPServer, SimpleHTTPRequestHandler
from urllib.parse import parse_qs, urlparse
import system_api
ROOT = pathlib.Path(__file__).parent / 'shell'
HOME = pathlib.Path(os.environ.get('VELA_HOME', str(pathlib.Path.home())))
ORIGIN = 'http://127.0.0.1:8766'
TOKEN = secrets.token_urlsafe(32)
TERMS = {}; TERM_LOCK = threading.Lock()
FOLDERS = {'Documents':'Documenti', 'Downloads':'Download', 'Pictures':'Immagini'}

def safe_path(value):
    rel = pathlib.Path(value)
    if rel.is_absolute() or not rel.parts or rel.parts[0] not in FOLDERS or '..' in rel.parts:
        raise ValueError('Questa posizione non è disponibile.')
    current = HOME
    for part in rel.parts:
        current = current / part
        if current.is_symlink(): raise ValueError('Questo collegamento non può essere aperto.')
    return current

def metrics():
    if not pathlib.Path('/proc/uptime').exists(): return {}
    mem = {line.split(':')[0]:int(line.split()[1]) for line in pathlib.Path('/proc/meminfo').read_text().splitlines() if len(line.split())>1 and line.split()[1].isdigit()}
    return {'uptime_seconds':float(pathlib.Path('/proc/uptime').read_text().split()[0]),'used_memory_mib':(mem['MemTotal']-mem['MemAvailable'])/1024}

def close_term(key):
    with TERM_LOCK: session = TERMS.pop(key, None)
    if session:
        try: os.killpg(session['pid'], signal.SIGHUP)
        except ProcessLookupError: pass
        os.close(session['fd'])
        try: os.waitpid(session['pid'], os.WNOHANG)
        except ChildProcessError: pass

class Handler(SimpleHTTPRequestHandler):
    def __init__(self,*a,**kw): super().__init__(*a,directory=str(ROOT),**kw)
    def end_headers(self):
        self.send_header('Cache-Control','no-store')
        self.send_header('X-Content-Type-Options','nosniff')
        self.send_header('Content-Security-Policy',"default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; frame-src 'self'; object-src 'self'; connect-src 'self'; frame-ancestors 'none'")
        super().end_headers()
    def send_json(self,obj,status=200):
        b=json.dumps(obj).encode(); self.send_response(status); self.send_header('Content-Type','application/json'); self.send_header('Content-Length',str(len(b))); self.end_headers(); self.wfile.write(b)
    def authorized(self):
        from http.cookies import SimpleCookie
        cookie=SimpleCookie()
        try: cookie.load(self.headers.get('Cookie',''))
        except Exception: return False
        return 'vela_session' in cookie and secrets.compare_digest(cookie['vela_session'].value,TOKEN)
    def do_GET(self):
        if self.headers.get('Host') != '127.0.0.1:8766': return self.send_json({'error':'Usa la sessione locale di Vela.'},403)
        parsed=urlparse(self.path); query=parse_qs(parsed.query)
        if parsed.path == '/api/session':
            if self.headers.get('Sec-Fetch-Site') not in (None,'same-origin','none'): return self.send_json({'error':'Richiesta non consentita.'},403)
            self.send_response(200); self.send_header('Set-Cookie',f'vela_session={TOKEN}; HttpOnly; SameSite=Strict; Path=/api'); self.send_header('Content-Length','2'); self.end_headers(); self.wfile.write(b'{}'); return
        if parsed.path.startswith('/api/') and not self.authorized(): return self.send_json({'error':'Riapri la sessione di Vela.'},401)
        try:
            extra = system_api.get(parsed.path, query) if parsed.path.startswith('/api/') else None
            if extra is not None: return self.send_json(extra)
            if parsed.path == '/api/files':
                value=query.get('path',['Documents'])[0]; p=safe_path(value); p.mkdir(exist_ok=True)
                items=[{'name':f.name,'path':str(f.relative_to(HOME)).replace('\\','/'),'directory':f.is_dir(),'size':f.stat().st_size} for f in p.iterdir() if not f.is_symlink()]
                return self.send_json({'path':value,'items':sorted(items,key=lambda f:(not f['directory'],f['name'].casefold()))})
            if parsed.path in ('/api/preview','/api/blob'):
                p=safe_path(query.get('path',[''])[0])
                if not p.is_file(): raise ValueError('Questo file non è disponibile.')
                kind=mimetypes.guess_type(p.name)[0] or 'application/octet-stream'
                if parsed.path == '/api/blob':
                    if not (kind.startswith('image/') or kind=='application/pdf') or p.stat().st_size>20*1024*1024: raise ValueError('Il file è troppo grande o non ha un’anteprima disponibile.')
                    b=p.read_bytes(); self.send_response(200); self.send_header('Content-Type',kind); self.send_header('Content-Length',str(len(b))); self.end_headers(); self.wfile.write(b); return
                if kind.startswith('image/') or kind=='application/pdf': return self.send_json({'kind':kind,'url':'/api/blob?path='+__import__('urllib.parse',fromlist=['quote']).quote(str(p.relative_to(HOME)).replace('\\','/'))})
                with p.open(errors='replace') as f: text=f.read(12000)
                return self.send_json({'kind':'text','text':text})
            if parsed.path == '/api/status':
                try:
                    with urllib.request.urlopen('http://127.0.0.1:11434/api/tags',timeout=.4) as r: models=json.load(r).get('models',[])
                except Exception: models=[]
                return self.send_json({'models':[m['name'] for m in models],'native':os.name=='posix' and pathlib.Path('/proc').exists(),'metrics':metrics()})
            return super().do_GET()
        except (ValueError,OSError) as e: return self.send_json({'error':str(e) if isinstance(e,ValueError) else 'Non riesco ad aprire questa posizione.'},400)
    def do_POST(self):
        if self.headers.get('Host') != '127.0.0.1:8766' or self.headers.get('Origin') != ORIGIN: return self.send_json({'error':'Richiesta non consentita.'},403)
        if not self.authorized(): return self.send_json({'error':'Riapri la sessione di Vela.'},401)
        try:
            length=int(self.headers.get('Content-Length',0))
            if not 0<length<=65536: raise ValueError('Richiesta troppo grande.')
            data=json.loads(self.rfile.read(length))
            extra = system_api.post(self.path, data)
            if extra is not None: return self.send_json(extra)
            if self.path == '/api/ready':
                m=metrics()
                if m: pathlib.Path('/tmp/vela-ready.json').write_text(json.dumps({'kernel_to_shell_seconds':m['uptime_seconds'],'used_memory_mib':m['used_memory_mib']}))
                return self.send_json({'ok':True})
            if self.path == '/api/file-action':
                action=data['action']; p=safe_path(data['path'])
                if action in ('rename','copy','trash') and len(pathlib.Path(data['path']).parts)<2: raise ValueError('Scegli un elemento dentro una cartella.')
                if action in ('new','rename','copy'):
                    name=data['name']
                    if not name or name in ('.','..') or '/' in name or '\\' in name or len(name)>200: raise ValueError('Scegli un nome senza barre.')
                    target=p/name if action=='new' else (safe_path(data['destination'])/name if action=='copy' and data.get('destination') else p.parent/name)
                    if target.exists() or target.is_symlink(): raise ValueError('Esiste già un elemento con questo nome.')
                    if action=='new': target.mkdir() if data.get('directory') else target.write_text('',encoding='utf-8')
                    elif action=='rename': p.rename(target)
                    elif p.is_file(): shutil.copyfile(p,target)
                    else: raise ValueError('Puoi duplicare un file alla volta.')
                elif action=='trash':
                    trash=HOME/'.local/share/vela/trash'; trash.mkdir(parents=True,exist_ok=True)
                    key=secrets.token_hex(12); p.rename(trash/key)
                    (trash/(key+'.json')).write_text(json.dumps({'path':data['path']}))
                    return self.send_json({'ok':True,'undo':key})
                elif action=='restore':
                    key=data['key']
                    if len(key)!=24 or any(c not in '0123456789abcdef' for c in key): raise ValueError('Ripristino non disponibile.')
                    trash=HOME/'.local/share/vela/trash'; meta=trash/(key+'.json'); target=safe_path(json.loads(meta.read_text())['path'])
                    if target.exists(): raise ValueError('La posizione originale è già occupata.')
                    (trash/key).rename(target); meta.unlink()
                else: raise ValueError('Azione non disponibile.')
                return self.send_json({'ok':True})
            if self.path.startswith('/api/terminal/'):
                if os.name!='posix': raise ValueError('Il terminale è disponibile nella sessione Linux di Vela.')
                import fcntl, pty, select, struct, termios
                if self.path.endswith('/open'):
                    if len(TERMS)>=16: raise ValueError('Chiudi un terminale prima di aprirne un altro.')
                    cwd=safe_path(data.get('path','Documents')); cwd.mkdir(exist_ok=True)
                    pid,fd=pty.fork()
                    if pid==0:
                        os.chdir(cwd); os.environ['TERM']='xterm-256color'; os.execvp('fish',['fish'])
                    os.set_blocking(fd,False); key=secrets.token_urlsafe(24)
                    with TERM_LOCK: TERMS[key]={'pid':pid,'fd':fd}
                    return self.send_json({'id':key})
                key=data['id']; session=TERMS.get(key)
                if not session: raise ValueError('Questa sessione è chiusa.')
                fd=session['fd']
                if self.path.endswith('/read'):
                    chunks=[]; alive=True
                    for _ in range(16):
                        if not select.select([fd],[],[],0)[0]: break
                        try: b=os.read(fd,4096)
                        except OSError: alive=False; break
                        if not b: alive=False; break
                        chunks.append(b)
                    if not alive: close_term(key)
                    return self.send_json({'data':base64.b64encode(b''.join(chunks)).decode(),'alive':alive})
                if self.path.endswith('/write'): os.write(fd,data['data'].encode())
                elif self.path.endswith('/resize'): fcntl.ioctl(fd,termios.TIOCSWINSZ,struct.pack('HHHH',max(2,min(100,data['rows'])),max(10,min(300,data['cols'])),0,0))
                elif self.path.endswith('/close'): close_term(key)
                else: raise ValueError('Azione non disponibile.')
                return self.send_json({'ok':True})
            if self.path == '/api/power':
                if os.name!='posix' or data.get('action') not in ('poweroff','reboot'): raise ValueError('Questa azione è disponibile nella sessione Linux.')
                subprocess.Popen(['sudo','-n','systemctl',data['action']]); return self.send_json({'ok':True})
            if self.path == '/api/chat':
                payload=json.dumps({'model':data['model'],'messages':[{'role':'user','content':data['prompt']}],'stream':False}).encode()
                req=urllib.request.Request('http://127.0.0.1:11434/api/chat',data=payload,headers={'Content-Type':'application/json'})
                try:
                    with urllib.request.urlopen(req,timeout=120) as r: result=json.load(r)
                    return self.send_json({'text':result['message']['content']})
                except Exception: return self.send_json({'error':'Il modello locale non risponde. Riprova più tardi.'},503)
            raise ValueError('Azione non disponibile.')
        except (ValueError,KeyError,OSError,TypeError): return self.send_json({'error':'Non riesco a completare questa azione. Controlla il nome e riprova.'},400)
if __name__=='__main__':
    try: ThreadingHTTPServer(('127.0.0.1',8766),Handler).serve_forever()
    finally:
        for key in list(TERMS): close_term(key)
