#!/bin/sh
# Apply component "desktop" to the RUNNING dev VM (tools/vm.sh -n dev) for fast iteration.
#   components/desktop/deploy.sh [--reset-layout]
# --reset-layout: throw away the user's Plasma panel/desktop config and let plasmashell
#                 re-run the layout (as on a first login). Use inside the devvm lock:
#   flock /workspace/vela/v2/coordination/devvm.lock components/desktop/deploy.sh
set -eu
HERE=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
VM="$HERE/../../tools/vm.sh"
TAR=/tmp/vela-desktop-overlay.$$.tar
tar -C "$HERE/overlay" -cf "$TAR" .
"$VM" -n dev scp-to "$TAR" /tmp/vela-desktop-overlay.tar
"$VM" -n dev scp-to "$HERE/hook.chroot.sh" /tmp/vela-desktop-hook.sh
rm -f "$TAR"
"$VM" -n dev ssh "sudo tar -C / --no-same-owner -xf /tmp/vela-desktop-overlay.tar && sudo sh /tmp/vela-desktop-hook.sh"
# per-user bits that normally come from /etc/skel at user creation
"$VM" -n dev ssh 'mkdir -p ~/.local/share/kxmlgui5/dolphin && cp /etc/skel/.local/share/kxmlgui5/dolphin/dolphinui.rc ~/.local/share/kxmlgui5/dolphin/ &&
  export XDG_RUNTIME_DIR=/run/user/$(id -u) DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/$(id -u)/bus;
  systemctl --user daemon-reload; systemctl --user start vela-desktop-apps.path vela-desktop-drives.path vela-desktop-session.service || true;
  kbuildsycoca6 >/dev/null 2>&1 || true'
if [ "${1:-}" = "--reset-layout" ]; then
  "$VM" -n dev ssh 'export XDG_RUNTIME_DIR=/run/user/$(id -u) DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/$(id -u)/bus;
    systemctl --user stop plasma-plasmashell.service;
    mv ~/.config/plasma-org.kde.plasma.desktop-appletsrc ~/.config/plasma-org.kde.plasma.desktop-appletsrc.bak 2>/dev/null || true;
    mv ~/.config/plasmashellrc ~/.config/plasmashellrc.bak 2>/dev/null || true;
    cp /etc/skel/.config/plasma-org.kde.plasma.desktop-appletsrc ~/.config/ 2>/dev/null || true;
    systemctl --user start plasma-plasmashell.service;
    qdbus6 org.kde.KWin /KWin reconfigure || true'
fi
echo "desktop: deployed to dev VM"
