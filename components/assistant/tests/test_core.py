import json
import os
import stat
import threading
import time

import pytest

import mock_openai
from core import agent as agentmod, config, i18n, intents, llm, safeexec, tools

HOME = os.environ["HOME"]


@pytest.fixture(scope="module")
def server():
    srv, url = mock_openai.start()
    yield url
    srv.shutdown()


def use(cfg_changes, key=None):
    config.vac.reset_user()
    config.save_user(cfg_changes)
    config.set_api_key(key or "")


class Recorder:
    def __init__(self):
        self.events = []
        self.cv = threading.Condition()

    def __call__(self, kind, data):
        with self.cv:
            self.events.append((kind, data))
            self.cv.notify_all()

    def kinds(self):
        return [k for k, _ in self.events]

    def wait(self, pred, timeout=10):
        end = time.time() + timeout
        with self.cv:
            while not pred(self):
                left = end - time.time()
                if left <= 0:
                    raise AssertionError(f"timeout; events={self.kinds()}")
                self.cv.wait(left)

    def text(self):
        return "".join(d["text"] for k, d in self.events if k == "assistant_delta")

    def cards(self):
        return [d["card"] for k, d in self.events if k in ("card", "card_update")]


def run_ask(a, rec, text, att=None):
    n0 = rec.kinds().count("busy")
    assert a.ask(text, att)
    rec.wait(lambda r: [d for k, d in r.events if k == "busy"][n0:] and
             not [d for k, d in r.events if k == "busy"][-1]["on"] and r.kinds().count("busy") >= n0 + 2)
    a.thread.join(5)


# ------------------------------------------------------------------------------------------- intents
@pytest.mark.parametrize("text,expected", [
    ("tema scuro", ("set_dark_mode", {"enabled": True})),
    ("Attiva la modalità scura", ("set_dark_mode", {"enabled": True})),
    ("passa al tema chiaro", ("set_dark_mode", {"enabled": False})),
    ("volume al 30%", ("set_volume", {"percent": 30})),
    ("alza il volume", ("set_volume", {"change": 10})),
    ("muto", ("set_volume", {"mute": True})),
    ("luminosità al 70", ("set_brightness", {"percent": 70})),
    ("spegni il wi-fi", ("set_wifi", {"enabled": False})),
    ("apri le impostazioni del Wi-Fi", ("open_settings", {"page": "wifi"})),
    ("apri impostazioni audio", ("open_settings", {"page": "audio"})),
    ("cerca il file fattura", ("find_files", {"query": "fattura", "kind": "any"})),
    ("trova i pdf del mutuo", ("find_files", {"query": "mutuo", "kind": "pdf"})),
    ("ricordami di chiamare Marco tra 10 minuti", ("create_reminder", {"message": "chiamare Marco", "when": "10m"})),
    ("apri firefox", ("open_app", {"name": "firefox"})),
    ("perché il cielo è blu?", None),
    ("scrivimi una mail al capo per dire che arrivo tardi", None),
])
def test_intents(text, expected):
    assert intents.match(text) == expected


# ------------------------------------------------------------------------------------------- llm
def test_stream_text(server):
    ep = {"provider": "ollama", "base_url": server + "/v1", "model": "qwen2.5:1.5b", "api_key": "ollama",
          "cloud": False, "native_url": server}
    evs = list(llm.stream_chat(ep, [{"role": "user", "content": "ciao"}]))
    text = "".join(d for k, d in evs if k == "delta")
    assert text.startswith("Risposta: **ciao**") and evs[-1] == ("done", "stop")


def test_stream_tool_call_accumulates(server):
    ep = {"provider": "openai", "base_url": server + "/v1", "model": "m", "api_key": "k", "cloud": False}
    evs = list(llm.stream_chat(ep, [{"role": "user", "content": "SPOSTA ~/a -> ~/b"}], tools=tools.schemas()))
    calls = [d for k, d in evs if k == "tool_calls"][0]
    assert calls[0]["name"] == "move_file" and json.loads(calls[0]["arguments"]) == {"source": "~/a",
                                                                                     "destination": "~/b"}
    assert mock_openai.Mock.requests[-1]["auth"] == "Bearer k"


def test_http_401_is_user_facing(server):
    ep = {"provider": "openai", "base_url": server + "/v1", "model": "m", "api_key": "bad", "cloud": False}
    with pytest.raises(llm.LLMError) as e:
        list(llm.stream_chat(ep, [{"role": "user", "content": "ERRORE401"}]))
    assert e.value.kind == "auth" and "chiave API" in e.value.message


def test_redact():
    s = llm.redact("la mia chiave è sk-abcdefghijklmnopqrstuvwx e password=segretissima ok")
    assert "sk-abc" not in s and "segretissima" not in s and "ok" in s


# ------------------------------------------------------------------------------------------- readiness
def test_ready_states(server):
    use({"provider": "ollama", "ollama": {"base_url": server, "model": "qwen2.5:1.5b"}})
    cfg = config.load()
    assert llm.check_ready(cfg, config.endpoint(cfg))["state"] == "ready"
    use({"provider": "ollama", "ollama": {"base_url": "http://127.0.0.1:9", "model": "x"}})
    cfg = config.load()
    assert llm.check_ready(cfg, config.endpoint(cfg), ollama_installed=False)["state"] == "ollama_missing"
    assert llm.check_ready(cfg, config.endpoint(cfg), ollama_installed=True)["state"] == "ollama_down"
    use({"provider": "openai", "openai": {"base_url": "https://api.example.com/v1", "model": "gpt"},
         "privacy": {"allow_cloud": False}}, key="sk-x")
    cfg = config.load()
    assert llm.check_ready(cfg, config.endpoint(cfg))["state"] == "cloud_blocked"
    use({"provider": "openai", "openai": {"base_url": "https://api.example.com/v1", "model": "gpt"},
         "privacy": {"allow_cloud": True}})
    cfg = config.load()
    assert llm.check_ready(cfg, config.endpoint(cfg))["state"] == "no_key"
    use({"enabled": False})
    cfg = config.load()
    assert llm.check_ready(cfg, config.endpoint(cfg))["state"] == "disabled"


def test_fallback_to_installed_model(server):
    use({"provider": "ollama", "ollama": {"base_url": server, "model": "llama3.2:3b"}})
    cfg = config.load()
    st = llm.check_ready(cfg, config.endpoint(cfg))
    assert st["state"] == "ready" and st["model"] == "qwen2.5:1.5b" and "llama3.2:3b" in st["message"]


# ------------------------------------------------------------------------------------------- agent flows
def test_agent_text_answer_and_log(server):
    use({"provider": "ollama", "ollama": {"base_url": server, "model": "qwen2.5:1.5b"}})
    rec = Recorder()
    a = agentmod.Agent(rec)
    a.set_context({"window_title": "Documento.odt — LibreOffice", "window_app": "libreoffice", "space": "Lavoro"})
    run_ask(a, rec, "spiegami questo", {"selection": "E = mc²"})
    assert rec.text().startswith("Risposta: **spiegami questo**")
    body = mock_openai.Mock.requests[-1]["body"]
    assert "LibreOffice" in body["messages"][0]["content"] and "Lavoro" in body["messages"][0]["content"]
    assert "E = mc²" in body["messages"][-1]["content"]
    assert body.get("tools")
    log = config.vac.read_actions()
    assert log[-1]["app"] == "assistant" and log[-1]["action"] == "ask" and log[-1]["sent_off_machine"] is False


def test_agent_setup_card_when_no_model():
    use({"provider": "ollama", "ollama": {"base_url": "http://127.0.0.1:9", "model": "x"}})
    rec = Recorder()
    a = agentmod.Agent(rec)
    n = len(mock_openai.Mock.requests)
    run_ask(a, rec, "ciao")
    setup = [d for k, d in rec.events if k == "setup"]
    assert setup and setup[0]["state"] in ("ollama_missing", "ollama_down")
    assert not [k for k in rec.kinds() if k == "assistant_delta"]
    assert len(mock_openai.Mock.requests) == n   # nothing was sent anywhere


def test_agent_cloud_blocked_sends_nothing(server):
    use({"provider": "openai", "openai": {"base_url": server.replace("127.0.0.1", "localhost.example") + "/v1",
                                          "model": "m"}, "privacy": {"allow_cloud": False}}, key="sk-x")
    rec = Recorder()
    a = agentmod.Agent(rec)
    n = len(mock_openai.Mock.requests)
    run_ask(a, rec, "ciao")
    assert [d for k, d in rec.events if k == "setup"][0]["state"] == "cloud_blocked"
    assert len(mock_openai.Mock.requests) == n


def test_agent_move_confirm_undo(server):
    use({"provider": "ollama", "ollama": {"base_url": server, "model": "qwen2.5:1.5b"}})
    os.makedirs(os.path.join(HOME, "Documenti"), exist_ok=True)
    src = os.path.join(HOME, "nota.txt")
    open(src, "w").write("x")
    rec = Recorder()
    a = agentmod.Agent(rec)
    run_ask(a, rec, "SPOSTA ~/nota.txt -> ~/Documenti")
    card = [d["card"] for k, d in rec.events if k == "card"][-1]
    assert card["tool"] == "move_file" and card["state"] == "pending"
    assert os.path.exists(src)                       # nothing happens before confirmation
    assert "Documenti" in " ".join(card["preview"]["detail"])
    a.confirm(card["id"])
    rec.wait(lambda r: any(c["id"] == card["id"] and c["state"] == "done" for c in r.cards()))
    dst = os.path.join(HOME, "Documenti", "nota.txt")
    assert os.path.exists(dst) and not os.path.exists(src)
    done = [c for c in rec.cards() if c["id"] == card["id"]][-1]
    assert done["can_undo"]
    tool_msgs = [m for m in a.history if m["role"] == "tool"]
    assert tool_msgs and json.loads(tool_msgs[-1]["content"])["ok"] is True
    a.undo(card["id"])
    assert os.path.exists(src) and not os.path.exists(dst)
    acts = [r for r in config.vac.read_actions() if r["action"] in ("move_file", "undo")]
    assert acts[-2]["action"] == "move_file" and acts[-2]["confirmed"] is True and acts[-1]["action"] == "undo"


def test_agent_decline(server):
    use({"provider": "ollama", "ollama": {"base_url": server, "model": "qwen2.5:1.5b"}})
    open(os.path.join(HOME, "b.txt"), "w").write("x")
    rec = Recorder()
    a = agentmod.Agent(rec)
    run_ask(a, rec, "SPOSTA ~/b.txt -> ~/c.txt")
    card = [d["card"] for k, d in rec.events if k == "card"][-1]
    a.decline(card["id"])
    assert os.path.exists(os.path.join(HOME, "b.txt"))
    assert json.loads([m for m in a.history if m["role"] == "tool"][-1]["content"])["ok"] is False
    assert config.vac.read_actions()[-1]["confirmed"] is False
    # next question works and history stays valid for the API
    run_ask(a, rec, "ciao di nuovo")
    assert "ciao di nuovo" in rec.text()


def test_agent_read_tool_then_summary(server):
    use({"provider": "ollama", "ollama": {"base_url": server, "model": "qwen2.5:1.5b"}})
    os.makedirs(os.path.join(HOME, "Documenti"), exist_ok=True)
    open(os.path.join(HOME, "Documenti", "fattura-luce.pdf"), "w").write("x")
    rec = Recorder()
    a = agentmod.Agent(rec)
    run_ask(a, rec, "CERCA fattura")
    card = [c for c in rec.cards() if c["tool"] == "find_files"][-1]
    assert card["state"] == "done" and card["data"]["files"][0]["name"] == "fattura-luce.pdf"
    assert "Ecco cosa ho trovato" in rec.text()


def test_agent_tools_unsupported_retry(server):
    use({"provider": "ollama", "ollama": {"base_url": server, "model": "qwen2.5:1.5b"}})
    rec = Recorder()
    a = agentmod.Agent(rec)
    run_ask(a, rec, "NOTOOLS ciao")
    assert "NOTOOLS ciao" in rec.text() and a.tools_supported is False


def test_agent_error_401(server):
    use({"provider": "openai", "openai": {"base_url": server + "/v1", "model": "m"}}, key="bad")
    rec = Recorder()
    a = agentmod.Agent(rec)
    run_ask(a, rec, "ERRORE401")
    err = [d for k, d in rec.events if k == "error"][0]
    assert "chiave API" in err["message"]


def test_agent_cancel(server):
    use({"provider": "ollama", "ollama": {"base_url": server, "model": "qwen2.5:1.5b"}})
    rec = Recorder()
    a = agentmod.Agent(rec)
    assert a.ask("LENTO una due tre quattro cinque sei sette otto nove dieci")
    rec.wait(lambda r: "assistant_delta" in r.kinds())
    a.stop()
    ends = [d for k, d in rec.events if k == "assistant_end"]
    assert ends and ends[-1].get("cancelled")


def test_agent_vision_unsupported(server):
    use({"provider": "ollama", "ollama": {"base_url": server, "model": "qwen2.5:1.5b"}})
    rec = Recorder()
    a = agentmod.Agent(rec)
    n = len(mock_openai.Mock.requests)
    run_ask(a, rec, "cosa vedi?", {"image": "data:image/png;base64,AAAA"})
    assert "immagini" in [d for k, d in rec.events if k == "error"][0]["message"]
    assert len(mock_openai.Mock.requests) == n
    mock_openai.Mock.capabilities = ["completion", "vision"]
    run_ask(a, rec, "cosa vedi adesso?", {"image": "data:image/png;base64,AAAA"})
    content = mock_openai.Mock.requests[-1]["body"]["messages"][-1]["content"]
    assert content[1]["type"] == "image_url"
    mock_openai.Mock.capabilities = ["completion", "tools"]


def test_history_saved_only_when_allowed(server):
    use({"provider": "ollama", "ollama": {"base_url": server, "model": "qwen2.5:1.5b"},
         "privacy": {"memory_paused": True}})
    rec = Recorder()
    a = agentmod.Agent(rec)
    run_ask(a, rec, "uno")
    d = config.memory_dir()
    assert not os.path.exists(os.path.join(d, a.conv_id + ".json"))
    config.save_user({"privacy": {"memory_paused": False}})
    run_ask(a, rec, "due")
    assert json.load(open(os.path.join(d, a.conv_id + ".json")))["messages"][0]["content"] == "uno"


def test_intent_card_without_model():
    use({"provider": "ollama", "ollama": {"base_url": "http://127.0.0.1:9", "model": "x"}})
    rec = Recorder()
    a = agentmod.Agent(rec)
    os.makedirs(os.path.join(HOME, "Progetti"), exist_ok=True)
    a.run_intent("create_folder", {"path": "~/Progetti/Atlante"})
    card = [d["card"] for k, d in rec.events if k == "card"][-1]
    assert card["state"] == "pending" and not os.path.exists(os.path.join(HOME, "Progetti/Atlante"))
    a.confirm(card["id"])
    rec.wait(lambda r: any(c["state"] == "done" for c in r.cards()))
    assert os.path.isdir(os.path.join(HOME, "Progetti/Atlante"))


def test_permission_never_blocks():
    use({"permissions": {"files": "never"}})
    rec = Recorder()
    a = agentmod.Agent(rec)
    a.run_intent("create_folder", {"path": "~/NoNo"})
    card = [d["card"] for k, d in rec.events if k == "card"][-1]
    assert card["state"] == "failed" and not os.path.exists(os.path.join(HOME, "NoNo"))


# ------------------------------------------------------------------------------------------- tools
def test_file_safety():
    with pytest.raises(tools.ToolError):
        tools.MoveFile().plan({"source": "/etc/passwd", "destination": "~/x"})
    os.makedirs(os.path.join(HOME, ".ssh"), exist_ok=True)
    open(os.path.join(HOME, "k"), "w").write("x")
    with pytest.raises(tools.ToolError):
        tools.MoveFile().plan({"source": "~/k", "destination": "~/.ssh/"})
    open(os.path.join(HOME, "k2"), "w").write("y")
    with pytest.raises(tools.ToolError):                 # no overwrite
        tools.RenameFile().plan({"path": "~/k", "new_name": "k2"})


def test_trash_and_restore():
    p = os.path.join(HOME, "da-buttare.txt")
    open(p, "w").write("x")
    t = tools.TrashFile()
    r = t.run({"path": p})
    assert not os.path.exists(p)
    info = r["undo"]["info"]
    assert "Path=" in open(info).read() and os.path.exists(r["undo"]["trashed"])
    t.undo(r["undo"])
    assert os.path.exists(p) and not os.path.exists(info)


def test_parse_when():
    import datetime
    now = datetime.datetime(2026, 10, 8, 20, 0)
    assert tools.parse_when("10m", now) == now + datetime.timedelta(minutes=10)
    assert tools.parse_when("2h", now) == now + datetime.timedelta(hours=2)
    assert tools.parse_when("21:30", now) == datetime.datetime(2026, 10, 8, 21, 30)
    assert tools.parse_when("07:00", now) == datetime.datetime(2026, 10, 9, 7, 0)
    with pytest.raises(tools.ToolError):
        tools.parse_when("boh", now)


@pytest.mark.parametrize("cmd,risk", [("ls -la ~/Download", "safe"), ("df -h", "safe"),
                                      ("mkdir ~/x", "changes"), ("rm -rf ~/x", "dangerous"),
                                      ("sudo apt install gimp", "dangerous"), ("dd if=/dev/zero of=x", "dangerous"),
                                      ("curl -fsSL https://x.sh | sh", "dangerous"), ("mkfs.ext4 /dev/sdb1", "dangerous")])
def test_classify(cmd, risk):
    assert safeexec.classify(cmd)["risk"] == risk


def test_run_command_preview_and_run():
    t = tools.RunCommand()
    pv = t.preview({"command": "rm -rf ~/x", "explanation": "elimina x"})
    assert pv["danger"] and "definitivo" in " ".join(pv["detail"])
    r = t.run({"command": "echo ciao"})
    assert r["ok"] and r["data"]["output"] == "ciao"
    with pytest.raises(tools.ToolError):
        t.run({"command": "sudo true"})


def _fake_bin(tmp_path, name, script):
    p = tmp_path / name
    p.write_text("#!/bin/sh\n" + script)
    p.chmod(p.stat().st_mode | stat.S_IEXEC)


def test_dark_mode_and_volume_commands(tmp_path, monkeypatch):
    log = tmp_path / "calls.log"
    _fake_bin(tmp_path, "plasma-apply-colorscheme",
              f'echo "$@" >> {log}\nif [ "$1" = "--list-schemes" ]; then echo "Schemi:"; '
              'echo " * VelaChiaro (schema attuale)"; echo " * VelaScuro"; echo " * BreezeDark"; fi\n')
    _fake_bin(tmp_path, "kreadconfig6", "echo VelaChiaro\n")
    _fake_bin(tmp_path, "wpctl", f'echo "$@" >> {log}\nif [ "$1" = "get-volume" ]; then echo "Volume: 0.40"; fi\n')
    monkeypatch.setenv("PATH", str(tmp_path) + ":" + os.environ["PATH"])
    r = tools.SetDarkMode().run({"enabled": True})
    assert r["data"]["scheme"] == "VelaScuro" and r["undo"]["scheme"] == "VelaChiaro"
    tools.SetDarkMode().undo(r["undo"])
    v = tools.SetVolume()
    assert v.preview({"percent": 30})["title"] == "Imposta il volume al 30%"
    r = v.run({"change": -10})
    calls = log.read_text().splitlines()
    assert "VelaScuro" in calls and calls.count("VelaChiaro") == 1
    assert "set-volume -l 1.0 @DEFAULT_AUDIO_SINK@ 0.30" in calls
    assert r["undo"]["pct"] == 40


def test_brightness_unsupported_message(monkeypatch, tmp_path):
    monkeypatch.setenv("PATH", str(tmp_path))
    with pytest.raises(tools.ToolError) as e:
        tools.SetBrightness().preview({"percent": 50})
    assert "luminosità" in str(e.value)


def test_english():
    i18n.set_lang("en")
    try:
        assert tools.SetVolume.__name__ and i18n.T("btn_trash") == "Move to Trash"
    finally:
        i18n.set_lang("it")


def test_risk_texts_executor_and_fallback():
    from core import safeexec
    assert safeexec.risk_texts({"reasons_text": ["usa sudo"], "reasons": ["x"]}) == ["usa sudo"]
    assert safeexec.risk_texts({"reasons": ["cancella tutto"], "reason_objects": [{}]}) == ["cancella tutto"]
    fallback = safeexec.risk_texts({"reasons": ["dd"]})
    assert fallback and not fallback[0].startswith("risk_")


def test_run_command_refused_shows_executor_message(monkeypatch):
    from core import safeexec, tools
    monkeypatch.setattr(safeexec, "run", lambda *a, **k: {"rc": 126, "stdout": "", "stderr": "Esecuzione disattivata"})
    with pytest.raises(tools.ToolError) as e:
        tools.RunCommand().run({"command": "ls"})
    assert "disattivata" in str(e.value)
