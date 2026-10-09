# Vela AI configuration — shared contract (v1)

Status: **stable v1** — additive changes only; breaking changes bump `version`.

## Files and precedence (lowest → highest)
1. `/etc/vela/ai.conf` — system defaults, JSON, shipped by settings-store (root-owned, 0644).
2. `/etc/vela/ai.conf.d/*.json` — optional drop-ins (other components/OEM), merged in lexical order.
3. `~/.config/vela/ai.json` — per-user overrides written by System Settings → "Intelligenza artificiale"
   and "Privacy e memoria" (and by onboarding). Honour `$XDG_CONFIG_HOME`.
4. Environment: `VELA_AI_PROVIDER`, `VELA_AI_BASE_URL`, `VELA_AI_MODEL`, `VELA_AI_API_KEY` (debug/dev only).

Merge = recursive dict merge (objects merged key by key, scalars/arrays replaced). Unknown keys must be ignored
and preserved on rewrite. Both files are UTF-8 JSON (`ai.conf` is JSON despite the extension).

**Secrets are never stored in ai.json.** The API key lives in `~/.config/vela/ai-secret` (single line, mode 0600,
dir 0700). A future version may move it to KWallet/Secret Service; always use the helper below.

## Schema
```jsonc
{
  "version": 1,
  "enabled": true,                       // master switch: false => no AI feature may run or send anything
  "provider": "ollama",                  // "ollama" | "openai"  (openai = any OpenAI-compatible endpoint)
  "ollama": {
    "base_url": "http://127.0.0.1:11434",
    "model": "qwen2.5:1.5b"              // model tag for chat; empty => first installed model
  },
  "openai": {
    "base_url": "https://api.openai.com/v1",
    "model": "gpt-4o-mini",
    "organization": ""                   // optional
  },
  "generation": { "temperature": 0.3, "max_tokens": 1024, "timeout_s": 60 },
  "language": "auto",                    // "auto" (follow system locale) | "it" | "en"
  "assistant": {
    "shortcut": "Meta+A",                // informational; the real binding is owned by desktop/assistant
    "history": true,                     // keep conversation history in ~/.local/share/vela/memory/assistant/
                                         //   (only when privacy.memory_enabled && !privacy.memory_paused)
    "tray": true                         // show the assistant icon in the top bar tray
  },
  "permissions": {                       // what the assistant/agents may read or do
    "active_window": true,               // title + app of the focused window
    "selected_text": true,               // primary selection / clipboard on request
    "clipboard": false,                  // read clipboard automatically
    "files": "ask",                      // "never" | "ask" | "home" (read files in $HOME without asking)
    "screen": "ask",                     // "never" | "ask"  (screenshot/screen vision)
    "microphone": "ask",                 // "never" | "ask"
    "web": "ask",                        // browser agent / fetching URLs: "never" | "ask" | "allow"
    "run_commands": "ask",               // AI terminal / actions: "never" | "ask"   ("ask" = preview + confirm)
    "network_settings": "ask",
    "system_changes": "ask"              // packages, settings, services
  },
  "privacy": {
    "allow_cloud": false,                // must be true for provider=openai to send anything off-machine
    "log_actions": true,                 // write the action log below
    "log_retention_days": 30,
    "memory_enabled": true,              // personal semantic memory/index
    "memory_paused": false,
    "memory_exclude_paths": ["~/.ssh", "~/.gnupg", "~/.local/share/keyrings"],
    "memory_exclude_sites": [],
    "redact_secrets": true               // strip things that look like keys/passwords before sending to an API
  }
}
```

## Runtime paths (shared)
- Action log (append-only JSON Lines): `~/.local/share/vela/ai-actions.jsonl`
  one object per line: `{"ts":"2026-10-08T20:50:00+02:00","app":"terminal","action":"run_command",
  "summary":"ls -la ~/Downloads","provider":"ollama","sent_off_machine":false,"confirmed":true,"result":"ok"}`
  Writers: every AI component. Reader/clearer: Settings → Privacy e memoria.
- Memory/index data: `~/.local/share/vela/memory/` (owned by `assistant`; Settings only shows size and can delete it
  after asking).
- Change notification: after writing ai.json Settings also touches `~/.config/vela/ai.json` atomically
  (write temp + rename). Consumers should re-read on each request or watch the file (QFileSystemWatcher/inotify on the dir).

## Helper (installed by settings-store)
- Python module: `/usr/lib/vela/python/vela_ai_config.py`
  ```python
  import sys; sys.path.insert(0, "/usr/lib/vela/python")
  import vela_ai_config as vac
  cfg = vac.load()                 # merged dict
  ep  = vac.endpoint(cfg)          # {"provider","base_url","model","api_key","cloud"}  (OpenAI-style chat/completions)
  ok, why = vac.may(cfg, "run_commands")   # -> ("ask"|"allow"|"never") semantics helper, see module docstring
  vac.log_action(app="terminal", action="run_command", summary="...", sent_off_machine=False, confirmed=True)
  vac.save_user({"provider": "openai"})    # merges into ~/.config/vela/ai.json
  vac.set_api_key("sk-...") / vac.get_api_key()
  ```
- CLI: `vela-ai-config get provider`, `vela-ai-config get ollama.model`, `vela-ai-config json`,
  `vela-ai-config endpoint` (prints JSON without the key unless `--with-key`), `vela-ai-config set key value`.

Both Ollama (>=0.1.24) and OpenAI-compatible servers expose `POST {base_url}/v1/chat/completions` —
for Ollama use `base_url + "/v1"`; `endpoint()` already returns the right `/v1` URL in both cases.

## Local model install
Settings never installs Ollama itself: its «Installa Ollama» button runs `vela-assistant --setup-local`
(owned by assistant component: per-user install, explicit click only), then re-checks the connection.

## Rules for consumers
- If `enabled` is false: do not call any model; show "L'assistente è disattivato in Impostazioni".
- If `provider == "openai"` and `privacy.allow_cloud` is false: refuse and point to Settings (no silent fallback).
- Never send content to a remote API without `allow_cloud`; mark `sent_off_machine:true` in the log when you do.
- "ask" ⇒ show a preview and require explicit confirmation every time.
