#!/bin/sh
cd $HOME/mikanos-rust/bootloader

if [ "$#" -gt 0 ]
then
    cargo build --release
else
    cargo build
fi
