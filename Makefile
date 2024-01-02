# Building
EXNAME := os
TARGET := riscv64gc-unknown-none-elf
MODE := release
KERNEL_ELF := target/$(TARGET)/$(MODE)/os
KERNEL_BIN := $(KERNEL_ELF).bin

# BOARD
BOARD ?= qemu
SBI ?= rustsbi
BOARD_BIOS := ../bootloader/$(SBI)-$(BOARD).bin



# LOG
LOG ?= info

# SetGdb:
GdbIns := riscv64-unknown-elf-gdb -ex $(KERNEL_ELF) -ex 'set arch riscv:rv64' -ex 'target remote localhost:1234'


# SetQemu:
# QemuIns := qemu-system-riscv64 \                                        
# 		-machine virt \
# 		-nographic \
# 		-bios $(BOARD_BIOS)  \
# 		-device loader,file=$(KERNEL_BIN),addr=0x80200000 \
#		-s -S

QemuIns := qemu-system-riscv64 -machine virt -nographic -bios $(BOARD_BIOS) -device loader,file=$(KERNEL_BIN),addr=0x80200000 -s -S


run:
	cargo build --release
	rust-objcopy --strip-all $(KERNEL_ELF) -O binary $(KERNEL_BIN)
	gnome-terminal -- bash -c "$(GdbIns); exec bash"
	$(QemuIns)






