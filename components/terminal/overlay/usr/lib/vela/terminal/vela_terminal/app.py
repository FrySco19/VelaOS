"""Vela AI Terminal — main window."""
from __future__ import annotations

import argparse
import json
import os
import shutil
import sys
import urllib.request

from PySide6.QtCore import QEvent, QFileSystemWatcher, QProcess, Qt, QTimer
from PySide6.QtGui import QAction, QIcon, QKeySequence, QShortcut
from PySide6.QtWidgets import (QApplication, QDialog, QFrame, QHBoxLayout, QLabel, QListWidget,
                               QListWidgetItem, QLineEdit, QMainWindow, QMenu, QMessageBox, QPlainTextEdit,
                               QPushButton, QScrollArea, QSplitter, QTabWidget, QToolButton, QVBoxLayout,
                               QWidget, QCheckBox)

from . import aiconfig, theme, confirm
from .cards import (CommandItem, InfoCard, OutputBlock, PlanCard, ProposalCard, Task, ThinkingCard,
                    UserBubble, label)
from .danger import classify, DANGER
from .history import History
from .i18n import tr
from .llm import LLM, LLMError, parse_json, propose_messages, parse_proposal, agent_messages, parse_agent, \
    explain_messages, parse_explain
from .style import qss
from .termwidget import TerminalWidget

LIB = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
BASHRC = os.path.join(LIB, "shell", "bashrc")
MAX_AGENT_STEPS = 15


class Composer(QFrame):
    def __init__(self, on_submit, parent=None):
        super().__init__(parent)
        self.setObjectName("composer")
        self.on_submit = on_submit
        lay = QHBoxLayout(self)
        lay.setContentsMargins(12, 8, 8, 8)
        lay.setSpacing(8)
        self.edit = QPlainTextEdit()
        self.edit.setObjectName("composerEdit")
        self.edit.setPlaceholderText(tr("composer_placeholder"))
        self.edit.setTabChangesFocus(True)
        self.edit.setFixedHeight(46)
        self.edit.installEventFilter(self)
        self.edit.textChanged.connect(self._grow)
        self.send = QPushButton("↑")
        self.send.setObjectName("send")
        self.send.setToolTip(tr("run"))
        self.send.clicked.connect(self._submit)
        lay.addWidget(self.edit, 1)
        lay.addWidget(self.send, 0, Qt.AlignmentFlag.AlignBottom)

    def _grow(self):
        lines = max(1, min(5, self.edit.document().lineCount()))
        self.edit.setFixedHeight(int(22 * lines + 24))

    def eventFilter(self, obj, e):
        if obj is self.edit and e.type() == QEvent.Type.KeyPress:
            if e.key() in (Qt.Key.Key_Return, Qt.Key.Key_Enter) and not (e.modifiers() & Qt.KeyboardModifier.ShiftModifier):
                self._submit()
                return True
        if obj is self.edit and e.type() in (QEvent.Type.FocusIn, QEvent.Type.FocusOut):
            self.setProperty("focus", e.type() == QEvent.Type.FocusIn)
            self.style().unpolish(self)
            self.style().polish(self)
        return super().eventFilter(obj, e)

    def _submit(self):
        t = self.edit.toPlainText().strip()
        if t:
            self.edit.clear()
            self.on_submit(t)

    def set_text(self, t):
        self.edit.setPlainText(t)
        self.edit.setFocus()


class HistoryDialog(QDialog):
    def __init__(self, hist: History, on_reuse, parent=None):
        super().__init__(parent)
        self.setWindowTitle(tr("history"))
        self.resize(640, 520)
        self.hist = hist
        self.on_reuse = on_reuse
        lay = QVBoxLayout(self)
        lay.setContentsMargins(16, 16, 16, 16)
        self.q = QLineEdit()
        self.q.setPlaceholderText(tr("history_search"))
        self.q.setClearButtonEnabled(True)
        self.q.textChanged.connect(self.refresh)
        lay.addWidget(self.q)
        self.list = QListWidget()
        self.list.itemDoubleClicked.connect(self._reuse)
        lay.addWidget(self.list, 1)
        row = QHBoxLayout()
        clear = QPushButton(tr("history_clear"))
        clear.clicked.connect(self._clear)
        reuse = QPushButton(tr("history_reuse"))
        reuse.setObjectName("primary")
        reuse.clicked.connect(lambda: self._reuse(self.list.currentItem()))
        row.addWidget(clear)
        row.addStretch(1)
        row.addWidget(reuse)
        lay.addLayout(row)
        self.refresh()

    def refresh(self):
        import time
        self.list.clear()
        rows = self.hist.search(self.q.text(), 100)
        if not rows:
            it = QListWidgetItem(tr("history_empty"))
            it.setFlags(Qt.ItemFlag.NoItemFlags)
            self.list.addItem(it)
            return
        for r in rows:
            when = time.strftime("%d/%m %H:%M", time.localtime(r["ts"]))
            status = "" if r["rc"] is None else ("  ✓" if r["rc"] == 0 else f"  ✗ {r['rc']}")
            cmds = "\n".join("  $ " + c for c in r["commands"][:4])
            it = QListWidgetItem(f"{r['intent']}\n{cmds}\n  {when}{status}")
            it.setData(Qt.ItemDataRole.UserRole, r)
            self.list.addItem(it)

    def _reuse(self, item):
        if item and item.data(Qt.ItemDataRole.UserRole):
            self.on_reuse(item.data(Qt.ItemDataRole.UserRole))
            self.accept()

    def _clear(self):
        if QMessageBox.question(self, tr("history_clear"), tr("history_clear_q")) == QMessageBox.StandardButton.Yes:
            self.hist.clear()
            self.refresh()


class AgentRun:
    def __init__(self, goal, card, term):
        self.goal = goal
        self.card = card
        self.term = term
        self.steps = []
        self.plan = []
        self.stopped = False
        self.current_item = None


class MainWindow(QMainWindow):
    def __init__(self, args):
        super().__init__()
        self.args = args
        self.setWindowTitle(tr("app_name"))
        self.setWindowIcon(QIcon.fromTheme("vela-terminal-ai", QIcon.fromTheme("utilities-terminal")))
        self.resize(1240, 780)
        self.dark = theme.is_dark()
        self.colors = theme.colors(self.dark)
        self.term_pal = theme.TERM["dark" if self.dark else "light"]
        self.hist = History()
        self.pending = {}        # terminal -> dict(block, item, cb, started, hist_id)
        self.agent = None
        self.fail_chip = None
        self.ai_state = ("checking", "")
        root = QWidget()
        root.setObjectName("root")
        self.setCentralWidget(root)
        outer = QVBoxLayout(root)
        outer.setContentsMargins(0, 0, 0, 0)
        outer.setSpacing(0)
        self.split = QSplitter(Qt.Orientation.Horizontal)
        self.split.setChildrenCollapsible(False)
        outer.addWidget(self.split)
        # left: terminals
        self.tabs = QTabWidget()
        self.tabs.setDocumentMode(True)
        self.tabs.setTabsClosable(True)
        self.tabs.setMovable(True)
        self.tabs.tabCloseRequested.connect(self.close_tab)
        self.tabs.currentChanged.connect(lambda _i: self._update_title())
        corner = QWidget()
        cl = QHBoxLayout(corner)
        cl.setContentsMargins(0, 0, 6, 0)
        cl.setSpacing(2)
        b_new = QToolButton()
        b_new.setText("+")
        b_new.setToolTip(tr("new_tab") + " (Ctrl+Maiusc+T)")
        b_new.clicked.connect(lambda: self.new_tab())
        self.b_panel = QToolButton()
        self.b_panel.setText("✦ " + tr("panel_title"))
        self.b_panel.setCheckable(True)
        self.b_panel.setChecked(True)
        self.b_panel.setToolTip(tr("toggle_panel") + " (Ctrl+Maiusc+I)")
        self.b_panel.toggled.connect(self.toggle_panel)
        cl.addWidget(b_new)
        cl.addWidget(self.b_panel)
        self.tabs.setCornerWidget(corner, Qt.Corner.TopRightCorner)
        self.split.addWidget(self.tabs)
        # right: AI panel
        self.panel = self._build_panel()
        self.split.addWidget(self.panel)
        self.split.setStretchFactor(0, 3)
        self.split.setStretchFactor(1, 2)
        self.split.setSizes([740, 500])
        self.setStyleSheet(qss(self.colors, self.dark))
        self._shortcuts()
        self.new_tab(args.workdir)
        if args.no_panel:
            self.b_panel.setChecked(False)
        # config watch
        self.watcher = QFileSystemWatcher(self)
        cfgdir = os.path.join(os.environ.get("XDG_CONFIG_HOME") or os.path.expanduser("~/.config"), "vela")
        os.makedirs(cfgdir, exist_ok=True)
        self.watcher.addPath(cfgdir)
        self.watcher.directoryChanged.connect(lambda _p: QTimer.singleShot(300, self.check_ai))
        QTimer.singleShot(50, self.check_ai)
        QTimer.singleShot(200, self._initial_actions)

    # ------------------------------------------------------------------ layout
    def _build_panel(self):
        panel = QWidget()
        panel.setObjectName("panel")
        panel.setMinimumWidth(360)
        lay = QVBoxLayout(panel)
        lay.setContentsMargins(14, 10, 14, 12)
        lay.setSpacing(10)
        head = QHBoxLayout()
        title = QLabel(tr("panel_title"))
        title.setObjectName("panelTitle")
        self.pill = QLabel(tr("status_checking"))
        self.pill.setObjectName("pill")
        hist_b = QToolButton()
        ic = QIcon.fromTheme("document-open-recent", QIcon.fromTheme("view-history"))
        if ic.isNull():
            hist_b.setText(tr("history"))
        else:
            hist_b.setIcon(ic)
        hist_b.setToolTip(tr("history") + " (Ctrl+Maiusc+H)")
        hist_b.clicked.connect(self.show_history)
        menu_b = QToolButton()
        ic = QIcon.fromTheme("open-menu-symbolic", QIcon.fromTheme("application-menu"))
        if ic.isNull():
            menu_b.setText("•••")
        else:
            menu_b.setIcon(ic)
        menu_b.setStyleSheet("QToolButton::menu-indicator{image:none;width:0}")
        menu_b.setToolTip(tr("menu"))
        menu_b.setPopupMode(QToolButton.ToolButtonPopupMode.InstantPopup)
        m = QMenu(menu_b)
        m.addAction(tr("open_settings"), self.open_settings)
        m.addAction(tr("history"), self.show_history)
        m.addSeparator()
        m.addAction(tr("zoom_in"), lambda: self.term().zoom(1))
        m.addAction(tr("zoom_out"), lambda: self.term().zoom(-1))
        m.addSeparator()
        m.addAction(tr("about"), lambda: QMessageBox.about(self, tr("app_name"), tr("about_text")))
        menu_b.setMenu(m)
        head.addWidget(title)
        head.addWidget(self.pill)
        head.addStretch(1)
        head.addWidget(hist_b)
        head.addWidget(menu_b)
        lay.addLayout(head)
        self.scroll = QScrollArea()
        self.scroll.setWidgetResizable(True)
        self.scroll.setHorizontalScrollBarPolicy(Qt.ScrollBarPolicy.ScrollBarAlwaysOff)
        inner = QWidget()
        inner.setObjectName("feedInner")
        self.feed = QVBoxLayout(inner)
        self.feed.setContentsMargins(0, 0, 4, 0)
        self.feed.setSpacing(10)
        self.feed.addStretch(1)
        self.scroll.setWidget(inner)
        self.scroll.verticalScrollBar().rangeChanged.connect(self._autoscroll)
        lay.addWidget(self.scroll, 1)
        self._welcome()
        self.agent_box = QCheckBox(tr("agent_mode"))
        self.agent_box.setToolTip(tr("agent_tip"))
        self.composer = Composer(self.submit)
        lay.addWidget(self.composer)
        row = QHBoxLayout()
        row.addWidget(self.agent_box)
        row.addStretch(1)
        hint = QLabel(tr("composer_hint"))
        hint.setObjectName("hint")
        hint.setWordWrap(True)
        lay.addLayout(row)
        lay.addWidget(hint)
        return panel

    def _welcome(self):
        w = QFrame()
        w.setObjectName("welcome")
        l = QVBoxLayout(w)
        l.setContentsMargins(4, 12, 4, 4)
        l.setSpacing(8)
        l.addWidget(label(tr("welcome_title"), "welcomeTitle"))
        l.addWidget(label(tr("welcome_body"), "muted"))
        l.addSpacing(6)
        l.addWidget(label(tr("examples"), "cardTitle"))
        for k in ("ex1", "ex2", "ex3"):
            b = QPushButton(tr(k))
            b.setStyleSheet("text-align:left; padding:8px 12px")
            b.clicked.connect(lambda _c=False, t=tr(k): self.composer.set_text(t))
            l.addWidget(b)
        self.welcome = w
        self.add_card(w)

    def add_card(self, w):
        self.feed.insertWidget(self.feed.count() - 1, w)
        return w

    def insert_after(self, ref, w):
        idx = self.feed.indexOf(ref)
        if idx < 0:
            return self.add_card(w)
        self.feed.insertWidget(idx + 1, w)
        return w

    def remove_card(self, w):
        if w is not None:
            self.feed.removeWidget(w)
            w.deleteLater()

    def _autoscroll(self, _mn, mx):
        # show the newest card (word-wrapped labels can over-estimate the content height)
        last = None
        for i in range(self.feed.count() - 1, -1, -1):
            w = self.feed.itemAt(i).widget()
            if w is not None and w.isVisible():
                last = w
                break
        if last is not None:
            QTimer.singleShot(0, lambda w=last: self.scroll.ensureWidgetVisible(w, 0, 0) if w is not None else None)
        else:
            self.scroll.verticalScrollBar().setValue(mx)

    def _shortcuts(self):
        def sc(keys, fn):
            s = QShortcut(QKeySequence(keys), self)
            s.setContext(Qt.ShortcutContext.WindowShortcut)
            s.activated.connect(fn)
        sc("Ctrl+Shift+T", lambda: self.new_tab())
        sc("Ctrl+Shift+W", lambda: self.close_tab(self.tabs.currentIndex()))
        sc("Ctrl+Shift+I", self.focus_composer)
        sc("Ctrl+Shift+H", self.show_history)
        sc("Ctrl+PgDown", lambda: self.tabs.setCurrentIndex((self.tabs.currentIndex() + 1) % self.tabs.count()))
        sc("Ctrl+PgUp", lambda: self.tabs.setCurrentIndex((self.tabs.currentIndex() - 1) % self.tabs.count()))
        sc("Ctrl+Shift+N", lambda: QProcess.startDetached(sys.executable, ["-m", "vela_terminal.app"]))

    def focus_composer(self):
        if not self.b_panel.isChecked():
            self.b_panel.setChecked(True)
        self.composer.edit.setFocus()

    def toggle_panel(self, on):
        self.panel.setVisible(on)
        if not on:
            self.term().setFocus()

    # ------------------------------------------------------------------ terminals
    def term(self) -> TerminalWidget:
        return self.tabs.currentWidget()

    def new_tab(self, cwd=None):
        cwd = cwd or (self.term().foreground_cwd() if self.tabs.count() else os.getcwd() if os.path.isdir(os.getcwd()) else os.path.expanduser("~"))
        if not os.path.isdir(cwd or ""):
            cwd = os.path.expanduser("~")
        argv = ["bash", "--rcfile", BASHRC, "-i"]
        t = TerminalWidget(argv, cwd=cwd, palette=self.term_pal)
        t.commandStarted.connect(lambda t=t: self._started(t))
        t.commandFinished.connect(lambda rc, out, t=t: self._finished(t, rc, out))
        t.cwdChanged.connect(lambda _p: self._update_title())
        t.titleChanged.connect(lambda _p: self._update_title())
        t.processExited.connect(lambda rc, t=t: self._exited(t, rc))
        t.askAi.connect(lambda text: (self.focus_composer(), self.composer.set_text(text + "\n\n")))
        i = self.tabs.addTab(t, os.path.basename(cwd) or "/")
        self.tabs.setCurrentIndex(i)
        t.start()
        t.setFocus()
        return t

    def close_tab(self, i):
        w = self.tabs.widget(i)
        if w is None:
            return
        w.close_pty()
        self.pending.pop(w, None)
        self.tabs.removeTab(i)
        w.deleteLater()
        if self.tabs.count() == 0:
            self.close()

    def _exited(self, t, rc):
        i = self.tabs.indexOf(t)
        if i >= 0:
            self.close_tab(i)

    def _update_title(self):
        t = self.term()
        if not t:
            return
        cwd = t.current_cwd.replace(os.path.expanduser("~"), "~", 1)
        self.tabs.setTabText(self.tabs.currentIndex(), os.path.basename(t.current_cwd.rstrip("/")) or "/")
        self.setWindowTitle(f"{cwd} — {tr('app_name')}")

    def cwd(self):
        t = self.term()
        return t.foreground_cwd() if t else os.path.expanduser("~")

    # ------------------------------------------------------------------ AI status
    def check_ai(self):
        cfg = aiconfig.load()
        ok, why, ep = aiconfig.ai_status(cfg)
        if not ok:
            self._set_ai_state(why, "")
            return
        self.pill.setText(tr("status_checking"))

        def probe():
            if ep["provider"] == "ollama":
                with urllib.request.urlopen(ep["native_url"] + "/api/tags", timeout=4) as r:
                    names = [m["name"] for m in json.load(r).get("models", [])]
                model = ep.get("model") or (names[0] if names else "")
                if not names:
                    return ("no_model", "")
                if model not in names and model + ":latest" not in names:
                    return ("model_missing", model)
                return ("ok", model + " · " + ("locale" if aiconfig.language() == "it" else "local"))
            return ("ok", (ep.get("model") or "?") + (" · cloud" if ep.get("cloud") else " · locale"))

        Task(probe, on_ok=lambda r: self._set_ai_state(*r),
             on_err=lambda e: self._set_ai_state("unreachable", str(getattr(e, "reason", e)))).start()

    def _set_ai_state(self, state, detail):
        self.ai_state = (state, detail)
        text = {"ok": detail, "disabled": tr("status_off"), "unreachable": tr("status_unreachable")}.get(state, tr("status_none"))
        self.pill.setText(text)
        self.pill.setProperty("ok", state == "ok")
        self.pill.style().unpolish(self.pill)
        self.pill.style().polish(self.pill)
        self.pill.setToolTip(self._human_detail(detail) if state != "ok" else "")

    def ai_ready(self):
        return self.ai_state[0] == "ok"

    @staticmethod
    def _human_detail(detail):
        d = str(detail)
        low = d.lower()
        if "refused" in low:
            return tr("det_refused")
        if "timed out" in low or "timeout" in low:
            return tr("det_timeout")
        if "name or service" in low or "resolution" in low or "nodename" in low:
            return tr("det_dns")
        if "unreachable" in low:
            return tr("det_net")
        return d[:120]

    def setup_card(self, code, detail=""):
        detail = self._human_detail(detail) if code in ("unreachable", "http") else detail
        msg = {"disabled": tr("setup_disabled"), "cloud_not_allowed": tr("setup_cloud"), "no_key": tr("setup_nokey"),
               "no_model": tr("setup_nomodel"), "unreachable": tr("setup_unreachable").format(detail=detail or "-"),
               "model_missing": tr("setup_missing_model").format(model=detail or "?"), "auth": tr("setup_auth"),
               "run_never": tr("setup_run_never"), "bad_response": tr("setup_bad"), "not_allowed": tr("setup_nomodel"),
               "http": tr("setup_unreachable").format(detail=detail[:120])}.get(code, tr("setup_nomodel"))
        actions = [(tr("open_settings"), self.open_settings), (tr("retry"), self.check_ai)]
        return self.add_card(InfoCard(tr("setup_title"), [msg, tr("setup_still_works")], actions))

    def open_settings(self):
        cmd = aiconfig.settings_command()
        if cmd:
            QProcess.startDetached(cmd[0], cmd[1:])
        else:
            QMessageBox.information(self, tr("app_name"), tr("setup_nomodel"))

    def _llm_error(self, e, thinking=None):
        self.remove_card(thinking)
        if isinstance(e, LLMError):
            code, detail = e.code, e.detail
            if code == "not_allowed":
                code = detail
        else:
            code, detail = "unreachable", str(e)
        if code in ("unreachable", "model_missing", "no_model"):
            self._set_ai_state(code if code != "no_model" else "no_model", detail)
        self.setup_card(code, detail)

    # ------------------------------------------------------------------ submit
    def submit(self, text):
        if self.welcome is not None:
            self.remove_card(self.welcome)
            self.welcome = None
        if text.startswith("$"):
            cmd = text[1:].strip()
            if cmd:
                self.add_card(UserBubble(text))
                card = ProposalCard("", [{"command": cmd}], source="manual", cwd_fn=self.cwd)
                self._wire_proposal(card, intent="", source="manual")
                self.add_card(card)
            return
        self.add_card(UserBubble(text))
        cfg = aiconfig.load()
        ok, why, _ep = aiconfig.ai_status(cfg)
        if not ok:
            self.setup_card(why)
            return
        if self.agent_box.isChecked():
            self.start_agent(text)
            return
        thinking = self.add_card(ThinkingCard(tr("thinking")))
        cwd = self.cwd()
        lang = aiconfig.language(cfg)
        recent = [h for h in self.hist.recent_intents(4) if h.get("intent")]

        def work():
            llm = LLM(cfg)
            return parse_proposal(parse_json(llm.chat(propose_messages(text, cwd, lang, recent))))

        Task(work, on_ok=lambda p: self._got_proposal(text, p, thinking),
             on_err=lambda e: self._llm_error(e, thinking)).start()

    def _got_proposal(self, intent, p, thinking):
        self.remove_card(thinking)
        if self.ai_state[0] != "ok":
            self.check_ai()
        card = ProposalCard(p["explanation"], p["commands"], p["note"], source="ai", cwd_fn=self.cwd)
        hid = self.hist.add(intent, [c["command"] for c in p["commands"]], self.cwd(), None, "ask", p["explanation"])
        self._wire_proposal(card, intent, "ai", hid)
        self.add_card(card)

    def _wire_proposal(self, card, intent, source, hid=None):
        for it in card.items:
            it.runRequested.connect(lambda item, c=card: self.run_item(item, c, intent, source, hid))
        if hasattr(card, "run_all_btn"):
            card.runAllRequested.connect(lambda c: self._run_all(c, intent, source, hid))

    def _run_all(self, card, intent, source, hid):
        queue = [i for i in card.items if i.state == "idle"]
        card.run_all_btn.setEnabled(False)

        def nxt(_rc=None, _out=None):
            if not queue:
                return
            it = queue.pop(0)
            self.run_item(it, card, intent, source, hid, cb=lambda rc, out: nxt() if rc == 0 else None)
        nxt()

    # ------------------------------------------------------------------ execution
    def run_item(self, item, anchor, intent="", source="ai", hid=None, cb=None):
        cmd = item.command.strip()
        if not cmd:
            return
        t = self.term()
        if source != "manual" and aiconfig.run_permission() == "never":
            item.show_message(tr("setup_run_never"))
            return
        a = classify(cmd)
        if a.level == DANGER:
            pv = (item.preview_data or {}).get("text", "")
            if not confirm.ask(self, cmd, a.to_dict(), pv, "terminal"):
                aiconfig.log_action("run_command", cmd, confirmed=False, result="declined")
                return
        if t.busy() or t in self.pending:
            item.show_message(tr("busy"))
            return
        block = OutputBlock(cmd)
        block.explainRequested.connect(self.explain)
        if isinstance(anchor, PlanCard):
            anchor.add_widget(block)
        elif anchor is not None:
            self.insert_after(anchor, block)
        else:
            self.add_card(block)
        item.set_state("running", tr("running"))
        self.pending[t] = {"block": block, "item": item, "cb": cb, "started": False, "hid": hid, "intent": intent}
        if not t.run_command(cmd):
            self.pending.pop(t, None)
            item.set_state("idle")
            item.show_message(tr("busy"))
            self.remove_card(block)
            return
        if source != "manual":
            aiconfig.log_action("run_command", (intent + " → " if intent else "") + cmd, confirmed=True, result="started")
        t.setFocus()  # the user may need to type (e.g. sudo password)

    def _started(self, t):
        p = self.pending.get(t)
        if p:
            p["started"] = True

    def _finished(self, t, rc, out):
        p = self.pending.get(t)
        if p and p["started"]:
            del self.pending[t]
            p["block"].finish(rc, out, can_explain=self.ai_ready())
            p["item"].set_state("done" if rc == 0 else "failed", "")
            if p["hid"]:
                self.hist.update_rc(p["hid"], rc)
            if p["cb"]:
                p["cb"](rc, out)
            return
        # a command the user typed directly
        if self.fail_chip is not None:
            self.remove_card(self.fail_chip)
            self.fail_chip = None
        if rc not in (0, 130, 141, 143, 148) and out.strip() and self.ai_ready() and self.panel.isVisible():
            last = self._last_command_line(t)
            blk = OutputBlock(last or "?")
            blk.finish(rc, out, can_explain=True)
            blk.explainRequested.connect(self.explain)
            blk.setToolTip(tr("last_failed").format(cmd=last, rc=rc))
            self.fail_chip = self.add_card(blk)

    def _last_command_line(self, t):
        try:
            import subprocess
            r = subprocess.run(["bash", "-c", "tail -n 1 ~/.bash_history"], capture_output=True, text=True, timeout=1)
            return r.stdout.strip()
        except Exception:
            return ""

    # ------------------------------------------------------------------ explain errors
    def explain(self, block):
        if not self.ai_ready():
            self.setup_card(self.ai_state[0], self.ai_state[1])
            return
        block.explain_btn.setEnabled(False)
        thinking = self.insert_after(block, ThinkingCard(tr("explaining")))
        cfg = aiconfig.load()
        cwd = self.cwd()
        lang = aiconfig.language(cfg)

        def work():
            return parse_explain(parse_json(LLM(cfg).chat(explain_messages(block.command, block.rc, block.output, cwd, lang))))

        def ok(r):
            self.remove_card(thinking)
            cmds = [r["fix"]] if r["fix"] else []
            text = r["summary"] + ((" " + r["cause"]) if r["cause"] else "")
            card = ProposalCard(text, cmds, source="ai", cwd_fn=self.cwd)
            self._wire_proposal(card, "spiega errore: " + block.command, "ai")
            self.insert_after(block, card)

        Task(work, on_ok=ok, on_err=lambda e: self._llm_error(e, thinking)).start()

    # ------------------------------------------------------------------ agent
    def start_agent(self, goal):
        if self.agent and not self.agent.stopped:
            self.stop_agent()
        card = PlanCard(goal)
        self.add_card(card)
        self.agent = AgentRun(goal, card, self.term())
        card.stopRequested.connect(self.stop_agent)
        self._agent_next()

    def _agent_next(self):
        ag = self.agent
        if not ag or ag.stopped:
            return
        if len(ag.steps) >= MAX_AGENT_STEPS:
            ag.stopped = True
            ag.card.finish(tr("agent_limit").format(n=MAX_AGENT_STEPS), tr("agent_stopped"))
            return
        thinking = ThinkingCard(tr("thinking_next"))
        ag.card.add_widget(thinking)
        cfg = aiconfig.load()
        cwd = self.cwd()
        lang = aiconfig.language(cfg)
        steps = list(ag.steps)

        def work():
            return parse_agent(parse_json(LLM(cfg).chat(agent_messages(ag.goal, cwd, lang, steps))))

        def ok(r, ag=ag):
            thinking.deleteLater()
            if ag.stopped:
                return
            ag.plan = r["plan"] or ag.plan
            ag.card.set_plan(ag.plan, len(ag.steps))
            if r["done"] or not r["next"]:
                ag.stopped = True
                ag.card.finish(r["message"], tr("agent_done"))
                self.hist.add(ag.goal, [s["command"] for s in ag.steps], cwd, 0, "agent", r["message"])
                return
            item = CommandItem(r["next"]["command"], r["next"]["explanation"], self.cwd, allow_skip=True)
            ag.current_item = item
            item.runRequested.connect(lambda it: self.run_item(it, ag.card, ag.goal, "ai", None,
                                                               cb=lambda rc, out: self._agent_step_done(it, rc, out)))
            item.skipRequested.connect(lambda it: self._agent_skip(it))
            ag.card.add_widget(item)

        def err(e, ag=ag):
            thinking.deleteLater()
            ag.stopped = True
            ag.card.finish("", tr("agent_stopped"))
            self._llm_error(e, None)

        Task(work, on_ok=ok, on_err=err).start()

    def _agent_step_done(self, item, rc, out):
        ag = self.agent
        if not ag:
            return
        # move the output block inside the plan card for readability
        ag.steps.append({"command": item.command, "rc": rc, "output": out})
        if not ag.stopped:
            self._agent_next()

    def _agent_skip(self, item):
        ag = self.agent
        if not ag or ag.stopped:
            return
        item.set_state("skipped", tr("skip"))
        ag.steps.append({"command": item.command, "skipped": True, "output": ""})
        self._agent_next()

    def stop_agent(self):
        ag = self.agent
        if not ag or ag.stopped:
            return
        ag.stopped = True
        if ag.term in self.pending:
            ag.term.interrupt()
        if ag.current_item and ag.current_item.state == "idle":
            ag.current_item.set_state("skipped", "")
        ag.card.finish("", tr("agent_stopped"))

    # ------------------------------------------------------------------ history
    def show_history(self):
        HistoryDialog(self.hist, self._reuse_history, self).exec()

    def _reuse_history(self, row):
        if self.welcome is not None:
            self.remove_card(self.welcome)
            self.welcome = None
        self.add_card(UserBubble(row["intent"]))
        card = ProposalCard(row.get("summary") or "", [{"command": c} for c in row["commands"]], source="ai", cwd_fn=self.cwd)
        self._wire_proposal(card, row["intent"], "ai")
        self.add_card(card)
        self.focus_composer()

    # ------------------------------------------------------------------ startup actions
    def _initial_actions(self):
        a = self.args
        if a.propose:
            if self.welcome is not None:
                self.remove_card(self.welcome)
                self.welcome = None
            card = ProposalCard(a.explanation or "", [{"command": a.propose}], source="ai", cwd_fn=self.cwd)
            self._wire_proposal(card, a.explanation or "", "ai")
            self.add_card(card)
        if a.intent:
            self.submit(a.intent)
        if a.agent:
            self.agent_box.setChecked(True)
            self.submit(a.agent)

    def closeEvent(self, e):
        for i in range(self.tabs.count()):
            self.tabs.widget(i).close_pty()
        super().closeEvent(e)


def main(argv=None):
    ap = argparse.ArgumentParser(prog="vela-ai-terminal", description=tr("about_text"))
    ap.add_argument("--workdir", "-w", help="cartella iniziale")
    ap.add_argument("--propose", help="propone questo comando (eseguito solo dopo conferma)")
    ap.add_argument("--explanation", help="spiegazione da mostrare con --propose")
    ap.add_argument("--intent", help="invia subito questa richiesta all'assistente")
    ap.add_argument("--agent", help="avvia la modalità agente con questo obiettivo")
    ap.add_argument("--no-panel", action="store_true", help="nasconde il pannello dell'assistente")
    args = ap.parse_args(argv)
    if args.workdir:
        args.workdir = os.path.abspath(os.path.expanduser(args.workdir))
    app = QApplication.instance() or QApplication(sys.argv[:1])
    app.setApplicationName("vela-ai-terminal")
    app.setApplicationDisplayName(tr("app_name"))
    app.setDesktopFileName("vela-ai-terminal")
    app.setOrganizationName("Vela")
    w = MainWindow(args)
    w.show()
    return app.exec()


if __name__ == "__main__":
    sys.exit(main())
