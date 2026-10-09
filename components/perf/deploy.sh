#!/bin/sh
# Apply the perf component to a running dev/ISO VM over SSH (tools/vm.sh).  Usage: deploy.sh [vm-name]
set -eu
NAME=${1:-dev}
HERE=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
VM="$HERE/../../tools/vm.sh"
T=$(mktemp -d); trap 'rm -rf "$T"' EXIT
tar -C "$HERE" -czf "$T/perf.tgz" overlay hook.chroot.sh packages.list tools
"$VM" -n "$NAME" scp-to "$T/perf.tgz" /tmp/perf.tgz
"$VM" -n "$NAME" ssh 'set -e; S="sudo -n"; $S true 2>/dev/null || S="echo vela | sudo -S -p \"\""; 
  rm -rf /tmp/perf && mkdir -p /tmp/perf && tar -C /tmp/perf -xzf /tmp/perf.tgz
  pk=$(sed -e "s/#.*//" /tmp/perf/packages.list | xargs); missing=""
  for p in $pk; do dpkg -s "$p" >/dev/null 2>&1 || missing="$missing $p"; done
  [ -z "$missing" ] || { echo "installing:$missing"; eval "$S apt-get install -y -q $missing" >/dev/null || echo "WARN apt failed (offline?)"; }
  eval "$S cp -a /tmp/perf/overlay/. /"
  eval "$S sh /tmp/perf/hook.chroot.sh"
  eval "$S systemctl daemon-reload"; eval "$S sysctl --system" >/dev/null
  eval "$S systemctl start vela-gpu-detect.service" || true
  eval "$S udevadm trigger --subsystem-match=block --action=change" || true
  echo "deployed perf; reboot to activate zram/journald/system.conf changes"'
