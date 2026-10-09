"""Small helpers to run desktop commands safely (argument lists, timeouts, no shell)."""
import os
import shutil
import subprocess

DRYRUN = os.environ.get("VELA_ASSISTANT_DRYRUN") == "1"
_dry_log = []


def which(name):
    return shutil.which(name)


def run(args, timeout=10, input_text=None, check=False, env=None):
    """Run a command; returns (rc, stdout, stderr). Never raises for missing binaries (rc=127)."""
    if DRYRUN and not getattr(run, "_allow", False):
        _dry_log.append(list(args))
        return 0, "", ""
    try:
        p = subprocess.run(args, capture_output=True, text=True, timeout=timeout, input=input_text,
                           env=env)
        return p.returncode, p.stdout, p.stderr
    except FileNotFoundError:
        return 127, "", f"{args[0]}: not found"
    except subprocess.TimeoutExpired:
        return 124, "", "timeout"


def read(args, timeout=5):
    """Like run() but always executes, even in dry-run mode (for read-only queries)."""
    try:
        p = subprocess.run(args, capture_output=True, text=True, timeout=timeout)
        return p.returncode, p.stdout, p.stderr
    except FileNotFoundError:
        return 127, "", f"{args[0]}: not found"
    except subprocess.TimeoutExpired:
        return 124, "", "timeout"


def spawn(args):
    """Start a GUI program detached from the assistant."""
    if DRYRUN:
        _dry_log.append(list(args))
        return True
    try:
        subprocess.Popen(args, stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                         start_new_session=True)
        return True
    except OSError:
        return False


def dry_log():
    return list(_dry_log)


def is_wayland():
    return bool(os.environ.get("WAYLAND_DISPLAY")) or os.environ.get("XDG_SESSION_TYPE") == "wayland"


def is_kde():
    return "KDE" in os.environ.get("XDG_CURRENT_DESKTOP", "").upper()
