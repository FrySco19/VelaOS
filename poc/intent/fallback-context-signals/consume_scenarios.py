#!/usr/bin/env python3
"""FALLBACK CONTEXT-SIGNALS for the host scenario fixtures.

NOT an LLM. Same Proposal rules and the same PNG renderer as consume_ciao.py.
Titles and honesty lines are the host demo's observed wording. Handles are
sha256 prefixes of object.handle. Nothing is executed.
"""
import json, os, sys

import consume_ciao as cc

HERE = os.path.dirname(os.path.abspath(__file__))
SCENARIO_DIR = os.path.join(HERE, "fixtures", "scenarios")
OUT = os.path.join(HERE, "out")
DEMO_CMD = "python3 poc/intent/fallback-context-signals/consume_scenarios.py"

# Host proposal copy. Keyed by the scenario field on the envelopes.
HOST = {
    "yt-favij": {
        "utterance": "YT Favij",
        "honesty": (
            "nessun match Favij osservato (Chrome+Edge match_count=0). "
            "Nessun URL/canale Favij inventato. Eventi non correlati etichettati unrelated."
        ),
    },
    "whatsapp-mamma": {
        "utterance": "WhatsApp mamma",
        "honesty": (
            "sessione WhatsApp osservata (WhatsApp.Root up; web.whatsapp.com visitata). "
            "contatto mamma non verificabile (contacts.db cifrato / non leggibile). "
            "Mai fingere chat con mamma."
        ),
    },
    "ig-barbiere": {
        "utterance": "IG barbiere",
        "honesty": (
            "zero match Instagram/barbiere osservati; nessun package IG. "
            "Proposal vuota/zero-match dichiarata. Nessuna azione inventata."
        ),
    },
}


def match_count(attrs):
    if "match_count" in attrs:
        return attrs["match_count"]
    keys = [k for k in attrs if k.endswith("_match_count")]
    if not keys:
        return None
    return sum(attrs[k] or 0 for k in keys)


def title_of(e):
    t = e["type"]
    attrs = e.get("attrs") or {}
    kind = (e.get("object") or {}).get("kind")
    if t == "clock_tick":
        return None
    if t == "service_up" and kind == "host_session":
        return f"Host session {attrs.get('computer', '?')} / {attrs.get('user', '?')}"
    if t == "service_up" and kind == "browser_profile":
        return f"Browser profile osservato ({attrs.get('browser', '?')})"
    if t == "service_up" and kind == "process":
        return f"Processo osservato: {attrs.get('process', '?')} (pid {attrs.get('pid', '?')})"
    if t == "file_changed" and kind == "local_path":
        if attrs.get("mamma_contact_verified") is False:
            return f"{attrs.get('path_hint')} · contatto mamma non verificabile"
        title = attrs.get("path_hint") or attrs.get("name") or "file"
        if attrs.get("last_write"):
            title += f"  (modificato {attrs['last_write']})"
        return title
    if t == "file_changed" and kind == "browser_history":
        browser = attrs.get("browser", "?")
        if "keywords" in attrs:
            keyword = ",".join(attrs["keywords"])
        else:
            keyword = attrs.get("keyword", "?")
        mc = match_count(attrs)
        parts = [f"Browser history osservata ({browser}) · keyword={keyword} · match_count={mc}"]
        extra = attrs.get("unrelated_recent_youtube") or []
        if extra:
            parts.append(f"unrelated_recent_youtube: {extra[0].get('title')}")
        if "ig_package_present" in attrs:
            parts.append(f"ig_package_present={str(attrs['ig_package_present']).lower()}")
        latest = attrs.get("latest")
        if isinstance(latest, dict):
            parts.append(f"latest={latest.get('title')} @ {latest.get('visited')}")
        return " · ".join(parts)
    return None


def related_of(e):
    attrs = e.get("attrs") or {}
    kind = (e.get("object") or {}).get("kind")
    if e["type"] == "file_changed" and kind == "browser_history" and match_count(attrs) == 0:
        return "zero-match"
    return "observed"


def build(path):
    envs = cc.load(path)
    kept = [e for e in envs if cc.keep(e)]
    dropped = len(envs) - len(kept)
    scenario = next((e.get("scenario") for e in kept if e.get("scenario")), None)
    host = HOST.get(scenario)
    if host is None:
        print(f"ERROR: no host wording for scenario {scenario!r}", file=sys.stderr)
        sys.exit(2)
    ux, rows = [], []
    for e in kept:
        if e["type"] == "clock_tick":
            ux.append(
                f"finestra UX: clock_tick {e['ts']} (ttl_signal {e.get('ttl_signal')}s, {e['source']})"
            )
            continue
        title = title_of(e)
        if title is None:
            continue
        obj = e.get("object") or {}
        rows.append({
            "title": title,
            "kind": e["type"],
            "source": "ContextSignal",
            "signal_source": e.get("source"),
            "ttl_signal": e.get("ttl_signal"),
            "score": None,
            "handle": cc.hhex(obj.get("handle", "")),
            "handle_display_only": True,
            "version": 1,
            "observed_ts": e["ts"],
            "mode": cc.MODE,
            "authority": False,
            "related": related_of(e),
        })
    mx = max((c["ttl_signal"] or 0) for c in rows) if rows else 1
    for c in rows:
        c["score"] = round((c["ttl_signal"] or 0) / mx, 3)
    rows.sort(key=lambda c: -c["score"])
    for i, c in enumerate(rows):
        c["index"] = i
    honesty = host["honesty"]
    utterance = host["utterance"]
    rationale = (
        f"Utterance «{utterance}»: nessun LLM; {cc.MODE}. {honesty} "
        + " ".join(ux)
        + f" Envelope scartati: {dropped}. Nessuna autorità: solo click utente → UserSelection → Policy (Seleziona)."
    )
    return {
        "utterance": utterance,
        "mode": cc.MODE,
        "llm": None,
        "authority": False,
        "rationale": rationale,
        "honesty": honesty,
        "ux_window": ux,
        "dropped": dropped,
        "candidates": rows,
    }


def main():
    print("DEMO COMMAND:", DEMO_CMD)
    print("MODE:", cc.MODE, "(NOT an LLM)")
    names = sorted(n for n in os.listdir(SCENARIO_DIR) if n.endswith(".jsonl"))
    if not names:
        print("ERROR: no scenario fixtures", file=sys.stderr)
        sys.exit(2)
    os.makedirs(OUT, exist_ok=True)
    for name in names:
        stem = name[:-6]
        proposal = build(os.path.join(SCENARIO_DIR, name))
        if not proposal["candidates"]:
            print(f"ERROR: empty proposal for {stem}", file=sys.stderr)
            sys.exit(2)
        print(f"--- {stem} utterance={proposal['utterance']!r} rows={len(proposal['candidates'])} llm=null")
        for c in proposal["candidates"]:
            print(f"  [{c['index']}] source={c['source']} related={c['related']} {c['title']}")
        jp = os.path.join(OUT, f"{stem}-proposal.json")
        rows_png = os.path.join(OUT, f"{stem}-01-proposal-rows.png")
        glass_png = os.path.join(OUT, f"{stem}-02-glass-open.png")
        with open(jp, "w", encoding="utf-8") as f:
            json.dump(proposal, f, ensure_ascii=False, indent=2)
            f.write("\n")
        cc.png_rows(proposal, rows_png)
        cc.png_glass(proposal, glass_png)
        for path in (jp, rows_png, glass_png):
            print("WROTE", path, os.path.getsize(path), "bytes")


if __name__ == "__main__":
    main()
