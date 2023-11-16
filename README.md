## Setup

- Install rust. Use whichever method you like; it is well documented.

- Install qemu:
  ```shell
  pacman -S qemu-emulators-full
  ```

- Compile our code:
  ```shell
  make all
  ```

- Run the OS:
  ```shell
  cargo run
  ```

## TODOs

### Kernel

General Stuff:
- [x] update syscall_abi cap variants to include all caps
- [x] implement copy for more caps
- [x] BUG: fix kernel allocator. Somehow, we have a huge range in kernel loader (0x80040000 .. 0xc0000000), but in kernel there's not much left: (0x80040000..0x80099000)
- [x] implement destroy for more caps (maybe add a simple drop to CapCounted?)
- [ ] Change Memory Allocator to use Page Alloc.
      Currently, we use the bump allocator, so creating and destroying a single page repeatedly will consume all memory.
- [ ] Don't map intermediate page tables automatically.
- [ ] Refactor cursors so that we don't need to keep all the intermediate objects
- [ ] Implement IPC calls, i.e. Endpoints (with cap transfer)
- [ ] add some TLS and save the hart/context id.
- [ ] figure out which context we should enable in PLIC for interrupts
- [ ] improve booting, pass init as boot arg
- [ ] change kernel device tree lib?
- [ ] add PCI to dev memory
- [ ] cleanup documentation of syscalls
- [ ] use unique syscall labels for capabilities (maybe add some simple name hashing?)
- [ ] recursive caddr lookup
- [ ] BUG: kernel should refuse to map addresses that have the 39th vaddress bit set to 1
- [ ] BUG: kernel loader can't load debug kernel: panicked at 'range end index 8636784 out of range for slice of length 8388608', /3/xmas-elf-0.8.0/src/sections.rs:38:57
- [ ] move device tree to top of init virtual memory

Destroy related tasks:
- [x] implement destroy for more caps (maybe add a simple drop to CapCounted?)
- [ ] cspace: destroy slots
- [ ] devmem: destroy state correctly. (destroy child pages on drop? leave state global?)
- [ ] irqControl: destroy state correctly. (maybe don't allocate state, but keep as global?)
- [ ] irq: destroy state (Notification) on Irq destroy
- [ ] memory: destroy children
- [ ] vspace: cleanup asid stuff on destroy
- [ ] notification: signal waitset on destroy
- [ ] task: signal waitset on destroy

### Userspace
- [x] add simple virtio file system driver
- [x] BUG: virtio: free descriptors after usage
- [x] add global alloc
- [x] load binaries from files
- [x] render to screen (virtio)
- [x] add allocation logic for address mapping (welche addresse hardcode ich heute?)
- [x] read from virtio human input device
- [ ] better keyboard layouts
- [ ] read keyboard layout from file
- [ ] flesh out scheduler so that a process is executed multiple times
- [ ] document (and maybe reorganize) address space layout
- [ ] VirtIO from device tree
