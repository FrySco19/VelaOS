#!/usr/bin/python3
"""Vela AI — native-messaging host for the Firefox "Vela AI" sidebar.

Protocol (Firefox native messaging, 4-byte native-endian length + UTF-8 JSON):
  request  {"id": n, "cmd": "status"}
           {"id": n, "cmd": "chat", "task": "summarize"|"ask"|"fill", "page": {...}, "question": str,
            "history": [{"role","content"}...]}
           {"id": n, "cmd": "save_note", "title": str, "url": str, "body": str}
           {"id": n, "cmd": "log", "action": str, "summary": str, "confirmed": bool}
  replies  {"id": n, "type": "chunk", "text": "..."}   (streamed)
           {"id": n, "type": "done", "text": full, ...} | {"id": n, "type": "error", "message": "..."}
Config is read ONLY through settings-store's vela_ai_config (AI_CONFIG.md, v1).
"""
import datetime, json, os, re, shutil, struct, subprocess, sys, threading, urllib.error, urllib.request

sys.path.insert(0, "/usr/lib/vela/python")
try:
    import vela_ai_config as vac
except ImportError:  # helper missing: refuse politely instead of crashing
    vac = None

APP = "browser"
MAX_PAGE_CHARS = 12000
_out_lock = threading.Lock()


def send(obj):
    data = json.dumps(obj, ensure_ascii=False).encode("utf-8")
    with _out_lock:
        sys.stdout.buffer.write(struct.pack("=I", len(data)))
        sys.stdout.buffer.write(data)
        sys.stdout.buffer.flush()


def read_msg():
    raw = sys.stdin.buffer.read(4)
    if len(raw) < 4:
        return None
    n = struct.unpack("=I", raw)[0]
    return json.loads(sys.stdin.buffer.read(n).decode("utf-8"))


def lang(cfg):
    l = (cfg or {}).get("language", "auto")
    if l in ("it", "en"):
        return l
    loc = os.environ.get("LC_ALL") or os.environ.get("LC_MESSAGES") or os.environ.get("LANG") or "it"
    return "en" if loc.startswith("en") else "it"


T = {
    "it": {
        "nohelper": "La configurazione dell'assistente non è installata.",
        "unreach": "Il modello non risponde. Controlla che sia attivo in Impostazioni > Intelligenza artificiale.",
        "nomodel": "Il modello «{m}» non è installato. Scegline un altro in Impostazioni > Intelligenza artificiale.",
        "auth": "La chiave API non è valida o manca. Inseriscila in Impostazioni > Intelligenza artificiale.",
        "http": "Il servizio ha risposto con un errore ({c}).",
        "timeout": "Il modello non ha risposto in tempo. Riprova con una pagina più corta.",
        "sys": "Sei l'assistente integrato nel browser di Vela. Rispondi in italiano, in modo chiaro e conciso. "
               "Usa solo il contenuto della pagina fornito; se l'informazione non c'è, dillo.",
        "sum": "Riassumi questa pagina in 5 punti brevi, poi una frase con la conclusione principale.",
        "fill": "Devi proporre i valori per i campi di un modulo web. Rispondi SOLO con un oggetto JSON "
                "{\"<id campo>\": \"<valore>\"}. Includi solo i campi per cui i dati dell'utente danno un valore. "
                "Non inventare dati personali. Non compilare mai password o dati di pagamento.",
        "notes_dir": "Note",
    },
    "en": {
        "nohelper": "The assistant configuration is not installed.",
        "unreach": "The model is not responding. Check it in Settings > Artificial intelligence.",
        "nomodel": "The model “{m}” is not installed. Pick another one in Settings > Artificial intelligence.",
        "auth": "The API key is missing or invalid. Enter it in Settings > Artificial intelligence.",
        "http": "The service returned an error ({c}).",
        "timeout": "The model didn't answer in time. Try a shorter page.",
        "sys": "You are the assistant built into the Vela browser. Answer in English, clearly and concisely. "
               "Use only the page content provided; if the information is not there, say so.",
        "sum": "Summarize this page in 5 short bullet points, then one sentence with the main takeaway.",
        "fill": "Propose values for the fields of a web form. Reply ONLY with a JSON object "
                "{\"<field id>\": \"<value>\"}. Include only fields the user's data gives a value for. "
                "Never invent personal data. Never fill passwords or payment data.",
        "notes_dir": "Notes",
    },
}

SECRET_RE = re.compile(r"(sk-[A-Za-z0-9_\-]{16,}|gh[pousr]_[A-Za-z0-9]{20,}|AKIA[0-9A-Z]{16}|"
                       r"eyJ[A-Za-z0-9_\-]{10,}\.[A-Za-z0-9_\-]{10,}\.[A-Za-z0-9_\-]{10,})")


def redact(text, cfg):
    if cfg.get("privacy", {}).get("redact_secrets", True):
        return SECRET_RE.sub("[rimosso]", text)
    return text


def status():
    if vac is None:
        return {"ok": False, "message": T["it"]["nohelper"]}
    cfg = vac.load()
    ep = vac.endpoint(cfg, with_key=False)
    decision, why = vac.may(cfg, "web")
    usable = cfg.get("enabled", True) and not (ep["cloud"] and not cfg.get("privacy", {}).get("allow_cloud", False))
    return {"ok": bool(usable), "message": "" if usable else why, "provider": ep["provider"],
            "model": ep.get("model", ""), "cloud": ep["cloud"], "language": lang(cfg),
            "web_permission": decision}


def page_block(page, cfg):
    page = page or {}
    text = (page.get("text") or "")[:MAX_PAGE_CHARS]
    sel = (page.get("selection") or "")[:4000]
    parts = [f"Titolo: {page.get('title','')}", f"Indirizzo: {page.get('url','')}"]
    if sel:
        parts.append("Testo selezionato:\n" + sel)
    parts.append("Contenuto della pagina:\n" + text)
    return redact("\n".join(parts), cfg)


def build_messages(req, cfg):
    t = T[lang(cfg)]
    task = req.get("task", "ask")
    msgs = [{"role": "system", "content": t["sys"]}]
    if task == "fill":
        fields = json.dumps(req.get("fields", []), ensure_ascii=False)[:6000]
        msgs = [{"role": "system", "content": t["fill"]},
                {"role": "user", "content": f"Campi del modulo:\n{fields}\n\nDati dell'utente:\n{req.get('question','')}"}]
        return msgs
    msgs.append({"role": "user", "content": page_block(req.get("page"), cfg)})
    msgs.append({"role": "assistant", "content": "Ho letto la pagina."})
    for h in (req.get("history") or [])[-8:]:
        if h.get("role") in ("user", "assistant") and isinstance(h.get("content"), str):
            msgs.append({"role": h["role"], "content": h["content"][:4000]})
    msgs.append({"role": "user", "content": t["sum"] if task == "summarize" else redact(req.get("question", ""), cfg)})
    return msgs


def chat(req):
    rid = req.get("id")
    if vac is None:
        return send({"id": rid, "type": "error", "message": T["it"]["nohelper"]})
    cfg = vac.load()
    t = T[lang(cfg)]
    decision, why = vac.may(cfg, "web")
    ep = vac.endpoint(cfg)
    # Reading the page is explicitly requested by the user (button press) — that is the "ask" confirmation.
    # "never" (assistant disabled / cloud not allowed / permission denied) is always honoured.
    if decision == "never":
        return send({"id": rid, "type": "error", "message": why})
    gen = cfg.get("generation", {})
    body = {"model": ep["model"], "messages": build_messages(req, cfg), "stream": True,
            "temperature": gen.get("temperature", 0.3), "max_tokens": gen.get("max_tokens", 1024)}
    if req.get("task") == "fill":
        body["temperature"] = 0
    headers = {"Content-Type": "application/json"}
    if ep.get("api_key"):
        headers["Authorization"] = "Bearer " + ep["api_key"]
    org = cfg.get("openai", {}).get("organization")
    if ep["provider"] == "openai" and org:
        headers["OpenAI-Organization"] = org
    url = ep["base_url"].rstrip("/") + "/chat/completions"
    full = []
    result, final = "ok", None
    try:
        r = urllib.request.Request(url, json.dumps(body).encode(), headers)
        with urllib.request.urlopen(r, timeout=gen.get("timeout_s", 60)) as resp:
            for line in resp:
                line = line.decode("utf-8", "replace").strip()
                if not line.startswith("data:"):
                    continue
                data = line[5:].strip()
                if data == "[DONE]":
                    break
                try:
                    delta = json.loads(data)["choices"][0].get("delta", {}).get("content") or ""
                except (ValueError, KeyError, IndexError, TypeError):
                    continue
                if delta:
                    full.append(delta)
                    send({"id": rid, "type": "chunk", "text": delta})
    except urllib.error.HTTPError as e:
        result = f"http {e.code}"
        detail = ""
        try:
            detail = e.read().decode("utf-8", "replace")
        except Exception:
            pass
        if e.code in (401, 403):
            msg = t["auth"]
        elif e.code == 404 and "model" in detail.lower():
            msg = t["nomodel"].format(m=ep["model"])
        else:
            msg = t["http"].format(c=e.code)
        final = {"id": rid, "type": "error", "message": msg}
    except (TimeoutError, OSError) as e:
        result = "error"
        msg = t["timeout"] if "timed out" in str(e) else t["unreach"]
        final = {"id": rid, "type": "error", "message": msg}
    else:
        final = {"id": rid, "type": "done", "text": "".join(full), "provider": ep["provider"],
                 "model": ep["model"], "cloud": ep["cloud"]}
    # Log BEFORE the final reply: the sidebar closes the port on "done", which ends this process.
    page = req.get("page") or {}
    try:
        vac.log_action(app=APP, action={"summarize": "summarize_page", "fill": "propose_form_values"}.get(
            req.get("task"), "ask_page"), summary=f"{page.get('url', '')} — {req.get('question', '')[:200]}",
            sent_off_machine=bool(ep["cloud"]), confirmed=True, result=result, provider=ep["provider"])
    except Exception:
        pass
    send(final)


def xdg_documents():
    try:
        d = subprocess.run(["xdg-user-dir", "DOCUMENTS"], capture_output=True, text=True, timeout=3).stdout.strip()
        if d and d != os.path.expanduser("~"):
            return d
    except (OSError, subprocess.SubprocessError):
        pass
    for name in ("Documenti", "Documents"):
        p = os.path.expanduser("~/" + name)
        if os.path.isdir(p):
            return p
    return os.path.expanduser("~/Documenti")


def save_note(req):
    cfg = vac.load() if vac else {}
    t = T[lang(cfg)]
    folder = os.path.join(xdg_documents(), t["notes_dir"])
    os.makedirs(folder, exist_ok=True)
    title = (req.get("title") or "Nota").strip()
    safe = re.sub(r"[^\w\- ]+", "", title, flags=re.UNICODE).strip()[:60] or "Nota"
    stamp = datetime.datetime.now().strftime("%Y-%m-%d %H.%M")
    path = os.path.join(folder, f"{stamp} {safe}.md")
    n = 2
    while os.path.exists(path):
        path = os.path.join(folder, f"{stamp} {safe} ({n}).md")
        n += 1
    with open(path, "w", encoding="utf-8") as f:
        f.write(f"# {title}\n\n")
        if req.get("url"):
            f.write(f"Fonte: <{req['url']}>  \n")
        f.write(f"Salvata il {datetime.datetime.now().strftime('%d/%m/%Y alle %H:%M')}\n\n")
        f.write((req.get("body") or "").strip() + "\n")
    if vac:
        try:
            vac.log_action(app=APP, action="save_note", summary=path, sent_off_machine=False, confirmed=True)
        except Exception:
            pass
    send({"id": req.get("id"), "type": "done", "path": path, "folder": folder})


def handle(req):
    cmd = req.get("cmd")
    rid = req.get("id")
    try:
        if cmd == "status":
            send({"id": rid, "type": "done", **status()})
        elif cmd == "chat":
            chat(req)
        elif cmd == "save_note":
            save_note(req)
        elif cmd == "open_folder":
            subprocess.Popen(["xdg-open", req.get("path") or xdg_documents()],
                             stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, start_new_session=True)
            send({"id": rid, "type": "done"})
        elif cmd == "open_settings":
            for argv in (["vela-settings", "ai"], ["systemsettings", "kcm_vela_ai"], ["systemsettings"]):
                if shutil.which(argv[0]):
                    subprocess.Popen(argv, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, start_new_session=True)
                    break
            send({"id": rid, "type": "done"})
        elif cmd == "log":
            if vac:
                vac.log_action(app=APP, action=str(req.get("action", ""))[:60], summary=req.get("summary", ""),
                               sent_off_machine=False, confirmed=req.get("confirmed"))
            send({"id": rid, "type": "done"})
        else:
            send({"id": rid, "type": "error", "message": "unknown command"})
    except Exception as e:  # never leave the sidebar hanging; details go to stderr (browser console), not the UI
        print(f"vela-ai host: {cmd}: {e!r}", file=sys.stderr)
        msg = "Unable to complete the request. Try again." if lang(vac.load() if vac else {}) == "en" else \
              "Impossibile completare la richiesta. Riprova."
        send({"id": rid, "type": "error", "message": msg})


def main():
    # One thread per request so a long answer does not block "status"; stdin EOF (port closed, e.g. the user
    # pressed "Ferma" or closed the sidebar) terminates the host immediately, aborting any request in flight.
    while True:
        msg = read_msg()
        if msg is None:
            os._exit(0)
        threading.Thread(target=handle, args=(msg,), daemon=True).start()


if __name__ == "__main__":
    main()
