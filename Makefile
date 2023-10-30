.PHONY = all kernel apps clean target/

#
# Phony targets
#

all: kernel apps

kernel: target/riscv64imac-unknown-none-elf/debug/kernel target/riscv64imac-unknown-none-elf/release/kernel_loader

apps: guest_root/hello_world

clean:
	rm -f guest_root/hello_world
	rm -rf target



#
# Apps placed inside guest_root
#

guest_root/% : target/riscv64imac-unknown-none-elf/release/%
	cp $< $@



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
