# VelaOS x86_64 bootable ISO
CC      ?= gcc
CFLAGS  := -std=gnu11 -O2 -ffreestanding -fno-stack-protector -fno-pic -fno-pie -mno-red-zone \
           -mgeneral-regs-only -mcmodel=small -m64 -Wall -Wextra -Wno-unused-parameter -Isrc
LDFLAGS := -nostdlib -static -no-pie -z max-page-size=0x1000 -T src/linker.ld
OUT     := build
ISO     := $(OUT)/velaos-x86_64.iso

all: $(ISO)

$(OUT)/boot.o: src/boot.asm | $(OUT)
	nasm -f elf64 $< -o $@
$(OUT)/%.o: src/%.c src/font.h | $(OUT)
	$(CC) $(CFLAGS) -c $< -o $@
src/fontdata.c: tools/genfont.py
	python3 tools/genfont.py $@
$(OUT)/vela.elf: $(OUT)/boot.o $(OUT)/kernel.o $(OUT)/fontdata.o
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
