#!/bin/sh
$HOME/qemu-6.2.0/build/qemu-system-x86_64 \
    -m 1G \
    -drive if=pflash,file=./lib/OVMF_CODE.fd \
    -drive if=pflash,file=./lib/OVMF_VARS.fd \
    -hda disk.img \
    -monitor stdio \
    -device nec-usb-xhci,id=xhci \
    -device usb-mouse -device usb-kbd \
    -drive if=none,file=disk1.img,id=usb0 \
    -device usb-storage,drive=usb0,bus=xhci.0 \
