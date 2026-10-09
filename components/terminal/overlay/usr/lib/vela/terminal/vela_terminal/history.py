"""Searchable history of AI-terminal intents and commands (SQLite + FTS5 when available)."""
from __future__ import annotations

import json
import os
import sqlite3
import time


def default_path() -> str:
    base = os.environ.get("XDG_DATA_HOME") or os.path.expanduser("~/.local/share")
    return os.path.join(base, "vela", "terminal", "history.sqlite")


class History:
    def __init__(self, path: str | None = None):
        self.path = path or default_path()
        os.makedirs(os.path.dirname(self.path), mode=0o700, exist_ok=True)
        self.db = sqlite3.connect(self.path)
        self.db.execute("PRAGMA journal_mode=WAL")
        self.db.execute("""CREATE TABLE IF NOT EXISTS entries(
            id INTEGER PRIMARY KEY, ts REAL, intent TEXT, commands TEXT, cwd TEXT,
            rc INTEGER, mode TEXT, summary TEXT)""")
        self.fts = True
        try:
            self.db.execute("CREATE VIRTUAL TABLE IF NOT EXISTS entries_fts USING fts5("
                            "intent, commands, summary, content='entries', content_rowid='id')")
        except sqlite3.OperationalError:
            self.fts = False
        self.db.commit()

    def add(self, intent: str, commands: list, cwd: str = "", rc: int | None = None,
            mode: str = "ask", summary: str = "") -> int:
        cur = self.db.execute("INSERT INTO entries(ts,intent,commands,cwd,rc,mode,summary) VALUES(?,?,?,?,?,?,?)",
                              (time.time(), intent, json.dumps(commands, ensure_ascii=False), cwd, rc, mode, summary))
        rid = cur.lastrowid
        if self.fts:
            self.db.execute("INSERT INTO entries_fts(rowid,intent,commands,summary) VALUES(?,?,?,?)",
                            (rid, intent, " ; ".join(commands), summary))
        self.db.commit()
        return rid

    def update_rc(self, rid: int, rc: int):
        self.db.execute("UPDATE entries SET rc=? WHERE id=?", (rc, rid))
        self.db.commit()

    def _row(self, r):
        return {"id": r[0], "ts": r[1], "intent": r[2], "commands": json.loads(r[3] or "[]"),
                "cwd": r[4], "rc": r[5], "mode": r[6], "summary": r[7]}

    def search(self, query: str = "", limit: int = 50) -> list:
        q = (query or "").strip()
        cols = "e.id,e.ts,e.intent,e.commands,e.cwd,e.rc,e.mode,e.summary"
        if not q:
            rows = self.db.execute(f"SELECT {cols} FROM entries e ORDER BY e.id DESC LIMIT ?", (limit,)).fetchall()
            return [self._row(r) for r in rows]
        rows = []
        if self.fts:
            terms = [t for t in q.replace('"', " ").split() if t]
            match = " ".join(f'"{t}"*' for t in terms)
            try:
                rows = self.db.execute(
                    f"SELECT {cols} FROM entries_fts f JOIN entries e ON e.id=f.rowid "
                    f"WHERE entries_fts MATCH ? ORDER BY rank LIMIT ?", (match, limit)).fetchall()
            except sqlite3.OperationalError:
                rows = []
        if not rows:
            like = f"%{q}%"
            rows = self.db.execute(f"SELECT {cols} FROM entries e WHERE e.intent LIKE ? OR e.commands LIKE ? "
                                   f"ORDER BY e.id DESC LIMIT ?", (like, like, limit)).fetchall()
        return [self._row(r) for r in rows]

    def recent_intents(self, n=4) -> list:
        return list(reversed(self.search("", n)))

    def clear(self):
        self.db.execute("DELETE FROM entries")
        if self.fts:
            self.db.execute("INSERT INTO entries_fts(entries_fts) VALUES('delete-all')")
        self.db.commit()
