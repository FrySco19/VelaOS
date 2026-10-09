#!/usr/bin/env python3
"""Vela system assistant.

  vela-assistant                 open the panel (or bring the running one to front)
  vela-assistant --toggle        show/hide (global shortcut Meta+A)
  vela-assistant --background    start hidden with the tray icon (autostart / D-Bus activation)
  vela-assistant --ask "testo"   open and ask
  vela-assistant --files F...    open with files attached (Dolphin «Chiedi all'assistente»)
  vela-assistant --setup-local   open the local-model setup (used by Benvenuto)
  vela-assistant --shots DIR     test mode: render states and save screenshots into DIR, then quit
"""
import argparse
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
os.environ.setdefault("QT_QUICK_CONTROLS_STYLE", "Basic")

SERVICE, PATH, IFACE = "org.vela.Assistant", "/Assistant", "org.vela.Assistant"

POSITION_SCRIPT = r"""
var list = workspace.windowList ? workspace.windowList() : workspace.clientList();
for (var i = 0; i < list.length; i++) {
    var w = list[i];
    var rc = String(w.resourceClass);
    if (rc === "org.vela.assistant" || rc === "vela-assistant" || String(w.desktopFileName) === "org.vela.assistant") {
        var a = workspace.clientArea(KWin.MaximizeArea, w);
        var g = w.frameGeometry;
        w.frameGeometry = {x: Math.round(a.x + (a.width - g.width) / 2), y: Math.round(a.y + a.height * 0.10),
                           width: g.width, height: g.height};
        w.keepAbove = true;
    }
}
"""


def parse():
    ap = argparse.ArgumentParser(add_help=True)
    ap.add_argument("--toggle", action="store_true")
    ap.add_argument("--background", action="store_true")
    ap.add_argument("--ask")
    ap.add_argument("--files", nargs="*")
    ap.add_argument("--intent")
    ap.add_argument("--setup-local", action="store_true")
    ap.add_argument("--shots")
    ap.add_argument("--no-tray", action="store_true")
    ap.add_argument("--no-dbus", action="store_true")
    return ap.parse_args()


def forward(a):
    """If an instance already owns org.vela.Assistant, forward the request and return True."""
    from PySide6.QtDBus import QDBusConnection, QDBusInterface, QDBusConnectionInterface
    bus = QDBusConnection.sessionBus()
    if not bus.isConnected():
        return False
    reg = bus.interface().isServiceRegistered(SERVICE)
    if not (reg.value() if hasattr(reg, "value") else bool(reg)):
        return False
    iface = QDBusInterface(SERVICE, PATH, IFACE, bus)
    token = os.environ.get("XDG_ACTIVATION_TOKEN", "")
    if a.ask:
        iface.call("Ask", a.ask, token)
    elif a.files:
        iface.call("AskFiles", a.files, token)
    elif a.intent:
        iface.call("RunIntent", a.intent, token)
    elif a.setup_local:
        iface.call("SetupLocal", token)
    elif a.background:
        pass
    else:
        iface.call("Activate", "toggle" if a.toggle else "show", token)
    return True


class Controller:
    def __init__(self, app, args):
        from PySide6.QtCore import QTimer
        import bridge as br
        self.app, self.args, self.br = app, args, br
        self.bridge = br.Bridge(app)
        self.engine = None
        self.win = None
        self.positioned = False
        self.tray = None
        self.bridge.showRequested.connect(lambda: self._present(capture=False))
        self.bridge.hideRequested.connect(self.hide)
        QTimer.singleShot(0, self.bridge.refreshStatus)

    # ---------------------------------------------------------------- window
    def _ensure_window(self):
        if self.win is not None:
            return
        from PySide6.QtCore import QUrl
        from PySide6.QtQml import QQmlApplicationEngine
        self.engine = QQmlApplicationEngine()
        for p in (os.path.join(HERE, "../../../../../../theme/overlay/usr/lib/x86_64-linux-gnu/qt6/qml"),):
            if os.path.isdir(p):
                self.engine.addImportPath(os.path.abspath(p))
        self.engine.addImageProvider("icon", self.br.IconProvider())
        self.engine.rootContext().setContextProperty("bridge", self.bridge)
        self.engine.rootContext().setContextProperty("chat", self.bridge.model)
        errors = []
        self.engine.warnings.connect(lambda ws: errors.extend(w.toString() for w in ws))
        self.engine.load(QUrl.fromLocalFile(os.path.join(HERE, "qml", "Main.qml")))
        if not self.engine.rootObjects():
            sys.stderr.write("\n".join(errors) + "\n")
            raise SystemExit(1)
        self.win = self.engine.rootObjects()[0]
        self.win.setProperty("_blurMargin", 0)
        self.win.widthChanged.connect(lambda: self.br.enable_blur(self.win, 16))
        self.win.heightChanged.connect(lambda: self.br.enable_blur(self.win, 16))
        self.win.visibleChanged.connect(lambda v: v and self.br.enable_blur(self.win, 16))

    def set_token(self, token):
        if token:
            os.environ["XDG_ACTIVATION_TOKEN"] = token

    def toggle(self):
        if self.win is not None and self.win.isVisible() and self.win.isActive():
            self.hide()
        else:
            self.show()

    def show(self, ask=None, files=None, intent=None, setup=False):
        self._ensure_window()
        if files:
            self.bridge.attach_files(files)
        self._present(capture=not self.win.isVisible())
        if ask:
            self.bridge.send(ask)
        if intent:
            self.bridge.run_intent_external(intent[0], intent[1])
        if setup:
            self.bridge.model.add("setup", "", extra={"state": "ollama_missing", "focus_local": True})
        if files and not ask:
            self.win.setProperty("prefill", "")
            self.win.setProperty("placeholderOverride", self.bridge.tr.get("files_ask_prompt", ""))

    def _present(self, capture=True):
        self._ensure_window()
        if capture:
            self.bridge.capture_context()
        self.win.show()
        self.win.raise_()
        self.win.requestActivate()
        self.bridge.focusInput.emit()
        if not self.positioned:
            self.positioned = True
            self._kwin_position()

    def hide(self):
        if self.win is not None:
            self.win.hide()

    def _kwin_position(self):
        try:
            import tempfile
            from PySide6.QtCore import QTimer
            from PySide6.QtDBus import QDBusConnection, QDBusInterface
            bus = QDBusConnection.sessionBus()
            iface = QDBusInterface("org.kde.KWin", "/Scripting", "org.kde.kwin.Scripting", bus)
            if not bus.isConnected() or not iface.isValid():
                return
            fd, path = tempfile.mkstemp(prefix="vela-pos-", suffix=".js")
            with os.fdopen(fd, "w") as f:
                f.write(POSITION_SCRIPT)

            def go():
                r = iface.call("loadScript", path, "vela-assistant-position")
                args = r.arguments() if r else []
                if args and isinstance(args[0], int) and args[0] >= 0:
                    s = QDBusInterface("org.kde.KWin", f"/Scripting/Script{args[0]}", "org.kde.kwin.Script", bus)
                    s.call("run")
                QTimer.singleShot(1000, lambda: (iface.call("unloadScript", "vela-assistant-position"),
                                                 os.path.exists(path) and os.unlink(path)))
            QTimer.singleShot(150, go)
        except Exception:
            pass

    # ---------------------------------------------------------------- tray
    def make_tray(self):
        from PySide6.QtGui import QIcon, QAction
        from PySide6.QtWidgets import QMenu, QSystemTrayIcon
        if not QSystemTrayIcon.isSystemTrayAvailable():
            return
        tr = self.bridge.tr
        icon = QIcon.fromTheme("vela-assistant-symbolic", QIcon.fromTheme("vela-assistant",
                                                                          QIcon.fromTheme("help-browser")))
        self.tray = QSystemTrayIcon(icon)
        self.tray.setToolTip(tr["tray_tip"].replace("Meta+A", self.bridge.shortcut))
        menu = QMenu()
        for label, fn in ((tr["tray_open"], self.show), (tr["new_chat"], lambda: (self.bridge.newChat(), self.show())),
                          (tr["tray_settings"], self.bridge.openSettings), (tr["open_help_title"], self.bridge.openHelp)):
            act = QAction(label, menu)
            act.triggered.connect(fn)
            menu.addAction(act)
        menu.addSeparator()
        q = QAction(tr["tray_quit"], menu)
        q.triggered.connect(self.app.quit)
        menu.addAction(q)
        self.tray.setContextMenu(menu)
        self.tray.activated.connect(lambda reason: self.toggle() if reason == QSystemTrayIcon.Trigger else None)
        self.tray._menu = menu
        self.tray.show()


def ensure_icon_theme():
    """Outside a Plasma session (tests, other desktops) Qt only searches :/icons — add the XDG icon dirs."""
    from PySide6.QtGui import QIcon
    paths = list(QIcon.themeSearchPaths())
    dirs = [os.path.join(os.environ.get("XDG_DATA_HOME") or os.path.expanduser("~/.local/share"), "icons")]
    dirs += [os.path.join(d, "icons") for d in (os.environ.get("XDG_DATA_DIRS") or "/usr/local/share:/usr/share").split(":")]
    dirs.append(os.path.abspath(os.path.join(HERE, "../../../../../../theme/overlay/usr/share/icons")))  # source tree
    for d in dirs:
        if os.path.isdir(d) and d not in paths:
            paths.append(d)
    QIcon.setThemeSearchPaths(paths)
    if not QIcon.themeName() or QIcon.themeName() == "hicolor":
        for t in ("Vela", "Papirus", "breeze"):
            if any(os.path.isdir(os.path.join(d, t)) for d in paths):
                QIcon.setThemeName(t)
                break
    if not QIcon.fallbackThemeName():
        QIcon.setFallbackThemeName("Papirus" if QIcon.themeName() != "Papirus" else "breeze")


def main():
    a = parse()
    if a.shots:
        import shots
        return shots.run(a.shots)
    from PySide6.QtCore import Qt
    from PySide6.QtGui import QFont, QIcon, QGuiApplication
    from PySide6.QtWidgets import QApplication
    QGuiApplication.setHighDpiScaleFactorRoundingPolicy(Qt.HighDpiScaleFactorRoundingPolicy.PassThrough)
    app = QApplication(sys.argv)
    app.setApplicationName("vela-assistant")
    app.setApplicationDisplayName("Assistente")
    app.setOrganizationName("Vela")
    app.setDesktopFileName("org.vela.assistant")
    ensure_icon_theme()
    app.setWindowIcon(QIcon.fromTheme("vela-assistant"))
    app.setQuitOnLastWindowClosed(False)
    f = QFont("Inter")
    f.setPointSizeF(10)
    app.setFont(f)

    if not a.no_dbus and forward(a):
        return 0

    sys.path.insert(0, HERE)
    import core.config as cfgmod
    cfg = cfgmod.load()
    ctl = Controller(app, a)

    if not a.no_dbus:
        from PySide6.QtDBus import QDBusConnection
        import bridge as br
        bus = QDBusConnection.sessionBus()
        if bus.isConnected():
            ctl.dbus = br.DBusService(ctl)
            bus.registerObject(PATH, ctl.dbus, QDBusConnection.ExportAllSlots)
            if not bus.registerService(SERVICE):
                # lost a start-up race: another instance got the name in the meantime
                if forward(a):
                    return 0

    want_tray = not a.no_tray and cfg.get("assistant", {}).get("tray", True)
    if want_tray:
        ctl.make_tray()
    if a.background:
        if ctl.tray is None and not os.environ.get("DBUS_STARTER_BUS_TYPE"):
            # autostart with the tray disabled: nothing to do until the shortcut/D-Bus wakes us up
            return 0
    elif a.ask:
        ctl.show(ask=a.ask)
    elif a.files:
        ctl.show(files=a.files)
    elif a.intent:
        try:
            j = json.loads(a.intent)
            ctl.show(intent=(j["tool"], j.get("args", {})))
        except (ValueError, KeyError):
            ctl.show()
    elif a.setup_local:
        ctl.show(setup=True)
    else:
        ctl.show()
    return app.exec()


if __name__ == "__main__":
    sys.exit(main())
