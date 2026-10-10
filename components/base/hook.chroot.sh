#!/bin/sh
# Vela 2 base hook  — runs as root in the chroot. Idempotent.
set -e

# --- Locale / keyboard / timezone: Italian default, English available ---
for l in it_IT.UTF-8 en_US.UTF-8 en_GB.UTF-8; do
  sed -i "s/^# *\($l UTF-8\)/\1/" /etc/locale.gen
  grep -q "^$l UTF-8" /etc/locale.gen || echo "$l UTF-8" >> /etc/locale.gen
done
locale-gen >/dev/null
printf 'LANG=it_IT.UTF-8\nLANGUAGE=it:en\n' > /etc/default/locale
cat > /etc/default/keyboard <<'EOT'
XKBMODEL="pc105"
XKBLAYOUT="it"
XKBVARIANT=""
XKBOPTIONS=""
BACKSPACE="guess"
EOT
ln -sf /usr/share/zoneinfo/Europe/Rome /etc/localtime
echo Europe/Rome > /etc/timezone

# --- Identity: Vela on top of Debian 13 (ID stays debian so apt/tools keep working) ---
if [ ! -e /usr/lib/os-release.debian ]; then
  dpkg-divert --local --rename --divert /usr/lib/os-release.debian --add /usr/lib/os-release
fi
VER=$(. /usr/lib/os-release.debian && echo "$VERSION_ID")
cat > /usr/lib/os-release <<EOT
PRETTY_NAME="Vela OS 2.1"
NAME="Vela OS"
VERSION_ID="$VER"
VERSION="2.1 (Debian $VER trixie)"
VELA_VERSION="2.1-local-preview"
VERSION_CODENAME=trixie
DEBIAN_VERSION_FULL=$(cat /etc/debian_version)
ID=debian
VARIANT="Vela"
VARIANT_ID=vela
LOGO=vela-logo
HOME_URL="https://github.com/FrySco19/VelaOS"
SUPPORT_URL="https://github.com/FrySco19/VelaOS/discussions"
BUG_REPORT_URL="https://github.com/FrySco19/VelaOS/issues"
EOT
ln -sf ../usr/lib/os-release /etc/os-release
printf 'Vela OS 2.1 \\n \\l\n\n' > /etc/issue
echo 'Vela OS 2.1' > /etc/issue.net

# --- Services ---
systemctl set-default graphical.target
for s in sddm NetworkManager bluetooth avahi-daemon; do systemctl enable "$s" 2>/dev/null || true; done
# Wait-online slows boot and is useless on a desktop
systemctl disable NetworkManager-wait-online.service 2>/dev/null || true
# NetworkManager manages all interfaces (no ifupdown)
if [ -f /etc/NetworkManager/NetworkManager.conf ]; then
  sed -i 's/^managed=false/managed=true/' /etc/NetworkManager/NetworkManager.conf
fi

# --- Flatpak + Flathub (static remote file in /etc/flatpak/remotes.d is picked up automatically) ---
# Also add it system-wide when possible so Discover shows Flathub immediately.
if command -v flatpak >/dev/null; then
  flatpak remote-add --system --if-not-exists flathub /etc/flatpak/remotes.d/flathub.flatpakrepo 2>/dev/null || true
fi

# --- Calamares: hide Debian's launcher and desktop-icon autostart (Vela has its own) ---
rm -f /etc/xdg/autostart/calamares-desktop-icon.desktop
if [ -f /usr/share/applications/calamares-install-debian.desktop ]; then
  grep -q '^NoDisplay=true' /usr/share/applications/calamares-install-debian.desktop || \
    sed -i 's/^\[Desktop Entry\]/[Desktop Entry]\nNoDisplay=true/' /usr/share/applications/calamares-install-debian.desktop
fi
chmod 755 /usr/lib/vela/base/postinstall.sh /usr/bin/vela-install /usr/lib/live/config/1185-vela-live 2>/dev/null || true

# --- sudo for live user is configured by live-config; installed users are in group sudo ---
# --- VirtualBox guest services (fasttrack 7.2.x) ---
systemctl enable virtualbox-guest-utils.service 2>/dev/null || true

# --- Keep only the newest kernel (live-build/live-boot use /live/vmlinuz = newest; avoids
#     a kernel without matching modules and saves ~100 MB) ---
latest=$(ls /boot/vmlinuz-* 2>/dev/null | sed 's|/boot/vmlinuz-||' | sort -V | tail -1)
if [ -n "$latest" ]; then
  for k in $(dpkg-query -W -f '${Package}\n' 'linux-image-[0-9]*' 2>/dev/null); do
    [ "$k" = "linux-image-$latest" ] || DEBIAN_FRONTEND=noninteractive apt-get purge -y -q "$k" >/dev/null || true
  done
fi

# --- Small cleanups ---
apt-get clean
