# WebWINE WinAPI C samples root Makefile
#
# Usage:
#   make                       # build all samples
#   make all                   # build all samples
#   make one SAMPLE=00_messagebox
#   make run SAMPLE=00_messagebox WINE=wine
#   make clean
#
# Toolchain override examples:
#   make CC=i686-w64-mingw32-gcc
#   make CC=x86_64-w64-mingw32-gcc

ifeq ($(origin CC), default)
CC := i686-w64-mingw32-gcc
endif
WINDRES ?= i686-w64-mingw32-windres
WINE ?= wine
BUILD_DIR := build

SAMPLES := \
	00_messagebox \
	01_window_basic \
	02_gdi_paint \
	03_file_io \
	04_open_file_dialog \
	05_save_file_dialog \
	06_memory \
	07_timers \
	08_keyboard_mouse \
	09_menus \
	10_common_controls \
	11_directdraw_windowed \
	12_resource_dialog

.PHONY: all one run clean list $(SAMPLES)

all: $(BUILD_DIR) $(SAMPLES)

$(BUILD_DIR):
	mkdir -p $(BUILD_DIR)

$(SAMPLES): $(BUILD_DIR)
	$(MAKE) -C $@ CC="$(CC)" WINDRES="$(WINDRES)" OUT="../$(BUILD_DIR)/$@.exe"

one: $(BUILD_DIR)
ifndef SAMPLE
	$(error SAMPLE is required. Example: make one SAMPLE=00_messagebox)
endif
	$(MAKE) -C $(SAMPLE) CC="$(CC)" WINDRES="$(WINDRES)" OUT="../$(BUILD_DIR)/$(SAMPLE).exe"

run: one
	$(WINE) $(BUILD_DIR)/$(SAMPLE).exe

list:
	@printf '%s\n' $(SAMPLES)

clean:
	rm -rf $(BUILD_DIR)
	@for d in $(SAMPLES); do $(MAKE) -C $$d clean >/dev/null 2>&1 || true; done
