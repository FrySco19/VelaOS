#!/usr/bin/env python3
"""Host-side check of the M5 seL4/Microkit platform pins (no guest build).

Asserts that Makefile, Dockerfile, rust-toolchain, Cargo manifest, the
Microkit system description and docs/CONTRACTS_v0.md (C6) agree on the
reference target. It proves configuration consistency only: it does not
build, boot or verify the guest (use keygen's verify-bridge for that).
"""
import re
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent

EXPECTED = {
    "board": "qemu_virt_aarch64",
    "config": "debug",
    "microkit": "2.3.0",
    "microkit_sha256": "e12e507f72c87cbf5c514df9e9b0c66103b42298852f44c045d5729ea3de4f89",
    "rust_toolchain": "nightly-2025-10-20",
    "rust_sel4_rev": "d441a880938209f661201704dfbee7b91c5aa905",
    "wasmi": "=2.0.0",
    "ed25519_compact": "=2.6.0",
    "target_triple": "aarch64-sel4-microkit",
}

failures = []


def check(cond, msg):
    print(("ok   " if cond else "FAIL ") + msg)
    if not cond:
        failures.append(msg)


def text(path):
    return path.read_text(encoding="utf-8")


def make_var(src, name):
    m = re.search(rf"^{name}\s*\?=\s*(\S+)\s*$", src, re.M)
    return m.group(1) if m else None


def docker_arg(src, name):
    m = re.search(rf"^ARG {name}=(\S+)\s*$", src, re.M)
    return m.group(1) if m else None


def wx_checked(src, target):
    """Require the check in this ELF's recipe, not an unrelated rule."""
    rule = re.search(rf"^{re.escape(target)}:[^\n]*\n((?:\t[^\n]*(?:\n|$))*)", src, re.M)
    return rule is not None and "python3 check-wx.py $@" in rule.group(1)


def main():
    mk = text(HERE / "Makefile")
    check(make_var(mk, "BOARD") == EXPECTED["board"], f"Makefile BOARD == {EXPECTED['board']}")
    check(make_var(mk, "CONFIG") == EXPECTED["config"], f"Makefile CONFIG == {EXPECTED['config']}")
    check(f"TARGET_TRIPLE := {EXPECTED['target_triple']}" in mk, "Makefile TARGET_TRIPLE pinned")
    check("--locked" in mk, "Makefile cargo builds use --locked")
    for target in ("$(EXECUTOR_ELF)", "$(PCB_ELF)", "$(BUILD)/lfs.elf"):
        check(wx_checked(mk, target), f"Makefile runs W^X check on {target}")
    run = mk.split("run:", 1)[1] if "run:" in mk else ""
    check("-nic none" in run, "QEMU run has no network (-nic none)")
    check("-machine virt,virtualization=on" in run and "-cpu cortex-a53" in run,
          "QEMU run uses virt + cortex-a53")

    df = text(HERE / "Dockerfile")
    check(docker_arg(df, "MICROKIT_VERSION") == EXPECTED["microkit"], f"Dockerfile Microkit == {EXPECTED['microkit']}")
    check(docker_arg(df, "MICROKIT_SDK_SHA256") == EXPECTED["microkit_sha256"], "Dockerfile Microkit SDK SHA-256 pinned")
    check("sha256sum -c" in df, "Dockerfile verifies SDK checksum")
    check(docker_arg(df, "RUST_TOOLCHAIN") == EXPECTED["rust_toolchain"], f"Dockerfile toolchain == {EXPECTED['rust_toolchain']}")
    check("type=secret,id=pcb_seed" in df, "Dockerfile takes PCB seed only as BuildKit secret")

    tc = text(HERE / "app" / "rust-toolchain.toml")
    check(f'channel = "{EXPECTED["rust_toolchain"]}"' in tc, "app/rust-toolchain.toml matches Dockerfile toolchain")

    cargo = text(HERE / "app" / "Cargo.toml")
    check(EXPECTED["rust_sel4_rev"] in cargo, "rust-sel4 git rev pinned")
    check(re.search(r'wasmi\s*=\s*\{\s*version\s*=\s*"=2\.0\.0"', cargo) is not None, "wasmi pinned =2.0.0")
    check(re.search(r'ed25519-compact\s*=\s*\{\s*version\s*=\s*"=2\.6\.0"', cargo) is not None, "ed25519-compact pinned =2.6.0")
    check("biscuit-auth" not in cargo, "no biscuit-auth in no_std guest (C3)")
    check((HERE / "app" / "Cargo.lock").is_file(), "app/Cargo.lock present")

    sysd = ET.parse(HERE / "executor.system").getroot()
    mrs = {m.get("name"): m.get("size") for m in sysd.findall("memory_region")}
    check(mrs == {"pcb_executor_page": "0x1_000"}, "exactly one shared region: 4 KiB pcb_executor_page")
    pds = {p.get("name"): p for p in sysd.findall("protection_domain")}
    check(set(pds) == {"pcb", "executor"}, "exactly two protection domains: pcb, executor")
    if set(pds) == {"pcb", "executor"}:
        def perms(pd):
            return [m.get("perms") for m in pds[pd].findall("map")]
        check(perms("pcb") == ["rw"], "pcb maps shared page rw (sole writer, OWN-v2)")
        check(perms("executor") == ["r"], "executor maps shared page r only (OWN-v2)")
        check(all("x" not in (p or "") for pd in pds for p in perms(pd)), "no executable mapping of shared page")
        check(int(pds["executor"].get("priority")) > int(pds["pcb"].get("priority")),
              "executor priority > pcb (protected-call server)")
    chans = sysd.findall("channel")
    check(len(chans) == 1, "exactly one channel")
    if len(chans) == 1:
        ends = {e.get("pd"): e for e in chans[0].findall("end")}
        check(ends.get("pcb") is not None and ends["pcb"].get("pp") == "true", "pcb end has pp=true (protected call)")
        check(ends.get("executor") is not None and ends["executor"].get("pp") in (None, "false"),
              "executor cannot ppcall pcb")
        check(ends.get("executor") is not None and ends["executor"].get("notify") == "false",
              "executor end notify=false (no notification ingress)")

    contracts = ROOT / "docs" / "CONTRACTS_v0.md"
    if contracts.is_file():
        c = text(contracts)
        for key in ("microkit", "rust_toolchain"):
            check(EXPECTED[key] in c, f"CONTRACTS_v0 C6 mentions {EXPECTED[key]}")
        check(EXPECTED["rust_sel4_rev"][:7] in c, "CONTRACTS_v0 C6 mentions rust-sel4 rev")
    else:
        check(False, "docs/CONTRACTS_v0.md present")

    if failures:
        print(f"\n{len(failures)} pin check(s) FAILED")
        return 1
    print("\nAll platform pin checks passed (configuration only; no boot performed).")
    return 0


if __name__ == "__main__":
    sys.exit(main())
