# VelaOS x86_64 bootable ISO
CC      ?= gcc
RUSTC   ?= rustc
CFLAGS  := -std=gnu11 -O2 -ffreestanding -fno-stack-protector -fno-pic -fno-pie -mno-red-zone \
           -mgeneral-regs-only -mcmodel=small -m64 -Wall -Wextra -Wno-unused-parameter -Isrc
LDFLAGS := --strip-debug -nostdlib -static -no-pie -z max-page-size=0x1000 -T src/linker.ld
OUT     := build
ISO     := $(OUT)/velaos-x86_64.iso

all: $(ISO)

$(OUT)/boot.o: src/boot.asm | $(OUT)
	nasm -f elf64 $< -o $@
$(OUT)/%.o: src/%.c src/font.h src/glass_ui.inc src/wallpaper_size.h | $(OUT)
	$(CC) $(CFLAGS) -c $< -o $@
src/fontdata.c: tools/genfont.py
	python3 tools/genfont.py $@
$(OUT)/libvela_glass.a: src/glass.rs | $(OUT)
	$(RUSTC) --edition=2024 --crate-type staticlib --target x86_64-unknown-none -C panic=abort -C opt-level=2 -C relocation-model=static -C no-redzone=yes $< -o $@
$(OUT)/wallpaper.o: src/wallpaper.asm assets/ocean.rgb565 | $(OUT)
	nasm -f elf64 $< -o $@
$(OUT)/vela.elf: $(OUT)/boot.o $(OUT)/kernel.o $(OUT)/fontdata.o $(OUT)/wallpaper.o $(OUT)/libvela_glass.a
	ld $(LDFLAGS) -o $@ $^
	grub-file --is-x86-multiboot2 $@
$(ISO): $(OUT)/vela.elf iso/boot/grub/grub.cfg
	rm -rf $(OUT)/isoroot && cp -r iso $(OUT)/isoroot && cp $(OUT)/vela.elf $(OUT)/isoroot/boot/vela.elf
	grub-mkrescue -o $@ $(OUT)/isoroot
$(OUT):
	mkdir -p $@

run: $(ISO)
	qemu-system-x86_64 -m 256 -cdrom $(ISO) -serial stdio

clean:
	rm -rf $(OUT)
.PHONY: all run clean
