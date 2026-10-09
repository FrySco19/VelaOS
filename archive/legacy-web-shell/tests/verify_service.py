import importlib.util, pathlib, tempfile, threading, http.client, json, shutil, sys
sys.path.insert(0,str(pathlib.Path(__file__).resolve().parents[1]))
spec=importlib.util.spec_from_file_location('vela',str(pathlib.Path(__file__).resolve().parents[1]/'service.py')); m=importlib.util.module_from_spec(spec); spec.loader.exec_module(m)
with tempfile.TemporaryDirectory() as tmp:
 m.HOME=pathlib.Path(tmp)
 for d in m.FOLDERS:(m.HOME/d).mkdir()
 server=m.ThreadingHTTPServer(('127.0.0.1',0),m.Handler); threading.Thread(target=server.serve_forever,daemon=True).start()
 def call(path,data=None,cookie=None,host='127.0.0.1:8766',origin=m.ORIGIN):
  c=http.client.HTTPConnection('127.0.0.1',server.server_port); headers={'Host':host,'Origin':origin}
  if cookie: headers['Cookie']=cookie
  c.request('POST' if data is not None else 'GET',path,json.dumps(data).encode() if data is not None else None,headers); r=c.getresponse(); b=r.read(); return r.status,b,r.getheader('Set-Cookie')
 assert call('/api/files')[0]==401
 assert call('/api/session',host='evil.example')[0]==403
 code,_,cookie=call('/api/session'); assert code==200; cookie=cookie.split(';')[0]
 assert call('/api/ready',{},cookie,origin='https://evil.example')[0]==403
 assert call('/api/files?path=Documents/../Downloads',cookie=cookie)[0]==400
 def action(a,**kw):
  code,b,_=call('/api/file-action',dict(action=a,**kw),cookie); assert code==200,(code,b); return json.loads(b)
 action('new',path='Documents',name='test.txt'); (m.HOME/'Documents/test.txt').write_text('file reale')
 action('rename',path='Documents/test.txt',name='renamed.txt'); action('copy',path='Documents/renamed.txt',name='copy.txt')
 assert (m.HOME/'Documents/copy.txt').read_text()=='file reale'
 assert call('/api/preview?path=Documents/renamed.txt',cookie=cookie)[0]==200
 r=action('trash',path='Documents/renamed.txt'); assert not (m.HOME/'Documents/renamed.txt').exists()
 action('restore',path='Documents',key=r['undo']); assert (m.HOME/'Documents/renamed.txt').read_text()=='file reale'
 assert call('/api/file-action',{'action':'new','path':'Documents','name':'../oops'},cookie)[0]==400
 server.shutdown();server.server_close()
print('PASS: session, Host/Origin, traversal, create, rename, duplicate, preview, trash and restore')

