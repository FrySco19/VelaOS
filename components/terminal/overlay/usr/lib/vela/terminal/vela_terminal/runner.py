"""Non-interactive command runner used by the safe executor (no TTY)."""
from __future__ import annotations

import os
import selectors
import signal
import subprocess
import time

MAX_BYTES = 256 * 1024


def run(command: str, cwd: str | None = None, timeout: float = 60, max_bytes: int = MAX_BYTES,
        on_output=None, cancel=None, env=None) -> dict:
    """Run `command` with bash -c in its own process group.

    on_output(stream_name, text) is called as output arrives. `cancel` is an object with
    is_set() (threading.Event) checked while running. Never raises for command failures.
    """
    cwd = os.path.expanduser(cwd or "~")
    if not os.path.isdir(cwd):
        cwd = os.path.expanduser("~")
    t0 = time.monotonic()
    e = dict(os.environ if env is None else env)
    e.setdefault("LANG", "it_IT.UTF-8")
    e["PAGER"] = "cat"
    e["GIT_PAGER"] = "cat"
    e["SYSTEMD_PAGER"] = ""
    e["DEBIAN_FRONTEND"] = e.get("DEBIAN_FRONTEND", "noninteractive")
    e["TERM"] = "dumb"
    try:
        p = subprocess.Popen(["bash", "-c", command], cwd=cwd, stdin=subprocess.DEVNULL,
                             stdout=subprocess.PIPE, stderr=subprocess.PIPE, env=e,
                             start_new_session=True)
    except OSError as ex:
        return {"rc": 127, "stdout": "", "stderr": str(ex), "truncated": False,
                "duration_ms": 0, "timed_out": False, "cancelled": False}
    out = {"stdout": bytearray(), "stderr": bytearray()}
    truncated = False
    timed_out = cancelled = False
    sel = selectors.DefaultSelector()
    sel.register(p.stdout, selectors.EVENT_READ, "stdout")
    sel.register(p.stderr, selectors.EVENT_READ, "stderr")
    open_streams = 2
    while open_streams:
        if timeout and time.monotonic() - t0 > timeout:
            timed_out = True
            break
        if cancel is not None and cancel.is_set():
            cancelled = True
            break
        for key, _ in sel.select(timeout=0.1):
            chunk = os.read(key.fileobj.fileno(), 65536)
            if not chunk:
                sel.unregister(key.fileobj)
                open_streams -= 1
                continue
            buf = out[key.data]
            room = max_bytes - len(buf)
            if room > 0:
                buf += chunk[:room]
            if len(chunk) > room:
                truncated = True
            if on_output:
                try:
                    on_output(key.data, chunk.decode("utf-8", "replace"))
                except Exception:
                    pass
    if timed_out or cancelled:
        try:
            os.killpg(p.pid, signal.SIGTERM)
            try:
                p.wait(2)
            except subprocess.TimeoutExpired:
                os.killpg(p.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
    rc = p.wait()
    sel.close()
    p.stdout.close()
    p.stderr.close()
    return {"rc": rc if not timed_out else 124, "stdout": out["stdout"].decode("utf-8", "replace"),
            "stderr": out["stderr"].decode("utf-8", "replace"), "truncated": truncated,
            "duration_ms": int((time.monotonic() - t0) * 1000), "timed_out": timed_out,
            "cancelled": cancelled}
