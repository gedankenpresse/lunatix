#!/bin/bash

qemu-system-riscv64 -s -m 1G \
    -M sifive_u -bios default \
    -nographic \
    -kernel u-boot.bin \
    -device loader,addr=0x84000000,force-raw=on,file="$1"
