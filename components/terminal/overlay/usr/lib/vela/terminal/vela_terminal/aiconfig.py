"""Access to the shared Vela AI configuration (contract: settings-store/AI_CONFIG.md).

Uses the official helper /usr/lib/vela/python/vela_ai_config.py when present; otherwise a
minimal, read-only compatible fallback (same files, same precedence) so the terminal still
works on a system without the settings-store component.
"""
from __future__ import annotations

import datetime
import json
import os
import sys

for _p in (os.environ.get("VELA_PYTHON_PATH", ""), "/usr/lib/vela/python"):
    if _p and os.path.isdir(_p) and _p not in sys.path:
        sys.path.append(_p)

try:  # pragma: no cover - depends on installation
    import vela_ai_config as _vac  # type: ignore
except Exception:  # noqa: BLE001
    _vac = None

APP = "terminal"


def _merge(a, b):
    out = dict(a)
    for k, v in (b or {}).items():
        out[k] = _merge(out[k], v) if isinstance(v, dict) and isinstance(out.get(k), dict) else v
    return out


def _read(path):
    try:
        with open(path, encoding="utf-8") as f:
            d = json.load(f)
        return d if isinstance(d, dict) else {}
    except (OSError, ValueError):
        return {}


def _cfg_home():
    return os.environ.get("XDG_CONFIG_HOME") or os.path.expanduser("~/.config")


def _data_home():
    return os.environ.get("XDG_DATA_HOME") or os.path.expanduser("~/.local/share")


def load() -> dict:
    if _vac is not None:
        return _vac.load()
    import glob
    cfg = {"version": 1, "enabled": True, "provider": "ollama",
           "ollama": {"base_url": "http://127.0.0.1:11434", "model": ""},
           "openai": {"base_url": "https://api.openai.com/v1", "model": ""},
           "generation": {"temperature": 0.3, "max_tokens": 1024, "timeout_s": 60},
           "language": "auto", "permissions": {"run_commands": "ask"},
           "privacy": {"allow_cloud": False, "log_actions": True, "redact_secrets": True}}
    cfg = _merge(cfg, _read("/etc/vela/ai.conf"))
    for p in sorted(glob.glob("/etc/vela/ai.conf.d/*.json")):
        cfg = _merge(cfg, _read(p))
    cfg = _merge(cfg, _read(os.path.join(_cfg_home(), "vela", "ai.json")))
    e = os.environ
    if e.get("VELA_AI_PROVIDER"):
        cfg["provider"] = e["VELA_AI_PROVIDER"]
    sect = cfg.setdefault(cfg.get("provider", "ollama"), {})
    if e.get("VELA_AI_BASE_URL"):
        sect["base_url"] = e["VELA_AI_BASE_URL"]
    if e.get("VELA_AI_MODEL"):
        sect["model"] = e["VELA_AI_MODEL"]
    return cfg


def endpoint(cfg=None) -> dict:
    cfg = cfg or load()
    if _vac is not None:
        return _vac.endpoint(cfg)
    prov = cfg.get("provider", "ollama")
    if prov == "ollama":
        s = cfg.get("ollama", {})
        base = s.get("base_url", "http://127.0.0.1:11434").rstrip("/")
        return {"provider": "ollama", "base_url": base + "/v1", "native_url": base,
                "model": s.get("model", ""), "api_key": "ollama", "cloud": False}
    s = cfg.get("openai", {})
    base = s.get("base_url", "https://api.openai.com/v1").rstrip("/")
    host = base.split("//", 1)[-1].split("/", 1)[0].split(":", 1)[0]
    key = os.environ.get("VELA_AI_API_KEY", "")
    if not key:
        try:
            with open(os.path.join(_cfg_home(), "vela", "ai-secret"), encoding="utf-8") as f:
                key = f.read().strip()
        except OSError:
            key = ""
    return {"provider": "openai", "base_url": base, "model": s.get("model", ""), "api_key": key,
            "cloud": host not in ("127.0.0.1", "localhost", "::1")}


def language(cfg=None) -> str:
    cfg = cfg or load()
    lang = cfg.get("language", "auto")
    if lang in ("it", "en"):
        return lang
    loc = os.environ.get("LC_ALL") or os.environ.get("LC_MESSAGES") or os.environ.get("LANG") or ""
    return "en" if loc.startswith("en") else "it"


def ai_status(cfg=None) -> tuple[bool, str, dict]:
    """(usable, reason_code, endpoint). reason_code: ok|disabled|cloud_not_allowed|no_model|run_never."""
    cfg = cfg or load()
    ep = endpoint(cfg)
    if not cfg.get("enabled", True):
        return False, "disabled", ep
    if ep.get("cloud") and not cfg.get("privacy", {}).get("allow_cloud", False):
        return False, "cloud_not_allowed", ep
    if ep.get("provider") == "openai" and not ep.get("api_key") and ep.get("cloud"):
        return False, "no_key", ep
    if not ep.get("model") and ep.get("provider") == "openai":
        return False, "no_model", ep
    return True, "ok", ep


def run_permission(cfg=None) -> str:
    """'ask' or 'never' for the AI running commands ('allow' is treated as 'ask' by the terminal)."""
    cfg = cfg or load()
    if not cfg.get("enabled", True):
        return "never"
    v = cfg.get("permissions", {}).get("run_commands", "ask")
    return "never" if v in ("never", False) else "ask"


def log_action(action, summary, sent_off_machine=False, confirmed=None, result="ok", app=APP):
    try:
        if _vac is not None:
            _vac.log_action(app=app, action=action, summary=summary, sent_off_machine=sent_off_machine,
                            confirmed=confirmed, result=result)
            return
        cfg = load()
        if not cfg.get("privacy", {}).get("log_actions", True):
            return
        path = os.path.join(_data_home(), "vela", "ai-actions.jsonl")
        os.makedirs(os.path.dirname(path), mode=0o700, exist_ok=True)
        rec = {"ts": datetime.datetime.now().astimezone().isoformat(timespec="seconds"), "app": app,
               "action": action, "summary": str(summary)[:500], "provider": cfg.get("provider"),
               "sent_off_machine": bool(sent_off_machine), "confirmed": confirmed, "result": result}
        with open(path, "a", encoding="utf-8") as f:
            f.write(json.dumps(rec, ensure_ascii=False) + "\n")
    except Exception:  # logging must never break the terminal
        pass


def settings_command() -> list:
    """Command that opens the AI settings page."""
    import shutil
    if shutil.which("systemsettings"):
        return ["systemsettings", "kcm_vela_ai"]
    if shutil.which("kcmshell6"):
        return ["kcmshell6", "kcm_vela_ai"]
    return []
