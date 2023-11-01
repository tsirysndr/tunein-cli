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
  await connect(async (client: Client) => {
    const context = client.host().directory(src);
    const ctr = client
      .pipeline(Job.build)
      .container()
      .from("rust:1.73-bullseye")
      .withExec(["apt-get", "update"])
      .withExec([
        "apt-get",
        "install",
        "-y",
        "build-essential",
        "libasound2-dev",
        "protobuf-compiler",
      ])
      .withDirectory("/app", context, { exclude })
      .withWorkdir("/app")
      .withMountedCache("/app/target", client.cacheVolume("target"))
      .withMountedCache("/root/cargo/registry", client.cacheVolume("registry"))
      .withMountedCache("/assets", client.cacheVolume("gh-release-assets"))
      .withEnvVariable("TAG", Deno.env.get("TAG") || "latest")
      .withEnvVariable(
        "TARGET",
        Deno.env.get("TARGET") || "x86_64-unknown-linux-gnu"
      )
      .withExec(["sh", "-c", "cargo build --release --target $TARGET"])
      .withExec([
        "sh",
        "-c",
        "tar czvf /assets/tunein_${TAG}_${TARGET}.tar.gz target/$TARGET/release/tunein",
      ])
      .withExec([
        "sh",
        "-c",
        "shasum -a 256 /assets/tunein_${TAG}_${TARGET}.tar.gz > /assets/tunein_${TAG}_${TARGET}.tar.gz.sha256",
      ]);

    await ctr.stdout();
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
