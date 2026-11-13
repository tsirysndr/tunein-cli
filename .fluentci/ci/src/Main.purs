module Main (prepare, web, test, format, linuxRelease, macosRelease, githubAssets, gemfury, nixBuild, flakehub, flakestry, check, linuxReleaseFromEnv, macosReleaseFromEnv, flakehubFromEnv, flakestryFromEnv, checkoutRelease) where

import Prelude
import CI.Cross as Cross
import CI.Dependencies as D
import CI.Package as Package
import CI.Support as S
import CI.Toolchain as Toolchain
import CI.Validation as V
import Data.Array as A
import Data.Either (either)
import Data.String as String
import Effect (Effect)
import Effect.Exception (throw)
import FluentCI.Plugin as P

-- Reuse the pinned bootstrap tools only after checking their actual native versions.
-- The published 0.2.0 archive installers have an untrimmed cached-path probe;
-- isolate fallback installs so an existing cache prefix is never overwritten.
nodeAndBun :: Effect Unit
nodeAndBun = do
  Toolchain.ensureExecutable "node" "v24.22.2" (S.captureIn "." [ "node", "--version" ]) do
    prefix <- S.fresh
    void (D.setupNode { version: "24.22.2", installDir: prefix <> "/node" })
  Toolchain.ensureExecutable "bun" "1.4.3" (S.captureIn "." [ "bun", "--version" ]) do
    prefix <- S.fresh
    void (D.setupBun { version: "1.4.3", installDir: prefix <> "/bun" })

requireOS :: Array String -> Effect String
requireOS supported = do
  os <- P.getOS >>= S.unwrap
  unless (A.elem os supported) (throw ("Unsupported host OS: " <> os))
  pure os

-- | Prepare Node and Bun, persist PATH for later GitHub steps, and return true on success.
prepare :: Effect Boolean
prepare = do
  nodeAndBun
  S.persistEnvironment
  pure true

-- | Install locked web dependencies and build the UI embedded in the Rust binary.
-- | @param source Source checkout containing web/bun.lock.
web :: { source :: String } -> Effect Boolean
web args = do
  nodeAndBun
  S.runIn (args.source <> "/web") [ "bun", "install", "--frozen-lockfile" ]
  S.runIn (args.source <> "/web") [ "bun", "run", "build" ]
  pure true

-- | Run locked web lint/tests and release-mode Rust tests on Linux; return true on success.
-- | @param source Source checkout; dependency installation runs on this CI host.
test :: { source :: String } -> Effect Boolean
test args = do
  void (requireOS [ "linux" ])
  S.run [ "sudo", "apt-get", "update" ]
  S.run [ "sudo", "apt-get", "install", "-y", "--no-install-recommends", "libasound2-dev", "libdbus-1-dev", "pkg-config", "protobuf-compiler" ]
  void (D.setupRust { version: "stable", components: "rustfmt,clippy", targets: "" })
  void (web args)
  S.runIn (args.source <> "/web") [ "bun", "run", "lint" ]
  S.runIn (args.source <> "/web") [ "bun", "run", "test" ]
  S.runIn args.source [ "cargo", "test", "--release", "--locked" ]
  pure true

-- | Check Rust formatting using the shared Rust installer.
-- | @param source Source checkout whose Rust code should be checked.
format :: { source :: String } -> Effect Boolean
format args = do
  void (D.setupRust { version: "stable", components: "rustfmt", targets: "" })
  S.runIn args.source [ "cargo", "fmt", "--all", "--check" ]
  pure true

-- | Cross-build TuneIn against Debian Trixie using shared package builders and return release file paths.
-- | @param source Source checkout to build, including packaging metadata.
-- | @param target Rust Linux triple: x86_64-unknown-linux-gnu, aarch64-unknown-linux-gnu, or armv7-unknown-linux-gnueabihf.
-- | @param version Release tag used in archive names and package metadata.
-- | @param output Destination for release assets and SHA-256 files.
linuxRelease :: { source :: String, target :: String, version :: String, output :: String } -> Effect (Array String)
linuxRelease args = do
  void (requireOS [ "linux" ])
  unless (V.validVersion args.version) (throw "Invalid release version")
  void (either throw pure (V.targetInfo args.target))
  S.run [ "sudo", "apt-get", "update" ]
  S.run [ "sudo", "apt-get", "install", "-y", "--no-install-recommends", "dpkg-dev", "rpm" ]
  void (web { source: args.source })
  stage <- S.fresh
  binary <- Cross.build { source: args.source, target: args.target, output: stage }
  files <- Package.packageRelease { source: args.source, target: args.target, version: args.version, binary, output: args.output }
  S.run [ "rm", "-rf", "--", stage ]
  pure files

-- | Build a native/cross macOS release with locked dependencies and return archive/checksum paths.
-- | @param source Source checkout to build.
-- | @param target x86_64-apple-darwin or aarch64-apple-darwin.
-- | @param version Release tag included in the archive filename.
-- | @param output Destination for the archive and its checksum.
macosRelease :: { source :: String, target :: String, version :: String, output :: String } -> Effect (Array String)
macosRelease args = do
  void (requireOS [ "macos" ])
  unless (A.elem args.target [ "x86_64-apple-darwin", "aarch64-apple-darwin" ]) (throw "Unsupported macOS target")
  unless (V.validVersion args.version) (throw "Invalid release version")
  S.run [ "brew", "install", "protobuf", "llvm@15" ]
  void (D.setupRust { version: "stable", components: "", targets: args.target })
  void (web { source: args.source })
  S.runIn args.source [ "cargo", "build", "--release", "--locked", "--target", args.target, "--bin", "tunein" ]
  S.run [ "mkdir", "-p", args.output ]
  output <- S.absolute args.output
  source <- S.absolute args.source
  let archive = output <> "/tunein_" <> args.version <> "_" <> args.target <> ".tar.gz"
  S.run [ "tar", "-czf", archive, "-C", source <> "/target/" <> args.target <> "/release", "tunein" ]
  S.checksum archive
  pure [ archive, archive <> ".sha256" ]

-- | Upload through gh-artifacts and return its execution report path; optionally create a release.
-- | @param directory Directory containing only this job's assets.
-- | @param tag Existing Git tag identifying the release.
-- | @param create Create a missing release with generated notes; otherwise upload to the existing release.
githubAssets :: { directory :: String, tag :: String, create :: Boolean } -> Effect String
githubAssets args = do
  unless (V.validVersion args.tag) (throw "Invalid release tag")
  void D.setupGh
  files <- A.filter S.releaseAsset <$> S.files args.directory
  when (A.null files) (throw "No release assets found")
  when args.create do
    repository <- S.requiredEnv "GITHUB_REPOSITORY"
    tags <- S.captureIn "." [ "gh", "api", "repos/" <> repository <> "/releases", "--paginate", "--jq", ".[] | .tag_name" ]
    unless (A.elem args.tag (String.split (String.Pattern "\n") (String.trim tags)))
      (S.run [ "gh", "release", "create", args.tag, "--verify-tag", "--generate-notes" ])
  root <- S.fresh
  result <- D.uploadRelease { project: ".", runDir: root <> "/upload", tag: args.tag, files: V.csv files, clobber: true }
  pure result.reportPath

-- | Upload generated packages through Gemfury; return false when credentials or packages are absent.
-- | @param directory Directory containing this target's Debian/RPM packages.
gemfury :: { directory :: String } -> Effect Boolean
gemfury args = do
  token <- S.env "FURY_TOKEN" ""
  if String.trim token == "" then pure false else do
    paths <- A.filter (\path -> String.endsWith (String.Pattern ".deb") path || String.endsWith (String.Pattern ".rpm") path) <$> S.files args.directory
    if A.null paths then pure false else do
      result <- D.uploadGemfury { files: V.csv paths }
      pure result.uploaded

nixSetup :: String -> Effect Unit
nixSetup cache = do
  void (requireOS [ "linux", "macos" ])
  void (D.setupNix { sudo: true, trustedUsers: [ "runner" ], extraConf: [ "experimental-features = nix-command flakes" ] })
  when (cache /= "") (void (D.setupCachix { cache }))

-- | Build the local Nix flake and push its result closure when Cachix credentials exist.
-- | @param source Source checkout containing flake.nix and flake.lock.
-- | @param cache Public Cachix cache name; empty disables Cachix configuration and uploads.
nixBuild :: { source :: String, cache :: String } -> Effect Boolean
nixBuild args = do
  nixSetup args.cache
  S.runIn args.source [ "nix", "build", "--no-write-lock-file", "--print-build-logs", ".#default" ]
  token <- S.env "CACHIX_AUTH_TOKEN" ""
  when (args.cache /= "" && String.trim token /= "") (S.runIn args.source [ "cachix", "push", args.cache, "./result" ])
  pure true

-- | Publish through nix-publish's GitHub OIDC FlakeHub function and return its report path.
-- | @param source Checkout of the exact release tag.
-- | @param tag Release version or tag to publish.
flakehub :: { source :: String, tag :: String } -> Effect String
flakehub args = do
  unless (V.validVersion args.tag) (throw "Invalid release tag")
  nixSetup ""
  void D.setupFlakehub
  root <- S.fresh
  result <- D.flakehub { project: args.source, runDir: root <> "/flakehub", name: "tsirysndr/tunein-cli", version: args.tag, visibility: "public" }
  pure result.reportPath

-- | Publish through nix-publish's Flakestry function using GitHub OIDC and return its report path.
-- | @param source Checkout of the exact release tag.
-- | @param tag Release version or tag to publish.
-- | @param ref Exact Git commit corresponding to the selected release.
flakestry :: { source :: String, tag :: String, ref :: String } -> Effect String
flakestry args = do
  unless (V.validVersion args.tag) (throw "Invalid release tag")
  nixSetup ""
  root <- S.fresh
  result <- D.flakestry { project: args.source, runDir: root <> "/flakestry", version: args.tag, ref: args.ref }
  pure result.reportPath

-- | Lint all GitHub workflows using a pinned actionlint installed through the Go setup component.
-- | @param directory Repository containing the GitHub workflow files.
check :: { directory :: String } -> Effect Boolean
check args = do
  void (D.setupGo { version: "1.26.2" })
  paths <- A.filter (String.endsWith (String.Pattern ".yml")) <$> S.files (args.directory <> "/.github/workflows")
  S.run ([ "go", "run", "github.com/rhysd/actionlint/cmd/actionlint@v1.7.12" ] <> paths)
  pure true

-- | Build and publish the Linux target selected by release workflow environment variables.
linuxReleaseFromEnv :: Effect (Array String)
linuxReleaseFromEnv = do
  source <- S.env "SOURCE_DIRECTORY" ".fluentci/source"
  target <- S.requiredEnv "TARGET"
  version <- S.requiredEnv "RELEASE_VERSION"
  let output = source <> "/.fluentci/out/" <> target
  built <- linuxRelease { source, target, version, output }
  void (githubAssets { directory: output, tag: version, create: false })
  void (gemfury { directory: output })
  pure built

-- | Build and publish the macOS target selected by release workflow environment variables.
macosReleaseFromEnv :: Effect (Array String)
macosReleaseFromEnv = do
  source <- S.env "SOURCE_DIRECTORY" ".fluentci/source"
  target <- S.requiredEnv "TARGET"
  version <- S.requiredEnv "RELEASE_VERSION"
  let output = source <> "/.fluentci/out/" <> target
  built <- macosRelease { source, target, version, output }
  void (githubAssets { directory: output, tag: version, create: true })
  pure built

-- | Publish the exact checkout and release tag selected by the FlakeHub workflow.
flakehubFromEnv :: Effect String
flakehubFromEnv = do
  source <- S.env "SOURCE_DIRECTORY" ".fluentci/source"
  tag <- S.requiredEnv "RELEASE_VERSION"
  flakehub { source, tag }

-- | Resolve the checked-out commit and publish it with the selected tag to Flakestry.
flakestryFromEnv :: Effect String
flakestryFromEnv = do
  source <- S.env "SOURCE_DIRECTORY" ".fluentci/source"
  tag <- S.requiredEnv "RELEASE_VERSION"
  ref <- String.trim <$> S.captureIn source [ "git", "rev-parse", "HEAD" ]
  flakestry { source, tag, ref }

-- | Create a detached application worktree for RELEASE_VERSION under .fluentci/source.
-- | Returns its absolute path and publishes SOURCE_DIRECTORY for subsequent steps.
-- | Existing worktrees are refused so different release trees cannot be mixed.
checkoutRelease :: Effect String
checkoutRelease = do
  tag <- S.requiredEnv "RELEASE_VERSION"
  unless (V.validVersion tag) (throw "Invalid release tag")
  root <- S.absolute "."
  let source = root <> "/.fluentci/source"
  S.run [ "git", "worktree", "add", "--detach", source, "refs/tags/" <> tag ]
  S.publishEnvironment "SOURCE_DIRECTORY" source
  pure source
