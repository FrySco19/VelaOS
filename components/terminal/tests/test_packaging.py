"""Static checks of the shipped config files (no GUI)."""
import configparser
import os
import shutil
import subprocess
import sys

import pytest

COMP = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OV = os.path.join(COMP, "overlay")
sys.path.insert(0, os.path.join(COMP, "tools"))


def test_konsole_files_match_theme_palette(tmp_path, monkeypatch):
    import gen_konsole
    monkeypatch.setattr(gen_konsole, "OUT", str(tmp_path))
    gen_konsole.main()
    for name in os.listdir(tmp_path):
        with open(tmp_path / name) as a, open(os.path.join(OV, "usr/share/konsole", name)) as b:
            assert a.read() == b.read(), f"{name} out of date: run tools/gen_konsole.py"


@pytest.mark.parametrize("mode,bg", [("VelaLight", "251,252,252"), ("VelaDark", "19,34,42")])
def test_konsole_profile(mode, bg):
    p = configparser.ConfigParser(interpolation=None)
    p.optionxform = str
    p.read(os.path.join(OV, f"usr/share/konsole/{mode}.profile"))
    assert p["Appearance"]["ColorScheme"] == mode
    assert p["Appearance"]["Font"].startswith("JetBrains Mono,")
    assert p["Interaction Options"]["AutoCopySelectedText"] == "false"
    assert p["Interaction Options"]["MiddleClickPasteMode"] == "0"
    s = configparser.ConfigParser(interpolation=None)
    s.read(os.path.join(OV, f"usr/share/konsole/{mode}.colorscheme"))
    assert s["Background"]["Color"] == bg


def test_konsole_shortcuts():
    x = open(os.path.join(OV, "etc/skel/.local/share/kxmlgui5/konsole/sessionui.rc")).read()
    assert 'name="edit_paste" shortcut="Ctrl+Shift+V; Ctrl+V"' in x
    assert 'name="edit_copy" shortcut="Ctrl+Shift+C"' in x
    assert 'version="1"' in x       # lower than Konsole's own file -> merged, not replacing menus


@pytest.mark.parametrize("script", ["usr/bin/vela-ai-terminal", "usr/bin/vela-executor",
                                    "usr/bin/vela-terminal-theme", "../hook.chroot.sh", "../deploy.sh"])
def test_shell_syntax(script):
    path = os.path.normpath(os.path.join(OV, script))
    assert subprocess.run(["sh", "-n", path]).returncode == 0
    if script.startswith("usr/bin"):
        assert os.access(path, os.X_OK)


@pytest.mark.skipif(not shutil.which("fish"), reason="fish not installed")
def test_fish_syntax():
    assert subprocess.run(["fish", "-n", os.path.join(OV, "etc/fish/conf.d/vela.fish")]).returncode == 0


@pytest.mark.skipif(not shutil.which("desktop-file-validate"), reason="no desktop-file-validate")
@pytest.mark.parametrize("f", ["usr/share/applications/vela-ai-terminal.desktop",
                               "etc/xdg/autostart/vela-terminal-theme.desktop"])
def test_desktop_files(f):
    r = subprocess.run(["desktop-file-validate", os.path.join(OV, f)], capture_output=True, text=True)
    assert r.returncode == 0, r.stdout + r.stderr


def test_starship_config_parses():
    import tomllib
    with open(os.path.join(OV, "usr/share/vela/terminal/starship.toml"), "rb") as f:
        cfg = tomllib.load(f)
    assert "❯" in cfg["character"]["success_symbol"]


def test_hook_in_scratch_root(tmp_path):
    if not shutil.which("kwriteconfig6"):
        pytest.skip("kwriteconfig6 missing")
    (tmp_path / "usr/bin").mkdir(parents=True)
    fish = tmp_path / "usr/bin/fish"
    fish.write_text("#!/bin/sh\n")
    fish.chmod(0o755)
    (tmp_path / "etc").mkdir()
    (tmp_path / "etc/shells").write_text("/bin/sh\n/bin/bash\n")
    (tmp_path / "etc/adduser.conf").write_text("# Default: DSHELL=/bin/bash\n#DSHELL=/bin/bash\n")
    env = dict(os.environ, VELA_ROOT=str(tmp_path))
    for _ in range(2):   # idempotent
        assert subprocess.run(["sh", os.path.join(COMP, "hook.chroot.sh")], env=env).returncode == 0
    assert (tmp_path / "etc/shells").read_text().count("/usr/bin/fish") == 1
    assert "\nDSHELL=/usr/bin/fish" in (tmp_path / "etc/adduser.conf").read_text()
    assert (tmp_path / "etc/default/useradd").read_text().strip() == "SHELL=/usr/bin/fish"
    rc = (tmp_path / "etc/xdg/konsolerc").read_text()
    assert "DefaultProfile=VelaLight.profile" in rc
    assert os.path.islink(tmp_path / "etc/systemd/user/sockets.target.wants/vela-executor.socket")
