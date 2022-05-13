#!/bin/sh

./bootloader/build.sh $1
if [ "$#" -gt 0 ]
then
    ./kernel/build.sh $1
else
    ./kernel/build.sh
fi

qemu-img create -f raw disk.img 200M
mkfs.fat -n 'MIKAN OS Rust' -s 2 -f 2 -R 32 -F 32 disk.img
mkdir -p mnt
sudo mount -o loop disk.img mnt
sudo mkdir -p mnt/EFI/BOOT
sudo cp bootloader/target/x86_64-unknown-uefi/release/mikanos-rust.efi mnt/EFI/BOOT/BOOTX64.EFI

if [ "$#" -gt 0 ]
then
    sudo cp kernel/target/x86_64-unknown-none/release/kernel mnt/kernel
else
    sudo cp kernel/target/x86_64-unknown-none/debug/kernel mnt/kernel
fi

sudo umount mnt

