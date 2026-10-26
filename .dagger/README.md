# tunein-cli CI with PureScript Dagger

Web UI and Rust tests, Rust formatting, Linux x86_64/aarch64/armv7 releases, embedded web UI, tarballs, Debian packages (x86_64/aarch64), x86_64 RPMs, Linux flake builds, existing-release uploads and optional Gemfury uploads.

The workflows use Dagger **v1.0.0-beta.21** and the vendored PureScript SDK.
The module lives in `ci/src/`; `Main.purs` exposes typed functions and `CI/`
contains the implementation. Packaging, release decisions, file enumeration, and command arguments are
implemented in PureScript. External tools run directly through `withExec`;
there are no project-owned shell scripts or embedded shell programs in the pipeline.
Images are pinned in the root `dagger.lock`.

## Run a function

Install Dagger v1.0.0-beta.21 using the [official installation guide](https://docs.dagger.io/install),
then enter the workspace. GitHub Actions uses the commit-pinned official Dagger
action in install-only mode; this repository does not maintain a CLI installer.

```sh
cd .dagger
dagger api call -m ci linux-release --source .. \
  --target x86_64-unknown-linux-gnu --version v0.1.0 --output ../dist
```

Linux Nix builds use the runner's architecture. Cachix credentials are optional;
an empty token performs a build without uploading. The configured cache is `tsirysndr`.

```sh
export CACHIX_AUTH_TOKEN="${CACHIX_AUTH_TOKEN:-}"
dagger api call -m ci nix-build --source .. \
  --cachix-token env://CACHIX_AUTH_TOKEN --run-id "manual-$(date +%s)"
```

GitHub Actions keeps event triggers, platform matrices, native jobs and artifact
transfer. Linux builds and package assembly execute inside Dagger. Manual release
builds use the requested tag's source separately from the pipeline revision.
Publishing functions accept Dagger `Secret` arguments and a unique `run-id` so
workflow retries execute uploads again. `publish-release` creates/updates a
public non-prerelease with generated notes on creation; `upload-release` only
attaches assets. Gemfury uploads run only when both secrets are configured.

## Cache volumes in GitHub Actions

`.github/actions/dagger/action.yml` restores explicit named volumes, calls the
build, snapshots them with `save-caches`, and saves that snapshot through
`actions/cache`. `restore-caches` and builds use the same module because Dagger
cache volumes are scoped to their owning module. Keys separate jobs, target
architectures and runner architectures; restored dependencies can survive new
lockfiles through the prefix fallback. Snapshot failures do not hide build failures.

Cargo registry, Git and target directories are cached; web builds also cache Bun
packages. Nix builds cache `/nix`, seeded from the pinned Nix image on a cold run.
These caches do not contain registry publishing credentials. GitHub cache quotas
and eviction still apply. SDK bootstrap/compiler caches remain in the Dagger
engine; the workflow snapshots only the explicitly listed application volumes.

```sh
dagger api call -m ci test --source ..
dagger api call -m ci format --source ..
```

Linux releases retain the Fluent CI pipeline's Rust 1.89/Bullseye baseline,
ARM sysroots, target library dependencies and the application’s existing protobuf compatibility wrapper. The new pipeline
uses `--locked` and selects target pkg-config directories explicitly. The web UI
build runs in Node 22 with Bun 1.4.2, then its output is embedded by Rust.
The old `.fluentci` entry points remain available but Linux workflows no longer
invoke them. macOS, BSD, FlakeHub and Flakestry/OIDC jobs are left as they were.

## Develop the pipeline

Dagger installs Node 22, PureScript 0.15.16 and Spago 1.0.5 **inside the SDK
runtime container**, under `/usr/local`; Spago's dependency cache is under
`/root/.cache/spago-nodejs`. Module compilation happens under `/src`. You only
need the Dagger CLI and a supported engine/container runtime on the host.

For a fast type check without starting Dagger, install PureScript and Spago on
your development machine, then run `spago build --strict` in `.dagger/ci`.
The committed bindings and Spago lockfile make this independent of codegen.
After compiling `.dagger/ci`, run `node --test .dagger/tests/*.test.mjs` from the repository root for
SDK HTTP deadline/error and isolated release-upload regression checks. To intentionally regenerate
bindings, run `python3 .dagger/generate.py`; this starts Dagger and stages only
the pipeline workspace, not the application tree.

See [SDK provenance and local patches](sdk/purescript/VENDORED.md) before
updating the vendored SDK. Retain its LICENSE, NOTICE and schema license.

The migration was checked with strict PureScript compilation, workflow lint,
PureScript pipeline tests and SDK transport tests. Full Dagger builds and publishing
were deliberately left to GitHub Actions; they were not run locally.
