"""Index of installed apps (.desktop files) with Italian names, for «apri …»."""
import configparser
import glob
import os
import unicodedata


def _norm(s):
    s = unicodedata.normalize("NFKD", s or "").encode("ascii", "ignore").decode().lower()
    return " ".join(s.replace("-", " ").replace("_", " ").split())


def app_dirs():
    data_home = os.environ.get("XDG_DATA_HOME") or os.path.expanduser("~/.local/share")
    dirs = [data_home] + (os.environ.get("XDG_DATA_DIRS") or "/usr/local/share:/usr/share").split(":")
    dirs += ["/var/lib/flatpak/exports/share", os.path.expanduser("~/.local/share/flatpak/exports/share")]
    out = []
    for d in dirs:
        a = os.path.join(d, "applications")
        if os.path.isdir(a) and a not in out:
            out.append(a)
    return out


def load_apps(lang="it"):
    apps, seen = [], set()
    for d in app_dirs():
        for path in sorted(glob.glob(os.path.join(d, "**", "*.desktop"), recursive=True)):
            did = os.path.relpath(path, d).replace("/", "-")
            if did in seen:
                continue
            seen.add(did)
            cp = configparser.RawConfigParser(strict=False, interpolation=None)
            cp.optionxform = str
            try:
                cp.read(path, encoding="utf-8")
                e = cp["Desktop Entry"]
            except (configparser.Error, KeyError, UnicodeDecodeError):
                continue
            if e.get("Type", "Application") != "Application" or e.get("NoDisplay") == "true" \
                    or e.get("Hidden") == "true":
                continue
            name = e.get(f"Name[{lang}]") or e.get("Name") or did
            generic = e.get(f"GenericName[{lang}]") or e.get("GenericName") or ""
            kw = e.get(f"Keywords[{lang}]") or e.get("Keywords") or ""
            apps.append({"id": did, "path": path, "name": name, "generic": generic,
                         "icon": e.get("Icon", ""), "keywords": [k for k in kw.split(";") if k],
                         "en": e.get("Name") or name})
    return apps


def find_app(query, apps):
    """Best match or None. Scores exact name > prefix > word/keyword > generic name."""
    q = _norm(query)
    if not q:
        return None
    best, best_score = None, 0
    for a in apps:
        names = [_norm(a["name"]), _norm(a["en"]), _norm(a["id"].removesuffix(".desktop").split(".")[-1])]
        score = 0
        for n in names:
            if q == n:
                score = max(score, 100)
            elif n.startswith(q) or q.startswith(n) and len(n) >= 4:
                score = max(score, 80)
            elif q in n.split():
                score = max(score, 70)
        if q in [_norm(k) for k in a["keywords"]]:
            score = max(score, 60)
        if q == _norm(a["generic"]) or q in _norm(a["generic"]).split():
            score = max(score, 50)
        if score > best_score:
            best, best_score = a, score
    return best if best_score >= 50 else None
