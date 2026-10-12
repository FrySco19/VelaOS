#!/usr/bin/env python3
"""FALLBACK CONTEXT-SIGNALS consumer for utterance «ciao».

NOT an LLM. Deterministic mapping of Context SIGNALS_v0 envelopes to
Proposal-only candidates. Display-only hashed handles; no execute, no grant,
no authority. Admit still requires Shell UserSelection -> Policy.

Ported from the host demo. PNG drawing is the host renderer (Pillow + Inter).
"""
import hashlib, json, os, sys
from PIL import Image, ImageDraw, ImageFont

HERE = os.path.dirname(os.path.abspath(__file__))
FIXTURE = os.path.join(HERE, "fixtures", "ciao-demo-signals.jsonl")
OUT = os.path.join(HERE, "out")
UTTERANCE = "ciao"
MODE = "FALLBACK CONTEXT-SIGNALS"
DEMO_CMD = "python3 poc/intent/fallback-context-signals/consume_ciao.py"

def load(path):
    envs = []
    with open(path, encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if line:
                envs.append(json.loads(line))
    return envs

def keep(e):
    return bool(e.get("ts")) and "intent" in (e.get("consumers") or [])

def hhex(handle):
    return hashlib.sha256(handle.encode("utf-8")).hexdigest()[:32]

def build(envs):
    kept = [e for e in envs if keep(e)]
    dropped = len(envs) - len(kept)
    cands, ux = [], []
    for e in kept:
        t, obj, attrs = e["type"], e.get("object") or {}, e.get("attrs") or {}
        if t == "clock_tick":
            ux.append(f"finestra UX: clock_tick {e['ts']} (ttl_signal {e.get('ttl_signal')}s, {e['source']})")
            continue
        if t == "service_up":
            title = f"Host session {attrs.get('computer','?')} / {attrs.get('user','?')}"
        elif t == "file_changed":
            title = attrs.get("path_hint") or attrs.get("name") or "file"
            if attrs.get("last_write"):
                title += f"  (modificato {attrs['last_write']})"
        else:
            continue
        cands.append({
            "title": title,
            "kind": t,
            "source": "ContextSignal",
            "signal_source": e.get("source"),
            "ttl_signal": e.get("ttl_signal"),
            "score": None,
            "handle": hhex(obj.get("handle", "")),
            "handle_display_only": True,
            "version": 1,
            "observed_ts": e["ts"],
            "mode": MODE,
            "authority": False,
        })
    mx = max((c["ttl_signal"] or 0) for c in cands) if cands else 1
    for c in cands:
        c["score"] = round((c["ttl_signal"] or 0) / mx, 3)
    cands.sort(key=lambda c: -c["score"])
    for i, c in enumerate(cands):
        c["index"] = i
    rationale = (f"Utterance «{UTTERANCE}»: nessun LLM disponibile; {MODE}. "
                 f"Candidati derivati da segnali Context osservati (score = ttl_signal normalizzato). "
                 + " ".join(ux) + f" Envelope scartati: {dropped}. Nessuna autorità: solo click utente → UserSelection → Policy.")
    return {"utterance": UTTERANCE, "mode": MODE, "llm": None, "authority": False,
            "rationale": rationale, "ux_window": ux, "dropped": dropped, "candidates": cands}

# ---- rendering (Art tokens v0 approx). Host renderer; not a new painter. ----
BG, FROST, TEXT, MUTED, ACCENT, LINE = "#0b1324", "#16213a", "#E8EEF5", "#8C9AB0", "#2EC4B6", "#26324d"
FONT_DIRS = (
    "/usr/share/fonts/opentype/inter/",
    "/usr/share/fonts/truetype/macos/",
)
def font(w, s):
    name = {"r": "Inter-Regular", "m": "Inter-Medium", "b": "Inter-SemiBold"}[w]
    for folder in FONT_DIRS:
        for ext in (".otf", ".ttf"):
            try:
                return ImageFont.truetype(folder + name + ext, s)
            except Exception:
                continue
    return ImageFont.load_default()

def wrap(d, text, f, width):
    out, cur = [], ""
    for w in text.split():
        t = (cur + " " + w).strip()
        if d.textlength(t, font=f) <= width: cur = t
        else: out.append(cur); cur = w
    if cur: out.append(cur)
    return out

def png_rows(p, path):
    W, rh = 1200, 92
    H = 150 + rh * len(p["candidates"]) + 60
    im = Image.new("RGB", (W, H), BG); d = ImageDraw.Draw(im)
    d.text((40, 32), f"{MODE} — nessuna autorità", font=font("b", 24), fill=TEXT)
    d.text((40, 70), f"Proposal per «{p['utterance']}» · LLM assente · handle hash display-only (non LFS)", font=font("r", 14), fill=MUTED)
    d.line((40, 110, W-40, 110), fill=LINE, width=1)
    y = 126
    for c in p["candidates"]:
        d.rounded_rectangle((40, y, W-40, y+rh-12), radius=10, fill=FROST)
        d.text((60, y+12), f"[{c['index']}]  {c['title']}", font=font("b", 18), fill=TEXT)
        d.text((60, y+44), f"source {c['source']} ({c['signal_source']})  ·  score {c['score']:.3f}  ·  handle {c['handle'][:12]}…  ·  v{c['version']}  ·  authority=false",
               font=font("r", 13), fill=MUTED)
        y += rh
    d.text((40, H-40), "Proposal-only · nessuna esecuzione · selezione solo via click utente → Policy", font=font("r", 12), fill=MUTED)
    im.save(path)

def png_glass(p, path):
    W, H = 1280, 800
    im = Image.new("RGB", (W, H), BG); d = ImageDraw.Draw(im)
    # familiar toolbar (Shell chrome placeholder)
    d.rectangle((0, 0, W, 52), fill="#0f1a30"); d.line((0, 52, W, 52), fill=LINE)
    d.rounded_rectangle((340, 11, 940, 41), radius=15, fill="#1b2742")
    d.text((360, 17), p["utterance"], font=font("r", 15), fill=TEXT)
    d.text((20, 17), "VelaOS", font=font("b", 15), fill=MUTED)
    # dock placeholder
    d.rounded_rectangle((440, H-64, 840, H-14), radius=14, fill="#0f1a30")
    # glass surface
    gx0, gy0, gx1 = 200, 100, W-200
    rows = p["candidates"]
    rat = wrap(d, p["rationale"], font("r", 14), gx1-gx0-64)
    gy1 = gy0 + 110 + 96*len(rows) + 22*len(rat) + 80
    d.rounded_rectangle((gx0, gy0, gx1, gy1), radius=18, fill=FROST, outline=LINE, width=1)
    d.text((gx0+32, gy0+28), f"Piano proposto per «{p['utterance']}» — candidati, nessuna autorità", font=font("b", 20), fill=TEXT)
    d.text((gx0+32, gy0+62), "Glass efimero · superficie contesto-driven · non chat", font=font("r", 12), fill=MUTED)
    y = gy0 + 100
    for i, c in enumerate(rows):
        d.rounded_rectangle((gx0+24, y, gx1-24, y+84), radius=10, fill="#1d2a47")
        d.text((gx0+44, y+12), f"[{c['index']}]  {c['title']}", font=font("b", 15), fill=TEXT)
        d.text((gx0+44, y+40), f"{c['source']} · score {c['score']:.3f} · handle {c['handle'][:12]}… · v{c['version']}", font=font("r", 12), fill=MUTED)
        bx = gx1-24-130
        if i == 0:
            d.rounded_rectangle((bx, y+24, bx+100, y+60), radius=8, fill=ACCENT)
            d.text((bx+28, y+32), "Avvia", font=font("b", 15), fill=BG)
        else:
            d.rounded_rectangle((bx, y+24, bx+100, y+60), radius=8, outline=MUTED, width=1)
            d.text((bx+24, y+32), "Rivedi", font=font("m", 15), fill=TEXT)
        y += 96
    y += 6
    d.text((gx0+32, y), "Rationale (display-only)", font=font("m", 12), fill=MUTED); y += 22
    for ln in rat:
        d.text((gx0+32, y), ln, font=font("r", 14), fill=TEXT); y += 22
    d.text((gx0+32, gy1-34), f"{MODE} · LLM assente · authority=false · click → UserSelection → Policy", font=font("r", 12), fill=MUTED)
    im.save(path)

def main():
    print("DEMO COMMAND:", DEMO_CMD)
    print("MODE:", MODE, "(NOT an LLM)")
    p = build(load(FIXTURE))
    print(json.dumps(p["candidates"], ensure_ascii=False, indent=2))
    if not p["candidates"]:
        print("ERROR: Proposal empty", file=sys.stderr); sys.exit(2)
    os.makedirs(OUT, exist_ok=True)
    jp = os.path.join(OUT, "proposal-ciao.json")
    with open(jp, "w", encoding="utf-8") as f:
        json.dump(p, f, ensure_ascii=False, indent=2)
        f.write("\n")
    a, b = os.path.join(OUT, "01-proposal-rows.png"), os.path.join(OUT, "02-glass-open.png")
    png_rows(p, a); png_glass(p, b)
    for x in (jp, a, b):
        print("WROTE", x, os.path.getsize(x), "bytes")

if __name__ == "__main__":
    main()
