"""Safe previews ("dry runs") of what a command would change.

Only read-only operations are performed here: path expansion, stat/walk with
budgets, and a short list of well-known simulation modes (apt-get -s,
git clean -n, rsync --dry-run, sed --sandbox without -i, find without -delete,
tar -t, unzip -l). The user's command itself is NEVER executed by preview().
"""
from __future__ import annotations

import difflib
import glob
import os
import shutil
import subprocess
import time

from .shparse import parse
from .danger import classify, SAFE, READONLY, _base, _flags, _effective_program, ASSIGN_RE

MAX_ITEMS = 200
WALK_BUDGET_S = 1.5
SUBPROC_TIMEOUT = 8


def _t(lang, it, en):
    return it if lang == "it" else en


def _items(lang, n):
    if lang == "it":
        return "1 elemento" if n == 1 else f"{n} elementi"
    return "1 item" if n == 1 else f"{n} items"


def _files(lang, n):
    if lang == "it":
        return "1 file" if n == 1 else f"{n} file"
    return "1 file" if n == 1 else f"{n} files"


def human_size(n: int) -> str:
    for unit in ("B", "KB", "MB", "GB", "TB"):
        if n < 1024 or unit == "TB":
            return f"{n:.0f} {unit}" if unit == "B" else f"{n:.1f} {unit}"
        n /= 1024.0
    return f"{n} B"


def _expand(arg: str, cwd: str):
    """Expand ~ and globs like the shell would, without running anything.
    Returns (paths, ok). ok=False when the arg contains $( ) / ` / $VAR we can't resolve."""
    if "$(" in arg or "`" in arg:
        return [], False
    if "$" in arg:
        expanded = os.path.expandvars(arg)
        if "$" in expanded:
            return [], False
        arg = expanded
    arg = os.path.expanduser(arg)
    full = arg if os.path.isabs(arg) else os.path.join(cwd, arg)
    if any(c in arg for c in "*?["):
        hits = sorted(glob.glob(full))
        return hits, True
    return [full], True


def _walk_stats(path: str, deadline: float):
    files = dirs = size = 0
    truncated = False
    for root, dnames, fnames in os.walk(path, onerror=lambda e: None):
        dirs += len(dnames)
        for f in fnames:
            files += 1
            try:
                size += os.lstat(os.path.join(root, f)).st_size
            except OSError:
                pass
        if time.monotonic() > deadline:
            truncated = True
            break
    return files, dirs, size, truncated


def _describe_paths(paths, cwd, lang, recursive, deadline):
    lines = []
    total_size = 0
    total_files = 0
    missing = 0
    for p in paths[:MAX_ITEMS]:
        rel = os.path.relpath(p, cwd) if p.startswith(cwd + os.sep) else p
        if not os.path.lexists(p):
            missing += 1
            lines.append(_t(lang, f"  {rel}  (non esiste)", f"  {rel}  (does not exist)"))
            continue
        if os.path.isdir(p) and not os.path.islink(p):
            if recursive:
                f, d, s, trunc = _walk_stats(p, deadline)
                total_size += s
                total_files += f
                more = "+" if trunc else ""
                lines.append(_t(lang, f"  {rel}/  cartella: {f}{more} file, {d}{more} sottocartelle, {human_size(s)}{more}",
                                f"  {rel}/  folder: {f}{more} files, {d}{more} subfolders, {human_size(s)}{more}"))
            else:
                lines.append(_t(lang, f"  {rel}/  cartella", f"  {rel}/  folder"))
        else:
            try:
                s = os.lstat(p).st_size
            except OSError:
                s = 0
            total_size += s
            total_files += 1
            lines.append(f"  {rel}  ({human_size(s)})")
    if len(paths) > MAX_ITEMS:
        lines.append(_t(lang, f"  … e altri {len(paths) - MAX_ITEMS}", f"  … and {len(paths) - MAX_ITEMS} more"))
    return lines, total_files, total_size, missing


def _run(argv, cwd, timeout=SUBPROC_TIMEOUT):
    env = dict(os.environ, LC_ALL=os.environ.get("LC_ALL", ""), GIT_PAGER="cat", PAGER="cat")
    try:
        p = subprocess.run(argv, cwd=cwd, capture_output=True, text=True, timeout=timeout,
                           stdin=subprocess.DEVNULL, env=env)
        return p.returncode, (p.stdout or "") + (p.stderr or "")
    except FileNotFoundError:
        return 127, ""
    except subprocess.TimeoutExpired:
        return 124, ""


def _trim(text, max_lines=60):
    lines = text.rstrip("\n").splitlines()
    if len(lines) > max_lines:
        return "\n".join(lines[:max_lines] + [f"… (+{len(lines) - max_lines})"])
    return "\n".join(lines)


def preview(command: str, cwd: str | None = None, lang: str = "it") -> dict:
    """Return {"kind": "...", "text": str, "items": int, "exact": bool}."""
    cwd = cwd or os.getcwd()
    a = classify(command)
    if a.level == SAFE:
        return {"kind": "readonly", "exact": True, "items": 0,
                "text": _t(lang, "Solo lettura: questo comando non modifica nulla.",
                           "Read-only: this command changes nothing.")}
    res = parse(command)
    deadline = time.monotonic() + WALK_BUDGET_S
    blocks = []
    exact = True
    items = 0
    for pipe in res.pipelines:
        for cmd in pipe.commands:
            words = [w.value for w in cmd.words]
            while words and ASSIGN_RE.match(words[0]):
                words = words[1:]
            prog = _effective_program(cmd)
            if prog in ("sudo", "doas", "pkexec", "run0") or not words:
                continue
            # drop wrappers (sudo, env, nice...) to find the real argv
            idx = next((i for i, w in enumerate(words) if _base(w) == prog), 0)
            argv = words[idx:]
            args = argv[1:]
            b, ok, n = _preview_one(prog, argv, args, cmd, cwd, lang, deadline)
            if b:
                blocks.append(b)
            exact = exact and ok
            items += n
            for r in cmd.redirects:
                if r.op in (">", ">|", ">>"):
                    t, okx = _expand(r.target, cwd)
                    if not okx or not t:
                        exact = False
                        continue
                    t = t[0]
                    if t in ("/dev/null",):
                        continue
                    if os.path.exists(t):
                        sz = human_size(os.path.getsize(t)) if os.path.isfile(t) else "?"
                        if r.op == ">>":
                            blocks.append(_t(lang, f"Aggiunge testo in fondo a {r.target} (ora {sz}).",
                                             f"Appends to {r.target} (currently {sz})."))
                        else:
                            blocks.append(_t(lang, f"Sovrascrive {r.target} (ora {sz}): il contenuto attuale andrà perso.",
                                             f"Overwrites {r.target} (currently {sz}): current content will be lost."))
                    else:
                        blocks.append(_t(lang, f"Crea il nuovo file {r.target}.", f"Creates new file {r.target}."))
                    items += 1
    if res.substitutions or res.uncertain:
        exact = False
    if not blocks:
        return {"kind": "none", "exact": False, "items": 0,
                "text": _t(lang, "Anteprima automatica non disponibile per questo comando: leggi la spiegazione prima di confermare.",
                           "No automatic preview for this command: read the explanation before confirming.")}
    text = "\n\n".join(blocks)
    if not exact:
        text += "\n\n" + _t(lang, "Nota: anteprima parziale (il comando contiene parti calcolate al momento).",
                            "Note: partial preview (the command contains parts computed at runtime).")
    return {"kind": "preview", "exact": exact, "items": items, "text": text}


def _preview_one(prog, argv, args, cmd, cwd, lang, deadline):
    """Return (text or None, exact, count)."""
    short, long_, pos = _flags(args)
    if prog in ("rm", "unlink", "trash-put", "trash", "rmdir") or (prog == "gio" and args[:1] in (["trash"], ["remove"])):
        if prog == "gio":
            short, long_, pos = _flags(args[1:])
        recursive = bool({"r", "R"} & short) or "--recursive" in long_
        paths, exact = [], True
        for p in pos:
            e, ok = _expand(p, cwd)
            exact &= ok
            paths += e
        if not paths:
            return (_t(lang, "Nessun file corrisponde: non verrà cancellato nulla.",
                       "No file matches: nothing will be deleted.") if exact else None), exact, 0
        lines, nfiles, size, missing = _describe_paths(paths, cwd, lang, recursive, deadline)
        if missing == len(paths):
            return _t(lang, "Nessuno di questi file esiste: non verrà cancellato nulla.\n",
                      "None of these files exist: nothing will be deleted.\n") + "\n".join(lines), exact, 0
        trash = prog in ("trash-put", "trash") or (prog == "gio" and args[:1] == ["trash"])
        head = (_t(lang, f"Sposterà nel Cestino {_items(lang, len(paths) - missing)} ({human_size(size)}):",
                   f"Will move {_items(lang, len(paths) - missing)} to the Trash ({human_size(size)}):") if trash else
                _t(lang, f"Cancellerà definitivamente {_items(lang, len(paths) - missing)}, {_files(lang, nfiles)} in totale ({human_size(size)}), senza Cestino:",
                   f"Will permanently delete {_items(lang, len(paths) - missing)}, {_files(lang, nfiles)} in total ({human_size(size)}), no Trash:"))
        return head + "\n" + "\n".join(lines), exact, len(paths)
    if prog in ("mv", "cp", "ln", "install") and len(pos) >= 2:
        srcs, exact = [], True
        for p in pos[:-1]:
            e, ok = _expand(p, cwd)
            exact &= ok
            srcs += e
        dst_l, ok = _expand(pos[-1], cwd)
        exact &= ok
        if not dst_l:
            return None, False, 0
        dst = dst_l[0]
        out = []
        verb = {"mv": _t(lang, "Sposta", "Moves"), "cp": _t(lang, "Copia", "Copies"),
                "ln": _t(lang, "Collega", "Links"), "install": _t(lang, "Installa", "Installs")}[prog]
        dst_is_dir = os.path.isdir(dst)
        overwrite = 0
        for s in srcs[:MAX_ITEMS]:
            target = os.path.join(dst, os.path.basename(s)) if dst_is_dir else dst
            mark = ""
            if os.path.exists(target):
                overwrite += 1
                mark = _t(lang, "  ⚠ sovrascrive un file esistente", "  ⚠ overwrites an existing file")
            if not os.path.lexists(s):
                mark = _t(lang, "  (non esiste)", "  (does not exist)")
            out.append(f"  {os.path.relpath(s, cwd)} → {os.path.relpath(target, cwd)}{mark}")
        if len(srcs) > MAX_ITEMS:
            out.append(f"  … +{len(srcs) - MAX_ITEMS}")
        head = f"{verb} {_items(lang, len(srcs))}"
        if not dst_is_dir and len(srcs) > 1:
            head += _t(lang, " — attenzione: la destinazione non è una cartella", " — warning: destination is not a folder")
        if overwrite:
            head += _t(lang, f" ({overwrite} sovrascritti)", f" ({overwrite} overwritten)")
        if not srcs:
            head = _t(lang, "Nessun file corrisponde.", "No file matches.")
        return head + ":\n" + "\n".join(out), exact, len(srcs)
    if prog in ("chmod", "chown", "chgrp") and len(pos) >= 2:
        recursive = "R" in short or "--recursive" in long_
        paths, exact = [], True
        for p in pos[1:]:
            e, ok = _expand(p, cwd)
            exact &= ok
            paths += e
        out = []
        for p in paths[:MAX_ITEMS]:
            try:
                st = os.lstat(p)
                import pwd
                import grp
                import stat as stmod
                owner = pwd.getpwuid(st.st_uid).pw_name if st.st_uid is not None else "?"
                group = grp.getgrgid(st.st_gid).gr_name
                out.append(f"  {os.path.relpath(p, cwd)}  {stmod.filemode(st.st_mode)} {owner}:{group}")
            except (OSError, KeyError):
                out.append(_t(lang, f"  {p}  (non esiste)", f"  {p}  (does not exist)"))
        extra = ""
        if recursive:
            tot = 0
            for p in paths:
                if os.path.isdir(p):
                    f, d, _s, tr = _walk_stats(p, deadline)
                    tot += f + d
            extra = _t(lang, f"\nIn modo ricorsivo: tocca anche {tot} elementi contenuti.",
                       f"\nRecursively: also affects {tot} contained items.")
        what = {"chmod": _t(lang, f"Imposta i permessi «{pos[0]}» su", f"Sets permissions '{pos[0]}' on"),
                "chown": _t(lang, f"Cambia il proprietario in «{pos[0]}» per", f"Changes owner to '{pos[0]}' for"),
                "chgrp": _t(lang, f"Cambia il gruppo in «{pos[0]}» per", f"Changes group to '{pos[0]}' for")}[prog]
        return f"{what} {_items(lang, len(paths))} " + _t(lang, "(stato attuale):", "(current state):") + "\n" + "\n".join(out) + extra, exact, len(paths)
    if prog in ("mkdir", "touch"):
        out = []
        for p in pos:
            e, ok = _expand(p, cwd)
            for x in e:
                ex = os.path.exists(x)
                if prog == "mkdir":
                    out.append(f"  {p}  " + (_t(lang, "(esiste già)", "(already exists)") if ex else _t(lang, "(nuova)", "(new)")))
                else:
                    out.append(f"  {p}  " + (_t(lang, "(aggiorna la data)", "(updates timestamp)") if ex else _t(lang, "(nuovo file vuoto)", "(new empty file)")))
        head = _t(lang, "Crea:", "Creates:")
        return head + "\n" + "\n".join(out), True, len(out)
    if prog == "sed" and any(x == "-i" or x.startswith("-i") or x.startswith("--in-place") for x in args):
        clean = [x for x in args if not (x == "-i" or x.startswith("-i") or x.startswith("--in-place"))]
        s2, l2, pos2 = _flags(clean)
        if "e" in s2 or "--expression" in l2 or "f" in s2:
            script_args = [x for x in clean if x.startswith("-")]
            files = pos2
            # with -e the scripts are option arguments: rebuild conservatively
            opts, files, i = [], [], 0
            while i < len(clean):
                x = clean[i]
                if x in ("-e", "--expression", "-f", "--file"):
                    opts += [x, clean[i + 1]] if i + 1 < len(clean) else [x]
                    i += 2
                    continue
                if x.startswith("-"):
                    opts.append(x)
                else:
                    files.append(x)
                i += 1
            del script_args
        else:
            if not pos2:
                return None, False, 0
            opts = [x for x in clean if x.startswith("-")] + [pos2[0]]
            files = pos2[1:]
        diffs = []
        for f in files:
            e, ok = _expand(f, cwd)
            for path in e:
                if not os.path.isfile(path) or os.path.getsize(path) > 2_000_000:
                    continue
                rc, out = _run(["sed", "--sandbox", *opts, path], cwd)
                if rc != 0:
                    diffs.append(_t(lang, f"  {f}: sed non riesce ad applicare lo script", f"  {f}: sed cannot apply the script"))
                    continue
                with open(path, errors="replace") as fh:
                    old = fh.read()
                d = "".join(difflib.unified_diff(old.splitlines(True), out.splitlines(True),
                                                 fromfile=f + _t(lang, " (prima)", " (before)"),
                                                 tofile=f + _t(lang, " (dopo)", " (after)"), n=1))
                diffs.append(d or _t(lang, f"  {f}: nessuna modifica", f"  {f}: no changes"))
        if not diffs:
            return None, False, 0
        return _t(lang, "Modifiche ai file (diff):", "File changes (diff):") + "\n" + _trim("\n".join(diffs), 80), True, len(diffs)
    if prog in ("apt", "apt-get", "aptitude") and args:
        sub = next((x for x in args if not x.startswith("-")), "")
        if sub in ("install", "remove", "purge", "upgrade", "full-upgrade", "dist-upgrade", "autoremove", "reinstall"):
            simsub = "dist-upgrade" if sub == "full-upgrade" else sub
            pkgs = [x for x in args if not x.startswith("-") and x != sub]
            rc, out = _run(["apt-get", "-s", "-q", simsub, *pkgs], cwd, timeout=20)
            inst = [l.split()[1] for l in out.splitlines() if l.startswith("Inst ")]
            remv = [l.split()[1] for l in out.splitlines() if l.startswith("Remv ") or l.startswith("Purg ")]
            if rc != 0 and not inst and not remv:
                tail = _trim(out, 8)
                return _t(lang, "Simulazione apt non riuscita:\n", "apt simulation failed:\n") + tail, False, 0
            parts = []
            if inst:
                parts.append(_t(lang, f"Installa o aggiorna {len(inst)} pacchetti: ", f"Installs or upgrades {len(inst)} packages: ") + ", ".join(inst[:40]) + (" …" if len(inst) > 40 else ""))
            if remv:
                parts.append(_t(lang, f"RIMUOVE {len(remv)} pacchetti: ", f"REMOVES {len(remv)} packages: ") + ", ".join(remv[:60]) + (" …" if len(remv) > 60 else ""))
            if not parts:
                parts.append(_t(lang, "Nessun pacchetto da cambiare.", "No packages to change."))
            return _t(lang, "Simulazione (apt-get -s):\n", "Simulation (apt-get -s):\n") + "\n".join(parts), True, len(inst) + len(remv)
    if prog == "git" and args:
        sub = args[0]
        if sub == "clean":
            flags = [x for x in args[1:] if x.startswith("-")]
            flags = [x.replace("f", "") for x in flags if x.replace("f", "") not in ("-", "")]
            rc, out = _run(["git", "clean", "-n", *flags], cwd)
            return _t(lang, "File che git clean cancellerebbe:\n", "Files git clean would delete:\n") + (_trim(out) or _t(lang, "  nessuno", "  none")), rc == 0, out.count("\n")
        if sub in ("reset", "checkout", "restore") :
            rc, out = _run(["git", "status", "--short"], cwd)
            rc2, stat = _run(["git", "diff", "--stat"], cwd)
            return _t(lang, "Modifiche locali che potrebbero andare perse:\n", "Local changes that may be lost:\n") + (_trim(out) or _t(lang, "  nessuna", "  none")) + ("\n" + _trim(stat, 20) if stat.strip() else ""), rc == 0, out.count("\n")
        if sub == "push":
            rc, out = _run(["git", "log", "--oneline", "@{u}..HEAD"], cwd)
            if rc == 0:
                return _t(lang, "Commit che verranno inviati:\n", "Commits that will be pushed:\n") + (_trim(out) or _t(lang, "  nessuno", "  none")), True, out.count("\n")
    if prog == "rsync" and not any(":" in p and not p.startswith("/") for p in pos):
        rc, out = _run(["rsync", "--dry-run", "--itemize-changes", *args], cwd, timeout=20)
        return _t(lang, "Simulazione (rsync --dry-run):\n", "Simulation (rsync --dry-run):\n") + _trim(out), rc == 0, out.count("\n")
    if prog == "find" and ("-delete" in args or any(x in ("-exec", "-execdir") for x in args)):
        if any(x in ("-fprint", "-fprintf", "-fls", "-fprint0", "-ok", "-okdir") for x in args):
            return None, False, 0
        new, skip = [], False
        for x in args:
            if skip:
                if x in (";", "+", "\\;"):
                    skip = False
                continue
            if x in ("-exec", "-execdir"):
                skip = True
                new.append("-print")
                continue
            new.append("-print" if x == "-delete" else x)
        rc, out = _run(["find", *new], cwd, timeout=10)
        n = len([l for l in out.splitlines() if l.strip()])
        return _t(lang, f"File trovati ({n}) su cui agirebbe:\n", f"Matched files ({n}) it would act on:\n") + (_trim(out, 50) or _t(lang, "  nessuno", "  none")), rc == 0, n
    if prog == "tar" and args and ("x" in args[0] or "--extract" in args):
        f = None
        for i, x in enumerate(args):
            if x.startswith("-") and "f" in x and not x.startswith("--") and i + 1 < len(args):
                f = args[i + 1]
            elif not x.startswith("-") and i == 0 and "f" in x and len(args) > 1:
                f = args[1]
        if f:
            rc, out = _run(["tar", "-tf", f], cwd, timeout=15)
            n = len(out.splitlines())
            return _t(lang, f"L'archivio contiene {n} elementi:\n", f"The archive contains {n} entries:\n") + _trim(out, 30), rc == 0, n
    if prog == "unzip" and pos:
        rc, out = _run(["unzip", "-l", pos[0]], cwd, timeout=15)
        return _t(lang, "Contenuto dell'archivio:\n", "Archive contents:\n") + _trim(out, 30), rc == 0, out.count("\n")
    if prog == "dd":
        of = [x[3:] for x in args if x.startswith("of=")]
        if of:
            tgt = of[0]
            if tgt.startswith("/dev/") and shutil.which("lsblk"):
                rc, out = _run(["lsblk", "-o", "NAME,SIZE,TYPE,MOUNTPOINTS,MODEL", tgt], cwd)
                return _t(lang, f"Scriverà direttamente su {tgt}. TUTTI i dati sul dispositivo verranno distrutti:\n",
                          f"Will write directly to {tgt}. ALL data on the device will be destroyed:\n") + _trim(out), True, 1
            ex = os.path.exists(os.path.join(cwd, os.path.expanduser(tgt)))
            return _t(lang, f"Scriverà su {tgt}" + (" (sovrascrivendolo)" if ex else " (nuovo file)"),
                      f"Will write to {tgt}" + (" (overwriting it)" if ex else " (new file)")), True, 1
    if prog == "truncate":
        out = []
        for p in pos:
            e, ok = _expand(p, cwd)
            for x in e:
                if os.path.isfile(x):
                    out.append(f"  {p}  ({human_size(os.path.getsize(x))})")
        if out:
            return _t(lang, "Ridimensiona o svuota:\n", "Resizes or empties:\n") + "\n".join(out), True, len(out)
    if prog == "flatpak" and args[:1] and args[0] in ("install", "uninstall", "remove", "update"):
        return _t(lang, "Flatpak chiederà conferma mostrando l'elenco esatto prima di procedere.",
                  "Flatpak will ask for confirmation showing the exact list before proceeding."), False, 0
    return None, prog in READONLY, 0
