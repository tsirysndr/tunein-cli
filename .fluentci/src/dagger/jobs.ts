import Client, { connect } from "../../deps.ts";

export enum Job {
  test = "test",
  build = "build",
}

export const exclude = ["target", ".git", ".devbox", ".fluentci"];

export const test = async (src = ".", options: string[] = []) => {
  await connect(async (client: Client) => {
    const context = client.host().directory(src);
    const ctr = client
      .pipeline(Job.test)
      .container()
      .from("rust:latest")
      .withDirectory("/app", context, { exclude })
      .withWorkdir("/app")
      .withMountedCache("/app/target", client.cacheVolume("target"))
      .withMountedCache("/root/cargo/registry", client.cacheVolume("registry"))
      .withExec(["cargo", "test", ...options]);

    const result = await ctr.stdout();

    console.log(result);
  });
  return "done";
};

export const build = async (src = ".") => {
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
  await connect(async (client: Client) => {
    const context = client.host().directory(src);
    const ctr = client
      .pipeline(Job.build)
      .container()
      .from("rust:1.76-bullseye")
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
      ])
      .withExec(["mkdir", "-p", "/build/sysroot"])
      .withExec([
        "apt-get",
        "download",
        "libasound2:armhf",
        "libasound2-dev:armhf",
        "libasound2:arm64",
        "libasound2-dev:arm64",
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
      .withDirectory("/app", context, { exclude })
      .withWorkdir("/app")
      .withMountedCache("/app/target", client.cacheVolume("target"))
      .withMountedCache("/root/cargo/registry", client.cacheVolume("registry"))
      .withMountedCache("/assets", client.cacheVolume("gh-release-assets"))
      .withEnvVariable("RUSTFLAGS", rustflags)
      .withEnvVariable(
        "PKG_CONFIG_ALLOW_CROSS",
        Deno.env.get("TARGET") !== "x86_64-unknown-linux-gnu" ? "1" : "0"
      )
      .withEnvVariable(
        "C_INCLUDE_PATH",
        Deno.env.get("TARGET") !== "x86_64-unknown-linux-gnu"
          ? "/build/sysroot/usr/include"
          : "/usr/include"
      )
      .withEnvVariable("TAG", Deno.env.get("TAG") || "latest")
      .withEnvVariable(
        "TARGET",
        Deno.env.get("TARGET") || "x86_64-unknown-linux-gnu"
      )
      .withExec(["sh", "-c", "rustup target add $TARGET"])
      .withExec(["sh", "-c", "cargo build --release --target $TARGET"])
      .withExec(["sh", "-c", "cp target/${TARGET}/release/tunein ."])
      .withExec([
        "sh",
        "-c",
        "tar czvf /assets/tunein_${TAG}_${TARGET}.tar.gz tunein",
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

    await ctr.stdout();

    const exe = await ctr.file(
      `/app/tunein_${Deno.env.get("TAG")}_${Deno.env.get("TARGET")}.tar.gz`
    );
    await exe.export(
      `./tunein_${Deno.env.get("TAG")}_${Deno.env.get("TARGET")}.tar.gz`
    );

    const sha = await ctr.file(
      `/app/tunein_${Deno.env.get("TAG")}_${Deno.env.get(
        "TARGET"
      )}.tar.gz.sha256`
    );
    await sha.export(
      `./tunein_${Deno.env.get("TAG")}_${Deno.env.get("TARGET")}.tar.gz.sha256`
    );
  });
  return "Done";
};

export type JobExec = (src?: string) =>
  | Promise<string>
  | ((
      src?: string,
      options?: {
        ignore: string[];
      }
    ) => Promise<string>);

export const runnableJobs: Record<Job, JobExec> = {
  [Job.test]: test,
  [Job.build]: build,
};

export const jobDescriptions: Record<Job, string> = {
  [Job.test]: "Run tests",
  [Job.build]: "Build the project",
};
