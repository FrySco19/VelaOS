"""Exercise real child-process cancellation, without desktop dependencies."""
import sys
import threading
import time

from core import sysutil


def test_cancel_running_process():
    cancel = threading.Event()
    outcome = []
    def worker():
        sysutil.execution.cancel = cancel
        outcome.append(sysutil.run([sys.executable, "-c", "import time; print('started', flush=True); time.sleep(30)"], timeout=40))
    thread = threading.Thread(target=worker)
    thread.start()
    time.sleep(0.2)
    cancel.set()
    thread.join(4)
    assert not thread.is_alive()
    assert outcome[0][0] == 130


def test_process_timeout_with_operation_event():
    sysutil.execution.cancel = threading.Event()
    try:
        rc, out, err = sysutil.run([sys.executable, "-c", "import time; time.sleep(30)"], timeout=0.15)
        assert rc == 124
    finally:
        del sysutil.execution.cancel


def test_cancel_before_process_has_no_side_effect(tmp_path):
    marker = tmp_path / "should-not-exist"
    cancel = threading.Event()
    cancel.set()
    sysutil.execution.cancel = cancel
    try:
        rc, _, _ = sysutil.run([sys.executable, "-c", "from pathlib import Path; import sys; Path(sys.argv[1]).touch()", str(marker)])
        assert rc == 130 and not marker.exists()
    finally:
        del sysutil.execution.cancel
