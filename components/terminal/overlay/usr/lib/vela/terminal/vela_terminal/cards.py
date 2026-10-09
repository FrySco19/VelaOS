"""Feed widgets for the AI panel: proposals, command items, output blocks, plans, info cards."""
from __future__ import annotations

from PySide6.QtCore import QObject, QRunnable, Qt, QThreadPool, QTimer, Signal
from PySide6.QtGui import QFont, QFontDatabase, QGuiApplication
from PySide6.QtWidgets import (QFrame, QHBoxLayout, QLabel, QLineEdit, QPlainTextEdit, QPushButton,
                               QSizePolicy, QToolButton, QVBoxLayout, QWidget)

from .danger import classify, SAFE, CHANGE, DANGER
from .i18n import tr
from . import preview as _preview


def mono(size=9.5):
    fams = set(QFontDatabase.families())
    fam = next((x for x in ("JetBrains Mono", "Noto Sans Mono", "DejaVu Sans Mono") if x in fams), "Monospace")
    f = QFont(fam)
    f.setStyleHint(QFont.StyleHint.Monospace)
    f.setPointSizeF(size)
    return f


class _Sig(QObject):
    ok = Signal(object)
    err = Signal(object)


class Task(QRunnable):
    """Run fn(*args) in the global thread pool; deliver result/exception on the GUI thread."""
    _live = set()

    def __init__(self, fn, *args, on_ok=None, on_err=None):
        super().__init__()
        self.fn, self.args = fn, args
        self.sig = _Sig()
        if on_ok:
            self.sig.ok.connect(on_ok)
        if on_err:
            self.sig.err.connect(on_err)
        self.setAutoDelete(False)

    def run(self):
        try:
            r = self.fn(*self.args)
        except Exception as e:  # noqa: BLE001
            self.sig.err.emit(e)
        else:
            self.sig.ok.emit(r)
        finally:
            Task._live.discard(self)

    def start(self):
        Task._live.add(self)
        QThreadPool.globalInstance().start(self)
        return self


def label(text, cls=None, wrap=True, selectable=True):
    l = QLabel(text)
    l.setWordWrap(wrap)
    if cls:
        l.setObjectName(cls)
    if selectable:
        l.setTextInteractionFlags(Qt.TextInteractionFlag.TextSelectableByMouse)
    l.setSizePolicy(QSizePolicy.Policy.Preferred, QSizePolicy.Policy.Minimum)
    return l


class Card(QFrame):
    def __init__(self, kind="card", parent=None):
        super().__init__(parent)
        self.setObjectName(kind)
        self.lay = QVBoxLayout(self)
        self.lay.setContentsMargins(14, 12, 14, 12)
        self.lay.setSpacing(8)


class UserBubble(QWidget):
    def __init__(self, text, parent=None):
        super().__init__(parent)
        row = QHBoxLayout(self)
        row.setContentsMargins(40, 0, 0, 0)
        row.addStretch(1)
        b = label(text, "userBubble")
        b.setMaximumWidth(520)
        row.addWidget(b)


class ThinkingCard(Card):
    def __init__(self, text, parent=None):
        super().__init__("thinking", parent)
        self.base = text
        self.l = label(text, "muted", selectable=False)
        self.lay.addWidget(self.l)
        self.n = 0
        self.t = QTimer(self)
        self.t.timeout.connect(self._tick)
        self.t.start(400)

    def _tick(self):
        self.n = (self.n + 1) % 4
        self.l.setText(self.base.rstrip("…") + "." * self.n)


class InfoCard(Card):
    def __init__(self, title, body, actions=(), kind="info", parent=None):
        super().__init__(kind, parent)
        t = label(title, "cardTitle")
        self.lay.addWidget(t)
        for para in [b for b in (body if isinstance(body, (list, tuple)) else [body]) if b]:
            self.lay.addWidget(label(para))
        if actions:
            row = QHBoxLayout()
            row.setSpacing(8)
            for text, cb in actions:
                btn = QPushButton(text)
                btn.clicked.connect(cb)
                row.addWidget(btn)
            row.addStretch(1)
            self.lay.addLayout(row)


class RiskChip(QLabel):
    def __init__(self, level, parent=None):
        super().__init__(parent)
        self.set_level(level)

    def set_level(self, level):
        self.setText({SAFE: tr("risk_safe"), CHANGE: tr("risk_change"), DANGER: tr("risk_danger")}[level])
        self.setObjectName({SAFE: "chipSafe", CHANGE: "chipChange", DANGER: "chipDanger"}[level])
        self.style().unpolish(self)
        self.style().polish(self)


class CommandItem(QFrame):
    runRequested = Signal(object)
    skipRequested = Signal(object)

    def __init__(self, command, explanation="", cwd_fn=None, allow_skip=False, parent=None):
        super().__init__(parent)
        self.setObjectName("cmdItem")
        self.cwd_fn = cwd_fn or (lambda: None)
        self.command = command
        self.state = "idle"
        lay = QVBoxLayout(self)
        lay.setContentsMargins(12, 10, 12, 10)
        lay.setSpacing(6)
        top = QHBoxLayout()
        top.setSpacing(8)
        self.cmd_label = QLabel(command)
        self.cmd_label.setObjectName("cmdText")
        self.cmd_label.setFont(mono(10))
        self.cmd_label.setWordWrap(True)
        self.cmd_label.setTextInteractionFlags(Qt.TextInteractionFlag.TextSelectableByMouse)
        self.cmd_edit = QLineEdit(command)
        self.cmd_edit.setFont(mono(10))
        self.cmd_edit.hide()
        self.cmd_edit.returnPressed.connect(self._finish_edit)
        self.chip = RiskChip(SAFE)
        top.addWidget(self.cmd_label, 1)
        top.addWidget(self.cmd_edit, 1)
        top.addWidget(self.chip, 0, Qt.AlignmentFlag.AlignTop)
        lay.addLayout(top)
        self.expl = label(explanation, "muted")
        self.expl.setVisible(bool(explanation))
        lay.addWidget(self.expl)
        self.reasons = label("", "reasons")
        lay.addWidget(self.reasons)
        self.pv_toggle = QToolButton()
        self.pv_toggle.setObjectName("linkButton")
        self.pv_toggle.setText("▸ " + tr("preview"))
        self.pv_toggle.setCheckable(True)
        self.pv_toggle.toggled.connect(self._toggle_preview)
        self.pv = QPlainTextEdit()
        self.pv.setObjectName("preview")
        self.pv.setReadOnly(True)
        self.pv.setFont(mono(9))
        self.pv.setLineWrapMode(QPlainTextEdit.LineWrapMode.NoWrap)
        self.pv.setMaximumHeight(200)
        self.pv.hide()
        lay.addWidget(self.pv_toggle, 0, Qt.AlignmentFlag.AlignLeft)
        lay.addWidget(self.pv)
        self.msg = label("", "inlineWarn")
        self.msg.hide()
        lay.addWidget(self.msg)
        row = QHBoxLayout()
        row.setSpacing(8)
        self.run_btn = QPushButton(tr("run"))
        self.run_btn.setObjectName("primary")
        self.run_btn.clicked.connect(lambda: self.runRequested.emit(self))
        self.edit_btn = QPushButton(tr("edit"))
        self.edit_btn.clicked.connect(self._toggle_edit)
        self.copy_btn = QPushButton(tr("copy"))
        self.copy_btn.clicked.connect(lambda: QGuiApplication.clipboard().setText(self.command))
        row.addWidget(self.run_btn)
        row.addWidget(self.edit_btn)
        row.addWidget(self.copy_btn)
        self.skip_btn = None
        if allow_skip:
            self.skip_btn = QPushButton(tr("skip"))
            self.skip_btn.clicked.connect(lambda: self.skipRequested.emit(self))
            row.addWidget(self.skip_btn)
        row.addStretch(1)
        self.state_label = label("", "muted", selectable=False)
        row.addWidget(self.state_label)
        lay.addLayout(row)
        self.preview_data = None
        self._assess()

    # ------------------------------------------------------------------
    def _assess(self):
        self.assessment = classify(self.command)
        lvl = self.assessment.level
        self.chip.set_level(lvl)
        rs = [r.text() for r in self.assessment.reasons if r.level == lvl] or [r.text() for r in self.assessment.reasons]
        self.reasons.setText(" · ".join(dict.fromkeys(rs))[:400])
        self.reasons.setProperty("level", lvl)
        self.reasons.style().unpolish(self.reasons)
        self.reasons.style().polish(self.reasons)
        self.run_btn.setObjectName("dangerButton" if lvl == DANGER else "primary")
        self.run_btn.setText(tr("run") + ("…" if lvl == DANGER else ""))
        self.run_btn.style().unpolish(self.run_btn)
        self.run_btn.style().polish(self.run_btn)
        self.preview_data = None
        self.pv.setPlainText(tr("preview_loading"))
        cmd = self.command
        Task(_preview.preview, cmd, self.cwd_fn(), on_ok=lambda r, c=cmd: self._got_preview(c, r),
             on_err=lambda e: self._got_preview(cmd, {"kind": "none", "text": str(e), "exact": False})).start()
        # show the preview open by default when it matters
        if lvl != SAFE and not self.pv_toggle.isChecked():
            self.pv_toggle.setChecked(True)

    def _got_preview(self, cmd, r):
        if cmd != self.command:
            return
        self.preview_data = r
        self.pv.setPlainText(r.get("text", ""))
        lines = r.get("text", "").count("\n") + 1
        lh = self.pv.fontMetrics().lineSpacing()
        self.pv.setFixedHeight(min(200, int(lh * lines + 26)))

    def _toggle_preview(self, on):
        self.pv.setVisible(on)
        self.pv_toggle.setText(("▾ " if on else "▸ ") + tr("preview"))

    def _toggle_edit(self):
        if self.cmd_edit.isVisible():
            self._finish_edit()
        else:
            self.cmd_edit.setText(self.command)
            self.cmd_label.hide()
            self.cmd_edit.show()
            self.cmd_edit.setFocus()
            self.edit_btn.setText(tr("done_edit"))

    def _finish_edit(self):
        new = self.cmd_edit.text().strip()
        self.cmd_edit.hide()
        self.cmd_label.show()
        self.edit_btn.setText(tr("edit"))
        if new and new != self.command:
            self.command = new
            self.cmd_label.setText(new)
            self._assess()

    def show_message(self, text):
        self.msg.setText(text)
        self.msg.setVisible(bool(text))

    def set_state(self, state, text=""):
        self.state = state
        active = state in ("idle",)
        for b in (self.run_btn, self.edit_btn, self.skip_btn):
            if b is not None:
                b.setEnabled(active)
        self.state_label.setText(text)
        if state != "idle":
            self.show_message("")
            # compact once handled: the output block below carries the result
            for w in (self.pv_toggle, self.pv, self.reasons, self.edit_btn, self.copy_btn, self.run_btn):
                w.hide()
            if self.skip_btn is not None:
                self.skip_btn.hide()


class ProposalCard(Card):
    runAllRequested = Signal(object)

    def __init__(self, explanation, commands, note="", source="ai", cwd_fn=None, parent=None):
        super().__init__("card", parent)
        self.items = []
        if source == "manual":
            self.lay.addWidget(label(tr("manual"), "muted"))
        if explanation:
            self.lay.addWidget(label(explanation))
        for c in commands:
            it = CommandItem(c["command"], c.get("explanation", ""), cwd_fn)
            self.items.append(it)
            self.lay.addWidget(it)
        if not commands and source != "manual":
            self.lay.addWidget(label(tr("no_commands"), "muted"))
        if note:
            self.lay.addWidget(label(note, "note"))
        if len(self.items) > 1 and all(i.assessment.level != DANGER for i in self.items):
            b = QPushButton(tr("run_all"))
            b.clicked.connect(lambda: self.runAllRequested.emit(self))
            row = QHBoxLayout()
            row.addWidget(b)
            row.addStretch(1)
            self.lay.addLayout(row)
            self.run_all_btn = b


class OutputBlock(Card):
    explainRequested = Signal(object)
    COLLAPSED_LINES = 12

    def __init__(self, command, parent=None):
        super().__init__("output", parent)
        self.command = command
        self.rc = None
        self.output = ""
        head = QHBoxLayout()
        head.setSpacing(8)
        self.icon = QLabel("●")
        self.icon.setObjectName("stRunning")
        c = QLabel(command)
        c.setFont(mono(9.5))
        c.setObjectName("cmdText")
        c.setWordWrap(True)
        c.setTextInteractionFlags(Qt.TextInteractionFlag.TextSelectableByMouse)
        self.status = label(tr("running"), "muted", wrap=False, selectable=False)
        head.addWidget(self.icon, 0, Qt.AlignmentFlag.AlignTop)
        head.addWidget(c, 1)
        head.addWidget(self.status, 0, Qt.AlignmentFlag.AlignTop)
        self.lay.addLayout(head)
        self.body = QPlainTextEdit()
        self.body.setObjectName("outputBody")
        self.body.setReadOnly(True)
        self.body.setFont(mono(9))
        self.body.setLineWrapMode(QPlainTextEdit.LineWrapMode.NoWrap)
        self.body.hide()
        self.lay.addWidget(self.body)
        row = QHBoxLayout()
        row.setSpacing(8)
        self.more = QToolButton()
        self.more.setObjectName("linkButton")
        self.more.hide()
        self.more.clicked.connect(self._toggle)
        self.copy_btn = QPushButton(tr("copy_output"))
        self.copy_btn.clicked.connect(lambda: QGuiApplication.clipboard().setText(self.output))
        self.copy_btn.hide()
        self.explain_btn = QPushButton(tr("explain_error"))
        self.explain_btn.setObjectName("primary")
        self.explain_btn.clicked.connect(lambda: self.explainRequested.emit(self))
        self.explain_btn.hide()
        row.addWidget(self.more)
        row.addStretch(1)
        row.addWidget(self.copy_btn)
        row.addWidget(self.explain_btn)
        self.lay.addLayout(row)
        self.expanded = False
        self.t0 = None
        import time
        self.t0 = time.monotonic()

    def finish(self, rc, output, can_explain=False):
        import time
        self.rc = rc
        self.output = output
        dt = time.monotonic() - self.t0
        dur = f" · {dt:.1f} s" if dt >= 0.1 else ""
        if rc == 0:
            self.icon.setObjectName("stOk")
            self.icon.setText("✓")
            self.status.setText(tr("exit_ok") + dur)
        elif rc in (130, 143):
            self.icon.setObjectName("stFail")
            self.icon.setText("■")
            self.status.setText(tr("interrupted") + dur)
        else:
            self.icon.setObjectName("stFail")
            self.icon.setText("✗")
            self.status.setText(tr("exit_fail").format(rc=rc) + dur)
            self.explain_btn.setVisible(can_explain)
        self.icon.style().unpolish(self.icon)
        self.icon.style().polish(self.icon)
        lines = output.splitlines()
        if output.strip():
            self.body.show()
            self.copy_btn.show()
            if len(lines) > self.COLLAPSED_LINES:
                self.more.show()
                self._render(False)
            else:
                self._render(True)
        else:
            self.lay.insertWidget(1, label(tr("no_output"), "muted"))

    def _render(self, full):
        self.expanded = full
        lines = self.output.splitlines()
        shown = lines if full else lines[:self.COLLAPSED_LINES]
        self.body.setPlainText("\n".join(shown))
        h = self.body.fontMetrics().lineSpacing() * min(len(shown), 30 if full else self.COLLAPSED_LINES) + 18
        self.body.setFixedHeight(int(h) + (14 if any(len(l) > 70 for l in shown) else 0))
        self.more.setText(tr("show_less") if full else tr("show_all").format(n=len(lines)))

    def _toggle(self):
        self._render(not self.expanded)


class PlanCard(Card):
    stopRequested = Signal()

    def __init__(self, goal, parent=None):
        super().__init__("plan", parent)
        head = QHBoxLayout()
        t = label(tr("goal") + ": " + goal, "cardTitle")
        head.addWidget(t, 1)
        self.stop_btn = QPushButton(tr("stop"))
        self.stop_btn.setObjectName("stopButton")
        self.stop_btn.clicked.connect(self.stopRequested.emit)
        head.addWidget(self.stop_btn, 0, Qt.AlignmentFlag.AlignTop)
        self.lay.addLayout(head)
        self.plan_label = label("", "planList")
        self.plan_label.setTextFormat(Qt.TextFormat.RichText)
        self.lay.addWidget(self.plan_label)
        self.steps_box = QVBoxLayout()
        self.steps_box.setSpacing(8)
        self.lay.addLayout(self.steps_box)
        self.final = label("", "finalMsg")
        self.final.hide()
        self.lay.addWidget(self.final)
        self.n_done = 0

    def set_plan(self, plan, n_done):
        self.n_done = n_done
        if not plan:
            self.plan_label.hide()
            return
        import html
        rows = []
        for i, s in enumerate(plan):
            mark = "✓" if i < n_done else ("▸" if i == n_done else "○")
            rows.append(f"<div style='margin:2px 0'>{mark}&nbsp;&nbsp;{html.escape(s)}</div>")
        self.plan_label.setText(f"<b>{tr('plan')}</b>" + "".join(rows))
        self.plan_label.show()

    def add_widget(self, w):
        self.steps_box.addWidget(w)

    def finish(self, message, title):
        self.stop_btn.hide()
        self.final.setText(f"{title}. {message}" if message else title)
        self.final.show()
