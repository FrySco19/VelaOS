#!/bin/sh
# Vela post-install (Calamares shellprocess@vela, runs chrooted in the new system).
# Idempotent; never fails the install for optional steps.
set -u
log() { echo "vela-postinstall: $*"; }

# 1. Btrfs snapshots with snapper (root config, apt pre/post snapshots via snapper's apt hook)
if [ "$(findmnt -n -o FSTYPE /)" = btrfs ] && command -v snapper >/dev/null; then
  if [ ! -f /etc/snapper/configs/root ]; then
    snapper --no-dbus -c root create-config / && log "snapper root config created" || log "snapper create-config failed"
  fi
  if [ -f /etc/snapper/configs/root ]; then
    snapper --no-dbus -c root set-config \
      TIMELINE_CREATE=yes TIMELINE_CLEANUP=yes NUMBER_CLEANUP=yes \
      NUMBER_LIMIT=10 NUMBER_LIMIT_IMPORTANT=5 \
      TIMELINE_LIMIT_HOURLY=5 TIMELINE_LIMIT_DAILY=7 TIMELINE_LIMIT_WEEKLY=2 \
      TIMELINE_LIMIT_MONTHLY=0 TIMELINE_LIMIT_YEARLY=0 ALLOW_GROUPS=sudo SYNC_ACL=yes || true
    chmod a+rx /.snapshots 2>/dev/null || true
    systemctl enable snapper-timeline.timer snapper-cleanup.timer 2>/dev/null || true
    snapper --no-dbus -c root create -t single -c important -d "Installazione iniziale" || true
  fi
fi

# 2. Remove installer leftovers
rm -f /usr/share/applications/vela-install.desktop /usr/bin/vela-install
rm -f /etc/polkit-1/rules.d/49-vela-live.rules
for d in /home/*/Scrivania /home/*/Desktop; do rm -f "$d/vela-install.desktop" 2>/dev/null; done
rm -rf /etc/calamares
rm -f /etc/sddm.conf.d/zz-vela-live.conf

# 3. Bootloader: make sure our grub defaults are applied
if command -v update-grub >/dev/null; then update-grub >/dev/null 2>&1 || true; fi
log "done"
exit 0
