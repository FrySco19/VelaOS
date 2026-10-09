"""A real terminal widget: PTY + pyte VT emulator + QPainter renderer.

Features: 256/true colour, bold/italic/underline/reverse, alternate screen (vim, htop),
scrollback, mouse selection (primary selection), Ctrl+Shift+C/V, Ctrl+V paste,
Ctrl+C copies when text is selected (otherwise SIGINT), bracketed paste, application cursor
keys, OSC 133 (prompt/command marks) and OSC 7 (cwd) shell integration so the AI panel knows
when a command starts/ends, its exit code and its output.
"""
from __future__ import annotations

import collections
import errno
import fcntl
import os
import re
import signal
import struct
import subprocess
import termios

import pyte
from pyte import modes as mo
from PySide6.QtCore import QEvent, QPoint, QRect, QSocketNotifier, Qt, QTimer, Signal
from PySide6.QtGui import (QColor, QFont, QFontDatabase, QFontMetricsF, QGuiApplication, QKeySequence,
                           QPainter, QPen)
from PySide6.QtWidgets import QMenu, QWidget

ANSI_NAMES = {"black": 0, "red": 1, "green": 2, "brown": 3, "yellow": 3, "blue": 4, "magenta": 5,
              "cyan": 6, "white": 7, "brightblack": 8, "brightred": 9, "brightgreen": 10,
              "brightbrown": 11, "brightyellow": 11, "brightblue": 12, "brightmagenta": 13,
              "brightcyan": 14, "brightwhite": 15}
ALT_MODES = {1049 << 5, 47 << 5, 1047 << 5}
DECCKM = 1 << 5
BRACKETED = 2004 << 5
ANSI_RE = re.compile(r"\x1b\[[0-?]*[ -/]*[@-~]|\x1b\][^\x07\x1b]*(\x07|\x1b\\)|\x1b[()][A-Za-z0-9]|\x1b[=>78DEHMcNO]")
OSC_RE = re.compile(rb"\x1b\](133|7|633);([^\x07\x1b]*)(\x07|\x1b\\)")


def strip_ansi(text: str) -> str:
    text = ANSI_RE.sub("", text)
    # apply carriage returns / backspaces crudely for readable logs
    out_lines = []
    for line in text.split("\n"):
        if "\r" in line:
            parts = [p for p in line.split("\r") if p]
            line = parts[-1] if parts else ""
        while "\b" in line:
            line = re.sub(r"[^\b]\b", "", line, count=1)
            line = line.replace("\b", "", 1) if line.startswith("\b") else line
        out_lines.append(line)
    return "\n".join(out_lines)


class VelaScreen(pyte.Screen):
    def __init__(self, columns, lines, scrollback=10000):
        super().__init__(columns, lines)
        self.scrollback = collections.deque(maxlen=scrollback)
        self.alt = False
        self._saved = None
        self.on_respond = None
        self.on_bell = None
        self.title = ""

    # scrollback ----------------------------------------------------------
    def index(self):
        top, bottom = self.margins or pyte.screens.Margins(0, self.lines - 1)
        if self.cursor.y == bottom and top == 0 and not self.alt:
            line = self.buffer[0]
            self.scrollback.append({x: line[x] for x in list(line.keys())})
        super().index()

    # modes ---------------------------------------------------------------
    def set_mode(self, *modes, **kwargs):
        if kwargs.get("private"):
            shifted = {m << 5 for m in modes}
            if shifted & ALT_MODES and not self.alt:
                self._saved = ({y: dict(self.buffer[y]) for y in range(self.lines)},
                               (self.cursor.x, self.cursor.y), self.cursor.attrs)
                self.alt = True
                self.erase_in_display(2)
                self.dirty.update(range(self.lines))
            modes = tuple(m for m in modes if (m << 5) not in ALT_MODES and m != 3)
            if not modes:
                return
        super().set_mode(*modes, **kwargs)

    def reset_mode(self, *modes, **kwargs):
        if kwargs.get("private"):
            shifted = {m << 5 for m in modes}
            if shifted & ALT_MODES and self.alt:
                self.alt = False
                if self._saved:
                    buf, (x, y), attrs = self._saved
                    self.buffer.clear()
                    for yy, row in buf.items():
                        if yy < self.lines:
                            line = self.buffer[yy]
                            for xx, ch in row.items():
                                if xx < self.columns:
                                    line[xx] = ch
                    self.cursor.x, self.cursor.y = min(x, self.columns - 1), min(y, self.lines - 1)
                    self._saved = None
                self.dirty.update(range(self.lines))
            modes = tuple(m for m in modes if (m << 5) not in ALT_MODES and m != 3)
            if not modes:
                return
        super().reset_mode(*modes, **kwargs)

    def write_process_input(self, data):
        if self.on_respond:
            self.on_respond(data)

    def bell(self, *args):
        if self.on_bell:
            self.on_bell()

    def set_title(self, param):
        self.title = param
        super().set_title(param)


class TerminalWidget(QWidget):
    promptReady = Signal()
    commandStarted = Signal()
    commandFinished = Signal(int, str)    # exit code, plain output
    cwdChanged = Signal(str)
    titleChanged = Signal(str)
    processExited = Signal(int)
    askAi = Signal(str)                   # selected text -> AI panel

    PAD = 10

    def __init__(self, argv, cwd=None, env=None, palette=None, font_family=None, font_size=10.5, parent=None):
        super().__init__(parent)
        self.setObjectName("terminal")
        self.setFocusPolicy(Qt.FocusPolicy.StrongFocus)
        self.setAttribute(Qt.WidgetAttribute.WA_InputMethodEnabled, True)
        self.setAttribute(Qt.WidgetAttribute.WA_OpaquePaintEvent, True)
        self.setCursor(Qt.CursorShape.IBeamCursor)
        self.setMouseTracking(False)
        self.argv = argv
        self.cwd = cwd or os.path.expanduser("~")
        self.env = env or dict(os.environ)
        self.pal = palette
        self.master = None
        self.proc = None
        self.notifier = None
        self.at_prompt = False
        self.capturing = False
        self.capture = bytearray()
        self.capture_limit = 2 * 1024 * 1024
        self.capture_truncated = False
        self.current_cwd = self.cwd
        self._pending = b""
        self.scroll = 0  # lines scrolled back
        self.sel_start = None
        self.sel_end = None
        self._selecting = False
        self._click_count = 0
        self._last_click = 0
        self._cursor_on = True
        fams = set(QFontDatabase.families())
        fam = font_family or next((f for f in ("JetBrains Mono", "Noto Sans Mono", "DejaVu Sans Mono") if f in fams), "Monospace")
        self.setFontSpec(fam, font_size)
        self.screen = VelaScreen(80, 24)
        self.screen.on_respond = self._respond
        self.screen.on_bell = QGuiApplication.beep if hasattr(QGuiApplication, "beep") else None
        self.stream = pyte.ByteStream(self.screen)
        self._ready = True
        self._repaint = QTimer(self)
        self._repaint.setSingleShot(True)
        self._repaint.setInterval(12)
        self._repaint.timeout.connect(self.update)
        self._blink = QTimer(self)
        self._blink.setInterval(600)
        self._blink.timeout.connect(self._toggle_cursor)

    # ------------------------------------------------------------------ setup
    def setFontSpec(self, family, size):
        f = QFont(family)
        f.setStyleHint(QFont.StyleHint.Monospace)
        f.setFixedPitch(True)
        f.setPointSizeF(size)
        f.setKerning(False)
        self.tfont = f
        self.bfont = QFont(f)
        self.bfont.setBold(True)
        self.ifont = QFont(f)
        self.ifont.setItalic(True)
        fm = QFontMetricsF(f)
        self.cw = fm.horizontalAdvance("M")
        self.ch = fm.height() * 1.12
        self.ascent = fm.ascent() + (self.ch - fm.height()) / 2
        if getattr(self, "_ready", False):
            self._resize_screen()
            self.update()

    def zoom(self, delta):
        size = max(7.0, min(28.0, self.tfont.pointSizeF() + delta))
        self.setFontSpec(self.tfont.family(), size)

    def setPalette_(self, pal):
        self.pal = pal
        self.update()

    def start(self):
        cols, rows = self._grid_size()
        self.screen.resize(rows, cols)
        master, slave = os.openpty()
        self._set_winsize(slave, rows, cols)
        env = dict(self.env)
        env.setdefault("TERM", "xterm-256color")
        env["COLORTERM"] = "truecolor"
        env["VELA_AI_TERMINAL"] = "1"

        def child_setup():
            os.setsid()
            fcntl.ioctl(0, termios.TIOCSCTTY, 0)
            signal.signal(signal.SIGINT, signal.SIG_DFL)
            signal.signal(signal.SIGPIPE, signal.SIG_DFL)

        self.proc = subprocess.Popen(self.argv, stdin=slave, stdout=slave, stderr=slave, cwd=self.cwd, env=env,
                                     preexec_fn=child_setup, close_fds=True)
        os.close(slave)
        fl = fcntl.fcntl(master, fcntl.F_GETFL)
        fcntl.fcntl(master, fcntl.F_SETFL, fl | os.O_NONBLOCK)
        self.master = master
        self.notifier = QSocketNotifier(master, QSocketNotifier.Type.Read, self)
        self.notifier.activated.connect(self._on_read)
        self._blink.start()

    def shell_pid(self):
        return self.proc.pid if self.proc else None

    def foreground_cwd(self):
        """Current directory of the shell (more reliable than OSC 7 if missing)."""
        try:
            return os.readlink(f"/proc/{self.proc.pid}/cwd")
        except (OSError, AttributeError):
            return self.current_cwd

    def busy(self):
        """True when a program is running in the foreground (not at the prompt)."""
        if self.master is None:
            return True
        try:
            fg = os.tcgetpgrp(self.master)
            return fg != self.proc.pid
        except OSError:
            return not self.at_prompt

    def close_pty(self):
        if self.proc and self.proc.poll() is None:
            try:
                os.killpg(self.proc.pid, signal.SIGHUP)
            except OSError:
                pass
        if self.notifier:
            self.notifier.setEnabled(False)
        if self.master is not None:
            try:
                os.close(self.master)
            except OSError:
                pass
            self.master = None

    # ------------------------------------------------------------------ io
    def write(self, data):
        if isinstance(data, str):
            data = data.encode("utf-8")
        if self.master is None:
            return
        while data:
            try:
                n = os.write(self.master, data)
                data = data[n:]
            except BlockingIOError:
                import select
                select.select([], [self.master], [], 0.05)
            except OSError:
                return

    def _respond(self, data):
        self.write(data)

    def _on_read(self):
        chunks = []
        total = 0
        while total < 512 * 1024:
            try:
                d = os.read(self.master, 65536)
            except BlockingIOError:
                break
            except OSError as e:
                if e.errno == errno.EIO:
                    self._child_exited()
                    return
                break
            if not d:
                self._child_exited()
                return
            chunks.append(d)
            total += len(d)
        if chunks:
            self._process(b"".join(chunks))

    def _child_exited(self):
        if self.notifier:
            self.notifier.setEnabled(False)
        rc = self.proc.wait() if self.proc else 0
        self.processExited.emit(rc)

    def _process(self, data: bytes):
        data = self._pending + data
        self._pending = b""
        # keep an unterminated OSC for the next read
        i = data.rfind(b"\x1b]")
        if i >= 0 and b"\x07" not in data[i:] and b"\x1b\\" not in data[i:] and len(data) - i < 4096:
            data, self._pending = data[:i], data[i:]
        pos = 0
        for m in OSC_RE.finditer(data):
            self._feed(data[pos:m.start()])
            self._osc(m.group(1), m.group(2))
            pos = m.end()
        self._feed(data[pos:])
        if self.scroll and not self.screen.alt:
            pass  # keep user's scroll position
        self._cursor_on = True
        if not self._repaint.isActive():
            self._repaint.start()

    def _feed(self, data: bytes):
        if not data:
            return
        if self.capturing:
            room = self.capture_limit - len(self.capture)
            if room > 0:
                self.capture += data[:room]
            if len(data) > room:
                self.capture_truncated = True
        title = self.screen.title
        self.stream.feed(data)
        if self.screen.title != title:
            self.titleChanged.emit(self.screen.title)

    def _osc(self, code: bytes, payload: bytes):
        p = payload.decode("utf-8", "replace")
        if code in (b"133", b"633"):
            kind = p.split(";", 1)[0]
            if kind == "A":
                pass
            elif kind == "B":
                self.at_prompt = True
                self.promptReady.emit()
            elif kind == "C":
                self.at_prompt = False
                self.capturing = True
                self.capture = bytearray()
                self.capture_truncated = False
                self.commandStarted.emit()
            elif kind == "D":
                if self.capturing:
                    self.capturing = False
                    try:
                        rc = int(p.split(";")[1])
                    except (IndexError, ValueError):
                        rc = 0
                    text = strip_ansi(self.capture.decode("utf-8", "replace"))
                    if self.capture_truncated:
                        text += "\n[…]"
                    self.commandFinished.emit(rc, text.strip("\n"))
        elif code == b"7":
            path = p
            if path.startswith("file://"):
                path = path[7:]
                path = path[path.find("/"):] if "/" in path else path
            from urllib.parse import unquote
            path = unquote(path)
            if path != self.current_cwd:
                self.current_cwd = path
                self.cwdChanged.emit(path)

    def run_command(self, command: str) -> bool:
        """Type `command` into the shell and press Enter. Returns False if the shell is busy."""
        if self.busy():
            return False
        self.scroll = 0
        self.write(b"\x05\x15")  # end of line, kill line
        if BRACKETED in self.screen.mode:
            self.write(b"\x1b[200~" + command.encode() + b"\x1b[201~\r")
        else:
            self.write(command.replace("\n", " ; ").encode() + b"\r")
        return True

    def interrupt(self):
        self.write(b"\x03")

    # ------------------------------------------------------------------ geometry
    def _grid_size(self):
        cols = max(20, int((self.width() - 2 * self.PAD) // self.cw))
        rows = max(5, int((self.height() - 2 * self.PAD) // self.ch))
        return cols, rows

    def _set_winsize(self, fd, rows, cols):
        try:
            fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", rows, cols, int(cols * self.cw), int(rows * self.ch)))
        except OSError:
            pass

    def _resize_screen(self):
        cols, rows = self._grid_size()
        if (cols, rows) != (self.screen.columns, self.screen.lines):
            # keep content: push lines that fall off the top into scrollback
            if rows < self.screen.lines and not self.screen.alt:
                over = self.screen.cursor.y - (rows - 1)
                for y in range(max(0, over)):
                    line = self.screen.buffer[y]
                    self.screen.scrollback.append({x: line[x] for x in list(line.keys())})
            self.screen.resize(rows, cols)
            if self.master is not None:
                self._set_winsize(self.master, rows, cols)

    def resizeEvent(self, e):
        self._resize_screen()
        super().resizeEvent(e)

    # ------------------------------------------------------------------ painting
    def _color(self, name, default):
        if name == "default":
            return default
        if name in ANSI_NAMES:
            return QColor(self.pal["ansi"][ANSI_NAMES[name]])
        if len(name) == 6:
            try:
                return QColor("#" + name)
            except Exception:
                return default
        return default

    def _lines(self):
        sb = self.screen.scrollback
        return len(sb), sb

    def _line_at(self, abs_y):
        n = len(self.screen.scrollback)
        if abs_y < n:
            return self.screen.scrollback[abs_y]
        return self.screen.buffer[abs_y - n]

    def paintEvent(self, e):
        p = QPainter(self)
        bg = QColor(self.pal["bg"])
        fg = QColor(self.pal["fg"])
        p.fillRect(self.rect(), bg)
        scr = self.screen
        nsb = len(scr.scrollback)
        top_abs = nsb - self.scroll
        sel = self._norm_sel()
        sel_color = QColor(self.pal["selection"])
        for row in range(scr.lines):
            abs_y = top_abs + row
            if abs_y < 0:
                continue
            line = self._line_at(abs_y)
            y = self.PAD + row * self.ch
            x = 0
            cols = scr.columns
            while x < cols:
                ch = line[x] if not isinstance(line, dict) or x in line else scr.default_char
                start = x
                attrs = (ch.fg, ch.bg, ch.bold, ch.italics, ch.underscore, ch.reverse, ch.strikethrough)
                text = [ch.data or " "]
                x += 1
                ascii_run = (ch.data or " ").isascii()
                while x < cols and ascii_run:
                    c2 = line[x] if not isinstance(line, dict) or x in line else scr.default_char
                    if (c2.fg, c2.bg, c2.bold, c2.italics, c2.underscore, c2.reverse, c2.strikethrough) != attrs \
                            or not (c2.data or " ").isascii() or self._in_sel(sel, abs_y, x) != self._in_sel(sel, abs_y, start):
                        break
                    text.append(c2.data or " ")
                    x += 1
                f_c = self._color(ch.fg, fg)
                b_c = self._color(ch.bg, None)
                if ch.bold and ch.fg in ANSI_NAMES and ANSI_NAMES[ch.fg] < 8:
                    f_c = QColor(self.pal["ansi"][ANSI_NAMES[ch.fg] + 8])
                if ch.reverse:
                    f_c, b_c = (b_c or bg), f_c
                if self._in_sel(sel, abs_y, start):
                    b_c = sel_color
                rx = self.PAD + start * self.cw
                rw = (x - start) * self.cw
                if b_c is not None:
                    p.fillRect(QRect(int(rx), int(y), int(rw + 1), int(self.ch + 1)), b_c)
                s = "".join(text)
                if s.strip():
                    p.setFont(self.bfont if ch.bold else (self.ifont if ch.italics else self.tfont))
                    p.setPen(f_c)
                    p.drawText(int(rx), int(y + self.ascent), s)
                    if ch.underscore:
                        p.drawLine(int(rx), int(y + self.ch - 2), int(rx + rw), int(y + self.ch - 2))
                    if ch.strikethrough:
                        p.drawLine(int(rx), int(y + self.ch / 2), int(rx + rw), int(y + self.ch / 2))
        # cursor
        if self.scroll == 0 and not scr.cursor.hidden and mo.DECTCEM in scr.mode:
            cx = self.PAD + scr.cursor.x * self.cw
            cy = self.PAD + scr.cursor.y * self.ch
            cc = QColor(self.pal["cursor"])
            if self.hasFocus():
                if self._cursor_on:
                    p.fillRect(QRect(int(cx), int(cy), int(self.cw), int(self.ch)), cc)
                    ch = scr.buffer[scr.cursor.y][scr.cursor.x]
                    if ch.data and ch.data.strip():
                        p.setPen(bg)
                        p.setFont(self.tfont)
                        p.drawText(int(cx), int(cy + self.ascent), ch.data)
            else:
                p.setPen(QPen(cc, 1))
                p.drawRect(QRect(int(cx), int(cy), int(self.cw) - 1, int(self.ch) - 1))
        # scroll indicator
        if self.scroll:
            total = nsb + scr.lines
            h = max(24, self.height() * scr.lines / total)
            ypos = (self.height() - h) * (top_abs / max(1, nsb))
            c = QColor(fg)
            c.setAlpha(70)
            p.setPen(Qt.PenStyle.NoPen)
            p.setBrush(c)
            p.drawRoundedRect(self.width() - 7, int(ypos), 4, int(h), 2, 2)
        p.end()

    def _toggle_cursor(self):
        if self.hasFocus() and self.scroll == 0:
            self._cursor_on = not self._cursor_on
            cx = self.PAD + self.screen.cursor.x * self.cw
            cy = self.PAD + self.screen.cursor.y * self.ch
            self.update(QRect(int(cx) - 1, int(cy) - 1, int(self.cw) + 2, int(self.ch) + 2))

    # ------------------------------------------------------------------ selection
    def _pos_to_cell(self, pos: QPoint):
        col = int((pos.x() - self.PAD) // self.cw)
        row = int((pos.y() - self.PAD) // self.ch)
        col = max(0, min(self.screen.columns - 1, col))
        row = max(0, min(self.screen.lines - 1, row))
        return len(self.screen.scrollback) - self.scroll + row, col

    def _norm_sel(self):
        if self.sel_start is None or self.sel_end is None or self.sel_start == self.sel_end:
            return None
        a, b = sorted([self.sel_start, self.sel_end])
        return a, b

    @staticmethod
    def _in_sel(sel, y, x):
        if not sel:
            return False
        (y1, x1), (y2, x2) = sel
        if y < y1 or y > y2:
            return False
        if y1 == y2:
            return x1 <= x < x2
        if y == y1:
            return x >= x1
        if y == y2:
            return x < x2
        return True

    def selected_text(self) -> str:
        sel = self._norm_sel()
        if not sel:
            return ""
        (y1, x1), (y2, x2) = sel
        out = []
        for y in range(y1, y2 + 1):
            line = self._line_at(y)
            a = x1 if y == y1 else 0
            b = x2 if y == y2 else self.screen.columns
            chars = []
            for x in range(a, b):
                c = line[x] if not isinstance(line, dict) or x in line else None
                chars.append((c.data if c else " ") or "")
            out.append("".join(chars).rstrip())
        return "\n".join(out)

    def has_selection(self):
        return self._norm_sel() is not None

    def clear_selection(self):
        self.sel_start = self.sel_end = None
        self.update()

    def select_all(self):
        self.sel_start = (0, 0)
        self.sel_end = (len(self.screen.scrollback) + self.screen.lines - 1, self.screen.columns)
        self.update()

    def copy(self):
        t = self.selected_text()
        if t:
            QGuiApplication.clipboard().setText(t)
        return bool(t)

    def paste(self, mode=None):
        cb = QGuiApplication.clipboard()
        text = cb.text(mode) if mode is not None else cb.text()
        if not text:
            return
        self.scroll = 0
        if BRACKETED in self.screen.mode:
            self.write(b"\x1b[200~" + text.replace("\x1b", "").encode() + b"\x1b[201~")
        else:
            self.write(text.replace("\r\n", "\r").replace("\n", "\r"))

    def mousePressEvent(self, e):
        self.setFocus()
        if e.button() == Qt.MouseButton.LeftButton:
            import time
            now = time.monotonic()
            self._click_count = self._click_count + 1 if now - self._last_click < 0.4 else 1
            self._last_click = now
            y, x = self._pos_to_cell(e.position().toPoint())
            if self._click_count == 2:
                self._select_word(y, x)
            elif self._click_count >= 3:
                self.sel_start, self.sel_end = (y, 0), (y, self.screen.columns)
                self._publish_primary()
            else:
                self.sel_start = self.sel_end = (y, x)
                self._selecting = True
            self.update()
        elif e.button() == Qt.MouseButton.MiddleButton:
            self.paste(QGuiApplication.clipboard().Mode.Selection)

    def _select_word(self, y, x):
        line = self._line_at(y)
        def ch(i):
            c = line[i] if not isinstance(line, dict) or i in line else None
            return (c.data if c else " ") or " "
        word = re.compile(r"[\w\-./~:@%+=,]")
        a = b = x
        while a > 0 and word.match(ch(a - 1)):
            a -= 1
        while b < self.screen.columns and word.match(ch(b)):
            b += 1
        self.sel_start, self.sel_end = (y, a), (y, b)
        self._publish_primary()

    def mouseMoveEvent(self, e):
        if self._selecting:
            pos = e.position().toPoint()
            if pos.y() < 0 and self.scroll < len(self.screen.scrollback):
                self.scroll += 1
            elif pos.y() > self.height() and self.scroll > 0:
                self.scroll -= 1
            y, x = self._pos_to_cell(pos)
            if pos.x() > self.PAD + self.screen.columns * self.cw:
                x = self.screen.columns
            self.sel_end = (y, x)
            self.update()

    def mouseReleaseEvent(self, e):
        if e.button() == Qt.MouseButton.LeftButton and self._selecting:
            self._selecting = False
            self._publish_primary()

    def _publish_primary(self):
        t = self.selected_text()
        cb = QGuiApplication.clipboard()
        if t and cb.supportsSelection():
            cb.setText(t, cb.Mode.Selection)

    def wheelEvent(self, e):
        steps = int(e.angleDelta().y() / 40) or (1 if e.angleDelta().y() > 0 else -1 if e.angleDelta().y() < 0 else 0)
        if e.modifiers() & Qt.KeyboardModifier.ControlModifier:
            self.zoom(0.5 if steps > 0 else -0.5)
            return
        if self.screen.alt:
            key = (b"\x1bOA" if DECCKM in self.screen.mode else b"\x1b[A") if steps > 0 else \
                (b"\x1bOB" if DECCKM in self.screen.mode else b"\x1b[B")
            self.write(key * min(abs(steps), 5))
            return
        self.scroll = max(0, min(len(self.screen.scrollback), self.scroll + steps))
        self.update()

    def contextMenuEvent(self, e):
        from .i18n import tr
        m = QMenu(self)
        a_copy = m.addAction(tr("copy"), self.copy)
        a_copy.setShortcut(QKeySequence("Ctrl+Shift+C"))
        a_copy.setEnabled(self.has_selection())
        a_paste = m.addAction(tr("paste"), self.paste)
        a_paste.setShortcut(QKeySequence("Ctrl+Shift+V"))
        m.addAction(tr("select_all"), self.select_all)
        m.addSeparator()
        if self.has_selection():
            m.addAction(tr("ask_ai_selection"), lambda: self.askAi.emit(self.selected_text()))
        m.addAction(tr("clear_screen"), self.clear_scrollback)
        m.addSeparator()
        m.addAction(tr("zoom_in"), lambda: self.zoom(1))
        m.addAction(tr("zoom_out"), lambda: self.zoom(-1))
        m.exec(e.globalPos())

    def clear_scrollback(self):
        self.screen.scrollback.clear()
        self.scroll = 0
        self.write(b"\x0c")  # Ctrl+L: shell redraws the prompt at the top
        self.update()

    # ------------------------------------------------------------------ keyboard
    def focusNextPrevChild(self, nxt):
        return False

    def event(self, e):
        if e.type() == QEvent.Type.ShortcutOverride:
            # make sure terminal keys reach us before window shortcuts
            k = e.key()
            mods = e.modifiers()
            ctrl = bool(mods & Qt.KeyboardModifier.ControlModifier)
            shift = bool(mods & Qt.KeyboardModifier.ShiftModifier)
            if ctrl and not shift and Qt.Key.Key_A <= k <= Qt.Key.Key_Z and k not in (Qt.Key.Key_V,):
                e.accept()
                return True
        return super().event(e)

    def keyPressEvent(self, e):
        k = e.key()
        mods = e.modifiers()
        ctrl = bool(mods & Qt.KeyboardModifier.ControlModifier)
        shift = bool(mods & Qt.KeyboardModifier.ShiftModifier)
        alt = bool(mods & Qt.KeyboardModifier.AltModifier)
        app = DECCKM in self.screen.mode
        if ctrl and shift and k == Qt.Key.Key_C:
            self.copy()
            return
        if (ctrl and shift and k == Qt.Key.Key_V) or (ctrl and not shift and k == Qt.Key.Key_V) or \
                (shift and k == Qt.Key.Key_Insert):
            self.paste()
            return
        if ctrl and not shift and k == Qt.Key.Key_C and self.has_selection():
            self.copy()
            self.clear_selection()
            return
        if ctrl and k in (Qt.Key.Key_Plus, Qt.Key.Key_Equal):
            self.zoom(1)
            return
        if ctrl and k == Qt.Key.Key_Minus:
            self.zoom(-1)
            return
        if ctrl and k == Qt.Key.Key_0:
            self.setFontSpec(self.tfont.family(), 10.5)
            return
        if shift and k in (Qt.Key.Key_PageUp, Qt.Key.Key_PageDown):
            delta = self.screen.lines - 1
            self.scroll = max(0, min(len(self.screen.scrollback), self.scroll + (delta if k == Qt.Key.Key_PageUp else -delta)))
            self.update()
            return
        mod_n = 1 + (1 if shift else 0) + (2 if alt else 0) + (4 if ctrl else 0)
        seq = None
        arrows = {Qt.Key.Key_Up: "A", Qt.Key.Key_Down: "B", Qt.Key.Key_Right: "C", Qt.Key.Key_Left: "D",
                  Qt.Key.Key_Home: "H", Qt.Key.Key_End: "F"}
        tilde = {Qt.Key.Key_Insert: 2, Qt.Key.Key_Delete: 3, Qt.Key.Key_PageUp: 5, Qt.Key.Key_PageDown: 6,
                 Qt.Key.Key_F5: 15, Qt.Key.Key_F6: 17, Qt.Key.Key_F7: 18, Qt.Key.Key_F8: 19, Qt.Key.Key_F9: 20,
                 Qt.Key.Key_F10: 21, Qt.Key.Key_F11: 23, Qt.Key.Key_F12: 24}
        fkeys = {Qt.Key.Key_F1: "P", Qt.Key.Key_F2: "Q", Qt.Key.Key_F3: "R", Qt.Key.Key_F4: "S"}
        if k in arrows:
            c = arrows[k]
            seq = f"\x1b[1;{mod_n}{c}" if mod_n > 1 else (f"\x1bO{c}" if app else f"\x1b[{c}")
        elif k in tilde:
            seq = f"\x1b[{tilde[k]};{mod_n}~" if mod_n > 1 else f"\x1b[{tilde[k]}~"
        elif k in fkeys:
            seq = f"\x1b[1;{mod_n}{fkeys[k]}" if mod_n > 1 else f"\x1bO{fkeys[k]}"
        elif k in (Qt.Key.Key_Return, Qt.Key.Key_Enter):
            seq = "\x1b\r" if alt else "\r"
        elif k == Qt.Key.Key_Backspace:
            seq = "\x08" if ctrl else ("\x1b\x7f" if alt else "\x7f")
        elif k == Qt.Key.Key_Tab:
            seq = "\t"
        elif k == Qt.Key.Key_Backtab:
            seq = "\x1b[Z"
        elif k == Qt.Key.Key_Escape:
            seq = "\x1b"
        elif ctrl and Qt.Key.Key_A <= k <= Qt.Key.Key_Z:
            seq = chr(k - Qt.Key.Key_A + 1)
            if alt:
                seq = "\x1b" + seq
        elif ctrl and k == Qt.Key.Key_Space:
            seq = "\x00"
        elif ctrl and k == Qt.Key.Key_BracketLeft:
            seq = "\x1b"
        elif ctrl and k == Qt.Key.Key_Backslash:
            seq = "\x1c"
        elif ctrl and k == Qt.Key.Key_BracketRight:
            seq = "\x1d"
        elif ctrl and k == Qt.Key.Key_Underscore:
            seq = "\x1f"
        else:
            t = e.text()
            if t:
                seq = ("\x1b" + t) if alt and not ctrl else t
        if seq:
            if self.scroll:
                self.scroll = 0
            if self.has_selection() and not self._selecting:
                self.sel_start = self.sel_end = None
            self._cursor_on = True
            self.write(seq)
            self.update()
            return
        super().keyPressEvent(e)

    def inputMethodEvent(self, e):
        if e.commitString():
            self.write(e.commitString())
        e.accept()

    def inputMethodQuery(self, q):
        if q == Qt.InputMethodQuery.ImCursorRectangle:
            cx = self.PAD + self.screen.cursor.x * self.cw
            cy = self.PAD + self.screen.cursor.y * self.ch
            return QRect(int(cx), int(cy), int(self.cw), int(self.ch))
        return super().inputMethodQuery(q)
