#!/usr/bin/python3
"""Tiny OpenAI-compatible mock (TEST ONLY): streams a fixed answer via SSE on /v1/chat/completions.
It echoes how many messages it received so tests can check the request shape. Not a model."""
import json, sys
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

class H(BaseHTTPRequestHandler):
    def log_message(self, *a): pass
    def do_POST(self):
        body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
        if body.get("model") == "missing":
            self.send_response(404); self.end_headers(); self.wfile.write(b'{"error":{"message":"model \\"missing\\" not found"}}'); return
        last = body["messages"][-1]["content"]
        if "Campi del modulo" in last:
            fields = json.loads(last.split("Campi del modulo:\n", 1)[1].split("\n\nDati", 1)[0])
            ans = json.dumps({f["id"]: ("Mario Rossi" if "nome" in f["label"].lower() else "mario@example.com")
                              for f in fields if any(k in f["label"].lower() for k in ("nome", "mail"))})
        else:
            ans = f"- Punto uno della pagina\n- Punto **due**\n\nRichiesta con {len(body['messages'])} messaggi (MOCK)."
        self.send_response(200); self.send_header("Content-Type", "text/event-stream"); self.end_headers()
        for i in range(0, len(ans), 8):
            self.wfile.write(b"data: " + json.dumps({"choices": [{"delta": {"content": ans[i:i+8]}}]}).encode() + b"\n\n")
            self.wfile.flush()
        self.wfile.write(b"data: [DONE]\n\n")

ThreadingHTTPServer(("127.0.0.1", int(sys.argv[1]) if len(sys.argv) > 1 else 18080), H).serve_forever()
