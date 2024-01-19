# Directories and files
BUILD_DIR := build
KERNEL_DIR := kernel
BOOT_DIR := boot
RUST_TARGET := i686-kernel/debug/kernel
KERNEL_BIN := $(BUILD_DIR)/kernel.bin
KERNEL_ENTRY_OBJ := $(BUILD_DIR)/kernel_entry.o
BOOTSECT_BIN := $(BUILD_DIR)/bootsect.bin
OS_IMAGE := $(BUILD_DIR)/os-image.bin

# Default target
all: $(OS_IMAGE)

# Build the Rust kernel and move the output to the build directory
$(KERNEL_BIN): $(KERNEL_ENTRY_OBJ)
	cd $(KERNEL_DIR) && cargo build
	ld -melf_i386 -o $(KERNEL_BIN) -Ttext 0x1000 $(KERNEL_ENTRY_OBJ) $(KERNEL_DIR)/target/$(RUST_TARGET)  --oformat binary

# Assemble the kernel entry and place the output in the build directory
$(KERNEL_ENTRY_OBJ):
	nasm $(BOOT_DIR)/kernel_entry.asm -f elf -o $(KERNEL_ENTRY_OBJ)

# Assemble the bootloader and place the output in the build directory
$(BOOTSECT_BIN):
	nasm $(BOOT_DIR)/bootsect.asm -f bin -o $(BOOTSECT_BIN)

# Create the OS image by concatenating the bootloader and kernel and place it in the build directory
$(OS_IMAGE): $(BOOTSECT_BIN) $(KERNEL_BIN)
	cat $(BOOTSECT_BIN) $(KERNEL_BIN) > $(OS_IMAGE)

# Run the operating system using QEMU
run: $(OS_IMAGE)
	qemu-system-i386 -fda $(OS_IMAGE)

# Clean up build files
clean:
	rm -f $(BUILD_DIR)/*

# Phony targets
.PHONY: all clean run
