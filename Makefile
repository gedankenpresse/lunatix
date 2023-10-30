.PHONY = all kernel apps clean target/

#
# Phony targets
#

all: kernel apps u-boot/u-boot.bin

kernel: target/riscv64imac-unknown-none-elf/debug/kernel target/riscv64imac-unknown-none-elf/release/kernel_loader

apps: guest_root/hello_world

clean:
	rm -f guest_root/hello_world
	rm -rf target
	make -C u-boot clean


#
# Basic Targets
#

guest_root/% : target/riscv64imac-unknown-none-elf/release/%
	cp $< $@

u-boot/u-boot.bin:
	make -C u-boot -E "ARCH=riscv" -E "CROSS_COMPILE=riscv64-linux-gnu-" qemu-riscv64_smode_defconfig u-boot.bin



#
# Rust crates
#

target/riscv64imac-unknown-none-elf/release/%: FORCE
	cargo build --release -p $*

target/riscv64imac-unknown-none-elf/debug/%: FORCE
	cargo build -p $*

target/riscv64imac-unknown-none-elf/debug/kernel: FORCE target/riscv64imac-unknown-none-elf/release/init
	cargo build -p kernel


#
# Helpers
#
FORCE:
