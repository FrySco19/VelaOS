"""Regression: an extra ELF must not hide an unchecked protection domain."""
import importlib.util
from pathlib import Path
import unittest

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("platform_pins", HERE / "check-pins.py")
pins = importlib.util.module_from_spec(spec)
spec.loader.exec_module(pins)


class WxRecipes(unittest.TestCase):
    def test_each_domain_requires_its_own_check(self):
        makefile = (HERE / "Makefile").read_text(encoding="utf-8")
        for target in ("$(EXECUTOR_ELF)", "$(PCB_ELF)", "$(BUILD)/lfs.elf"):
            with self.subTest(target=target):
                self.assertTrue(pins.wx_checked(makefile, target))
                rule_start = makefile.index(target + ":")
                check_at = makefile.index("python3 check-wx.py $@", rule_start)
                damaged = makefile[:check_at] + makefile[check_at:].replace("python3 check-wx.py $@", "echo unchecked", 1)
                self.assertFalse(pins.wx_checked(damaged, target))


if __name__ == "__main__":
    unittest.main()
