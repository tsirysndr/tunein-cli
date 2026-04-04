import { dag } from "../sdk/client.gen.ts";
import { buildRustFlags, getDirectory } from "./lib.ts";

export enum Job {
  test = "test",
  build = "build",
}

export const exclude = ["target", ".git", ".devbox", ".fluentci"];

export const test = async (src = ".", options: string[] = []) => {
  const context = await getDirectory(src);
  const ctr = dag
    .container()
    .from("rust:1.89-bullseye")
    .withDirectory("/app", context, { exclude })
    .withWorkdir("/app")
    .withMountedCache("/app/target", dag.cacheVolume("target"))
    .withMountedCache("/root/cargo/registry", dag.cacheVolume("registry"))
    .withExec(["cargo", "test", ...options]);

  return ctr.stdout();
};

// Helper: extract a downloaded .deb by package base-name and arch into a sysroot.
// Uses `find` so the exact version/epoch in the filename doesn't matter.
function dpkgExtract(pkg: string, arch: string, dest: string): string[] {
  return [
    "sh",
    "-c",
    `deb=$(find /tmp/debs -maxdepth 1 -name '${pkg}_*_${arch}.deb' | head -1) && [ -n "$deb" ] && dpkg -x "$deb" ${dest} || (echo "ERROR: could not find ${pkg}_*_${arch}.deb" && exit 1)`,
  ];
}

export const build = async (src = ".") => {
  const rustflags = buildRustFlags();
  const context = await getDirectory(src);
  const ctr = dag
    .container()
    .from("rust:1.89-bullseye")
    .withExec(["dpkg", "--add-architecture", "armhf"])
    .withExec(["dpkg", "--add-architecture", "arm64"])
    .withExec(["apt-get", "update"])
    .withExec([
      "apt-get",
      "install",
      "-y",
      "build-essential",
      "libasound2-dev",
      "protobuf-compiler",
    ])
    .withExec([
      "apt-get",
      "install",
      "-y",
      "-qq",
      "gcc-arm-linux-gnueabihf",
      "libc6-armhf-cross",
      "libc6-dev-armhf-cross",
      "gcc-aarch64-linux-gnu",
      "libc6-arm64-cross",
      "libc6-dev-arm64-cross",
      "libc6-armel-cross",
      "libc6-dev-armel-cross",
      "binutils-arm-linux-gnueabi",
      "gcc-arm-linux-gnueabi",
      "libncurses5-dev",
      "bison",
      "flex",
      "libssl-dev",
      "bc",
      "pkg-config",
      "libudev-dev",
      "libdbus-1-dev",
    ])
    .withExec(["mkdir", "-p", "/build/sysroot", "/tmp/debs"])
    // Download all cross-arch .deb files into /tmp/debs to avoid the
    // "_apt permission denied" error that occurs when downloading into /.
    .withExec([
      "sh",
      "-c",
      [
        "cd /tmp/debs && apt-get download",
        // armhf packages
        "libasound2:armhf",
        "libasound2-dev:armhf",
        "libdbus-1-dev:armhf",
        "libdbus-1-3:armhf",
        "libsystemd-dev:armhf",
        "libsystemd0:armhf",
        "libcap2:armhf",
        "libcap-dev:armhf",
        "libgcrypt20:armhf",
        "libgcrypt20-dev:armhf",
        "libgpg-error0:armhf",
        "libgpg-error-dev:armhf",
        "liblz4-1:armhf",
        "liblz4-dev:armhf",
        "libxxhash0:armhf",
        "libxxhash-dev:armhf",
        "liblzma5:armhf",
        "liblzma-dev:armhf",
        "libzstd1:armhf",
        "libzstd-dev:armhf",
        // arm64 packages
        "libasound2:arm64",
        "libasound2-dev:arm64",
        "libdbus-1-dev:arm64",
        "libdbus-1-3:arm64",
        "libsystemd-dev:arm64",
        "libsystemd0:arm64",
        "libcap2:arm64",
        "libcap-dev:arm64",
        "libgcrypt20:arm64",
        "libgcrypt20-dev:arm64",
        "libgpg-error0:arm64",
        "libgpg-error-dev:arm64",
        "liblz4-1:arm64",
        "liblz4-dev:arm64",
        "libxxhash0:arm64",
        "libxxhash-dev:arm64",
        "liblzma5:arm64",
        "liblzma-dev:arm64",
        "libzstd1:arm64",
        "libzstd-dev:arm64",
      ].join(" "),
    ])
    // ── armhf sysroot extractions ──────────────────────────────────────────
    .withExec(dpkgExtract("libasound2-dev", "armhf", "/build/sysroot/"))
    .withExec(dpkgExtract("libasound2", "armhf", "/build/sysroot/"))
    .withExec(dpkgExtract("libdbus-1-dev", "armhf", "/build/sysroot/"))
    .withExec(dpkgExtract("libdbus-1-3", "armhf", "/build/sysroot/"))
    .withExec(dpkgExtract("libsystemd-dev", "armhf", "/build/sysroot/"))
    .withExec(dpkgExtract("libsystemd0", "armhf", "/build/sysroot/"))
    .withExec(dpkgExtract("libcap-dev", "armhf", "/build/sysroot/"))
    .withExec(dpkgExtract("libcap2", "armhf", "/build/sysroot/"))
    .withExec(dpkgExtract("libgcrypt20-dev", "armhf", "/build/sysroot/"))
    .withExec(dpkgExtract("libgcrypt20", "armhf", "/build/sysroot/"))
    .withExec(dpkgExtract("libgpg-error-dev", "armhf", "/build/sysroot/"))
    .withExec(dpkgExtract("libgpg-error0", "armhf", "/build/sysroot/"))
    .withExec(dpkgExtract("liblz4-1", "armhf", "/build/sysroot/"))
    .withExec(dpkgExtract("liblz4-dev", "armhf", "/build/sysroot/"))
    .withExec(dpkgExtract("liblzma5", "armhf", "/build/sysroot/"))
    .withExec(dpkgExtract("liblzma-dev", "armhf", "/build/sysroot/"))
    .withExec(dpkgExtract("libxxhash0", "armhf", "/build/sysroot/"))
    .withExec(dpkgExtract("libxxhash-dev", "armhf", "/build/sysroot/"))
    .withExec(dpkgExtract("libzstd1", "armhf", "/build/sysroot/"))
    .withExec(dpkgExtract("libzstd-dev", "armhf", "/build/sysroot/"))
    // ── arm64 sysroot extractions ──────────────────────────────────────────
    .withExec(dpkgExtract("libasound2-dev", "arm64", "/build/sysroot/"))
    .withExec(dpkgExtract("libasound2", "arm64", "/build/sysroot/"))
    .withExec(dpkgExtract("libdbus-1-dev", "arm64", "/build/sysroot/"))
    .withExec(dpkgExtract("libdbus-1-3", "arm64", "/build/sysroot/"))
    .withExec(dpkgExtract("libsystemd-dev", "arm64", "/build/sysroot/"))
    .withExec(dpkgExtract("libsystemd0", "arm64", "/build/sysroot/"))
    .withExec(dpkgExtract("libcap-dev", "arm64", "/build/sysroot/"))
    .withExec(dpkgExtract("libcap2", "arm64", "/build/sysroot/"))
    .withExec(dpkgExtract("libgcrypt20-dev", "arm64", "/build/sysroot/"))
    .withExec(dpkgExtract("libgcrypt20", "arm64", "/build/sysroot/"))
    .withExec(dpkgExtract("libgpg-error-dev", "arm64", "/build/sysroot/"))
    .withExec(dpkgExtract("libgpg-error0", "arm64", "/build/sysroot/"))
    .withExec(dpkgExtract("liblz4-1", "arm64", "/build/sysroot/"))
    .withExec(dpkgExtract("liblz4-dev", "arm64", "/build/sysroot/"))
    .withExec(dpkgExtract("liblzma5", "arm64", "/build/sysroot/"))
    .withExec(dpkgExtract("liblzma-dev", "arm64", "/build/sysroot/"))
    .withExec(dpkgExtract("libxxhash0", "arm64", "/build/sysroot/"))
    .withExec(dpkgExtract("libxxhash-dev", "arm64", "/build/sysroot/"))
    .withExec(dpkgExtract("libzstd1", "arm64", "/build/sysroot/"))
    .withExec(dpkgExtract("libzstd-dev", "arm64", "/build/sysroot/"))
    // ── app sources & build ────────────────────────────────────────────────
    .withDirectory("/app", context, { exclude })
    .withWorkdir("/app")
    .withMountedCache("/app/target", dag.cacheVolume("target"))
    .withMountedCache("/root/cargo/registry", dag.cacheVolume("registry"))
    .withMountedCache("/assets", dag.cacheVolume("gh-release-assets"))
    .withEnvVariable("RUSTFLAGS", rustflags)
    .withEnvVariable(
      "PKG_CONFIG_ALLOW_CROSS",
      Deno.env.get("TARGET") !== "x86_64-unknown-linux-gnu" ? "1" : "0",
    )
    .withEnvVariable(
      "C_INCLUDE_PATH",
      Deno.env.get("TARGET") !== "x86_64-unknown-linux-gnu"
        ? "/build/sysroot/usr/include"
        : "/usr/include",
    )
    .withEnvVariable("TAG", Deno.env.get("TAG") || "latest")
    .withEnvVariable(
      "TARGET",
      Deno.env.get("TARGET") || "x86_64-unknown-linux-gnu",
    )
    .withExec([
      "sh",
      "-c",
      "mv /usr/bin/protoc /usr/bin/_protoc && cp tools/protoc /usr/bin/protoc && chmod a+x /usr/bin/protoc",
    ])
    .withExec(["sh", "-c", "rustup target add $TARGET"])
    .withExec(["sh", "-c", "cargo build --release --target $TARGET"])
    .withExec(["sh", "-c", "cp target/${TARGET}/release/tunein ."])
    .withExec([
      "sh",
      "-c",
      "tar czvf /assets/tunein_${TAG}_${TARGET}.tar.gz tunein README.md LICENSE",
    ])
    .withExec([
      "sh",
      "-c",
      "shasum -a 256 /assets/tunein_${TAG}_${TARGET}.tar.gz > /assets/tunein_${TAG}_${TARGET}.tar.gz.sha256",
    ])
    .withExec(["sh", "-c", "cp /assets/tunein_${TAG}_${TARGET}.tar.gz ."])
    .withExec([
      "sh",
      "-c",
      "cp /assets/tunein_${TAG}_${TARGET}.tar.gz.sha256 .",
    ]);

  const exe = await ctr.file(
    `/app/tunein_${Deno.env.get("TAG")}_${Deno.env.get("TARGET")}.tar.gz`,
  );
  await exe.export(
    `./tunein_${Deno.env.get("TAG")}_${Deno.env.get("TARGET")}.tar.gz`,
  );

  const sha = await ctr.file(
    `/app/tunein_${Deno.env.get("TAG")}_${Deno.env.get(
      "TARGET",
    )}.tar.gz.sha256`,
  );
  await sha.export(
    `./tunein_${Deno.env.get("TAG")}_${Deno.env.get("TARGET")}.tar.gz.sha256`,
  );
  return ctr.stdout();
};

export type JobExec = (src?: string) =>
  | Promise<string>
  | ((
      src?: string,
      options?: {
        ignore: string[];
      },
    ) => Promise<string>);

export const runnableJobs: Record<Job, JobExec> = {
  [Job.test]: test,
  [Job.build]: build,
};

export const jobDescriptions: Record<Job, string> = {
  [Job.test]: "Run tests",
  [Job.build]: "Build the project",
};
