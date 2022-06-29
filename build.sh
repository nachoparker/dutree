#!/bin/sh

cargo install cross

echo "Building... Linux ARM64"
cross build --release --target aarch64-unknown-linux-gnu
mv target/aarch64-unknown-linux-gnu/release/dutree ./dutree_linux_arm64

echo "Building... Linux ARMv7"
cross build --release --target armv7-unknown-linux-gnueabihf
mv target/armv7-unknown-linux-gnueabihf/release/dutree ./dutree_linux_arm7

echo "Building... Linux x86_64"
cross build --release --target x86_64-unknown-linux-gnu
mv target/x86_64-unknown-linux-gnu/release/dutree ./dutree_linux_x86_64

echo "Done"
