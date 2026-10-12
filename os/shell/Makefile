# VelaOS shell (OS-0). Pure C99 + libm; vendored stb headers; fonts in assets/fonts.
CC      ?= cc
CFLAGS  ?= -O2 -g0
CFLAGS  += -std=gnu99 -Wall -Wextra -Wno-unused-function -Iinclude -Isrc
LDLIBS  += -lm
CORE    := src/gfx.c src/text.c src/theme.c src/wallpaper.c src/glyphs.c src/shell.c
BUILD   := build

.PHONY: all test preview guest clean
all: $(BUILD)/velashell-preview $(BUILD)/velashell

$(BUILD):
	mkdir -p $(BUILD)

$(BUILD)/velashell-preview: $(CORE) src/main_headless.c include/vs.h | $(BUILD)
	$(CC) $(CFLAGS) -o $@ $(CORE) src/main_headless.c $(LDLIBS)

# Guest binary: static so it can run as PID 1 inside a minimal Linux initramfs.
$(BUILD)/velashell: $(CORE) src/main_fbdev.c include/vs.h | $(BUILD)
	$(CC) $(CFLAGS) -static -o $@ $(CORE) src/main_fbdev.c $(LDLIBS)

test: $(BUILD)/velashell-preview
	./$(BUILD)/velashell-preview --test --size 1920x1080
	./$(BUILD)/velashell-preview --test --size 1280x800

preview: $(BUILD)/velashell-preview
	mkdir -p $(BUILD)/preview
	./$(BUILD)/velashell-preview --out $(BUILD)/preview

guest: $(BUILD)/velashell
	./guest/build-initramfs.sh

clean:
	rm -rf $(BUILD)
