MICROKIT_SDK ?= /microkit
# Modern SDK board IDs encode physical RAM size. "rpi4b" is not valid.
MICROKIT_BOARD ?= rpi4b_2gb
MICROKIT_CONFIG ?= release
ROOT := $(abspath ../../..)
BUILD := $(ROOT)/build/rpi4/$(MICROKIT_BOARD)/$(MICROKIT_CONFIG)
APP := $(ROOT)/poc/m5_executor/app
SDK_INCLUDES := $(MICROKIT_SDK)/board/$(MICROKIT_BOARD)/$(MICROKIT_CONFIG)/include
TARGET := aarch64-sel4-microkit

.PHONY: all check-sdk domains
all: $(BUILD)/vela-rpi4.img
check-sdk:
	@test -d "$(SDK_INCLUDES)" || (echo "Missing SDK board/config: $(MICROKIT_BOARD)/$(MICROKIT_CONFIG)"; exit 1)
	@test "$(MICROKIT_CONFIG)" = release || (echo "Use release: kernel debug UART must not share the isolated driver"; exit 1)
domains: check-sdk
	mkdir -p "$(BUILD)"
	cd "$(APP)" && SEL4_INCLUDE_DIRS="$(SDK_INCLUDES)" CARGO_TARGET_DIR="$(BUILD)/cargo" cargo build --locked --release --bin vela-rpi4-uart --bin vela-rpi4-video
	cp "$(BUILD)/cargo/$(TARGET)/release/vela-rpi4-uart.elf" "$(BUILD)/uart.elf"
	cp "$(BUILD)/cargo/$(TARGET)/release/vela-rpi4-video.elf" "$(BUILD)/video.elf"
	python3 "$(ROOT)/poc/m5_executor/check-wx.py" "$(BUILD)/uart.elf"
	python3 "$(ROOT)/poc/m5_executor/check-wx.py" "$(BUILD)/video.elf"
$(BUILD)/vela-rpi4.img: vela-rpi4.system uart.rs video.rs domains
	"$(MICROKIT_SDK)/bin/microkit" vela-rpi4.system --search-path "$(BUILD)" --board "$(MICROKIT_BOARD)" --config "$(MICROKIT_CONFIG)" --output "$@" --report "$(BUILD)/report.txt"
# ARM output is a loader raw binary, not a partitioned MicroSD installer.
# U-Boot, after board firmware setup:
# fatload mmc 0 0x10000000 vela-rpi4.img
# go 0x10000000
