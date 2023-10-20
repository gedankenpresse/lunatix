#!/bin/bash

D=$(realpath $(dirname $0))

qemu-system-riscv64 -s -m 1G \
    -M virt -bios default \
    -nographic \
    -kernel u-boot.bin \
    -fsdev local,security_model=mapped-xattr,id=guest_root,readonly,path=$D/guest_root \
    -device virtio-9p-device,fsdev=guest_root,mount_tag=/ \
    -device loader,addr=0x84000000,force-raw=on,file="$1" \
    -device loader,addr=0x84800000,force-raw=on,file="$2"
