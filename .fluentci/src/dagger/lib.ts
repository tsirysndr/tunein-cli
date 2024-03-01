export function buildRustFlags(): string {
  let rustflags = "";
  switch (Deno.env.get("TARGET")) {
    case "aarch64-unknown-linux-gnu":
      rustflags = `-C linker=aarch64-linux-gnu-gcc \
        -L/usr/aarch64-linux-gnu/lib \
        -L/build/sysroot/usr/lib/aarch64-linux-gnu \
        -L/build/sysroot/lib/aarch64-linux-gnu`;
      break;
    case "armv7-unknown-linux-gnueabihf":
      rustflags = `-C linker=arm-linux-gnueabihf-gcc \
        -L/usr/arm-linux-gnueabihf/lib \
        -L/build/sysroot/usr/lib/arm-linux-gnueabihf \
        -L/build/sysroot/lib/arm-linux-gnueabihf`;
      break;
    default:
      break;
  }
  return rustflags;
}
