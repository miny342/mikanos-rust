#!/bin/sh
if [ -e /dev/kvm ]; then
  echo kvm OK
  KVM="-enable-kvm -cpu host"
else
  echo kvm NG
fi

# WSLg環境下では、waylandで起動する場合画面上半分でカーソルが動かなくなる
# 実際の環境に近づけるためUSBからBootする
GDK_BACKEND=x11 qemu-system-x86_64 \
    ${KVM} \
    -m 1G \
    -drive if=pflash,format=raw,readonly=on,file=./lib/OVMF_CODE.fd \
    -drive if=pflash,format=raw,readonly=on,file=./lib/OVMF_VARS.fd \
    -drive file=disk.img,format=raw,if=none,id=stick \
    -serial stdio \
    -device nec-usb-xhci \
    -device usb-kbd \
    -device usb-mouse \
    -device usb-storage,drive=stick
