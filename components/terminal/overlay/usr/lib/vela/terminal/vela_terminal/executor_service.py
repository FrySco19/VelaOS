"""Vela safe executor service: per-user Unix-socket JSON API.

Socket: $XDG_RUNTIME_DIR/vela/executor.sock (dir 0700, socket 0600), or the fd passed by
systemd socket activation (LISTEN_FDS). Protocol: one JSON object per line.

  -> {"id": 1, "method": "prepare", "params": {"command": "...", "cwd": "...", "caller": "assistant", "intent": "..."}}
  <- {"id": 1, "result": {...}}            or {"id": 1, "error": {"code": "...", "message": "..."}}
  streaming run: {"id": 2, "event": "output", "stream": "stdout", "data": "..."} lines, then the result.

Methods: ping, classify, preview, prepare, run, cancel. See EXECUTOR_API.md.
"""
from __future__ import annotations

import asyncio
import json
import os
import secrets
import shutil
import socket
import sys
import threading
import time

from . import aiconfig
from .safe_exec import classify, preview, RISK
from . import runner
from .danger import classify as _classify, DANGER

VERSION = 1
TOKEN_TTL = 600


def socket_path() -> str:
    rt = os.environ.get("XDG_RUNTIME_DIR") or f"/tmp/vela-{os.getuid()}"
    return os.path.join(rt, "vela", "executor.sock")


class Executor:
    def __init__(self, confirm_cmd=None):
        self.tokens: dict[str, tuple[str, str, float]] = {}
        self.runs: dict[str, threading.Event] = {}
        # command used to show the danger confirmation dialog; must exit 0 only if confirmed
        self.confirm_cmd = confirm_cmd or os.environ.get("VELA_EXECUTOR_CONFIRM_CMD") or \
            f"{sys.executable} -m vela_terminal.confirm"

    # ------------------------------------------------------------------
    def _gc(self):
        now = time.time()
        for t in [t for t, (_c, _w, exp) in self.tokens.items() if exp < now]:
            del self.tokens[t]

    def m_ping(self, p):
        return {"version": VERSION, "pid": os.getpid(), "service": "vela-executor"}

    def m_classify(self, p):
        return classify(p["command"], p.get("lang"))

    def m_preview(self, p):
        return preview(p["command"], p.get("cwd") or os.path.expanduser("~"), p.get("lang"))

    def m_prepare(self, p):
        self._gc()
        cmd = p["command"]
        cwd = os.path.expanduser(p.get("cwd") or "~")
        info = classify(cmd, p.get("lang"))
        pv = preview(cmd, cwd, p.get("lang"))
        tok = secrets.token_urlsafe(18)
        self.tokens[tok] = (cmd, cwd, time.time() + TOKEN_TTL)
        return {"assessment": info, "preview": pv, "token": tok, "expires_in": TOKEN_TTL,
                "requires_executor_dialog": info["level"] == DANGER}

    def _confirm_danger(self, cmd, cwd, info, pv, caller) -> tuple[bool, str]:
        if not (os.environ.get("WAYLAND_DISPLAY") or os.environ.get("DISPLAY")) and \
                "VELA_EXECUTOR_CONFIRM_CMD" not in os.environ:
            return False, "no_display"
        payload = json.dumps({"command": cmd, "cwd": cwd, "assessment": info, "preview": pv,
                              "caller": caller}, ensure_ascii=False)
        import subprocess
        env = dict(os.environ)
        env["PYTHONPATH"] = os.path.dirname(os.path.dirname(os.path.abspath(__file__))) + \
            (":" + env["PYTHONPATH"] if env.get("PYTHONPATH") else "")
        try:
            r = subprocess.run(self.confirm_cmd, shell=True, input=payload, text=True, env=env,
                               timeout=600, capture_output=True)
        except subprocess.TimeoutExpired:
            return False, "timeout"
        return r.returncode == 0, ("confirmed" if r.returncode == 0 else "declined")

    def m_run(self, p, emit=None):
        cmd = p["command"]
        cwd = os.path.expanduser(p.get("cwd") or "~")
        caller = str(p.get("caller") or "unknown")[:40]
        tok = p.get("token")
        self._gc()
        entry = self.tokens.pop(tok, None) if tok else None
        if not entry or entry[0] != cmd or entry[1] != cwd:
            raise ApiError("needs_confirmation",
                           "Chiama prima 'prepare', mostra anteprima e rischio all'utente e passa il token ricevuto.")
        if aiconfig.run_permission() == "never":
            aiconfig.log_action("run_command", cmd, confirmed=False, result="refused_by_settings", app=caller)
            raise ApiError("forbidden", "L'esecuzione di comandi da parte dell'IA è disattivata in Impostazioni.")
        a = _classify(cmd)
        if a.level == DANGER:
            info = classify(cmd, p.get("lang"))
            pv = preview(cmd, cwd, p.get("lang"))
            ok, why = self._confirm_danger(cmd, cwd, info, pv, caller)
            if not ok:
                aiconfig.log_action("run_command", cmd, confirmed=False, result="danger_" + why, app=caller)
                raise ApiError("danger_not_confirmed",
                               {"no_display": "Comando pericoloso: serve una sessione grafica per la conferma.",
                                "declined": "L'utente non ha confermato il comando pericoloso.",
                                "timeout": "Conferma scaduta."}.get(why, why))
        run_id = secrets.token_hex(6)
        cancel = threading.Event()
        self.runs[run_id] = cancel
        if emit:
            emit({"event": "started", "run_id": run_id})
        try:
            res = runner.run(cmd, cwd=cwd, timeout=float(p.get("timeout") or 60),
                             on_output=(lambda s, d: emit({"event": "output", "stream": s, "data": d})) if emit and p.get("stream") else None,
                             cancel=cancel)
        finally:
            self.runs.pop(run_id, None)
        res["run_id"] = run_id
        res["risk"] = RISK[a.level]
        intent = str(p.get("intent") or "")
        aiconfig.log_action("run_command", (intent + " → " if intent else "") + cmd, confirmed=True,
                            result="ok" if res["rc"] == 0 else f"exit {res['rc']}", app=caller)
        return res

    def m_cancel(self, p):
        ev = self.runs.get(p.get("run_id", ""))
        if ev:
            ev.set()
            return {"cancelled": True}
        return {"cancelled": False}


class ApiError(Exception):
    def __init__(self, code, message):
        super().__init__(message)
        self.code = code
        self.message = message


_ACTIVE = {"conns": 0, "last": time.monotonic()}


async def _handle(ex: Executor, reader: asyncio.StreamReader, writer: asyncio.StreamWriter):
    _ACTIVE["conns"] += 1
    try:
        await _handle_conn(ex, reader, writer)
    finally:
        _ACTIVE["conns"] -= 1
        _ACTIVE["last"] = time.monotonic()


async def _handle_conn(ex: Executor, reader: asyncio.StreamReader, writer: asyncio.StreamWriter):
    loop = asyncio.get_running_loop()
    lock = asyncio.Lock()

    async def send(obj):
        async with lock:
            writer.write((json.dumps(obj, ensure_ascii=False) + "\n").encode())
            await writer.drain()

    async def serve_one(req):
        rid = req.get("id")
        method = req.get("method", "")
        params = req.get("params") or {}
        fn = getattr(ex, "m_" + method, None) if method.isidentifier() else None
        if fn is None:
            await send({"id": rid, "error": {"code": "unknown_method", "message": method}})
            return
        try:
            if method == "run":
                def emit(ev):
                    ev["id"] = rid
                    asyncio.run_coroutine_threadsafe(send(ev), loop)
                res = await loop.run_in_executor(None, lambda: fn(params, emit))
            else:
                res = await loop.run_in_executor(None, fn, params)
            await send({"id": rid, "result": res})
        except ApiError as e:
            await send({"id": rid, "error": {"code": e.code, "message": e.message}})
        except KeyError as e:
            await send({"id": rid, "error": {"code": "bad_request", "message": f"missing {e}"}})
        except Exception as e:  # noqa: BLE001
            await send({"id": rid, "error": {"code": "internal", "message": str(e)}})

    tasks = set()
    try:
        while True:
            line = await reader.readline()
            if not line:
                break
            if len(line) > 1_000_000:
                await send({"id": None, "error": {"code": "too_large", "message": "request too large"}})
                continue
            try:
                req = json.loads(line)
            except ValueError:
                await send({"id": None, "error": {"code": "bad_json", "message": "invalid JSON"}})
                continue
            t = asyncio.create_task(serve_one(req))
            tasks.add(t)
            t.add_done_callback(tasks.discard)
        if tasks:
            await asyncio.gather(*tasks, return_exceptions=True)
    finally:
        writer.close()


async def serve(path: str | None = None, idle_exit: float = 0):
    ex = Executor()
    lfds = int(os.environ.get("LISTEN_FDS", "0") or 0)
    if lfds >= 1 and os.environ.get("LISTEN_PID") in (None, str(os.getpid())):
        sock = socket.socket(fileno=3)
        server = await asyncio.start_unix_server(lambda r, w: _handle(ex, r, w), sock=sock)
    else:
        path = path or socket_path()
        os.makedirs(os.path.dirname(path), mode=0o700, exist_ok=True)
        os.chmod(os.path.dirname(path), 0o700)
        if os.path.exists(path):
            os.unlink(path)
        old = os.umask(0o177)
        try:
            server = await asyncio.start_unix_server(lambda r, w: _handle(ex, r, w), path=path)
        finally:
            os.umask(old)
    import signal as _signal
    loop = asyncio.get_running_loop()
    stop = loop.create_future()
    for sig in (_signal.SIGTERM, _signal.SIGINT):
        try:
            loop.add_signal_handler(sig, lambda: stop.done() or stop.set_result(True))
        except (NotImplementedError, RuntimeError):
            pass
    async def idle_watch():
        while not stop.done():
            await asyncio.sleep(min(30.0, idle_exit / 2))
            if _ACTIVE["conns"] == 0 and not ex.runs and \
                    time.monotonic() - _ACTIVE["last"] > idle_exit and not stop.done():
                stop.set_result(True)

    watcher = asyncio.create_task(idle_watch()) if idle_exit > 0 else None
    async with server:
        await stop
    if watcher:
        watcher.cancel()
    if lfds < 1 and path and os.path.exists(path):
        try:
            os.unlink(path)
        except OSError:
            pass


def main(argv=None):
    import argparse
    ap = argparse.ArgumentParser(prog="vela-executor", description="Vela safe executor service")
    ap.add_argument("--socket", help="socket path (default $XDG_RUNTIME_DIR/vela/executor.sock)")
    ap.add_argument("--idle-exit", type=float, default=0, metavar="SEC",
                    help="exit after SEC seconds without clients (socket activation restarts it)")
    args = ap.parse_args(argv)
    try:
        asyncio.run(serve(args.socket, args.idle_exit))
    except KeyboardInterrupt:
        pass


if __name__ == "__main__":
    main()
