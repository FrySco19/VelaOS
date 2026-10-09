"""Client for the Vela safe executor service (stdlib only)."""
from __future__ import annotations

import itertools
import json
import os
import socket

from .executor_service import socket_path


class ExecutorError(Exception):
    def __init__(self, code, message):
        super().__init__(f"{code}: {message}")
        self.code = code
        self.message = message


class ExecutorClient:
    def __init__(self, path: str | None = None, timeout: float = 900):
        self.path = path or socket_path()
        self.timeout = timeout
        self._ids = itertools.count(1)
        self.sock = None
        self.buf = b""

    def connect(self):
        s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        s.settimeout(self.timeout)
        s.connect(self.path)
        self.sock = s
        return self

    def close(self):
        if self.sock:
            self.sock.close()
            self.sock = None

    def __enter__(self):
        return self.connect()

    def __exit__(self, *a):
        self.close()

    def _readline(self):
        while b"\n" not in self.buf:
            chunk = self.sock.recv(65536)
            if not chunk:
                raise ExecutorError("disconnected", "connection closed")
            self.buf += chunk
        line, self.buf = self.buf.split(b"\n", 1)
        return json.loads(line)

    def call(self, method, on_event=None, **params):
        if not self.sock:
            self.connect()
        rid = next(self._ids)
        self.sock.sendall((json.dumps({"id": rid, "method": method, "params": params}) + "\n").encode())
        while True:
            msg = self._readline()
            if msg.get("id") != rid:
                continue
            if "event" in msg:
                if on_event:
                    on_event(msg)
                continue
            if "error" in msg:
                raise ExecutorError(msg["error"].get("code"), msg["error"].get("message"))
            return msg["result"]
