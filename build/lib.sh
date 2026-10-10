# shellcheck shell=sh
# Shared helpers for Vela 2 build scripts (sourced, POSIX sh).
VELA_ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
VELA_VERSION=${VELA_VERSION:-$(cat "$VELA_ROOT/build/VERSION" 2>/dev/null || echo 2.0)}
VELA_BUILD_DIR=${VELA_BUILD_DIR:-/var/tmp/vela2-live}
VELA_OUT=${VELA_OUT:-$VELA_ROOT/build/out}
VELA_LOCK=$VELA_ROOT/coordination/build.lock
# Order of component application
VELA_COMPONENTS=${VELA_COMPONENTS:-"base perf theme desktop settings-store apps terminal assistant local-ai onboarding qa"}
VELA_SSH_KEY=${VELA_SSH_KEY:-$VELA_ROOT/.local/ssh/id_ed25519}

log() { printf '\033[1;34m[vela]\033[0m %s\n' "$*" >&2; }
die() { printf '\033[1;31m[vela] ERROR:\033[0m %s\n' "$*" >&2; exit 1; }

# Print package names of a packages.list (strip comments/blank lines)
vela_pkglist() { sed -e 's/#.*//' -e 's/[[:space:]]\+/\n/g' "$1" | sed '/^$/d'; }

ensure_ssh_key() {
  if [ ! -f "$VELA_SSH_KEY" ]; then
    mkdir -p "$(dirname "$VELA_SSH_KEY")"
    ssh-keygen -q -t ed25519 -N '' -C vela-dev -f "$VELA_SSH_KEY"
  fi
  # key must stay readable by the local developer (builds run under sudo)
  [ -n "${SUDO_UID:-}" ] && chown -R "$SUDO_UID:${SUDO_GID:-$SUDO_UID}" "$(dirname "$VELA_SSH_KEY")" || true
}

# Assemble component inputs into a live-build style tree.
#   $1 = target dir that receives: package-lists/, includes.chroot/, hooks/normal/
#   $2 = 1 for test build (adds openssh-server + dev key)
vela_assemble() {
  dst=$1; test_build=${2:-0}
  mkdir -p "$dst/package-lists" "$dst/includes.chroot" "$dst/hooks/normal"
  i=10
  for c in $VELA_COMPONENTS; do
    cdir=$VELA_ROOT/components/$c
    [ -d "$cdir" ] || die "missing component: $c"
    if [ -f "$cdir/packages.list" ]; then
      vela_pkglist "$cdir/packages.list" > "$dst/package-lists/$i-vela-$c.list.chroot"
    fi
    if [ -d "$cdir/overlay" ]; then
      cp -a "$cdir/overlay/." "$dst/includes.chroot/"
      find "$dst/includes.chroot" -type d -name __pycache__ -prune -exec rm -rf {} +
      # ZIP extraction on Windows loses Unix modes. Restore safe defaults and
      # executable launchers before live-build copies this tree into the image.
      find "$dst/includes.chroot" -type d -exec chmod 755 {} +
      find "$dst/includes.chroot" -type f -exec chmod 644 {} +
      if [ "$(id -u)" = 0 ]; then chown -R 0:0 "$dst/includes.chroot"; fi
      find "$dst/includes.chroot" -type f -exec sh -c '
        for f do
          [ "$(head -c 2 "$f")" != "#!" ] || chmod 755 "$f"
        done
      ' sh {} +
    fi
    if [ -f "$cdir/hook.chroot.sh" ]; then
      cp "$cdir/hook.chroot.sh" "$dst/hooks/normal/70$i-vela-$c.hook.chroot"
      chmod 755 "$dst/hooks/normal/70$i-vela-$c.hook.chroot"
    fi
    i=$((i+1))
  done
  mkdir -p "$dst/includes.chroot/etc/vela"
  printf '%s\n' $VELA_COMPONENTS > "$dst/includes.chroot/etc/vela/components.list"
  if [ "$test_build" = 1 ]; then
    ensure_ssh_key
    printf '%s\n' openssh-server > "$dst/package-lists/99-vela-test.list.chroot"
    mkdir -p "$dst/includes.chroot/etc/skel/.ssh" "$dst/includes.chroot/etc/vela"
    cp "$VELA_SSH_KEY.pub" "$dst/includes.chroot/etc/skel/.ssh/authorized_keys"
    chmod 700 "$dst/includes.chroot/etc/skel/.ssh"; chmod 600 "$dst/includes.chroot/etc/skel/.ssh/authorized_keys"
    echo "test build $(date -Is)" > "$dst/includes.chroot/etc/vela/test-build"
    cat > "$dst/hooks/normal/7099-vela-testbuild.hook.chroot" <<'EOT'
#!/bin/sh
set -e
systemctl enable ssh.service 2>/dev/null || true
mkdir -p /etc/ssh/sshd_config.d
printf 'PasswordAuthentication yes\nUseDNS no\n' > /etc/ssh/sshd_config.d/50-vela-test.conf
EOT
    chmod 755 "$dst/hooks/normal/7099-vela-testbuild.hook.chroot"
  fi
}
