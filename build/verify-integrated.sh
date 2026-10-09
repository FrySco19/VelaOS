#!/bin/sh
# Verify the actual root filesystem shipped in an ISO, not the source overlays.
set -eu
ROOT=${1:?usage: verify-integrated.sh extracted-root}
check() { [ -s "$ROOT/$1" ] || { echo "FAIL missing $1"; exit 1; }; echo "PASS $1"; }
for f in \
 etc/vela/components.list etc/vela/ai.conf \
 usr/bin/vela-theme usr/bin/vela-desktop-settings usr/bin/vela-ai-config \
 usr/bin/vela-spazi usr/bin/vela-ai-terminal usr/bin/vela-executor \
 usr/bin/vela-assistant usr/bin/vela-welcome usr/bin/vela-help \
 usr/share/plasma/look-and-feel/org.vela.desktop/metadata.json \
 usr/share/plasma/layout-templates/org.vela.desktop.defaultPanels/metadata.json \
 usr/share/vela/store/featured.json; do check "$f"; done
for f in "$ROOT"/usr/bin/vela-*; do
 [ -x "$f" ] || { echo "FAIL not executable $f"; exit 1; }
done
for k in ai privacy aspetto updates; do
 check "usr/lib/x86_64-linux-gnu/qt6/plugins/plasma/kcms/systemsettings/kcm_vela_$k.so"
done
for c in base perf theme desktop settings-store apps terminal assistant onboarding; do
 grep -qx "$c" "$ROOT/etc/vela/components.list" || { echo "FAIL component $c"; exit 1; }
done
echo 'PASS integrated filesystem'
