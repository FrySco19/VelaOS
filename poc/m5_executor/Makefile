MICROKIT_SDK ?= /microkit
BOARD ?= qemu_virt_aarch64
CONFIG ?= debug
BUILD ?= build
CARGO_TARGET_DIR := $(BUILD)/cargo-target
SEL4_INCLUDE_DIRS := $(MICROKIT_SDK)/board/$(BOARD)/$(CONFIG)/include
TARGET_TRIPLE := aarch64-sel4-microkit
EXECUTOR_ELF := $(BUILD)/executor.elf
PCB_ELF := $(BUILD)/pcb.elf
LOADER := $(BUILD)/loader.img

.PHONY: all build run clean

all: build

build: $(LOADER)

$(BUILD)/cargo-target/.built: app/Cargo.toml app/Cargo.lock app/src/main.rs app/src/bridge.rs app/src/bin/pcb.rs app/rust-toolchain.toml app/rustc-wrapper.sh app/.cargo/config.toml app/targets/aarch64-sel4-microkit.json
	mkdir -p $(BUILD)
	cd app && SEL4_INCLUDE_DIRS="$(SEL4_INCLUDE_DIRS)" CARGO_TARGET_DIR="../$(CARGO_TARGET_DIR)" cargo build --locked --release --bin velaos-executor
	cd app && SEL4_INCLUDE_DIRS="$(SEL4_INCLUDE_DIRS)" CARGO_TARGET_DIR="../$(CARGO_TARGET_DIR)" cargo build --locked --release --bin velaos-pcb
	touch $@

$(EXECUTOR_ELF): $(BUILD)/cargo-target/.built
	cp $(CARGO_TARGET_DIR)/$(TARGET_TRIPLE)/release/velaos-executor.elf $@
	python3 check-wx.py $@

$(PCB_ELF): $(BUILD)/cargo-target/.built
	cp $(CARGO_TARGET_DIR)/$(TARGET_TRIPLE)/release/velaos-pcb.elf $@
	python3 check-wx.py $@

$(LOADER): executor.system $(EXECUTOR_ELF) $(PCB_ELF)
	$(MICROKIT_SDK)/bin/microkit executor.system \
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
