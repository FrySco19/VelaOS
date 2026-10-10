"""Vela AI configuration helper (contract: AI_CONFIG.md, version 1).

Precedence: /etc/vela/ai.conf < /etc/vela/ai.conf.d/*.json < ~/.config/vela/ai.json < VELA_AI_* env.
The API key is kept in ~/.config/vela/ai-secret (0600), never in ai.json.

may(cfg, perm) returns (decision, reason) where decision is "allow", "ask" or "never".
"""
import copy, datetime, glob, json, os, tempfile

SYSTEM_CONF = "/etc/vela/ai.conf"
SYSTEM_DIR = "/etc/vela/ai.conf.d"

BUILTIN = {
    "version": 1,
    "enabled": True,
    "provider": "openai",
    "ollama": {"base_url": "http://127.0.0.1:11434", "model": "qwen2.5:1.5b"},
    "openai": {"base_url": "http://127.0.0.1:11435/v1", "model": "vela-local", "organization": ""},
    "generation": {"temperature": 0.3, "max_tokens": 1024, "timeout_s": 60},
    "language": "auto",
    "assistant": {"shortcut": "Meta+A", "history": True, "tray": True},
    "permissions": {
        "active_window": True, "selected_text": True, "clipboard": False,
        "files": "ask", "screen": "ask", "microphone": "ask", "web": "ask",
        "run_commands": "never", "network_settings": "ask", "system_changes": "ask",
    },
    "privacy": {
        "allow_cloud": False, "log_actions": True, "log_retention_days": 30,
        "memory_enabled": True, "memory_paused": False,
        "memory_exclude_paths": ["~/.ssh", "~/.gnupg", "~/.local/share/keyrings"],
        "memory_exclude_sites": [], "redact_secrets": True,
    },
}


def _xdg(var, default):
    return os.environ.get(var) or os.path.expanduser(default)


def user_dir():
    return os.path.join(_xdg("XDG_CONFIG_HOME", "~/.config"), "vela")


def user_conf():
    return os.path.join(user_dir(), "ai.json")


def secret_path():
    return os.path.join(user_dir(), "ai-secret")


def data_dir():
    return os.path.join(_xdg("XDG_DATA_HOME", "~/.local/share"), "vela")


def action_log_path():
    return os.path.join(data_dir(), "ai-actions.jsonl")


def memory_dir():
    return os.path.join(data_dir(), "memory")


def merge(base, over):
    out = copy.deepcopy(base)
    for k, v in (over or {}).items():
        if isinstance(v, dict) and isinstance(out.get(k), dict):
            out[k] = merge(out[k], v)
        else:
            out[k] = copy.deepcopy(v)
    return out


def _read_json(path):
    try:
        with open(path, encoding="utf-8") as f:
            data = json.load(f)
        return data if isinstance(data, dict) else {}
    except (OSError, ValueError):
        return {}


def load_system():
    cfg = merge(BUILTIN, _read_json(SYSTEM_CONF))
    for p in sorted(glob.glob(os.path.join(SYSTEM_DIR, "*.json"))):
        cfg = merge(cfg, _read_json(p))
    return cfg


def load_user_raw():
    return _read_json(user_conf())


def load(env=True):
    cfg = merge(load_system(), load_user_raw())
    if env:
        e = os.environ
        if e.get("VELA_AI_PROVIDER"):
            cfg["provider"] = e["VELA_AI_PROVIDER"]
        prov = cfg.get("provider", "ollama")
        sect = cfg.setdefault(prov if prov in ("ollama", "openai") else "openai", {})
        if e.get("VELA_AI_BASE_URL"):
            sect["base_url"] = e["VELA_AI_BASE_URL"]
        if e.get("VELA_AI_MODEL"):
            sect["model"] = e["VELA_AI_MODEL"]
    return cfg


def _atomic_write(path, text, mode=0o600):
    d = os.path.dirname(path)
    os.makedirs(d, mode=0o700, exist_ok=True)
    fd, tmp = tempfile.mkstemp(dir=d, prefix=".tmp-")
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as f:
            f.write(text)
        os.chmod(tmp, mode)
        os.replace(tmp, path)
    except BaseException:
        try:
            os.unlink(tmp)
        except OSError:
            pass
        raise


def save_user(changes):
    """Merge `changes` into ~/.config/vela/ai.json (unknown keys preserved)."""
    data = merge(load_user_raw(), changes)
    data["version"] = 1
    _atomic_write(user_conf(), json.dumps(data, indent=2, ensure_ascii=False) + "\n", 0o644)
    return data


def reset_user():
    try:
        os.unlink(user_conf())
    except FileNotFoundError:
        pass


def get_api_key():
    if os.environ.get("VELA_AI_API_KEY"):
        return os.environ["VELA_AI_API_KEY"]
    try:
        with open(secret_path(), encoding="utf-8") as f:
            return f.read().strip()
    except OSError:
        return ""


def set_api_key(key):
    key = (key or "").strip()
    if not key:
        try:
            os.unlink(secret_path())
        except FileNotFoundError:
            pass
        return
    _atomic_write(secret_path(), key + "\n", 0o600)


def endpoint(cfg=None, with_key=True):
    cfg = cfg or load()
    prov = cfg.get("provider", "ollama")
    if prov == "ollama":
        s = cfg.get("ollama", {})
        base = s.get("base_url", "http://127.0.0.1:11434").rstrip("/")
        return {"provider": "ollama", "base_url": base + "/v1", "native_url": base,
                "model": s.get("model", ""), "api_key": "ollama" if with_key else "", "cloud": False}
    s = cfg.get("openai", {})
    base = s.get("base_url", "https://api.openai.com/v1").rstrip("/")
    host = base.split("//", 1)[-1].split("/", 1)[0].split(":", 1)[0]
    local = host in ("127.0.0.1", "localhost", "::1")
    key = get_api_key() if with_key else ""
    if base == "http://127.0.0.1:11435/v1":
        key = ""
        if with_key:
            runtime = os.environ.get("XDG_RUNTIME_DIR") or f"/run/user/{os.getuid()}"
            try:
                with open(os.path.join(runtime, "vela-local-ai.key"), encoding="utf-8") as f:
                    key = f.read().strip()
            except OSError:
                pass
    return {"provider": "openai", "base_url": base, "model": s.get("model", ""),
            "api_key": key, "cloud": not local}


def may(cfg, perm):
    """Return (decision, reason). decision in {"allow","ask","never"}."""
    if not cfg.get("enabled", True):
        return "never", "L'assistente è disattivato in Impostazioni."
    ep = endpoint(cfg, with_key=False)
    if ep["cloud"] and not cfg.get("privacy", {}).get("allow_cloud", False):
        return "never", "L'invio a un servizio esterno non è consentito in Impostazioni > Privacy e memoria."
    v = cfg.get("permissions", {}).get(perm, "ask")
    if v is True or v in ("allow", "home"):
        return "allow", ""
    if v is False or v == "never":
        return "never", "Permesso negato in Impostazioni > Intelligenza artificiale."
    return "ask", ""


def log_action(app, action, summary, sent_off_machine=False, confirmed=None, result="ok", provider=None):
    cfg = load()
    if not cfg.get("privacy", {}).get("log_actions", True):
        return
    rec = {"ts": datetime.datetime.now().astimezone().isoformat(timespec="seconds"),
           "app": app, "action": action, "summary": str(summary)[:500],
           "provider": provider or cfg.get("provider"), "sent_off_machine": bool(sent_off_machine),
           "confirmed": confirmed, "result": result}
    os.makedirs(data_dir(), mode=0o700, exist_ok=True)
    with open(action_log_path(), "a", encoding="utf-8") as f:
        f.write(json.dumps(rec, ensure_ascii=False) + "\n")


def read_actions(limit=500):
    try:
        with open(action_log_path(), encoding="utf-8") as f:
            lines = f.readlines()[-limit:]
    except OSError:
        return []
    out = []
    for ln in lines:
        try:
            out.append(json.loads(ln))
        except ValueError:
            pass
    return out


def prune_actions(days=None):
    cfg = load()
    days = days if days is not None else int(cfg.get("privacy", {}).get("log_retention_days", 30))
    if days <= 0:
        return
    cutoff = datetime.datetime.now().astimezone() - datetime.timedelta(days=days)
    keep = []
    for r in read_actions(limit=10**7):
        try:
            if datetime.datetime.fromisoformat(r["ts"]) >= cutoff:
                keep.append(r)
        except (KeyError, ValueError):
            pass
    if os.path.exists(action_log_path()):
        _atomic_write(action_log_path(), "".join(json.dumps(r, ensure_ascii=False) + "\n" for r in keep))


def clear_actions():
    try:
        os.unlink(action_log_path())
    except FileNotFoundError:
        pass


def _get(cfg, dotted):
    cur = cfg
    for part in dotted.split("."):
        if not isinstance(cur, dict) or part not in cur:
            raise KeyError(dotted)
        cur = cur[part]
    return cur


def main(argv=None):
    import sys
    argv = list(sys.argv[1:] if argv is None else argv)
    usage = "uso: vela-ai-config get KEY | set KEY VALUE | json | endpoint [--with-key] | reset"
    if not argv:
        print(usage); return 2
    cmd = argv.pop(0)
    if cmd == "json":
        print(json.dumps(load(), indent=2, ensure_ascii=False)); return 0
    if cmd == "endpoint":
        print(json.dumps(endpoint(with_key="--with-key" in argv), indent=2)); return 0
    if cmd == "get" and argv:
        try:
            v = _get(load(), argv[0])
        except KeyError:
            print(f"chiave sconosciuta: {argv[0]}", file=sys.stderr); return 1
        print(json.dumps(v, ensure_ascii=False) if isinstance(v, (dict, list, bool)) or v is None else v); return 0
    if cmd == "set" and len(argv) == 2:
        try:
            val = json.loads(argv[1])
        except ValueError:
            val = argv[1]
        changes = cur = {}
        parts = argv[0].split(".")
        for p in parts[:-1]:
            cur = cur.setdefault(p, {})
        cur[parts[-1]] = val
        save_user(changes); return 0
    if cmd == "reset":
        reset_user(); return 0
    print(usage); return 2


if __name__ == "__main__":
    raise SystemExit(main())
