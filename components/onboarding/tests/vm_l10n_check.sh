#!/bin/sh
# Italian-completeness checks, run INSIDE the dev VM (as user vela):
#   tools/vm.sh -n dev ssh 'sh -s' < components/onboarding/tests/vm_l10n_check.sh
# Read-only. Prints a plain-text report.
echo "== locale"
locale -a 2>/dev/null | grep -E '^(it_IT|en_US)' | tr '\n' ' '; echo
echo "LANG=$LANG LANGUAGE=${LANGUAGE:-}"; cat /etc/default/locale 2>/dev/null
grep -E '^(it_IT|en_US)\.UTF-8' /etc/locale.gen
echo "== keyboard"; grep -E '^XKB' /etc/default/keyboard 2>/dev/null
for f in "$HOME/.config/kxkbrc" /etc/xdg/kxkbrc; do [ -f "$f" ] && { echo "$f:"; grep -E 'LayoutList|VariantList|Use=' "$f"; }; done
echo "== plasma-localerc"; cat "$HOME/.config/plasma-localerc" 2>/dev/null
echo "== packages"
for p in hunspell-it hyphen-it mythes-it aspell-it manpages-it qt6-translations-l10n firefox-esr-l10n-it libreoffice-l10n-it \
         orca speech-dispatcher-espeak-ng espeak-ng hunspell-en-us plasma-welcome fonts-inter; do
  if dpkg -s "$p" >/dev/null 2>&1; then echo "  ok      $p"; else echo "  MISSING $p"; fi
done
echo "== KDE translations present (it .mo per installed KDE component)"
n=$(ls /usr/share/locale/it/LC_MESSAGES/ 2>/dev/null | wc -l); echo "  it catalogs: $n"
for m in plasmashell plasma_shell_org.kde.plasma.desktop kcmshell6 systemsettings dolphin konsole kate okular spectacle \
         plasma-discover libdiscover kwin calamares-ui calamares knotifications6_qt kio6 plasma_applet_org.kde.plasma.kickoff; do
  ls /usr/share/locale/it/LC_MESSAGES/$m.mo >/dev/null 2>&1 && echo "  ok      $m.mo" || echo "  absent  $m.mo"
done
echo "== menu entries without Italian name/comment (visible apps)"
for f in /usr/local/share/applications/*.desktop /usr/share/applications/*.desktop "$HOME"/.local/share/applications/*.desktop; do
  [ -f "$f" ] || continue
  b=$(basename "$f")
  # skip shadowed system copy when a local override exists
  case $f in /usr/share/*) [ -f "/usr/local/share/applications/$b" ] && continue;; esac
  sed -n '/^\[Desktop Entry\]/,/^\[/p' "$f" > /tmp/.de.$$
  grep -qiE '^NoDisplay=true|^Hidden=true' /tmp/.de.$$ && continue
  grep -qE '^OnlyShowIn=' /tmp/.de.$$ && ! grep -qE '^OnlyShowIn=.*KDE' /tmp/.de.$$ && continue
  grep -qE '^NotShowIn=.*KDE' /tmp/.de.$$ && continue
  name=$(grep -m1 '^Name=' /tmp/.de.$$ | cut -d= -f2-)
  nit=$(grep -m1 -E '^Name\[it(_IT)?\]=' /tmp/.de.$$ | cut -d= -f2-)
  cit=$(grep -m1 -E '^(Comment|GenericName)\[it(_IT)?\]=' /tmp/.de.$$ | cut -d= -f2-)
  cen=$(grep -m1 -E '^(Comment|GenericName)=' /tmp/.de.$$ | cut -d= -f2-)
  if [ -z "$nit" ] || { [ -n "$cen" ] && [ -z "$cit" ]; }; then
    echo "  $b | Name=$name | Name[it]=${nit:-—} | Comment=$cen | it:${cit:-—}"
  fi
done
rm -f /tmp/.de.$$
echo "== welcome autostart"
ls /etc/xdg/autostart/ | grep -iE 'vela|welcome' ; grep -h '^Hidden' /etc/xdg/autostart/org.kde.plasma-welcome*.desktop 2>/dev/null
