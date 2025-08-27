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
    .withExec(["mkdir", "-p", "/build/sysroot"])
    .withExec([
      "apt-get",
      "download",
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
    ])
    .withExec([
      "dpkg",
      "-x",
      "libasound2-dev_1.2.4-1.1_arm64.deb",
      "/build/sysroot/",
    ])
    .withExec([
      "dpkg",
      "-x",
      "libasound2_1.2.4-1.1_arm64.deb",
      "/build/sysroot/",
    ])
    .withExec([
      "dpkg",
      "-x",
      "libasound2-dev_1.2.4-1.1_armhf.deb",
      "/build/sysroot/",
    ])
    .withExec([
      "dpkg",
      "-x",
      "libasound2_1.2.4-1.1_armhf.deb",
      "/build/sysroot/",
    ])
    .withExec([
      "dpkg",
      "-x",
      "libdbus-1-dev_1.12.28-0+deb11u1_armhf.deb",
      "/build/sysroot/",
    ])
    .withExec([
      "dpkg",
      "-x",
      "libdbus-1-3_1.12.28-0+deb11u1_armhf.deb",
      "/build/sysroot/",
    ])
    .withExec([
      "dpkg",
      "-x",
      "libsystemd-dev_247.3-7+deb11u7_armhf.deb",
      "/build/sysroot/",
    ])
    .withExec([
      "dpkg",
      "-x",
      "libsystemd0_247.3-7+deb11u7_armhf.deb",
      "/build/sysroot/",
    ])
    .withExec([
      "dpkg",
      "-x",
      "libcap-dev_1%3a2.44-1+deb11u1_armhf.deb",
      "/build/sysroot/",
    ])
    .withExec([
      "dpkg",
      "-x",
      "libcap2_1%3a2.44-1+deb11u1_armhf.deb",
      "/build/sysroot/",
    ])
    .withExec([
      "dpkg",
      "-x",
      "libgcrypt20-dev_1.8.7-6_armhf.deb",
      "/build/sysroot/",
    ])
    .withExec([
      "dpkg",
      "-x",
      "libgcrypt20_1.8.7-6_armhf.deb",
      "/build/sysroot/",
    ])
    .withExec([
      "dpkg",
      "-x",
      "libgpg-error-dev_1.38-2_armhf.debeb",
      "/build/sysroot/",
    ])
    .withExec([
      "dpkg",
      "-x",
      "libgpg-error0_1.38-2_armhf.deb",
      "/build/sysroot/",
    ])
    .withExec([
      "dpkg",
      "-x",
      "liblz4-1_1.9.3-2_armhf.deb",
      "/build/sysroot/",
    ])
    .withExec([
      "dpkg",
      "-x",
      "liblz4-dev_1.9.3-2_armhf.deb",
      "/build/sysroot/",
    ])
    .withExec([
      "dpkg",
      "-x",
      "liblzma-dev_5.2.5-2.1~deb11u1_armhf.deb",
      "/build/sysroot/",
    ])
    .withExec([
      "dpkg",
      "-x",
      "liblzma5_5.2.5-2.1~deb11u1_armhf.deb",
      "/build/sysroot/",
    ])
    .withExec([
      "dpkg",
      "-x",
      "libxxhash-dev_0.8.0-2_armhf.deb",
      "/build/sysroot/",
    ])
    .withExec([
      "dpkg",
      "-x",
      "libxxhash0_0.8.0-2_armhf.deb",
      "/build/sysroot/",
    ])
    .withExec([
      "dpkg",
      "-x",
      "libzstd1_1.4.8+dfsg-2.1_armhf.deb",
      "/build/sysroot/",
    ])
    .withExec([
      "dpkg",
      "-x",
      "libzstd-dev_1.4.8+dfsg-2.1_armhf.deb",
      "/build/sysroot/",
    ])
    .withExec([
      "dpkg",
      "-x",
      "libdbus-1-dev_1.12.28-0+deb11u1_arm64.deb",
      "/build/sysroot/",
    ])
    .withExec([
      "dpkg",
      "-x",
      "libdbus-1-3_1.12.28-0+deb11u1_arm64.deb",
      "/build/sysroot/",
    ])
    .withExec([
      "dpkg",
      "-x",
      "libsystemd-dev_247.3-7+deb11u7_arm64.deb",
      "/build/sysroot/",
    ])
    .withExec([
      "dpkg",
      "-x",
      "libsystemd0_247.3-7+deb11u7_arm64.deb",
      "/build/sysroot/",
    ])
    .withExec([
      "dpkg",
      "-x",
      "libcap-dev_1%3a2.44-1+deb11u1_arm64.deb",
      "/build/sysroot/",
    ])
    .withExec([
      "dpkg",
      "-x",
      "libcap2_1%3a2.44-1+deb11u1_arm64.deb",
      "/build/sysroot/",
    ])
    .withExec([
      "dpkg",
      "-x",
      "libgcrypt20-dev_1.8.7-6_arm64.deb",
      "/build/sysroot/",
    ])
    .withExec([
      "dpkg",
      "-x",
      "libgcrypt20_1.8.7-6_arm64.deb",
      "/build/sysroot/",
    ])
    .withExec([
      "dpkg",
      "-x",
      "libgpg-error-dev_1.38-2_arm64.deb",
      "/build/sysroot/",
    ])
    .withExec([
      "dpkg",
      "-x",
      "libgpg-error0_1.38-2_arm64.deb",
      "/build/sysroot/",
    ])
    .withExec([
      "dpkg",
      "-x",
      "liblz4-1_1.9.3-2_arm64.deb",
      "/build/sysroot/",
    ])
    .withExec([
      "dpkg",
      "-x",
      "liblz4-dev_1.9.3-2_arm64.deb",
      "/build/sysroot/",
    ])
    .withExec([
      "dpkg",
      "-x",
      "liblzma-dev_5.2.5-2.1~deb11u1_arm64.deb",
      "/build/sysroot/",
    ])
    .withExec([
      "dpkg",
      "-x",
      "liblzma5_5.2.5-2.1~deb11u1_arm64.deb",
      "/build/sysroot/",
    ])
    .withExec([
      "dpkg",
      "-x",
      "libxxhash-dev_0.8.0-2_arm64.deb",
      "/build/sysroot/",
    ])
    .withExec([
      "dpkg",
      "-x",
      "libxxhash0_0.8.0-2_arm64.deb",
      "/build/sysroot/",
    ])
    .withExec([
      "dpkg",
      "-x",
      "libzstd1_1.4.8+dfsg-2.1_arm64.deb",
      "/build/sysroot/",
    ])
    .withExec([
      "dpkg",
      "-x",
      "libzstd-dev_1.4.8+dfsg-2.1_arm64.deb",
      "/build/sysroot/",
    ])
    .withDirectory("/app", context, { exclude })
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
    `/app/tunein_${Deno.env.get("TAG")}_${
      Deno.env.get("TARGET")
    }.tar.gz.sha256`,
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
