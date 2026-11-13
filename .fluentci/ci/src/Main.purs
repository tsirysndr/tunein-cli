module Main (prepare, web, test, format, linuxRelease, macosRelease, githubAssets, gemfury, nixBuild, flakehub, flakestry, check, linuxReleaseFromEnv, macosReleaseFromEnv, flakehubFromEnv, flakestryFromEnv) where

import Prelude
import CI.Cross as Cross
import CI.Dependencies as D
import CI.Package as Package
import CI.Support as S
import CI.Validation as V
import Data.Array as A
import Data.Either (either)
import Data.String as String
import Effect (Effect)
import Effect.Exception (throw)
import FluentCI.Plugin as P

type Result = { status :: String }
type ReleaseResult = { status :: String, files :: Array String }

nodeAndBun :: Effect Unit
nodeAndBun = do
  void (D.setupNode { version: "24.22.2" })
  void (D.setupBun { version: "1.4.3" })

requireOS :: Array String -> Effect String
requireOS supported = do
  os <- P.getOS >>= S.unwrap
  unless (A.elem os supported) (throw ("Unsupported host OS: " <> os))
  pure os

-- | Prepare Node and Bun and persist the validated PATH for later GitHub steps.
prepare :: Effect Result
prepare = do
  nodeAndBun
  S.persistEnvironment
  pure { status: "ready" }

-- | Install locked web dependencies and build the UI embedded in the Rust binary.
-- | @param source Source checkout containing web/bun.lock.
web :: { source :: String } -> Effect Result
web args = do
  nodeAndBun
  S.runIn (args.source <> "/web") [ "bun", "install", "--frozen-lockfile" ]
  S.runIn (args.source <> "/web") [ "bun", "run", "build" ]
  pure { status: "built" }

-- | Run locked web lint/tests and release-mode Rust tests on Linux.
-- | @param source Source checkout; dependency installation runs on this CI host.
test :: { source :: String } -> Effect Result
test args = do
  void (requireOS [ "linux" ])
  S.run [ "sudo", "apt-get", "update" ]
  S.run [ "sudo", "apt-get", "install", "-y", "--no-install-recommends", "libasound2-dev", "libdbus-1-dev", "pkg-config", "protobuf-compiler" ]
  void (D.setupRust { version: "stable", components: "rustfmt,clippy", targets: "" })
  void (web args)
  S.runIn (args.source <> "/web") [ "bun", "run", "lint" ]
  S.runIn (args.source <> "/web") [ "bun", "run", "test" ]
  S.runIn args.source [ "cargo", "test", "--release", "--locked" ]
  pure { status: "passed" }

-- | Check Rust formatting using the shared Rust installer.
-- | @param source Source checkout whose Rust code should be checked.
format :: { source :: String } -> Effect Result
format args = do
  void (D.setupRust { version: "stable", components: "rustfmt", targets: "" })
  S.runIn args.source [ "cargo", "fmt", "--all", "--check" ]
  pure { status: "passed" }

-- | Cross-build TuneIn against Debian Trixie and call the shared dpkg/rpmbuild components.
-- | @param source Source checkout to build, including packaging metadata.
-- | @param target Rust Linux triple: x86_64-unknown-linux-gnu, aarch64-unknown-linux-gnu, or armv7-unknown-linux-gnueabihf.
-- | @param version Release tag used in archive names and package metadata.
-- | @param output Destination for release assets and SHA-256 files.
linuxRelease :: { source :: String, target :: String, version :: String, output :: String } -> Effect ReleaseResult
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
  pure { status: "built", files }

-- | Build a native/cross macOS release with locked dependencies and checksums.
-- | @param source Source checkout to build.
-- | @param target x86_64-apple-darwin or aarch64-apple-darwin.
-- | @param version Release tag included in the archive filename.
-- | @param output Destination for the archive and its checksum.
macosRelease :: { source :: String, target :: String, version :: String, output :: String } -> Effect ReleaseResult
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
  pure { status: "built", files: [ archive, archive <> ".sha256" ] }

-- | Upload release assets through gh-artifacts, creating a release only when requested.
-- | @param directory Directory containing only this job's assets.
-- | @param tag Existing Git tag identifying the release.
-- | @param create Create a missing release with generated notes; otherwise upload to the existing release.
githubAssets :: { directory :: String, tag :: String, create :: Boolean } -> Effect D.ExecutionResult
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
  D.uploadRelease { project: ".", runDir: root <> "/upload", tag: args.tag, files: V.csv files, clobber: true }

-- | Discover generated Linux packages and delegate authenticated uploads to Gemfury.
-- | @param directory Directory containing this target's Debian/RPM packages.
gemfury :: { directory :: String } -> Effect { uploaded :: Boolean, files :: Array String }
gemfury args = do
  token <- S.env "FURY_TOKEN" ""
  if String.trim token == "" then pure { uploaded: false, files: [] } else do
    paths <- A.filter (\path -> String.endsWith (String.Pattern ".deb") path || String.endsWith (String.Pattern ".rpm") path) <$> S.files args.directory
    if A.null paths then pure { uploaded: false, files: [] }
      else D.uploadGemfury { files: V.csv paths }

nixSetup :: String -> Effect Unit
nixSetup cache = do
  void (requireOS [ "linux", "macos" ])
  void (D.setupNix { sudo: true, trustedUsers: [ "runner" ], extraConf: [ "experimental-features = nix-command flakes" ] })
  when (cache /= "") (void (D.setupCachix { cache }))

-- | Build the local Nix flake and push its result closure when Cachix credentials exist.
-- | @param source Source checkout containing flake.nix and flake.lock.
-- | @param cache Public Cachix cache name; empty disables Cachix configuration and uploads.
nixBuild :: { source :: String, cache :: String } -> Effect Result
nixBuild args = do
  nixSetup args.cache
  S.runIn args.source [ "nix", "build", "--no-write-lock-file", "--print-build-logs", ".#default" ]
  token <- S.env "CACHIX_AUTH_TOKEN" ""
  when (args.cache /= "" && String.trim token /= "") (S.runIn args.source [ "cachix", "push", args.cache, "./result" ])
  pure { status: "built" }

-- | Publish a selected release using nix-publish's GitHub OIDC FlakeHub function.
-- | @param source Checkout of the exact release tag.
-- | @param tag Release version or tag to publish.
flakehub :: { source :: String, tag :: String } -> Effect D.ExecutionResult
flakehub args = do
  unless (V.validVersion args.tag) (throw "Invalid release tag")
  nixSetup ""
  void D.setupFlakehub
  root <- S.fresh
  D.flakehub { project: args.source, runDir: root <> "/flakehub", name: "tsirysndr/tunein-cli", version: args.tag, visibility: "public" }

-- | Publish a selected release using nix-publish's Flakestry function and GitHub OIDC.
-- | @param source Checkout of the exact release tag.
-- | @param tag Release version or tag to publish.
-- | @param ref Exact Git commit corresponding to the selected release.
flakestry :: { source :: String, tag :: String, ref :: String } -> Effect D.ExecutionResult
flakestry args = do
  unless (V.validVersion args.tag) (throw "Invalid release tag")
  nixSetup ""
  root <- S.fresh
  D.flakestry { project: args.source, runDir: root <> "/flakestry", version: args.tag, ref: args.ref }

-- | Lint all GitHub workflows using a pinned actionlint installed through the Go setup component.
-- | @param directory Repository containing the GitHub workflow files.
check :: { directory :: String } -> Effect Result
check args = do
  void (D.setupGo { version: "1.26.2" })
  paths <- A.filter (String.endsWith (String.Pattern ".yml")) <$> S.files (args.directory <> "/.github/workflows")
  S.run ([ "go", "run", "github.com/rhysd/actionlint/cmd/actionlint@v1.7.12" ] <> paths)
  pure { status: "passed" }

-- | Build and publish the Linux target selected by release workflow environment variables.
linuxReleaseFromEnv :: Effect ReleaseResult
linuxReleaseFromEnv = do
  source <- S.env "SOURCE_DIRECTORY" "."
  target <- S.requiredEnv "TARGET"
  version <- S.requiredEnv "RELEASE_VERSION"
  let output = source <> "/.fluentci/out/" <> target
  built <- linuxRelease { source, target, version, output }
  void (githubAssets { directory: output, tag: version, create: false })
  void (gemfury { directory: output })
  pure built

-- | Build and publish the macOS target selected by release workflow environment variables.
macosReleaseFromEnv :: Effect ReleaseResult
macosReleaseFromEnv = do
  source <- S.env "SOURCE_DIRECTORY" "."
  target <- S.requiredEnv "TARGET"
  version <- S.requiredEnv "RELEASE_VERSION"
  let output = source <> "/.fluentci/out/" <> target
  built <- macosRelease { source, target, version, output }
  void (githubAssets { directory: output, tag: version, create: true })
  pure built

-- | Publish the exact checkout and release tag selected by the FlakeHub workflow.
flakehubFromEnv :: Effect D.ExecutionResult
flakehubFromEnv = do
  source <- S.env "SOURCE_DIRECTORY" "."
  tag <- S.requiredEnv "RELEASE_VERSION"
  flakehub { source, tag }

-- | Resolve the checked-out commit and publish it with the selected tag to Flakestry.
flakestryFromEnv :: Effect D.ExecutionResult
flakestryFromEnv = do
  source <- S.env "SOURCE_DIRECTORY" "."
  tag <- S.requiredEnv "RELEASE_VERSION"
  ref <- String.trim <$> S.captureIn source [ "git", "rev-parse", "HEAD" ]
  flakestry { source, tag, ref }
