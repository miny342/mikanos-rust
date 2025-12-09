#!/bin/sh

qemu-img create -f raw disk.img 200M
mkfs.fat -n 'MIKAN OS' -s 2 -f 2 -R 32 -F 32 disk.img
mmd -i disk.img ::/EFI
mmd -i disk.img ::/EFI/BOOT

mcopy -i disk.img $1 ::/kernel
mcopy -i disk.img target/BOOTX64.EFI ::/EFI/BOOT/BOOTX64.EFI

cd ..
./run_qemu.sh kernel/disk.img

# QEMUの終了コードを確認して、33(=0x10 << 1 | 1)ならtestの都合上正常終了として扱う
if [ $? -eq 33 ] || [ $? -eq 0 ]; then
  exit 0
fi
exit 1
