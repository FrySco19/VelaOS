"""Explicit confirmation dialog for dangerous commands (hard guard).

Standalone: reads {"command","cwd","assessment","preview","caller"} JSON on stdin,
exits 0 ONLY if the user ticked the acknowledgement and pressed "Esegui comunque".
"""
from __future__ import annotations

import json
import sys

from PySide6.QtCore import Qt
from PySide6.QtGui import QFont, QFontDatabase
from PySide6.QtWidgets import (QApplication, QCheckBox, QDialog, QHBoxLayout, QLabel, QPlainTextEdit,
                               QPushButton, QVBoxLayout)

from . import theme
from .i18n import tr


def mono_font(size=10):
    fams = set(QFontDatabase.families())
    fam = next((x for x in ("JetBrains Mono", "Noto Sans Mono", "DejaVu Sans Mono") if x in fams), "Monospace")
    f = QFont(fam)
    f.setStyleHint(QFont.StyleHint.Monospace)
    f.setPointSize(size)
    return f


class DangerDialog(QDialog):
    def __init__(self, command, assessment, preview_text="", caller="", lang="it", parent=None):
        super().__init__(parent)
        self.setObjectName("dangerDialog")
        self.setWindowTitle(tr("danger_title", lang))
        self.setModal(True)
        self.setMinimumWidth(560)
        c = theme.colors(theme.is_dark())
        lay = QVBoxLayout(self)
        lay.setContentsMargins(24, 20, 24, 20)
        lay.setSpacing(12)
        title = QLabel(tr("danger_heading", lang))
        f = title.font()
        f.setPointSize(f.pointSize() + 4)
        f.setWeight(QFont.Weight.DemiBold)
        title.setFont(f)
        lay.addWidget(title)
        if caller and caller not in ("terminal",):
            who = QLabel(tr("danger_requested_by", lang).format(caller=caller))
            who.setStyleSheet(f"color:{c['textSecondary']}")
            lay.addWidget(who)
        cmd = QPlainTextEdit(command)
        cmd.setReadOnly(True)
        cmd.setFont(mono_font())
        cmd.setFixedHeight(int(cmd.fontMetrics().lineSpacing() * min(4, command.count("\n") + 1 + len(command) // 70) + 20))
        cmd.setObjectName("dangerCommand")
        lay.addWidget(cmd)
        reasons = [r for r in assessment.get("reason_objects", assessment.get("reasons", []))]
        texts = []
        for r in reasons:
            if isinstance(r, dict):
                if r.get("level") == "danger":
                    texts.append(r.get("it" if lang == "it" else "en") or r.get("text", ""))
            else:
                texts.append(str(r))
        why = QLabel("\n".join("• " + t for t in texts[:6]) or assessment.get("summary", ""))
        why.setWordWrap(True)
        why.setStyleSheet(f"color:{c['negative']}; font-weight:600")
        lay.addWidget(why)
        if preview_text:
            pv = QPlainTextEdit(preview_text)
            pv.setReadOnly(True)
            pv.setFont(mono_font(9))
            pv.setLineWrapMode(QPlainTextEdit.LineWrapMode.NoWrap)
            pv.setFixedHeight(int(pv.fontMetrics().lineSpacing() * min(8, preview_text.count("\n") + 1) + 26))
            lay.addWidget(QLabel(tr("preview", lang)))
            lay.addWidget(pv)
        self.ack = QCheckBox(tr("danger_ack", lang))
        self.ack.setObjectName("dangerAck")
        lay.addWidget(self.ack)
        row = QHBoxLayout()
        row.addStretch(1)
        self.cancel_btn = QPushButton(tr("cancel", lang))
        self.cancel_btn.setDefault(True)
        self.cancel_btn.setAutoDefault(True)
        self.ok_btn = QPushButton(tr("run_anyway", lang))
        self.ok_btn.setObjectName("dangerRun")
        self.ok_btn.setEnabled(False)
        self.ok_btn.setAutoDefault(False)
        self.ok_btn.setStyleSheet(
            f"QPushButton#dangerRun{{background:{c['negative']};color:white;border:none;border-radius:8px;padding:7px 16px;font-weight:600}}"
            f"QPushButton#dangerRun:disabled{{background:{c['border']};color:{c['textSecondary']}}}")
        self.cancel_btn.setStyleSheet("padding:7px 16px")
        row.addWidget(self.cancel_btn)
        row.addWidget(self.ok_btn)
        lay.addLayout(row)
        self.ack.toggled.connect(self.ok_btn.setEnabled)
        self.cancel_btn.clicked.connect(self.reject)
        self.ok_btn.clicked.connect(self._accept)

    def _accept(self):
        if self.ack.isChecked():  # double check: the button is only enabled when ticked
            self.accept()


def ask(parent, command, assessment, preview_text="", caller="terminal", lang="it") -> bool:
    d = DangerDialog(command, assessment, preview_text, caller, lang, parent)
    return d.exec() == QDialog.DialogCode.Accepted and d.ack.isChecked()


def main():
    data = json.loads(sys.stdin.read() or "{}")
    app = QApplication.instance() or QApplication(sys.argv[:1])
    app.setApplicationName("vela-executor")
    app.setDesktopFileName("vela-ai-terminal")
    from . import aiconfig
    lang = aiconfig.language()
    pv = (data.get("preview") or {}).get("text", "")
    ok = ask(None, data.get("command", ""), data.get("assessment") or {}, pv, data.get("caller", ""), lang)
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
