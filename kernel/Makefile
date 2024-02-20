# Directories and files
ASM_DIR := ../asm
BUILD_DIR := ../build
RUST_TARGET := target/x86_64_kernel/release/libkernel.a
KERNEL_ELF := kernel.elf

# Default target
all: kernel 

# Build the Rust kernel and move the output to the build directory
kernel: gdt
	cargo build --release
	ld.lld -o $(BUILD_DIR)/$(KERNEL_ELF) -T link.ld $(RUST_TARGET) $(BUILD_DIR)/gdt_flush.o

# Clean up build files
clean:
	rm -f $(BUILD_DIR)/$(KERNEL_ELF)

# Compile the GDT flush function
gdt:
	nasm -f elf64 $(ASM_DIR)/gdt_flush.asm -o $(BUILD_DIR)/gdt_flush.o

# Phony targets to handle non-file targets
.PHONY: all clean
