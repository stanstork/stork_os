# TARGET_NAME := simple_app
# RUST_TARGET := x86_64-unknown-none
# SRC_DIR := src
# OUT_DIR := target
# MODE := release
# RUSTC := cargo
# BUILD_CMD := $(RUSTC) build --release --target $(RUST_TARGET)
# ELF_FILE := $(OUT_DIR)/$(RUST_TARGET)/$(MODE)/$(TARGET_NAME)
# BUILD_DIR := ../../build
# LINKER := ld.lld
# LINKER_SCRIPT := link.ld

# # Default target
# all: build

# # Build the ELF file
# build:
# 	@echo "Building ELF file for target: $(RUST_TARGET)"
# 	@RUSTFLAGS="-C link-arg=-T$(LINKER_SCRIPT)" $(BUILD_CMD)
# 	cp $(ELF_FILE) $(BUILD_DIR)/$(TARGET_NAME)

# # Inspect the ELF file
# inspect: build
# 	@echo "Inspecting ELF file: $(ELF_FILE)"
# 	readelf -h $(ELF_FILE)

# Name of the executable
TARGET = simple_app

# Rust compiler options
RUSTFLAGS = -C panic=abort -C target-cpu=x86-64 -C relocation-model=static -C link-args="-nostartfiles -nostdlib"

# Rule to build the executable
$(TARGET): src/main.rs
	rustc $(RUSTFLAGS) src/main.rs -o $(TARGET)
	mv $(TARGET) ../../build

# Clean rule to remove the executable
clean:
	rm -f $(TARGET)

# Default rule to build the executable
all: $(TARGET)
