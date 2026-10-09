"""Mock-LLM integration tests for prompts/parsers, plus preview and history."""
import json
import os

import pytest

import mock_llm


@pytest.fixture()
def home(tmp_path, monkeypatch):
    monkeypatch.setenv("HOME", str(tmp_path))
    monkeypatch.setenv("XDG_CONFIG_HOME", str(tmp_path / ".config"))
    monkeypatch.setenv("XDG_DATA_HOME", str(tmp_path / ".local/share"))
    return tmp_path


@pytest.fixture()
def mock(home):
    srv = mock_llm.start()
    d = home / ".config" / "vela"
    d.mkdir(parents=True)
    (d / "ai.json").write_text(json.dumps({"provider": "ollama", "ollama": {
        "base_url": f"http://127.0.0.1:{srv.server_address[1]}", "model": "mock"}}))
    mock_llm.CALLS.clear()
    yield srv
    srv.shutdown()


def test_propose_roundtrip(mock, home):
    from vela_terminal.llm import LLM, parse_json, propose_messages, parse_proposal
    p = parse_proposal(parse_json(LLM().chat(propose_messages("spazio libero?", str(home), "it"))))
    assert p["commands"][0]["command"] == "df -h ~"
    body = mock_llm.CALLS[-1]
    assert body["model"] == "mock" and body["response_format"] == {"type": "json_object"}
    assert "Debian 13" in body["messages"][0]["content"]


def test_agent_and_explain(mock, home):
    from vela_terminal.llm import LLM, parse_json, agent_messages, parse_agent, explain_messages, parse_explain
    r = parse_agent(parse_json(LLM().chat(agent_messages("obiettivo", str(home), "it", []))))
    assert r["next"]["command"] == "mkdir -p prova_agente" and not r["done"] and len(r["plan"]) == 3
    steps = [{"command": "a", "rc": 0, "output": ""}] * 3
    r = parse_agent(parse_json(LLM().chat(agent_messages("obiettivo", str(home), "it", steps))))
    assert r["done"] and r["next"] is None
    e = parse_explain(parse_json(LLM().chat(explain_messages("cat x", 1, "No such file", str(home), "it"))))
    assert e["fix"]["command"] == "ls -la"


def test_disabled_and_cloud_not_allowed(home):
    from vela_terminal.llm import LLM, LLMError
    d = home / ".config" / "vela"
    d.mkdir(parents=True)
    (d / "ai.json").write_text(json.dumps({"enabled": False}))
    with pytest.raises(LLMError) as ei:
        LLM().chat([{"role": "user", "content": "x"}])
    assert ei.value.code == "not_allowed" and ei.value.detail == "disabled"
    (d / "ai.json").write_text(json.dumps({"provider": "openai", "openai": {"base_url": "https://api.example.com/v1", "model": "m"},
                                           "privacy": {"allow_cloud": False}}))
    with pytest.raises(LLMError) as ei:
        LLM().chat([{"role": "user", "content": "x"}])
    assert ei.value.detail == "cloud_not_allowed"


def test_unreachable(home):
    from vela_terminal.llm import LLM, LLMError
    d = home / ".config" / "vela"
    d.mkdir(parents=True)
    (d / "ai.json").write_text(json.dumps({"provider": "ollama", "ollama": {"base_url": "http://127.0.0.1:9", "model": "m"}}))
    with pytest.raises(LLMError) as ei:
        LLM().chat([{"role": "user", "content": "x"}])
    assert ei.value.code == "unreachable"


def test_parse_json_variants():
    from vela_terminal.llm import parse_json, parse_proposal, LLMError
    assert parse_json('```json\n{"a": 1}\n```') == {"a": 1}
    assert parse_json('Ecco: {"a": 2} fine') == {"a": 2}
    with pytest.raises(LLMError):
        parse_json("nessun json")
    p = parse_proposal({"commands": ["ls", {"command": " "}, {"command": "pwd", "explanation": "x"}]})
    assert [c["command"] for c in p["commands"]] == ["ls", "pwd"]


def test_redact():
    from vela_terminal.llm import redact
    t = redact("export OPENAI_API_KEY=sk-abcdefghijklmnopqrstuvwxyz password=hunter2 ghp_" + "a" * 30)
    assert "sk-abc" not in t and "hunter2" not in t and "ghp_aaaa" not in t


def test_preview_cases(tmp_path):
    from vela_terminal.preview import preview
    (tmp_path / "d" / "s").mkdir(parents=True)
    (tmp_path / "d" / "s" / "f.log").write_text("x" * 100)
    (tmp_path / "a.txt").write_text("hello\n")
    r = preview("rm -rf d", str(tmp_path))
    assert "Cancellerà definitivamente 1 elemento, 1 file" in r["text"] and r["exact"]
    r = preview("rm nonesiste.txt", str(tmp_path))
    assert "non verrà cancellato nulla" in r["text"]
    r = preview("sed -i s/hello/ciao/ a.txt", str(tmp_path))
    assert "-hello" in r["text"] and "+ciao" in r["text"]
    assert (tmp_path / "a.txt").read_text() == "hello\n", "preview must not modify files"
    r = preview("find . -name '*.log' -delete", str(tmp_path))
    assert "f.log" in r["text"] and (tmp_path / "d" / "s" / "f.log").exists()
    r = preview("echo x > a.txt", str(tmp_path))
    assert "Sovrascrive a.txt" in r["text"]
    r = preview("ls -la", str(tmp_path))
    assert r["kind"] == "readonly"
    r = preview("rm $(cat lista)", str(tmp_path))
    assert r["exact"] is False
    r = preview("mv a.txt d/", str(tmp_path))
    assert "a.txt → d/a.txt" in r["text"]
    r = preview("sed -i '1e touch PWNED' a.txt", str(tmp_path))
    assert not (tmp_path / "PWNED").exists(), "sed preview must run sandboxed"


def test_history(tmp_path):
    from vela_terminal.history import History
    h = History(str(tmp_path / "h.sqlite"))
    i = h.add("trova i file più grandi", ["du -sh * | sort -h"], "/tmp")
    h.add("spazio libero", ["df -h"], "/tmp")
    h.update_rc(i, 0)
    assert h.search("grandi")[0]["commands"] == ["du -sh * | sort -h"]
    assert h.search("df")[0]["intent"] == "spazio libero"
    assert len(h.search("")) == 2
    h.clear()
    assert h.search("") == []
