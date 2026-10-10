from core import tools


def names(text, history=None):
    return {s["function"]["name"] for s in tools.select_schemas(text, history)}


def test_file_domain_reduces_prompt_without_shell_access():
    selected = names("Organizza la cartella ~/Downloads")
    assert "organize_folder" in selected and "run_command" not in selected
    assert len(selected) < len(tools.schemas())


def test_multiple_domains_are_combined():
    selected = names("Controlla l'audio e installa VLC")
    assert {"diagnose_audio", "set_audio_output", "install_catalog_app"} <= selected


def test_short_followup_retains_previous_domain():
    selected = names("Usa ~/Downloads", [{"role": "user", "content": "Organizza la cartella"}])
    assert "organize_folder" in selected


def test_unrecognized_request_retains_registered_tools():
    selected = names("Mi puoi aiutare?")
    assert selected == {s["function"]["name"] for s in tools.schemas()}
    assert "run_command" not in selected
