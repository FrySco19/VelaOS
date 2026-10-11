MICROKIT_SDK ?= /microkit
BOARD ?= qemu_virt_aarch64
CONFIG ?= debug
BUILD ?= build
CARGO_TARGET_DIR := $(BUILD)/cargo-target
SEL4_INCLUDE_DIRS := $(MICROKIT_SDK)/board/$(BOARD)/$(CONFIG)/include
APP_ELF := $(CARGO_TARGET_DIR)/aarch64-sel4-microkit/release/velaos-gate-a-hello.elf
PD_ELF := $(BUILD)/hello_domain.elf
LOADER := $(BUILD)/loader.img

.PHONY: all build run clean

all: build

build: $(LOADER)

$(APP_ELF): app/Cargo.toml app/Cargo.lock app/src/main.rs app/rust-toolchain.toml app/rustc-wrapper.sh app/.cargo/config.toml app/targets/aarch64-sel4-microkit.json
	mkdir -p $(BUILD)
	cd app && SEL4_INCLUDE_DIRS="$(SEL4_INCLUDE_DIRS)" CARGO_TARGET_DIR="../$(CARGO_TARGET_DIR)" cargo build --locked --release

$(PD_ELF): $(APP_ELF)
	cp $(APP_ELF) $(PD_ELF)

$(LOADER): hello.system $(PD_ELF)
	$(MICROKIT_SDK)/bin/microkit hello.system \
		--search-path $(BUILD) \
		--board $(BOARD) \
		--config $(CONFIG) \
		--output $(LOADER)

run: $(LOADER)
	qemu-system-aarch64 \
		-machine virt,virtualization=on \
		-cpu cortex-a53 \
		-m size=2G \
		-nic none \
		-serial mon:stdio \
		-nographic \
		-device loader,file=$(LOADER),addr=0x70000000,cpu-num=0

clean:
	rm -rf $(BUILD) app/target
