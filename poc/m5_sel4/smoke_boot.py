"""Bounded Linux CI boot proof; serial markers are not performance guarantees."""
import os
import selectors
import subprocess
import sys
import time

REQUIRED = (
    b"VelaOS Gate E: 101 malformed PCB frames rejected; domain remains live",
    b"VelaOS Gate E: downstream malformed frames rejected",
    b"VelaOS Gate D: fuel exhaustion and 64 KiB memory cap verified",
    b"VelaOS Gate E: intent completed with result 42",
    b"VelaOS Gate F: END end_to_end",
)


def main():
    if len(sys.argv) != 2:
        return 2
    process = subprocess.Popen([
        "qemu-system-aarch64", "-machine", "virt,virtualization=on",
        "-cpu", "cortex-a53", "-m", "size=2G", "-nic", "none",
        "-serial", "stdio", "-monitor", "none", "-display", "none",
        "-device", f"loader,file={sys.argv[1]},addr=0x70000000,cpu-num=0",
    ], stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    output = bytearray()
    deadline = time.monotonic() + 60
    try:
        with selectors.DefaultSelector() as selector:
            selector.register(process.stdout, selectors.EVENT_READ)
            while time.monotonic() < deadline:
                for key, _ in selector.select(timeout=0.2):
                    chunk = os.read(key.fileobj.fileno(), 4096)
                    if not chunk:
                        print("QEMU exited before all required markers", file=sys.stderr)
                        return 1
                    output.extend(chunk)
                    if len(output) > 1024 * 1024:
                        print("Serial output limit exceeded", file=sys.stderr)
                        return 1
                    sys.stdout.buffer.write(chunk)
                    sys.stdout.buffer.flush()
                if b"panicked at" in output or b"FAULT" in output:
                    print("Guest fault detected", file=sys.stderr)
                    return 1
                if all(marker in output for marker in REQUIRED):
                    print("\nBoot smoke passed; durable LFS, SLM and hardware remain unqualified.")
                    return 0
            print("Boot deadline exceeded; missing required markers", file=sys.stderr)
            return 1
    finally:
        process.terminate()
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait()


if __name__ == "__main__":
    raise SystemExit(main())
