"""Mock OpenAI-compatible (and minimal Ollama-native) server for tests.

Behaviour is driven by the last user message:
  contains "SPOSTA"      -> tool call move_file(source=..., destination=...) parsed as "SPOSTA <src> -> <dst>"
  contains "CERCA"       -> tool call find_files(query=<rest>) then, after the tool result, a text summary
  contains "TEMA"        -> tool call set_dark_mode(enabled=true)
  contains "ERRORE401"   -> HTTP 401
  contains "NOTOOLS"     -> HTTP 400 "tools not supported" when tools are present, else text
  contains "LENTO"       -> slow stream (for cancel)
  otherwise              -> streams "Risposta: <text>" in several chunks, with markdown
Requests are recorded in Mock.requests.
"""
import json
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer


class Mock:
    requests = []
    models = ["qwen2.5:1.5b"]
    capabilities = ["completion", "tools"]


def _last_user(msgs):
    for m in reversed(msgs):
        if m["role"] == "user":
            c = m["content"]
            return c if isinstance(c, str) else " ".join(p.get("text", "") for p in c if p.get("type") == "text")
    return ""


class H(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def log_message(self, *a):
        pass

    def _json(self, code, obj):
        b = json.dumps(obj).encode()
        self.send_response(code)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(b)))
        self.end_headers()
        self.wfile.write(b)

    def do_GET(self):
        if self.path == "/api/tags":
            return self._json(200, {"models": [{"name": m} for m in Mock.models]})
        self._json(404, {"error": "not found"})

    def _sse(self, chunks):
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream")
        self.send_header("Transfer-Encoding", "chunked")
        self.end_headers()

        def w(s):
            b = s.encode()
            self.wfile.write(f"{len(b):X}\r\n".encode() + b + b"\r\n")
            self.wfile.flush()
        for c in chunks:
            if c == "SLEEP":
                time.sleep(0.3)
                continue
            w("data: " + json.dumps(c) + "\n\n")
        w("data: [DONE]\n\n")
        self.wfile.write(b"0\r\n\r\n")

    def do_POST(self):
        n = int(self.headers.get("Content-Length", 0))
        body = json.loads(self.rfile.read(n) or b"{}")
        if self.path == "/api/show":
            return self._json(200, {"capabilities": Mock.capabilities})
        if not self.path.endswith("/chat/completions"):
            return self._json(404, {"error": "not found"})
        Mock.requests.append({"body": body, "auth": self.headers.get("Authorization")})
        msgs = body.get("messages", [])
        last = _last_user(msgs)
        after_tool = msgs and msgs[-1]["role"] == "tool"

        def delta(**d):
            return {"choices": [{"index": 0, "delta": d, "finish_reason": None}]}

        def finish(r):
            return {"choices": [{"index": 0, "delta": {}, "finish_reason": r}]}

        if "ERRORE401" in last:
            return self._json(401, {"error": {"message": "invalid api key"}})
        if "NOTOOLS" in last and body.get("tools"):
            return self._json(400, {"error": {"message": "registry.ollama.ai/x does not support tools"}})
        if after_tool:
            res = msgs[-1]["content"]
            return self._sse([delta(role="assistant"), delta(content="Ecco cosa ho trovato: "),
                              delta(content=res[:80]), finish("stop")])
        import re
        mv = re.search(r"[Ss]posta (\S+) in (\S+)", last)
        if "SPOSTA" in last or mv:
            if mv:
                src, dst = mv.group(1), mv.group(2)
            else:
                src, dst = [s.strip() for s in last.split("SPOSTA", 1)[1].split("->")]
            args = json.dumps({"source": src, "destination": dst})
            return self._sse([delta(role="assistant"),
                              delta(tool_calls=[{"index": 0, "id": "call_1", "type": "function",
                                                 "function": {"name": "move_file", "arguments": args[:10]}}]),
                              delta(tool_calls=[{"index": 0, "function": {"arguments": args[10:]}}]),
                              finish("tool_calls")])
        if "CERCA" in last:
            q = last.split("CERCA", 1)[1].strip().split("\n")[0]
            return self._sse([delta(role="assistant"),
                              delta(tool_calls=[{"index": 0, "id": "call_f", "type": "function",
                                                 "function": {"name": "find_files",
                                                              "arguments": json.dumps({"query": q})}}]),
                              finish("tool_calls")])
        if "TEMA" in last:
            return self._sse([delta(role="assistant"),
                              delta(tool_calls=[{"index": 0, "id": "call_t", "type": "function",
                                                 "function": {"name": "set_dark_mode",
                                                              "arguments": "{\"enabled\": true}"}}]),
                              finish("tool_calls")])
        chunks = [delta(role="assistant")]
        words = ("Risposta: **" + last.split("\n")[0][:60] + "**\n\n- uno\n- due").split(" ")
        for i, wd in enumerate(words):
            chunks.append(delta(content=wd + (" " if i < len(words) - 1 else "")))
            if "LENTO" in last:
                chunks.append("SLEEP")
        chunks.append(finish("stop"))
        return self._sse(chunks)


def start(port=0):
    srv = ThreadingHTTPServer(("127.0.0.1", port), H)
    t = threading.Thread(target=srv.serve_forever, daemon=True)
    t.start()
    return srv, f"http://127.0.0.1:{srv.server_address[1]}"


if __name__ == "__main__":
    import sys
    srv, url = start(int(sys.argv[1]) if len(sys.argv) > 1 else 18080)
    print(url, flush=True)
    try:
        while True:
            time.sleep(3600)
    except KeyboardInterrupt:
        pass
