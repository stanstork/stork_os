BUILD_DIR         := build
KERNEL_DIR        := kernel
BOOTLOADER_DIR	  := boot/uefi

KERNEL_BINARY     := ${BUILD_DIR}/kernel.elf
BOOTLOADER_BINARY := ${BUILD_DIR}/bootx64.efi

DISK_IMG          := ${BUILD_DIR}/kernel.img
DISK_IMG_SIZE     := 2880

QEMU_FLAGS :=                                                \
	-bios ovmf                                            \
	-drive if=none,id=uas-disk1,file=${DISK_IMG},format=raw    \
	-device usb-storage,drive=uas-disk1                        \
	-serial stdio                                              \
	-usb                                                       \
	-net none                                                  \
	-vga std \
	-d int

.PHONY: all clean emu

all: ${DISK_IMG}

bootloader: ${BOOTLOADER_BINARY}

debug: ${DISK_IMG}
	qemu-system-x86_64    \
		${QEMU_FLAGS}       \
		-S                  \
		-gdb tcp::1234

emu: ${DISK_IMG}
	qemu-system-x86_64    \
		${QEMU_FLAGS}

kernel: ${KERNEL_BINARY}

${DISK_IMG}: ${BUILD_DIR} ${KERNEL_BINARY} ${BOOTLOADER_BINARY} 
	# Create UEFI boot disk image in DOS format.
	dd if=/dev/zero of=${DISK_IMG} bs=512 count=93750
	mformat -i ${DISK_IMG} ::
	mmd -i ${DISK_IMG} ::/EFI
	mmd -i ${DISK_IMG} ::/EFI/BOOT
	# Copy the bootloader to the boot partition.
	mcopy -i ${DISK_IMG} ${BOOTLOADER_BINARY} ::/efi/boot/bootx64.efi
	mcopy -i ${DISK_IMG} assets/fonts/zap-light16.psf ::/zap-light16.psf
	mcopy -i ${DISK_IMG} ${KERNEL_BINARY} ::/kernel.elf

${BOOTLOADER_BINARY}:
	make -C ${BOOTLOADER_DIR}

${BUILD_DIR}:
	mkdir -p ${BUILD_DIR}

${KERNEL_BINARY}:
	make -C ${KERNEL_DIR}

clean:
	rm -rf ${BUILD_DIR}/*
