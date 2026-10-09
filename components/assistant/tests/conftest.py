import os
import sys
import tempfile

_TMP = tempfile.mkdtemp(prefix="vela-asst-test-")
os.environ["HOME"] = os.path.join(_TMP, "home")
os.environ["XDG_CONFIG_HOME"] = os.path.join(_TMP, "home", ".config")
os.environ["XDG_DATA_HOME"] = os.path.join(_TMP, "home", ".local", "share")
os.environ.pop("VELA_AI_PROVIDER", None)
os.environ.pop("VELA_AI_BASE_URL", None)
os.environ.pop("VELA_AI_API_KEY", None)
os.environ["LANG"] = "it_IT.UTF-8"
os.makedirs(os.environ["HOME"], exist_ok=True)
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.abspath(os.path.join(HERE, "../overlay/usr/lib/vela/assistant")))
sys.path.insert(0, HERE)
