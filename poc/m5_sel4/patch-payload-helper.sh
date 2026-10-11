#!/bin/sh
set -eu

checkout="$(find /root/.cargo/git/checkouts -path '*/crates/sel4-patch-elf/src/lib.rs' -print -quit | sed 's#/crates/sel4-patch-elf/src/lib.rs$##')"
test -n "$checkout"
test "$(git -C "$checkout" rev-parse HEAD)" = "d441a880938209f661201704dfbee7b91c5aa905"

python3 - "$checkout" <<'PY'
from pathlib import Path
import sys

root = Path(sys.argv[1])
replacements = (
    (
        root / "crates/sel4-patch-elf/src/dynamic.rs",
        "use super as low_level;",
        "use crate as low_level;",
    ),
    (
        root / "crates/sel4-patch-elf/src/lib.rs",
        "self.phdrs.push_mut(phdr)",
        "{ self.phdrs.push(phdr); self.phdrs.last().unwrap() }",
    ),
)

for path, old, new in replacements:
    text = path.read_text()
    if text.count(old) == 1:
        path.write_text(text.replace(old, new))
    elif text.count(new) == 1:
        continue
    else:
        raise SystemExit(f"expected exactly one patch target in {path}")
PY
