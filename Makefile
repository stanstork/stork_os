# Directories and files
BUILD_DIR := build
KERNEL_DIR := kernel
ASM_DIR := asm
RUST_TARGET := target/x86_64_kernel/release/libkernel.a
KERNEL_BIN := kernel.bin
BOOT_BIN := boot.bin
OS_IMAGE := os-image.bin

# Default target
all: os 

# Create OS image by concatenating boot sector and kernel binaries
os: kernel
	cd $(BUILD_DIR) && cat $(BOOT_BIN) $(KERNEL_BIN) > $(OS_IMAGE)

# Build the Rust kernel and move the output to the build directory
kernel: kernel_entry
	cd $(KERNEL_DIR) && cargo build --release && cd ..
	ld.lld -o $(BUILD_DIR)/$(KERNEL_BIN) -Ttext 0x8200 $(BUILD_DIR)/kernel_entry.o $(BUILD_DIR)/gdt_flush.o $(KERNEL_DIR)/$(RUST_TARGET) --oformat binary

# Compile the kernel entry point
kernel_entry: boot gdt
	nasm -f elf64 $(ASM_DIR)/kernel_entry.asm -o $(BUILD_DIR)/kernel_entry.o

# Compile the GDT flush function
gdt:
	nasm -f elf64 $(ASM_DIR)/gdt_flush.asm -o $(BUILD_DIR)/gdt_flush.o

# Compile the boot sector
boot:
	cd $(ASM_DIR)/bootsector && nasm -f bin boot.asm -o ../../$(BUILD_DIR)/boot.bin

# Clean up build files
clean:
	rm -f $(BUILD_DIR)/*

# Phony targets to handle non-file targets
.PHONY: all clean os kernel kernel_entry boot
