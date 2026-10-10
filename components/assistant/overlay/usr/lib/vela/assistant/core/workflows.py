"""Narrow local workflows. File plans are durable before any mutation."""
import contextlib
import fcntl
import json
import os
import stat
import tempfile
import uuid

from . import config, sysutil


class WorkflowError(Exception):
    pass


GROUPS = {
    "Immagini": {".jpg", ".jpeg", ".png", ".webp", ".gif", ".heic", ".svg"},
    "Video": {".mp4", ".mkv", ".mov", ".webm"},
    "Audio": {".mp3", ".wav", ".flac", ".ogg", ".m4a"},
    "Archivi": {".zip", ".7z", ".tar", ".gz", ".xz"},
}


def home_path(path):
    home = os.path.realpath(os.path.expanduser("~"))
    raw = os.path.abspath(os.path.expanduser(path))
    p = os.path.realpath(raw)
    if p != raw or not p.startswith(home + os.sep):
        raise WorkflowError("Scegli una cartella nella Home, senza collegamenti simbolici.")
    parts = os.path.relpath(p, home).split(os.sep)
    if any(s.startswith(".") for s in parts):
        raise WorkflowError("Le cartelle nascoste e di configurazione sono escluse.")
    return p


def journal_dir():
    p = os.path.join(config.data_dir(), "operations")
    os.makedirs(p, mode=0o700, exist_ok=True)
    return p


def save(rec):
    p = os.path.join(journal_dir(), rec["id"] + ".json")
    fd, tmp = tempfile.mkstemp(dir=journal_dir(), prefix=".plan-")
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as f:
            json.dump(rec, f, ensure_ascii=False)
            f.flush()
            os.fsync(f.fileno())
        os.replace(tmp, p)
        fd = os.open(journal_dir(), os.O_RDONLY | os.O_DIRECTORY)
        try:
            os.fsync(fd)
        finally:
            os.close(fd)
    finally:
        if os.path.exists(tmp):
            os.unlink(tmp)


def load(ident):
    if not isinstance(ident, str) or len(ident) != 32 or any(c not in "0123456789abcdef" for c in ident):
        raise WorkflowError("Identificativo operazione non valido.")
    with open(os.path.join(journal_dir(), ident + ".json"), encoding="utf-8") as f:
        return json.load(f)


@contextlib.contextmanager
def locked():
    with open(os.path.join(journal_dir(), ".lock"), "a") as f:
        fcntl.flock(f, fcntl.LOCK_EX)
        yield


def fingerprint(p):
    st = os.lstat(p)
    if not stat.S_ISREG(st.st_mode):
        raise WorkflowError("Il file non è più un file normale.")
    # Metadata catches replacement and edits; no reading of document contents.
    return [st.st_dev, st.st_ino, st.st_size, st.st_mtime_ns]


def sync_folder(path):
    fd = os.open(path, os.O_RDONLY | os.O_DIRECTORY)
    try:
        os.fsync(fd)
    finally:
        os.close(fd)


def plan(folder):
    base = home_path(folder)
    if not os.path.isdir(base):
        raise WorkflowError("Cartella inesistente.")
    rec = {"id": uuid.uuid4().hex, "tool": "organize_folder", "folder": base,
           "state": "planned", "entries": [], "created": [], "skipped": []}
    names = sorted(os.listdir(base))
    if len(names) > 1000:
        raise WorkflowError("La cartella supera il limite di 1000 elementi: scegli una cartella più piccola.")
    for name in names:
        src = os.path.join(base, name)
        ext = os.path.splitext(name)[1].lower()
        group = next((g for g, exts in GROUPS.items() if ext in exts), None)
        if name.startswith(".") or not group or not os.path.isfile(src) or os.path.islink(src):
            rec["skipped"].append(name)
            continue
        parent = os.path.join(base, group)
        dst = os.path.join(parent, name)
        if os.path.lexists(dst) or (os.path.lexists(parent) and
                                  (os.path.islink(parent) or not os.path.isdir(parent))):
            rec["skipped"].append(name)
            continue
        rec["entries"].append({"source": src, "destination": dst,
                               "fingerprint": fingerprint(src), "state": "planned"})
    save(rec)
    return rec


def validate_entry(rec, e):
    base = home_path(rec["folder"])
    src, dst = home_path(e["source"]), home_path(e["destination"])
    group = os.path.basename(os.path.dirname(dst))
    if (os.path.dirname(src) != base or os.path.dirname(os.path.dirname(dst)) != base or
            os.path.basename(src) != os.path.basename(dst) or group not in GROUPS or
            os.path.splitext(src)[1].lower() not in GROUPS[group]):
        raise WorkflowError("Il piano non corrisponde alle regole di organizzazione.")
    return src, dst


def execute(ident):
    with locked():
        rec = load(ident)
        if rec["state"] != "planned":
            raise WorkflowError("Operazione già eseguita o interrotta: consulta la cronologia.")
        # Preflight all files before the first mutation.
        if sysutil.cancelled():
            raise WorkflowError("Operazione annullata prima degli spostamenti.")
        for e in rec["entries"]:
            src, dst = validate_entry(rec, e)
            if fingerprint(src) != e["fingerprint"] or os.path.lexists(dst):
                raise WorkflowError("La cartella è cambiata dall'anteprima. Crea una nuova proposta.")
        rec["state"] = "running"
        save(rec)
        try:
            for e in rec["entries"]:
                if sysutil.cancelled():
                    raise WorkflowError("Operazione interrotta. Puoi ripristinare gli spostamenti già eseguiti.")
                src, dst = validate_entry(rec, e)
                parent = os.path.dirname(dst)
                if not os.path.exists(parent):
                    # Record before mkdir: recovery removes it only if empty.
                    if parent not in rec["created"]:
                        rec["created"].append(parent)
                        save(rec)
                    os.mkdir(parent)
                if fingerprint(src) != e["fingerprint"]:
                    raise WorkflowError("Un file è cambiato durante l'operazione.")
                e["state"] = "moving"
                save(rec)
                # Exclusive creation: never overwrite, same filesystem only.
                os.link(src, dst, follow_symlinks=False)
                sync_folder(parent)
                if fingerprint(dst) != e["fingerprint"]:
                    raise WorkflowError("Verifica del file spostato fallita.")
                os.unlink(src)
                sync_folder(os.path.dirname(src))
                e["state"] = "moved"
                save(rec)
            rec["state"] = "done"
            save(rec)
            return rec
        except (OSError, WorkflowError) as exc:
            rec["state"], rec["error"] = "interrupted", str(exc)
            save(rec)
            raise WorkflowError("Operazione interrotta. Il registro permette il recupero: " + str(exc))


def undo(ident):
    with locked():
        rec = load(ident)
        if rec["state"] not in ("done", "running", "interrupted", "undoing"):
            raise WorkflowError("Questa operazione non richiede recupero.")
        rec["state"] = "undoing"
        save(rec)
        for e in reversed(rec["entries"]):
            if e["state"] not in ("moving", "moved"):
                continue
            src, dst = validate_entry(rec, e)
            if os.path.lexists(dst):
                if fingerprint(dst) != e["fingerprint"]:
                    raise WorkflowError("Il file nella destinazione è cambiato: recupero sospeso.")
                if os.path.lexists(src):
                    if fingerprint(src) != e["fingerprint"]:
                        raise WorkflowError("La posizione originale è occupata: nessun file sovrascritto.")
                else:
                    os.link(dst, src, follow_symlinks=False)
                    sync_folder(os.path.dirname(src))
                os.unlink(dst)
                sync_folder(os.path.dirname(dst))
            elif not os.path.lexists(src) or fingerprint(src) != e["fingerprint"]:
                raise WorkflowError("File non trovato: recupero sospeso.")
            e["state"] = "restored"
            save(rec)
        for p in reversed(rec["created"]):
            home_path(p)
            try:
                os.rmdir(p)
            except OSError:
                pass
        rec["state"] = "undone"
        save(rec)
        return rec


def receipts():
    out = []
    for name in sorted(os.listdir(journal_dir())):
        if name.endswith(".json"):
            try:
                rec = load(name[:-5])
                if rec["state"] in ("done", "running", "interrupted", "undoing"):
                    out.append(rec)
            except (OSError, ValueError, KeyError, WorkflowError):
                continue
    return out


def audio_state():
    rc, out, err = sysutil.read(["pactl", "--format=json", "list", "sinks"], 5)
    if rc:
        raise WorkflowError("Il servizio audio non risponde: " + err[:200])
    sinks = json.loads(out)
    rc, default, _ = sysutil.read(["pactl", "get-default-sink"], 5)
    if rc:
        raise WorkflowError("Impossibile leggere l'uscita audio predefinita.")
    return {"default": default.strip(), "sinks": [
        {"name": s["name"], "description": s.get("description", s["name"]),
         "mute": s.get("mute"), "state": s.get("state")} for s in sinks]}


CATALOG = {
    "vlc": {"name": "VLC", "id": "org.videolan.VLC"},
    "gimp": {"name": "GIMP", "id": "org.gimp.GIMP"},
    "libreoffice": {"name": "LibreOffice", "id": "org.libreoffice.LibreOffice"},
    "krita": {"name": "Krita", "id": "org.kde.krita"},
}


def catalog_app(name):
    app = CATALOG.get(name.lower().strip())
    if not app:
        raise WorkflowError("Applicazione fuori catalogo. Disponibili: " + ", ".join(CATALOG))
    return app
