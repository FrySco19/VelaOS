FROM debian:trixie
RUN apt-get update && apt-get install -y live-build debootstrap squashfs-tools xorriso isolinux syslinux-common grub-pc-bin grub-efi-amd64-bin mtools dosfstools ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /src
COPY . /src
CMD ["sh", "build.sh"]
