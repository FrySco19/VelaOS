#!/bin/sh
# Scripted equivalent of the Calamares install, run as root INSIDE a booted Vela live ISO
# (test build, with SSH). Installs to /dev/vda: GPT, BIOS-boot + ESP + Btrfs (@,@home,@cache,@log),
# user vela/vela (passwordless sudo, autologin to Plasma), GRUB for BIOS and EFI (removable path).
# Used by tools/devvm.sh create. Destroys /dev/vda.
set -eu
DISK=${1:-/dev/vda}
T=/target
export DEBIAN_FRONTEND=noninteractive
log() { echo "[devvm-install] $*"; }
[ -b "$DISK" ] || { echo "no disk $DISK"; exit 1; }
SRC=/run/live/rootfs/filesystem.squashfs
[ -d "$SRC/usr" ] || { echo "live rootfs not found at $SRC"; exit 1; }

log "partitioning $DISK"
wipefs -af "$DISK"
sfdisk --label gpt "$DISK" <<EOT
size=1MiB, type=21686148-6E6F-7449-6E6F-744E65656445, name=bios
size=512MiB, type=C12A7328-F81F-11D2-BA4B-00A0C93EC93B, name=EFI
type=0FC63DAF-8483-4772-8E79-3D69D8477DE4, name=vela
EOT
udevadm settle
P2=${DISK}2; P3=${DISK}3
mkfs.vfat -F32 -n EFI "$P2" >/dev/null
mkfs.btrfs -f -L vela "$P3" >/dev/null
mkdir -p $T; mount "$P3" $T
for sv in @ @home @cache @log; do btrfs subvolume create "$T/$sv" >/dev/null; done
umount $T
O=noatime,compress=zstd:1
mount -o subvol=@,$O "$P3" $T
mkdir -p $T/home $T/var/cache $T/var/log $T/boot/efi
mount -o subvol=@home,$O "$P3" $T/home
mount -o subvol=@cache,$O "$P3" $T/var/cache
mount -o subvol=@log,$O "$P3" $T/var/log
mount -o umask=0077 "$P2" $T/boot/efi

log "copying system (rsync from squashfs)"
rsync -aHAX --info=progress2 --no-inc-recursive "$SRC/" $T/ | tail -1
UB=$(blkid -s UUID -o value "$P3"); UE=$(blkid -s UUID -o value "$P2")
cat > $T/etc/fstab <<EOT
UUID=$UB /          btrfs subvol=@,$O 0 0
UUID=$UB /home      btrfs subvol=@home,$O 0 0
UUID=$UB /var/cache btrfs subvol=@cache,$O 0 0
UUID=$UB /var/log   btrfs subvol=@log,$O 0 0
UUID=$UE /boot/efi  vfat  umask=0077 0 1
EOT
for d in dev dev/pts proc sys run; do mount --bind /$d $T/$d; done
[ -d /sys/firmware/efi/efivars ] && mount --bind /sys/firmware/efi/efivars $T/sys/firmware/efi/efivars || true
cp /etc/resolv.conf $T/etc/resolv.conf 2>/dev/null || true

log "configuring target"
chroot $T /bin/sh -eu <<'EOT'
export DEBIAN_FRONTEND=noninteractive
apt-get purge -y -q live-boot live-boot-initramfs-tools live-config live-config-systemd live-tools calamares calamares-settings-debian >/dev/null 2>&1 || \
  apt-get purge -y -q live-boot live-config live-config-systemd >/dev/null || true
rm -f /etc/polkit-1/rules.d/49-vela-live.rules /usr/share/applications/vela-install.desktop
echo vela-dev > /etc/hostname
printf '127.0.0.1 localhost\n127.0.1.1 vela-dev\n::1 localhost ip6-localhost ip6-loopback\n' > /etc/hosts
systemd-machine-id-setup >/dev/null
id vela >/dev/null 2>&1 || useradd -m -s /bin/bash -G sudo,audio,video,plugdev,netdev,users vela
for g in bluetooth lpadmin scanner; do getent group $g >/dev/null && usermod -aG $g vela || true; done
echo vela:vela | chpasswd
echo 'vela ALL=(ALL) NOPASSWD:ALL' > /etc/sudoers.d/90-vela-dev; chmod 440 /etc/sudoers.d/90-vela-dev
mkdir -p /etc/sddm.conf.d
printf '[Autologin]\nUser=vela\nSession=plasma\nRelogin=false\n' > /etc/sddm.conf.d/50-vela-dev-autologin.conf
ssh-keygen -A >/dev/null 2>&1 || true
systemctl enable ssh 2>/dev/null || true
apt-get update -q >/dev/null 2>&1 || true
apt-get install -y -q grub-pc grub-efi-amd64-bin wl-clipboard xclip >/dev/null 2>&1 || apt-get install -y -q grub-pc grub-efi-amd64-bin
if [ -x /usr/lib/vela/base/postinstall.sh ]; then /usr/lib/vela/base/postinstall.sh || true; fi
EOT
log "bootloader"
chroot $T grub-install --target=i386-pc "$DISK"
chroot $T grub-install --target=x86_64-efi --efi-directory=/boot/efi --removable --no-nvram --bootloader-id=Vela
chroot $T update-initramfs -u -k all >/dev/null
chroot $T update-grub 2>&1 | tail -2
sync
for d in sys/firmware/efi/efivars run sys proc dev/pts dev boot/efi var/log var/cache home; do umount $T/$d 2>/dev/null || true; done
umount $T
log "done"
