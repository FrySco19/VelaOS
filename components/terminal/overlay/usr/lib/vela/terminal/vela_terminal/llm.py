"""OpenAI-compatible chat client + prompts for the AI terminal (no third-party deps).

Never invents output: every AI text shown in the UI comes from a real model response.
"""
from __future__ import annotations

import json
import os
import re
import urllib.error
import urllib.request

from . import aiconfig


class LLMError(Exception):
    def __init__(self, code: str, detail: str = ""):
        super().__init__(f"{code}: {detail}")
        self.code = code      # unreachable | http | model_missing | bad_response | not_allowed | auth
        self.detail = detail


SECRET_PATTERNS = [
    (re.compile(r"-----BEGIN [A-Z ]*PRIVATE KEY-----.*?-----END [A-Z ]*PRIVATE KEY-----", re.S), "[CHIAVE PRIVATA RIMOSSA]"),
    (re.compile(r"\b(sk|rk|pk)-[A-Za-z0-9_\-]{16,}"), "[SEGRETO]"),
    (re.compile(r"\bgh[pousr]_[A-Za-z0-9]{20,}"), "[SEGRETO]"),
    (re.compile(r"\bAKIA[0-9A-Z]{16}\b"), "[SEGRETO]"),
    (re.compile(r"\bxox[abprs]-[A-Za-z0-9-]{10,}"), "[SEGRETO]"),
    (re.compile(r"(?i)(bearer\s+)[A-Za-z0-9._\-]{16,}"), r"\1[SEGRETO]"),
    (re.compile(r"(?i)\b(password|passwd|pwd|secret|token|api[_-]?key)\s*[=:]\s*\S+"), r"\1=[SEGRETO]"),
]


def redact(text: str) -> str:
    for pat, rep in SECRET_PATTERNS:
        text = pat.sub(rep, text)
    return text


class LLM:
    def __init__(self, cfg: dict | None = None):
        self.cfg = cfg or aiconfig.load()
        self.ep = aiconfig.endpoint(self.cfg)
        gen = self.cfg.get("generation", {})
        self.temperature = float(gen.get("temperature", 0.3))
        self.max_tokens = int(gen.get("max_tokens", 1024))
        self.timeout = float(gen.get("timeout_s", 60))

    @property
    def sends_off_machine(self) -> bool:
        return bool(self.ep.get("cloud"))

    def describe(self) -> str:
        m = self.ep.get("model") or "?"
        return f"{m} · {'cloud' if self.sends_off_machine else 'locale'}"

    def _resolve_model(self) -> str:
        model = self.ep.get("model", "")
        if model or self.ep.get("provider") != "ollama":
            return model
        # empty => first installed Ollama model (contract)
        try:
            with urllib.request.urlopen(self.ep["native_url"] + "/api/tags", timeout=5) as r:
                tags = json.load(r).get("models", [])
            if tags:
                return tags[0]["name"]
        except Exception:
            pass
        raise LLMError("model_missing", "nessun modello installato")

    def chat(self, messages: list, json_mode: bool = True) -> str:
        ok, why, _ep = aiconfig.ai_status(self.cfg)
        if not ok:
            raise LLMError("not_allowed", why)
        if self.sends_off_machine and self.cfg.get("privacy", {}).get("redact_secrets", True):
            messages = [dict(m, content=redact(m["content"])) for m in messages]
        body = {"model": self._resolve_model(), "messages": messages, "temperature": self.temperature,
                "max_tokens": self.max_tokens, "stream": False}
        if json_mode:
            body["response_format"] = {"type": "json_object"}
        req = urllib.request.Request(self.ep["base_url"].rstrip("/") + "/chat/completions",
                                     data=json.dumps(body).encode(), method="POST",
                                     headers={"Content-Type": "application/json"})
        if self.ep.get("api_key"):
            req.add_header("Authorization", "Bearer " + self.ep["api_key"])
        org = self.cfg.get("openai", {}).get("organization") if self.ep.get("provider") == "openai" else ""
        if org:
            req.add_header("OpenAI-Organization", org)
        try:
            with urllib.request.urlopen(req, timeout=self.timeout) as r:
                data = json.load(r)
        except urllib.error.HTTPError as e:
            detail = ""
            try:
                detail = e.read().decode("utf-8", "replace")[:300]
            except Exception:
                pass
            if e.code == 404 and "model" in detail.lower():
                raise LLMError("model_missing", detail)
            if e.code in (401, 403):
                raise LLMError("auth", detail)
            if e.code == 400 and json_mode and "response_format" in detail:
                return self.chat(messages, json_mode=False)
            raise LLMError("http", f"{e.code} {detail}")
        except (urllib.error.URLError, OSError, TimeoutError) as e:
            raise LLMError("unreachable", str(e))
        except ValueError as e:
            raise LLMError("bad_response", str(e))
        try:
            content = data["choices"][0]["message"]["content"] or ""
        except (KeyError, IndexError, TypeError):
            raise LLMError("bad_response", json.dumps(data)[:200])
        aiconfig.log_action("ai_request", f"richiesta al modello ({self.describe()})",
                            sent_off_machine=self.sends_off_machine, confirmed=None, result="ok")
        return content


def parse_json(text: str) -> dict:
    t = text.strip()
    t = re.sub(r"^```(?:json)?\s*|\s*```$", "", t)
    try:
        d = json.loads(t)
        if isinstance(d, dict):
            return d
    except ValueError:
        pass
    i, j = t.find("{"), t.rfind("}")
    if i >= 0 and j > i:
        try:
            d = json.loads(t[i:j + 1])
            if isinstance(d, dict):
                return d
        except ValueError:
            pass
    raise LLMError("bad_response", "risposta non in formato JSON")


def _env_block(cwd: str, lang: str) -> str:
    user = os.environ.get("USER") or "vela"
    try:
        listing = sorted(os.listdir(cwd))[:40]
    except OSError:
        listing = []
    language = "italiano" if lang == "it" else "English"
    return (f"Sistema: Vela OS (Debian 13 trixie, KDE Plasma 6, Wayland). Shell: bash. Utente: {user} "
            f"(sudo richiede password). Cartella corrente: {cwd}. Home: {os.path.expanduser('~')}. "
            f"Cartelle utente in italiano: ~/Scaricati, ~/Documenti, ~/Immagini, ~/Musica, ~/Video, ~/Scrivania. "
            f"File nella cartella corrente (max 40): {', '.join(listing)}. "
            f"Software: apt (pacchetti Debian), flatpak (Flathub), Dolphin, Konsole. Lingua delle risposte: {language}.")


RULES = ("Regole: comandi bash reali e corretti per Debian 13; uno per voce; preferisci comandi in sola lettura e "
         "reversibili (es. gio trash invece di rm quando l'utente vuole cancellare file personali); non usare sudo se non "
         "serve davvero; mai comandi interattivi che restano in attesa (usa -y solo se l'utente lo chiede); non inventare "
         "risultati: non sai cosa produrrà il comando. Spiegazioni brevi, chiare, senza gergo inutile.")


def propose_messages(intent: str, cwd: str, lang: str, history: list | None = None) -> list:
    sys_p = (_env_block(cwd, lang) + "\n" + RULES + "\nRispondi SOLO con JSON: "
             '{"explanation": "cosa farai, 1-2 frasi", "commands": [{"command": "...", "explanation": "cosa fa"}], '
             '"note": "avvertenze opzionali"}. Se la richiesta non si fa col terminale o è ambigua, metti commands: [] '
             "e spiega o chiedi chiarimenti in explanation.")
    msgs = [{"role": "system", "content": sys_p}]
    for h in (history or [])[-4:]:
        msgs.append({"role": "user", "content": h["intent"]})
        msgs.append({"role": "assistant", "content": json.dumps({"commands": [{"command": c} for c in h.get("commands", [])]})})
    msgs.append({"role": "user", "content": intent})
    return msgs


def parse_proposal(d: dict) -> dict:
    cmds = []
    for c in d.get("commands") or []:
        if isinstance(c, str):
            c = {"command": c, "explanation": ""}
        if isinstance(c, dict) and str(c.get("command", "")).strip():
            cmds.append({"command": str(c["command"]).strip(), "explanation": str(c.get("explanation", "")).strip()})
    return {"explanation": str(d.get("explanation", "")).strip(), "commands": cmds[:8],
            "note": str(d.get("note", "") or "").strip()}


def agent_messages(goal: str, cwd: str, lang: str, steps: list) -> list:
    sys_p = (_env_block(cwd, lang) + "\n" + RULES + "\nSei in modalità agente: porti a termine un obiettivo in più passi. "
             "A ogni turno ricevi i risultati dei passi già eseguiti e proponi UN solo comando successivo. "
             "Ogni comando viene mostrato all'utente e eseguito solo se lui conferma. Rispondi SOLO con JSON: "
             '{"plan": ["passo 1", "passo 2", ...], "next": {"command": "...", "explanation": "..."} oppure null, '
             '"done": true|false, "message": "breve stato o risultato finale"}. Quando l\'obiettivo è raggiunto o '
             "impossibile, done=true e next=null, e spiega il risultato in message basandoti SOLO sugli output reali.")
    msgs = [{"role": "system", "content": sys_p}, {"role": "user", "content": "Obiettivo: " + goal}]
    for s in steps:
        out = (s.get("output") or "")[-2500:]
        msgs.append({"role": "assistant", "content": json.dumps({"next": {"command": s["command"]}})})
        status = "saltato dall'utente" if s.get("skipped") else f"codice di uscita {s.get('rc')}"
        msgs.append({"role": "user", "content": f"Risultato ({status}):\n{out}"})
    return msgs


def parse_agent(d: dict) -> dict:
    nxt = d.get("next")
    if isinstance(nxt, str):
        nxt = {"command": nxt, "explanation": ""}
    if not isinstance(nxt, dict) or not str(nxt.get("command", "")).strip():
        nxt = None
    else:
        nxt = {"command": str(nxt["command"]).strip(), "explanation": str(nxt.get("explanation", "")).strip()}
    plan = [str(x) for x in (d.get("plan") or []) if str(x).strip()][:12]
    done = bool(d.get("done")) or nxt is None
    return {"plan": plan, "next": nxt, "done": done, "message": str(d.get("message", "") or "").strip()}


def explain_messages(command: str, rc: int, output: str, cwd: str, lang: str) -> list:
    sys_p = (_env_block(cwd, lang) + "\n" + RULES + "\nUn comando è fallito. Spiega in parole semplici cosa è successo "
             "e proponi una correzione se ce n'è una sicura. Rispondi SOLO con JSON: "
             '{"summary": "1 frase", "cause": "causa probabile", "fix": {"command": "...", "explanation": "..."} oppure null}.')
    return [{"role": "system", "content": sys_p},
            {"role": "user", "content": f"Comando: {command}\nCodice di uscita: {rc}\nOutput (ultime righe):\n{output[-3000:]}"}]


def parse_explain(d: dict) -> dict:
    fix = d.get("fix")
    if isinstance(fix, str):
        fix = {"command": fix, "explanation": ""}
    if not isinstance(fix, dict) or not str(fix.get("command", "")).strip():
        fix = None
    else:
        fix = {"command": str(fix["command"]).strip(), "explanation": str(fix.get("explanation", "")).strip()}
    return {"summary": str(d.get("summary", "")).strip(), "cause": str(d.get("cause", "")).strip(), "fix": fix}
