#!/bin/sh
# Vela OS 2 — component "terminal": Konsole defaults, fish as default shell,
# Vela AI Terminal + safe executor service. Runs as root in the chroot. Idempotent.
# Only writes the keys terminal owns into shared files (see coordination/requests.md).
set -e
ROOT=${VELA_ROOT:-}

KW=$(command -v kwriteconfig6 || true)
if [ -z "$KW" ]; then
    echo "terminal: kwriteconfig6 missing (package libkf6config-bin)" >&2
    exit 1
fi
kw() { "$KW" --file "$ROOT/etc/xdg/$1" --group "$2" --key "$3" "$4"; }

# ---------------- Konsole ----------------
# Profiles/colour schemes live in /usr/share/konsole (VelaLight/VelaDark, generated from the
# same palette as the AI Terminal). Default = light, like the system default appearance;
# vela-terminal-theme (autostart + theme switch) moves it to VelaDark when the system is dark.
kw konsolerc "Desktop Entry" DefaultProfile VelaLight.profile
kw konsolerc TabBar NewTabBehavior PutNewTabAfterCurrentTab
kw konsolerc TabBar CloseTabOnMiddleMouseButton true
kw konsolerc TabBar ExpandTabWidth true

# ---------------- fish as default shell for new users ----------------
FISH=/usr/bin/fish
if [ -x "$ROOT$FISH" ]; then
    grep -qx "$FISH" "$ROOT/etc/shells" 2>/dev/null || echo "$FISH" >> "$ROOT/etc/shells"
    # adduser: used by live-config (user-setup-apply) for the live user, and by hand
    if [ -f "$ROOT/etc/adduser.conf" ]; then
        if grep -q '^#\{0,1\}DSHELL=' "$ROOT/etc/adduser.conf"; then
            sed -i "s|^#\{0,1\}DSHELL=.*|DSHELL=$FISH|" "$ROOT/etc/adduser.conf"
        else
            echo "DSHELL=$FISH" >> "$ROOT/etc/adduser.conf"
        fi
    fi
    # useradd (used by Calamares unless its users.conf sets userShell)
    if [ -f "$ROOT/etc/default/useradd" ]; then
        if grep -q '^#\{0,1\} *SHELL=' "$ROOT/etc/default/useradd"; then
            sed -i "s|^#\{0,1\} *SHELL=.*|SHELL=$FISH|" "$ROOT/etc/default/useradd"
        else
            echo "SHELL=$FISH" >> "$ROOT/etc/default/useradd"
        fi
    else
        mkdir -p "$ROOT/etc/default"
        echo "SHELL=$FISH" > "$ROOT/etc/default/useradd"
    fi
fi

# ---------------- executor service (per-user, socket-activated) ----------------
if [ -z "$ROOT" ]; then
    systemctl --global enable vela-executor.socket >/dev/null 2>&1 || true
else
    mkdir -p "$ROOT/etc/systemd/user/sockets.target.wants"
    ln -sf /usr/lib/systemd/user/vela-executor.socket \
        "$ROOT/etc/systemd/user/sockets.target.wants/vela-executor.socket"
fi

# ---------------- docs ----------------
# The overlay ships /usr/share/doc/vela-terminal/ (EXECUTOR_API.md, README.md).

# byte-compile our Python package (faster first start), ignore failures
python3 -m compileall -q "$ROOT/usr/lib/vela/terminal/vela_terminal" >/dev/null 2>&1 || true
exit 0
