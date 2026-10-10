"""Actions the assistant can take. Each tool has: a JSON schema for the model, a kind
("read": runs at once, "open": opens something, runs at once, "change": needs a preview card + confirmation),
preview(args) -> card, run(args) -> Result (with optional undo data), undo(data) -> Result.
"""
import datetime
import json
import os
import re
import shutil
import time
import urllib.parse

from . import config  # noqa: F401  (adds /usr/lib/vela/python to sys.path for safeexec)
from . import apps as appsmod
from . import i18n, safeexec, sysutil, workflows

T = i18n.T
HOME = os.path.expanduser("~")


class ToolError(Exception):
    """User-facing failure (message already translated)."""


def result(ok=True, message="", data=None, undo=None):
    return {"ok": ok, "message": message, "data": data or {}, "undo": undo}


# =========================================================================================== settings pages
SETTINGS_PAGES = {
    "wifi": ("kcm_networkmanagement", "page_wifi"), "rete": ("kcm_networkmanagement", "page_network"),
    "bluetooth": ("kcm_bluetooth", "page_bluetooth"), "audio": ("kcm_pulseaudio", "page_audio"),
    "schermo": ("kcm_kscreen", "page_display"), "aspetto": ("kcm_lookandfeel", "page_appearance"),
    "colori": ("kcm_colors", "page_colors"), "sfondo": ("kcm_wallpaper", "page_wallpaper"),
    "tastiera": ("kcm_keyboard", "page_keyboard"), "mouse": ("kcm_mouse", "page_mouse"),
    "touchpad": ("kcm_touchpad", "page_touchpad"), "notifiche": ("kcm_notifications", "page_notifications"),
    "energia": ("kcm_powerdevilprofilesconfig", "page_power"), "utenti": ("kcm_users", "page_users"),
    "data_ora": ("kcm_clock", "page_clock"), "lingua": ("kcm_regionandlang", "page_language"),
    "stampanti": ("kcm_printer_manager", "page_printers"), "app_predefinite": ("kcm_componentchooser",
                                                                                "page_default_apps"),
    "accessibilita": ("kcm_access", "page_accessibility"), "scorciatoie": ("kcm_keys", "page_shortcuts"),
    "info": ("kcm_about-distro", "page_about"), "ia": ("kcm_vela_ai", "page_ai"),
    "privacy": ("kcm_vela_privacy", "page_privacy"), "avvio": ("kcm_autostart", "page_autostart"),
    "font": ("kcm_fonts", "page_fonts"), "icone": ("kcm_icons", "page_icons"),
    "aggiornamenti": ("kcm_updates", "page_updates"),
}

# =========================================================================================== helpers
def _expand(p):
    p = (p or "").strip().strip("\"'")
    if p.startswith("file://"):
        p = urllib.parse.unquote(p[7:])
    p = os.path.expanduser(p)
    if not os.path.isabs(p):
        p = os.path.join(HOME, p)
    return os.path.normpath(p)


def _in_home(p):
    real = os.path.realpath(p)
    return real == os.path.realpath(HOME) or real.startswith(os.path.realpath(HOME) + os.sep)


def _protected(p):
    rel = os.path.relpath(os.path.realpath(p), os.path.realpath(HOME))
    top = rel.split(os.sep)[0]
    return rel == "." or top in (".ssh", ".gnupg", ".config", ".local", ".cache", ".mozilla", ".pki")


def _short(p):
    return "~" + p[len(HOME):] if p.startswith(HOME) else p


def _human_size(n):
    for unit in ("B", "KB", "MB", "GB", "TB"):
        if n < 1024 or unit == "TB":
            return (f"{n:.0f} {unit}" if unit == "B" else f"{n:.1f} {unit}").replace(".", ",")
        n /= 1024.0


# =========================================================================================== tool classes
class Tool:
    name = ""
    kind = "change"          # read | open | change
    permission = None        # key in ai.json permissions checked with vac.may()
    icon = "dialog-information"
    description = ""
    parameters = {"type": "object", "properties": {}, "required": []}

    def schema(self):
        return {"type": "function", "function": {"name": self.name, "description": self.description,
                                                 "parameters": self.parameters}}

    def preview(self, args):
        return {"title": self.name, "detail": [], "danger": False, "confirm": T("btn_confirm")}

    def run(self, args):
        raise NotImplementedError

    def undo(self, data):
        raise ToolError(T("undo_unavailable"))


# ---------------------------------------------------------------------------------------- open
class OpenApp(Tool):
    name, kind, icon = "open_app", "open", "system-run"
    description = "Open an installed application by name (e.g. 'Firefox', 'File', 'Calcolatrice', 'Terminale')."
    parameters = {"type": "object", "properties": {"name": {"type": "string", "description": "App name"}},
                  "required": ["name"]}
    _cache = None

    def find(self, name):
        if OpenApp._cache is None or time.time() - OpenApp._cache[0] > 60:
            OpenApp._cache = (time.time(), appsmod.load_apps(i18n.lang()))
        return appsmod.find_app(name, OpenApp._cache[1])

    def preview(self, args):
        a = self.find(args.get("name", ""))
        return {"title": T("open_app_title", name=a["name"] if a else args.get("name", "")), "detail": [],
                "danger": False, "confirm": T("btn_open")}

    def run(self, args):
        a = self.find(args.get("name", ""))
        if not a:
            raise ToolError(T("app_not_found", name=args.get("name", "")))
        did = a["id"]
        for cmd in (["kstart", "--application", did.removesuffix(".desktop")], ["gtk-launch", did],
                    ["kioclient", "exec", a["path"]], ["gio", "launch", a["path"]]):
            if sysutil.which(cmd[0]):
                sysutil.spawn(cmd)
                return result(True, T("opened", name=a["name"]), {"app": a["name"], "icon": a["icon"]})
        raise ToolError(T("app_cannot_start", name=a["name"]))


class OpenSettings(Tool):
    name, kind, icon = "open_settings", "open", "preferences-system"
    description = ("Open a page of System Settings. page is one of: " + ", ".join(SETTINGS_PAGES) + ".")
    parameters = {"type": "object", "properties": {"page": {"type": "string", "enum": list(SETTINGS_PAGES)}},
                  "required": ["page"]}

    def preview(self, args):
        page = args.get("page", "")
        label = T(SETTINGS_PAGES[page][1]) if page in SETTINGS_PAGES else T("settings")
        return {"title": T("open_settings_title", page=label), "detail": [], "danger": False,
                "confirm": T("btn_open")}

    def run(self, args):
        page = args.get("page", "")
        if page not in SETTINGS_PAGES:
            raise ToolError(T("settings_page_unknown", page=page))
        kcm, label = SETTINGS_PAGES[page]
        for cmd in (["systemsettings", kcm], ["kcmshell6", kcm]):
            if sysutil.which(cmd[0]):
                sysutil.spawn(cmd)
                return result(True, T("opened_settings", page=T(label)))
        raise ToolError(T("settings_missing"))


class OpenPath(Tool):
    name, kind, icon = "open_path", "open", "document-open"
    description = "Open a file with its default app, or a folder in the file manager."
    parameters = {"type": "object", "properties": {"path": {"type": "string"},
                                                   "reveal": {"type": "boolean",
                                                              "description": "Show the file in its folder"}},
                  "required": ["path"]}

    def preview(self, args):
        return {"title": T("open_app_title", name=os.path.basename(_expand(args.get("path", "")))), "detail": [],
                "danger": False, "confirm": T("btn_open")}

    def run(self, args):
        p = _expand(args.get("path", ""))
        if not os.path.exists(p):
            raise ToolError(T("path_missing", path=_short(p)))
        if args.get("reveal") and sysutil.which("dolphin"):
            sysutil.spawn(["dolphin", "--select", p])
        else:
            sysutil.spawn(["xdg-open", p])
        return result(True, T("opened", name=_short(p)))


# ---------------------------------------------------------------------------------------- read
class FindFiles(Tool):
    name, kind, icon = "find_files", "read", "edit-find"
    description = ("Search files in the user's home by name (and content when the desktop index allows it). "
                   "Returns paths. Use for 'find', 'where is', 'trova', 'cerca il file'.")
    parameters = {"type": "object", "properties": {
        "query": {"type": "string", "description": "words in the file name or content"},
        "kind": {"type": "string", "enum": ["any", "image", "document", "pdf", "audio", "video", "folder"]}},
        "required": ["query"]}
    EXT = {"image": {".png", ".jpg", ".jpeg", ".gif", ".webp", ".svg", ".heic", ".bmp"},
           "document": {".pdf", ".odt", ".doc", ".docx", ".txt", ".md", ".ods", ".xls", ".xlsx", ".odp", ".ppt",
                        ".pptx", ".rtf"},
           "pdf": {".pdf"}, "audio": {".mp3", ".flac", ".ogg", ".wav", ".m4a", ".opus"},
           "video": {".mp4", ".mkv", ".webm", ".avi", ".mov"}}

    def _ok(self, path, kind):
        if kind in (None, "", "any"):
            return True
        if kind == "folder":
            return os.path.isdir(path)
        return os.path.splitext(path)[1].lower() in self.EXT.get(kind, set())

    def search(self, query, kind="any", limit=20):
        words = [w for w in re.split(r"\s+", (query or "").lower()) if w]
        if not words:
            return [], "none"
        found, engine = [], "walk"
        for tool in ("baloosearch6", "baloosearch"):
            if sysutil.which(tool):
                rc, out, _ = sysutil.read([tool, "-l", "200", " ".join(words)], 6)
                if rc == 0 and out.strip():
                    found = [ln.strip() for ln in out.splitlines() if ln.strip().startswith("/")]
                    engine = "baloo"
                break
        if not found and (sysutil.which("plocate") or sysutil.which("locate")):
            rc, out, _ = sysutil.read([sysutil.which("plocate") or "locate", "-i", "-l", "400", "--basename",
                                       words[0]], 6)
            if rc == 0:
                found = [ln for ln in out.splitlines() if ln.startswith(HOME)
                         and all(w in os.path.basename(ln).lower() for w in words)]
                engine = "locate"
        if not found:
            found, engine = self._walk(words), "walk"
        res = []
        for p in found:
            if "/." in p[len(HOME):] or not os.path.exists(p) or not self._ok(p, kind):
                continue
            res.append(p)
            if len(res) >= limit:
                break
        res.sort(key=lambda p: -os.path.getmtime(p) if os.path.exists(p) else 0)
        return res, engine

    def _walk(self, words, budget=2.5, max_hits=200):
        hits, t0 = [], time.time()
        for root, dirs, files in os.walk(HOME):
            dirs[:] = [d for d in dirs if not d.startswith(".") and d not in ("node_modules", "__pycache__")]
            for n in dirs + files:
                if all(w in n.lower() for w in words):
                    hits.append(os.path.join(root, n))
            if len(hits) >= max_hits or time.time() - t0 > budget:
                break
        return hits

    def preview(self, args):
        return {"title": T("find_title", q=args.get("query", "")), "detail": [], "danger": False,
                "confirm": T("btn_open")}

    def run(self, args):
        paths, engine = self.search(args.get("query", ""), args.get("kind", "any"))
        items = []
        for p in paths:
            try:
                st = os.stat(p)
            except OSError:
                continue
            items.append({"path": p, "short": _short(p), "name": os.path.basename(p), "is_dir": os.path.isdir(p),
                          "size": _human_size(st.st_size),
                          "modified": datetime.datetime.fromtimestamp(st.st_mtime).strftime("%d/%m/%Y %H:%M")})
        msg = T("found_n", n=len(items)) if items else T("found_none", q=args.get("query", ""))
        return result(True, msg, {"files": items, "engine": engine})


class SystemStatus(Tool):
    name, kind, icon = "system_status", "read", "computer"
    description = "Get system information: free disk space, memory, battery, network address, uptime, OS version."

    def preview(self, args):
        return {"title": T("status_title"), "detail": [], "danger": False, "confirm": T("btn_open")}

    def run(self, args):
        info = {}
        try:
            mem = {}
            for ln in open("/proc/meminfo"):
                k, v = ln.split(":", 1)
                mem[k] = int(v.split()[0]) * 1024
            info["memory"] = T("mem_fmt", used=_human_size(mem["MemTotal"] - mem["MemAvailable"]),
                               total=_human_size(mem["MemTotal"]))
        except (OSError, KeyError, ValueError):
            pass
        du = shutil.disk_usage(HOME)
        info["disk"] = T("disk_fmt", free=_human_size(du.free), total=_human_size(du.total))
        try:
            up = float(open("/proc/uptime").read().split()[0])
            info["uptime"] = T("uptime_fmt", h=int(up // 3600), m=int(up % 3600 // 60))
        except (OSError, ValueError):
            pass
        try:
            info["os"] = re.search(r'PRETTY_NAME="([^"]+)"', open("/etc/os-release").read()).group(1)
        except (OSError, AttributeError):
            pass
        bats = [d for d in os.listdir("/sys/class/power_supply") if d.startswith("BAT")] \
            if os.path.isdir("/sys/class/power_supply") else []
        if bats:
            try:
                b = "/sys/class/power_supply/" + bats[0]
                info["battery"] = open(b + "/capacity").read().strip() + "% (" + open(b + "/status").read().strip() + ")"
            except OSError:
                pass
        rc, out, _ = sysutil.read(["hostname", "-I"], 2)
        if rc == 0 and out.strip():
            info["ip"] = out.split()[0]
        try:
            info["load"] = open("/proc/loadavg").read().split()[0]
        except OSError:
            pass
        return result(True, "", info)


# ---------------------------------------------------------------------------------------- change: files
class MoveFile(Tool):
    name, icon, permission = "move_file", "edit-move", "files"
    description = ("Move a file or folder inside the user's home. If destination is an existing folder the item "
                   "goes inside it. Never overwrites.")
    parameters = {"type": "object", "properties": {"source": {"type": "string"},
                                                   "destination": {"type": "string"}},
                  "required": ["source", "destination"]}

    def plan(self, args):
        src, dst = _expand(args.get("source")), _expand(args.get("destination"))
        if not os.path.exists(src):
            raise ToolError(T("path_missing", path=_short(src)))
        if os.path.isdir(dst):
            dst = os.path.join(dst, os.path.basename(src))
        for p in (src, dst):
            if not _in_home(p) or _protected(p):
                raise ToolError(T("outside_home", path=_short(p)))
        if os.path.exists(dst):
            raise ToolError(T("dest_exists", path=_short(dst)))
        if dst.startswith(src + os.sep):
            raise ToolError(T("move_into_self"))
        return src, dst

    def preview(self, args):
        src, dst = self.plan(args)
        return {"title": T("move_title", name=os.path.basename(src)),
                "detail": [T("from_to", src=_short(os.path.dirname(src)), dst=_short(os.path.dirname(dst)))]
                + ([T("new_name", name=os.path.basename(dst))] if os.path.basename(dst) != os.path.basename(src)
                   else []),
                "danger": False, "confirm": T("btn_move")}

    def run(self, args):
        src, dst = self.plan(args)
        os.makedirs(os.path.dirname(dst), exist_ok=True)
        shutil.move(src, dst)
        return result(True, T("moved", name=os.path.basename(dst), dst=_short(os.path.dirname(dst))),
                      {"path": dst}, undo={"tool": self.name, "src": src, "dst": dst})

    def undo(self, d):
        if not os.path.exists(d["dst"]) or os.path.exists(d["src"]):
            raise ToolError(T("undo_conflict"))
        os.makedirs(os.path.dirname(d["src"]), exist_ok=True)
        shutil.move(d["dst"], d["src"])
        return result(True, T("undone"))


class RenameFile(MoveFile):
    name, icon = "rename_file", "edit-rename"
    description = "Rename a file or folder (same folder, new name). Never overwrites."
    parameters = {"type": "object", "properties": {"path": {"type": "string"}, "new_name": {"type": "string"}},
                  "required": ["path", "new_name"]}

    def plan(self, args):
        new = (args.get("new_name") or "").strip()
        if not new or "/" in new or new in (".", ".."):
            raise ToolError(T("bad_name"))
        src = _expand(args.get("path"))
        return MoveFile.plan(self, {"source": src, "destination": os.path.join(os.path.dirname(src), new)})

    def preview(self, args):
        src, dst = self.plan(args)
        return {"title": T("rename_title", name=os.path.basename(src)),
                "detail": [T("new_name", name=os.path.basename(dst)), _short(os.path.dirname(src))],
                "danger": False, "confirm": T("btn_rename")}


class CreateFolder(Tool):
    name, icon, permission = "create_folder", "folder-new", "files"
    description = "Create a new folder inside the user's home."
    parameters = {"type": "object", "properties": {"path": {"type": "string"}}, "required": ["path"]}

    def plan(self, args):
        p = _expand(args.get("path"))
        if not _in_home(p) or _protected(p):
            raise ToolError(T("outside_home", path=_short(p)))
        if os.path.exists(p):
            raise ToolError(T("dest_exists", path=_short(p)))
        return p

    def preview(self, args):
        p = self.plan(args)
        return {"title": T("mkdir_title", name=os.path.basename(p)), "detail": [_short(os.path.dirname(p))],
                "danger": False, "confirm": T("btn_create")}

    def run(self, args):
        p = self.plan(args)
        os.makedirs(p)
        return result(True, T("created", name=_short(p)), {"path": p}, undo={"tool": self.name, "path": p})

    def undo(self, d):
        try:
            os.rmdir(d["path"])
        except OSError:
            raise ToolError(T("undo_not_empty"))
        return result(True, T("undone"))


class TrashFile(Tool):
    """Freedesktop.org home trash (the same Cestino Dolphin shows), so undo can restore it."""
    name, icon, permission = "trash_file", "user-trash", "files"
    description = "Move a file or folder to the Trash (Cestino). It can be restored."
    parameters = {"type": "object", "properties": {"path": {"type": "string"}}, "required": ["path"]}

    def plan(self, args):
        p = _expand(args.get("path"))
        if not os.path.lexists(p):
            raise ToolError(T("path_missing", path=_short(p)))
        if not _in_home(p) or _protected(p):
            raise ToolError(T("outside_home", path=_short(p)))
        return p

    def preview(self, args):
        p = self.plan(args)
        return {"title": T("trash_title", name=os.path.basename(p)), "detail": [_short(os.path.dirname(p))],
                "danger": False, "confirm": T("btn_trash")}

    @staticmethod
    def trash_dir():
        return os.path.join(os.environ.get("XDG_DATA_HOME") or os.path.join(HOME, ".local/share"), "Trash")

    def run(self, args):
        p = self.plan(args)
        td = self.trash_dir()
        os.makedirs(os.path.join(td, "files"), exist_ok=True)
        os.makedirs(os.path.join(td, "info"), exist_ok=True)
        base = os.path.basename(p)
        name, i = base, 1
        while os.path.lexists(os.path.join(td, "files", name)) or os.path.exists(
                os.path.join(td, "info", name + ".trashinfo")):
            i += 1
            stem, ext = os.path.splitext(base)
            name = f"{stem} ({i}){ext}"
        info = os.path.join(td, "info", name + ".trashinfo")
        with open(info, "w", encoding="utf-8") as f:
            f.write("[Trash Info]\nPath=" + urllib.parse.quote(p) + "\nDeletionDate="
                    + datetime.datetime.now().strftime("%Y-%m-%dT%H:%M:%S") + "\n")
        try:
            shutil.move(p, os.path.join(td, "files", name))
        except OSError:
            os.unlink(info)
            raise
        return result(True, T("trashed", name=base), {},
                      undo={"tool": self.name, "orig": p, "trashed": os.path.join(td, "files", name), "info": info})

    def undo(self, d):
        if not os.path.lexists(d["trashed"]) or os.path.lexists(d["orig"]):
            raise ToolError(T("undo_conflict"))
        os.makedirs(os.path.dirname(d["orig"]), exist_ok=True)
        shutil.move(d["trashed"], d["orig"])
        try:
            os.unlink(d["info"])
        except OSError:
            pass
        return result(True, T("restored", name=os.path.basename(d["orig"])))


# ---------------------------------------------------------------------------------------- change: settings
def _kread(file, group, key):
    for tool in ("kreadconfig6", "kreadconfig5"):
        if sysutil.which(tool):
            rc, out, _ = sysutil.read([tool, "--file", file, "--group", group, "--key", key], 3)
            return out.strip() if rc == 0 else ""
    return ""


def _schemes():
    if not sysutil.which("plasma-apply-colorscheme"):
        return []
    rc, out, _ = sysutil.read(["plasma-apply-colorscheme", "--list-schemes"], 5)
    return [re.sub(r"\s*\(.*\)$", "", ln.strip(" *").strip()) for ln in out.splitlines()[1:] if ln.strip()]


class SetDarkMode(Tool):
    """Uses theme's `vela-theme` CLI (keeps accent, icons, window decorations in sync); falls back to
    plasma-apply-colorscheme with the Vela schemes."""
    name, icon, permission = "set_dark_mode", "weather-clear-night", "system_changes"
    description = "Switch the system appearance to dark (enabled=true) or light (enabled=false)."
    parameters = {"type": "object", "properties": {"enabled": {"type": "boolean"}}, "required": ["enabled"]}
    DARK = ["VelaScuro", "VelaDark", "BreezeDark"]
    LIGHT = ["VelaChiaro", "VelaLight", "BreezeLight", "BreezeClassic"]

    def _pick(self, dark):
        avail = _schemes()
        for s in (self.DARK if dark else self.LIGHT):
            if s in avail:
                return s
        return None

    @staticmethod
    def _theme_mode():
        rc, out, _ = sysutil.read(["vela-theme", "status"], 5)
        try:
            return json.loads(out).get("mode") if rc == 0 else None
        except ValueError:
            return None

    def preview(self, args):
        dark = bool(args.get("enabled", True))
        return {"title": T("dark_on_title") if dark else T("dark_off_title"),
                "detail": [T("dark_detail")], "danger": False, "confirm": T("btn_apply")}

    def run(self, args):
        dark = bool(args.get("enabled", True))
        if sysutil.which("vela-theme"):
            prev = self._theme_mode()
            rc, _, _ = sysutil.run(["vela-theme", "apply", "--mode", "dark" if dark else "light"], 30)
            if rc != 0:
                raise ToolError(T("apply_failed"))
            want = "dark" if dark else "light"
            return result(True, T("dark_on_done") if dark else T("dark_off_done"), {"mode": want},
                          undo={"tool": self.name, "mode": prev} if prev and prev != want else None)
        scheme = self._pick(dark)
        if not scheme:
            raise ToolError(T("dark_unsupported"))
        prev = _kread("kdeglobals", "General", "ColorScheme")
        rc, _, err = sysutil.run(["plasma-apply-colorscheme", scheme], 15)
        if rc != 0:
            raise ToolError(T("apply_failed"))
        return result(True, T("dark_on_done") if dark else T("dark_off_done"), {"scheme": scheme},
                      undo={"tool": self.name, "scheme": prev} if prev and prev != scheme else None)

    def undo(self, d):
        if d.get("mode"):
            rc, _, _ = sysutil.run(["vela-theme", "apply", "--mode", d["mode"]], 30)
        else:
            rc, _, _ = sysutil.run(["plasma-apply-colorscheme", d["scheme"]], 15)
        if rc != 0:
            raise ToolError(T("apply_failed"))
        return result(True, T("undone"))


def _get_volume():
    if sysutil.which("wpctl"):
        rc, out, _ = sysutil.read(["wpctl", "get-volume", "@DEFAULT_AUDIO_SINK@"], 3)
        m = re.search(r"Volume:\s*([\d.]+)", out or "")
        if rc == 0 and m:
            return round(float(m.group(1)) * 100), "MUTED" in out
    if sysutil.which("pactl"):
        rc, out, _ = sysutil.read(["pactl", "get-sink-volume", "@DEFAULT_SINK@"], 3)
        m = re.search(r"(\d+)%", out or "")
        rc2, out2, _ = sysutil.read(["pactl", "get-sink-mute", "@DEFAULT_SINK@"], 3)
        if rc == 0 and m:
            return int(m.group(1)), "yes" in (out2 or "").lower() or "sì" in (out2 or "").lower()
    return None, None


def _set_volume(pct=None, mute=None):
    if sysutil.which("wpctl"):
        if pct is not None:
            rc, _, _ = sysutil.run(["wpctl", "set-volume", "-l", "1.0", "@DEFAULT_AUDIO_SINK@", f"{pct / 100:.2f}"])
            if rc != 0:
                return False
        if mute is not None:
            rc, _, _ = sysutil.run(["wpctl", "set-mute", "@DEFAULT_AUDIO_SINK@", "1" if mute else "0"])
            if rc != 0:
                return False
        return True
    if sysutil.which("pactl"):
        if pct is not None:
            sysutil.run(["pactl", "set-sink-volume", "@DEFAULT_SINK@", f"{pct}%"])
        if mute is not None:
            sysutil.run(["pactl", "set-sink-mute", "@DEFAULT_SINK@", "1" if mute else "0"])
        return True
    return False


class SetVolume(Tool):
    name, icon, permission = "set_volume", "audio-volume-high", "system_changes"
    description = ("Change the output volume. Give percent (0-100) for an absolute level, or change (e.g. +10/-10) "
                   "for relative, and/or mute true/false.")
    parameters = {"type": "object", "properties": {"percent": {"type": "integer", "minimum": 0, "maximum": 100},
                                                   "change": {"type": "integer"},
                                                   "mute": {"type": "boolean"}}}

    def _target(self, args):
        cur, muted = _get_volume()
        if cur is None:
            raise ToolError(T("no_audio"))
        pct = args.get("percent")
        if pct is None and args.get("change") is not None:
            pct = cur + int(args["change"])
        if pct is not None:
            pct = max(0, min(100, int(pct)))
        return cur, muted, pct, args.get("mute")

    def preview(self, args):
        cur, muted, pct, mute = self._target(args)
        if mute is True:
            title = T("mute_title")
        elif mute is False and pct is None:
            title = T("unmute_title")
        else:
            title = T("volume_title", pct=pct if pct is not None else cur)
        return {"title": title, "detail": [T("volume_now", pct=cur) + (" · " + T("muted") if muted else "")],
                "danger": False, "confirm": T("btn_apply")}

    def run(self, args):
        cur, muted, pct, mute = self._target(args)
        if mute is None and pct is not None and muted and pct > 0:
            mute = False
        if not _set_volume(pct, mute):
            raise ToolError(T("apply_failed"))
        msg = T("muted_done") if mute else T("volume_done", pct=pct if pct is not None else cur)
        return result(True, msg, {}, undo={"tool": self.name, "pct": cur, "mute": muted})

    def undo(self, d):
        _set_volume(d["pct"], d["mute"])
        return result(True, T("undone"))


class SetBrightness(Tool):
    name, icon, permission = "set_brightness", "video-display-brightness", "system_changes"
    description = "Set the screen brightness in percent (0-100)."
    parameters = {"type": "object", "properties": {"percent": {"type": "integer", "minimum": 1, "maximum": 100},
                                                   "change": {"type": "integer"}}}
    BUS = ["busctl", "--user"]
    SVC, PATH = "org.kde.ScreenBrightness", "/org/kde/ScreenBrightness"

    def _displays(self):
        if not sysutil.which("busctl"):
            return []
        rc, out, _ = sysutil.read(self.BUS + ["get-property", self.SVC, self.PATH, self.SVC,
                                              "DisplaysDBusNames"], 3)
        return re.findall(r'"([^"]+)"', out) if rc == 0 else []

    def _get(self, disp):
        vals = []
        for prop in ("Brightness", "MaxBrightness"):
            rc, out, _ = sysutil.read(self.BUS + ["get-property", self.SVC, f"{self.PATH}/{disp}",
                                                  self.SVC + ".Display", prop], 3)
            m = re.search(r"(\d+)", out or "")
            if rc != 0 or not m:
                return None, None
            vals.append(int(m.group(1)))
        return vals

    def _state(self):
        disps = self._displays()
        if disps:
            cur, mx = self._get(disps[0])
            if cur is not None and mx:
                return "kde", disps[0], round(cur * 100 / mx), mx
        if sysutil.which("brightnessctl"):
            rc, out, _ = sysutil.read(["brightnessctl", "-m"], 3)
            m = re.search(r",(\d+)%,", out or "")
            if rc == 0 and m:
                return "ctl", None, int(m.group(1)), 100
        raise ToolError(T("no_brightness"))

    def _set(self, backend, disp, mx, pct):
        if backend == "kde":
            rc, _, _ = sysutil.run(self.BUS + ["call", self.SVC, f"{self.PATH}/{disp}", self.SVC + ".Display",
                                               "SetBrightness", "iu", str(max(1, round(pct * mx / 100))), "0"])
        else:
            rc, _, _ = sysutil.run(["brightnessctl", "set", f"{pct}%"])
        if rc != 0:
            raise ToolError(T("apply_failed"))

    def _target(self, args):
        backend, disp, cur, mx = self._state()
        pct = args.get("percent")
        if pct is None:
            pct = cur + int(args.get("change") or 0)
        return backend, disp, cur, mx, max(1, min(100, int(pct)))

    def preview(self, args):
        backend, disp, cur, mx, pct = self._target(args)
        return {"title": T("brightness_title", pct=pct), "detail": [T("brightness_now", pct=cur)],
                "danger": False, "confirm": T("btn_apply")}

    def run(self, args):
        backend, disp, cur, mx, pct = self._target(args)
        self._set(backend, disp, mx, pct)
        return result(True, T("brightness_done", pct=pct), {}, undo={"tool": self.name, "pct": cur})

    def undo(self, d):
        backend, disp, cur, mx = self._state()
        self._set(backend, disp, mx, d["pct"])
        return result(True, T("undone"))


class SetWifi(Tool):
    name, icon, permission = "set_wifi", "network-wireless", "network_settings"
    description = "Turn Wi-Fi on (enabled=true) or off (enabled=false)."
    parameters = {"type": "object", "properties": {"enabled": {"type": "boolean"}}, "required": ["enabled"]}

    def _state(self):
        if not sysutil.which("nmcli"):
            raise ToolError(T("no_nm"))
        rc, out, _ = sysutil.read(["nmcli", "radio", "wifi"], 3)
        if rc != 0:
            raise ToolError(T("no_nm"))
        rc2, devs, _ = sysutil.read(["nmcli", "-t", "-f", "TYPE", "device"], 3)
        return out.strip() == "enabled", "wifi" in (devs or "").split()

    def preview(self, args):
        on, has_dev = self._state()
        en = bool(args.get("enabled", True))
        detail = [T("wifi_now_on") if on else T("wifi_now_off")]
        if not has_dev:
            detail.append(T("wifi_no_device"))
        return {"title": T("wifi_on_title") if en else T("wifi_off_title"), "detail": detail,
                "danger": not en, "confirm": T("btn_turn_on") if en else T("btn_turn_off")}

    def run(self, args):
        on, _ = self._state()
        en = bool(args.get("enabled", True))
        rc, _, err = sysutil.run(["nmcli", "radio", "wifi", "on" if en else "off"], 10)
        if rc != 0:
            raise ToolError(T("apply_failed"))
        return result(True, T("wifi_on_done") if en else T("wifi_off_done"), {},
                      undo={"tool": self.name, "on": on} if on != en else None)

    def undo(self, d):
        sysutil.run(["nmcli", "radio", "wifi", "on" if d["on"] else "off"], 10)
        return result(True, T("undone"))


# ---------------------------------------------------------------------------------------- change: reminders
def parse_when(when, now=None):
    """Accept ISO datetime, 'HH:MM', or minutes from now (int / '10m' / '2h'). Returns a datetime."""
    now = now or datetime.datetime.now()
    if isinstance(when, (int, float)):
        return now + datetime.timedelta(minutes=float(when))
    w = str(when or "").strip().lower()
    m = re.fullmatch(r"\+?(\d+(?:[.,]\d+)?)\s*(m|min|minuti|minutes?|h|ore|ora|hours?|s|sec|secondi)?", w)
    if m:
        n = float(m.group(1).replace(",", "."))
        unit = (m.group(2) or "m")[0]
        return now + datetime.timedelta(hours=n) if unit in "ho" else \
            now + datetime.timedelta(seconds=n) if unit == "s" else now + datetime.timedelta(minutes=n)
    m = re.fullmatch(r"(\d{1,2})[:.](\d{2})", w)
    if m:
        t = now.replace(hour=int(m.group(1)), minute=int(m.group(2)), second=0, microsecond=0)
        return t if t > now else t + datetime.timedelta(days=1)
    try:
        t = datetime.datetime.fromisoformat(str(when).replace("Z", "+00:00"))
        if t.tzinfo:
            t = t.astimezone().replace(tzinfo=None)
        return t
    except ValueError:
        raise ToolError(T("bad_time", when=when))


class CreateReminder(Tool):
    name, icon = "create_reminder", "appointment-new"
    description = ("Create a reminder: a desktop notification at a given time. when = minutes from now (e.g. 10), "
                   "'HH:MM', or ISO datetime.")
    parameters = {"type": "object", "properties": {"message": {"type": "string"},
                                                   "when": {"type": "string"}},
                  "required": ["message", "when"]}

    def preview(self, args):
        t = parse_when(args.get("when"))
        if t < datetime.datetime.now() - datetime.timedelta(seconds=5):
            raise ToolError(T("time_past"))
        return {"title": T("reminder_title"), "detail": [args.get("message", ""), i18n.fmt_when(t)],
                "danger": False, "confirm": T("btn_create")}

    def run(self, args):
        t = parse_when(args.get("when"))
        msg = (args.get("message") or "").strip()[:200] or T("reminder_title")
        unit = "vela-reminder-" + datetime.datetime.now().strftime("%Y%m%d%H%M%S%f")
        secs = max(1, int((t - datetime.datetime.now()).total_seconds()))
        if not sysutil.which("systemd-run"):
            raise ToolError(T("reminder_unsupported"))
        cmd = ["systemd-run", "--user", "--collect", "--unit", unit, "--description", "Vela: " + msg[:60],
               f"--on-active={secs}s", "--timer-property=AccuracySec=1s",
               "notify-send", "--app-name=Vela", "--icon=vela-assistant", "--urgency=critical",
               T("reminder_title"), msg]
        rc, _, err = sysutil.run(cmd, 10)
        if rc != 0:
            raise ToolError(T("reminder_failed"))
        return result(True, T("reminder_done", when=i18n.fmt_when(t)), {"unit": unit},
                      undo={"tool": self.name, "unit": unit})

    def undo(self, d):
        sysutil.run(["systemctl", "--user", "stop", d["unit"] + ".timer"], 10)
        return result(True, T("reminder_cancelled"))


# ---------------------------------------------------------------------------------------- change: shell
class RunCommand(Tool):
    name, icon, permission = "run_command", "utilities-terminal", "run_commands"
    description = ("Run a shell command (non-interactive, in the home folder) and return its output. Use only "
                   "when no other tool fits. Always explain what it does in 'explanation'.")
    parameters = {"type": "object", "properties": {"command": {"type": "string"},
                                                   "explanation": {"type": "string",
                                                                   "description": "one short sentence for the user"}},
                  "required": ["command"]}

    def preview(self, args):
        cmd = (args.get("command") or "").strip()
        if not cmd:
            raise ToolError(T("empty_command"))
        c = safeexec.classify(cmd)
        detail = [args.get("explanation") or ""]
        if c["risk"] == "dangerous":
            detail.append(T("cmd_dangerous", why=", ".join(safeexec.risk_texts(c))))
        pv = safeexec.preview(cmd)
        if pv:
            detail.append(pv)
        return {"title": T("run_title"), "code": cmd, "detail": [d for d in detail if d],
                "danger": c["risk"] == "dangerous", "confirm": T("btn_run"),
                "needs_sudo": c.get("needs_sudo", False)}

    def run(self, args):
        cmd = (args.get("command") or "").strip()
        c = safeexec.classify(cmd)
        if c.get("needs_sudo"):
            raise ToolError(T("cmd_sudo"))
        r = safeexec.run(cmd, timeout=60, confirmed_danger=bool(args.get("_confirmed_danger")))
        if r.get("rc") == 126 and not r.get("stdout") and r.get("stderr"):
            raise ToolError(r["stderr"])  # refused by the executor (not confirmed / disabled in Impostazioni)
        out = (r.get("stdout") or "") + (("\n" + r["stderr"]) if r.get("stderr") else "")
        ok = r.get("rc", 1) == 0
        return result(ok, T("cmd_ok") if ok else T("cmd_failed", rc=r.get("rc")),
                      {"output": out.strip()[-6000:], "rc": r.get("rc")})


class OpenHelp(Tool):
    name, kind, icon = "open_help", "open", "help-contents"
    description = "Open «Guida e scorciatoie», the Vela help page with keyboard shortcuts."

    def preview(self, args):
        return {"title": T("open_help_title"), "detail": [], "danger": False, "confirm": T("btn_open")}

    def run(self, args):
        if not sysutil.which("vela-help"):
            raise ToolError(T("help_missing"))
        sysutil.spawn(["vela-help"])
        return result(True, T("opened", name=T("open_help_title")))


class OrganizeFolder(Tool):
    name, icon, permission = "organize_folder", "folder", "files"
    description = ("Prepare a preview to organize one explicit home folder automatically into the fixed groups "
                   "Immagini, Video, Audio, Archivi. No need to ask about categories. "
                   "Leaves documents, unknown files and collisions untouched. Persistent undo. "
                   "Ask only if the folder path is missing.")
    parameters = {"type": "object", "properties": {"folder": {"type": "string"}}, "required": ["folder"]}

    def preview(self, args):
        try:
            rec = workflows.plan(args["folder"])
        except (OSError, workflows.WorkflowError) as e:
            raise ToolError(str(e))
        args["_plan_id"] = rec["id"]
        return {"title": "Organizza " + rec["folder"], "danger": False, "confirm": "Sposta i file",
                "detail": [f"{len(rec['entries'])} spostamenti; {len(rec['skipped'])} elementi lasciati invariati."] +
                          [os.path.basename(e["source"]) + " → " + os.path.relpath(e["destination"], rec["folder"])
                           for e in rec["entries"]]}

    def run(self, args):
        try:
            rec = workflows.execute(args["_plan_id"])
        except (OSError, workflows.WorkflowError) as e:
            rec = workflows.load(args["_plan_id"])
            if rec["state"] in ("running", "interrupted"):
                return result(False, str(e), {"operation": rec["id"]},
                              undo={"tool": self.name, "operation": rec["id"]})
            raise ToolError(str(e))
        return result(True, f"Verificati {len(rec['entries'])} spostamenti.", {"operation": rec["id"]},
                      undo={"tool": self.name, "operation": rec["id"]})

    def undo(self, data):
        try:
            workflows.undo(data["operation"])
        except (OSError, workflows.WorkflowError) as e:
            raise ToolError(str(e))
        return result(True, "Posizioni originali ripristinate e verificate.")


class DiagnoseAudio(Tool):
    name, kind, icon = "diagnose_audio", "read", "audio-card"
    description = "Read actual audio outputs, current default and mute states. Do not claim the sound is audible."

    def preview(self, args):
        return {"title": T("audio_diagnosis"), "detail": [], "danger": False, "confirm": T("btn_open")}

    def run(self, args):
        try:
            state = workflows.audio_state()
        except (ValueError, workflows.WorkflowError) as e:
            raise ToolError(str(e))
        return result(True, "Stato audio letto. Per confermare il suono serve una prova dell'utente.", state)


class SetAudioOutput(Tool):
    name, icon, permission = "set_audio_output", "audio-card", "system_changes"
    description = "Change default audio output to an exact sink name returned by diagnose_audio. Requires confirmation."
    parameters = {"type": "object", "properties": {"name": {"type": "string"}}, "required": ["name"]}

    def preview(self, args):
        try:
            state = workflows.audio_state()
        except (ValueError, workflows.WorkflowError) as e:
            raise ToolError(str(e))
        sink = next((s for s in state["sinks"] if s["name"] == args["name"]), None)
        if not sink:
            raise ToolError("Uscita audio inesistente. Ripeti la diagnosi.")
        args["_previous"] = state["default"]
        return {"title": "Usa " + sink["description"], "detail": ["Uscita attuale: " + state["default"],
                "Le applicazioni già aperte possono mantenere la loro uscita."], "danger": False, "confirm": "Applica"}

    def run(self, args):
        try:
            state = workflows.audio_state()
        except (ValueError, workflows.WorkflowError) as e:
            raise ToolError(str(e))
        if state["default"] != args["_previous"] or args["name"] not in [s["name"] for s in state["sinks"]]:
            raise ToolError("Lo stato audio è cambiato dall'anteprima.")
        rc, _, err = sysutil.run(["pactl", "set-default-sink", args["name"]], 5)
        if rc or workflows.audio_state()["default"] != args["name"]:
            raise ToolError("Cambio uscita non verificato: " + err[:200])
        return result(True, "Uscita predefinita verificata. Prova ora il suono.", {},
                      undo={"tool": self.name, "name": args["_previous"], "expected": args["name"]})

    def undo(self, data):
        if workflows.audio_state()["default"] != data["expected"]:
            raise ToolError("L'uscita è cambiata nel frattempo: ripristino sospeso.")
        rc, _, _ = sysutil.run(["pactl", "set-default-sink", data["name"]], 5)
        if rc or workflows.audio_state()["default"] != data["name"]:
            raise ToolError("Ripristino uscita non verificato.")
        return result(True, "Uscita precedente ripristinata.")


class InstallCatalogApp(Tool):
    name, icon, permission = "install_catalog_app", "system-software-install", "system_changes"
    description = "Install a user Flatpak from the allowlisted catalog: vlc, gimp, libreoffice, krita. Network needed."
    parameters = {"type": "object", "properties": {"name": {"type": "string", "enum": list(workflows.CATALOG)}},
                  "required": ["name"]}

    def check_origin(self):
        rc, remotes, _ = sysutil.read(["flatpak", "remotes", "--user", "--columns=name,url"], 5)
        if rc or not any(line.split() == ["flathub", "https://dl.flathub.org/repo/"]
                         for line in remotes.splitlines()):
            raise ToolError("Catalogo utente Flathub non configurato o origine diversa.")

    def preview(self, args):
        try:
            app = workflows.catalog_app(args["name"])
        except workflows.WorkflowError as e:
            raise ToolError(str(e))
        self.check_origin()
        rc, info, err = sysutil.read(["flatpak", "remote-info", "--user", "--show-metadata", "flathub", app["id"]], 30)
        if rc:
            raise ToolError("Impossibile verificare l'applicazione nel catalogo: " + err[:200])
        args["_metadata"] = info
        return {"title": "Installa " + app["name"], "detail": ["Origine: Flathub · " + app["id"],
                "Installazione nel tuo account; richiede rete. Disinstallazione e recupero dei dati non sono automatici.",
                info[:6000]], "danger": False, "confirm": "Installa"}

    def run(self, args):
        app = workflows.catalog_app(args["name"])
        self.check_origin()
        rc, info, err = sysutil.read(["flatpak", "remote-info", "--user", "--show-metadata", "flathub", app["id"]], 30)
        if rc or info != args["_metadata"]:
            raise ToolError("Il catalogo è cambiato dall'anteprima. Ripeti la proposta.")
        rc, out, err = sysutil.run(["flatpak", "install", "--user", "--noninteractive", "flathub", app["id"]], 1800)
        if rc:
            raise ToolError("Installazione non completata: " + (err or out)[-500:])
        rc, location, _ = sysutil.read(["flatpak", "info", "--user", "--show-location", app["id"]], 10)
        if rc or not location.strip():
            raise ToolError("Installazione non verificata.")
        return result(True, "Installazione verificata: " + app["name"], {"app": app["id"]})


ALL = [OrganizeFolder(), DiagnoseAudio(), SetAudioOutput(), InstallCatalogApp(), OpenHelp(), OpenApp(), OpenSettings(), OpenPath(), FindFiles(), SystemStatus(), MoveFile(), RenameFile(), CreateFolder(),
       TrashFile(), SetDarkMode(), SetVolume(), SetBrightness(), SetWifi(), CreateReminder(), RunCommand()]
BY_NAME = {t.name: t for t in ALL}


def schemas():
    return [t.schema() for t in ALL if t.name != "run_command"]


def select_schemas(text, history=None):
    """Reduce CPU prefill for known domains. Routing never authorizes an action."""
    text = (text or "").lower()
    names = set()
    domains = [
        (r"cartell|file|document|foto|immagin|folder|organizz|organiz|download|directory|spost|rinomin|creare|create",
         {"organize_folder", "find_files", "open_path", "move_file", "rename_file", "create_folder", "trash_file"}),
        (r"audio|suono|sento|cuffi|altoparl|volume|sound|speaker|headphone|hear",
         {"diagnose_audio", "set_audio_output", "set_volume", "open_settings"}),
        (r"install|catalog|vlc|gimp|krita|libreoffice|applicaz|app\b",
         {"install_catalog_app", "open_app"}),
        (r"impostaz|settings|wi.?fi|bluetooth|tema|theme|schermo|display|brightness|luminosit|batter|sistema",
         {"open_settings", "system_status", "set_wifi", "set_dark_mode", "set_brightness"}),
        (r"promemoria|ricordami|remind", {"create_reminder"}),
        (r"aiuto|guida|help|scorciato|shortcut", {"open_help"}),
    ]
    # Keep the previous turn's domain for short follow-ups; never inspect attachments.
    previous = next((m.get("content", "") for m in reversed(history or [])
                     if m.get("role") == "user" and isinstance(m.get("content"), str)), "")
    routed = text + (" " + previous.lower() if len(text) < 80 else "")
    for pattern, group in domains:
        if re.search(pattern, routed):
            names.update(group)
    if not names:
        return schemas()
    return [t.schema() for t in ALL if t.name in names]


def validate_args(tool, args):
    """Reject invalid and internal model arguments before preview can attach a plan."""
    if not isinstance(args, dict):
        raise ToolError("Argomenti non validi.")
    props = tool.parameters.get("properties", {})
    if set(args) - set(props) or any(k not in args for k in tool.parameters.get("required", [])):
        raise ToolError("Argomenti mancanti o non consentiti.")
    for key, value in args.items():
        rule = props[key]
        typ = rule.get("type")
        expected = {"string": str, "integer": int, "boolean": bool, "number": (int, float),
                    "array": list, "object": dict}.get(typ)
        if expected and (not isinstance(value, expected) or typ in ("integer", "number") and isinstance(value, bool)):
            raise ToolError("Tipo di argomento non valido: " + key)
        if isinstance(value, str) and (len(value) > 4096 or "\x00" in value):
            raise ToolError("Argomento troppo lungo o non valido.")
        if "enum" in rule and value not in rule["enum"]:
            raise ToolError("Valore non consentito: " + key)
        if isinstance(value, (int, float)) and not isinstance(value, bool):
            if "minimum" in rule and value < rule["minimum"] or "maximum" in rule and value > rule["maximum"]:
                raise ToolError("Valore fuori intervallo: " + key)


def get(name):
    return BY_NAME.get(name)


def parse_args(raw):
    if isinstance(raw, dict):
        return raw
    try:
        a = json.loads(raw or "{}")
        return a if isinstance(a, dict) else {}
    except ValueError:
        return {}
