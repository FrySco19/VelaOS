#!/usr/bin/env python3
"""Extract strings and write po/it/<domain>.po. Fails if a string has no Italian translation."""
import os, subprocess, sys
import polib
sys.path.insert(0, os.path.dirname(__file__))
from translations_it import IT
root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
missing = []
for d in ("ai", "privacy", "updates", "aspetto"):
    dom = f"kcm_vela_{d}"
    src = [os.path.join(d, f) for f in os.listdir(os.path.join(root, d)) if f.endswith(".cpp")]
    src += [os.path.join(d, "ui", f) for f in os.listdir(os.path.join(root, d, "ui")) if f.endswith(".qml")]
    pot = f"/tmp/{dom}.pot"
    subprocess.run(["xgettext", "--from-code=UTF-8", "-C", "--qt", "-kde", "-ki18nd:2", "-ki18ndp:2,3", "-ktd:1",
                    "-o", pot] + sorted(src), cwd=root, check=True, stderr=subprocess.DEVNULL)
    po = polib.pofile(pot)
    po.metadata.update({"Language": "it", "Content-Type": "text/plain; charset=UTF-8",
                        "Plural-Forms": "nplurals=2; plural=n != 1;", "Project-Id-Version": dom})
    for e in po:
        e.occurrences = []
        if e.msgid_plural:
            t = IT.get((e.msgid, e.msgid_plural))
            if not t: missing.append(e.msgid); continue
            e.msgstr_plural = {0: t[0], 1: t[1]}
        else:
            t = IT.get(e.msgid)
            if not t: missing.append(e.msgid); continue
            e.msgstr = t
    os.makedirs(os.path.join(root, "po", "it"), exist_ok=True)
    po.save(os.path.join(root, "po", "it", dom + ".po"))
if missing:
    print("MISSING:", *missing, sep="\n  "); sys.exit(1)
print("ok")
