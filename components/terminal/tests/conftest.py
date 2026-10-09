import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
LIB = os.path.normpath(os.path.join(HERE, "..", "overlay", "usr", "lib", "vela", "terminal"))
if LIB not in sys.path:
    sys.path.insert(0, LIB)

PYLIB = os.path.normpath(os.path.join(HERE, "..", "overlay", "usr", "lib", "vela", "python"))
if PYLIB not in sys.path:
    sys.path.insert(0, PYLIB)
os.environ.setdefault("VELA_TERMINAL_LIB", LIB)
if HERE not in sys.path:
    sys.path.insert(0, HERE)
