import { dag } from "../sdk/client.gen.ts";
import { Directory, DirectoryID } from "../deps.ts";

export const getDirectory = async (
  src: string | Directory | undefined = ".",
) => {
  if (src instanceof Directory) {
    return src;
  }
  if (typeof src === "string") {
    try {
      const directory = dag.loadDirectoryFromID(src as DirectoryID);
      await directory.id();
      return directory;
    } catch (_) {
      return dag.host
        ? dag.host().directory(src)
        : dag.currentModule().source().directory(src);
    }
  }
  return dag.host
    ? dag.host().directory(src)
    : dag.currentModule().source().directory(src);
};

export function buildRustFlags(): string {
  let rustflags = "";
  switch (Deno.env.get("TARGET")) {
    case "aarch64-unknown-linux-gnu":
      rustflags = `-Clink-arg=-lsystemd \
        -Clink-arg=-lcap \
        -Clink-arg=-lgcrypt \
        -Clink-arg=-lgpg-error \
        -Clink-arg=-llz4 \
        -Clink-arg=-llzma \
        -Clink-arg=-lpsx \
        -Clink-arg=-lxxhash \
        -Clink-arg=-lzstd \
        -C linker=aarch64-linux-gnu-gcc \
        -L/usr/aarch64-linux-gnu/lib \
        -L/build/sysroot/usr/lib/aarch64-linux-gnu \
        -L/build/sysroot/lib/aarch64-linux-gnu`;
      break;
    case "armv7-unknown-linux-gnueabihf":
      rustflags = `-Clink-arg=-lsystemd \
        -Clink-arg=-lcap \
        -Clink-arg=-lgcrypt \
        -Clink-arg=-lgpg-error \
        -Clink-arg=-llz4 \
        -Clink-arg=-llzma \
        -Clink-arg=-lpsx \
        -Clink-arg=-lxxhash \
        -Clink-arg=-lzstd \
        -C linker=arm-linux-gnueabihf-gcc \
        -L/usr/arm-linux-gnueabihf/lib \
        -L/build/sysroot/usr/lib/arm-linux-gnueabihf \
        -L/build/sysroot/lib/arm-linux-gnueabihf`;
      break;
    default:
      break;
  }
  return rustflags;
}
