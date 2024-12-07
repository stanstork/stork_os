# Variables
APP_NAME = demo_app

# Default target: build the application
all: build-app

# Build the user application
build-app:
	@echo "Building application $(APP_NAME)..."
	cargo build --release
	@echo "Copying the application to the build directory..."
	cp target/x86_64-stork_os/release/$(APP_NAME) ../../build