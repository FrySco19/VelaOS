"""Tiny OpenAI-compatible mock server for tests (POST /v1/chat/completions, GET /api/tags).

Replies are canned and deterministic; used ONLY in automated tests, never shipped.
"""
import json
import sys
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

CALLS = []


def reply_for(messages):
    sys_p = messages[0]["content"]
    last = messages[-1]["content"]
    if "modalità agente" in sys_p:
        n_results = sum(1 for m in messages if m["role"] == "user" and m["content"].startswith("Risultato"))
        plan = ["Crea la cartella di prova", "Scrivi un file", "Controlla il contenuto"]
        cmds = ["mkdir -p prova_agente", "echo ciao > prova_agente/nota.txt", "cat prova_agente/nota.txt"]
        if n_results >= 3:
            return {"plan": plan, "next": None, "done": True, "message": "Il file contiene «ciao»."}
        return {"plan": plan, "next": {"command": cmds[n_results], "explanation": plan[n_results]}, "done": False,
                "message": ""}
    if "Un comando è fallito" in sys_p:
        return {"summary": "Il file non esiste.", "cause": "Hai chiesto di leggere un file assente.",
                "fix": {"command": "ls -la", "explanation": "Elenca i file presenti"}}
    if "pericoloso" in last or "cancella tutto" in last:
        return {"explanation": "Cancello la cartella di prova.",
                "commands": [{"command": "rm -rf prova_da_cancellare", "explanation": "cancella la cartella"}]}
    if "errore" in last:
        return {"explanation": "Leggo un file che non esiste.",
                "commands": [{"command": "cat file_che_non_esiste.txt", "explanation": "mostra il file"}]}
    return {"explanation": "Mostro lo spazio libero sul disco.",
            "commands": [{"command": "df -h ~", "explanation": "spazio usato e libero"},
                         {"command": "du -sh ~/* 2>/dev/null | sort -h | tail -n 5", "explanation": "cartelle più grandi"}],
            "note": ""}


class H(BaseHTTPRequestHandler):
    def log_message(self, *a):
        pass

    def _json(self, obj, code=200):
        b = json.dumps(obj).encode()
        self.send_response(code)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(b)))
        self.end_headers()
        self.wfile.write(b)

    def do_GET(self):
        if self.path == "/api/tags":
            self._json({"models": [{"name": "mock:latest"}]})
        else:
            self._json({}, 404)

    def do_POST(self):
        n = int(self.headers.get("Content-Length", 0))
        body = json.loads(self.rfile.read(n))
        CALLS.append(body)
        content = json.dumps(reply_for(body["messages"]), ensure_ascii=False)
        self._json({"choices": [{"message": {"role": "assistant", "content": content}}]})


def start(port=0):
    srv = ThreadingHTTPServer(("127.0.0.1", port), H)
    th = threading.Thread(target=srv.serve_forever, daemon=True)
    th.start()
    return srv


if __name__ == "__main__":
    s = start(int(sys.argv[1]) if len(sys.argv) > 1 else 11999)
    print("mock on", s.server_address, flush=True)
    threading.Event().wait()
