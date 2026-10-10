"""Backend for Vela Benvenuto / Guida e scorciatoie.

Every system change goes through Backend._run(), which honours VELA_WELCOME_DRYRUN=1 (commands are only logged to
$VELA_WELCOME_LOG or ~/.cache/vela/welcome-actions.log). Files are written under the user's XDG dirs only.
"""
import json, os, shutil, subprocess, sys, threading, time, urllib.request, urllib.error, urllib.parse

from PySide6.QtCore import QObject, Property, Signal, Slot, QTimer

HERE = os.path.dirname(os.path.abspath(__file__))
for p in ("/usr/lib/vela/python",
          os.path.join(HERE, "../../../../../../settings-store/overlay/usr/lib/vela/python")):
    if os.path.isdir(p) and p not in sys.path:
        sys.path.insert(0, os.path.abspath(p))
try:
    import vela_ai_config as vac  # shared contract, components/settings-store/AI_CONFIG.md
except Exception:  # pragma: no cover - only if settings-store is missing
    vac = None

DRYRUN = os.environ.get("VELA_WELCOME_DRYRUN") == "1"


def xdg(var, default):
    return os.environ.get(var) or os.path.expanduser(default)


CONF_DIR = os.path.join(xdg("XDG_CONFIG_HOME", "~/.config"), "vela")
STATE_FILE = os.path.join(CONF_DIR, "welcome.json")
APPEARANCE_FILE = os.path.join(CONF_DIR, "appearance.json")
DESKTOP_FILE = os.path.join(CONF_DIR, "desktop.json")
A11Y_FILE = os.path.join(CONF_DIR, "accessibility.json")
TIPS_FILE = os.path.join(CONF_DIR, "tips.json")
LOG_FILE = os.environ.get("VELA_WELCOME_LOG") or os.path.join(xdg("XDG_CACHE_HOME", "~/.cache"), "vela",
                                                               "welcome-actions.log")

# Same palette and order as Impostazioni > Aspetto (settings-store kcm_vela_aspetto).
ACCENTS = [("coral", "#D4624A"), ("seablue", "#1C6F86"), ("blue", "#2F80ED"), ("green", "#2E8B57"),
           ("purple", "#8E5BD0"), ("pink", "#D6457A"), ("orange", "#D9822B"), ("graphite", "#5F6B73")]
KEYBOARDS = ["it", "us", "gb", "de", "fr", "es", "pt", "ch"]
KB_VARIANT = {"ch": "fr"}

# Provisional design tokens; replaced by /usr/share/vela/theme/tokens.json when agent `theme` ships it.
DEFAULT_TOKENS = {
    "light": {"bg": "#F4F6FA", "sidebar": "#E9EEF5", "surface": "#FFFFFF", "surface2": "#EEF2F7",
              "border": "#D6DEE8", "text": "#0E1A2B", "muted": "#566579", "navy": "#0B2545",
              "field": "#FFFFFF", "shadow": "#1A0B2545", "success": "#1E7F4F", "danger": "#C0392B"},
    "dark": {"bg": "#0B1526", "sidebar": "#0E1B30", "surface": "#13233B", "surface2": "#1A2E4A",
             "border": "#2A3F5E", "text": "#EAF0F7", "muted": "#9DAEC4", "navy": "#0B2545",
             "field": "#0F1E33", "shadow": "#66000000", "success": "#4CC38A", "danger": "#FF7A6B"},
    "radius": 12, "font": "Inter",
}
DEFAULT_TOKENS["light"].update(bg="#F3F6F7", sidebar="#E8EFF1", surface="#FFFFFF", surface2="#F6F9FA", border="#C9D6DA",
                               text="#15303A", muted="#587279", success="#2E8B57", danger="#C93A3A", navy="#0B2A3A")
DEFAULT_TOKENS["dark"].update(bg="#15232B", sidebar="#172730", surface="#1A2A33", surface2="#1E303A", border="#34505C",
                              text="#E3EEF0", muted="#93AAB1", field="#1A2A33", success="#5CBF8A", danger="#E66A6A",
                              navy="#0B2A3A")


def aspetto_hours():
    """Day/night switch times used by `vela-aspetto modo auto` (theme). Defaults documented in requests.md."""
    p = os.path.join(xdg("XDG_CONFIG_HOME", "~/.config"), "vela", "aspetto.conf")
    day, night = "07:00", "19:30"
    try:
        for line in open(p, encoding="utf-8"):
            k, _, v = line.partition("=")
            if k.strip() == "giorno" and v.strip():
                day = v.strip()
            elif k.strip() == "notte" and v.strip():
                night = v.strip()
    except OSError:
        pass
    return day, night


def _minutes(hhmm):
    try:
        h, m = hhmm.split(":")
        return int(h) * 60 + int(m)
    except ValueError:
        return 0


def tool_supports(tool, word):
    """True if `tool` exists and its source/usage mentions `word` (feature detection for other components' CLIs)."""
    path = shutil.which(tool)
    if not path:
        return False
    try:
        with open(path, "rb") as f:
            return word.encode() in f.read(200000)
    except OSError:
        return False


def _read(path, default=None):
    try:
        with open(path, encoding="utf-8") as f:
            return json.load(f)
    except (OSError, ValueError):
        return {} if default is None else default


def _write(path, data):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    tmp = path + ".tmp"
    with open(tmp, "w", encoding="utf-8") as f:
        json.dump(data, f, indent=1, ensure_ascii=False)
    os.replace(tmp, path)


def is_live():
    if os.environ.get("VELA_WELCOME_LIVE") in ("0", "1"):
        return os.environ["VELA_WELCOME_LIVE"] == "1"
    try:
        cmd = open("/proc/cmdline").read()
    except OSError:
        cmd = ""
    return "boot=live" in cmd or os.path.isdir("/run/live/medium")


class Theme(QObject):
    changed = Signal()

    def __init__(self):
        super().__init__()
        tok = {k: (dict(v) if isinstance(v, dict) else v) for k, v in DEFAULT_TOKENS.items()}
        shipped = _read(os.environ.get("VELA_TOKENS") or "/usr/share/vela/theme/tokens.json")
        # theme's tokens.json (components/theme/TOKENS.md) -> our names
        names = {"bg": "window", "sidebar": "sidebar", "surface": "view", "surface2": "viewAlt", "border": "border",
                 "text": "text", "muted": "textSecondary", "field": "view", "success": "positive",
                 "danger": "negative", "focus": "focus"}
        for k in ("light", "dark"):
            src = shipped.get(k) if isinstance(shipped.get(k), dict) else {}
            for ours, theirs in names.items():
                v = src.get(theirs)
                if isinstance(v, str) and v.startswith("#"):
                    tok[k][ours] = v
            brand = shipped.get("brand") or {}
            if brand.get("marine800"):
                tok[k]["navy"] = brand["marine800"]
        self._tok = tok
        ap = _read(APPEARANCE_FILE)
        self._mode = ap.get("mode", "light")
        self._accent = ap.get("accent", ACCENTS[0][1])
        a11y = _read(A11Y_FILE)
        self._contrast = bool(a11y.get("high_contrast"))
        self._large = bool(a11y.get("large_text"))
        self._motion = not bool(a11y.get("reduce_motion"))

    def _dark(self):
        if self._mode == "auto":
            day, night = aspetto_hours()
            now = time.localtime().tm_hour * 60 + time.localtime().tm_min
            return now >= _minutes(night) or now < _minutes(day)
        return self._mode == "dark"

    def getC(self):
        c = dict(self._tok["dark" if self._dark() else "light"])
        c["accent"] = self._accent
        c["onAccent"] = "#FFFFFF"
        if self._contrast:
            def shade(hexc, f):
                r, g, b = (int(hexc[i:i + 2], 16) for i in (1, 3, 5))
                if f < 1:
                    r, g, b = (int(x * f) for x in (r, g, b))
                else:
                    r, g, b = (int(x + (255 - x) * (f - 1)) for x in (r, g, b))
                return "#%02X%02X%02X" % (r, g, b)
            c["accent"] = shade(self._accent, 1.35) if self._dark() else shade(self._accent, 0.72)
            c["onAccent"] = "#000000" if self._dark() else "#FFFFFF"
            if self._dark():
                c.update(bg="#000000", sidebar="#000000", surface="#000000", surface2="#101010", border="#FFFFFF",
                         text="#FFFFFF", muted="#E6E6E6", field="#000000")
            else:
                c.update(bg="#FFFFFF", sidebar="#FFFFFF", surface="#FFFFFF", surface2="#F0F0F0", border="#000000",
                         text="#000000", muted="#1A1A1A", field="#FFFFFF")
        return c

    c = Property("QVariantMap", getC, notify=changed)
    dark = Property(bool, _dark, notify=changed)
    radius = Property(int, lambda s: int(s._tok.get("radius", 12)), constant=True)
    font = Property(str, lambda s: s._tok.get("font", "Inter"), constant=True)
    scale = Property(float, lambda s: 1.15 if s._large else 1.0, notify=changed)
    animations = Property(bool, lambda s: s._motion, notify=changed)
    highContrast = Property(bool, lambda s: s._contrast, notify=changed)

    def set(self, **kw):
        for k, v in kw.items():
            setattr(self, "_" + k, v)
        self.changed.emit()


class Backend(QObject):
    langChanged = Signal()
    stateChanged = Signal()
    wifiChanged = Signal()
    aiStatus = Signal(str, str)      # kind (ok|warn|error|busy), message
    message = Signal(str)

    def __init__(self, theme, guide_mode=False):
        super().__init__()
        self.theme = theme
        self.guide_mode = guide_mode
        self._strings = _read(os.path.join(HERE, "strings.json"))
        akey = ((self._ai_cfg().get("assistant") or {}).get("shortcut") or "Meta+A")
        self._akey = akey
        self._shortcuts = _read(os.path.join(HERE, "shortcuts.json")).get("items", [])
        for it in self._shortcuts:
            it["keys"] = [akey.split("+") if c == ["@assistant"] else c for c in it.get("keys", [])]
        self._tips = _read(os.path.join(HERE, "tips.json")).get("tips", [])
        for t in self._tips:
            for lang in ("it", "en"):
                t[lang] = [x.replace("@assistant", akey.replace("Space", "Spazio" if lang == "it" else "Space"))
                           for x in t.get(lang, [])]
        st = _read(STATE_FILE)
        env_lang = (os.environ.get("LANGUAGE") or os.environ.get("LC_ALL") or os.environ.get("LANG") or "it")[:2]
        self._lang = os.environ.get("VELA_WELCOME_LANG") or st.get("language") or ("en" if env_lang == "en" else "it")
        self._kb = st.get("keyboard", "us" if self._lang == "en" else "it")
        self._aspetto_raw = {}
        ap = _read(APPEARANCE_FILE)
        if not ap and shutil.which("vela-aspetto") and not DRYRUN:
            ap = self._aspetto_state()
            if ap:
                _write(APPEARANCE_FILE, ap)
                theme.set(mode=ap["mode"], accent=ap["accent"])
        self._mode = ap.get("mode", "light")
        self._accent = ap.get("accent", ACCENTS[0][1])
        dk = _read(DESKTOP_FILE)
        self._dock_pos = dk.get("dock_position", "bottom")
        self._dock_autohide = bool(dk.get("dock_autohide", False))
        self._desk_icons = bool(dk.get("new_apps_desktop", True))
        self._dock_new = bool(dk.get("new_apps_dock", False))
        self._single_click = bool(dk.get("single_click", False))
        a = _read(A11Y_FILE)
        if not a and self._aspetto_raw:
            a = {"high_contrast": bool(self._aspetto_raw.get("high_contrast")),
                 "large_text": str(self._aspetto_raw.get("text", "")).lower() in ("grande", "large", "true"),
                 "reduce_motion": str(self._aspetto_raw.get("motion", "")).lower() in ("ridotto", "reduced")}
        self._a11y = {"large_text": bool(a.get("large_text")), "high_contrast": bool(a.get("high_contrast")),
                      "reduce_motion": bool(a.get("reduce_motion")), "screen_reader": bool(a.get("screen_reader"))}
        if self._aspetto_raw and any(self._a11y.values()):
            theme.set(contrast=self._a11y["high_contrast"], large=self._a11y["large_text"],
                      motion=not self._a11y["reduce_motion"])
        cfg = self._ai_cfg()
        self._ai_choice = st.get("ai_choice") or ("off" if not cfg.get("enabled", True) else
                                                   ("online" if cfg.get("provider") == "openai" and
                                                    cfg.get("openai", {}).get("base_url") != "http://127.0.0.1:11435/v1"
                                                    else "local"))
        self._wifi = []
        self._wifi_state = "unknown"   # unknown|scanning|nowifi|list|connecting|failed
        self._net_summary = ""
        self._wifi_msg = ""
        self._log("start", {"dryrun": DRYRUN, "live": is_live(), "lang": self._lang})

    # ------------------------------------------------------------------ utils
    def _log(self, what, data=None):
        os.makedirs(os.path.dirname(LOG_FILE), exist_ok=True)
        with open(LOG_FILE, "a", encoding="utf-8") as f:
            f.write(json.dumps({"ts": time.strftime("%Y-%m-%dT%H:%M:%S"), "what": what, "data": data},
                               ensure_ascii=False) + "\n")

    def _run(self, argv, wait=True, timeout=20):
        """Run a system command (or only log it in dry-run). Returns (rc, stdout)."""
        self._log("run", argv)
        if DRYRUN:
            return 0, ""
        if shutil.which(argv[0]) is None:
            self._log("missing", argv[0])
            return 127, ""
        try:
            if not wait:
                subprocess.Popen(argv, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, start_new_session=True)
                return 0, ""
            r = subprocess.run(argv, capture_output=True, text=True, timeout=timeout)
            if r.returncode:
                self._log("rc", {"argv": argv, "rc": r.returncode, "err": r.stderr[-400:]})
            return r.returncode, r.stdout
        except (OSError, subprocess.TimeoutExpired) as e:
            self._log("error", str(e))
            return 1, ""

    def _kwrite(self, file, group, key, value):
        argv = ["kwriteconfig6", "--notify", "--file", file]
        for g in (group if isinstance(group, (list, tuple)) else [group]):
            argv += ["--group", g]
        argv += ["--key", key]
        if isinstance(value, bool):
            argv += ["--type", "bool", "true" if value else "false"]
        else:
            argv += [str(value)]
        return self._run(argv)

    def _bg(self, fn, *a):
        threading.Thread(target=fn, args=a, daemon=True).start()

    def _ai_cfg(self):
        if vac:
            try:
                return vac.load()
            except Exception as e:  # malformed file etc.
                self._log("ai_load_error", str(e))
        return {}

    def _ai_save(self, patch):
        self._log("ai_save", patch)
        if vac:
            vac.save_user(patch)
        else:
            p = os.path.join(CONF_DIR, "ai.json")
            d = _read(p)
            d.update(patch)
            _write(p, d)

    def _save_state(self, **kw):
        st = _read(STATE_FILE)
        st.update(kw)
        _write(STATE_FILE, st)

    def _aspetto_state(self):
        """Current appearance from theme's `vela-aspetto stato` (JSON: mode, accent, ...)."""
        try:
            r = subprocess.run(["vela-aspetto", "stato"], capture_output=True, text=True, timeout=5)
            st = json.loads(r.stdout or "{}")
        except (OSError, ValueError, subprocess.TimeoutExpired):
            return {}
        if isinstance(st, dict):
            self._aspetto_raw = st
        mode = {"chiaro": "light", "scuro": "dark", "auto": "auto", "light": "light", "dark": "dark"}.get(
            str(st.get("mode") or st.get("modo") or "").lower())
        accent = st.get("accent") or st.get("accento") or ""
        if not (isinstance(accent, str) and accent.startswith("#") and len(accent) == 7):
            accent = ACCENTS[0][1]
        return {"mode": mode, "accent": accent} if mode else {}

    # ------------------------------------------------------------------ i18n
    def getLang(self):
        return self._lang

    def setLang(self, v):
        if v not in ("it", "en") or v == self._lang:
            return
        self._lang = v
        self._save_state(language=v)
        loc = "it_IT.UTF-8" if v == "it" else "en_US.UTF-8"
        self._bg(self._apply_lang, v, loc)
        self.langChanged.emit()

    def _apply_lang(self, v, loc):
        self._kwrite("plasma-localerc", "Translations", "LANGUAGE", "it:en_US" if v == "it" else "en_US")
        self._kwrite("plasma-localerc", "Formats", "LANG", loc)
        # ai.json "language" stays "auto": the assistant follows the system language after the next sign-in

    lang = Property(str, getLang, setLang, notify=langChanged)

    def getS(self):
        base = dict(self._strings.get("it", {}))
        base.update(self._strings.get(self._lang, {}))
        return base

    s = Property("QVariantMap", getS, notify=langChanged)

    @Slot(str, result=str)
    def tr(self, key):
        d = self._strings.get(self._lang, {})
        return d.get(key) or self._strings.get("it", {}).get(key) or key

    @Slot(str, str, result=str)
    def tr1(self, key, arg):
        return self.tr(key).replace("%1", arg)

    # ------------------------------------------------------------------ static info
    @Property(bool, constant=True)
    def live(self):
        return is_live()

    @Property(bool, constant=True)
    def guideMode(self):
        return self.guide_mode

    @Property(bool, constant=True)
    def canSetupLocal(self):
        return DRYRUN or tool_supports("vela-assistant", "setup-local")

    @Slot()
    def setupLocal(self):
        """Hand over to the assistant's own local-model setup (download/choose a model)."""
        self._run(["vela-assistant", "--setup-local"], wait=False)

    @Property(bool, constant=True)
    def dryRun(self):
        return DRYRUN

    @Property("QVariantList", constant=True)
    def accents(self):
        return [{"id": i, "color": c} for i, c in ACCENTS]

    @Property("QVariantList", constant=True)
    def keyboards(self):
        return KEYBOARDS

    @Property("QVariantList", constant=True)
    def shortcuts(self):
        return self._shortcuts

    @Property("QVariantList", constant=True)
    def tips(self):
        return self._tips

    @Property(str, constant=True)
    def assistantShortcut(self):
        return self._akey

    @Property(str, notify=langChanged)
    def autoHint(self):
        day, night = aspetto_hours()
        if self._lang == "it":
            return "Scuro dalle %s alle %s." % (night, day.lstrip("0") if day[:2] != "00" else day)
        return "Dark from %s to %s." % (night, day)

    @Property(bool, constant=True)
    def orcaAvailable(self):
        return DRYRUN or shutil.which("orca") is not None

    # ------------------------------------------------------------------ keyboard
    def getKb(self):
        return self._kb

    def setKb(self, v):
        if v not in KEYBOARDS or v == self._kb:
            return
        self._kb = v
        self._save_state(keyboard=v)
        self._bg(self._apply_kb, v)
        self.stateChanged.emit()

    def _apply_kb(self, v):
        self._kwrite("kxkbrc", "Layout", "Use", True)
        self._kwrite("kxkbrc", "Layout", "LayoutList", v)
        self._kwrite("kxkbrc", "Layout", "VariantList", KB_VARIANT.get(v, ""))
        self._run(["dbus-send", "--session", "--type=signal", "/Layouts", "org.kde.keyboard.reloadConfig"])
        if os.environ.get("XDG_SESSION_TYPE") == "x11" and not DRYRUN:
            args = ["setxkbmap", v] + (["-variant", KB_VARIANT[v]] if v in KB_VARIANT else [])
            self._run(args)

    keyboard = Property(str, getKb, setKb, notify=stateChanged)

    # ------------------------------------------------------------------ appearance
    def _save_appearance(self):
        _write(APPEARANCE_FILE, dict(_read(APPEARANCE_FILE), mode=self._mode, accent=self._accent))
        self.theme.set(mode=self._mode, accent=self._accent)
        self._bg(self._apply_appearance)

    def _apply_appearance(self):
        mode, accent = self._mode, self._accent
        if shutil.which("vela-aspetto") or DRYRUN:
            # theme's CLI: colour scheme + wallpaper variant + auto day/night timer
            self._run(["vela-aspetto", "modo", {"light": "chiaro", "dark": "scuro", "auto": "auto"}[mode]])
            self._run(["vela-aspetto", "accento", "predefinito" if accent.lower() == ACCENTS[0][1].lower() else accent])
            return
        dark = mode == "dark" or (mode == "auto" and self.theme.dark)
        scheme = self._pick_scheme(dark)
        if scheme:
            self._run(["plasma-apply-colorscheme", "--accent-color", accent, scheme])

    def _pick_scheme(self, dark):
        names = (["VelaScuro", "BreezeDark"] if dark else ["VelaChiaro", "BreezeLight"])
        if self.theme.highContrast:
            names = ["VelaContrasto" + ("Scuro" if dark else ""), "VelaContrasto"] + names
        dirs = ["/usr/share/color-schemes", os.path.join(xdg("XDG_DATA_HOME", "~/.local/share"), "color-schemes")]
        for n in names:
            if any(os.path.exists(os.path.join(d, n + ".colors")) for d in dirs):
                return n
        return names[-1]

    def getMode(self):
        return self._mode

    def setMode(self, v):
        if v in ("light", "dark", "auto") and v != self._mode:
            self._mode = v
            self._save_appearance()
            self.stateChanged.emit()

    mode = Property(str, getMode, setMode, notify=stateChanged)

    def getAccent(self):
        return self._accent

    def setAccent(self, v):
        if v != self._accent:
            self._accent = v
            self._save_appearance()
            self.stateChanged.emit()

    accent = Property(str, getAccent, setAccent, notify=stateChanged)

    # ------------------------------------------------------------------ accessibility
    @Slot(str, result=bool)
    def a11y(self, key):
        return bool(self._a11y.get(key))

    @Slot(str, bool)
    def setA11y(self, key, on):
        if key not in self._a11y or self._a11y[key] == on:
            return
        self._a11y[key] = on
        _write(A11Y_FILE, dict(_read(A11Y_FILE), **self._a11y))
        self.theme.set(contrast=self._a11y["high_contrast"], large=self._a11y["large_text"],
                       motion=not self._a11y["reduce_motion"])
        self._bg(self._apply_a11y, key, on)
        self.stateChanged.emit()

    def _apply_a11y(self, key, on):
        if key == "reduce_motion":
            if shutil.which("vela-aspetto") or DRYRUN:
                self._run(["vela-aspetto", "movimento", "ridotto" if on else "normale"])
            else:
                self._kwrite("kdeglobals", "KDE", "AnimationDurationFactor", "0" if on else "1")
        elif key == "screen_reader":
            self._kwrite("kaccessrc", "ScreenReader", "Enabled", on)
            if on:
                self._run(["orca", "--replace"], wait=False)
            else:
                self._run(["orca", "--quit"])
        elif key == "large_text":
            if DRYRUN or tool_supports("vela-aspetto", "testo"):
                self._run(["vela-aspetto", "testo", "grande" if on else "normale"])
            else:
                self._apply_fonts(on)
        elif key == "high_contrast":
            if DRYRUN or tool_supports("vela-aspetto", "contrasto"):
                self._run(["vela-aspetto", "contrasto", "alto" if on else "normale"])
            else:
                self._apply_appearance()

    def _apply_fonts(self, on):
        keys = ["font", "fixed", "menuFont", "toolBarFont", "smallestReadableFont"]
        a = _read(A11Y_FILE)
        saved = a.get("saved_fonts") or {}
        if on:
            for k in keys:
                rc, cur = self._run(["kreadconfig6", "--file", "kdeglobals", "--group", "General", "--key", k])
                cur = cur.strip()
                if not cur:
                    cur = {"fixed": "Hack,10,-1,5,400,0,0,0,0,0,0,0,0,0,0,1",
                           "smallestReadableFont": "Inter,8,-1,5,400,0,0,0,0,0,0,0,0,0,0,1"}.get(
                        k, "Inter,10,-1,5,400,0,0,0,0,0,0,0,0,0,0,1")
                saved.setdefault(k, cur)
                parts = saved[k].split(",")
                try:
                    parts[1] = str(round(float(parts[1]) * 1.25))
                except (IndexError, ValueError):
                    continue
                self._kwrite("kdeglobals", "General", k, ",".join(parts))
            _write(A11Y_FILE, dict(_read(A11Y_FILE), saved_fonts=saved))
        else:
            for k, v in saved.items():
                self._kwrite("kdeglobals", "General", k, v)
            _write(A11Y_FILE, dict(_read(A11Y_FILE), saved_fonts={}))

    # ------------------------------------------------------------------ dock & desktop
    def _save_desktop(self):
        data = dict(_read(DESKTOP_FILE), dock_position=self._dock_pos, dock_autohide=self._dock_autohide,
                    new_apps_desktop=self._desk_icons, new_apps_dock=self._dock_new, single_click=self._single_click)
        _write(DESKTOP_FILE, data)
        self._bg(self._apply_desktop)
        self.stateChanged.emit()

    def _apply_desktop(self):
        # desktop owns kdeglobals [KDE] SingleClick as a system default; this is the user's own choice (~/.config)
        self._kwrite("kdeglobals", "KDE", "SingleClick", self._single_click)
        # desktop's CLI: what appears on the desktop / in the Dock when an app is installed
        if shutil.which("vela-desktop-settings") or DRYRUN:
            self._run(["vela-desktop-settings", "--set",
                       "NewApps.AddToDesktop=" + ("true" if self._desk_icons else "false"),
                       "NewApps.AddToDock=" + ("true" if self._dock_new else "false")])
        if DRYRUN or tool_supports("vela-desktop-settings", "--dock"):
            self._run(["vela-desktop-settings", "--dock", self._dock_pos,
                       "--dock-autohide", "true" if self._dock_autohide else "false"])
        else:
            # Fallback: Plasma scripting on the panel that holds the task manager (the Dock).
            js = ("for (const p of panels()) { if (p.widgets().some(w => w.type == 'org.kde.plasma.icontasks' || "
                  "w.type == 'org.kde.plasma.taskmanager')) { p.location = '%s'; p.hiding = '%s'; } }"
                  % (self._dock_pos, "autohide" if self._dock_autohide else "none"))
            self._run(["dbus-send", "--session", "--type=method_call", "--dest=org.kde.plasmashell", "/PlasmaShell",
                       "org.kde.PlasmaShell.evaluateScript", "string:" + js])

    def _prop(name):
        def g(self):
            return getattr(self, name)

        def s(self, v):
            if getattr(self, name) != v:
                setattr(self, name, v)
                self._save_desktop()
        return g, s

    dockPosition = Property(str, *_prop("_dock_pos"), notify=stateChanged)
    dockAutohide = Property(bool, *_prop("_dock_autohide"), notify=stateChanged)
    desktopIcons = Property(bool, *_prop("_desk_icons"), notify=stateChanged)
    dockNewApps = Property(bool, *_prop("_dock_new"), notify=stateChanged)
    singleClick = Property(bool, *_prop("_single_click"), notify=stateChanged)

    # ------------------------------------------------------------------ network
    def _wifi_get(self):
        return self._wifi

    wifi = Property("QVariantList", _wifi_get, notify=wifiChanged)
    wifiState = Property(str, lambda s: s._wifi_state, notify=wifiChanged)
    netSummary = Property(str, lambda s: s._net_summary, notify=wifiChanged)
    wifiMessage = Property(str, lambda s: s._wifi_msg, notify=wifiChanged)

    @Slot()
    def scanWifi(self):
        self._wifi_state = "scanning"
        self.wifiChanged.emit()
        self._bg(self._scan)

    def _scan(self):
        fake = os.environ.get("VELA_WELCOME_FAKE_WIFI")
        if fake:  # test hook: JSON list of {ssid,signal,secure,active}
            time.sleep(0.6)
            self._wifi = json.loads(fake)
            self._wifi_state = "list" if self._wifi else "nowifi"
            self._net_summary = "online"
            self.wifiChanged.emit()
            return
        if not shutil.which("nmcli"):
            self._wifi, self._wifi_state, self._net_summary = [], "nowifi", "unknown"
            self.wifiChanged.emit()
            return
        r = subprocess.run(["nmcli", "-t", "-f", "TYPE,STATE,CONNECTION", "device"], capture_output=True, text=True)
        devs = [l.split(":", 2) for l in r.stdout.splitlines() if l.count(":") >= 2]
        has_wifi = any(d[0] == "wifi" for d in devs)
        wired = any(d[0] == "ethernet" and d[1].startswith("connected") for d in devs)
        c = subprocess.run(["nmcli", "-t", "-f", "CONNECTIVITY", "general"], capture_output=True, text=True)
        online = c.stdout.strip() in ("full", "limited", "portal")
        self._net_summary = "wired" if wired else ("online" if online else "offline")
        if not has_wifi:
            self._wifi, self._wifi_state = [], "nowifi"
            self.wifiChanged.emit()
            return
        r = subprocess.run(["nmcli", "-t", "-f", "IN-USE,SSID,SIGNAL,SECURITY", "device", "wifi", "list",
                            "--rescan", "yes"], capture_output=True, text=True, timeout=30)
        seen, nets = set(), []
        for line in r.stdout.splitlines():
            parts = line.replace("\\:", "\x00").split(":")
            if len(parts) < 4:
                continue
            ssid = parts[1].replace("\x00", ":")
            if not ssid or ssid in seen:
                continue
            seen.add(ssid)
            nets.append({"ssid": ssid, "signal": int(parts[2] or 0), "secure": parts[3] not in ("", "--"),
                         "active": parts[0] == "*"})
        nets.sort(key=lambda n: (not n["active"], -n["signal"]))
        self._wifi, self._wifi_state = nets[:12], "list"
        self.wifiChanged.emit()

    @Slot(str, str)
    def connectWifi(self, ssid, password):
        self._wifi_state, self._wifi_msg = "connecting", ""
        self.wifiChanged.emit()
        self._bg(self._connect, ssid, password)

    def _connect(self, ssid, password):
        argv = ["nmcli", "device", "wifi", "connect", ssid] + (["password", password] if password else [])
        self._log("run", ["nmcli", "device", "wifi", "connect", ssid, "password", "***" if password else ""])
        if DRYRUN or os.environ.get("VELA_WELCOME_FAKE_WIFI"):
            time.sleep(0.8)
            ok = password != "wrong"
        else:
            r = subprocess.run(argv, capture_output=True, text=True, timeout=60)
            ok = r.returncode == 0
        if ok:
            for n in self._wifi:
                n["active"] = n["ssid"] == ssid
            self._wifi_state, self._wifi_msg, self._net_summary = "list", "", "online"
        else:
            self._wifi_state, self._wifi_msg = "failed", ssid
        self.wifiChanged.emit()

    @Slot()
    def openNetworkSettings(self):
        self._run(["systemsettings", "kcm_networkmanagement"], wait=False)

    # ------------------------------------------------------------------ AI
    def getAiChoice(self):
        return self._ai_choice

    def setAiChoice(self, v):
        if v in ("local", "online", "off") and v != self._ai_choice:
            self._ai_choice = v
            self.stateChanged.emit()

    aiChoice = Property(str, getAiChoice, setAiChoice, notify=stateChanged)

    @Slot(str, result=str)
    def aiGet(self, path):
        cur = self._ai_cfg()
        for p in path.split("."):
            cur = cur.get(p) if isinstance(cur, dict) else None
        if isinstance(cur, bool):
            return "true" if cur else "false"
        return "" if cur is None else str(cur)

    @Slot(result=bool)
    def hasApiKey(self):
        return bool(vac and vac.get_api_key())

    @Slot()
    def checkLocal(self):
        self.aiStatus.emit("busy", self.tr("ai.checking"))
        self._bg(self._check_local)

    def _check_local(self):
        if self.aiGet("openai.base_url") == "http://127.0.0.1:11435/v1":
            try:
                with urllib.request.urlopen("http://127.0.0.1:11435/health", timeout=4) as r:
                    json.load(r)
                self.aiStatus.emit("ok", "Gemma 4 E2B · locale · incluso / local · included")
            except OSError:
                self.aiStatus.emit("warn", "Modello incluso in caricamento / Included model loading")
            return
        base = self.aiGet("ollama.base_url") or "http://127.0.0.1:11434"
        try:
            with urllib.request.urlopen(base.rstrip("/") + "/api/tags", timeout=4) as r:
                models = [m.get("name") for m in json.load(r).get("models", [])]
            want = self.aiGet("ollama.model")
            if not models:
                self.aiStatus.emit("warn", self.tr("ai.localNoModel"))
            else:
                self.aiStatus.emit("ok", self.tr1("ai.localOk", want if want in models else models[0]))
        except Exception as e:
            self._log("ollama", str(e))
            self.aiStatus.emit("warn", self.tr("ai.localDown"))

    @Slot(str, str, str)
    def testOnline(self, url, model, key):
        self.aiStatus.emit("busy", self.tr("ai.checking"))
        self._bg(self._test_online, url, model, key)

    def _test_online(self, url, model, key):
        key = key or (vac.get_api_key() if vac else "")
        try:
            req = urllib.request.Request(url.rstrip("/") + "/models", headers={"Authorization": "Bearer " + key})
            with urllib.request.urlopen(req, timeout=8) as r:
                ids = [m.get("id") for m in json.load(r).get("data", [])]
            if model and ids and model not in ids:
                self.aiStatus.emit("warn", self.tr1("ai.testFail", model + " ?"))
            else:
                self.aiStatus.emit("ok", self.tr("ai.testOk"))
        except urllib.error.HTTPError as e:
            self.aiStatus.emit("error", self.tr1("ai.testFail", "HTTP %d" % e.code))
        except Exception as e:
            reason = getattr(e, "reason", e)
            self.aiStatus.emit("error", self.tr1("ai.testFail", str(reason)[:80]))

    @Slot(str, str, str, bool, result=str)
    def saveAi(self, url, model, key, consent):
        """Writes ai.json through the shared helper. Returns '' or an error string key."""
        ch = self._ai_choice
        if ch == "off":
            self._ai_save({"enabled": False})
        elif ch == "local":
            self._ai_save({"enabled": True, "provider": "openai", "privacy": {"allow_cloud": False},
                           "openai": {"base_url": "http://127.0.0.1:11435/v1", "model": "vela-local"}})
        else:
            if not consent:
                return "ai.needConsent"
            if not key and not self.hasApiKey():
                return "ai.needKey"
            patch = {"enabled": True, "provider": "openai", "privacy": {"allow_cloud": True},
                     "openai": {"base_url": url.strip() or "https://api.openai.com/v1", "model": model.strip()}}
            self._ai_save(patch)
            if key and vac:
                vac.set_api_key(key.strip())
            self._log("api_key_saved", bool(key))
        self._save_state(ai_choice=ch)
        return ""

    @Slot(str, result=bool)
    def privGet(self, name):
        cfg = self._ai_cfg()
        perm = cfg.get("permissions", {})
        priv = cfg.get("privacy", {})
        if name in ("active_window", "selected_text", "clipboard"):
            return bool(perm.get(name))
        if name in ("screen", "microphone"):
            return perm.get(name, "ask") != "never"
        if name == "memory":
            return bool(priv.get("memory_enabled", True))
        if name == "log":
            return bool(priv.get("log_actions", True))
        return False

    @Slot(str, bool)
    def privSet(self, name, on):
        if name in ("active_window", "selected_text", "clipboard"):
            self._ai_save({"permissions": {name: on}})
        elif name in ("screen", "microphone"):
            self._ai_save({"permissions": {name: "ask" if on else "never"}})
        elif name == "memory":
            self._ai_save({"privacy": {"memory_enabled": on}})
        elif name == "log":
            self._ai_save({"privacy": {"log_actions": on}})

    # ------------------------------------------------------------------ lifecycle
    @Slot()
    def launchInstaller(self):
        for argv in (["vela-install"], ["calamares-install-debian"], ["pkexec", "calamares"]):
            if DRYRUN or shutil.which(argv[0]):
                self._run(argv, wait=False)
                return
        self.message.emit(self.tr("welcome.installFail"))

    @Slot()
    def finish(self):
        self._save_state(done=True, finished_at=time.strftime("%Y-%m-%dT%H:%M:%S"), version=1)
        st = _read(TIPS_FILE)
        if "first_day" not in st:
            _write(TIPS_FILE, dict(st, first_day=time.strftime("%Y-%m-%d"), enabled=True, shown=[]))
        self._log("finish", None)

    @Slot()
    def openGuide(self):
        self._run([sys.executable, os.path.join(HERE, "vela_welcome.py"), "--guida"], wait=False)

    @Slot()
    def openWelcome(self):
        self._run([sys.executable, os.path.join(HERE, "vela_welcome.py"), "--force"], wait=False)

    def getTipsEnabled(self):
        return bool(_read(TIPS_FILE).get("enabled", True))

    def setTipsEnabled(self, v):
        _write(TIPS_FILE, dict(_read(TIPS_FILE), enabled=bool(v)))
        self.stateChanged.emit()

    tipsEnabled = Property(bool, getTipsEnabled, setTipsEnabled, notify=stateChanged)

    @Slot(str, result=str)
    def keyName(self, k):
        s = self.tr("key." + k)
        return k if s == "key." + k else s
