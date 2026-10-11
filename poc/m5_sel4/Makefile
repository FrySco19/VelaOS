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
ROOT_TASK_TARGET := aarch64-sel4-roottask-minimal
ROOT_TASK_TARGET_DIR := $(BUILD)/pcb-root-target/$(ROOT_TASK_TARGET)/release
ROOT_TASK_ELF := $(ROOT_TASK_TARGET_DIR)/velaos-pcb-root.elf
RECIPIENT_ELF := $(ROOT_TASK_TARGET_DIR)/velaos-pcb-recipient.elf
RUST_SEL4_CHECKOUT := $(shell find /root/.cargo/git/checkouts -path '*/crates/sel4-patch-elf/src/lib.rs' -print -quit | sed 's#/crates/sel4-patch-elf/src/lib.rs$$##')
SEL4_KERNEL := $(MICROKIT_SDK)/board/$(BOARD)/$(CONFIG)/elf/sel4.elf
SEL4_CONFIG_JSON := $(SEL4_INCLUDE_DIRS)/kernel/gen_config.json
SEL4_PLATFORM_INFO := $(MICROKIT_SDK)/board/$(BOARD)/$(CONFIG)/platform_gen.json
KERNEL_LOADER_ELF := $(RUST_SEL4_CHECKOUT)/target/aarch64-unknown-none/release/sel4-kernel-loader
ADD_PAYLOAD_BIN := $(RUST_SEL4_CHECKOUT)/target/release/sel4-kernel-loader-add-payload
QEMU_DTB := $(BUILD)/qemu.dtb
PCB_ROOT_IMAGE := $(BUILD)/pcb-root.img

.PHONY: all build run benchmark gate-c run-gate-c clean

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

benchmark: $(LOADER)
	python3 gate_f_timing.py $(LOADER)

gate-c:
	cd pcb_root && SEL4_INCLUDE_DIRS="$(SEL4_INCLUDE_DIRS)" CARGO_TARGET_DIR="../$(BUILD)/pcb-root-target" cargo build --locked --release --target $(ROOT_TASK_TARGET) --bin velaos-pcb-recipient
	cd pcb_root && RECIPIENT_ELF="$(abspath $(RECIPIENT_ELF))" SEL4_INCLUDE_DIRS="$(SEL4_INCLUDE_DIRS)" CARGO_TARGET_DIR="../$(BUILD)/pcb-root-target" cargo build --locked --release --target $(ROOT_TASK_TARGET) --bin velaos-pcb-root
	cargo fetch --locked --manifest-path pcb_root/Cargo.toml
	./patch-payload-helper.sh
	cd "$(RUST_SEL4_CHECKOUT)" && CC=aarch64-linux-gnu-gcc SEL4_KERNEL="$(SEL4_KERNEL)" SEL4_INCLUDE_DIRS="$(SEL4_INCLUDE_DIRS)" SEL4_PLATFORM_INFO="$(SEL4_PLATFORM_INFO)" cargo build --locked -p sel4-kernel-loader --target aarch64-unknown-none --release -Z build-std=core,alloc,compiler_builtins -Z build-std-features=compiler-builtins-mem
	cd "$(RUST_SEL4_CHECKOUT)" && cargo build --locked -p sel4-kernel-loader-add-payload --release
	mkdir -p "$(BUILD)"
	qemu-system-aarch64 -machine virt,virtualization=on,dumpdtb="$(QEMU_DTB)" -cpu cortex-a53 -m size=2G -nic none -display none -S -monitor none -serial none
	"$(ADD_PAYLOAD_BIN)" --sel4-config "$(SEL4_CONFIG_JSON)" --kernel "$(SEL4_KERNEL)" --dtb "$(QEMU_DTB)" --platform-info "$(SEL4_PLATFORM_INFO)" --loader "$(KERNEL_LOADER_ELF)" --app "$(ROOT_TASK_ELF)" --out-file "$(PCB_ROOT_IMAGE)"

run-gate-c: gate-c
	qemu-system-aarch64 -machine virt,virtualization=on -cpu cortex-a53 -m size=2G -nic none -nographic -kernel "$(PCB_ROOT_IMAGE)" -dtb "$(QEMU_DTB)"

clean:
	rm -rf $(BUILD) app/target pcb_root/target
