## TODOs

### Kernel

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
- [ ] implement destroy for more caps (maybe add a simple drop to CapCounted?)
- [x] implement copy for more caps
- [ ] cleanup documentation of syscalls
- [ ] use unique syscall labels for capabilities (maybe add some simple name hashing?)
- [ ] recursive caddr lookup
- [ ] update syscall_abi cap variants to include all caps

### Userspace
- [ ] add global alloc
- [ ] flesh out scheduler so that a process is executed multiple times
- [ ] add simple virtio file system driver
- [ ] load binaries from files
- [ ] render to screen (pci?)
