#!/bin/bash

export ARCH=riscv
export CROSS_COMPILE=riscv64-linux-gnu-

cd u-boot;
make qemu-riscv64_smode_defconfig
make
