TARGET = x86_64-unknown-none
APP_NAME = simple_app
OUT_DIR = ../../build
APP_BIN = $(OUT_DIR)/$(APP_NAME).elf

all: $(APP_BIN)

# Rule to compile the Rust application
$(APP_BIN): src/main.rs
	rustup target add $(TARGET)
	cargo build --release --target $(TARGET)
	cp target/$(TARGET)/release/$(APP_NAME) $(APP_BIN)

clean:
	rm -rf $(OUT_DIR)

.PHONY: all clean
