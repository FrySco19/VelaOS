"""Test mode (`vela-assistant --shots DIR`): drive the real UI through its main states against a mock
OpenAI-compatible server and save screenshots. Used by tests/run_xvfb.sh and in the dev VM.

Screenshots are the assistant window composited on a plain marine gradient (Xvfb has no compositor or wallpaper).
"""
import json
import os
import sys
import tempfile
import time

HERE = os.path.dirname(os.path.abspath(__file__))


def run(outdir):
    os.makedirs(outdir, exist_ok=True)
    tmp = tempfile.mkdtemp(prefix="vela-shots-")
    home = os.path.join(tmp, "home")
    os.makedirs(os.path.join(home, "Documenti"), exist_ok=True)
    os.makedirs(os.path.join(home, "Download"), exist_ok=True)
    for n in ("fattura-luce-settembre.pdf", "fattura-gas-agosto.pdf", "contratto-affitto.pdf"):
        open(os.path.join(home, "Download", n), "w").write("x")
    open(os.path.join(home, "Scrivania-appunti.txt"), "w").write("appunti")
    os.environ.update(HOME=home, XDG_CONFIG_HOME=os.path.join(home, ".config"),
                      XDG_DATA_HOME=os.path.join(home, ".local/share"), VELA_ASSISTANT_NO_BLUR="1")
    sys.path.insert(0, os.path.abspath(os.path.join(HERE, "../../../../../tests")))
    tests_dir = os.environ.get("VELA_ASSISTANT_TESTS") or os.path.abspath(os.path.join(HERE, "../../../../../tests"))
    sys.path.insert(0, tests_dir)
    import mock_openai
    srv, url = mock_openai.start()

    from PySide6.QtCore import Qt, QTimer, QPointF, QRectF
    from PySide6.QtGui import QFont, QGuiApplication, QImage, QPainter, QLinearGradient, QColor, QPalette, QIcon
    from PySide6.QtWidgets import QApplication

    app = QApplication(sys.argv[:1])
    app.setDesktopFileName("org.vela.assistant")
    f = QFont("Inter")
    f.setPointSizeF(10)
    app.setFont(f)
    import core.config as config
    import core.i18n as i18n
    import vela_assistant as va
    va.ensure_icon_theme()

    def cfg(**over):
        config.vac.reset_user()
        config.save_user(over)

    cfg(provider="ollama", ollama={"base_url": "http://127.0.0.1:9", "model": "qwen2.5:1.5b"})
    ctl = va.Controller(app, None)
    br = ctl.bridge
    ctl._ensure_window()
    win = ctl.win

    light = QGuiApplication.palette()
    dark = QPalette()
    for role, col in ((QPalette.Window, "#15232B"), (QPalette.WindowText, "#E3EEF0"), (QPalette.Base, "#1A2A33"),
                      (QPalette.AlternateBase, "#1E303A"), (QPalette.Text, "#E3EEF0"), (QPalette.Button, "#253944"),
                      (QPalette.ButtonText, "#E3EEF0"), (QPalette.Highlight, "#D4624A"),
                      (QPalette.HighlightedText, "#FFFFFF"), (QPalette.PlaceholderText, "#93AAB1"),
                      (QPalette.ToolTipBase, "#E3EEF0"), (QPalette.ToolTipText, "#15232B")):
        dark.setColor(role, QColor(col))
    lightp = QPalette()
    for role, col in ((QPalette.Window, "#F3F6F7"), (QPalette.WindowText, "#15303A"), (QPalette.Base, "#FFFFFF"),
                      (QPalette.AlternateBase, "#F6F9FA"), (QPalette.Text, "#15303A"), (QPalette.Button, "#FFFFFF"),
                      (QPalette.ButtonText, "#15303A"), (QPalette.Highlight, "#D4624A"),
                      (QPalette.HighlightedText, "#FFFFFF"), (QPalette.PlaceholderText, "#587279"),
                      (QPalette.ToolTipBase, "#15303A"), (QPalette.ToolTipText, "#F3F6F7")):
        lightp.setColor(role, QColor(col))

    errors = []
    ctl.engine.warnings.connect(lambda ws: errors.extend(w.toString() for w in ws))
    shots = []

    def save(name):
        img = win.grabWindow()
        bg = QImage(img.width() + 80, img.height() + 80, QImage.Format_ARGB32)
        p = QPainter(bg)
        g = QLinearGradient(QPointF(0, 0), QPointF(bg.width(), bg.height()))
        g.setColorAt(0, QColor("#123B4D"))
        g.setColorAt(1, QColor("#397984"))
        p.fillRect(QRectF(0, 0, bg.width(), bg.height()), g)
        p.drawImage(40, 40, img)
        p.end()
        path = os.path.join(outdir, name + ".png")
        bg.save(path)
        shots.append(path)

    def fake_context():
        br._ctx = {"window_title": "Relazione trimestrale.odt — LibreOffice Writer", "window_app": "libreoffice",
                   "window_icon": "libreoffice-writer", "space": "Lavoro"}
        br._selection = "Il fatturato del terzo trimestre è cresciuto del 12% rispetto allo stesso periodo del 2025."
        br.agent.set_context(br._ctx)
        br.contextChanged.emit()

    def wait_idle(cond, timeout=8.0):
        end = time.time() + timeout
        while time.time() < end:
            app.processEvents()
            if cond():
                break
            time.sleep(0.02)
        for _ in range(30):
            app.processEvents()
            time.sleep(0.01)

    def set_lang(l):
        os.environ["LANG"] = "en_US.UTF-8" if l == "en" else "it_IT.UTF-8"
        i18n.set_lang(l)
        br.changed.emit()

    def scene(theme):
        from PySide6.QtGui import QIcon
        if QIcon.themeName().startswith("Papirus"):
            QIcon.setThemeName("Papirus-Dark" if theme == "dark" else "Papirus")
        app.setPalette(dark if theme == "dark" else lightp)
        br._bump_icons()
        br.newChat()
        wait_idle(lambda: True, 0.2)

    steps = []
    for theme in ("light", "dark"):
        def s_empty(theme=theme):
            scene(theme)
            cfg(provider="ollama", ollama={"base_url": url, "model": "qwen2.5:1.5b"})
            br.refreshStatus()
            fake_context()
            ctl._present(capture=False)
            wait_idle(lambda: br.status.get("state") == "ready")
            save(f"01-vuoto-{theme}")
        def s_answer(theme=theme):
            br.newChat()
            fake_context()
            br.send("Riassumi il testo selezionato in una frase")
            wait_idle(lambda: not br.busy and br.model.count >= 2)
            save(f"02-risposta-{theme}")
        def s_move(theme=theme):
            br.newChat()
            br.removeChip("selection")
            br.send("Sposta ~/Scrivania-appunti.txt in ~/Documenti")
            wait_idle(lambda: not br.busy and any(i["kind"] == "card" for i in br.model.items))
            save(f"03-conferma-{theme}")
            card = [i for i in br.model.items if i["kind"] == "card"][-1]["card"]
            br.confirm(card["id"], False)
            wait_idle(lambda: [i for i in br.model.items if i["kind"] == "card"][-1]["card"]["state"] == "done")
            save(f"04-fatto-annulla-{theme}")
            br.undo(card["id"])
            wait_idle(lambda: [i for i in br.model.items if i["kind"] == "card"][-1]["card"]["state"] == "undone")
        def s_find(theme=theme):
            br.newChat()
            br.send("trova i pdf fattura")
            wait_idle(lambda: any(i["kind"] == "card" and i["card"].get("state") == "done" for i in br.model.items))
            save(f"05-file-trovati-{theme}")
        def s_danger(theme=theme):
            br.newChat()
            br.agent.run_intent("run_command", {"command": "rm -rf ~/Download/vecchi",
                                                "explanation": "Elimina la cartella «vecchi» in Download."})
            wait_idle(lambda: any(i["kind"] == "card" for i in br.model.items))
            save(f"06-comando-pericoloso-{theme}")
        def s_setup(theme=theme):
            br.newChat()
            cfg(provider="ollama", ollama={"base_url": "http://127.0.0.1:9", "model": "qwen2.5:1.5b"})
            br.refreshStatus()
            br.send("Che tempo fa domani a Roma?")
            wait_idle(lambda: any(i["kind"] == "setup" for i in br.model.items))
            save(f"07-configura-locale-{theme}")
        steps += [s_empty, s_answer, s_move, s_find, s_danger, s_setup]

    def s_setup_progress():
        br._install = {"active": True, "phase": "pull", "done": 412_000_000, "total": 986_000_000,
                       "message": i18n.T("install_pull", done="0,4 GB", total="1,0 GB"), "error": ""}
        br.installChanged.emit()
        wait_idle(lambda: True, 0.3)
        save("08-download-modello-dark")
        br._install = {"active": False, "phase": "", "done": 0, "total": 0, "message": "", "error": ""}
        br.installChanged.emit()

    def s_offline_intent():
        br.newChat()
        br.send("volume al 30")
        wait_idle(lambda: any(i["kind"] == "card" for i in br.model.items))
        save("09-comando-senza-modello-dark")

    def s_english():
        set_lang("en")
        scene("light")
        cfg(provider="ollama", ollama={"base_url": url, "model": "qwen2.5:1.5b"}, language="en")
        br.refreshStatus()
        fake_context()
        wait_idle(lambda: br.status.get("state") == "ready")
        save("10-english-empty-light")
        set_lang("it")
        cfg(provider="ollama", ollama={"base_url": url, "model": "qwen2.5:1.5b"})

    steps += [s_setup_progress, s_offline_intent, s_english]

    rc = 0
    try:
        for s in steps:
            s()
    except Exception as e:
        import traceback
        traceback.print_exc()
        rc = 1
    srv.shutdown()
    report = {"screenshots": shots, "qml_warnings": errors}
    with open(os.path.join(outdir, "shots.json"), "w") as fh:
        json.dump(report, fh, indent=1, ensure_ascii=False)
    print(json.dumps(report, indent=1, ensure_ascii=False))
    return rc if not errors else 2
