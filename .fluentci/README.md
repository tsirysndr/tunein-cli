# TuneIn CI/CD

All project-specific pipeline logic lives in the PureScript component in
[`ci/src`](./ci/src). `.github/workflows/` supplies only events, runners,
checkout selection, permissions/secrets, and calls into this component.

## Structure

- `workflows/*.toml`: named FluentCI workflows; CI plus manual build/publish workflows.
- `ci/fluentci-module.toml`: the local typed Wasm component and pinned native dependencies.
- `ci/src/Main.purs`: setup composition, tests, formatting, releases, Nix, and publishing.
- `ci/src/CI/Cross.purs`: Debian Trixie containers and literal cross-compiler/sysroot commands.
- `ci/src/CI/Package.purs`: TuneIn file staging and metadata updates; actual builders are shared components.
- `ci/src/CI/BsdGuest.purs`: BSD build/package logic, compiled for Node inside the VM.
- `ci/test/Main.purs`: PureScript policy regression cases, executed by CI.
- `dependency-contracts.json`: the published typed contracts used to select pinned dependency versions.

The old Deno entry points, Dagger SDK copies, legacy engine adapters, provider
fixtures, and duplicated package/upload implementations have been removed.

## Shared components

`setup-purescript` bootstraps PureScript, Spago, Node, Bun, and npm. Operations call
`setup-node`, `setup-bun`, `setup-rust`, `setup-go`, `setup-gh`,
`setup-determinate-nix`, `setup-cachix`, and `setup-flakehub-push` as needed.
Published registry versions and SHA-256 checksums are pinned in `fluentci.lock`.

`dpkg` and `rpmbuild` validate and build the staged packages. `gh-artifacts`
uploads GitHub release files. `gemfury` uploads generated Linux packages, and
`nix-publish` handles FlakeHub/Flakestry OIDC publishing. These implementations
are imported as typed component dependencies rather than copied into this project.

All public lists sent to modules use comma-separated strings, with CSV quoting
when a filename contains commas or quotes. Internal arrays remain arrays.

## Running workflows

Provision FluentCI v0.17.3 and run the published toolchain setup before building
the local component. GitHub's `fluentci-toolchain` action handles this automatically.
After installing the toolchain:

```text
fluentci run --workflow ci
fluentci run --workflow nix
fluentci run ci --help
fluentci run ci linux-release --help
```

Release workflow environment variables are `RELEASE_VERSION`, `TARGET`, and
optional `SOURCE_DIRECTORY` (defaults to the selected `.fluentci/source` worktree). For example,
`TARGET=aarch64-unknown-linux-gnu` selects the ARM64 Linux build. Manual release
workflows can publish artifacts; they are not dry runs. Each release target writes
to `<source>/.fluentci/out/<target>`.

The Linux release image is the official `rust:trixie`, pinned by its verified
Docker Hub manifest digest in `CI.Cross`. Cross sysroots use Trixie packages,
including the time64 ALSA runtime. Outputs retain their existing target names:
Linux x86-64 (archive, DEB, RPM), ARM64 (archive, DEB), ARMv7 (archive), macOS
x86-64/ARM64, and FreeBSD/NetBSD x86-64/ARM64 archives. Archives have SHA-256 files.
Docker must be available on Linux release runners; it is already supplied by the
selected GitHub hosted runners. Native build libraries use the runner's package
manager from PureScript.

BSD VM providers only bootstrap Node and invoke the compiled PureScript guest.
The minimal `GuestHost` binding exposes process/environment, hashing, and file
primitives. It contains no target selection or pipeline decisions. This keeps
ARM64 BSD builds working even though FluentCI v0.17.3 has no ARM64 BSD CLI asset.

The only other JavaScript is the SDK/component import and build plumbing:
`Dependencies.js`, `bootstrap.mjs`, and `build.mjs`. There are no shell scripts
or shell-program strings implementing CI decisions. Node bootstrap glue obtains
the SDK bundled with the pinned FluentCI CLI and installs the locked compiler
packages; the PureScript `prepare` function persists environment for GitHub steps.

## Credentials and release selection

- `GH_TOKEN`: GitHub release creation/upload.
- `FURY_TOKEN` and `FURY_ACCOUNT`: optional Gemfury upload. An absent token skips it.
- `CACHIX_AUTH_TOKEN`: optional Cachix push; public cache configuration still applies.
- `id-token: write`: required on FlakeHub and Flakestry publishing jobs.

Credentials stay in native environment variables and are not copied into command
arguments. Manual release and publishing jobs check out their selected tag;
Flakestry resolves the exact commit from that checkout. The pipeline checkout stays at `github.workflow_sha` for release jobs. PureScript
creates the selected tag as a detached worktree at `.fluentci/source` and exports
`SOURCE_DIRECTORY`, so older application tags can use the current CI definitions.
There is no separate tooling checkout.

The CI job groups workflow lint, PureScript policy checks, Rust formatting, and
web/Rust tests in one runner. New pushes cancel stale CI/Nix runs. Provider-specific
BSD VM actions remain responsible only for guest lifecycle.

Runtime verification belongs to CI. No builds, tests, modules, installers, or VMs
were executed locally during this migration.

## Public return types

The pinned FluentCI v0.17.3 inspector supports concrete scalar and list returns.
Setup/check/build-status functions return booleans, release builders return lists
of asset paths, and GitHub/Nix publishing functions return execution report paths.
Gemfury returns false for credential/package skips and true after upload.
Record-shaped results from imported components remain internal. Native errors
still throw and stop the workflow; they are never converted into success values.

The build applies `ci/sdk-compat.mjs` before importing the pinned inspector.
It normalizes the compiler's `ParensInType` nodes in type resolution/rendering,
so signatures such as `Effect (Array String)` retain typed list returns. This is
SDK build compatibility code; all CI decisions remain in PureScript. Repeated
builds leave an already normalized adapter unchanged, and an unknown SDK layout
fails explicitly rather than silently producing a different contract.

Node and Bun calls first verify the pinned version on PATH and reuse matching
bootstrap installations. The published setup 0.2.0 cached-directory probes can
misread a trailing newline; when installation is needed, TuneIn passes a fresh
per-invocation prefix to the shared setup component. Existing cached prefixes
are never overwritten. The selected executable is verified again after setup.
