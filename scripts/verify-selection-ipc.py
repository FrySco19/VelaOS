"""Build/boot the six-domain E2E image with the shared UserSelection IPC probe.

Run under WSL with Docker. First build velaos-executor-e2e using the existing
E2E runner. The input seed is a TEST fixture embedded only in its sender ELF.
"""
import argparse
import hashlib
import json
import pathlib
import re
import subprocess

parser = argparse.ArgumentParser()
parser.add_argument("--input-seed", required=True)
parser.add_argument("--out", required=True)
args = parser.parse_args()
root = pathlib.Path(__file__).resolve().parents[1]
out = pathlib.Path(args.out).resolve()
out.mkdir(parents=True, exist_ok=True)
build_command = ["docker", "build", "--progress=plain", "--secret",
    "id=input_seed,src=" + str(pathlib.Path(args.input_seed).resolve()),
    "-f", "poc/m5_executor/Dockerfile.selection-probe", "-t", "velaos-selection-ipc", "."]
with (out / "build.log").open("w") as log:
    build = subprocess.run(build_command, cwd=root, stdout=log, stderr=subprocess.STDOUT, timeout=1800)
if build.returncode:
    raise SystemExit("FAIL build; inspect " + str(out / "build.log"))
print("PASS: Microkit six-domain selection image built", flush=True)
command = ["docker", "run", "--rm", "--entrypoint", "timeout", "velaos-selection-ipc",
    "30s", "qemu-system-aarch64", "-machine", "virt,virtualization=on", "-cpu", "cortex-a53",
    "-m", "2G", "-nic", "none", "-nographic", "-device",
    "loader,file=build/selection-loader.img,addr=0x70000000,cpu-num=0"]
run = subprocess.run(command, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=90)
log = run.stdout.decode(errors="replace")
(out / "boot.log").write_text(log)
if run.returncode != 124 or any(term in log for term in ("panicked", "FAULT", "unexpected fixture denial")):
    raise SystemExit("FAIL boot; inspect " + str(out / "boot.log"))
source = (root / "poc/m5_executor/keygen/src/bin/verify-bridge-e2e.rs").read_text()
existing = re.findall(r'"([^"\n]+)"', source.split("const MARKERS:")[1].split("const FORBIDDEN:")[0])
selection = [
    "SELECTION PCB: ready; no_std shared verifier ed25519-dalek",
    "SELECTION INPUT: publish signed valid UserSelection",
    "SELECTION PCB: VALID Ed25519; emitted bounded TEST authorization",
    "SELECTION EXECUTOR SINK: TEST authorization received once; hash verified",
    "SELECTION INPUT: publish bad signature",
    "SELECTION PCB: REJECT bad signature; authority=0",
    "SELECTION INPUT: publish altered hash",
    "SELECTION PCB: REJECT altered hash; authority=0",
    "SELECTION PROBE PASS: valid approval, bad signature and altered hash over IPC",
]
for marker in existing + selection:
    if log.count(marker) != 1:
        raise SystemExit("FAIL missing/duplicate marker: " + marker)
if "LFS committed epoch=1 request=13" in log or "LFS refused request=13" in log:
    raise SystemExit("FAIL expired grant reached storage")
container = subprocess.check_output(["docker", "create", "velaos-selection-ipc"], text=True).strip()
if not re.fullmatch(r"[0-9a-f]{64}", container):
    raise SystemExit("Unexpected export container ID")
try:
    subprocess.run(["docker", "cp", container + ":/work/build/selection-loader.img", str(out / "selection-loader.img")], check=True)
finally:
    subprocess.run(["docker", "rm", container], check=True, stdout=subprocess.DEVNULL)
image_hash = hashlib.sha256((out / "selection-loader.img").read_bytes()).hexdigest()
summary = {"pass": True, "baseline_markers": len(existing), "selection_markers": len(selection),
    "loader_sha256": image_hash,
    "qemu_exit": run.returncode, "scope": "Actual shared-buffer IPC / no_std Dalek verification; test sender and token, not human input or production Biscuit issuance"}
(out / "result.json").write_text(json.dumps(summary, indent=2))
for line in log.splitlines():
    if "SELECTION" in line or "Booting" in line or "ALL E2E" in line:
        print(line)
print("PASS: " + str(len(existing)) + " baseline markers + " + str(len(selection)) + " selection IPC markers", flush=True)
