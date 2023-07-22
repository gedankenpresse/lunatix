#!/bin/bash


echo $0

BASEDIR=$(dirname "$0")
echo basedir "$BASEDIR"

UBOOT="$BASEDIR/u-boot.bin"
LOADER="$BASEDIR/$1"
KERNEL="$2"

qemu-system-riscv64 -s -m 1G \
    -M virt -bios default \
    -nographic \
    -kernel "$UBOOT" \
    -device loader,addr=0x84000000,force-raw=on,file="$LOADER" \
    -device loader,addr=0x84800000,force-raw=on,file="$KERNEL"
