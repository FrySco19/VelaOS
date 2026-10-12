#!/usr/bin/env python3
import subprocess
import sys
import time


def main() -> int:
    if len(sys.argv) != 2:
        print(f"usage: {sys.argv[0]} LOADER_IMAGE", file=sys.stderr)
        return 2

    qemu = subprocess.Popen(
        [
            "qemu-system-aarch64",
            "-machine",
            "virt,virtualization=on",
            "-cpu",
            "cortex-a53",
            "-m",
            "size=2G",
            "-nic",
            "none",
            "-serial",
            "mon:stdio",
            "-nographic",
            "-device",
            f"loader,file={sys.argv[1]},addr=0x70000000,cpu-num=0",
        ],
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        bufsize=1,
    )
    active: dict[str, int] = {}

    try:
        assert qemu.stdout is not None
        for line in qemu.stdout:
            timestamp = time.monotonic_ns()
            print(line, end="", flush=True)
            begin = "VelaOS Gate F: BEGIN "
            end = "VelaOS Gate F: END "
            if begin in line:
                name = line.split(begin, 1)[1].strip()
                active[name] = timestamp
            elif end in line:
                name = line.split(end, 1)[1].strip()
                started = active.pop(name, None)
                if started is None:
                    print(f"[host timing] unmatched END marker: {name}", flush=True)
                else:
                    elapsed_ms = (timestamp - started) / 1_000_000
                    print(
                        f"[host timing] {name}: {elapsed_ms:.3f} ms "
                        "(serial-marker interval; includes QEMU and host I/O)",
                        flush=True,
                    )
        return qemu.wait()
    except KeyboardInterrupt:
        qemu.terminate()
        return qemu.wait()
    finally:
        if qemu.poll() is None:
            qemu.terminate()


if __name__ == "__main__":
    raise SystemExit(main())
