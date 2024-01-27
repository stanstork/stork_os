# Directories and files
BUILD_DIR := build
KERNEL_DIR := kernel
RUST_TARGET := x86_64_kernel/release/libkernel.a
KERNEL_BIN := $(BUILD_DIR)/kernel.bin
ASM_SRC = $(shell find kernel -type f -name "*.asm")
OBJ_FILES := $(ASM_SRC:.asm=.o)

# Default target
all: kernel

# Build the Rust kernel and move the output to the build directory
kernel: ${OBJ_FILES}
	cd $(KERNEL_DIR) && cargo build --release && cd ..
	ld.lld -o $(KERNEL_BIN) -Ttext 0x8200 $^ $(KERNEL_DIR)/target/$(RUST_TARGET) --oformat binary
	rm -f $(OBJ_FILES)


%.o: %.asm
	nasm $< -f elf64 -o $@

# Clean up build files
clean:
	rm -f $(BUILD_DIR)/*

# Phony targets
.PHONY: all clean
