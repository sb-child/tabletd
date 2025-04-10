#!/bin/sh

sudo dmesg -C
echo 0x19F | sudo tee /sys/module/drm/parameters/debug
./target/debug/tabletd
echo 0 | sudo tee /sys/module/drm/parameters/debug
