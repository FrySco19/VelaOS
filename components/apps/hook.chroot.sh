#!/bin/sh
# apps component hook (runs as root in the chroot; idempotent).
set -e

# --- File search (Baloo): file-name indexing ON, content indexing OFF (fast); hidden folders are never indexed.
#     Users can enable content indexing in Impostazioni > Ricerca. Live session: off (vela-live-baloo-off.service).
kwriteconfig6 --file /etc/xdg/baloofilerc --group "Basic Settings" --key "Indexing-Enabled" true
kwriteconfig6 --file /etc/xdg/baloofilerc --group "General" --key "only basic indexing" true
kwriteconfig6 --file /etc/xdg/baloofilerc --group "General" --key "index hidden folders" false
systemctl enable vela-live-baloo-off.service >/dev/null 2>&1 || \
  ln -sf /etc/systemd/system/vela-live-baloo-off.service /etc/systemd/system/multi-user.target.wants/vela-live-baloo-off.service

# --- Plasma Browser Integration: Firefox already exposes media controls (MPRIS); the "install the extension"
#     reminder would be noise on first browser start.
for f in kded6rc kded5rc; do
  kwriteconfig6 --file /etc/xdg/$f --group "Module-browserintegrationreminder" --key "autoload" false
done

# --- Vela AI browser extension + native host: make sure permissions are sane whatever the overlay copy did.
chmod 0755 /usr/lib/vela/apps/vela_ai_browser_host.py /usr/lib/vela/apps/vela_spazi.py /usr/bin/vela-spazi /usr/bin/vela-istantanee
chmod -R a+rX "/usr/share/mozilla/extensions/{ec8030f7-c20a-464f-9b0e-13a3a9e97384}/vela-ai@vela.os" \
  /usr/lib/mozilla/native-messaging-hosts /etc/firefox /etc/firefox-esr

# --- Printing: CUPS on, browsing for network printers (cups-browsed is not installed: ipp-usb + IPP Everywhere).
systemctl enable cups.service >/dev/null 2>&1 || true

exit 0
