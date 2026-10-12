"""Local-only Microkit two-cold-boot proof. Dedicated test image, no physical disk.
Run inside the Docker-capable development host with prepared signed E2E bundle.
"""
import argparse
import hashlib
import json
import pathlib
import re
import subprocess

p = argparse.ArgumentParser()
p.add_argument("--pcb-seed", required=True)
p.add_argument("--lfs-seed", required=True)
p.add_argument("--out", required=True)
p.add_argument("--cache-image", help="Optional local image holding the same pinned SDK and cached build objects")
a = p.parse_args()
root = pathlib.Path(__file__).resolve().parents[1]
out = pathlib.Path(a.out).resolve()
out.mkdir(parents=True, exist_ok=True)
disk = out / "notes.img"
if disk.exists():
    raise SystemExit("Refusing to replace existing test disk; use a fresh output directory")
disk.write_bytes(bytes(4096))
build_args = [
    "docker", "build", "--secret", "id=pcb_seed,src=" + str(pathlib.Path(a.pcb_seed).resolve()),
    "--secret", "id=lfs_seed,src=" + str(pathlib.Path(a.lfs_seed).resolve()),
    "-f", "poc/m5_executor/Dockerfile.hardware", "-t", "velaos-lfs-hardware", ".",
]
if a.cache_image:
    build_args[2:2] = ["--build-arg", "HARDWARE_BASE=" + a.cache_image]
build = subprocess.run(build_args, cwd=root, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
(out / "build.log").write_bytes(build.stdout)
if build.returncode:
    raise SystemExit("Build failed; inspect " + str(out / "build.log"))
source = (root / "poc/m5_executor/keygen/src/bin/verify-bridge-e2e.rs").read_text()
markers = re.findall(r'"([^"\n]+)"', source.split("const MARKERS:")[1].split("const FORBIDDEN:")[0])
markers = [m for m in markers if not m.startswith("LFS writer ready")]
markers.append("LFS writer ready (VirtIO durable bounded ledger, capacity=8)")
markers += ["VirtIO modern queue ready; FLUSH negotiated", "blank dedicated disk; no prior commit",
            "data+signed history flushed generation=1; receipt release allowed", "note bytes=26 stored on block backend"]

logs = []
for boot in (1, 2):
    run = subprocess.run([
        "docker", "run", "--rm", "-v", str(out) + ":/disk", "--entrypoint", "timeout",
        "velaos-lfs-hardware", "30s", "qemu-system-aarch64", "-machine", "virt,virtualization=on",
        "-cpu", "cortex-a53", "-m", "2G", "-nic", "none", "-nographic",
        "-global", "virtio-mmio.force-legacy=false",
        "-drive", "file=/disk/notes.img,format=raw,if=none,id=notes,cache=writeback",
        "-device", "virtio-blk-device,drive=notes",
        "-device", "loader,file=build-hardware/loader.img,addr=0x70000000,cpu-num=0",
    ], stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    log = run.stdout.decode(errors="replace")
    (out / f"boot-{boot}.log").write_text(log)
    if run.returncode != 124 or "panicked" in log or "FAULT" in log:
        raise SystemExit(f"Boot {boot} failed; inspect serial log")
    required = markers if boot == 1 else [
        "cold boot recovered signed note bytes=26 receipts=1 generation=1",
        "recovered note verified: appunti: design bridge IPC",
        "PCB receipt verified after cold boot replayed=1",
        "PCB notary verified+stored receipt request=12 value=21 seq=1",
        "ALL E2E CHECKS PASSED; intent -> selection -> grant -> Wasm 21 -> receipt notarized",
    ]
    for marker in required:
        if log.count(marker) != 1:
            raise SystemExit(f"Boot {boot}: missing/duplicate marker: {marker}")
    if "LFS committed epoch=1 request=13" in log or "LFS refused request=13" in log:
        raise SystemExit("Expired grant reached storage")
    logs.append({"boot": boot, "markers": len(required), "sha256": hashlib.sha256(disk.read_bytes()).hexdigest()})
    print(f"PASS boot {boot}: {len(required)} real serial markers")
if logs[0]["sha256"] != logs[1]["sha256"]:
    raise SystemExit("Second boot changed disk during idempotent retry")
(out / "result.json").write_text(json.dumps({"boots": logs, "scope": "QEMU block persistence; host-generated selection/Biscuit fixture; not Glass"}, indent=2))
print("PASS: two separate QEMU processes, identical disk state, signed note and receipt recovered")

