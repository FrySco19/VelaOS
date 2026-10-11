MICROKIT_SDK ?= /microkit
BOARD ?= qemu_virt_aarch64
CONFIG ?= debug
BUILD ?= build
CARGO_TARGET_DIR := $(BUILD)/cargo-target
SEL4_INCLUDE_DIRS := $(MICROKIT_SDK)/board/$(BOARD)/$(CONFIG)/include
PD_NAMES := pcb orchestrator bus lfs executor
APP_ELFS := $(addprefix $(CARGO_TARGET_DIR)/aarch64-sel4-microkit/release/,$(addsuffix .elf,$(PD_NAMES)))
PD_ELFS := $(addprefix $(BUILD)/,$(addsuffix .elf,$(PD_NAMES)))
APP_BUILD_STAMP := $(BUILD)/cargo-target/.built
LOADER := $(BUILD)/loader.img

.PHONY: all build run clean

all: build

build: $(LOADER)

$(APP_BUILD_STAMP): app/Cargo.toml app/Cargo.lock $(wildcard app/src/*.rs app/src/bin/*.rs) app/rust-toolchain.toml app/rustc-wrapper.sh app/.cargo/config.toml app/targets/aarch64-sel4-microkit.json
	mkdir -p $(BUILD)
	cd app && SEL4_INCLUDE_DIRS="$(SEL4_INCLUDE_DIRS)" CARGO_TARGET_DIR="../$(CARGO_TARGET_DIR)" cargo build --locked --release --bins
	touch $@

$(BUILD)/%.elf: $(APP_BUILD_STAMP)
	cp $(CARGO_TARGET_DIR)/aarch64-sel4-microkit/release/$*.elf $@

$(LOADER): velaos.system $(PD_ELFS)
	$(MICROKIT_SDK)/bin/microkit velaos.system \
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
