"""Conversation controller: context → model (streaming) → tool calls → preview cards → confirmation → undo.

Qt-free. The UI passes `emit(kind, data)`; it is called from worker threads, so the UI must marshal to its own
thread (the Qt bridge does this with a queued signal). Events:
  busy {on}                     assistant_start {id}       assistant_delta {id, text}   assistant_end {id}
  card {card}                   card_update {card}         setup {state, message, ...}  error {message, detail}
  status {model, provider, cloud, host, note}
"""
import datetime
import json
import os
import threading
import time
import urllib.parse
import uuid

from . import config, i18n, intents, llm, tools

T = i18n.T
MAX_ROUNDS = 4
MAX_HISTORY = 24


def _host(url):
    return urllib.parse.urlsplit(url).hostname or url


def system_prompt(ctx, lang):
    now = datetime.datetime.now()
    if lang == "en":
        lines = ["You are the Vela system assistant, built into the Vela operating system (Linux, KDE Plasma).",
                 "Answer in English, briefly and clearly, using Markdown when useful.",
                 "You can act on the computer through the provided tools. Every change is shown to the user, "
                 "who must confirm it: never claim an action happened until the tool result says so.",
                 "If you don't know something, say so. Never invent files, settings or results."]
    else:
        lines = ["Sei l'assistente di sistema di Vela, integrato nel sistema operativo Vela (Linux, KDE Plasma).",
                 "Rispondi in italiano, in modo breve e chiaro, usando Markdown quando serve.",
                 "Puoi agire sul computer con gli strumenti forniti. Ogni modifica viene mostrata all'utente, che "
                 "deve confermarla: non dire mai che un'azione è avvenuta finché il risultato dello strumento non lo "
                 "conferma.",
                 "Se non sai qualcosa, dillo. Non inventare file, impostazioni o risultati.",
                 "Per aprire le impostazioni usa open_settings; per cercare file usa find_files; usa run_command "
                 "solo se nessun altro strumento va bene."]
    lines.append(("Now: " if lang == "en" else "Data e ora: ") + now.strftime("%Y-%m-%d %H:%M (%A)"))
    lines.append(("Home folder: " if lang == "en" else "Cartella Home: ") + os.path.expanduser("~"))
    if ctx.get("window_title") or ctx.get("window_app"):
        lines.append(("Active window: " if lang == "en" else "Finestra attiva: ")
                     + f"«{ctx.get('window_title', '')}» ({ctx.get('window_app', '')})")
    if ctx.get("space"):
        lines.append(("Current Space: " if lang == "en" else "Spazio corrente: ") + ctx["space"])
    return "\n".join(lines)


class Agent:
    def __init__(self, emit, load_cfg=None):
        self.emit = emit
        self.load_cfg = load_cfg or config.load
        self.history = []            # OpenAI messages without the system prompt
        self.cards = {}              # id -> card
        self.pending = {}            # call_id -> card id (model-requested, waiting for user)
        self.cancel = threading.Event()
        self.thread = None
        self.tools_supported = True
        self.context = {}
        self.conv_id = datetime.datetime.now().strftime("%Y%m%d-%H%M%S-") + uuid.uuid4().hex[:6]
        self.lock = threading.RLock()
        self.ready = None

    # ------------------------------------------------------------------ public API (UI thread)
    def new_conversation(self):
        self.stop()
        with self.lock:
            self.history, self.cards, self.pending = [], {}, {}
            self.conv_id = datetime.datetime.now().strftime("%Y%m%d-%H%M%S-") + uuid.uuid4().hex[:6]

    def set_context(self, ctx):
        self.context = dict(ctx or {})

    def busy(self):
        return self.thread is not None and self.thread.is_alive()

    def stop(self):
        self.cancel.set()
        if self.thread is not None:
            self.thread.join(timeout=3)
        self.cancel = threading.Event()

    def check(self):
        cfg = self.load_cfg()
        ep = config.endpoint(cfg)
        st = llm.check_ready(cfg, ep)
        self.ready = st
        self.emit("status", {"model": st.get("model", ""), "provider": ep["provider"], "cloud": ep["cloud"],
                             "host": _host(ep["base_url"]), "note": st.get("message", "") if st["state"] == "ready"
                             else "", "state": st["state"]})
        return st

    def ask(self, text, attachments=None):
        """attachments: {selection, clipboard, files:[described], image:data-uri}"""
        if self.busy():
            return False
        self._decline_pending_silently()
        self.thread = threading.Thread(target=self._ask_worker, args=(text, attachments or {}), daemon=True)
        self.thread.start()
        return True

    def run_intent(self, tool_name, args, user_text=None):
        """A command recognised without the model (panel fast path or KRunner)."""
        self._decline_pending_silently()
        self._make_card(tool_name, args, call_id=None)

    def confirm(self, card_id, danger_ok=False):
        card = self.cards.get(card_id)
        if not card or card["state"] != "pending":
            return
        if card["preview"].get("danger") and card["tool"] == "run_command":
            if not danger_ok:
                return      # the UI must collect the explicit second confirmation first
            card["args"] = dict(card["args"], _confirmed_danger=True)
        card["state"] = "running"
        self.emit("card_update", {"card": dict(card)})
        threading.Thread(target=self._execute, args=(card, True), daemon=True).start()

    def decline(self, card_id):
        card = self.cards.get(card_id)
        if not card or card["state"] != "pending":
            return
        card["state"] = "declined"
        card["message"] = T("declined")
        config.log(card["tool"], card["preview"].get("title", ""), confirmed=False, result="declined")
        self._resolve_call(card, {"ok": False, "error": "The user declined this action."})
        self.emit("card_update", {"card": dict(card)})

    def undo(self, card_id):
        card = self.cards.get(card_id)
        if not card or not card.get("undo_data") or card["state"] != "done":
            return
        tool = tools.get(card["tool"])
        try:
            r = tool.undo(card["undo_data"])
            card["state"], card["message"] = "undone", r["message"]
            config.log("undo", card["preview"].get("title", ""), confirmed=True, result="ok")
        except tools.ToolError as e:
            card["message"] = str(e)
            config.log("undo", card["preview"].get("title", ""), confirmed=True, result="error")
        except OSError as e:
            card["message"] = T("undo_conflict")
            config.log("undo", f"{card['preview'].get('title', '')}: {e}", confirmed=True, result="error")
        card["undo_data"] = None if card["state"] == "undone" else card["undo_data"]
        card["can_undo"] = bool(card["undo_data"])
        self.emit("card_update", {"card": dict(card)})
        if card.get("call_id"):
            self.history.append({"role": "user", "content": f"[{card['tool']} annullato dall'utente]"})

    # ------------------------------------------------------------------ cards
    def _make_card(self, tool_name, args, call_id):
        tool = tools.get(tool_name)
        cid = uuid.uuid4().hex[:10]
        card = {"id": cid, "tool": tool_name, "args": args, "call_id": call_id, "kind": tool.kind if tool else "",
                "icon": tool.icon if tool else "dialog-error", "state": "pending", "message": "", "data": {},
                "preview": {"title": tool_name, "detail": []}, "undo_data": None, "can_undo": False}
        self.cards[cid] = card
        if tool is None:
            card.update(state="failed", message=T("err_generic"))
            self.emit("card", {"card": dict(card)})
            self._resolve_call(card, {"ok": False, "error": f"unknown tool {tool_name}"})
            return card
        cfg = self.load_cfg()
        if tool.permission:
            decision, why = config.may(cfg, tool.permission)
            # 'never' for local reasons only: the cloud gate does not apply to purely local actions
            if decision == "never" and cfg.get("permissions", {}).get(tool.permission) in (False, "never"):
                card.update(state="failed", message=T("perm_denied"))
                self.emit("card", {"card": dict(card)})
                self._resolve_call(card, {"ok": False, "error": "Not allowed by the user's settings."})
                return card
        try:
            card["preview"] = tool.preview(args)
        except tools.ToolError as e:
            card.update(state="failed", message=str(e))
            self.emit("card", {"card": dict(card)})
            self._resolve_call(card, {"ok": False, "error": str(e)})
            return card
        except Exception as e:  # defensive: never crash the UI on a tool bug
            card.update(state="failed", message=T("err_generic"))
            self.emit("card", {"card": dict(card)})
            self._resolve_call(card, {"ok": False, "error": repr(e)})
            return card
        if tool.kind == "change":
            if call_id:
                self.pending[call_id] = cid
            self.emit("card", {"card": dict(card)})
        else:
            card["state"] = "running"
            self.emit("card", {"card": dict(card)})
            self._execute(card, None)
        return card

    def _execute(self, card, confirmed):
        tool = tools.get(card["tool"])
        try:
            r = tool.run(card["args"])
            card["state"] = "done" if r["ok"] else "failed"
            card["message"], card["data"] = r["message"], r["data"]
            card["undo_data"] = r.get("undo")
            card["can_undo"] = bool(r.get("undo"))
            payload = {"ok": r["ok"], "message": r["message"], "data": r["data"]}
            res = "ok" if r["ok"] else "error"
        except tools.ToolError as e:
            card["state"], card["message"] = "failed", str(e)
            payload, res = {"ok": False, "error": str(e)}, "error"
        except Exception as e:
            card["state"], card["message"] = "failed", T("err_generic")
            payload, res = {"ok": False, "error": repr(e)}, "error"
        if tool.kind == "change":
            config.log(card["tool"], card["preview"].get("title", "") + (
                " · " + card["preview"]["code"] if card["preview"].get("code") else ""), confirmed=bool(confirmed),
                result=res)
        self.emit("card_update", {"card": dict(card)})
        self._resolve_call(card, payload)

    def _resolve_call(self, card, payload):
        cid = card.get("call_id")
        if not cid:
            return
        with self.lock:
            self.history.append({"role": "tool", "tool_call_id": cid,
                                 "content": json.dumps(payload, ensure_ascii=False)[:6000]})
            self.pending.pop(cid, None)

    def _decline_pending_silently(self):
        for call_id, cid in list(self.pending.items()):
            card = self.cards.get(cid)
            if card and card["state"] == "pending":
                card["state"], card["message"] = "declined", T("declined")
                self.emit("card_update", {"card": dict(card)})
                self._resolve_call(card, {"ok": False, "error": "Not confirmed by the user."})

    # ------------------------------------------------------------------ worker
    def _user_content(self, text, att, lang):
        parts = [text]
        lab = (lambda it, en: en if lang == "en" else it)
        if att.get("selection"):
            parts.append(lab("Testo selezionato dall'utente:", "Text selected by the user:")
                         + "\n«" + att["selection"] + "»")
        if att.get("clipboard"):
            parts.append(lab("Contenuto degli appunti:", "Clipboard content:") + "\n«" + att["clipboard"] + "»")
        for f in att.get("files") or []:
            head = lab("File allegato", "Attached file") + f": {f['path']} ({f.get('kind', '')}, {f.get('size', 0)} B)"
            parts.append(head + ("\n```\n" + f["text"] + "\n```" if f.get("text") else ""))
        content = "\n\n".join(p for p in parts if p)
        if att.get("image"):
            return [{"type": "text", "text": content}, {"type": "image_url", "image_url": {"url": att["image"]}}]
        return content

    def _ask_worker(self, text, att):
        self.emit("busy", {"on": True})
        try:
            self._ask(text, att)
        finally:
            self.emit("busy", {"on": False})
            self._save_history()

    def _ask(self, text, att):
        cfg = self.load_cfg()
        lang = config.language(cfg)
        i18n.set_lang(lang)
        ep = config.endpoint(cfg)
        st = llm.check_ready(cfg, ep)
        self.ready = st
        if st["state"] != "ready":
            self.emit("setup", st)
            return
        ep = dict(ep, model=st["model"])
        if att.get("image") and ep["provider"] == "ollama":
            caps = llm.ollama_capabilities(ep["native_url"], ep["model"])
            if caps and "vision" not in caps:
                self.emit("error", {"message": T("vision_unsupported"), "detail": ", ".join(caps)})
                return
        self.emit("status", {"model": ep["model"], "provider": ep["provider"], "cloud": ep["cloud"],
                             "host": _host(ep["base_url"]), "note": st.get("message", ""), "state": "ready"})
        with self.lock:
            self.history.append({"role": "user", "content": self._user_content(text, att, lang)})
        config.log("ask", text[:160] + (" [+screenshot]" if att.get("image") else "")
                   + (f" [+{len(att.get('files') or [])} file]" if att.get("files") else ""),
                   sent_off_machine=ep["cloud"])
        gen = cfg.get("generation", {})
        for _round in range(MAX_ROUNDS):
            if self.cancel.is_set():
                return
            msgs = [{"role": "system", "content": system_prompt(self.context, lang)}] + self._trimmed_history()
            if ep["cloud"] and cfg.get("privacy", {}).get("redact_secrets", True):
                msgs = llm.redact_messages(msgs)
            item = uuid.uuid4().hex[:10]
            text_acc, calls = "", []
            self.emit("assistant_start", {"id": item})
            try:
                try:
                    events = list_or_stream(ep, msgs, tools.schemas() if self.tools_supported else None, gen,
                                            self.cancel)
                    for kind, data in events:
                        if kind == "delta":
                            text_acc += data
                            self.emit("assistant_delta", {"id": item, "text": data})
                        elif kind == "tool_calls":
                            calls = data
                except llm.ToolsUnsupported:
                    self.tools_supported = False
                    for kind, data in list_or_stream(ep, msgs, None, gen, self.cancel):
                        if kind == "delta":
                            text_acc += data
                            self.emit("assistant_delta", {"id": item, "text": data})
            except llm.Cancelled:
                self.emit("assistant_end", {"id": item, "text": text_acc, "cancelled": True})
                if text_acc:
                    self.history.append({"role": "assistant", "content": text_acc})
                return
            except llm.LLMError as e:
                self.emit("assistant_end", {"id": item, "text": text_acc, "empty": not text_acc})
                self.emit("error", {"message": e.message, "detail": e.detail})
                config.log("ask", f"{e.kind}: {e.detail[:200]}", result="error", sent_off_machine=ep["cloud"])
                return
            self.emit("assistant_end", {"id": item, "text": text_acc, "empty": not text_acc})
            msg = {"role": "assistant", "content": text_acc or None}
            if calls:
                msg["tool_calls"] = [{"id": c["id"], "type": "function",
                                      "function": {"name": c["name"], "arguments": c["arguments"] or "{}"}}
                                     for c in calls]
            with self.lock:
                self.history.append(msg)
            if not calls:
                return
            for c in calls:
                self._make_card(c["name"], tools.parse_args(c["arguments"]), call_id=c["id"])
            if self.pending:
                return   # wait for the user; tool results are appended when they decide
            # only read/open tools ran: let the model use the results

    def _trimmed_history(self):
        with self.lock:
            h = list(self.history)
        # drop pending tool calls without results (would make the API reject the request)
        answered = {m.get("tool_call_id") for m in h if m.get("role") == "tool"}
        out = []
        for m in h:
            if m.get("role") == "assistant" and m.get("tool_calls"):
                tcs = [tc for tc in m["tool_calls"] if tc["id"] in answered]
                if not tcs:
                    if m.get("content"):
                        out.append({"role": "assistant", "content": m["content"]})
                    continue
                m = dict(m, tool_calls=tcs)
            out.append(m)
        if len(out) > MAX_HISTORY:
            out = out[-MAX_HISTORY:]
            while out and out[0].get("role") != "user":
                out.pop(0)
        return out

    def _save_history(self):
        cfg = self.load_cfg()
        if not config.history_enabled(cfg) or not self.history:
            return
        d = config.memory_dir()
        try:
            os.makedirs(d, mode=0o700, exist_ok=True)
            hist = []
            for m in self.history:
                m = dict(m)
                if isinstance(m.get("content"), list):   # drop screenshots, keep the text
                    m["content"] = " ".join(p.get("text", "") for p in m["content"] if p.get("type") == "text")
                hist.append(m)
            tmp = os.path.join(d, f".{self.conv_id}.tmp")
            with open(tmp, "w", encoding="utf-8") as f:
                json.dump({"id": self.conv_id, "updated": time.time(), "messages": hist}, f, ensure_ascii=False)
            os.replace(tmp, os.path.join(d, f"{self.conv_id}.json"))
        except OSError:
            pass


def list_or_stream(ep, msgs, tool_schemas, gen, cancel):
    return llm.stream_chat(ep, msgs, tools=tool_schemas, temperature=gen.get("temperature", 0.3),
                           max_tokens=gen.get("max_tokens", 1024), timeout=gen.get("timeout_s", 60), cancel=cancel)
