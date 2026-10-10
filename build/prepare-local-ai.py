#!/usr/bin/env python3
"""Fetch immutable build inputs; verify before placing them in the ISO overlay."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import tarfile
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
COMP = ROOT / "components/local-ai"


def digest(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for block in iter(lambda: f.read(8 * 1024 * 1024), b""):
            h.update(block)
    return h.hexdigest()


def fetch(asset, path):
    if path.exists() and digest(path) == asset["sha256"]:
        return
    tmp = path.with_suffix(path.suffix + ".download")
    with urllib.request.urlopen(asset["url"], timeout=60) as response, open(tmp, "wb") as out:
        shutil.copyfileobj(response, out, 8 * 1024 * 1024)
    if digest(tmp) != asset["sha256"]:
        raise RuntimeError("Upstream checksum mismatch: " + asset["url"])
    os.replace(tmp, path)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--cache", type=Path, default=ROOT / ".local/ai-cache")
    opts = ap.parse_args()
    manifest = json.loads((COMP / "assets.json").read_text())
    opts.cache.mkdir(parents=True, exist_ok=True)
    model, runtime = opts.cache / "gemma-4-E2B.gguf", opts.cache / "runtime.tar.gz"
    fetch(manifest["model"], model)
    fetch(manifest["runtime"], runtime)
    overlay = COMP / "overlay"
    libs = overlay / "usr/lib/vela/local-ai"
    weights = overlay / "usr/share/vela/models"
    docs = overlay / "usr/share/doc/vela-local-ai"
    for p in (libs, weights, docs):
        p.mkdir(parents=True, exist_ok=True)
    with tarfile.open(runtime) as tar:
        for member in tar.getmembers():
            name = Path(member.name).name
            library = name.startswith("lib") and ".so" in name
            if member.isfile() and (name == "llama-server" or library or name == "LICENSE"):
                target = docs / "llama.cpp-LICENSE" if name == "LICENSE" else libs / name
                with tar.extractfile(member) as source, open(target, "wb") as out:
                    shutil.copyfileobj(source, out)
            elif member.issym() and library:
                # Keep the SONAME chain within this directory, never archive paths.
                if member.linkname != Path(member.linkname).name or not member.linkname.startswith("lib"):
                    raise RuntimeError("Unsafe runtime library link")
                target = libs / name
                if target.is_symlink() or target.exists():
                    target.unlink()
                target.symlink_to(member.linkname)
    os.chmod(libs / "llama-server", 0o755)
    shutil.copyfile(model, weights / "gemma-4-E2B.gguf")
    (weights / "SHA256SUMS").write_text(manifest["model"]["sha256"] + "  /usr/share/vela/models/gemma-4-E2B.gguf\n")
    shutil.copyfile(COMP / "assets.json", docs / "assets.json")
    shutil.copyfile(COMP / "MODEL-LICENSE", docs / "Gemma-4-LICENSE")
    print("Verified local model and runtime assembled.")


if __name__ == "__main__":
    main()
