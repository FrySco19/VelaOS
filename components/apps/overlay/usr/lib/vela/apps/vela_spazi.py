#!/usr/bin/python3
"""Spazi — one KWin virtual desktop per project, with its own folder, browser profile and apps.

Data: ~/.config/vela/spazi.json  {"version":1,"spaces":[{id,name,color,folder,browser_profile,apps:[...],desktop}]}
KWin: org.kde.KWin /VirtualDesktopManager (org.kde.KWin.VirtualDesktopManager) — create/rename/remove/switch.
Apps of a space are started the first time the space is opened in a session (marker in $XDG_RUNTIME_DIR).
CLI: vela-spazi [--nuovo] [--apri NOME] [--elenco]
"""
import json, os, re, shutil, subprocess, sys, time, uuid

LANG = "en" if (os.environ.get("LC_ALL") or os.environ.get("LC_MESSAGES") or os.environ.get("LANG") or "it").startswith("en") else "it"
S = {
 "it": dict(
  title="Spazi", new="Nuovo spazio", open="Apri", more="Altre azioni", rename="Rinomina…", save_apps="Usa le app aperte ora",
  open_folder="Apri la cartella", delete="Elimina…", empty="Nessuno spazio. Crea uno spazio per ogni progetto: finestre, cartella e browser restano separati.",
  intro="Ogni spazio ha le sue finestre, una cartella e un browser con schede e accessi separati.",
  name="Nome", name_ph="Es. Tesi, Casa, Lavoro", color="Colore", folder_cb="Crea la cartella {p}", browser_cb="Browser separato per questo spazio",
  apps_label="Apri all'avvio dello spazio", create="Crea", cancel="Annulla", save="Salva",
  a_files="File", a_browser="Browser", a_terminal="Terminale", a_editor="Kate", a_office="LibreOffice Writer",
  del_q="Eliminare lo spazio «{n}»?", del_d="Le finestre aperte passano allo spazio precedente. La cartella e i file restano dove sono.",
  del_b="Elimina", rename_t="Rinomina lo spazio", exists="Esiste già uno spazio con questo nome.",
  nokwin="Gli spazi funzionano solo nella sessione Plasma. Accedi a Plasma e riprova.",
  saved="App salvate per «{n}»: {a}.", noapps="In questo spazio non ci sono finestre di app riconosciute.",
  current="In uso", apps_n="{n} app", no_apps="nessuna app"),
 "en": dict(
  title="Spaces", new="New space", open="Open", more="More actions", rename="Rename…", save_apps="Use the apps open now",
  open_folder="Open folder", delete="Delete…", empty="No spaces yet. Create one per project: windows, folder and browser stay separate.",
  intro="Each space has its own windows, a folder and a browser with separate tabs and sign-ins.",
  name="Name", name_ph="E.g. Thesis, Home, Work", color="Color", folder_cb="Create the folder {p}", browser_cb="Separate browser for this space",
  apps_label="Open when the space starts", create="Create", cancel="Cancel", save="Save",
  a_files="Files", a_browser="Browser", a_terminal="Terminal", a_editor="Kate", a_office="LibreOffice Writer",
  del_q="Delete the space “{n}”?", del_d="Open windows move to the previous space. The folder and files stay where they are.",
  del_b="Delete", rename_t="Rename space", exists="A space with this name already exists.",
  nokwin="Spaces only work in the Plasma session. Sign in to Plasma and try again.",
  saved="Apps saved for “{n}”: {a}.", noapps="There are no recognized app windows in this space.",
  current="In use", apps_n="{n} apps", no_apps="no apps"),
}[LANG]

COLORS = ["#1C6F86", "#D4624A", "#2E8B57", "#B7791F", "#7A5AA6", "#587279"]
APP_CHOICES = [  # key, label, desktop id
    ("files", S["a_files"], "org.kde.dolphin"), ("browser", S["a_browser"], "firefox-esr"),
    ("terminal", S["a_terminal"], "org.kde.konsole"), ("editor", S["a_editor"], "org.kde.kate"),
    ("office", S["a_office"], "libreoffice-writer"),
]

def cfg_path():
    return os.path.join(os.environ.get("XDG_CONFIG_HOME") or os.path.expanduser("~/.config"), "vela", "spazi.json")

def load():
    try:
        with open(cfg_path(), encoding="utf-8") as f:
            d = json.load(f)
        return d if isinstance(d, dict) and isinstance(d.get("spaces"), list) else {"version": 1, "spaces": []}
    except (OSError, ValueError):
        return {"version": 1, "spaces": []}

def save(d):
    p = cfg_path(); os.makedirs(os.path.dirname(p), exist_ok=True)
    tmp = p + ".tmp"
    with open(tmp, "w", encoding="utf-8") as f:
        json.dump(d, f, indent=2, ensure_ascii=False)
    os.replace(tmp, p)

def projects_root():
    for name in ("Progetti", "Projects"):
        p = os.path.expanduser("~/" + name)
        if os.path.isdir(p):
            return p
    return os.path.expanduser("~/" + ("Progetti" if LANG == "it" else "Projects"))

def slug(name):
    s = re.sub(r"[^\w\-]+", "-", name.strip().lower(), flags=re.UNICODE).strip("-")
    return s or uuid.uuid4().hex[:8]

# ---------------- KWin ----------------
class KWin:
    """Virtual desktops through KWin's D-Bus API. Raises RuntimeError when not in a Plasma session."""
    def __init__(self):
        import dbus
        self.dbus = dbus
        try:
            bus = dbus.SessionBus()
            self.obj = bus.get_object("org.kde.KWin", "/VirtualDesktopManager")
            self.props = dbus.Interface(self.obj, "org.freedesktop.DBus.Properties")
            self.vdm = dbus.Interface(self.obj, "org.kde.KWin.VirtualDesktopManager")
            self.desktops()
        except dbus.DBusException as e:
            raise RuntimeError(str(e))
    IFACE = "org.kde.KWin.VirtualDesktopManager"
    def desktops(self):  # [(position, id, name)]
        return [(int(p), str(i), str(n)) for p, i, n in self.props.Get(self.IFACE, "desktops")]
    def current(self):
        return str(self.props.Get(self.IFACE, "current"))
    def switch(self, did):
        self.props.Set(self.IFACE, "current", self.dbus.String(did))
    def create(self, name):
        before = {d[1] for d in self.desktops()}
        self.vdm.createDesktop(self.dbus.UInt32(len(before)), name)
        for _ in range(20):
            new = [d for d in self.desktops() if d[1] not in before]
            if new:
                return new[0][1]
            time.sleep(0.05)
        raise RuntimeError("desktop not created")
    def rename(self, did, name):
        self.vdm.setDesktopName(did, name)
    def remove(self, did):
        self.vdm.removeDesktop(did)
    def windows_on(self, did, service, wait):
        """Normal windows on desktop `did`. A one-shot KWin script sends them back with callDBus() to `service`
        (registered by the UI with QtDBus); `wait()` spins the Qt event loop until the reply or a timeout."""
        tag = "vela-spazi-" + uuid.uuid4().hex[:10]
        script = os.path.join(os.environ.get("XDG_RUNTIME_DIR", "/tmp"), tag + ".js")
        with open(script, "w") as f:
            f.write("""
var res = [];
var ws = workspace.windowList ? workspace.windowList() : workspace.clientList();
for (var i = 0; i < ws.length; i++) {
  var w = ws[i];
  if (!w.normalWindow || w.skipTaskbar) continue;
  var on = w.onAllDesktops, ds = w.desktops || [];
  for (var j = 0; j < ds.length; j++) if (ds[j].id == "%s") on = true;
  if (on) res.push({file: w.desktopFileName || "", cls: w.resourceClass || ""});
}
callDBus("%s", "/Spazi", "org.vela.Spazi", "report", JSON.stringify(res));
""" % (did, service))
        try:
            bus = self.dbus.SessionBus()
            scripting = self.dbus.Interface(bus.get_object("org.kde.KWin", "/Scripting"), "org.kde.kwin.Scripting")
            sid = int(scripting.loadScript(script, tag))
            try:
                self.dbus.Interface(bus.get_object("org.kde.KWin", f"/Scripting/Script{sid}"), "org.kde.kwin.Script").run()
            except self.dbus.DBusException:
                try:
                    scripting.start()
                except self.dbus.DBusException:
                    pass
            reply = wait()
            try:
                scripting.unloadScript(tag)
            except self.dbus.DBusException:
                pass
            return json.loads(reply) if reply else None
        finally:
            try:
                os.unlink(script)
            except OSError:
                pass

# ---------------- launching ----------------
def firefox_profile_dir(name):
    import configparser
    for root in (os.path.expanduser("~/.config/mozilla/firefox"), os.path.expanduser("~/.mozilla/firefox")):
        ini = os.path.join(root, "profiles.ini")
        if not os.path.exists(ini):
            continue
        cp = configparser.RawConfigParser(); cp.read(ini)
        for sec in cp.sections():
            if cp.get(sec, "Name", fallback=None) == name:
                p = cp.get(sec, "Path")
                return p if cp.get(sec, "IsRelative", fallback="1") == "0" else os.path.join(root, p)
    return None

def ensure_firefox_profile(name):
    d = firefox_profile_dir(name)
    if not d:
        subprocess.run(["firefox-esr", "--headless", "-CreateProfile", name], capture_output=True, timeout=60)
        d = firefox_profile_dir(name)
    if d:
        os.makedirs(d, exist_ok=True)
        with open(os.path.join(d, "user.js"), "w") as f:
            f.write('// Vela Spazi: this profile belongs to one space and reopens its tabs.\n'
                    'user_pref("browser.startup.page", 3);\n')
    return d

def launch(app, space):
    folder = space.get("folder") or os.path.expanduser("~")
    did = app if isinstance(app, str) else app.get("id", "")
    cmds = {
        "org.kde.dolphin": ["dolphin", "--new-window", folder],
        "org.kde.konsole": ["konsole", "--workdir", folder],
        "org.kde.kate": ["kate", "--new", folder],
        "firefox-esr": (["firefox-esr", "-P", space["browser_profile"]] if space.get("browser_profile") else ["firefox-esr", "--new-window"]),
    }
    argv = cmds.get(did)
    if argv is None:
        if shutil.which("kstart"):
            argv = ["kstart", "--application", did]
        else:
            argv = ["gtk-launch", did]
    try:
        subprocess.Popen(argv, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, start_new_session=True,
                         cwd=folder if os.path.isdir(folder) else None)
    except OSError:
        pass

def marker(space):
    d = os.path.join(os.environ.get("XDG_RUNTIME_DIR", "/tmp"), "vela-spazi"); os.makedirs(d, exist_ok=True)
    return os.path.join(d, space["id"])

def find_desktop(kw, space):
    ds = kw.desktops()
    for p, i, n in ds:
        if i == space.get("desktop"):
            return i
    for p, i, n in ds:
        if n == space["name"]:
            return i
    return None

def open_space(kw, data, space):
    did = find_desktop(kw, space)
    if not did:
        did = kw.create(space["name"])
    if space.get("desktop") != did:
        space["desktop"] = did; save(data)
    kw.switch(did)
    m = marker(space)
    if not os.path.exists(m):
        open(m, "w").close()
        time.sleep(0.15)
        for app in space.get("apps", []):
            launch(app, space)

def create_space(kw, data, name, color, make_folder, browser, apps):
    name = name.strip()
    sp = {"id": uuid.uuid4().hex[:12], "name": name, "color": color, "folder": "", "browser_profile": "", "apps": apps}
    if make_folder:
        sp["folder"] = os.path.join(projects_root(), name.replace("/", "-"))
        os.makedirs(sp["folder"], exist_ok=True)
    if browser:
        sp["browser_profile"] = "Spazio " + name if LANG == "it" else "Space " + name
        ensure_firefox_profile(sp["browser_profile"])
    sp["desktop"] = kw.create(name)
    data["spaces"].append(sp); save(data)
    return sp

# ---------------- UI ----------------
def run_ui(argv):
    from PySide6.QtCore import Qt, QSize, QTimer
    from PySide6.QtGui import QIcon, QColor, QPainter, QPixmap, QAction
    from PySide6.QtWidgets import (QApplication, QWidget, QVBoxLayout, QHBoxLayout, QLabel, QPushButton, QToolButton,
                                   QMenu, QDialog, QLineEdit, QCheckBox, QDialogButtonBox, QMessageBox, QScrollArea,
                                   QFrame, QButtonGroup, QRadioButton, QInputDialog, QGridLayout)
    app = QApplication.instance() or QApplication(argv)
    app.setApplicationName("vela-spazi"); app.setApplicationDisplayName(S["title"])
    app.setDesktopFileName("org.vela.spazi")
    app.setWindowIcon(QIcon.fromTheme("vela-spazi", QIcon.fromTheme("virtual-desktops")))

    try:
        kw = KWin() if not os.environ.get("VELA_SPAZI_FAKE") else FakeKWin()
    except Exception:
        QMessageBox.information(None, S["title"], S["nokwin"]); return 1
    data = load()

    from PySide6.QtCore import QObject, Slot, ClassInfo, QEventLoop
    from PySide6.QtDBus import QDBusConnection
    service = "org.vela.Spazi.p%d" % os.getpid()

    @ClassInfo({"D-Bus Interface": "org.vela.Spazi"})
    class Receiver(QObject):
        reply = None
        loop = None
        @Slot(str)
        def report(self, text):
            self.reply = text
            if self.loop:
                self.loop.quit()
    recv = Receiver()
    sbus = QDBusConnection.sessionBus()
    sbus.registerService(service)
    sbus.registerObject("/Spazi", recv, QDBusConnection.ExportAllSlots)

    def wait_reply(timeout_ms=2500):
        recv.reply = None; recv.loop = QEventLoop()
        QTimer.singleShot(timeout_ms, recv.loop.quit)
        recv.loop.exec(); recv.loop = None
        return recv.reply

    def dot(color, size=14):
        pm = QPixmap(size, size); pm.fill(Qt.transparent)
        p = QPainter(pm); p.setRenderHint(QPainter.Antialiasing); p.setBrush(QColor(color)); p.setPen(Qt.NoPen)
        p.drawEllipse(0, 0, size, size); p.end(); return pm

    class SpaceDialog(QDialog):
        def __init__(self, parent=None):
            super().__init__(parent)
            self.setWindowTitle(S["new"]); self.setMinimumWidth(420)
            lay = QVBoxLayout(self); lay.setContentsMargins(24, 20, 24, 20); lay.setSpacing(12)
            lay.addWidget(QLabel(S["name"]))
            self.name = QLineEdit(); self.name.setPlaceholderText(S["name_ph"]); lay.addWidget(self.name)
            lay.addWidget(QLabel(S["color"]))
            row = QHBoxLayout(); self.cg = QButtonGroup(self)
            for i, c in enumerate(COLORS):
                b = QRadioButton(); b.setIcon(QIcon(dot(c, 18))); b.setIconSize(QSize(18, 18)); b.setProperty("c", c)
                b.setChecked(i == len(data["spaces"]) % len(COLORS)); self.cg.addButton(b); row.addWidget(b)
            row.addStretch(1); lay.addLayout(row)
            self.folder = QCheckBox(); self.folder.setChecked(True); lay.addWidget(self.folder)
            self.browser = QCheckBox(S["browser_cb"]); self.browser.setChecked(True); lay.addWidget(self.browser)
            lay.addSpacing(4); lay.addWidget(QLabel(S["apps_label"]))
            grid = QGridLayout(); self.appcb = []
            for k, (key, label, did) in enumerate(APP_CHOICES):
                cb = QCheckBox(label); cb.setChecked(key in ("files", "browser")); cb.setProperty("did", did)
                if key == "office" and not shutil.which("libreoffice") and not os.environ.get("VELA_SPAZI_FAKE"):
                    cb.setEnabled(False)
                self.appcb.append(cb); grid.addWidget(cb, k // 2, k % 2)
            lay.addLayout(grid)
            self.err = QLabel(); self.err.setStyleSheet("color: palette(link);"); self.err.hide(); lay.addWidget(self.err)
            bb = QDialogButtonBox(); self.ok = bb.addButton(S["create"], QDialogButtonBox.AcceptRole)
            bb.addButton(S["cancel"], QDialogButtonBox.RejectRole); self.ok.setDefault(True)
            bb.accepted.connect(self.check); bb.rejected.connect(self.reject); lay.addWidget(bb)
            self.name.textChanged.connect(self.upd); self.upd()
        def upd(self):
            n = self.name.text().strip()
            self.folder.setText(S["folder_cb"].format(p=os.path.join(os.path.basename(projects_root()), n or "…")))
            self.ok.setEnabled(bool(n)); self.err.hide()
        def check(self):
            n = self.name.text().strip()
            if any(s["name"].lower() == n.lower() for s in data["spaces"]):
                self.err.setText(S["exists"]); self.err.show(); return
            self.accept()
        def values(self):
            return (self.name.text().strip(), self.cg.checkedButton().property("c"), self.folder.isChecked(),
                    self.browser.isChecked(), [cb.property("did") for cb in self.appcb if cb.isChecked()])

    class Main(QWidget):
        def __init__(self):
            super().__init__()
            self.setWindowTitle(S["title"]); self.resize(520, 560)
            lay = QVBoxLayout(self); lay.setContentsMargins(24, 20, 24, 20); lay.setSpacing(12)
            head = QHBoxLayout()
            t = QLabel(S["title"]); f = t.font(); f.setPointSizeF(f.pointSizeF() * 1.5); f.setBold(True); t.setFont(f)
            head.addWidget(t); head.addStretch(1)
            nb = QPushButton(QIcon.fromTheme("list-add"), S["new"]); nb.clicked.connect(self.new); head.addWidget(nb)
            lay.addLayout(head)
            intro = QLabel(S["intro"]); intro.setWordWrap(True); intro.setEnabled(False); lay.addWidget(intro)
            self.area = QScrollArea(); self.area.setWidgetResizable(True); self.area.setFrameShape(QFrame.NoFrame)
            lay.addWidget(self.area, 1)
            self.msg = QLabel(); self.msg.setWordWrap(True); self.msg.hide(); lay.addWidget(self.msg)
            self.refresh()
            self.timer = QTimer(self); self.timer.timeout.connect(self.refresh_current); self.timer.start(1500)
        def info(self, text):
            self.msg.setText(text); self.msg.show(); QTimer.singleShot(5000, self.msg.hide)
        def refresh_current(self):
            try:
                cur = kw.current()
            except Exception:
                return
            if cur != getattr(self, "_cur", None):
                self.refresh()
        def refresh(self):
            try:
                self._cur = kw.current()
            except Exception:
                self._cur = None
            w = QWidget(); v = QVBoxLayout(w); v.setContentsMargins(0, 0, 0, 0); v.setSpacing(8)
            if not data["spaces"]:
                e = QLabel(S["empty"]); e.setWordWrap(True); e.setAlignment(Qt.AlignCenter); e.setEnabled(False)
                e.setMinimumHeight(160); v.addWidget(e)
            for sp in data["spaces"]:
                v.addWidget(self.row(sp))
            v.addStretch(1); self.area.setWidget(w)
        def row(self, sp):
            fr = QFrame(); fr.setFrameShape(QFrame.StyledPanel); fr.setObjectName("card")
            fr.setStyleSheet("#card { border-radius: 12px; background: palette(base); border: 1px solid palette(midlight); }")
            h = QHBoxLayout(fr); h.setContentsMargins(14, 10, 10, 10); h.setSpacing(12)
            ic = QLabel(); ic.setPixmap(dot(sp.get("color", COLORS[0]), 14)); h.addWidget(ic)
            col = QVBoxLayout(); col.setSpacing(2)
            nm = QLabel(sp["name"]); f = nm.font(); f.setBold(True); nm.setFont(f); col.addWidget(nm)
            apps = [l for k, l, d in APP_CHOICES if d in sp.get("apps", [])] + \
                   [a for a in sp.get("apps", []) if a not in [d for _, _, d in APP_CHOICES]]
            sub = []
            if sp.get("folder"):
                sub.append(sp["folder"].replace(os.path.expanduser("~"), "~"))
            sub.append(", ".join(apps) if apps else S["no_apps"])
            sl = QLabel(" · ".join(sub)); sl.setEnabled(False); col.addWidget(sl)
            h.addLayout(col, 1)
            if self._cur and sp.get("desktop") == self._cur:
                cur = QLabel(S["current"]); cur.setEnabled(False); h.addWidget(cur)
            ob = QPushButton(S["open"]); ob.clicked.connect(lambda _=False, s=sp: self.open(s)); h.addWidget(ob)
            mb = QToolButton(); mb.setIcon(QIcon.fromTheme("overflow-menu", QIcon.fromTheme("application-menu")))
            mb.setToolTip(S["more"]); mb.setPopupMode(QToolButton.InstantPopup); mb.setAutoRaise(True)
            m = QMenu(mb)
            m.addAction(QIcon.fromTheme("edit-rename"), S["rename"], lambda s=sp: self.rename(s))
            m.addAction(QIcon.fromTheme("document-save"), S["save_apps"], lambda s=sp: self.save_apps(s))
            if sp.get("folder"):
                m.addAction(QIcon.fromTheme("folder-open"), S["open_folder"],
                            lambda s=sp: subprocess.Popen(["xdg-open", s["folder"]], start_new_session=True))
            m.addSeparator()
            m.addAction(QIcon.fromTheme("edit-delete"), S["delete"], lambda s=sp: self.delete(s))
            mb.setMenu(m); h.addWidget(mb)
            return fr
        def new(self):
            d = SpaceDialog(self)
            if d.exec() == QDialog.Accepted:
                sp = create_space(kw, data, *d.values())
                self.refresh(); open_space(kw, data, sp)
        def open(self, sp):
            open_space(kw, data, sp); self.refresh()
        def rename(self, sp):
            n, ok = QInputDialog.getText(self, S["rename_t"], S["name"], text=sp["name"])
            n = n.strip()
            if ok and n and n != sp["name"]:
                if any(s["name"].lower() == n.lower() for s in data["spaces"] if s is not sp):
                    self.info(S["exists"]); return
                did = find_desktop(kw, sp)
                if did:
                    kw.rename(did, n)
                sp["name"] = n; save(data); self.refresh()
        def save_apps(self, sp):
            did = find_desktop(kw, sp)
            wins = kw.windows_on(did, service, wait_reply) if did else None
            ids = []
            for w in wins or []:
                f = (w.get("file") or "").removesuffix(".desktop")
                if f and f not in ids and f != "org.vela.spazi":
                    ids.append(f)
            if not ids:
                self.info(S["noapps"]); return
            sp["apps"] = ids; save(data); self.refresh()
            self.info(S["saved"].format(n=sp["name"], a=", ".join(ids)))
        def delete(self, sp):
            box = QMessageBox(self); box.setIcon(QMessageBox.Question)
            box.setText(S["del_q"].format(n=sp["name"])); box.setInformativeText(S["del_d"])
            dl = box.addButton(S["del_b"], QMessageBox.DestructiveRole); box.addButton(S["cancel"], QMessageBox.RejectRole)
            box.exec()
            if box.clickedButton() is dl:
                did = find_desktop(kw, sp)
                if did and len(kw.desktops()) > 1:
                    kw.remove(did)
                data["spaces"].remove(sp); save(data)
                try:
                    os.unlink(marker(sp))
                except OSError:
                    pass
                self.refresh()

    w = Main()
    if "--nuovo" in argv or "--new" in argv:
        QTimer.singleShot(0, w.new)
    w.show()
    return app.exec()

class FakeKWin:
    """Offline stand-in for screenshots/tests on machines without Plasma (VELA_SPAZI_FAKE=1)."""
    def __init__(self): self.d = [(0, "d1", "Spazio 1")]; self.cur = "d1"
    def desktops(self): return list(self.d)
    def current(self): return self.cur
    def switch(self, i): self.cur = i
    def create(self, n): i = "d%d" % (len(self.d) + 1); self.d.append((len(self.d), i, n)); return i
    def rename(self, i, n): self.d = [(p, j, n if j == i else m) for p, j, m in self.d]
    def remove(self, i): self.d = [x for x in self.d if x[1] != i]
    def windows_on(self, i, service=None, wait=None): return [{"file": "org.kde.dolphin"}, {"file": "firefox-esr"}]

def main(argv):
    if "--elenco" in argv or "--list" in argv:
        for sp in load()["spaces"]:
            print(sp["name"])
        return 0
    if "--apri" in argv or "--open" in argv:
        k = argv.index("--apri" if "--apri" in argv else "--open")
        name = argv[k + 1] if len(argv) > k + 1 else ""
        data = load()
        sp = next((s for s in data["spaces"] if s["name"].lower() == name.lower()), None)
        if not sp:
            print(f"vela-spazi: «{name}»?", file=sys.stderr); return 2
        try:
            open_space(KWin(), data, sp)
        except RuntimeError:
            print(S["nokwin"], file=sys.stderr); return 1
        return 0
    return run_ui(argv)

if __name__ == "__main__":
    sys.exit(main(sys.argv))
