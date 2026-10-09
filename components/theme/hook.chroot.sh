#!/bin/sh
# theme: apply the Vela visual identity system-wide (runs as root in the chroot; idempotent).
# Only touches visual keys in shared /etc/xdg files, via kwriteconfig6 (see coordination/requests.md).
set -e

KW=$(command -v kwriteconfig6 || true)
if [ -z "$KW" ]; then
  echo "theme: kwriteconfig6 missing (libkf6config-bin)" >&2; exit 1
fi
kw() { # kw FILE GROUP[/SUBGROUP] KEY VALUE   (writes into /etc/xdg/FILE)
  f=$1; g=$2; k=$3; v=$4
  set -- --file "/etc/xdg/$f"
  oldifs=$IFS; IFS=/
  for part in $g; do set -- "$@" --group "$part"; done
  IFS=$oldifs
  "$KW" "$@" --key "$k" "$v"
}

# ------------------------------------------------------------ icon themes
sh /usr/lib/vela/theme/build-icon-themes.sh
command -v gtk-update-icon-cache >/dev/null && gtk-update-icon-cache -qf /usr/share/icons/hicolor || true

# ------------------------------------------------------------ fonts
ln -sf ../conf.avail/60-vela.conf /etc/fonts/conf.d/60-vela.conf
fc-cache -f >/dev/null 2>&1 || true

# ------------------------------------------------------------ kdeglobals (colors, fonts, icons, LNF)
UI="Inter,10,-1,5,400,0,0,0,0,0,0,0,0,0,0,1"
kw kdeglobals KDE LookAndFeelPackage org.vela.desktop
kw kdeglobals KDE widgetStyle Breeze
kw kdeglobals KDE AnimationDurationFactor 0.75
kw kdeglobals KDE ShowIconsInMenuItems false
kw kdeglobals KDE ShowIconsOnPushButtons false
kw kdeglobals General ColorScheme VelaLight
kw kdeglobals General font "$UI"
kw kdeglobals General menuFont "$UI"
kw kdeglobals General toolBarFont "Inter,9,-1,5,400,0,0,0,0,0,0,0,0,0,0,1"
kw kdeglobals General smallestReadableFont "Inter,8,-1,5,400,0,0,0,0,0,0,0,0,0,0,1"
kw kdeglobals General fixed "JetBrains Mono,10,-1,5,400,0,0,0,0,0,0,0,0,0,0,1"
kw kdeglobals WM activeFont "Inter,10,-1,5,600,0,0,0,0,0,0,0,0,0,0,1"
kw kdeglobals Icons Theme Vela
# Copy every color group of VelaLight into /etc/xdg/kdeglobals so apps get Vela colors before any
# user-level apply (KColorScheme reads the [Colors:*] groups, not just the scheme name).
awk '
  /^\[/ { grp=$0; sub(/^\[/,"",grp); sub(/\]$/,"",grp); keep = (grp ~ /^(Colors:|ColorEffects:|WM$)/); next }
  keep && /=/ { i=index($0,"="); print grp "\t" substr($0,1,i-1) "\t" substr($0,i+1) }
' /usr/share/color-schemes/VelaLight.colors | while IFS="$(printf '\t')" read -r grp key val; do
  g=$(echo "$grp" | sed 's/\]\[/\//g')
  kw kdeglobals "$g" "$key" "$val"
done

# ------------------------------------------------------------ Plasma style, splash, cursor, lock screen
kw plasmarc Theme name vela
kw ksplashrc KSplash Engine KSplashQML
kw ksplashrc KSplash Theme org.vela.desktop
kw kcminputrc Mouse cursorTheme Bibata-Modern-Classic
kw kcminputrc Mouse cursorSize 24
kw kscreenlockerrc Greeter WallpaperPlugin org.kde.image
kw kscreenlockerrc "Greeter/Wallpaper/org.kde.image/General" Image "file:///usr/share/wallpapers/Vela/"
kw kscreenlockerrc "Greeter/Wallpaper/org.kde.image/General" PreviewImage "file:///usr/share/wallpapers/Vela/contents/screenshot.jpg"
# GTK apps (Firefox, etc.): kde-gtk-config follows the color scheme; set sane defaults for first start
mkdir -p /etc/gtk-3.0 /etc/gtk-4.0
for v in 3.0 4.0; do
  f=/etc/gtk-$v/settings.ini
  [ -f "$f" ] && grep -q '^gtk-theme-name' "$f" && continue
  cat > "$f" <<EOT
[Settings]
gtk-theme-name=Breeze
gtk-icon-theme-name=Vela
gtk-cursor-theme-name=Bibata-Modern-Classic
gtk-cursor-theme-size=24
gtk-font-name=Inter 10
gtk-xft-antialias=1
gtk-xft-hinting=1
gtk-xft-hintstyle=hintslight
gtk-xft-rgba=none
EOT
done
# XCursor fallback for X11 apps (Debian alternative x-cursor-theme)
if [ -f /usr/share/icons/Bibata-Modern-Classic/cursor.theme ]; then
  update-alternatives --install /usr/share/icons/default/index.theme x-cursor-theme /usr/share/icons/Bibata-Modern-Classic/cursor.theme 50 >/dev/null 2>&1 || true
  update-alternatives --set x-cursor-theme /usr/share/icons/Bibata-Modern-Classic/cursor.theme >/dev/null 2>&1 || true
fi

# ------------------------------------------------------------ window decoration + effects (visual keys only)
kw kwinrc org.kde.kdecoration2 library org.kde.kwin.aurorae
kw kwinrc org.kde.kdecoration2 theme __aurorae__svg__VelaLight
kw kwinrc org.kde.kdecoration2 ButtonsOnLeft XIA
kw kwinrc org.kde.kdecoration2 ButtonsOnRight ""
kw kwinrc org.kde.kdecoration2 BorderSize None
kw kwinrc org.kde.kdecoration2 BorderSizeAuto false
kw kwinrc Plugins blurEnabled true
kw kwinrc Plugins contrastEnabled true
kw kwinrc Plugins wobblywindowsEnabled false
kw kwinrc Plugins magiclampEnabled true
kw kwinrc Plugins squashEnabled false
kw kwinrc Plugins scaleEnabled true
kw kwinrc Plugins glideEnabled false
kw kwinrc Plugins translucencyEnabled false
kw kwinrc Plugins slidingpopupsEnabled true
kw kwinrc Plugins fadingpopupsEnabled true
kw kwinrc Effect-blur BlurStrength 10
kw kwinrc Effect-blur NoiseStrength 3
kw kwinrc Effect-magiclamp AnimationDuration 220
kw kwinrc Effect-scale InScale 0.92
kw kwinrc Effect-scale OutScale 0.92

# Breeze application style tweaks: translucent (blurred) menus, quieter toolbars
kw breezerc Style MenuOpacity 88
kw breezerc Style ToolBarDrawItemSeparator false
kw breezerc Style SidePanelDrawFrame false
kw breezerc Style MnemonicsMode MN_AUTO
kw breezerc Common OutlineCloseButton false

# ------------------------------------------------------------ SDDM theme "vela" (Breeze greeter, Vela look)
if [ -d /usr/share/sddm/themes/breeze ]; then
  rm -rf /usr/share/sddm/themes/vela
  cp -a /usr/share/sddm/themes/breeze /usr/share/sddm/themes/vela
  rm -f /usr/share/sddm/themes/vela/theme.conf.user
  sed -i -e 's/^Name=.*/Name=Vela/' -e '/^Name\[/d' -e 's/^Description=.*/Description=Vela/' -e '/^Description\[/d' \
         -e 's/^Theme-Id=.*/Theme-Id=vela/' /usr/share/sddm/themes/vela/metadata.desktop
  cp /usr/share/plasma/look-and-feel/org.vela.desktop/contents/splash/images/vela.svg /usr/share/sddm/themes/vela/vela-logo.svg
  cat > /usr/share/sddm/themes/vela/theme.conf <<'EOT'
[General]
showlogo=shown
showClock=true
logo=/usr/share/sddm/themes/vela/vela-logo.svg
type=image
color=#0B2A3A
fontSize=10
background=/usr/share/wallpapers/Vela/contents/images_dark/3840x2160.jpg
needsFullUserModel=false
EOT
  # Greeter user's Plasma settings (what "Applica impostazioni di Plasma" in the SDDM KCM does)
  SDDMHOME=$(getent passwd sddm | cut -d: -f6); SDDMHOME=${SDDMHOME:-/var/lib/sddm}
  mkdir -p "$SDDMHOME/.config"
  cp /usr/share/color-schemes/VelaDark.colors "$SDDMHOME/.config/kdeglobals"
  "$KW" --file "$SDDMHOME/.config/kdeglobals" --group General --key font "$UI"
  "$KW" --file "$SDDMHOME/.config/kdeglobals" --group General --key ColorScheme VelaDark
  "$KW" --file "$SDDMHOME/.config/kdeglobals" --group Icons --key Theme Vela-Dark
  "$KW" --file "$SDDMHOME/.config/plasmarc" --group Theme --key name vela
  "$KW" --file "$SDDMHOME/.config/kcminputrc" --group Mouse --key cursorTheme Bibata-Modern-Classic
  chown -R sddm:sddm "$SDDMHOME/.config" 2>/dev/null || true
fi

# ------------------------------------------------------------ Plymouth
if command -v plymouth-set-default-theme >/dev/null; then
  plymouth-set-default-theme vela || true
fi
# initramfs is regenerated by live-build at the end of the build; on installed systems update-initramfs runs on kernel updates.

# ------------------------------------------------------------ auto light/dark timer for every user
systemctl --global enable vela-theme-auto.timer >/dev/null 2>&1 || true

exit 0
