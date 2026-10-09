#!/usr/bin/env python3
"""OCR screenshots (tesseract ita+eng) and flag English UI words in an Italian session.
usage: ocr_english.py OUT.md IMG.png [...]"""
import re, subprocess, sys
EN = set("""about account add advanced all allow and apply apps are back browse cancel change check choose clear close
colors configure connect copy create custom default delete desktop details device devices disabled display done download
edit empty enable enabled error file files find folder folders for from general get help hide home install installed
keyboard language launch learn less loading lock log manage more move network new next none not now off open options
other password paste please preferences print properties quit recent refresh remove rename reset restart restore save
screen search select send settings share show shut shutdown sign size skip sleep software sort start system the this
tools trash try undo uninstall update updates upgrade user users view volume welcome window windows with your you
favorites places power session applications""".split())
OK = {"desktop", "file", "browser", "software", "app", "dock", "store", "wi-fi", "password", "home", "online", "default", "computer"}
def main():
    out, imgs = sys.argv[1], sys.argv[2:]
    rows = []
    for img in imgs:
        txt = subprocess.run(["tesseract", img, "-", "-l", "ita+eng", "--psm", "11"], capture_output=True, text=True).stdout
        words = re.findall(r"[A-Za-z][A-Za-z'-]{2,}", txt)
        hits = sorted({w for w in words if w.lower() in EN and w.lower() not in OK})
        lines = [l.strip() for l in txt.splitlines() if any(re.search(r"\b%s\b" % re.escape(h), l) for h in hits)]
        rows.append((img, hits, lines[:15]))
    with open(out, "w") as f:
        f.write("# OCR English-word scan (tesseract ita+eng, heuristic: verify by eye)\n\n")
        for img, hits, lines in rows:
            f.write(f"## {img.split('/')[-1]}\n- English-looking words: {', '.join(hits) or 'none'}\n")
            for l in lines:
                f.write(f"  - `{l}`\n")
            f.write("\n")
    print(out, sum(1 for r in rows if r[1]), "of", len(rows), "images with hits")
main()
