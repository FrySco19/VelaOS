#!/usr/bin/env python3
import argparse
import hashlib
import json
from pathlib import Path

ap = argparse.ArgumentParser()
ap.add_argument("iso", type=Path)
ap.add_argument("output", type=Path)
ap.add_argument("--part-mib", type=int, default=1900)
args = ap.parse_args()
if not 1 <= args.part_mib <= 1900:
    ap.error("part size must be between 1 and 1900 MiB")
args.output.mkdir(parents=True, exist_ok=True)
record = {"iso": args.iso.name, "parts": []}
whole = hashlib.sha256()
with args.iso.open("rb") as source:
    for index in range(1000):
        block = source.read(min(8, args.part_mib) * 1024 * 1024)
        if not block:
            break
        name = args.iso.name + f".part{index + 1:02d}"
        h, size = hashlib.sha256(), 0
        with (args.output / name).open("xb") as out:
            while block:
                out.write(block)
                h.update(block)
                whole.update(block)
                size += len(block)
                if size >= args.part_mib * 1024 * 1024:
                    break
                block = source.read(min(8 * 1024 * 1024, args.part_mib * 1024 * 1024 - size))
        record["parts"].append({"name": name, "bytes": size, "sha256": h.hexdigest()})
record["sha256"] = whole.hexdigest()
(args.output / "ISO-PARTS.json").write_text(json.dumps(record, indent=2) + "\n")
print(json.dumps(record, indent=2))
