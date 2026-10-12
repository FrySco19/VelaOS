#!/usr/bin/env python3
import re
import subprocess
import sys


def main() -> int:
    if len(sys.argv) != 2:
        print(f"usage: {sys.argv[0]} ELF", file=sys.stderr)
        return 2

    result = subprocess.run(
        ["readelf", "-lW", sys.argv[1]],
        check=True,
        capture_output=True,
        text=True,
    )
    loads = [
        line
        for line in result.stdout.splitlines()
        if re.match(r"^\s*LOAD\s", line)
    ]
    if not loads:
        print("ELF has no PT_LOAD segments", file=sys.stderr)
        return 1

    for line in loads:
        fields = line.split()
        flags = "".join(fields[6:-1])
        if "W" in flags and "E" in flags:
            print(f"writable-executable ELF segment: {line}", file=sys.stderr)
            return 1

    print(f"W^X ELF segment check passed ({len(loads)} PT_LOAD segments)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
