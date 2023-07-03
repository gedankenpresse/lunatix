#!/bin/bash

qemu-system-riscv64 -s -m 1G \
    -M virt -bios default \
    -nographic \
    -S \
    -kernel u-boot.bin \
    -device loader,addr=0x84000000,force-raw=on,file="$1"
