#!/usr/bin/python3
"""Protocol test for vela_ai_browser_host.py against mock_llm.py. Run: python3 test_host.py [host_path]"""
import json, os, struct, subprocess, sys, tempfile, time

HERE = os.path.dirname(os.path.abspath(__file__))
HOST = sys.argv[1] if len(sys.argv) > 1 else os.path.join(HERE, "../overlay/usr/lib/vela/apps/vela_ai_browser_host.py")
home = tempfile.mkdtemp(prefix="vela-host-test-")
env = dict(os.environ, HOME=home, XDG_CONFIG_HOME=home + "/.config", XDG_DATA_HOME=home + "/.local/share",
           VELA_AI_PROVIDER="openai", VELA_AI_BASE_URL="http://127.0.0.1:18080/v1", VELA_AI_MODEL="mock", LANG="it_IT.UTF-8")
mock = subprocess.Popen([sys.executable, os.path.join(HERE, "mock_llm.py"), "18080"])
time.sleep(0.5)
fails = 0

def run(msgs, env=env):
    p = subprocess.Popen([sys.executable, HOST], stdin=subprocess.PIPE, stdout=subprocess.PIPE, env=env)
    for m in msgs:
        d = json.dumps(m).encode(); p.stdin.write(struct.pack("=I", len(d)) + d)
    p.stdin.flush()
    out, want = [], {m["id"] for m in msgs}
    while want:
        raw = p.stdout.read(4)
        if len(raw) < 4: break
        r = json.loads(p.stdout.read(struct.unpack("=I", raw)[0]))
        out.append(r)
        if r["type"] in ("done", "error"): want.discard(r["id"])
    p.stdin.close(); p.wait(5)
    return out

def check(name, cond, info=""):
    global fails
    print(("PASS " if cond else "FAIL ") + name + ("" if cond else f"  {info}"))
    fails += 0 if cond else 1

try:
    r = run([{"id": 1, "cmd": "status"}])
    check("status ok/local", r[-1].get("ok") is True and r[-1].get("cloud") is False and r[-1]["model"] == "mock", r)
    page = {"title": "Test", "url": "https://example.com", "text": "Contenuto sk-ABCDEFGHIJKLMNOPQRSTUV di prova " * 20}
    r = run([{"id": 2, "cmd": "chat", "task": "summarize", "page": page}])
    chunks = [x for x in r if x["type"] == "chunk"]; done = r[-1]
    check("summarize streams chunks", len(chunks) > 3, r)
    check("summarize done text == joined chunks", done["type"] == "done" and done["text"] == "".join(c["text"] for c in chunks), done)
    r = run([{"id": 3, "cmd": "chat", "task": "ask", "question": "Di cosa parla?", "page": page,
              "history": [{"role": "user", "content": "a"}, {"role": "assistant", "content": "b"}]}])
    check("ask includes history (6 messages)", "6 messaggi" in r[-1].get("text", ""), r[-1])
    fields = [{"id": "f1_x", "label": "Nome e cognome", "type": "text"}, {"id": "f2_y", "label": "Email", "type": "email"}]
    r = run([{"id": 4, "cmd": "chat", "task": "fill", "question": "Mario Rossi mario@example.com", "fields": fields}])
    check("fill returns JSON mapping", json.loads(r[-1]["text"]) == {"f1_x": "Mario Rossi", "f2_y": "mario@example.com"}, r[-1])
    r = run([{"id": 5, "cmd": "save_note", "title": "Pagina: prova/1", "url": "https://example.com", "body": "- uno"}])
    path = r[-1].get("path", "")
    check("save_note writes markdown", os.path.isfile(path) and "Fonte: <https://example.com>" in open(path).read(), r)
    log = os.path.join(home, ".local/share/vela/ai-actions.jsonl")
    lines = [json.loads(l) for l in open(log)] if os.path.exists(log) else []
    check("action log written (app=browser)", len(lines) >= 4 and all(l["app"] == "browser" for l in lines), lines)
    check("log marks local (sent_off_machine=false)", all(l["sent_off_machine"] is False for l in lines))
    # cloud refusal: real remote URL with allow_cloud=false must be refused without any network call
    e2 = dict(env, VELA_AI_BASE_URL="https://api.example.invalid/v1")
    r = run([{"id": 6, "cmd": "chat", "task": "summarize", "page": page}], e2)
    check("cloud refused without allow_cloud", r[-1]["type"] == "error" and "Privacy" in r[-1]["message"], r[-1])
    # disabled
    os.makedirs(home + "/.config/vela", exist_ok=True)
    json.dump({"version": 1, "enabled": False}, open(home + "/.config/vela/ai.json", "w"))
    r = run([{"id": 7, "cmd": "status"}, {"id": 8, "cmd": "chat", "task": "ask", "question": "x", "page": page}])
    check("disabled: status not ok", any(x.get("id") == 7 and x.get("ok") is False for x in r), r)
    check("disabled: chat refused", any(x.get("id") == 8 and x["type"] == "error" and "disattivato" in x["message"] for x in r), r)
    os.unlink(home + "/.config/vela/ai.json")
    r = run([{"id": 9, "cmd": "chat", "task": "ask", "question": "x", "page": page}], dict(env, VELA_AI_MODEL="missing"))
    check("missing model -> human message", r[-1]["type"] == "error" and "non è installato" in r[-1]["message"], r[-1])
    r = run([{"id": 10, "cmd": "chat", "task": "ask", "question": "x", "page": page}], dict(env, VELA_AI_BASE_URL="http://127.0.0.1:1/v1"))
    check("unreachable -> human message", r[-1]["type"] == "error" and "non risponde" in r[-1]["message"], r[-1])
finally:
    mock.kill()
print("RESULT:", "OK" if not fails else f"{fails} FAILED")
sys.exit(1 if fails else 0)
