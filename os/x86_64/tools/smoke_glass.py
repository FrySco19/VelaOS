"""Boot and exercise the native Glass lifecycle; no fixture network/audio claims."""
import pathlib
import socket
import subprocess
import sys
import tempfile
import time

iso = pathlib.Path(sys.argv[1]).resolve()
output = pathlib.Path(sys.argv[2]).resolve()
output.mkdir(parents=True, exist_ok=True)
serial = output / "serial.log"
with tempfile.TemporaryDirectory(prefix="vela-glass-") as tmp:
    monitor = pathlib.Path(tmp) / "monitor.sock"
    process = subprocess.Popen([
        "qemu-system-x86_64", "-m", "512", "-vga", "std", "-nic", "none",
        "-cdrom", str(iso), "-display", "none", "-serial", "file:" + str(serial),
        "-monitor", "unix:" + str(monitor) + ",server=on,wait=off",
    ], stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
    def await_marker(marker, count=1):
        until = time.monotonic() + 40
        while time.monotonic() < until:
            text = serial.read_text(errors="replace") if serial.exists() else ""
            if text.count(marker) >= count:
                return
            if process.poll() is not None:
                raise RuntimeError("QEMU exited: " + process.stderr.read().decode(errors="replace"))
            time.sleep(0.1)
        raise RuntimeError("Missing serial marker: " + marker)
    def command(text):
        with socket.socket(socket.AF_UNIX) as connection:
            connection.settimeout(3)
            connection.connect(str(monitor))
            connection.recv(4096)
            connection.sendall((text + "\n").encode())
            connection.recv(4096)
        time.sleep(0.15)
    def type_text(text):
        for char in text:
            command("sendkey " + ("spc" if char == " " else char) + " 50")
    try:
        await_marker("welcome screen")
        time.sleep(1)
        command("sendkey ret")
        await_marker("idle desktop, no application windows resident")
        command("screendump " + str(output / "desktop.ppm"))
        type_text("appunta idee")
        command("sendkey ret")
        type_text("bridge ipc")
        command("screendump " + str(output / "note.ppm"))
        command("sendkey ctrl-ret")
        await_marker("RAM draft retained; no durable receipt claimed")
        await_marker("surface released; temporary buffers cleared")
        type_text("wifi")
        command("sendkey ret")
        command("screendump " + str(output / "network.ppm"))
        command("sendkey esc")
        await_marker("surface released; temporary buffers cleared", 2)
        print("PASS: native boot, note entry, RAM retention, automatic close, network surface, cancellation.")
        print(serial.read_text())
    finally:
        process.terminate()
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait()
