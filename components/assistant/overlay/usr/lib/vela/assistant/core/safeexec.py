"""Shell command execution for the assistant.

Prefers terminal component's executor (/usr/lib/vela/python/vela_safe_exec.py, same rules as the AI terminal);
falls back to this built-in classifier with the same semantics. Every command needs the user's confirmation
anyway; «dangerous» ones get a red card and a second explicit confirmation.
"""
import os
import re
import subprocess

try:
    import vela_safe_exec as _ext  # provided by terminal component (optional)
except Exception:  # pragma: no cover
    _ext = None

DANGEROUS = [
    (r"\brm\s+(-[a-zA-Z]*[rRf][a-zA-Z]*\s+)", "rm_recursive"),
    (r"\bdd\b", "dd"),
    (r"\bmkfs(\.\w+)?\b|\bwipefs\b|\bfdisk\b|\bsfdisk\b|\bparted\b|\bgdisk\b", "disk"),
    (r"\b(sudo|pkexec|su|doas)\b", "root"),
    (r"\bch(mod|own)\s+(-[a-zA-Z]*R[a-zA-Z]*\s+)", "recursive_perms"),
    (r">\s*/dev/(sd|nvme|vd|hd|mmcblk)", "raw_device"),
    (r"\b(shutdown|reboot|poweroff|halt)\b|systemctl\s+(poweroff|reboot|halt)", "power"),
    (r"(curl|wget)\b[^|]*\|\s*(sudo\s+)?(ba|z|da)?sh\b", "pipe_shell"),
    (r":\(\)\s*\{\s*:\|:&\s*\};:", "forkbomb"),
    (r"\bkill(all)?\s+-9\s+(-1|1)\b", "kill_all"),
    (r"\bapt(-get)?\s+(remove|purge|autoremove)\b|\bdpkg\s+(-r|-P|--purge|--remove)\b", "remove_packages"),
    (r"\bgit\s+(push\s+.*--force|reset\s+--hard|clean\s+-[a-z]*f)", "git_destructive"),
    (r"\bmv\s+.*\s+/dev/null\b", "mv_null"),
]
CHANGES = r"\b(rm|mv|cp|mkdir|rmdir|touch|chmod|chown|ln|tee|sed\s+-i|truncate|apt|apt-get|dpkg|flatpak|pip|npm|" \
          r"systemctl|kwriteconfig6|plasma-apply|nmcli|git\s+(commit|push|checkout|merge|rebase)|tar\s+-?x|unzip)\b|>"


def risk_texts(classification):
    """Localized danger reasons: the terminal executor returns sentences (reasons_text); the built-in
    fallback returns short codes translated through i18n."""
    texts = classification.get("reasons_text")
    if texts:
        return list(texts)
    if "reason_objects" in classification:  # executor without reasons_text: reasons are already sentences
        return [str(r) for r in classification.get("reasons", [])]
    from .i18n import T
    return [T("risk_" + r) for r in classification.get("reasons", [])]


def classify(cmd):
    if _ext is not None:
        try:
            return _ext.classify(cmd)
        except Exception:
            pass
    reasons = [name for pat, name in DANGEROUS if re.search(pat, cmd)]
    needs_sudo = "root" in reasons
    if reasons:
        return {"risk": "dangerous", "reasons": reasons, "needs_sudo": needs_sudo}
    if re.search(CHANGES, cmd):
        return {"risk": "changes", "reasons": [], "needs_sudo": False}
    return {"risk": "safe", "reasons": [], "needs_sudo": False}


def preview(cmd, cwd=None):
    """Dry-run description from the terminal executor (what will change), or None."""
    if _ext is not None and hasattr(_ext, "preview"):
        try:
            pv = _ext.preview(cmd, cwd or os.path.expanduser("~"))
            return pv.get("text") if isinstance(pv, dict) else None
        except Exception:
            return None
    return None


def run(cmd, cwd=None, timeout=60, confirmed_danger=False):
    """Only called after the user confirmed in the assistant card (and confirmed the danger, if any)."""
    if _ext is not None:
        try:
            return _ext.run(cmd, cwd=cwd, timeout=timeout, confirmed=True, confirmed_danger=confirmed_danger,
                            caller="assistant")
        except TypeError:
            return _ext.run(cmd, cwd=cwd, timeout=timeout)
        except PermissionError as e:  # vela_safe_exec.NotConfirmed (not confirmed / execution disabled)
            return {"rc": 126, "stdout": "", "stderr": str(e), "truncated": False}
    if classify(cmd)["risk"] == "dangerous" and not confirmed_danger:
        return {"rc": 126, "stdout": "", "stderr": "not confirmed", "truncated": False}
    env = dict(os.environ, LC_ALL=os.environ.get("LC_ALL", ""), TERM="dumb", PAGER="cat", GIT_PAGER="cat")
    try:
        p = subprocess.run(["/bin/sh", "-c", cmd], cwd=cwd or os.path.expanduser("~"), capture_output=True,
                           text=True, timeout=timeout, stdin=subprocess.DEVNULL, env=env)
        rc, out, err = p.returncode, p.stdout, p.stderr
    except subprocess.TimeoutExpired as e:
        rc, out, err = 124, (e.stdout or b"").decode("utf-8", "replace") if isinstance(e.stdout, bytes) else (e.stdout or ""), "timeout"
    limit = 16000
    trunc = len(out) > limit or len(err) > limit
    return {"rc": rc, "stdout": out[-limit:], "stderr": err[-limit:], "truncated": trunc}


def using_terminal_executor():
    return _ext is not None
