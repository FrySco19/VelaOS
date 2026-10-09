# Vela safe executor — API

The same rules the **Terminale IA** uses to decide whether a command is harmless, changes
something, or is dangerous, and the same "preview first, run only after confirmation" flow,
available to other Vela components (the assistant, scripts). Owner: component `terminal`.

There are three ways in, all with the same classifier (`vela_terminal.danger`):

| Way | For | Where |
|---|---|---|
| Python module | in-process callers that already show their own confirmation UI | `/usr/lib/vela/python/vela_safe_exec.py` |
| Unix-socket service | other processes; the executor shows **its own** dialog for dangerous commands | `$XDG_RUNTIME_DIR/vela/executor.sock` |
| CLI | shell scripts, debugging | `vela-exec classify\|preview\|run` |

To let the user review a command inside the Terminale IA (proposal card, preview, risk chip,
real shell) instead: `vela-ai-terminal --propose "<cmd>" --explanation "<una frase>" [--workdir DIR]`.

## Risk levels

| `risk` | `level` | Meaning | What the caller must do |
|---|---|---|---|
| `safe` | `safe` | read-only: lists, shows, searches (`ls`, `df -h`, `git status`) | still ask before running (rule of the AI config) |
| `changes` | `change` | modifies files, settings or packages in a normal way; also any unknown program | ask before running |
| `dangerous` | `danger` | can lose data or break the system: `rm -r`, `sudo`, disks, `curl … \| sh`, `git reset --hard`, power, killing everything, writes to system paths, … | separate, explicit confirmation that names the danger |

Unknown programs are never `safe`. If the parser is unsure (`uncertain: true`) the level is
raised, never lowered.

## 1. Python module

```python
import sys; sys.path.insert(0, "/usr/lib/vela/python")
import vela_safe_exec as vx

a = vx.classify("sudo rm -rf /var/cache/apt/archives")      # lang from the AI config (it/en)
a["risk"]          # "dangerous"
a["reasons"]       # ["usa sudo: esegue come amministratore (root)", "cancella file di sistema (...)", ...]  plain strings, localized
a["reason_objects"]# [{"level": "danger", "code": "privilege", "text": ..., "it": ..., "en": ...}, ...]
a["needs_sudo"]    # True
a["requires_explicit_confirmation"]  # True for dangerous
a["summary"]       # all reasons in one line
a["programs"]      # ["sudo", "rm"]
a["uncertain"]     # parser was not sure (level already raised)

p = vx.preview("rm -rf build dist", cwd="/home/vela/progetto")
# {"kind": "preview"|"readonly"|"none", "exact": bool, "items": int, "text": "Verranno cancellati 2 elementi ..."}
# Built only from safe simulations (glob/walk, apt-get -s, git clean -n, rsync --dry-run,
# sed --sandbox diff, find without -delete, tar -t, unzip -l, lsblk). Never runs the command.

r = vx.run(cmd, cwd=None, timeout=60, confirmed=True, confirmed_danger=False, caller="assistant", intent="")
# {"rc": int, "stdout": str, "stderr": str, "truncated": bool, "duration_ms": int,
#  "timed_out": bool, "cancelled": bool, "risk": "safe"|"changes"|"dangerous"}
```

`run()` raises `vx.NotConfirmed` (a `PermissionError`) when:
- `confirmed` is not `True` — set it only after the user confirmed **this exact command**;
- the command is dangerous and `confirmed_danger` is not `True` — set it only after a second,
  explicit confirmation that showed the reasons;
- the user turned off command execution in Impostazioni → IA (`permissions.run_commands = "never"`).

Commands run with `bash -c` in a new session, stdin `/dev/null`, `TERM=dumb`, `PAGER=cat`,
no TTY (so `sudo` without cached credentials fails cleanly; use the Terminale IA for commands
that need a password). Output is capped at 256 KB per stream (`truncated: true`). Every run is
logged in `~/.local/share/vela/ai-actions.jsonl` through the settings-store helper.

## 2. Service (Unix socket, JSON lines)

- Socket: `$XDG_RUNTIME_DIR/vela/executor.sock` (dir 0700, socket 0600 — only your user).
  Started on demand by the systemd user unit `vela-executor.socket`; exits after 10 minutes idle.
- One JSON object per line, UTF-8. Requests carry an `id`; replies echo it.
  Several requests may be in flight on one connection.

```
-> {"id": 1, "method": "ping"}
<- {"id": 1, "result": {"version": 1, "pid": 1234, "service": "vela-executor"}}
<- {"id": 7, "error": {"code": "needs_confirmation", "message": "..."}}
```

| Method | Params | Result |
|---|---|---|
| `ping` | – | `{version, pid, service}` |
| `classify` | `command`, `lang?` | assessment, as in the module |
| `preview` | `command`, `cwd?`, `lang?` | preview, as in the module |
| `prepare` | `command`, `cwd?`, `lang?` | `{assessment, preview, token, expires_in: 600, requires_executor_dialog}` |
| `run` | `command`, `cwd?`, `token`, `caller`, `intent?`, `timeout?` (s, default 60), `stream?` (bool), `lang?` | run result + `run_id` |
| `cancel` | `run_id` | `{cancelled: bool}` |

Flow:
1. `prepare` → show the user the command, `assessment.reasons` and `preview.text`, ask.
2. Only if the user said yes: `run` with the **same** `command` and `cwd` and the `token`.
   Tokens are single-use, bound to (command, cwd), valid 10 minutes.
3. If the command is dangerous, the service shows **its own** confirmation window
   (*"Comando pericoloso"*, the command, the reasons, the preview; the run button stays disabled
   until the user ticks *"Ho letto e voglio eseguire proprio questo comando"*). This is on top of
   the caller's confirmation and cannot be skipped by the caller. Without a graphical session it
   refuses (`danger_not_confirmed`).
4. With `stream: true` the service sends, before the final result:
   `{"id": N, "event": "started", "run_id": "..."}` and
   `{"id": N, "event": "output", "stream": "stdout"|"stderr", "data": "..."}`.
   `cancel` (on any connection) stops the process group.

Error codes: `needs_confirmation` (no/wrong/expired token), `forbidden` (execution turned off in
Impostazioni), `danger_not_confirmed` (user declined, timed out, or no display), `bad_request`
(missing param), `unknown_method`, `bad_json`, `too_large` (request line > 1 MB), `internal`.

Python client (same package):

```python
sys.path.insert(0, "/usr/lib/vela/terminal")
from vela_terminal.client import ExecutorClient, ExecutorError
with ExecutorClient() as c:
    prep = c.call("prepare", command="du -sh ~/Scaricati", cwd="~")
    # ... ask the user ...
    res = c.call("run", command="du -sh ~/Scaricati", cwd="~", token=prep["token"],
                 caller="assistant", intent="quanto occupa Scaricati", stream=True,
                 on_event=lambda ev: print(ev.get("data", ""), end=""))
```

Shell test: `printf '%s\n' '{"id":1,"method":"classify","params":{"command":"rm -rf ~"}}' | socat - UNIX-CONNECT:$XDG_RUNTIME_DIR/vela/executor.sock`

**Security note.** The socket is protected by file permissions only: any process running as
the same user can talk to it, exactly as it could run a shell. Its value is that dangerous
commands always go through the executor's own dialog and every run is logged; it is not a
sandbox. The dialog command can be overridden with `VELA_EXECUTOR_CONFIRM_CMD` (used by tests).

## 3. CLI

```
vela-exec classify "rm -rf ~/x"          # JSON assessment
vela-exec preview  "rm -rf build" [--cwd DIR]
vela-exec run      "du -sh ~"     [--cwd DIR] [--yes]  # shows risk + preview, asks; dangerous: type «sì» (--yes never skips that)
```
