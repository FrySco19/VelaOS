#!/usr/bin/env python3
"""Suggerimenti: one tip per day during the first week after setup (desktop notification).

Runs at login (autostart). Shows at most one tip per day; stops after 7 tips or when disabled in
Guida e scorciatoie > Suggerimenti. State: ~/.config/vela/tips.json {enabled, first_day, shown: [dates]}.
  vela-tips            normal login run (waits ~60 s so it doesn't compete with startup)
  vela-tips --now      show today's tip immediately (testing)
  vela-tips --dry-run  print what would be shown
"""
import datetime, json, os, shutil, subprocess, sys, time

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import backend as be  # noqa: E402  (no Qt needed for the helpers used here)


def lang():
    st = be._read(be.STATE_FILE)
    if st.get("language") in ("it", "en"):
        return st["language"]
    env = (os.environ.get("LANGUAGE") or os.environ.get("LANG") or "it")[:2]
    return "en" if env == "en" else "it"


def pick(today=None):
    today = today or datetime.date.today()
    st = be._read(be.TIPS_FILE)
    if not st.get("enabled", True):
        return None, st, "disabled"
    if be.is_live():
        return None, st, "live session"
    if "first_day" not in st:
        st["first_day"] = today.isoformat()
    shown = st.get("shown", [])
    if today.isoformat() in shown:
        return None, st, "already shown today"
    tips = be._read(os.path.join(HERE, "tips.json")).get("tips", [])
    if len(shown) >= min(7, len(tips)):
        return None, st, "all tips shown"
    first = datetime.date.fromisoformat(st["first_day"])
    if (today - first).days > 13:
        return None, st, "more than two weeks after setup"
    cfg = {}
    try:
        sys.path.insert(0, "/usr/lib/vela/python")
        import vela_ai_config as vac
        cfg = vac.load()
    except Exception:
        pass
    akey = ((cfg.get("assistant") or {}).get("shortcut")) or "Meta+A"
    l = lang()
    title, body = tips[len(shown)][l]
    body = body.replace("@assistant", akey.replace("Space", "Spazio" if l == "it" else "Space"))
    title = title.replace("@assistant", akey)
    return (title, body, l), st, "ok"


def main():
    dry = "--dry-run" in sys.argv
    if "--now" not in sys.argv and not dry:
        time.sleep(60)
    tip, st, why = pick()
    if dry:
        print(json.dumps({"tip": tip, "why": why}, ensure_ascii=False))
        return 0
    if not tip:
        if "first_day" in st:
            be._write(be.TIPS_FILE, st)
        return 0
    title, body, l = tip
    st.setdefault("shown", []).append(datetime.date.today().isoformat())
    be._write(be.TIPS_FILE, st)
    if not shutil.which("notify-send"):
        return 0
    more = "Altri suggerimenti" if l == "it" else "More tips"
    off = "Non mostrare più" if l == "it" else "Don't show again"
    app = "Suggerimenti" if l == "it" else "Tips"
    try:
        r = subprocess.run(["notify-send", "--app-name=" + app, "--icon=vela-help", "--expire-time=15000",
                            "--action=more=" + more, "--action=off=" + off, "--wait", title, body],
                           capture_output=True, text=True, timeout=600)
    except (OSError, subprocess.TimeoutExpired):
        return 0
    act = r.stdout.strip()
    if act == "more":
        subprocess.Popen([sys.executable, os.path.join(HERE, "vela_welcome.py"), "--guida"], start_new_session=True)
    elif act == "off":
        be._write(be.TIPS_FILE, dict(be._read(be.TIPS_FILE), enabled=False))
    return 0


if __name__ == "__main__":
    sys.exit(main())
