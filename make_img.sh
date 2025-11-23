#!/bin/sh

if [ "$#" -gt 0 ]
then
    cd bootloader
    cargo build --release
    cd ../kernel
    cargo build --release
else
    cd bootloader
    cargo build
    cd ../kernel
    cargo build
fi
cd ..

qemu-img create -f raw disk.img 200M
mkfs.fat -n 'MIKAN OS' -s 2 -f 2 -R 32 -F 32 disk.img
mkdir -p mnt/EFI/BOOT
mmd -i disk.img ::/EFI
mmd -i disk.img ::/EFI/BOOT

if [ "$#" -gt 0 ]
then
    mcopy -i disk.img kernel/target/x86_64-unknown-none/release/kernel ::/kernel
    mcopy -i disk.img bootloader/target/x86_64-unknown-uefi/release/mikanos-rust.efi ::/EFI/BOOT/BOOTX64.EFI
else
    mcopy -i disk.img kernel/target/x86_64-unknown-none/debug/kernel ::/kernel
    mcopy -i disk.img bootloader/target/x86_64-unknown-uefi/debug/mikanos-rust.efi ::/EFI/BOOT/BOOTX64.EFI
fi
