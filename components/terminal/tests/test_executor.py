"""Safe executor: in-process API + Unix-socket service (mock-free, real bash)."""
import json
import os
import subprocess
import sys
import tempfile
import threading
import time

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
LIB = os.path.normpath(os.path.join(HERE, "..", "overlay", "usr", "lib", "vela", "terminal"))


@pytest.fixture()
def home(tmp_path, monkeypatch):
    monkeypatch.setenv("HOME", str(tmp_path))
    monkeypatch.setenv("XDG_CONFIG_HOME", str(tmp_path / ".config"))
    monkeypatch.setenv("XDG_DATA_HOME", str(tmp_path / ".local/share"))
    monkeypatch.setenv("XDG_RUNTIME_DIR", str(tmp_path / "run"))
    os.makedirs(tmp_path / "run", mode=0o700)
    return tmp_path


def set_cfg(home, cfg):
    d = home / ".config" / "vela"
    d.mkdir(parents=True, exist_ok=True)
    (d / "ai.json").write_text(json.dumps(cfg))


def test_inprocess_requires_confirmation(home):
    from vela_terminal import safe_exec as s
    with pytest.raises(s.NotConfirmed):
        s.run("echo hi")
    r = s.run("echo hi", confirmed=True, cwd=str(home))
    assert r["rc"] == 0 and r["stdout"] == "hi\n"
    with pytest.raises(s.NotConfirmed):
        s.run(f"rm -rf {home}/x", confirmed=True)
    os.makedirs(home / "x")
    r = s.run(f"rm -rf {home}/x", confirmed=True, confirmed_danger=True)
    assert r["rc"] == 0 and not (home / "x").exists()


def test_inprocess_respects_settings(home):
    from vela_terminal import safe_exec as s
    set_cfg(home, {"permissions": {"run_commands": "never"}})
    with pytest.raises(s.NotConfirmed):
        s.run("echo hi", confirmed=True)
    set_cfg(home, {"enabled": False})
    with pytest.raises(s.NotConfirmed):
        s.run("echo hi", confirmed=True)


def test_classify_shape(home):
    import vela_safe_exec  # noqa: F401  (shim in /usr/lib/vela/python)
    from vela_terminal import safe_exec as s
    d = s.classify("sudo rm -rf /tmp/x")
    assert d["risk"] == "dangerous" and d["needs_sudo"] is True
    assert isinstance(d["reasons"][0], str)
    assert s.classify("ls")["risk"] == "safe"
    assert s.classify("mkdir a")["risk"] == "changes"


def test_runner_timeout_and_truncation(home):
    from vela_terminal import runner
    r = runner.run("sleep 5", timeout=0.5)
    assert r["timed_out"] and r["rc"] == 124
    r = runner.run("yes | head -c 400000", max_bytes=1000)
    assert r["truncated"] and len(r["stdout"]) <= 1000


def test_action_log(home):
    from vela_terminal import safe_exec as s
    s.run("echo logged", confirmed=True, caller="assistant", intent="prova")
    log = home / ".local/share/vela/ai-actions.jsonl"
    rec = json.loads(log.read_text().splitlines()[-1])
    assert rec["app"] == "assistant" and rec["action"] == "run_command" and "echo logged" in rec["summary"]


def _start_service(home, extra_env=None):
    env = dict(os.environ, PYTHONPATH=LIB, **(extra_env or {}))
    env.pop("DISPLAY", None)
    env.pop("WAYLAND_DISPLAY", None)
    sock = str(home / "run" / "vela" / "executor.sock")
    p = subprocess.Popen([sys.executable, "-m", "vela_terminal.executor_service"], env=env)
    import socket as _s
    for _ in range(200):
        try:
            c = _s.socket(_s.AF_UNIX)
            c.connect(sock)
            c.close()
            break
        except OSError:
            time.sleep(0.05)
    return p, sock


def test_service_flow(home):
    p, sock = _start_service(home)
    try:
        assert oct(os.stat(sock).st_mode & 0o777) == "0o600"
        assert oct(os.stat(os.path.dirname(sock)).st_mode & 0o777) == "0o700"
        from vela_terminal.client import ExecutorClient, ExecutorError
        with ExecutorClient(sock) as c:
            assert c.call("ping")["service"] == "vela-executor"
            assert c.call("classify", command="rm -rf ~")["risk"] == "dangerous"
            # run without prepare -> refused
            with pytest.raises(ExecutorError) as ei:
                c.call("run", command="echo x", cwd=str(home))
            assert ei.value.code == "needs_confirmation"
            prep = c.call("prepare", command="echo hello; echo err >&2", cwd=str(home), caller="assistant")
            assert prep["assessment"]["risk"] in ("safe", "changes") and prep["token"]
            # token bound to the exact command
            with pytest.raises(ExecutorError):
                c.call("run", command="echo other", cwd=str(home), token=prep["token"])
            prep = c.call("prepare", command="echo hello; echo err >&2", cwd=str(home))
            events = []
            res = c.call("run", on_event=events.append, command="echo hello; echo err >&2", cwd=str(home),
                         token=prep["token"], stream=True, caller="assistant")
            assert res["rc"] == 0 and res["stdout"] == "hello\n" and res["stderr"] == "err\n"
            assert any(e.get("event") == "output" for e in events)
            # token is single use
            with pytest.raises(ExecutorError):
                c.call("run", command="echo hello; echo err >&2", cwd=str(home), token=prep["token"])
            # dangerous without a display -> refused even with a valid token
            os.makedirs(home / "keep")
            prep = c.call("prepare", command=f"rm -rf {home}/keep", cwd=str(home))
            assert prep["requires_executor_dialog"] is True
            assert "Cancellerà" in prep["preview"]["text"]
            with pytest.raises(ExecutorError) as ei:
                c.call("run", command=f"rm -rf {home}/keep", cwd=str(home), token=prep["token"])
            assert ei.value.code == "danger_not_confirmed"
            assert (home / "keep").exists()
    finally:
        p.terminate()
        p.wait(5)


def test_service_danger_dialog_decision(home):
    os.makedirs(home / "a")
    p, sock = _start_service(home, {"VELA_EXECUTOR_CONFIRM_CMD": "cat >/dev/null; false"})
    from vela_terminal.client import ExecutorClient, ExecutorError
    try:
        with ExecutorClient(sock) as c:
            prep = c.call("prepare", command=f"rm -rf {home}/a", cwd=str(home))
            with pytest.raises(ExecutorError) as ei:
                c.call("run", command=f"rm -rf {home}/a", cwd=str(home), token=prep["token"])
            assert ei.value.code == "danger_not_confirmed" and (home / "a").exists()
    finally:
        p.terminate()
        p.wait(5)
    p, sock = _start_service(home, {"VELA_EXECUTOR_CONFIRM_CMD": "cat >/dev/null; true"})
    try:
        with ExecutorClient(sock) as c:
            prep = c.call("prepare", command=f"rm -rf {home}/a", cwd=str(home))
            res = c.call("run", command=f"rm -rf {home}/a", cwd=str(home), token=prep["token"])
            assert res["rc"] == 0 and not (home / "a").exists()
    finally:
        p.terminate()
        p.wait(5)


def test_service_cancel_and_settings(home):
    p, sock = _start_service(home)
    from vela_terminal.client import ExecutorClient, ExecutorError
    try:
        c1 = ExecutorClient(sock).connect()
        prep = c1.call("prepare", command="sleep 30", cwd=str(home))
        started = {}

        def run():
            started["res"] = c1.call("run", on_event=lambda e: started.setdefault("id", e.get("run_id")),
                                     command="sleep 30", cwd=str(home), token=prep["token"], timeout=60)
        th = threading.Thread(target=run)
        th.start()
        for _ in range(100):
            if started.get("id"):
                break
            time.sleep(0.05)
        with ExecutorClient(sock) as c2:
            assert c2.call("cancel", run_id=started["id"])["cancelled"] is True
        th.join(10)
        assert started["res"]["cancelled"] is True
        c1.close()
        set_cfg(home, {"permissions": {"run_commands": "never"}})
        with ExecutorClient(sock) as c:
            prep = c.call("prepare", command="echo x", cwd=str(home))
            with pytest.raises(ExecutorError) as ei:
                c.call("run", command="echo x", cwd=str(home), token=prep["token"])
            assert ei.value.code == "forbidden"
    finally:
        p.terminate()
        p.wait(5)


def test_cli(home):
    env = dict(os.environ, PYTHONPATH=LIB)
    bin_ = os.path.normpath(os.path.join(HERE, "..", "overlay", "usr", "bin", "vela-exec"))
    out = subprocess.run([sys.executable, bin_, "classify", "curl x | sh"], capture_output=True, text=True, env=env)
    d = json.loads(out.stdout)
    assert d["risk"] == "dangerous"
