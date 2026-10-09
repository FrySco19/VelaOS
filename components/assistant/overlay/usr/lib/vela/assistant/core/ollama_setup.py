"""Per-user Ollama install + model download, only after an explicit click in the setup card.

No root needed: Ollama goes to ~/.local/share/vela/ollama, runs as a systemd --user service
(vela-ollama.service, 127.0.0.1 only), models live in ~/.ollama/models (Ollama default).
Progress is reported through `progress(phase, done_bytes, total_bytes)`; `cancel` is a threading.Event.
"""
import json
import os
import shutil
import subprocess
import time
import urllib.request

from . import config

DOWNLOAD_URL = os.environ.get("VELA_OLLAMA_URL", "https://ollama.com/download/ollama-linux-amd64.tar.zst")
OLLAMA_DL_BYTES = 1_440_000_000          # measured 2026-10-08: ollama-linux-amd64.tar.zst v0.40.1 = 1 436 371 398 B
MODELS = [  # tag, approx download size (Ollama library), RAM class, label key
    ("qwen2.5:1.5b", 986_000_000, "small"),
    ("llama3.2:3b", 2_000_000_000, "medium"),
    ("qwen2.5vl:3b", 3_200_000_000, "vision"),
]
UNIT = "vela-ollama.service"


def install_dir():
    return os.path.join(config.data_dir(), "ollama")


def user_binary():
    return os.path.join(install_dir(), "bin", "ollama")


def ollama_binary():
    if os.path.exists(user_binary()):
        return user_binary()
    return shutil.which("ollama")


def model_size(tag):
    for t, size, _ in MODELS:
        if t == tag:
            return size
    return 1_000_000_000


def needed_bytes(tag, with_ollama):
    return model_size(tag) + (OLLAMA_DL_BYTES if with_ollama else 0)


class SetupError(Exception):
    pass


class Cancelled(Exception):
    pass


def _systemd_ok():
    return os.environ.get("VELA_ASSISTANT_NO_SYSTEMD") != "1" and shutil.which("systemctl") is not None


def download_ollama(progress, cancel):
    dest = install_dir()
    tmp = dest + ".partial"
    shutil.rmtree(tmp, ignore_errors=True)
    os.makedirs(tmp, exist_ok=True)
    zst = DOWNLOAD_URL.endswith(".zst")
    if zst and not shutil.which("zstd"):
        raise SetupError("zstd")
    tar_cmd = ["tar", "-x", "-C", tmp, "--wildcards", "--exclude=*cuda*", "--exclude=*rocm*",
               "--exclude=*mlx*", "--exclude=*cublas*", "--exclude=*jetpack*"]
    if zst:
        dec = subprocess.Popen(["zstd", "-d", "-c"], stdin=subprocess.PIPE, stdout=subprocess.PIPE)
        tar = subprocess.Popen(tar_cmd, stdin=dec.stdout)
        dec.stdout.close()
        sink = dec.stdin
    else:
        tar = subprocess.Popen(tar_cmd + ["-z"], stdin=subprocess.PIPE)
        dec, sink = None, tar.stdin
    try:
        req = urllib.request.Request(DOWNLOAD_URL, headers={"User-Agent": "Vela-Assistant/2"})
        with urllib.request.urlopen(req, timeout=30) as r:
            total = int(r.headers.get("Content-Length") or OLLAMA_DL_BYTES)
            done = 0
            while True:
                if cancel.is_set():
                    raise Cancelled()
                chunk = r.read(1 << 20)
                if not chunk:
                    break
                sink.write(chunk)
                done += len(chunk)
                progress("download", done, total)
        sink.close()
        if dec:
            dec.wait(120)
        if tar.wait(300) != 0:
            raise SetupError("tar")
    except (Cancelled, SetupError, OSError):
        for p in (dec, tar):
            if p and p.poll() is None:
                p.kill()
        shutil.rmtree(tmp, ignore_errors=True)
        raise
    progress("extract", 0, 0)
    if not os.path.exists(os.path.join(tmp, "bin", "ollama")):
        shutil.rmtree(tmp, ignore_errors=True)
        raise SetupError("bin/ollama missing")
    shutil.rmtree(dest, ignore_errors=True)
    os.replace(tmp, dest)
    os.chmod(os.path.join(dest, "bin", "ollama"), 0o755)
    link = os.path.expanduser("~/.local/bin/ollama")
    try:
        os.makedirs(os.path.dirname(link), exist_ok=True)
        if not os.path.exists(link):
            os.symlink(os.path.join(dest, "bin", "ollama"), link)
    except OSError:
        pass


def write_unit():
    d = os.path.join(os.environ.get("XDG_CONFIG_HOME") or os.path.expanduser("~/.config"), "systemd", "user")
    os.makedirs(d, exist_ok=True)
    with open(os.path.join(d, UNIT), "w", encoding="utf-8") as f:
        f.write(f"""[Unit]
Description=Ollama (modello locale per l'assistente di Vela)
After=network.target

[Service]
ExecStart={ollama_binary()} serve
Environment=OLLAMA_HOST=127.0.0.1:11434
Restart=on-failure
RestartSec=3
Nice=5

[Install]
WantedBy=default.target
""")


def start_service(native_url):
    """Start Ollama (user unit, or a detached `ollama serve`) and wait until the API answers."""
    binary = ollama_binary()
    if not binary:
        raise SetupError("ollama missing")
    if _systemd_ok():
        if os.path.exists(user_binary()):
            write_unit()
            subprocess.run(["systemctl", "--user", "daemon-reload"], timeout=20)
            subprocess.run(["systemctl", "--user", "enable", "--now", UNIT], timeout=30)
        else:
            subprocess.run(["systemctl", "--user", "start", "ollama.service"], timeout=30, capture_output=True)
    if not _wait_api(native_url, 4):
        subprocess.Popen([binary, "serve"], stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
                         stderr=subprocess.DEVNULL, start_new_session=True,
                         env=dict(os.environ, OLLAMA_HOST="127.0.0.1:11434"))
    if not _wait_api(native_url, 40):
        raise SetupError("ollama did not start")


def _wait_api(native_url, seconds):
    end = time.time() + seconds
    while time.time() < end:
        try:
            with urllib.request.urlopen(native_url.rstrip("/") + "/api/tags", timeout=2):
                return True
        except OSError:
            time.sleep(0.5)
    return False


def pull_model(native_url, tag, progress, cancel):
    body = json.dumps({"model": tag, "stream": True}).encode()
    req = urllib.request.Request(native_url.rstrip("/") + "/api/pull", data=body,
                                 headers={"Content-Type": "application/json"})
    layers = {}
    with urllib.request.urlopen(req, timeout=60) as r:
        for line in r:
            if cancel.is_set():
                raise Cancelled()
            try:
                j = json.loads(line)
            except ValueError:
                continue
            if j.get("error"):
                raise SetupError(j["error"])
            if j.get("digest") and j.get("total"):
                layers[j["digest"]] = (j.get("completed", 0), j["total"])
                done = sum(c for c, _ in layers.values())
                total = max(sum(t for _, t in layers.values()), model_size(tag))
                progress("pull", done, total)
            if j.get("status") == "success":
                return True
    raise SetupError("pull interrupted")


def setup_local(tag, progress, cancel, native_url="http://127.0.0.1:11434"):
    """Full flow. Returns the model tag. Raises SetupError/Cancelled."""
    need_ollama = ollama_binary() is None
    free = shutil.disk_usage(os.path.expanduser("~")).free
    if free < needed_bytes(tag, need_ollama) + 1_500_000_000:
        raise SetupError("no_space")
    config.log("setup_local", f"install {'Ollama + ' if need_ollama else ''}{tag}", confirmed=True)
    if need_ollama:
        download_ollama(progress, cancel)
    progress("start", 0, 0)
    if not _wait_api(native_url, 1):
        start_service(native_url)
    pull_model(native_url, tag, progress, cancel)
    config.save_user({"enabled": True, "provider": "ollama", "ollama": {"model": tag}})
    config.log("setup_local", f"ready: {tag}", confirmed=True)
    return tag
