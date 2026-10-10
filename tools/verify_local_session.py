#!/usr/bin/env python3
"""Real-model check in a disposable Vela live session; only modifies its own fixture."""
import json
import hashlib
import os
from pathlib import Path
import stat
import sys
import tempfile
import time
import urllib.error
import urllib.request

sys.path.insert(0, "/usr/lib/vela/assistant")
from core import agent, config, workflows

runtime = os.environ.setdefault("XDG_RUNTIME_DIR", f"/run/user/{os.getuid()}")
os.environ.setdefault("DBUS_SESSION_BUS_ADDRESS", "unix:path=" + runtime + "/bus")
ep = config.endpoint()
assert ep["base_url"] == "http://127.0.0.1:11435/v1" and not ep["cloud"]
key = Path(os.environ["XDG_RUNTIME_DIR"]) / "vela-local-ai.key"
assert stat.S_IMODE(key.stat().st_mode) == 0o600
model = Path("/usr/share/vela/models/gemma-4-E2B.gguf")
h = hashlib.sha256()
with model.open("rb") as f:
    for block in iter(lambda: f.read(8 * 1024 * 1024), b""):
        h.update(block)
assert h.hexdigest() == "fa401b55b07ee70a54c6dae3903c783a6e65064312529ea57175cb5f8dec6634"
try:
    urllib.request.urlopen(urllib.request.Request(ep["base_url"] + "/chat/completions", data=b"{}",
                                                headers={"Content-Type": "application/json"}), timeout=10)
    raise AssertionError("Unauthenticated inference was accepted")
except urllib.error.HTTPError as exc:
    assert exc.code == 401

folder = Path(tempfile.mkdtemp(prefix="Vela-verifica-", dir=Path.home()))
original = {"foto.png": b"fixture image", "video.mp4": b"fixture video", "lavoro.pdf": b"preserve document"}
for name, data in original.items():
    (folder / name).write_bytes(data)
events = []
a = agent.Agent(lambda kind, payload: events.append((kind, payload)))
started = time.perf_counter()
assert a.ask("Per favore organizza la cartella " + str(folder) + " per tipo di file.")
a.thread.join(240)
assert not a.busy(), "Inference did not finish"
cards = [c for c in a.cards.values() if c["tool"] == "organize_folder" and c["state"] == "pending"]
assert len(cards) == 1, events
proposal_seconds = time.perf_counter() - started
assert {p.name: p.read_bytes() for p in folder.iterdir()} == original
card = cards[0]
assert os.path.realpath(os.path.expanduser(card["args"]["folder"])) == str(folder), "Model proposed another folder"
a.confirm(card["id"])
deadline = time.monotonic() + 30
while card["state"] == "running" and time.monotonic() < deadline:
    time.sleep(.1)
assert card["state"] == "done", card
b = agent.Agent(lambda *x: None)
recovery = "recovery-" + card["undo_data"]["operation"]
b.undo(recovery)
assert b.cards[recovery]["state"] == "undone"
assert {p.name: p.read_bytes() for p in folder.iterdir()} == original
for p in folder.iterdir():
    p.unlink()
folder.rmdir()
report = {"real_model_proposal_seconds": proposal_seconds, "preview_no_mutations": True,
          "installed_model_sha256": h.hexdigest(),
          "confirmed_moves": True, "undo_new_controller": True, "original_bytes_restored": True,
          "local_endpoint": True, "unauthenticated_inference_rejected": True,
          "session_key_0600": True, "audio_state": workflows.audio_state(),
          "notes": "Run with VM network cable disconnected to validate offline operation. Audio audibility not tested."}
print(json.dumps(report, indent=2, ensure_ascii=False))
