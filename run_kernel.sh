#!/bin/bash

qemu-system-riscv64 -s -m 1G \
    -M virt -bios default \
    -nographic \
    -kernel u-boot.bin \
    -device loader,addr=0x84000000,force-raw=on,file="$1" \
    -device loader,addr=0x84800000,force-raw=on,file="$2"
