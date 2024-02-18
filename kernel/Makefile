# Directories and files
BUILD_DIR := ../build
RUST_TARGET := target/x86_64_kernel/release/libkernel.a
KERNEL_ELF := kernel.elf

# Default target
all: kernel 

# Build the Rust kernel and move the output to the build directory
kernel:
	cargo build --release
	ld.lld -o $(BUILD_DIR)/$(KERNEL_ELF) -T link.ld $(RUST_TARGET)

# Clean up build files
clean:
	rm -f $(BUILD_DIR)/$(KERNEL_ELF)

# Phony targets to handle non-file targets
.PHONY: all clean
