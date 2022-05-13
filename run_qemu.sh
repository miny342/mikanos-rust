#!/bin/sh
qemu-system-x86_64 -drive if=pflash,file=./lib/OVMF_CODE.fd -drive if=pflash,file=./lib/OVMF_VARS.fd -hda disk.img -monitor stdio
