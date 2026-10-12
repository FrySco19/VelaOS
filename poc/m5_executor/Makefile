MICROKIT_SDK ?= /microkit
BOARD ?= qemu_virt_aarch64
CONFIG ?= debug
BUILD ?= build
# E2E=1 builds the verify-bridge-e2e image: PCB e2e sequence + simulated LFS domain.
E2E ?= 0
CARGO_TARGET_DIR := $(BUILD)/cargo-target
SEL4_INCLUDE_DIRS := $(MICROKIT_SDK)/board/$(BOARD)/$(CONFIG)/include
TARGET_TRIPLE := aarch64-sel4-microkit
EXECUTOR_ELF := $(BUILD)/executor.elf
PCB_ELF := $(BUILD)/pcb.elf
LOADER := $(BUILD)/loader.img
CARGO_ENV := SEL4_INCLUDE_DIRS="$(SEL4_INCLUDE_DIRS)" CARGO_TARGET_DIR="../$(CARGO_TARGET_DIR)"

ifeq ($(E2E),1)
SYSTEM_FILE := executor-e2e.system
CARGO_FEATURES := --features e2e
LFS_ELF := $(BUILD)/lfs.elf
E2E_INPUTS := app/src/bin/lfs.rs app/src/guest_clock.rs app/generated/e2e_frame.bin app/generated/e2e_meta.bin app/generated/e2e_frame_short.bin app/generated/e2e_clock.bin
else
SYSTEM_FILE := executor.system
CARGO_FEATURES :=
LFS_ELF :=
E2E_INPUTS :=
endif

.PHONY: all build run clean

all: build

build: $(LOADER)

$(BUILD)/cargo-target/.built: app/Cargo.toml app/Cargo.lock app/src/main.rs app/src/bridge.rs app/src/ledger_wire.rs app/src/trusted_key.rs app/src/bin/pcb.rs app/rust-toolchain.toml app/rustc-wrapper.sh app/.cargo/config.toml app/targets/aarch64-sel4-microkit.json $(E2E_INPUTS)
	mkdir -p $(BUILD)
	cd app && $(CARGO_ENV) cargo build --locked --release $(CARGO_FEATURES) --bin velaos-executor
	cd app && $(CARGO_ENV) cargo build --locked --release $(CARGO_FEATURES) --bin velaos-pcb
ifeq ($(E2E),1)
	cd app && $(CARGO_ENV) cargo build --locked --release $(CARGO_FEATURES) --bin velaos-lfs
endif
	touch $@

$(EXECUTOR_ELF): $(BUILD)/cargo-target/.built
	cp $(CARGO_TARGET_DIR)/$(TARGET_TRIPLE)/release/velaos-executor.elf $@
	python3 check-wx.py $@

$(PCB_ELF): $(BUILD)/cargo-target/.built
	cp $(CARGO_TARGET_DIR)/$(TARGET_TRIPLE)/release/velaos-pcb.elf $@
	python3 check-wx.py $@

$(BUILD)/lfs.elf: $(BUILD)/cargo-target/.built
	cp $(CARGO_TARGET_DIR)/$(TARGET_TRIPLE)/release/velaos-lfs.elf $@
	python3 check-wx.py $@

$(LOADER): $(SYSTEM_FILE) $(EXECUTOR_ELF) $(PCB_ELF) $(LFS_ELF)
	$(MICROKIT_SDK)/bin/microkit $(SYSTEM_FILE) \
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
