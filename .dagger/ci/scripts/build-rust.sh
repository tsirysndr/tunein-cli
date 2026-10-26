#!/usr/bin/env bash
set -euo pipefail
case "$TARGET" in
  x86_64-unknown-linux-gnu) triple= ;;
  aarch64-unknown-linux-gnu) triple=aarch64-linux-gnu ;;
  armv7-unknown-linux-gnueabihf) triple=arm-linux-gnueabihf ;;
  *) exit 1 ;;
esac
if [[ -n "$triple" ]]; then
  export PKG_CONFIG_ALLOW_CROSS=1
  export PKG_CONFIG_LIBDIR="/build/sysroot/usr/lib/$triple/pkgconfig:/build/sysroot/usr/share/pkgconfig"
  export PKG_CONFIG_SYSROOT_DIR=/build/sysroot
  export C_INCLUDE_PATH=/build/sysroot/usr/include
  export RUSTFLAGS="-Clink-arg=-lsystemd -Clink-arg=-lcap -Clink-arg=-lgcrypt -Clink-arg=-lgpg-error -Clink-arg=-llz4 -Clink-arg=-llzma -Clink-arg=-lpsx -Clink-arg=-lxxhash -Clink-arg=-lzstd -C linker=$triple-gcc -L/usr/$triple/lib -L/build/sysroot/usr/lib/$triple -L/build/sysroot/lib/$triple -Clink-arg=-Wl,-rpath-link,/build/sysroot/usr/lib/$triple -Clink-arg=-Wl,-rpath-link,/build/sysroot/lib/$triple -Clink-arg=-Wl,-rpath-link,/usr/$triple/lib"
fi
# Older tags rely on this wrapper for proto3 optional fields.
mv /usr/bin/protoc /usr/bin/_protoc
install -m 0755 tools/protoc /usr/bin/protoc
cargo build --release --locked --target "$TARGET" --bin tunein
