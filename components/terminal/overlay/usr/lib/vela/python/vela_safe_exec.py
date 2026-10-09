"""Vela safe executor — public in-process API (owner: terminal component).

    import sys; sys.path.insert(0, "/usr/lib/vela/python")
    import vela_safe_exec as vse
    info = vse.classify("rm -rf ~/x")      # {"risk": "dangerous", "reasons": [...], "needs_sudo": False, ...}
    pv   = vse.preview("rm -rf ~/x", cwd)  # {"kind", "text", "exact", "items"}
    res  = vse.run(cmd, cwd, timeout=60, confirmed=True, confirmed_danger=False, caller="assistant")

See /usr/share/doc/vela-terminal/EXECUTOR_API.md.
"""
import os
import sys

_LIB = os.environ.get("VELA_TERMINAL_LIB", "/usr/lib/vela/terminal")
if _LIB not in sys.path:
    sys.path.insert(0, _LIB)

from vela_terminal.safe_exec import classify, preview, run, NotConfirmed, RISK  # noqa: E402,F401

__all__ = ["classify", "preview", "run", "NotConfirmed", "RISK"]
