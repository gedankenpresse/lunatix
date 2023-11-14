#!/bin/bash

D=$(realpath $(dirname $0))

qemu-system-riscv64 -s -m 1G \
    -M virt -bios default \
    -serial stdio \
    -kernel u-boot/u-boot.bin \
    -fsdev local,security_model=mapped-xattr,id=guest_root,readonly=on,path=$D/guest_root \
    -device virtio-9p-device,fsdev=guest_root,mount_tag=/ \
    -device virtio-gpu-device \
    -device virtio-keyboard-device \
    -device loader,addr=0x84000000,force-raw=on,file="$1" \
    -device loader,addr=0x84800000,force-raw=on,file="$2"
#    -d guest_errors,trace:cpu_halt,trace:cpu_unhalt,trace:virtio_irq,trace:virtio_set_status,trace:virtio_notify,trace:virtio_queue_notify,trace:virtio_gpu_cmd_res_back_attach,trace:virtio_gpu_cmd_get_display_info,trace:virtio_gpu_cmd_res_create_2d \
