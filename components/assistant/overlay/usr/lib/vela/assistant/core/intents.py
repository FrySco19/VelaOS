"""Clear commands understood without a model («tema scuro», «volume al 30», «apri impostazioni wi-fi»…).

These only build the same action cards the model would request; state changes still need confirmation.
Returns (tool_name, args) or None. Shared by the panel and the KRunner plugin.
"""
import re
import unicodedata


def _n(s):
    s = unicodedata.normalize("NFKD", s or "").encode("ascii", "ignore").decode().lower()
    s = re.sub(r"[!?.,;]+$", "", s.strip())
    return " ".join(s.split())


SETTINGS_WORDS = [
    (r"wi ?-?fi|wireless", "wifi"), (r"rete|network|internet", "rete"), (r"bluetooth", "bluetooth"),
    (r"audio|suono|sound|volume|altoparlant", "audio"), (r"schermo|schermi|display|monitor|risoluzione", "schermo"),
    (r"aspetto|appearance|tema", "aspetto"), (r"colori|colors", "colori"), (r"sfondo|wallpaper", "sfondo"),
    (r"tastiera|keyboard", "tastiera"), (r"mouse", "mouse"), (r"touchpad|trackpad", "touchpad"),
    (r"notifiche|notifications", "notifiche"), (r"energia|batteria|power|risparmio", "energia"),
    (r"utenti|account|users", "utenti"), (r"data|ora|orologio|clock", "data_ora"),
    (r"lingua|language|regione", "lingua"), (r"stampant|printer", "stampanti"),
    (r"app predefinite|default apps|applicazioni predefinite", "app_predefinite"),
    (r"accessibilita|accessibility", "accessibilita"), (r"scorciatoie|shortcuts", "scorciatoie"),
    (r"informazioni|info sistema|about|sistema", "info"), (r"intelligenza artificiale|\bia\b|\bai\b|assistente", "ia"),
    (r"privacy|memoria", "privacy"), (r"font|caratteri", "font"), (r"icone|icons", "icone"),
    (r"aggiornament|updates", "aggiornamenti"), (r"avvio automatico|autostart", "avvio"),
]

_ON = r"(attiva|accendi|abilita|metti|imposta|passa a(l|lla)?|usa|turn on|enable|switch to)"
_OFF = r"(disattiva|spegni|disabilita|togli|turn off|disable)"


def match(text):
    workflow = _workflow_match(text)
    if workflow:
        return workflow
    t = _n(text)
    if not t or len(t) > 80:
        return None

    # help
    if re.fullmatch(r"(aiuto|help|guida|scorciatoie|scorciatoie da tastiera|mostra (le )?scorciatoie|apri la guida|guida e scorciatoie|keyboard shortcuts|shortcuts)", t):
        return "open_help", {}

    # dark / light mode
    if re.fullmatch(rf"({_ON} )?(il |la )?(tema|modalita|aspetto|dark mode|modo) ?(scur[oa]|notte|dark)( mode)?|dark mode", t) \
            or re.fullmatch(r"(tema|modalita) scur[ao]", t):
        return "set_dark_mode", {"enabled": True}
    if re.fullmatch(rf"({_ON} )?(il |la )?(tema|modalita|aspetto|modo) ?(chiar[oa]|giorno|light)( mode)?|light mode", t) \
            or re.fullmatch(rf"{_OFF} (il |la )?(tema|modalita) scur[ao]|{_OFF} (la )?dark mode", t):
        return "set_dark_mode", {"enabled": False}

    # volume
    m = re.fullmatch(r"(metti |imposta |porta |set )?(il )?volume (al |a |to )?(\d{1,3}) ?%?", t)
    if m:
        return "set_volume", {"percent": min(100, int(m.group(4)))}
    if re.fullmatch(r"(alza|aumenta) (il )?volume|volume (su|piu alto)|turn (the )?volume up|volume up", t):
        return "set_volume", {"change": 10}
    if re.fullmatch(r"(abbassa|diminuisci) (il )?volume|volume (giu|piu basso)|turn (the )?volume down|volume down", t):
        return "set_volume", {"change": -10}
    if re.fullmatch(r"(muto|silenzia|silenzio|togli (l')?audio|mute|disattiva (l')?audio|metti in muto)( (l')?audio)?", t):
        return "set_volume", {"mute": True}
    if re.fullmatch(r"(riattiva|ripristina) (l')?audio|unmute|togli (il )?muto", t):
        return "set_volume", {"mute": False}

    # brightness
    m = re.fullmatch(r"(metti |imposta |porta |set )?(la )?luminosita (al |a |to )?(\d{1,3}) ?%?|brightness (to )?(\d{1,3}) ?%?", t)
    if m:
        return "set_brightness", {"percent": max(1, min(100, int(m.group(4) or m.group(6))))}
    if re.fullmatch(r"(alza|aumenta) (la )?luminosita|brightness up", t):
        return "set_brightness", {"change": 15}
    if re.fullmatch(r"(abbassa|diminuisci) (la )?luminosita|brightness down", t):
        return "set_brightness", {"change": -15}

    # Wi-Fi radio
    if re.fullmatch(rf"{_ON} (il )?wi ?-?fi", t):
        return "set_wifi", {"enabled": True}
    if re.fullmatch(rf"{_OFF} (il )?wi ?-?fi", t):
        return "set_wifi", {"enabled": False}

    # settings pages
    m = re.fullmatch(r"(apri |mostra |vai (a|alle|nelle) |open )?(le )?(impostazioni|settings|preferenze)( (di|del|della|dello|delle|dei|degli|per|for|sul|sulla))?( (il|la|lo|l'|le|i|gli))? ?(?P<rest>.*)", t)
    if m:
        rest = m.group("rest").strip()
        if not rest:
            return "open_app", {"name": "Impostazioni"}
        for pat, page in SETTINGS_WORDS:
            if re.search(pat, rest):
                return "open_settings", {"page": page}
        return None

    # find files
    m = re.fullmatch(r"(cerca|trova|dov'?e|dove si trova|find|search( for)?) (il |i |la |le |lo |gli |un |una )?(file|documento|documenti|cartella|foto|immagine|immagini|pdf)( (di|del|della|con|chiamat[oa]|named))? (.+)", t)
    if m:
        kind = {"cartella": "folder", "foto": "image", "immagine": "image", "immagini": "image", "pdf": "pdf",
                "documento": "document", "documenti": "document"}.get(m.group(4), "any")
        q = re.sub(r"^(il |la |lo |i |le |gli |l')", "", m.group(7)).strip()
        return "find_files", {"query": q, "kind": kind}

    # reminders: «ricordami di X tra 10 minuti» / «ricordami alle 18:30 di X»
    m = re.fullmatch(r"(ricordami|promemoria|remind me)( di| to)? (.+?) (tra|fra|in) (\d+) ?(minuti|minuto|min|ore|ora|h|minutes?|hours?)", t)
    if m:
        unit = "h" if m.group(6)[0] in "oh" else "m"
        return "create_reminder", {"message": _orig_tail(text, m.group(3)), "when": f"{m.group(5)}{unit}"}
    m = re.fullmatch(r"(ricordami|promemoria|remind me)( di| to)? (.+?) alle (\d{1,2})[:.](\d{2})", t)
    if m:
        return "create_reminder", {"message": _orig_tail(text, m.group(3)), "when": f"{m.group(4)}:{m.group(5)}"}
    m = re.fullmatch(r"(ricordami|promemoria|remind me) alle (\d{1,2})[:.](\d{2})( di| to)? (.+)", t)
    if m:
        return "create_reminder", {"message": _orig_tail(text, m.group(5)), "when": f"{m.group(2)}:{m.group(3)}"}

    # open apps: «apri firefox», «avvia il terminale»
    m = re.fullmatch(r"(apri|avvia|lancia|open|launch|start) (l'|il |la |lo |i |le |gli |the |app )?(.{2,40})", t)
    if m:
        return "open_app", {"name": m.group(3)}
    return None


def _workflow_match(text):
    """Explicit single workflows can prepare a preview without CPU inference."""
    raw = (text or "").strip()
    if len(raw) > 512 or "\n" in raw:
        return None
    prefix = r"(?:(?:per favore[,]?|please|mi aiuti a|puoi)\s+)?"
    path = r'''(?P<path>"[^"\n]+"|'[^'\n]+'|(?:~/|/)[^\s?]+)'''
    m = re.fullmatch(prefix + r"(?:organizza(?:re)?|organize)\s+(?:(?:la cartella|cartella|the folder|folder)\s+)?" + path +
                     r"(?:\s+(?:per tipo(?: di file)?|by file type))?\??", raw, re.IGNORECASE)
    if m:
        folder = m.group("path")
        if folder[0] in "\"'":
            folder = folder[1:-1]
        return "organize_folder", {"folder": folder}
    m = re.fullmatch(prefix + r"(?:installa(?:re)?|install)\s+(vlc|gimp|krita|libreoffice)(?:\s+(?:dal catalogo|from the catalog))?\??",
                     raw, re.IGNORECASE)
    if m:
        return "install_catalog_app", {"name": m.group(1).lower()}
    if re.fullmatch(prefix + r"(?:controlla|diagnostica|check|diagnose)\s+(?:l['’])?(?:audio|sound)\??", raw, re.IGNORECASE):
        return "diagnose_audio", {}
    return None


def _orig_tail(original, normalized_piece):
    """Recover the user's original casing/accents for a normalized substring when possible."""
    o = " ".join(original.split())
    idx = _n(o).find(normalized_piece)
    if idx >= 0 and len(_n(o)) == len(o.lower()):
        return o[idx:idx + len(normalized_piece)]
    return normalized_piece
