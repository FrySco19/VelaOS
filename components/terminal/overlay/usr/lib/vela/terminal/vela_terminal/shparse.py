"""A small, conservative POSIX/bash command-line parser used by the danger classifier.

It does NOT execute anything. It splits a command line into pipelines of simple
commands, keeping track of redirections and of nested command strings
($(...), `...`, <(...), >(...), here-documents) so that they can be analysed
recursively. When something cannot be parsed reliably, ``ParseResult.uncertain``
is set and callers must treat the command as risky.
"""
from __future__ import annotations

from dataclasses import dataclass, field
import re

OPERATORS = ("&&", "||", ";;", "|&", ";", "&", "|", "(", ")", "\n")
REDIR_RE = re.compile(r"(\d*|&)(>>|>\||>&|<<<|<<-|<<|<>|<&|>|<)")


@dataclass
class Word:
    value: str            # unquoted value (quotes removed, escapes resolved)
    raw: str              # as written
    quoted: bool = False  # any part was quoted
    has_expansion: bool = False  # contains $VAR, $(...), `...` or unquoted glob chars
    has_glob: bool = False


@dataclass
class Redirect:
    op: str
    target: str
    fd: str = ""


@dataclass
class SimpleCommand:
    words: list = field(default_factory=list)       # list[Word]
    redirects: list = field(default_factory=list)   # list[Redirect]
    raw: str = ""

    @property
    def argv(self):
        return [w.value for w in self.words]


@dataclass
class Pipeline:
    commands: list = field(default_factory=list)    # list[SimpleCommand]
    background: bool = False


@dataclass
class ParseResult:
    pipelines: list = field(default_factory=list)
    substitutions: list = field(default_factory=list)  # nested command strings
    heredocs: list = field(default_factory=list)       # (command_index_key, body)
    uncertain: bool = False
    errors: list = field(default_factory=list)


class _Lexer:
    def __init__(self, text: str):
        self.s = text
        self.i = 0
        self.n = len(text)
        self.subs: list[str] = []
        self.uncertain = False
        self.errors: list[str] = []

    def peek(self, k=0):
        j = self.i + k
        return self.s[j] if j < self.n else ""

    def _balanced(self, open_ch: str, close_ch: str) -> str:
        """Read until the matching close_ch (self.i is just after the opener)."""
        depth = 1
        start = self.i
        quote = None
        while self.i < self.n:
            c = self.s[self.i]
            if quote:
                if c == "\\" and quote == '"':
                    self.i += 2
                    continue
                if c == quote:
                    quote = None
            else:
                if c in ("'", '"'):
                    quote = c
                elif c == "\\":
                    self.i += 2
                    continue
                elif c == open_ch:
                    depth += 1
                elif c == close_ch:
                    depth -= 1
                    if depth == 0:
                        body = self.s[start:self.i]
                        self.i += 1
                        return body
            self.i += 1
        self.uncertain = True
        self.errors.append("unbalanced " + open_ch)
        return self.s[start:]

    def _backtick(self) -> str:
        start = self.i
        while self.i < self.n:
            c = self.s[self.i]
            if c == "\\":
                self.i += 2
                continue
            if c == "`":
                body = self.s[start:self.i]
                self.i += 1
                return body
            self.i += 1
        self.uncertain = True
        self.errors.append("unbalanced `")
        return self.s[start:]

    def tokens(self):
        """Yield ('op', str) | ('word', Word) | ('redir', (fd, op))."""
        while self.i < self.n:
            c = self.s[self.i]
            if c in " \t\r":
                self.i += 1
                continue
            if c == "\\" and self.peek(1) == "\n":
                self.i += 2
                continue
            if c == "#":
                while self.i < self.n and self.s[self.i] != "\n":
                    self.i += 1
                continue
            # process substitution <( ... ) >( ... )
            if c in "<>" and self.peek(1) == "(":
                self.i += 2
                body = self._balanced("(", ")")
                self.subs.append(body)
                yield ("word", Word(value=f"{c}(...)", raw=f"{c}({body})", has_expansion=True))
                continue
            m = REDIR_RE.match(self.s, self.i) if (c.isdigit() or c in "<>&") else None
            if m and not (c == "&" and m.group(2) is None):
                # avoid treating '&&' or '& ' as redirect
                if c == "&" and not self.s.startswith(("&>", "&>>"), self.i):
                    m = None
            if m:
                fd, op = m.group(1), m.group(2)
                # a digit sequence followed by a redirection operator, e.g. 2>
                self.i = m.end()
                yield ("redir", (fd, op))
                continue
            matched_op = None
            for op in OPERATORS:
                if self.s.startswith(op, self.i):
                    matched_op = op
                    break
            if matched_op:
                self.i += len(matched_op)
                yield ("op", matched_op)
                continue
            before = self.i
            w = self._word()
            if self.i == before:  # defensive: unknown char, consume it
                self.i += 1
                self.uncertain = True
                continue
            yield ("word", w)

    def _word(self) -> Word:
        val = []
        start = self.i
        quoted = False
        expansion = False
        glob = False
        while self.i < self.n:
            c = self.s[self.i]
            if c in " \t\r\n;&|()<>":
                # '<' or '>' inside word ends it (redirection)
                break
            if c == "'":
                quoted = True
                j = self.s.find("'", self.i + 1)
                if j < 0:
                    self.uncertain = True
                    self.errors.append("unterminated '")
                    val.append(self.s[self.i + 1:])
                    self.i = self.n
                    break
                val.append(self.s[self.i + 1:j])
                self.i = j + 1
                continue
            if c == "$" and self.peek(1) == "'":
                quoted = True
                j = self.i + 2
                buf = []
                while j < self.n and self.s[j] != "'":
                    if self.s[j] == "\\" and j + 1 < self.n:
                        esc = {"n": "\n", "t": "\t", "r": "\r", "\\": "\\", "'": "'", '"': '"',
                               "a": "\a", "b": "\b", "e": "\x1b", "E": "\x1b", "0": "\0"}
                        buf.append(esc.get(self.s[j + 1], self.s[j:j + 2]))
                        j += 2
                        continue
                    buf.append(self.s[j])
                    j += 1
                val.append("".join(buf))
                self.i = j + 1
                continue
            if c == '"':
                quoted = True
                self.i += 1
                while self.i < self.n and self.s[self.i] != '"':
                    d = self.s[self.i]
                    if d == "\\" and self.peek(1) in '"\\$`\n':
                        val.append(self.peek(1))
                        self.i += 2
                        continue
                    if d == "$" and self.peek(1) == "(":
                        expansion = True
                        if self.peek(2) == "(":
                            self.i += 3
                            body = self._balanced("(", ")")
                            if self.peek() == ")":
                                self.i += 1
                            val.append("$((" + body + "))")
                            continue
                        self.i += 2
                        body = self._balanced("(", ")")
                        self.subs.append(body)
                        val.append("$(" + body + ")")
                        continue
                    if d == "`":
                        expansion = True
                        self.i += 1
                        body = self._backtick()
                        self.subs.append(body)
                        val.append("`" + body + "`")
                        continue
                    if d == "$":
                        expansion = True
                    val.append(d)
                    self.i += 1
                if self.i >= self.n:
                    self.uncertain = True
                    self.errors.append('unterminated "')
                self.i += 1
                continue
            if c == "\\":
                if self.i + 1 < self.n:
                    val.append(self.s[self.i + 1])
                self.i += 2
                quoted = True
                continue
            if c == "$" and self.peek(1) == "(":
                expansion = True
                if self.peek(2) == "(":
                    self.i += 3
                    body = self._balanced("(", ")")
                    if self.peek() == ")":
                        self.i += 1
                    val.append("$((" + body + "))")
                    continue
                self.i += 2
                body = self._balanced("(", ")")
                self.subs.append(body)
                val.append("$(" + body + ")")
                continue
            if c == "$" and self.peek(1) == "{":
                expansion = True
                self.i += 2
                body = self._balanced("{", "}")
                val.append("${" + body + "}")
                continue
            if c == "`":
                expansion = True
                self.i += 1
                body = self._backtick()
                self.subs.append(body)
                val.append("`" + body + "`")
                continue
            if c == "$":
                expansion = True
            if c in "*?[":
                glob = True
            if c == "{" and "," in self.s[self.i:self.s.find("}", self.i) if "}" in self.s[self.i:] else self.i]:
                glob = True
            val.append(c)
            self.i += 1
        return Word(value="".join(val), raw=self.s[start:self.i], quoted=quoted,
                    has_expansion=expansion, has_glob=glob)


def parse(text: str) -> ParseResult:
    text = text or ""
    res = ParseResult()
    lx = _Lexer(text)
    pipelines: list[Pipeline] = []
    cur_pipe = Pipeline()
    cur_cmd = SimpleCommand()
    pending_redir = None
    pending_heredocs: list[tuple[str, bool, SimpleCommand]] = []

    def finish_cmd():
        nonlocal cur_cmd
        if cur_cmd.words or cur_cmd.redirects:
            cur_pipe.commands.append(cur_cmd)
        cur_cmd = SimpleCommand()

    def finish_pipe(background=False):
        nonlocal cur_pipe
        finish_cmd()
        if cur_pipe.commands:
            cur_pipe.background = background
            pipelines.append(cur_pipe)
        cur_pipe = Pipeline()

    toks = lx.tokens()
    for kind, val in toks:
        if pending_redir is not None:
            fd, op = pending_redir
            if kind != "word":
                res.uncertain = True
                res.errors.append("redirection without target")
                pending_redir = None
            else:
                pending_redir = None
                if op in ("<<", "<<-"):
                    delim = val.value
                    pending_heredocs.append((delim, op == "<<-", cur_cmd))
                    cur_cmd.redirects.append(Redirect(op=op, target=delim, fd=fd))
                else:
                    cur_cmd.redirects.append(Redirect(op=op, target=val.value, fd=fd))
                continue
        if kind == "word":
            cur_cmd.words.append(val)
        elif kind == "redir":
            pending_redir = val
        elif kind == "op":
            if val == "|" or val == "|&":
                finish_cmd()
            elif val in ("&&", "||", ";", ";;"):
                finish_pipe()
            elif val == "&":
                finish_pipe(background=True)
            elif val == "\n":
                finish_pipe()
                # consume here-document bodies
                for delim, strip_tabs, cmd in pending_heredocs:
                    body_lines = []
                    while lx.i < lx.n:
                        j = lx.s.find("\n", lx.i)
                        line = lx.s[lx.i:] if j < 0 else lx.s[lx.i:j]
                        lx.i = lx.n if j < 0 else j + 1
                        check = line.lstrip("\t") if strip_tabs else line
                        if check == delim:
                            break
                        body_lines.append(line)
                    else:
                        if not body_lines or True:
                            pass
                    res.heredocs.append((cmd, "\n".join(body_lines)))
                pending_heredocs = []
            elif val in ("(", ")"):
                # subshell / function definition: just split commands
                finish_pipe()
    if pending_redir is not None:
        res.uncertain = True
        res.errors.append("redirection without target")
    finish_pipe()
    for delim, _s, cmd in pending_heredocs:
        res.heredocs.append((cmd, ""))
    res.pipelines = pipelines
    res.substitutions = lx.subs
    res.uncertain = res.uncertain or lx.uncertain
    res.errors += lx.errors
    return res
