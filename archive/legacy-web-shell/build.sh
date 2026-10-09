#!/bin/sh
set -eu
SRC=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
BUILD=${VELA_BUILD_DIR:-/var/tmp/vela-live}
mkdir -p "$BUILD"
cd "$BUILD"
if [ -d config ]; then lb clean --purge; fi
lb config --debootstrap-options '--include=ca-certificates' --distribution trixie --architectures amd64 --binary-images iso-hybrid --archive-areas 'main contrib non-free non-free-firmware' --bootloaders 'syslinux grub-efi' --debian-installer none --bootappend-live 'boot=live noeject components username=vela hostname=vela locales=it_IT.UTF-8 keyboard-layouts=it timezone=Europe/Rome quiet splash' --iso-volume VELA_PREVIEW --memtest none --apt-recommends false
printf '%s\n' 'LB_CHROOT_SQUASHFS_COMPRESSION_TYPE="zstd"' 'LB_CHROOT_SQUASHFS_COMPRESSION_LEVEL="10"' >> config/chroot
mkdir -p config/package-lists config/includes.chroot/opt/vela config/includes.chroot/usr/share/xsessions config/includes.chroot/etc/lightdm/lightdm.conf.d config/hooks/live
mkdir -p config/archives
printf '%s\n' 'deb https://fasttrack.debian.net/debian-fasttrack trixie-fasttrack main contrib' > config/archives/fasttrack.list.chroot
cp "$SRC/fasttrack.asc" config/archives/fasttrack.key.chroot
mkdir -p config/includes.bootstrap/etc/ssl/certs
cp /etc/ssl/certs/ca-certificates.crt config/includes.bootstrap/etc/ssl/certs/
mkdir -p config/includes.bootstrap/etc/apt/apt.conf.d
printf '%s\n' 'Acquire::https::CaInfo "/etc/ssl/certs/ca-certificates.crt";' > config/includes.bootstrap/etc/apt/apt.conf.d/99-vela-ca
cat > config/package-lists/vela.list.chroot <<'EOF'
linux-image-amd64
live-boot
live-config
user-setup
systemd-sysv
xserver-xorg
xinit
openbox
lightdm
chromium
firefox-esr
xclip
xfce4-settings
pavucontrol
network-manager-gnome
synaptic
mousepad
wmctrl
python3
fonts-inter
fonts-dejavu
xterm
fish
dbus-x11
network-manager
pulseaudio
alsa-utils
btrfs-progs
sudo
curl
ca-certificates
mesa-utils
virtualbox-guest-x11
virtualbox-guest-utils
EOF
cp -r "$SRC/shell" "$BUILD/config/includes.chroot/opt/vela/"
cp "$SRC/service.py" "$SRC/system_api.py" "$SRC/openbox.xml" "$BUILD/config/includes.chroot/opt/vela/"
mkdir -p config/includes.chroot/etc/chromium/policies/managed
cp "$SRC/chromium-policy.json" config/includes.chroot/etc/chromium/policies/managed/vela.json
cat > config/includes.chroot/usr/share/xsessions/vela.desktop <<'EOF'
[Desktop Entry]
Name=Vela
Exec=/opt/vela/session.sh
Type=Application
EOF
cat > config/includes.chroot/opt/vela/session.sh <<'EOF'
#!/bin/sh
openbox --config-file /opt/vela/openbox.xml &
python3 /opt/vela/service.py &
until curl -sf http://127.0.0.1:8766/ >/dev/null; do sleep .2; done
exec chromium --kiosk --no-first-run --disable-session-crashed-bubble --disable-features=Translate,TranslateUI --password-store=basic http://127.0.0.1:8766/
EOF
chmod +x config/includes.chroot/opt/vela/session.sh
cat > config/includes.chroot/etc/lightdm/lightdm.conf.d/20-vela.conf <<'EOF'
[Seat:*]
autologin-user=vela
autologin-user-timeout=0
user-session=vela
EOF
cat > config/hooks/live/0500-vela.hook.chroot <<'EOF'
#!/bin/sh
set -e
apt-get install -y user-setup
systemctl enable lightdm NetworkManager
mkdir -p /etc/skel/Documents /etc/skel/Downloads /etc/skel/Pictures
echo 'Benvenuto in Vela. I tuoi file restano sul dispositivo.' > /etc/skel/Documents/Benvenuto.txt
EOF
chmod +x config/hooks/live/0500-vela.hook.chroot
lb build
FINALDIR=$(mktemp -d /var/tmp/vela-image.XXXXXX)
sh "$SRC/finalize-iso.sh" "$BUILD/live-image-amd64.hybrid.iso" "$FINALDIR/vela.iso"
cp "$FINALDIR/vela.iso" "$SRC/Vela-Banchina-preview.iso"
cd "$SRC"
sha256sum Vela-Banchina-preview.iso > SHA256SUMS-BANCHINA
