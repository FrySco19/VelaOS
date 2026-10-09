#!/bin/sh
# Vela OS 2 — component "onboarding": Benvenuto (first run), Guida e scorciatoie, Suggerimenti, Italian l10n checks.
# Runs as root inside the chroot after the overlay is copied. Idempotent.
set -e
ROOT=${VELA_ROOT:-}

chmod 0755 "$ROOT/usr/bin/vela-welcome" "$ROOT/usr/bin/vela-help" "$ROOT/usr/bin/vela-tips" \
           "$ROOT/usr/lib/vela/welcome/vela_welcome.py" "$ROOT/usr/lib/vela/welcome/vela_tips.py"

# Precompile Python so the first start is faster (users can't write to /usr).
if command -v python3 >/dev/null 2>&1; then
    python3 -m compileall -q "$ROOT/usr/lib/vela/welcome" >/dev/null 2>&1 || true
fi

# Only one welcome app: hide KDE's plasma-welcome if some package pulled it in.
for f in "$ROOT/etc/xdg/autostart/org.kde.plasma-welcome.desktop" "$ROOT/etc/xdg/autostart/plasma-welcome.desktop"; do
    if [ -f "$f" ] && ! grep -q '^Hidden=true' "$f"; then
        printf 'Hidden=true\n' >> "$f"
        echo "onboarding: hid $(basename "$f") (Benvenuto replaces it)"
    fi
done

# Italian default + English available (base sets them; we only verify and fill gaps).
if [ -f "$ROOT/etc/locale.gen" ]; then
    changed=0
    for l in it_IT.UTF-8 en_US.UTF-8; do
        if ! grep -q "^$l UTF-8" "$ROOT/etc/locale.gen"; then
            sed -i "s/^# *\($l UTF-8\)/\1/" "$ROOT/etc/locale.gen"
            grep -q "^$l UTF-8" "$ROOT/etc/locale.gen" || echo "$l UTF-8" >> "$ROOT/etc/locale.gen"
            changed=1
        fi
    done
    if [ "$changed" = 1 ] && [ -z "$ROOT" ] && command -v locale-gen >/dev/null 2>&1; then
        locale-gen >/dev/null
        echo "onboarding: generated missing locales"
    fi
fi

# Screen reader: Plasma's kaccess toggles Orca with Meta+Alt+S; keep it off by default (Benvenuto can enable it).
if command -v kwriteconfig6 >/dev/null 2>&1; then
    kwriteconfig6 --file "$ROOT/etc/xdg/kaccessrc" --group ScreenReader --key Enabled --type bool false
fi
echo "onboarding: hook done"
