#!/bin/sh
# settings-store hook (Vela 2): Settings branding, Vela KCMs, Store (Discover + Flathub), update policy.
# Runs as root inside the chroot. Idempotent. No network needed.
set -e

APPS=/usr/share/applications
LOCALAPPS=/usr/local/share/applications
mkdir -p "$LOCALAPPS"

# Rewrite a .desktop file: drop every localized Name/GenericName/Comment/Keywords, then add ours.
# Placed in /usr/local/share/applications, which shadows /usr/share/applications with the same id
# and survives package upgrades.
override_desktop() { # src dst name name_it generic generic_it comment comment_it keywords keywords_it
  [ -f "$1" ] || { echo "settings-store: missing $1, skip"; return 0; }
  awk -v n="$3" -v nit="$4" -v g="$5" -v git="$6" -v c="$7" -v cit="$8" -v k="$9" -v kit="${10}" '
    BEGIN { main=0 }
    /^\[/ { if (main==1) { main=2 } ; if ($0=="[Desktop Entry]") { main=1; print; print "Name=" n; print "Name[it]=" nit;
            print "GenericName=" g; print "GenericName[it]=" git; print "Comment=" c; print "Comment[it]=" cit;
            print "Keywords=" k; print "Keywords[it]=" kit; next } }
    main==1 && /^(Name|GenericName|Comment|Keywords)(\[[^]]*\])?=/ { next }
    { print }' "$1" > "$2.tmp"
  mv "$2.tmp" "$2"
  chmod 644 "$2"
}

override_desktop "$APPS/systemsettings.desktop" "$LOCALAPPS/systemsettings.desktop" \
  "Settings" "Impostazioni" "System Settings" "Impostazioni di sistema" \
  "Wi-Fi, Bluetooth, sound, displays, appearance, users and more" \
  "Wi-Fi, Bluetooth, audio, schermi, aspetto, utenti e altro" \
  "settings;preferences;control panel;system;configuration;" \
  "impostazioni;preferenze;pannello di controllo;sistema;configurazione;opzioni;"

override_desktop "$APPS/org.kde.discover.desktop" "$LOCALAPPS/org.kde.discover.desktop" \
  "Store" "Store" "Vela Store" "Vela Store" \
  "Install apps and updates" "Installa app e aggiornamenti" \
  "store;app store;software;install;apps;updates;flatpak;flathub;discover;" \
  "store;negozio;app;applicazioni;programmi;installa;aggiornamenti;software;flatpak;flathub;discover;"

# The Store keeps its "See available updates" action; make it Italian-friendly
if [ -f "$LOCALAPPS/org.kde.discover.desktop" ]; then
  sed -i 's/^Name\[it\]=Vedi gli aggiornamenti disponibili$/Name[it]=Mostra aggiornamenti/' "$LOCALAPPS/org.kde.discover.desktop"
fi

# --- Discover: prefer Flathub for new apps, Debian (PackageKit) still handles system updates ---
if command -v kwriteconfig6 >/dev/null 2>&1; then
  kwriteconfig6 --file /etc/xdg/discoverrc --group ResourcesModel --key currentApplicationBackend flatpak-backend
  # Update notifications: at most once a week, never install on their own (security updates are
  # handled by unattended-upgrades); offline updates are applied on restart.
  kwriteconfig6 --file /etc/xdg/PlasmaDiscoverUpdates --group Global --key RequiredNotificationInterval 604800
  kwriteconfig6 --file /etc/xdg/PlasmaDiscoverUpdates --group Global --key UseUnattendedUpdates false
  kwriteconfig6 --file /etc/xdg/PlasmaDiscoverUpdates --group Global --key UseOfflineUpdates true
else
  echo "settings-store: kwriteconfig6 not found (libkf6config-bin) — Discover defaults not set" >&2
fi

# --- Discover notifier: first check 10 min after login, never in the live session ---
NOTIF=/etc/xdg/autostart/org.kde.discover.notifier.desktop
if [ -f "$NOTIF" ]; then
  # conffile: editing it is allowed and kept on upgrade
  sed -i 's|^Exec=.*DiscoverNotifier.*$|Exec=sh -c "[ -d /run/live/medium ] \|\| exec /usr/lib/x86_64-linux-gnu/libexec/DiscoverNotifier --check-delay 600"|' "$NOTIF"
fi

# --- Flathub system remote (file shipped by base in /etc/flatpak/remotes.d; add it explicitly too) ---
if command -v flatpak >/dev/null 2>&1; then
  for f in /etc/flatpak/remotes.d/flathub.flatpakrepo; do
    [ -f "$f" ] && flatpak remote-add --system --if-not-exists flathub "$f" >/dev/null 2>&1 || true
  done
  # Exports dirs exist from the start so KDE's menu cache (kbuildsycoca) watches them and newly
  # installed Flatpak apps appear in the launcher immediately.
  mkdir -p /var/lib/flatpak/exports/share/applications /var/lib/flatpak/exports/share/icons/hicolor
  mkdir -p /etc/skel/.local/share/flatpak/exports/share/applications
fi

# --- unattended-upgrades: our 20vela-auto-upgrades enables it; drop Debian's debconf-generated file
#     if present so there is one source of truth ---
if [ -f /etc/apt/apt.conf.d/20auto-upgrades ] && [ -f /etc/apt/apt.conf.d/20vela-auto-upgrades ]; then
  rm -f /etc/apt/apt.conf.d/20auto-upgrades
fi

# --- permissions of shipped files ---
chmod 755 /usr/bin/vela-ai-config /usr/lib/vela/settings/vela-updates-helper 2>/dev/null || true
chmod 644 /etc/vela/ai.conf

# --- menu cache ---
command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database -q "$LOCALAPPS" "$APPS" || true
exit 0
