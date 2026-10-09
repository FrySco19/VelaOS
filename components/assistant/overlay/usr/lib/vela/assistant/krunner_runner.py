#!/usr/bin/env python3
"""KRunner D-Bus runner (org.kde.krunner1) so the launcher (Meta+Space) can hand questions and quick commands
to the assistant.

  «? domanda» / «chiedi …» / a question ending with «?»  → «Chiedi all'assistente: …» (Enter opens the panel)
  «tema scuro», «volume al 30», «spegni il Wi-Fi», «ricordami …» → the matching action; Enter opens the panel
                                                                  with the confirmation card (never runs directly)
Service org.vela.AssistantRunner, path /runner; D-Bus activated, exits after 10 minutes without requests.
"""
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import dbus  # noqa: E402
import dbus.service  # noqa: E402
from dbus.mainloop.glib import DBusGMainLoop  # noqa: E402
from gi.repository import GLib  # noqa: E402

from core import config, i18n, intents  # noqa: E402

IFACE = "org.kde.krunner1"
NAME = "org.vela.AssistantRunner"
IDLE_EXIT_S = 600
# KRunner match types
EXACT, HELPER, POSSIBLE, INFORMATIONAL = 100, 70, 30, 50
INTENT_LABELS = {"set_dark_mode": None, "set_volume": None, "set_brightness": None, "set_wifi": None,
                 "create_reminder": None, "open_help": None}


def describe_intent(tool, args):
    """Title for the launcher row (static: no system queries on every keystroke)."""
    T = i18n.T
    if tool == "set_dark_mode":
        return T("dark_on_title") if args.get("enabled") else T("dark_off_title")
    if tool == "set_volume":
        if args.get("mute") is True:
            return T("mute_title")
        if args.get("mute") is False:
            return T("unmute_title")
        if args.get("percent") is not None:
            return T("volume_title", pct=args["percent"])
        return T("volume_up") if args.get("change", 0) > 0 else T("volume_down")
    if tool == "set_brightness":
        if args.get("percent") is not None:
            return T("brightness_title", pct=args["percent"])
        return T("brightness_up") if args.get("change", 0) > 0 else T("brightness_down")
    if tool == "set_wifi":
        return T("wifi_on_title") if args.get("enabled") else T("wifi_off_title")
    if tool == "create_reminder":
        return T("reminder_title") + ": " + args.get("message", "")
    if tool == "open_help":
        return T("open_help_title")
    return None


def build_matches(query, enabled=True):
    q = (query or "").strip()
    if not q or not enabled:
        return []
    out = []
    low = q.lower()
    ask = None
    if q.startswith("?"):
        ask, rel, typ = q[1:].strip(), 1.0, EXACT
    elif low.startswith(("chiedi ", "chiedi:", "ask ")):
        ask, rel, typ = q.split(None, 1)[1] if " " in q else "", 1.0, EXACT
    m = intents.match(q) if ask is None else None
    if m and m[0] in INTENT_LABELS:
        title = describe_intent(*m)
        if title:
            out.append(("intent:" + json.dumps({"tool": m[0], "args": m[1]}), title, "vela-assistant", HELPER, 0.9,
                        {"subtext": i18n.T("krunner_sub_confirm"), "category": i18n.T("app_name")}))
    if ask is None and (q.endswith("?") and len(q) >= 8 or len(q.split()) >= 4) and not m:
        ask, rel, typ = q, (0.6 if q.endswith("?") else 0.2), POSSIBLE
    if ask:
        out.append(("ask:" + ask, i18n.T("krunner_ask", q=ask if len(ask) < 70 else ask[:67] + "…"), "vela-assistant",
                    typ, rel, {"subtext": i18n.T("krunner_sub"), "category": i18n.T("app_name")}))
    return out


class Runner(dbus.service.Object):
    def __init__(self, bus, loop):
        super().__init__(bus, "/runner")
        self.bus, self.loop = bus, loop
        self.token = ""
        self._idle = None
        self._arm()

    def _arm(self):
        if self._idle:
            GLib.source_remove(self._idle)
        self._idle = GLib.timeout_add_seconds(IDLE_EXIT_S, lambda: (self.loop.quit(), False)[1])

    @dbus.service.method(IFACE, in_signature="", out_signature="a(sss)")
    def Actions(self):
        return dbus.Array([], signature="(sss)")

    @dbus.service.method(IFACE, in_signature="", out_signature="a{sv}")
    def Config(self):
        return dbus.Dictionary({"MinLetterCount": dbus.Int32(3)}, signature="sv")

    @dbus.service.method(IFACE, in_signature="s", out_signature="a(sssida{sv})")
    def Match(self, query):
        self._arm()
        try:
            cfg = config.load()
            i18n.set_lang(config.language(cfg))
            ms = build_matches(str(query), cfg.get("enabled", True))
        except Exception:
            ms = []
        arr = dbus.Array([], signature="(sssida{sv})")
        for mid, text, icon, typ, rel, props in ms:
            arr.append(dbus.Struct((mid, text, icon, dbus.Int32(typ), dbus.Double(rel),
                                    dbus.Dictionary({k: dbus.String(v) for k, v in props.items()}, signature="sv")),
                                   signature="sssida{sv}"))
        return arr

    @dbus.service.method(IFACE, in_signature="s", out_signature="")
    def SetActivationToken(self, token):
        self.token = str(token)

    @dbus.service.method(IFACE, in_signature="ss", out_signature="")
    def Run(self, match_id, action_id):
        self._arm()
        mid = str(match_id)
        try:
            obj = self.bus.get_object("org.vela.Assistant", "/Assistant")   # D-Bus activation starts it if needed
            iface = dbus.Interface(obj, "org.vela.Assistant")
            if mid.startswith("ask:"):
                iface.Ask(mid[4:], self.token, timeout=10)
            elif mid.startswith("intent:"):
                iface.RunIntent(mid[7:], self.token, timeout=10)
        except dbus.DBusException as e:
            sys.stderr.write(f"vela runner: {e}\n")
        self.token = ""

    @dbus.service.method(IFACE, in_signature="", out_signature="")
    def Teardown(self):
        self._arm()


def main():
    DBusGMainLoop(set_as_default=True)
    bus = dbus.SessionBus()
    name = dbus.service.BusName(NAME, bus, do_not_queue=True)  # noqa: F841
    loop = GLib.MainLoop()
    Runner(bus, loop)
    loop.run()


if __name__ == "__main__":
    main()
