# Name of the executable
TARGET = demo_app

# Rust compiler options
RUSTFLAGS = -C panic=abort -C target-cpu=x86-64 -C relocation-model=static -C link-args="-nostartfiles -nostdlib"

# Rule to build the executable
$(TARGET): src/main.rs
	rustc $(RUSTFLAGS) src/main.rs -o $(TARGET)
	mv $(TARGET) ../../build

# Default rule to build the executable
all: $(TARGET)
