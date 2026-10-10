"""Qt side: chat model, QML bridge, D-Bus service, context capture, blur, tray."""
import base64
import ctypes
import json
import os
import shutil
import subprocess
import tempfile
import threading
import time
import uuid

from PySide6.QtCore import (QAbstractListModel, QByteArray, QModelIndex, QObject, Property, Qt, QTimer, Signal,
                            Slot, ClassInfo, QFileSystemWatcher)
from PySide6.QtGui import QGuiApplication, QPainterPath, QRegion

from core import agent as agentmod, config, context, i18n, intents, llm, ollama_setup, sysutil, tools

T = i18n.T


# ============================================================================================ chat model
class ChatModel(QAbstractListModel):
    ROLES = {Qt.UserRole + 1: b"kind", Qt.UserRole + 2: b"text", Qt.UserRole + 3: b"card",
             Qt.UserRole + 4: b"uid", Qt.UserRole + 5: b"streaming", Qt.UserRole + 6: b"extra"}
    countChanged = Signal()

    def __init__(self):
        super().__init__()
        self.items = []

    def rowCount(self, parent=QModelIndex()):
        return len(self.items)

    def roleNames(self):
        return {k: QByteArray(v) for k, v in self.ROLES.items()}

    def data(self, index, role):
        if not index.isValid() or index.row() >= len(self.items):
            return None
        name = self.ROLES.get(role)
        return self.items[index.row()].get(name.decode()) if name else None

    def add(self, kind, text="", card=None, uid=None, streaming=False, extra=None):
        uid = uid or uuid.uuid4().hex[:10]
        self.beginInsertRows(QModelIndex(), len(self.items), len(self.items))
        self.items.append({"kind": kind, "text": text, "card": card or {}, "uid": uid, "streaming": streaming,
                           "extra": extra or {}})
        self.endInsertRows()
        self.countChanged.emit()
        return uid

    def find(self, uid):
        for i, it in enumerate(self.items):
            if it["uid"] == uid:
                return i
        return -1

    def update(self, uid, **fields):
        i = self.find(uid)
        if i < 0:
            return False
        self.items[i].update(fields)
        idx = self.index(i)
        self.dataChanged.emit(idx, idx)
        return True

    def remove(self, uid):
        i = self.find(uid)
        if i >= 0:
            self.beginRemoveRows(QModelIndex(), i, i)
            self.items.pop(i)
            self.endRemoveRows()
            self.countChanged.emit()

    def clear(self):
        self.beginResetModel()
        self.items = []
        self.endResetModel()
        self.countChanged.emit()

    count = Property(int, lambda s: len(s.items), notify=countChanged)


def _card_for_qml(card):
    c = dict(card)
    c.pop("undo_data", None)
    c["args"] = {k: v for k, v in (c.get("args") or {}).items() if not k.startswith("_")}
    return c


# ============================================================================================ bridge
class Bridge(QObject):
    agentEvent = Signal(str, "QVariant")
    changed = Signal()
    contextChanged = Signal()
    installChanged = Signal()
    iconGenChanged = Signal()
    showRequested = Signal()
    hideRequested = Signal()
    focusInput = Signal()
    toast = Signal(str)

    def __init__(self, app):
        super().__init__()
        self.app = app
        cfg = config.load()
        i18n.set_lang(config.language(cfg))
        self.model = ChatModel()
        self._busy = False
        self._status = {"model": "", "provider": "", "cloud": False, "host": "", "note": "", "state": ""}
        self._ctx = {}
        self._selection = ""
        self._clipboard = ""
        self._files = []
        self._image = ""
        self._image_path = ""
        self._install = {"active": False, "phase": "", "done": 0, "total": 0, "message": "", "error": ""}
        self._install_cancel = threading.Event()
        self._stream_uid = {}
        self.agentEvent.connect(self._on_agent_event, Qt.QueuedConnection)
        self.agent = agentmod.Agent(lambda k, d: self.agentEvent.emit(k, d))
        self._ctx_token = ""
        self._icon_gen = 0
        try:
            app.paletteChanged.connect(self._bump_icons)
        except (AttributeError, TypeError):
            pass
        self._watch = QFileSystemWatcher(self)
        cdir = os.path.dirname(config.config_file())
        if os.path.isdir(cdir):
            self._watch.addPath(cdir)
        self._watch.directoryChanged.connect(lambda _p: QTimer.singleShot(200, self.refreshStatus))

    def _bump_icons(self, *_):
        self._icon_gen += 1
        self.iconGenChanged.emit()

    iconGen = Property(int, lambda s: s._icon_gen, notify=iconGenChanged)

    # ------------------------------------------------------------------ properties
    def _get_tr(self):
        return i18n.table()

    tr = Property("QVariantMap", _get_tr, notify=changed)
    busy = Property(bool, lambda s: s._busy, notify=changed)
    status = Property("QVariantMap", lambda s: s._status, notify=changed)
    messages = Property(QObject, lambda s: s.model, constant=True)
    windowTitle = Property(str, lambda s: s._ctx.get("window_title", ""), notify=contextChanged)
    windowApp = Property(str, lambda s: s._ctx.get("window_app", ""), notify=contextChanged)
    windowIcon = Property(str, lambda s: s._ctx.get("window_icon", ""), notify=contextChanged)
    space = Property(str, lambda s: s._ctx.get("space", ""), notify=contextChanged)
    selection = Property(str, lambda s: s._selection, notify=contextChanged)
    clipboard = Property(str, lambda s: s._clipboard, notify=contextChanged)
    files = Property("QVariantList", lambda s: [{"name": f["name"], "path": f["path"], "kind": f.get("kind", "")}
                                                for f in s._files], notify=contextChanged)
    screenshot = Property(str, lambda s: ("file://" + s._image_path) if s._image_path else "",
                          notify=contextChanged)
    install = Property("QVariantMap", lambda s: s._install, notify=installChanged)

    @Property(bool, constant=True)
    def blur(self):
        return blur_available()

    @Property(bool, constant=True)
    def animations(self):
        """False when the user chose reduced motion (kdeglobals [KDE] AnimationDurationFactor=0)."""
        try:
            import configparser
            cp = configparser.RawConfigParser(strict=False)
            cp.read(os.path.join(os.environ.get("XDG_CONFIG_HOME") or os.path.expanduser("~/.config"), "kdeglobals"))
            return float(cp.get("KDE", "AnimationDurationFactor", fallback="1")) > 0
        except (ValueError, configparser.Error):
            return True

    @Property("QVariantList", constant=True)
    def localModels(self):
        out = []
        for tag, size, cls in ollama_setup.MODELS:
            out.append({"tag": tag, "size": _gb(size), "label": T("model_choice_" + cls, size=_gb(size))})
        return out

    @Slot(str, result=bool)
    def hasIcon(self, names):
        from PySide6.QtGui import QIcon
        return any(n and QIcon.hasThemeIcon(n) for n in (names or "").split("|"))

    @Slot(str, result=str)
    def downloadSummary(self, tag):
        need = ollama_setup.needed_bytes(tag, ollama_setup.ollama_binary() is None)
        free = shutil.disk_usage(os.path.expanduser("~")).free
        return T("setup_local_download", size=_gb(need), free=_gb(free))

    @Property(str, constant=True)
    def shortcut(self):
        return config.load().get("assistant", {}).get("shortcut", "Meta+A")

    # ------------------------------------------------------------------ status / setup
    @Slot()
    def refreshStatus(self):
        cfg = config.load()
        i18n.set_lang(config.language(cfg))
        threading.Thread(target=self.agent.check, daemon=True).start()
        self.changed.emit()

    # ------------------------------------------------------------------ context capture (called before showing)
    def capture_context(self, after=None):
        """Selection now (before our window takes focus); active window via KWin script (async callback)."""
        cfg = config.load()
        perms = cfg.get("permissions", {})
        self._selection = context.selected_text() if perms.get("selected_text", True) and cfg.get("enabled", True) \
            else ""
        self._clipboard = context.clipboard_text() if perms.get("clipboard", False) and cfg.get("enabled", True) \
            else ""
        self._ctx = {}
        if perms.get("active_window", True) and cfg.get("enabled", True):
            if not self._kwin_context():
                x = context.x11_active_window()
                if x and x.get("app") not in ("vela-assistant", "vela_assistant.py"):
                    self._set_ctx(x)
        self.contextChanged.emit()
        if after:
            after()

    def _kwin_context(self):
        try:
            from PySide6.QtDBus import QDBusConnection, QDBusInterface
        except ImportError:
            return False
        bus = QDBusConnection.sessionBus()
        if not bus.isConnected():
            return False
        iface = QDBusInterface("org.kde.KWin", "/Scripting", "org.kde.kwin.Scripting", bus)
        if not iface.isValid():
            return False
        self._ctx_token = uuid.uuid4().hex[:8]
        fd, path = tempfile.mkstemp(prefix="vela-ctx-", suffix=".js")
        with os.fdopen(fd, "w") as f:
            f.write(context.kwin_script_text(self._ctx_token))
        name = "vela-assistant-ctx-" + self._ctx_token
        reply = iface.call("loadScript", path, name)
        args = reply.arguments() if reply else []
        if not args or not isinstance(args[0], int) or args[0] < 0:
            os.unlink(path)
            return False
        sid = args[0]
        for p in (f"/Scripting/Script{sid}", f"/{sid}"):
            s = QDBusInterface("org.kde.KWin", p, "org.kde.kwin.Script", bus)
            if s.isValid():
                s.call("run")
                break
        QTimer.singleShot(1500, lambda: (iface.call("unloadScript", name), _unlink(path)))
        return True

    def report_context(self, title, app, desktop_file, space, token):
        if token != self._ctx_token:
            return
        if app.lower() in ("vela-assistant", "python3") and "Assistente" in title:
            return
        self._set_ctx({"title": title, "app": app, "desktop_file": desktop_file, "space": space})
        self.contextChanged.emit()

    def _set_ctx(self, x):
        icon = x.get("desktop_file") or x.get("app", "").lower()
        self._ctx = {"window_title": x.get("title", "")[:200], "window_app": x.get("app", ""),
                     "window_icon": icon, "space": x.get("space", "")}
        self.agent.set_context(self._ctx)

    @Slot(str)
    def removeChip(self, kind):
        if kind == "window":
            self._ctx = {k: v for k, v in self._ctx.items() if not k.startswith("window")}
        elif kind == "space":
            self._ctx.pop("space", None)
        elif kind == "selection":
            self._selection = ""
        elif kind == "clipboard":
            self._clipboard = ""
        elif kind == "screenshot":
            _unlink(self._image_path)
            self._image, self._image_path = "", ""
        elif kind.startswith("file:"):
            self._files = [f for f in self._files if f["path"] != kind[5:]]
        self.agent.set_context(self._ctx)
        self.contextChanged.emit()

    @Slot()
    def useClipboard(self):
        txt = context.clipboard_text()
        if not txt:
            txt = QGuiApplication.clipboard().text()
        self._clipboard = context._clean(txt)
        self.contextChanged.emit()

    def attach_files(self, paths):
        cfg = config.load()
        if cfg.get("permissions", {}).get("files") in ("never", False):
            self.model.add("error", T("files_denied"))
            return
        out = []
        for p in paths:
            if p.startswith("file://"):
                import urllib.parse
                p = urllib.parse.unquote(p[7:])
            try:
                out.append(context.describe_file(p))
            except OSError:
                pass
        self._files = out[:5]
        config.log("attach_files", ", ".join(f["path"] for f in self._files), confirmed=True)
        self.contextChanged.emit()

    @Slot()
    def lookAtScreen(self):
        """Explicit click only: hide, screenshot with Spectacle, show again with the image attached."""
        cfg = config.load()
        if cfg.get("permissions", {}).get("screen") in ("never", False):
            self.model.add("error", T("screen_denied"))
            return
        self.hideRequested.emit()
        QTimer.singleShot(350, self._take_screenshot)

    def _take_screenshot(self):
        fd, path = tempfile.mkstemp(prefix="vela-screen-", suffix=".png")
        os.close(fd)
        ok = False
        for cmd in (["spectacle", "-b", "-n", "-f", "-o", path], ["grim", path], ["import", "-window", "root", path]):
            if sysutil.which(cmd[0]):
                rc, _, _ = sysutil.read(cmd, 15)
                ok = rc == 0 and os.path.getsize(path) > 0
                if ok:
                    break
        if not ok:
            screen = QGuiApplication.primaryScreen()
            if screen is not None:
                ok = screen.grabWindow(0).save(path, "PNG")
        if ok:
            self._image_path = path
            self._image = _image_data_uri(path)
            config.log("screenshot", T("screenshot_attached"), confirmed=True)
        else:
            _unlink(path)
            self.model.add("error", T("screen_failed"))
        self.contextChanged.emit()
        self.showRequested.emit()

    # ------------------------------------------------------------------ chat
    @Slot(str)
    def send(self, text):
        self._send(text, True)

    def _send(self, text, bubble):
        text = (text or "").strip()
        if not text or self._busy:
            return
        cfg = config.load()
        i18n.set_lang(config.language(cfg))
        if bubble:
            self.model.add("user", text, extra=self._chips_summary())
        if not cfg.get("enabled", True):
            self.model.add("setup", T("state_disabled"), extra={"state": "disabled"})
            return
        m = intents.match(text) if not (self._files or self._image or self._selection or self._clipboard) else None
        if m and m[0] == "open_app" and not tools.get("open_app").find(m[1]["name"]):
            m = None
        if m:
            self.run_intent_external(m[0], m[1])
            return
        att = {"selection": self._selection, "clipboard": self._clipboard, "files": self._files,
               "image": self._image}
        # attachments are used for this question only
        self._selection, self._clipboard, self._files = "", "", []
        _unlink(self._image_path)
        self._image, self._image_path = "", ""
        self.contextChanged.emit()
        self.agent.ask(text, att)

    def _chips_summary(self):
        chips = []
        if self._selection:
            chips.append(T("attach_selection"))
        if self._clipboard:
            chips.append(T("attach_clipboard"))
        if self._files:
            chips.append(", ".join(f["name"] for f in self._files))
        if self._image:
            chips.append(T("screenshot_attached"))
        return {"chips": chips}

    @Slot()
    def stop(self):
        threading.Thread(target=self.agent.stop, daemon=True).start()

    @Slot()
    def newChat(self):
        self.agent.new_conversation()
        self.model.clear()
        self._stream_uid = {}
        self.focusInput.emit()

    @Slot(str, bool)
    def confirm(self, card_id, danger_ok):
        self.agent.confirm(card_id, danger_ok)

    @Slot(str)
    def decline(self, card_id):
        self.agent.decline(card_id)

    @Slot(str)
    def cancelOperation(self, card_id):
        self.agent.cancel_operation(card_id)

    @Slot(str)
    def undo(self, card_id):
        threading.Thread(target=self.agent.undo, args=(card_id,), daemon=True).start()

    def run_intent_external(self, tool_name, args):
        if self._busy:
            return
        self._busy = True
        self.changed.emit()
        def prepare():
            try:
                self.agent.run_intent(tool_name, args)
            finally:
                self.agentEvent.emit("busy", {"on": False})
        threading.Thread(target=prepare, daemon=True).start()

    @Slot(str)
    def proposeAudioOutput(self, name):
        self.run_intent_external("set_audio_output", {"name": name})

    def _on_agent_event(self, kind, data):
        data = data or {}
        if kind == "busy":
            self._busy = bool(data.get("on"))
            self.changed.emit()
        elif kind == "status":
            self._status = dict(data)
            self.changed.emit()
        elif kind == "assistant_start":
            self._stream_uid[data["id"]] = self.model.add("assistant", "", uid=data["id"], streaming=True)
        elif kind == "assistant_delta":
            i = self.model.find(data["id"])
            if i >= 0:
                self.model.update(data["id"], text=self.model.items[i]["text"] + data["text"])
        elif kind == "assistant_end":
            if data.get("empty"):
                self.model.remove(data["id"])
            else:
                self.model.update(data["id"], streaming=False)
        elif kind == "card":
            c = data["card"]
            self.model.add("card", card=_card_for_qml(c), uid="card-" + c["id"])
        elif kind == "card_update":
            c = data["card"]
            self.model.update("card-" + c["id"], card=_card_for_qml(c))
        elif kind == "setup":
            self._status = dict(self._status, state=data.get("state", ""))
            self.model.add("setup", data.get("message", ""), extra=dict(data))
            self.changed.emit()
        elif kind == "error":
            self.model.add("error", data.get("message", ""), extra={"detail": data.get("detail", "")})
        elif kind == "_install":
            self.handle_install_event(data)

    # ------------------------------------------------------------------ setup actions
    @Slot(str)
    def startLocalSetup(self, tag):
        if self._install["active"]:
            return
        self._install_cancel = threading.Event()
        self._install = {"active": True, "phase": "start", "done": 0, "total": 0, "message": "", "error": ""}
        self.installChanged.emit()
        ep = config.endpoint()
        native = ep.get("native_url") or "http://127.0.0.1:11434"
        threading.Thread(target=self._install_worker, args=(tag, native), daemon=True).start()

    def _install_worker(self, tag, native):
        last = [0.0]

        def progress(phase, done, total):
            now = time.time()
            if phase in ("download", "pull") and now - last[0] < 0.25 and done < total:
                return
            last[0] = now
            key = {"download": "install_download", "extract": "install_extract", "start": "install_start",
                   "pull": "install_pull"}[phase]
            msg = T(key, done=_gb(done), total=_gb(total)) if total else T(key, done="", total="")
            self.agentEvent.emit("_install", {"active": True, "phase": phase, "done": done, "total": total,
                                              "message": msg, "error": ""})
        try:
            ollama_setup.setup_local(tag, progress, self._install_cancel, native)
            self.agentEvent.emit("_install", {"active": False, "phase": "done", "done": 0, "total": 0,
                                              "message": T("install_done"), "error": ""})
        except ollama_setup.Cancelled:
            self.agentEvent.emit("_install", {"active": False, "phase": "cancelled", "done": 0, "total": 0,
                                              "message": T("install_cancelled"), "error": ""})
        except ollama_setup.SetupError as e:
            why = str(e)
            msg = T("install_no_space", need=_gb(ollama_setup.needed_bytes(tag, True) + 1_500_000_000)) \
                if why == "no_space" else T("install_failed", why=why)
            config.log("setup_local", why, confirmed=True, result="error")
            self.agentEvent.emit("_install", {"active": False, "phase": "error", "done": 0, "total": 0,
                                              "message": msg, "error": why})
        except OSError as e:
            config.log("setup_local", str(e), confirmed=True, result="error")
            self.agentEvent.emit("_install", {"active": False, "phase": "error", "done": 0, "total": 0,
                                              "message": T("install_failed", why=getattr(e, "reason", None) or str(e)),
                                              "error": str(e)})

    @Slot()
    def cancelSetup(self):
        self._install_cancel.set()

    @Slot()
    def startOllama(self):
        self.startLocalSetup(config.load().get("ollama", {}).get("model") or ollama_setup.MODELS[0][0])

    @Slot(str, str, str, bool, result=str)
    def saveOnline(self, url, model, key, consent):
        url = (url or "").strip().rstrip("/")
        model = (model or "").strip()
        if not url.startswith(("http://", "https://")) or not model:
            return T("err_generic")
        config.save_user({"enabled": True, "provider": "openai", "openai": {"base_url": url, "model": model},
                          "privacy": {"allow_cloud": bool(consent)}})
        if key.strip():
            config.set_api_key(key.strip())
        config.log("setup_online", f"{url} · {model} · allow_cloud={bool(consent)}", confirmed=True)
        self.refreshStatus()
        return ""

    @Slot()
    def enableAssistant(self):
        config.save_user({"enabled": True})
        if config.endpoint()["base_url"] == "http://127.0.0.1:11435/v1":
            sysutil.run(["systemctl", "--user", "start", "vela-local-ai.service"], timeout=5)
        self.refreshStatus()

    @Slot()
    def openSettings(self):
        for cmd in (["systemsettings", "kcm_vela_ai"], ["kcmshell6", "kcm_vela_ai"]):
            if sysutil.which(cmd[0]):
                sysutil.spawn(cmd)
                self.hideRequested.emit()
                return

    @Slot()
    def openHelp(self):
        if sysutil.which("vela-help"):
            sysutil.spawn(["vela-help"])
            self.hideRequested.emit()

    @Slot(str)
    def openPath(self, path):
        sysutil.spawn(["xdg-open", path])

    @Slot(str)
    def reveal(self, path):
        if sysutil.which("dolphin"):
            sysutil.spawn(["dolphin", "--select", path])
        else:
            sysutil.spawn(["xdg-open", os.path.dirname(path)])

    @Slot(str)
    def copy(self, text):
        QGuiApplication.clipboard().setText(text)
        self.toast.emit(T("copied"))

    @Slot()
    def retry(self):
        """Ask the last question again (after setup or an error) without repeating the bubble."""
        for it in reversed(list(self.model.items)):
            if it["kind"] in ("setup", "error"):
                self.model.remove(it["uid"])
                continue
            if it["kind"] == "user":
                # the failed attempt may be in the history already: drop it so the question isn't doubled
                h = self.agent.history
                if h and h[-1].get("role") == "user":
                    h.pop()
                self._send(it["text"], False)
                return
            break

    def handle_install_event(self, data):
        self._install = dict(data)
        self.installChanged.emit()
        if data.get("phase") == "done":
            self.refreshStatus()


def _gb(n):
    if n >= 1e9:
        return f"{n / 1e9:.1f} GB".replace(".", ",")
    return f"{max(0, n) / 1e6:.0f} MB"


def _unlink(p):
    if p:
        try:
            os.unlink(p)
        except OSError:
            pass


def _image_data_uri(path, max_side=1600):
    """Downscale for the model (vision models work at ~1-2 MP) and return a PNG/JPEG data URI."""
    from PySide6.QtGui import QImage
    from PySide6.QtCore import QBuffer, QIODevice
    img = QImage(path)
    if img.isNull():
        return ""
    if max(img.width(), img.height()) > max_side:
        img = img.scaled(max_side, max_side, Qt.KeepAspectRatio, Qt.SmoothTransformation)
    buf = QBuffer()
    buf.open(QIODevice.WriteOnly)
    img.save(buf, "JPEG", 85)
    return "data:image/jpeg;base64," + base64.b64encode(bytes(buf.data())).decode()


# ============================================================================================ D-Bus service
@ClassInfo({"D-Bus Interface": "org.vela.Assistant"})
class DBusService(QObject):
    """org.vela.Assistant on the session bus. `token` = XDG activation token (Wayland focus), may be empty."""

    def __init__(self, controller):
        super().__init__()
        self.c = controller

    @Slot(str, str)
    def Activate(self, action, token):
        self.c.set_token(token)
        self.c.toggle() if action == "toggle" else self.c.show()

    @Slot()
    def Toggle(self):
        self.c.toggle()

    @Slot(str, str)
    def Ask(self, text, token):
        self.c.set_token(token)
        self.c.show(ask=text)

    @Slot("QStringList", str)
    def AskFiles(self, paths, token):
        self.c.set_token(token)
        self.c.show(files=list(paths))

    @Slot(str, str)
    def RunIntent(self, payload, token):
        self.c.set_token(token)
        try:
            j = json.loads(payload)
            self.c.show(intent=(j["tool"], j.get("args", {})))
        except (ValueError, KeyError):
            pass

    @Slot(str)
    def SetupLocal(self, token):
        self.c.set_token(token)
        self.c.show(setup=True)

    @Slot(str, str, str, str, str)
    def ReportContext(self, title, app, desktop_file, space, token):
        self.c.bridge.report_context(title, app, desktop_file, space, token)

    @Slot(result=str)
    def Ping(self):
        return "ok"


# ============================================================================================ icons
from PySide6.QtQuick import QQuickImageProvider  # noqa: E402


class IconProvider(QQuickImageProvider):
    """image://icon/name1|name2?size  -> first theme icon found (Vela/Papirus/Breeze)."""

    def __init__(self):
        super().__init__(QQuickImageProvider.Pixmap)

    def requestPixmap(self, ident, size, requested):
        from PySide6.QtGui import QIcon, QPixmap
        from PySide6.QtCore import QSize
        import urllib.parse
        names = urllib.parse.unquote(ident).split("?")[0].split("|")
        w = requested.width() if requested.width() > 0 else 32
        h = requested.height() if requested.height() > 0 else w
        for n in names:
            if n and QIcon.hasThemeIcon(n):
                pm = QIcon.fromTheme(n).pixmap(QSize(w, h))
                if not pm.isNull():
                    if size is not None:
                        size.setWidth(pm.width()); size.setHeight(pm.height())
                    return pm
        pm = QPixmap(w, h)
        pm.fill(Qt.transparent)
        return pm


# ============================================================================================ blur
def _kws():
    lib = getattr(_kws, "lib", None)
    if lib is None:
        lib = ctypes.CDLL("libKF6WindowSystem.so.6")
        _kws.lib = lib
    return lib


def blur_available():
    """KWindowEffects::isEffectAvailable(BlurBehind=7)."""
    if os.environ.get("VELA_ASSISTANT_NO_BLUR") == "1":
        return False
    try:
        fn = _kws()._ZN14KWindowEffects17isEffectAvailableENS_6EffectE
        fn.argtypes = [ctypes.c_int]
        fn.restype = ctypes.c_bool
        return bool(fn(7))
    except Exception:
        return False


def enable_blur(window, radius=16):
    """KWin blur behind the rounded panel (KWindowEffects via ctypes; no-op when unavailable)."""
    if os.environ.get("VELA_ASSISTANT_NO_BLUR") == "1":
        return False
    try:
        import shiboken6
        fn = _kws()._ZN14KWindowEffects16enableBlurBehindEP7QWindowbRK7QRegion
        fn.argtypes = [ctypes.c_void_p, ctypes.c_bool, ctypes.c_void_p]
        fn.restype = None
        m = int(window.property("panelMargin") or 0)
        path = QPainterPath()
        from PySide6.QtCore import QRectF
        path.addRoundedRect(QRectF(m, m, window.width() - 2 * m, window.height() - 2 * m), radius, radius)
        region = QRegion(path.toFillPolygon().toPolygon())
        enable_blur._region = region   # keep alive
        fn(shiboken6.getCppPointer(window)[0], True, shiboken6.getCppPointer(region)[0])
        return True
    except Exception:
        return False
