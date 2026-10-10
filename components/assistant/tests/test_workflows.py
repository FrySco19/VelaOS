import json
import os
from pathlib import Path

import pytest

from core import agent, config, tools, workflows


@pytest.fixture(autouse=True)
def fresh_permissions():
    config.save_user({"enabled": True, "permissions": {"files": "ask"}})


@pytest.fixture
def folder(tmp_path):
    p = Path(os.path.expanduser("~")) / ("fixture-" + tmp_path.name)
    p.mkdir()
    (p / "foto.png").write_bytes(b"original image")
    (p / "video.mp4").write_bytes(b"original video")
    (p / "lavoro.pdf").write_bytes(b"do not move")
    return p


def test_preview_does_not_move_and_restart_can_undo(folder):
    a = agent.Agent(lambda *x: None)
    card = a._make_card("organize_folder", {"folder": str(folder)}, "call_folder")
    assert card["state"] == "pending" and (folder / "foto.png").exists()
    a._execute(card, True)
    assert card["state"] == "done"
    assert (folder / "Immagini/foto.png").read_bytes() == b"original image"
    assert (folder / "lavoro.pdf").exists()
    b = agent.Agent(lambda *x: None)
    restored = b.cards["recovery-" + card["undo_data"]["operation"]]
    b.undo(restored["id"])
    assert restored["state"] == "undone"
    assert (folder / "foto.png").read_bytes() == b"original image"
    assert not (folder / "Immagini").exists()


def test_undo_refused_while_another_operation_runs(folder):
    a = agent.Agent(lambda *x: None)
    card = a._make_card("organize_folder", {"folder": str(folder)}, "call_folder")
    a._execute(card, True)
    a.operation_lock.acquire()
    try:
        a.undo(card["id"])
        assert card["state"] == "done" and (folder / "Immagini/foto.png").exists()
    finally:
        a.operation_lock.release()
    a.undo(card["id"])
    assert card["state"] == "undone"


@pytest.mark.parametrize("changes", [{"enabled": False}, {"permissions": {"files": "never"}}])
def test_revoked_permission_blocks_pending_confirmation(folder, changes):
    a = agent.Agent(lambda *x: None)
    card = a._make_card("organize_folder", {"folder": str(folder)}, None)
    assert card["state"] == "pending"
    config.save_user(changes)
    a.confirm(card["id"])
    assert card["state"] == "failed" and (folder / "foto.png").exists()
    assert not (folder / "Immagini").exists()


def test_disabled_blocks_native_intent(folder):
    config.save_user({"enabled": False})
    a = agent.Agent(lambda *x: None)
    a.run_intent("organize_folder", {"folder": str(folder)})
    assert all(card["state"] == "failed" for card in a.cards.values())
    assert (folder / "foto.png").exists()


def test_modified_after_preview_refused_before_any_move(folder):
    rec = workflows.plan(str(folder))
    (folder / "video.mp4").write_bytes(b"changed")
    with pytest.raises(workflows.WorkflowError):
        workflows.execute(rec["id"])
    assert (folder / "foto.png").exists()
    assert not (folder / "Immagini").exists()


def test_collision_after_preview_never_overwritten(folder):
    rec = workflows.plan(str(folder))
    (folder / "Immagini").mkdir()
    (folder / "Immagini/foto.png").write_bytes(b"another file")
    with pytest.raises(workflows.WorkflowError):
        workflows.execute(rec["id"])
    assert (folder / "Immagini/foto.png").read_bytes() == b"another file"
    assert (folder / "foto.png").exists()


def test_mid_move_crash_recovers_duplicate_link(folder, monkeypatch):
    rec = workflows.plan(str(folder))
    unlink = os.unlink
    def crash(path, *args, **kwargs):
        if path == str(folder / "foto.png"):
            raise OSError("simulated interruption after link")
        return unlink(path, *args, **kwargs)
    with monkeypatch.context() as m:
        m.setattr(os, "unlink", crash)
        with pytest.raises(workflows.WorkflowError):
            workflows.execute(rec["id"])
    workflows.undo(rec["id"])
    assert (folder / "foto.png").read_bytes() == b"original image"
    assert not (folder / "Immagini").exists()


def test_abrupt_process_exit_recovers_durable_plan(folder):
    import subprocess
    import sys
    rec = workflows.plan(str(folder))
    code = '''
import os, sys
from core import workflows
unlink = os.unlink
def exit_after_link(path, *args, **kwargs):
    if path.endswith('/foto.png'):
        os._exit(91)
    return unlink(path, *args, **kwargs)
os.unlink = exit_after_link
workflows.execute(sys.argv[1])
'''
    env = dict(os.environ, PYTHONPATH=str(Path(__file__).resolve().parents[1] / "overlay/usr/lib/vela/assistant"))
    child = subprocess.run([sys.executable, "-c", code, rec["id"]], env=env)
    assert child.returncode == 91
    assert workflows.load(rec["id"])["state"] == "running"
    workflows.undo(rec["id"])
    assert (folder / "foto.png").read_bytes() == b"original image"
    assert not (folder / "Immagini").exists()


def test_undo_stops_on_original_path_conflict(folder):
    rec = workflows.execute(workflows.plan(str(folder))["id"])
    (folder / "foto.png").write_bytes(b"new file")
    with pytest.raises(workflows.WorkflowError):
        workflows.undo(rec["id"])
    assert (folder / "foto.png").read_bytes() == b"new file"
    assert (folder / "Immagini/foto.png").read_bytes() == b"original image"


def test_symlink_destination_blocked(folder, tmp_path):
    (folder / "Immagini").symlink_to(tmp_path, target_is_directory=True)
    rec = workflows.plan(str(folder))
    assert all(not e["source"].endswith("foto.png") for e in rec["entries"])


@pytest.mark.parametrize("args", [{"folder": "/tmp", "_plan_id": "fake"}, {"folder": 1}, {},
                                  {"folder": "x", "extra": True}])
def test_invalid_model_arguments_cannot_create_plan(args):
    a = agent.Agent(lambda *x: None)
    card = a._make_card("organize_folder", args, "malformed")
    assert card["state"] == "failed"


def test_model_cannot_call_shell():
    assert "run_command" not in [s["function"]["name"] for s in tools.schemas()]
    a = agent.Agent(lambda *x: None)
    assert a._make_card("run_command", {"command": "echo unsafe"}, "shell")["state"] == "failed"


def test_fake_app_not_in_catalog():
    with pytest.raises(workflows.WorkflowError):
        workflows.catalog_app("vlc; rm -rf ~")


@pytest.mark.parametrize("index", range(30))
def test_real_file_workflow_roundtrip(folder, index):
    original = {p.name: p.read_bytes() for p in folder.iterdir()}
    rec = workflows.execute(workflows.plan(str(folder))["id"])
    assert rec["state"] == "done"
    assert (folder / "lavoro.pdf").read_bytes() == original["lavoro.pdf"]
    workflows.undo(rec["id"])
    assert {p.name: p.read_bytes() for p in folder.iterdir()} == original


def test_audio_preflight_stale_state_refused(monkeypatch):
    states = iter([{"default": "old", "sinks": [{"name": "new", "description": "Headphones"}]},
                   {"default": "changed", "sinks": [{"name": "new"}]}])
    monkeypatch.setattr(workflows, "audio_state", lambda: next(states))
    t = tools.SetAudioOutput()
    args = {"name": "new"}
    t.preview(args)
    with pytest.raises(tools.ToolError):
        t.run(args)


def test_local_missing_server_never_reported_ready(monkeypatch):
    from core import llm
    monkeypatch.setattr(llm, "_get_json", lambda *a, **k: (_ for _ in ()).throw(OSError("down")))
    assert llm.check_ready({"enabled": True}, {"base_url": "http://127.0.0.1:11435/v1",
                          "model": "vela-local", "provider": "openai", "cloud": False})["state"] == "local_loading"


def test_cancel_interrupts_wait_for_first_token():
    import http.server
    import threading
    import time
    from core import llm
    started = threading.Event()
    class Handler(http.server.BaseHTTPRequestHandler):
        def do_POST(self):
            self.rfile.read(int(self.headers["Content-Length"]))
            self.send_response(200)
            self.send_header("Content-Type", "text/event-stream")
            self.end_headers()
            self.wfile.flush()
            started.set()
            time.sleep(2)
        def log_message(self, *args):
            pass
    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    server.daemon_threads = True
    threading.Thread(target=server.serve_forever, daemon=True).start()
    cancel, outcome = threading.Event(), []
    def request():
        try:
            list(llm.stream_chat({"base_url": f"http://127.0.0.1:{server.server_port}/v1", "model": "test"},
                                 [{"role": "user", "content": "hello"}], cancel=cancel))
        except llm.Cancelled:
            outcome.append("cancelled")
    worker = threading.Thread(target=request, daemon=True)
    try:
        worker.start()
        assert started.wait(1)
        cancel.set()
        worker.join(1)
        assert not worker.is_alive() and outcome == ["cancelled"]
    finally:
        server.shutdown()
        server.server_close()
