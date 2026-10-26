#!/usr/bin/env bash
set -euo pipefail
case "$TARGET" in
  x86_64-unknown-linux-gnu) arch= ;;
  aarch64-unknown-linux-gnu) arch=arm64 ;;
  armv7-unknown-linux-gnueabihf) arch=armhf ;;
  *) exit 1 ;;
esac
[[ -z "$arch" ]] || dpkg --add-architecture "$arch"
apt-get update
apt-get install -y --no-install-recommends build-essential libasound2-dev libdbus-1-dev protobuf-compiler pkg-config
if [[ -n "$arch" ]]; then
  case "$arch" in
    arm64) triple=aarch64-linux-gnu ;;
    armhf) triple=arm-linux-gnueabihf ;;
  esac
  apt-get install -y --no-install-recommends "gcc-$triple" "libc6-dev-$arch-cross" symlinks
  mkdir -p /build/sysroot /tmp/debs
  cd /tmp/debs
  packages=(libasound2 libasound2-dev libdbus-1-dev libdbus-1-3 libsystemd-dev libsystemd0 libcap2 libcap-dev libgcrypt20 libgcrypt20-dev libgpg-error0 libgpg-error-dev liblz4-1 liblz4-dev libxxhash0 libxxhash-dev liblzma5 liblzma-dev libzstd1 libzstd-dev)
  for package in "${packages[@]}"; do apt-get download "$package:$arch"; done
  for package in ./*.deb; do dpkg -x "$package" /build/sysroot; done
  symlinks -cr /build/sysroot
fi
rm -rf /var/lib/apt/lists/* /tmp/debs
