"""Context the assistant may use: active window, selected text, clipboard, current Space.

Active window + current Space on KDE Plasma 6 come from a one-shot KWin script that calls back
org.vela.Assistant.ReportContext over D-Bus (works on Wayland and X11). On other X11 desktops xprop is used.
Selected text/clipboard are read only when the permissions in ai.json allow it.
"""
import os
import re

from . import sysutil

MAX_SELECTION = 4000

KWIN_SCRIPT = r"""
var w = workspace.activeWindow;
var d = workspace.currentDesktop;
callDBus("org.vela.Assistant", "/Assistant", "org.vela.Assistant", "ReportContext",
         w ? String(w.caption) : "", w ? String(w.resourceClass) : "",
         w ? String(w.desktopFileName || "") : "", d ? String(d.name) : "", "%TOKEN%");
"""


def kwin_script_text(token):
    return KWIN_SCRIPT.replace("%TOKEN%", re.sub(r"[^A-Za-z0-9_-]", "", token))


def x11_active_window():
    """Fallback for X11 sessions without KWin scripting: returns dict or None."""
    if not os.environ.get("DISPLAY") or not sysutil.which("xprop"):
        return None
    rc, out, _ = sysutil.read(["xprop", "-root", "_NET_ACTIVE_WINDOW"], 2)
    m = re.search(r"window id # (0x[0-9a-fA-F]+)", out or "")
    if rc != 0 or not m or int(m.group(1), 16) == 0:
        return None
    rc, out, _ = sysutil.read(["xprop", "-id", m.group(1), "_NET_WM_NAME", "WM_CLASS"], 2)
    title = re.search(r'_NET_WM_NAME\(UTF8_STRING\) = "(.*)"', out or "")
    cls_line = re.search(r"WM_CLASS\(STRING\) = (.*)", out or "")
    cls = re.findall(r'"([^"]*)"', cls_line.group(1)) if cls_line else []
    return {"title": title.group(1) if title else "", "app": cls[-1] if cls else "", "desktop_file": "",
            "space": ""}


def _clean(text):
    if not text:
        return ""
    text = text.replace("\x00", "").strip()
    if len(text) > MAX_SELECTION:
        text = text[:MAX_SELECTION] + " […]"
    return text


def selected_text():
    """Primary selection (text highlighted by the user in any app)."""
    if sysutil.is_wayland() and sysutil.which("wl-paste"):
        rc, out, _ = sysutil.read(["wl-paste", "--primary", "--no-newline", "--type", "text/plain"], 2)
        if rc == 0:
            return _clean(out)
    if os.environ.get("DISPLAY") and sysutil.which("xclip"):
        rc, out, _ = sysutil.read(["xclip", "-o", "-selection", "primary"], 2)
        if rc == 0:
            return _clean(out)
    return ""


def clipboard_text():
    if sysutil.is_wayland() and sysutil.which("wl-paste"):
        rc, out, _ = sysutil.read(["wl-paste", "--no-newline", "--type", "text/plain"], 2)
        if rc == 0:
            return _clean(out)
    if os.environ.get("DISPLAY") and sysutil.which("xclip"):
        rc, out, _ = sysutil.read(["xclip", "-o", "-selection", "clipboard"], 2)
        if rc == 0:
            return _clean(out)
    return ""


TEXT_EXT = {".txt", ".md", ".markdown", ".py", ".sh", ".js", ".ts", ".json", ".yaml", ".yml", ".toml", ".ini",
            ".conf", ".cfg", ".csv", ".tsv", ".html", ".htm", ".css", ".xml", ".c", ".h", ".cpp", ".hpp", ".rs",
            ".go", ".java", ".kt", ".rb", ".php", ".sql", ".log", ".tex", ".rst", ".qml", ".desktop", ".srt"}
MAX_FILE_CHARS = 12000


def describe_file(path):
    """Return dict(name, path, size, kind, text|None) for a file the user explicitly attached."""
    import mimetypes
    st = os.stat(path)
    name = os.path.basename(path.rstrip("/")) or path
    info = {"name": name, "path": path, "size": st.st_size, "is_dir": os.path.isdir(path), "text": None,
            "mime": mimetypes.guess_type(path)[0] or ""}
    if info["is_dir"]:
        try:
            entries = sorted(os.listdir(path))
        except OSError:
            entries = []
        info["text"] = "\n".join(entries[:200])
        info["kind"] = "dir"
        return info
    ext = os.path.splitext(name)[1].lower()
    if ext in TEXT_EXT or info["mime"].startswith("text/"):
        try:
            with open(path, encoding="utf-8", errors="replace") as f:
                t = f.read(MAX_FILE_CHARS + 1)
            info["text"] = t[:MAX_FILE_CHARS] + ("\n[…]" if len(t) > MAX_FILE_CHARS else "")
            info["kind"] = "text"
        except OSError:
            info["kind"] = "other"
    elif ext == ".pdf" and sysutil.which("pdftotext"):
        rc, out, _ = sysutil.read(["pdftotext", "-l", "5", "-layout", path, "-"], 10)
        info["text"] = out[:MAX_FILE_CHARS] if rc == 0 else None
        info["kind"] = "pdf"
    elif info["mime"].startswith("image/"):
        info["kind"] = "image"
    else:
        info["kind"] = "other"
    return info
