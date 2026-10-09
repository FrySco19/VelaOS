"""GUI smoke/integration test for the Vela AI Terminal (mock LLM, real PTY + bash).

Run (isolated HOME, own X display):  components/terminal/tests/run_gui_smoke.sh
Screenshots go to $VT_ARTIFACTS (default /workspace/vela/artifacts/terminal).
"""
import json
import os
import shutil
import sys
import time
import traceback

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.normpath(os.path.join(HERE, "..", "overlay", "usr", "lib", "vela", "terminal")))
sys.path.insert(0, HERE)

import mock_llm  # noqa: E402

ART = os.environ.get("VT_ARTIFACTS", "/workspace/vela/artifacts/terminal")
os.makedirs(ART, exist_ok=True)
srv = mock_llm.start()
PORT = srv.server_address[1]
HOME = os.environ["HOME"]
cfgdir = os.path.join(os.environ.get("XDG_CONFIG_HOME", HOME + "/.config"), "vela")
os.makedirs(cfgdir, exist_ok=True)


def set_cfg(base_url, model="mock"):
    with open(os.path.join(cfgdir, "ai.json"), "w") as f:
        json.dump({"version": 1, "provider": "ollama", "ollama": {"base_url": base_url, "model": model}}, f)


set_cfg(f"http://127.0.0.1:{PORT}")

from PySide6.QtWidgets import QApplication  # noqa: E402
from vela_terminal import app as vapp, confirm, cards  # noqa: E402
from vela_terminal.danger import DANGER  # noqa: E402

qapp = QApplication(sys.argv[:1])
RESULTS = []


def pump(sec=0.05):
    end = time.monotonic() + sec
    while time.monotonic() < end:
        qapp.processEvents()
        time.sleep(0.01)


def wait(cond, timeout=15, what=""):
    end = time.monotonic() + timeout
    while time.monotonic() < end:
        qapp.processEvents()
        try:
            v = cond()
        except Exception:
            v = None
        if v:
            return v
        time.sleep(0.02)
    raise AssertionError("timeout waiting for " + what)


def shot(w, name):
    pump(0.3)
    w.grab().save(os.path.join(ART, name))


def check(name, fn):
    try:
        fn()
        RESULTS.append((name, "PASS", ""))
        print("PASS", name, flush=True)
    except Exception as e:  # noqa: BLE001
        RESULTS.append((name, "FAIL", repr(e)))
        print("FAIL", name, repr(e), flush=True)
        traceback.print_exc()


def feed_widgets(w, cls):
    out = []
    for i in range(w.feed.count()):
        it = w.feed.itemAt(i).widget()
        if isinstance(it, cls):
            out.append(it)
    return out


def all_widgets(w, cls):
    return w.panel.findChildren(cls)


class Args:
    workdir = HOME
    propose = None
    explanation = None
    intent = None
    agent = None
    no_panel = False


win = vapp.MainWindow(Args())
win.resize(1320, 820)
win.show()
term = win.term()


def screen_text():
    return "\n".join(term.screen.display)


def t_prompt():
    wait(lambda: term.at_prompt, 15, "first prompt")
    wait(lambda: win.ai_state[0] == "ok", 10, "AI status ok")
    shot(win, "gui-01-start.png")


def t_direct_typing():
    from PySide6.QtCore import Qt
    from PySide6.QtGui import QKeyEvent
    from PySide6.QtCore import QEvent
    term.setFocus()
    for ch in "echo VELA_$((40+2))":
        ev = QKeyEvent(QEvent.Type.KeyPress, 0, Qt.KeyboardModifier.NoModifier, ch)
        QApplication.sendEvent(term, ev)
    QApplication.sendEvent(term, QKeyEvent(QEvent.Type.KeyPress, Qt.Key.Key_Return, Qt.KeyboardModifier.NoModifier, "\r"))
    wait(lambda: "VELA_42" in screen_text(), 10, "typed command output")
    # clipboard: select line containing VELA_42 and copy with Ctrl+C (selection present => copy, not SIGINT)
    lines = term.screen.display
    y = [i for i, l in enumerate(lines) if l.strip() == "VELA_42"][0]
    abs_y = len(term.screen.scrollback) + y
    term.sel_start, term.sel_end = (abs_y, 0), (abs_y, 7)
    QApplication.sendEvent(term, QKeyEvent(QEvent.Type.KeyPress, Qt.Key.Key_C, Qt.KeyboardModifier.ControlModifier, "\x03"))
    pump(0.2)
    assert QApplication.clipboard().text() == "VELA_42", QApplication.clipboard().text()
    assert not term.has_selection()
    # paste with Ctrl+V into the shell
    QApplication.clipboard().setText("echo incollato_ok")
    QApplication.sendEvent(term, QKeyEvent(QEvent.Type.KeyPress, Qt.Key.Key_V, Qt.KeyboardModifier.ControlModifier, "\x16"))
    QApplication.sendEvent(term, QKeyEvent(QEvent.Type.KeyPress, Qt.Key.Key_Return, Qt.KeyboardModifier.NoModifier, "\r"))
    wait(lambda: screen_text().count("incollato_ok") >= 2, 10, "pasted command ran")
    shot(win, "gui-02-direct-typing.png")


def t_propose_and_run():
    win.submit("Quanto spazio libero ho sul disco?")
    card = wait(lambda: [c for c in feed_widgets(win, cards.ProposalCard) if c.items], 15, "proposal")[-1]
    assert len(card.items) == 2
    assert card.items[0].assessment.level == "safe"
    wait(lambda: card.items[0].preview_data, 10, "preview")
    shot(win, "gui-03-proposal.png")
    card.items[0].run_btn.click()
    blk = wait(lambda: [b for b in feed_widgets(win, cards.OutputBlock) if b.rc is not None], 20, "output block")[-1]
    assert blk.rc == 0, blk.rc
    assert "File system" in blk.output or "Filesystem" in blk.output, blk.output[:200]
    shot(win, "gui-04-output-block.png")


def t_danger_guard():
    os.makedirs(os.path.join(HOME, "prova_da_cancellare", "sub"), exist_ok=True)
    open(os.path.join(HOME, "prova_da_cancellare", "sub", "x.txt"), "w").write("x")
    win.submit("cancella tutto, è pericoloso")
    card = wait(lambda: [c for c in feed_widgets(win, cards.ProposalCard) if c.items and c.items[0].command.startswith("rm -rf")], 15, "danger proposal")[-1]
    item = card.items[0]
    assert item.assessment.level == DANGER
    wait(lambda: item.preview_data, 10, "danger preview")
    assert "Cancellerà definitivamente" in item.preview_data["text"], item.preview_data
    shot(win, "gui-05-danger-proposal.png")
    seen = {}
    real_ask = confirm.ask

    def deny(*a, **k):
        seen["asked"] = True
        return False
    confirm.ask = deny
    n_blocks = len(feed_widgets(win, cards.OutputBlock))
    item.run_btn.click()
    pump(0.5)
    assert seen.get("asked"), "danger dialog not shown"
    assert len(feed_widgets(win, cards.OutputBlock)) == n_blocks, "ran without confirmation!"
    assert os.path.isdir(os.path.join(HOME, "prova_da_cancellare"))
    confirm.ask = lambda *a, **k: True
    item.run_btn.click()
    wait(lambda: not os.path.exists(os.path.join(HOME, "prova_da_cancellare")), 10, "rm after confirm")
    confirm.ask = real_ask


def t_danger_dialog_widget():
    from vela_terminal.danger import classify
    a = classify("rm -rf ~/x").to_dict()
    d = confirm.DangerDialog("rm -rf ~/x", a, "Cancellerà definitivamente 1 elemento", "assistant", "it", win)
    d.show()
    pump(0.3)
    assert not d.ok_btn.isEnabled(), "run enabled before acknowledgement"
    d.grab().save(os.path.join(ART, "gui-06-danger-dialog.png"))
    d.ack.setChecked(True)
    assert d.ok_btn.isEnabled()
    d.reject()


def t_explain_error():
    win.submit("mostrami un errore")
    card = wait(lambda: [c for c in feed_widgets(win, cards.ProposalCard) if c.items and c.items[0].command.startswith("cat file_che")], 15, "error proposal")[-1]
    card.items[0].run_btn.click()
    blk = wait(lambda: [b for b in feed_widgets(win, cards.OutputBlock) if b.command.startswith("cat file_che") and b.rc is not None], 15, "failed block")[-1]
    assert blk.rc != 0
    assert blk.explain_btn.isVisible() or not blk.explain_btn.isHidden()
    blk.explain_btn.click()
    fix = wait(lambda: [c for c in feed_widgets(win, cards.ProposalCard) if c.items and c.items[0].command == "ls -la"], 15, "fix proposal")
    shot(win, "gui-07-explain-error.png")


def t_agent():
    win.agent_box.setChecked(True)
    win.submit("crea una nota di prova e verificala")
    plan = wait(lambda: feed_widgets(win, cards.PlanCard), 10, "plan card")[-1]
    for step in range(3):
        item = wait(lambda: [i for i in plan.findChildren(cards.CommandItem) if i.state == "idle"], 15, f"agent step {step}")[-1]
        if step == 1:
            shot(win, "gui-08-agent-running.png")
        item.run_btn.click()
        wait(lambda: item.state in ("done", "failed"), 15, f"agent step {step} done")
    wait(lambda: plan.final.isVisible() or not plan.final.isHidden(), 15, "agent final")
    assert "ciao" in plan.final.text(), plan.final.text()
    assert open(os.path.join(HOME, "prova_agente", "nota.txt")).read().strip() == "ciao"
    shot(win, "gui-09-agent-done.png")
    win.agent_box.setChecked(False)


def t_agent_stop():
    win.agent_box.setChecked(True)
    win.submit("crea una nota di prova e verificala")
    plan = wait(lambda: feed_widgets(win, cards.PlanCard), 10, "plan card")[-1]
    item = wait(lambda: [i for i in plan.findChildren(cards.CommandItem) if i.state == "idle"], 15, "first step")[-1]
    plan.stop_btn.click()
    pump(0.3)
    assert win.agent.stopped and item.state != "idle"
    win.agent_box.setChecked(False)


def t_manual_command_without_ai():
    set_cfg("http://127.0.0.1:9")  # nothing listens there
    win.check_ai()
    wait(lambda: win.ai_state[0] == "unreachable", 10, "unreachable state")
    win.submit("$ rm -rf ~/qualcosa")
    card = wait(lambda: [c for c in feed_widgets(win, cards.ProposalCard) if c.items and c.items[0].command == "rm -rf ~/qualcosa"], 5, "manual card")[-1]
    assert card.items[0].assessment.level == DANGER
    win.submit("trova i file grandi")
    info = wait(lambda: feed_widgets(win, cards.InfoCard), 15, "setup info card")[-1]
    texts = " ".join(l.text() for l in info.findChildren(cards.QLabel))
    assert "non risponde" in texts and "funziona normalmente" in texts, texts
    shot(win, "gui-10-no-model.png")
    # the terminal still works
    term.run_command("echo ANCORA_OK")
    wait(lambda: "ANCORA_OK" in screen_text(), 10, "terminal works without AI")
    set_cfg(f"http://127.0.0.1:{PORT}")


def t_history():
    rows = win.hist.search("spazio")
    assert rows and "spazio" in rows[0]["intent"].lower(), rows
    d = vapp.HistoryDialog(win.hist, lambda r: None, win)
    d.q.setText("nota")
    d.show()
    pump(0.3)
    d.grab().save(os.path.join(ART, "gui-11-history.png"))
    assert d.list.count() >= 1
    d.reject()


def t_fullscreen_app():
    # alternate screen: run a curses-like full-screen program and come back
    term.run_command("printf '\\033[?1049h\\033[2J\\033[HSCHERMO_ALTERNATIVO'; sleep 1; printf '\\033[?1049l'")
    wait(lambda: term.screen.alt and term.screen.display[0].startswith("SCHERMO_ALTERNATIVO"), 5, "alt screen")
    wait(lambda: term.at_prompt and not term.screen.alt and
         not any(l.startswith("SCHERMO_ALTERNATIVO") for l in term.screen.display), 8, "alt screen restored")
    assert any("printf" in l for l in term.screen.display), "main screen content not restored"


def t_log():
    log = os.path.join(os.environ.get("XDG_DATA_HOME", HOME + "/.local/share"), "vela", "ai-actions.jsonl")
    lines = [json.loads(l) for l in open(log)]
    assert any(l["action"] == "run_command" and l["app"] == "terminal" for l in lines), lines[-3:]


for name, fn in [("prompt+ai status", t_prompt), ("direct typing, Ctrl+C copy, Ctrl+V paste", t_direct_typing),
                 ("propose+run safe command", t_propose_and_run), ("danger guard (deny then confirm)", t_danger_guard),
                 ("danger dialog requires acknowledgement", t_danger_dialog_widget),
                 ("explain error + fix", t_explain_error), ("agent mode 3 steps", t_agent), ("agent stop", t_agent_stop),
                 ("no model: setup hint, manual $ commands, terminal works", t_manual_command_without_ai),
                 ("history search", t_history), ("alternate screen", t_fullscreen_app), ("action log", t_log)]:
    check(name, fn)

win.close()
pump(0.3)
srv.shutdown()
with open(os.path.join(ART, "gui_smoke_results.txt"), "w") as f:
    for r in RESULTS:
        f.write("\t".join(r) + "\n")
fails = [r for r in RESULTS if r[1] != "PASS"]
print(f"{len(RESULTS) - len(fails)}/{len(RESULTS)} passed")
sys.exit(1 if fails else 0)
