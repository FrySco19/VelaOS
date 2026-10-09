"""Access to the shared AI configuration (contract: components/settings-store/AI_CONFIG.md).

Everything goes through vela_ai_config (installed by settings-store at /usr/lib/vela/python). When running from
the source tree, the module is picked up from the sibling settings-store component.
"""
import os
import sys

_HERE = os.path.dirname(os.path.abspath(__file__))
for _p in ("/usr/lib/vela/python",
           os.path.abspath(os.path.join(_HERE, "../../../../../../../settings-store/overlay/usr/lib/vela/python"))):
    if os.path.isdir(_p) and _p not in sys.path:
        sys.path.append(_p)

import vela_ai_config as vac  # noqa: E402

APP = "assistant"


def load():
    return vac.load()


def endpoint(cfg=None, with_key=True):
    return vac.endpoint(cfg or vac.load(), with_key=with_key)


def may(cfg, perm):
    return vac.may(cfg, perm)


def log(action, summary, confirmed=None, result="ok", sent_off_machine=False):
    try:
        vac.log_action(app=APP, action=action, summary=summary, sent_off_machine=sent_off_machine,
                       confirmed=confirmed, result=result)
    except OSError:
        pass


def save_user(changes):
    return vac.save_user(changes)


def set_api_key(key):
    vac.set_api_key(key)


def get_api_key():
    return vac.get_api_key()


def data_dir():
    return vac.data_dir()


def memory_dir():
    return os.path.join(vac.memory_dir(), "assistant")


def config_file():
    return vac.user_conf()


def language(cfg=None):
    cfg = cfg or vac.load()
    lang = cfg.get("language", "auto")
    if lang in ("it", "en"):
        return lang
    env = (os.environ.get("LANGUAGE") or os.environ.get("LC_ALL") or os.environ.get("LC_MESSAGES")
           or os.environ.get("LANG") or "it")
    return "en" if env[:2] == "en" else "it"


def history_enabled(cfg):
    p = cfg.get("privacy", {})
    return bool(cfg.get("assistant", {}).get("history", True)) and p.get("memory_enabled", True) \
        and not p.get("memory_paused", False)
