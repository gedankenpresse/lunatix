.PHONY = all kernel apps clean target/

#
# Phony targets
#

all: kernel apps u-boot/u-boot.bin qemu_virt.dtb.txt qemu_sifive_u.dtb.txt

kernel: target/riscv64imac-unknown-none-elf/debug/kernel target/riscv64imac-unknown-none-elf/release/kernel_loader

apps: guest_root/hello_world guest_root/walk_cspace guest_root/echo_srv guest_root/echo_client

drivers: guest_root/drivers/uart

clean:
	rm -f guest_root/hello_world
	rm -f *.dtb *.dtb.txt
	rm -rf target
# make -C u-boot clean


#
# Basic Targets
#

guest_root/drivers/% : target/riscv64imac-unknown-none-elf/release/%_driver
	mkdir -p guest_root/drivers
	cp $< $@

guest_root/% : target/riscv64imac-unknown-none-elf/release/%
	cp $< $@

u-boot/u-boot.bin:
	make -C u-boot -E "ARCH=riscv" -E "CROSS_COMPILE=riscv64-linux-gnu-" qemu-riscv64_smode_defconfig u-boot.bin

qemu_%.dtb:
	@qemu-system-riscv64 -machine $* -machine dumpdtb=qemu_$*.dtb

%.dtb.txt: %.dtb
	dtc -I dtb -O dts $< > $@



#
# Rust crates
#

target/riscv64imac-unknown-none-elf/release/%: FORCE
	cargo build --release -p $*

target/riscv64imac-unknown-none-elf/debug/%: FORCE
	cargo build -p $*

target/riscv64imac-unknown-none-elf/debug/kernel: FORCE target/riscv64imac-unknown-none-elf/release/stage0_init
	cargo build -p kernel


#
# Helpers
#
FORCE:
