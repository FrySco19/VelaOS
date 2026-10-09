#!/bin/sh
# Rebuild the Vela KCMs (kcm_vela_ai, kcm_vela_privacy) on a Debian 13 host with
#   cmake extra-cmake-modules g++ qt6-base-dev qt6-declarative-dev libkf6kcmutils-dev libkf6i18n-dev libkf6coreaddons-dev gettext python3-polib
# and copy the installed files into overlay/ (same ABI as the trixie ISO).
set -e
here=$(cd "$(dirname "$0")" && pwd)
b=${BUILD_DIR:-/tmp/vela-kcm-build}
stage=$(mktemp -d)
python3 "$here/kcm/po/make_po.py"
cmake -S "$here/kcm" -B "$b" -DCMAKE_INSTALL_PREFIX=/usr -DCMAKE_BUILD_TYPE=Release -DBUILD_TESTING=OFF >/dev/null
nice make -C "$b" -j"${JOBS:-1}" >/dev/null
DESTDIR="$stage" make -C "$b" install >/dev/null
strip --strip-unneeded "$stage"/usr/lib/*/qt6/plugins/plasma/kcms/systemsettings/kcm_vela_*.so
cd "$stage"
find . -type f | while read -r f; do
  mkdir -p "$here/overlay/$(dirname "$f")"
  cp "$f" "$here/overlay/$f"
  echo "installed overlay/${f#./}"
done
rm -rf "$stage"
