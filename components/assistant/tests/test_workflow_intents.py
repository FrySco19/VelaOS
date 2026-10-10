import pytest
from core import intents


@pytest.mark.parametrize("text,path", [
    ("Mi aiuti a organizzare la cartella ~/Downloads per tipo di file?", "~/Downloads"),
    ('organizza la cartella "~/Foto Estate"', "~/Foto Estate"),
    ("Please organize the folder ~/MyPictures by file type", "~/MyPictures"),
])
def test_explicit_folder_preview_preserves_literal_path(text, path):
    assert intents.match(text) == ("organize_folder", {"folder": path})


def test_ambiguous_or_multiple_folder_requests_need_model():
    assert intents.match("Organizza i miei file") is None
    assert intents.match("Organizza ~/Downloads e cancella ~/Documenti") is None


def test_catalog_is_allowlisted_and_single_action():
    assert intents.match("Installa VLC dal catalogo") == ("install_catalog_app", {"name": "vlc"})
    assert intents.match("Installa InventataXYZ") is None
    assert intents.match("Installa VLC e GIMP") is None


def test_audio_read_only_intent():
    assert intents.match("Controlla l'audio") == ("diagnose_audio", {})
