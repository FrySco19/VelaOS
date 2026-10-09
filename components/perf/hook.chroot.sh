#!/bin/sh
# Vela perf — build-time hook (runs as root in the chroot). Idempotent.
set -e

# 1. our helpers are executable
chmod 0755 /usr/lib/vela/perf/* 2>/dev/null || true

# 2. boot-time GPU detection + login warm-up timer
systemctl enable vela-gpu-detect.service || echo "perf: WARN vela-gpu-detect.service not found (overlay not applied?)"
systemctl --global enable vela-warmup.timer || echo "perf: WARN vela-warmup.timer not found"

# 3. Debian's Guest Additions X11 launcher is replaced by /etc/xdg/autostart/vela-vboxclient.desktop
#    (works in Wayland and X11; avoids two clipboard clients in the X11 session)
if ! dpkg-divert --list /etc/X11/Xsession.d/98vboxadd-xclient | grep -q vela; then
  dpkg-divert --package vela-perf --rename --divert /etc/X11/Xsession.d/98vboxadd-xclient.vela-disabled \
    --add /etc/X11/Xsession.d/98vboxadd-xclient
fi

# 4. services that only slow boot/login down on a desktop
for u in NetworkManager-wait-online.service systemd-networkd-wait-online.service; do
  systemctl disable "$u" >/dev/null 2>&1 || true
  systemctl mask "$u" >/dev/null 2>&1 || true
done
# Discover/PackageKit handles updates with the user's consent; apt's own daily jobs would compete for disk at boot
for u in apt-daily.timer apt-daily-upgrade.timer; do
  systemctl disable "$u" >/dev/null 2>&1 || true
done
# zram replaces disk swap tools if any got pulled in
systemctl disable zramswap.service >/dev/null 2>&1 || true

# 5. Guest Additions system service on, in VirtualBox only (unit has ConditionVirtualization=oracle)
systemctl enable virtualbox-guest-utils.service >/dev/null 2>&1 || true

# 6. timesyncd conflicts with VBoxService time sync only inside VirtualBox (handled by the unit's Conflicts=),
#    nothing to do here.
exit 0
