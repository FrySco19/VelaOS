#!/usr/bin/env python3
"""Copy review helper (owner: onboarding). Extracts user-visible strings from every component and flags
violations of STYLE_GUIDE_COPY.md. Output: a Markdown report (all strings + findings) for human review.

usage: copy_review.py [repo_root] [--out report.md] [--json findings.json]
Heuristics only: a human (onboarding) reads the report and files fixes in coordination/requests.md.
"""
import json, os, re, sys

root = sys.argv[1] if len(sys.argv) > 1 and not sys.argv[1].startswith("--") else "/workspace/vela/v2"
out = sys.argv[sys.argv.index("--out") + 1] if "--out" in sys.argv else None
jout = sys.argv[sys.argv.index("--json") + 1] if "--json" in sys.argv else None

EXT = {".qml", ".py", ".js", ".desktop", ".sh", ".json", ".conf", ".notifyrc", ".html", ".ui", ".kcfg", ".desc"}
SKIP_DIRS = {".git", "__pycache__", "node_modules", "vendor", "tests", "src"}
PATTERNS = [
    re.compile(r'\b(?:qsTr|i18nc?|i18np|tr|_)\(\s*(?:"[^"]*"\s*,\s*)?"((?:[^"\\]|\\.){3,})"'),
    re.compile(r'\b(?:text|title|placeholderText|label|subtitle|description|hint|tooltip|ToolTip\.text|heading|explanation)\s*:\s*"((?:[^"\\]|\\.){3,})"'),
    re.compile(r'^(?:Name|GenericName|Comment|Keywords|X-KDE-Keywords)(?:\[it\])?=(.+)$', re.M),
    re.compile(r'kdialog[^\n]*?--(?:title|msgbox|yesno|sorry|error|checklist|radiolist|inputbox|passivepopup)\s+"([^"]{3,})"'),
    re.compile(r'notify-send[^\n]*?"([^"$]{3,})"'),
    re.compile(r'"(?:it|text_it|it_IT)"\s*:\s*"((?:[^"\\]|\\.){3,})"'),
]
IT_WORDS = re.compile(r"\b(il|lo|la|gli|le|un|una|di|del|della|che|per|non|con|sul|nella|questo|questa|è|sono|puoi|apri|nessun\w*|impossibile)\b", re.I)
GENERIC = re.compile(r'"((?:[^"\\\n]|\\.){8,240})"|\'((?:[^\'\\\n]|\\.){8,240})\'')
RULES = [
    (r"\bAI\b", "In italiano si scrive «IA», non «AI» (glossario)"),
    (r"\bDiscover\b", "Lo store si chiama «Store» (glossario)"),
    (r"Impostazioni di sistema", "L'app si chiama «Impostazioni» (glossario)"),
    (r"(?-i:\b(?!Wi-Fi\b)[Ww][Ii]-?[Ff][Ii]\b)", "Scrivere «Wi-Fi»"),
    (r"!", "Niente punti esclamativi"),
    (r"\bSpotlight\b|\bMission Control\b|\bFinder\b|\bLaunchpad\b", "Niente nomi di prodotti Apple"),
    (r"\bclicca qui\b|\bclick here\b", "Niente «clicca qui»: il pulsante dice l'azione"),
    (r"\bOps\b|\bOops\b|qualcosa è andato storto", "Errori: dire cosa è successo e cosa fare"),
    (r"\.\.\.", "Usare il carattere «…»"),
    (r"\be'|perche'|piu'|gia'|puo'", "Accenti: è, perché, più, già, può"),
    (r"\b(potente|intelligente|rivoluzionari\w*|magic\w*|straordinari\w*|incredibil\w*|senza limiti|il futuro)\b", "Niente aggettivi promozionali"),
    (r"\b(Porto|Banchina|Cima)\b", "Nomi di fantasia v1: usare Tutte le app / Dock / Barra in alto"),
    (r"\b(Errore|Error)\s*:?\s*0x|\berrno\b|Traceback", "Niente codici tecnici a schermo"),
    (r"\b(Settings|Search|Cancel|Delete|Loading|Please|Click|Open|Close|Save|Enable|Disable)\b", "Probabile testo inglese in UI italiana"),
    (r"\bdesktop virtual[ei]\b|\barea di lavoro\b|\baree di lavoro\b", "Usare «Spazi» (glossario)"),
    (r"\b(clipboard|screenshot\b.*\bistantanea|cattura schermo)\b", "Glossario: «Appunti», «Screenshot»"),
    (r"\bVela ?OS\b", "Nel testo dell'interfaccia: «Vela» (solo Informazioni: «Vela 2»)"),
    (r"\bAPI key\b|\btoken API\b", "Usare «Chiave API»"),
    (r"\bLogout\b|\bArresta il sistema\b", "Usare «Esci», «Spegni»"),
]
RULES = [(re.compile(r, re.I if not r.startswith(r"\bAI") else 0), m) for r, m in RULES]
EN_HINT = re.compile(r"\b(the|and|your|you|with|this|that|is|are|to|of)\b", re.I)

def looks_english(s):
    return len(EN_HINT.findall(s)) >= 2 and not re.search(r"[àèéìòù]|\b(il|la|di|che|per|non|una?)\b", s, re.I)

strings, findings = [], []
for comp in sorted(os.listdir(os.path.join(root, "components"))):
    if comp in ("qa",):
        continue
    for dp, dn, fn in os.walk(os.path.join(root, "components", comp)):
        dn[:] = [d for d in dn if d not in SKIP_DIRS]
        for f in fn:
            ext = os.path.splitext(f)[1].lower()
            if ext not in EXT and f not in ("hook.chroot.sh",):
                continue
            p = os.path.join(dp, f)
            rel = os.path.relpath(p, root)
            if comp == "onboarding" and f in ("strings.json", "shortcuts.json", "tips.json"):
                # our own data: check only the Italian side
                pass
            try:
                txt = open(p, encoding="utf-8", errors="replace").read()
            except OSError:
                continue
            if len(txt) > 1_500_000:
                continue
            lines = txt.splitlines()
            for i, line in enumerate(lines, 1):
                if line.lstrip().startswith(("#", "//")) and ext not in (".desktop",):
                    continue
                cands = []
                for rx in PATTERNS:
                    cands += [m.group(1) for m in rx.finditer(line)]
                if ext in (".py", ".js", ".qml", ".sh"):
                    for m in GENERIC.finditer(line):
                        g = m.group(1) or m.group(2)
                        if g and " " in g and IT_WORDS.search(g) and not re.search(r"[{}()=;<>|]|\\[nt]|%\(", g):
                            cands.append(g)
                for s in dict.fromkeys(cands):
                        s = s.strip()
                        if not s or s.startswith(("http", "/", "$", "org.", "%", "#")) or re.fullmatch(r"[\w.\-]+", s) and "_" in s:
                            continue
                        is_en_variant = bool(re.search(r"\[(en|en_US|en_GB)\]=", line)) or '"en"' in line \
                            or line.startswith(("Keywords", "X-KDE-Keywords"))
                        strings.append((comp, rel, i, s))
                        if is_en_variant:
                            continue
                        for rr, msg in RULES:
                            if rr.search(s):
                                # «!» allowed in shell / code-ish strings like "!=" only
                                if msg.startswith("Niente punti") and re.search(r"!=|\$!|#!", s):
                                    continue
                                findings.append({"comp": comp, "file": rel, "line": i, "text": s, "rule": msg})
                        if looks_english(s) and not rel.endswith((".desktop",)):
                            findings.append({"comp": comp, "file": rel, "line": i, "text": s, "rule": "Probabile frase inglese senza traduzione italiana"})

# de-dup
seen, uniq = set(), []
for f in findings:
    k = (f["file"], f["line"], f["rule"])
    if k not in seen:
        seen.add(k); uniq.append(f)
rep = ["# Copy review report", "", f"Strings extracted: {len(strings)} · findings: {len(uniq)}", "", "## Findings", ""]
for f in uniq:
    rep.append(f"- [{f['comp']}] `{f['file']}:{f['line']}` — {f['rule']}: «{f['text'][:160]}»")
rep += ["", "## All extracted strings (for manual reading)", ""]
cur = None
for comp, rel, i, s in strings:
    if comp != cur:
        rep.append(f"\n### {comp}\n"); cur = comp
    rep.append(f"- `{rel}:{i}` {s[:200]}")
text = "\n".join(rep) + "\n"
if out:
    open(out, "w", encoding="utf-8").write(text)
else:
    print(text)
if jout:
    json.dump(uniq, open(jout, "w"), indent=1, ensure_ascii=False)
print(f"strings={len(strings)} findings={len(uniq)}", file=sys.stderr)
