# Directories and files
BUILD_DIR := build
KERNEL_DIR := kernel
RUST_TARGET := x86_64_kernel/release/libkernel.a
KERNEL_BIN := $(BUILD_DIR)/kernel.bin
KERNEL_ENTRY_OBJ := $(BUILD_DIR)/kernel_entry.o

# Default target
all: $(KERNEL_ENTRY_OBJ) $(KERNEL_BIN)

# Build the Rust kernel and move the output to the build directory
$(KERNEL_BIN): $(KERNEL_ENTRY_OBJ)
	cd $(KERNEL_DIR) && cargo build --release && cd ..
	ld.lld -o $(KERNEL_BIN) -Ttext 0x8200 $(KERNEL_ENTRY_OBJ) $(KERNEL_DIR)/target/$(RUST_TARGET)  --oformat binary

# Assemble the kernel entry and place the output in the build directory
$(KERNEL_ENTRY_OBJ):
	nasm $(KERNEL_DIR)/kernel_entry.asm -f elf64 -o $(KERNEL_ENTRY_OBJ)

# Clean up build files
clean:
	rm -f $(BUILD_DIR)/*

# Phony targets
.PHONY: all clean
