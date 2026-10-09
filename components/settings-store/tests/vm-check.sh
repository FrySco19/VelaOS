#!/bin/sh
# Run INSIDE the dev VM as user vela (tools/vm.sh -n dev ssh 'sh -s' < this). Prints PASS/FAIL lines.
ok() { echo "PASS $*"; }; ko() { echo "FAIL $*"; }
chk() { d=$1; shift; if "$@" >/dev/null 2>&1; then ok "$d"; else ko "$d"; fi; }
export LANG=it_IT.UTF-8 LC_ALL=it_IT.UTF-8
P=/usr/lib/x86_64-linux-gnu/qt6/plugins/plasma/kcms
echo "== KCM plugins present"
for k in kcm_networkmanagement kcm_bluetooth kcm_pulseaudio kcm_kscreen kcm_nightlight kcm_colors kcm_lookandfeel kcm_wallpaper \
  kcm_notifications kcm_users kcm_regionandlang kcm_keyboard kcm_keys kcm_mouse kcm_touchpad kcm_powerdevilprofilesconfig \
  kcm_printer_manager kcm_clock kcm_componentchooser kcm_autostart kcm_baloofile kcm_access kcm_screenlocker kcm_sddm \
  kcm_updates kcm_flatpak kcm_firewall kcm_disks kcm_about-distro kcm_vela_ai kcm_vela_privacy kcm_vela_aspetto kcm_vela_updates; do
  f=$(find $P -name "$k.so" 2>/dev/null | head -1); [ -n "$f" ] && ok "kcm $k" || ko "kcm $k missing"
done
echo "== branding"
chk "os-release PRETTY_NAME Vela OS 2" grep -q 'PRETTY_NAME="Vela OS 2"' /etc/os-release
chk "Settings named Impostazioni" grep -q '^Name\[it\]=Impostazioni$' /usr/local/share/applications/systemsettings.desktop
chk "Store named Store" grep -q '^Name\[it\]=Store$' /usr/local/share/applications/org.kde.discover.desktop
chk "category file" test -f /usr/share/systemsettings/categories/settings-vela-ai.desktop
chk "featured json" python3 -c "import json;assert len(json.load(open('/usr/share/vela/store/featured.json')))>20"
chk "featuredurlrc" grep -q FeaturedListingURL /usr/share/discover/featuredurlrc
chk "discover prefers flatpak" grep -q flatpak-backend /etc/xdg/discoverrc
chk "notifier delayed" grep -q 'check-delay 600' /etc/xdg/autostart/org.kde.discover.notifier.desktop
echo "== store backends"
chk "flathub remote" sh -c 'flatpak remotes --system | grep -q flathub'
for b in flatpak-backend packagekit-backend fwupd-backend; do chk "discover $b" test -f /usr/lib/x86_64-linux-gnu/qt6/plugins/discover/$b.so; done
chk "packagekitd" sh -c 'pkcon get-roles >/dev/null'
echo "== updates"
chk "unattended-upgrades installed" test -x /usr/bin/unattended-upgrade
chk "auto security on" grep -q 'Unattended-Upgrade "1"' /etc/apt/apt.conf.d/20vela-auto-upgrades
chk "apt-config effective" sh -c 'apt-config dump | grep -q "APT::Periodic::Unattended-Upgrade \"1\""'
chk "polkit action" test -f /usr/share/polkit-1/actions/org.vela.settings.updates.policy
chk "helper exec" test -x /usr/lib/vela/settings/vela-updates-helper
chk "snapper root config" test -f /etc/snapper/configs/root
echo "== AI config"
chk "vela-ai-config json" vela-ai-config get provider
chk "it translations" test -f /usr/share/locale/it/LC_MESSAGES/kcm_vela_ai.mo
echo "== flatpak exports in XDG_DATA_DIRS of session"
pid=$(pgrep -u vela plasmashell | head -1); [ -n "$pid" ] && tr '\0' '\n' < /proc/$pid/environ | grep '^XDG_DATA_DIRS' | grep -q flatpak && ok "plasmashell XDG_DATA_DIRS has flatpak" || ko "plasmashell XDG_DATA_DIRS lacks flatpak"
