"""Safe executor core: classify -> preview -> confirmed run, with action logging.

This is the in-process API (also exported as /usr/lib/vela/python/vela_safe_exec.py).
Hard rule: run() refuses unless confirmed=True; for "dangerous" commands it also needs
confirmed_danger=True, which callers may only set after the user explicitly confirmed the
danger in a dedicated UI (the executor service shows its own dialog for that).
"""
from __future__ import annotations

from . import aiconfig
from .danger import classify as _classify, SAFE, CHANGE, DANGER
from .preview import preview as _preview
from . import runner

RISK = {SAFE: "safe", CHANGE: "changes", DANGER: "dangerous"}


class NotConfirmed(PermissionError):
    pass


def classify(cmd: str, lang: str | None = None) -> dict:
    lang = lang or aiconfig.language()
    a = _classify(cmd)
    d = a.to_dict(lang)
    d["risk"] = RISK[a.level]
    d["reasons_text"] = [r["text"] for r in d["reasons"]]
    d["needs_sudo"] = any(r.code == "privilege" for r in a.reasons)
    # compatibility with the assistant proposal: reasons as plain strings
    d["reason_objects"] = d["reasons"]
    d["reasons"] = d["reasons_text"]
    return d


def preview(cmd: str, cwd: str | None = None, lang: str | None = None) -> dict:
    return _preview(cmd, cwd, lang or aiconfig.language())


def run(cmd: str, cwd: str | None = None, timeout: float = 60, confirmed: bool = False,
        confirmed_danger: bool = False, caller: str = "terminal", intent: str = "",
        on_output=None, cancel=None) -> dict:
    a = _classify(cmd)
    if aiconfig.run_permission() == "never":
        aiconfig.log_action("run_command", cmd, confirmed=False, result="refused_by_settings", app=caller)
        raise NotConfirmed("L'esecuzione di comandi da parte dell'IA è disattivata in Impostazioni.")
    if not confirmed:
        raise NotConfirmed("Serve la conferma dell'utente prima di eseguire un comando.")
    if a.level == DANGER and not confirmed_danger:
        raise NotConfirmed("Comando pericoloso: serve una conferma esplicita e separata dell'utente.")
    res = runner.run(cmd, cwd=cwd, timeout=timeout, on_output=on_output, cancel=cancel)
    res["risk"] = RISK[a.level]
    aiconfig.log_action("run_command", (intent + " → " if intent else "") + cmd, confirmed=True,
                        result="ok" if res["rc"] == 0 else f"exit {res['rc']}", app=caller)
    return res
