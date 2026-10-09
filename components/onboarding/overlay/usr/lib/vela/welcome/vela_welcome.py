#!/usr/bin/env python3
"""Vela Benvenuto (first-run setup) and Guida e scorciatoie.

  vela-welcome               open the setup (always)
  vela-welcome --first-run   open only if setup was never completed (autostart)
  vela-welcome --guida       open "Guida e scorciatoie"
  vela-welcome --walk DIR    test mode: screenshot every page (it/en, light/dark) into DIR, then quit
"""
import argparse, json, os, sys

os.environ.setdefault("QT_QUICK_CONTROLS_STYLE", "Basic")
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import backend as be  # noqa: E402


def main():
    ap = argparse.ArgumentParser(add_help=False)
    ap.add_argument("--first-run", action="store_true")
    ap.add_argument("--guida", "--help-page", dest="guida", action="store_true")
    ap.add_argument("--force", action="store_true")
    ap.add_argument("--walk")
    ap.add_argument("--keytest")
    a, _ = ap.parse_known_args()

    if a.first_run and be._read(be.STATE_FILE).get("done"):
        return 0

    from PySide6.QtCore import QTimer, QUrl, Qt
    from PySide6.QtGui import QGuiApplication, QFont, QIcon
    from PySide6.QtQml import QQmlApplicationEngine
    from PySide6 import QtQuick  # noqa: F401  (so root objects are typed QQuickWindow)

    QGuiApplication.setHighDpiScaleFactorRoundingPolicy(Qt.HighDpiScaleFactorRoundingPolicy.PassThrough)
    app = QGuiApplication(sys.argv)
    app.setApplicationName("vela-welcome")
    app.setOrganizationName("Vela")
    app.setDesktopFileName("org.vela.help" if a.guida else "org.vela.welcome")
    app.setWindowIcon(QIcon.fromTheme("vela-help" if a.guida else "vela-welcome", QIcon.fromTheme("help-browser")))
    f = QFont("Inter")
    f.setPixelSize(14)
    app.setFont(f)

    theme = be.Theme()
    backend = be.Backend(theme, guide_mode=a.guida)
    engine = QQmlApplicationEngine()
    ctx = engine.rootContext()
    ctx.setContextProperty("theme", theme)
    ctx.setContextProperty("backend", backend)
    errors = []
    engine.warnings.connect(lambda ws: errors.extend(w.toString() for w in ws))
    engine.load(QUrl.fromLocalFile(os.path.join(HERE, "qml", "Main.qml")))
    if not engine.rootObjects():
        print("\n".join(errors), file=sys.stderr)
        return 1
    win = engine.rootObjects()[0]

    def guarded(fn, *args):
        def run():
            try:
                fn(*args)
            except Exception:
                import traceback
                traceback.print_exc()
                app.exit(3)
        return run
    if a.walk:
        QTimer.singleShot(400, guarded(walk, app, win, backend, a.walk, errors, guarded))
    if a.keytest:
        QTimer.singleShot(400, guarded(keytest, app, win, backend, a.keytest, errors, guarded))
    rc = app.exec()
    del engine
    return rc


def _grab(win, path):
    img = win.grabWindow()
    img.save(path)


def walk(app, win, backend, outdir, errors, guarded):
    """Screenshots of every wizard page in a few configurations (used by tests/run_xvfb.sh)."""
    from PySide6.QtCore import QTimer
    os.makedirs(outdir, exist_ok=True)
    from PySide6.QtQuick import QQuickItem
    wiz = win.findChild(QQuickItem, "wizard")
    guide = win.findChild(QQuickItem, "guide")
    plan = []
    if guide is not None:
        for lang in ("it", "en"):
            plan.append(("lang", lang))
            plan.append(("shot", f"guide-shortcuts-{lang}.png"))
            plan.append(("guidetab", "tips"))
            plan.append(("shot", f"guide-tips-{lang}.png"))
            plan.append(("guidetab", "shortcuts"))
        plan.append(("mode", "dark"))
        plan.append(("shot", "guide-shortcuts-it-dark.png"))
        plan.append(("mode", "light"))
    else:
        n = int(wiz.property("steps").property("length").toInt())
        for lang, mode in (("it", "light"), ("en", "light"), ("it", "dark")):
            plan.append(("lang", lang))
            plan.append(("mode", mode))
            for i in range(n):
                plan.append(("jump", i))
                plan.append(("shot", f"{i:02d}-{lang}-{mode}.png"))
        plan.append(("mode", "light"))
        plan.append(("lang", "it"))
        plan.append(("a11y", ("large_text", True)))
        plan.append(("a11y", ("high_contrast", True)))
        plan.append(("jump", 4))
        plan.append(("shot", "04-it-a11y-large-contrast.png"))
        plan.append(("a11y", ("large_text", False)))
        plan.append(("a11y", ("high_contrast", False)))
        plan.append(("jump", 0))
    state = {"i": 0}

    def step():
        if state["i"] >= len(plan):
            with open(os.path.join(outdir, "qml-warnings.txt"), "w") as f:
                f.write("\n".join(errors) + "\n")
            app.quit()
            return
        kind, arg = plan[state["i"]]
        state["i"] += 1
        delay = 120
        if kind == "lang":
            backend.setLang(arg)
        elif kind == "mode":
            backend.setMode(arg)
        elif kind == "jump":
            from PySide6.QtCore import QMetaObject, Q_ARG
            QMetaObject.invokeMethod(wiz, "jump", Q_ARG("QVariant", arg))
            delay = 1300  # let network scan / animations settle
        elif kind == "guidetab":
            guide.setProperty("tab", arg)
        elif kind == "a11y":
            backend.setA11y(*arg)
        elif kind == "shot":
            _grab(win, os.path.join(outdir, arg))
        QTimer.singleShot(delay, guarded(step))

    step()


def keytest(app, win, backend, outdir, errors, guarded):
    """Keyboard-only run: Tab/Enter through the wizard and record which items get focus."""
    from PySide6.QtCore import QTimer, Qt, QEvent
    from PySide6.QtGui import QKeyEvent
    from PySide6.QtQuick import QQuickItem
    os.makedirs(outdir, exist_ok=True)
    wiz = win.findChild(QQuickItem, "wizard")
    log = []

    def send(key, mod=Qt.KeyboardModifier.NoModifier):
        for t in (QEvent.Type.KeyPress, QEvent.Type.KeyRelease):
            app.sendEvent(win, QKeyEvent(t, key, mod))

    def focused():
        it = win.activeFocusItem()
        if it is None:
            return None
        name = it.property("text") or it.property("title") or it.property("accessibleName") or it.objectName() \
            or it.metaObject().className()
        return str(name)

    seq = []
    # walk the whole wizard using only Tab and Return on the "Continua" button
    for page in range(10):
        seq.append(("tabs", 40))
    state = {"i": 0, "pages": []}

    def step():
        if state["i"] >= len(seq) or wiz.property("current") == 9:
            state["pages"].append(wiz.property("current"))
            _grab(win, os.path.join(outdir, "keyboard-final.png"))
            with open(os.path.join(outdir, "keyboard-test.json"), "w") as f:
                json.dump({"visited_pages": state["pages"], "log": log}, f, indent=1, ensure_ascii=False)
            app.quit()
            return
        state["i"] += 1
        state["pages"].append(wiz.property("current"))
        cur = wiz.property("current")
        seen = []
        for _ in range(60):
            send(Qt.Key.Key_Tab)
            app.processEvents()
            name = focused()
            seen.append(name)
            if name == backend.tr("nav.next"):
                break
        log.append({"page": cur, "focus_chain": seen})
        send(Qt.Key.Key_Return)
        QTimer.singleShot(700, guarded(step))

    win.requestActivate()
    QTimer.singleShot(600, guarded(step))


if __name__ == "__main__":
    sys.exit(main())
