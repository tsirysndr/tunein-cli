module CI.Build (buildArgs, linuxRelease, sourceArgs, test, format) where

import Prelude
import Dagger (Client)
import Dagger.Container as Container
import Dagger.CurrentModule as CurrentModule
import Dagger.Directory as Directory
import Dagger.Modules as Module
import Dagger.Query as Query
import Dagger.Types (Container, Directory, Platform(..))
import Data.Maybe (Maybe(..))
import Effect.Aff (Aff, throwError)
import Effect.Exception (error)

type BuildArgs = { source :: Directory, target :: String, version :: String }

buildArgs :: Module.Arguments BuildArgs
buildArgs = ado
  source <- Module.argument "source"
  target <- Module.argument "target"
  version <- Module.argument "version"
  in { source, target, version }

scripts :: Client -> Directory
scripts client = Query.currentModule client # CurrentModule.source # Directory.directory "scripts"

withSource :: Directory -> Container -> Container
withSource source = Container.withDirectoryWith "/src" source
  (Container.defaultWithDirectoryOpts
    { exclude = Just [ ".git", ".dagger", ".source", ".fluentci", "target", "**/node_modules", "**/.env", "**/.env.*", "result", "result-*", "dist/*.tar.gz*", "dist/*.deb*", "dist/*.rpm*", "dist/staging", "dist/rpmbuild", "*.db*", "covers", "smolsonic.toml" ] })

withCargoCaches :: Client -> String -> Container -> Container
withCargoCaches client target container = container
  # Container.withEnvVariable "CARGO_HOME" "/cargo"
  # Container.withMountedCache "/cargo/registry" (Query.cacheVolume "tunein-cli-cargo-registry" client)
  # Container.withMountedCache "/cargo/git" (Query.cacheVolume "tunein-cli-cargo-git" client)
  # Container.withMountedCache "/src/target" (Query.cacheVolume ("tunein-cli-target-" <> target) client)

web :: Client -> Directory -> Container
web client source = Query.container client
  # Container.from "node:22-bookworm-slim"
  # Container.withFile "/usr/local/bin/bun" (Query.container client # Container.from "oven/bun:1.4.2" # Container.file "/usr/local/bin/bun")
  # Container.withMountedCache "/root/.bun/install/cache" (Query.cacheVolume "tunein-cli-bun" client)
  # Container.withDirectoryWith "/web" (Directory.directory "web" source)
      (Container.defaultWithDirectoryOpts { exclude = Just [ "node_modules", "dist", ".env", ".env.*" ] })
  # Container.withWorkdir "/web"
  # Container.withExec [ "bun", "install", "--frozen-lockfile" ]
  # Container.withExec [ "bun", "run", "build" ]

linuxRelease :: Client -> BuildArgs -> Aff Directory
linuxRelease client args = do
  label <- case args.target of
    "x86_64-unknown-linux-gnu" -> pure "linux-amd64"
    "aarch64-unknown-linux-gnu" -> pure "linux-aarch64"
    "armv7-unknown-linux-gnueabihf" -> pure "linux-armhf"
    _ -> throwError (error "Unsupported Linux release target")
  let
    environment = Query.containerWith
      (Query.defaultContainerOpts { platform = Just (Platform "linux/amd64") }) client
      # Container.from "rust:1.89-bullseye"
      # Container.withDirectory "/pipeline" (scripts client)
      # Container.withEnvVariable "TARGET" args.target
      # Container.withExec [ "bash", "/pipeline/cross-deps.sh" ]
      # Container.withExec [ "rustup", "target", "add", args.target ]
    built = environment
      # withSource args.source
      # Container.withDirectory "/src/web/dist" (Container.directory "/web/dist" (web client args.source))
      # Container.withWorkdir "/src"
      # withCargoCaches client args.target
      # Container.withExec [ "bash", "/pipeline/build-rust.sh" ]
      -- Cache mounts cannot be exported directly: materialize the executable.
      # Container.withExec [ "mkdir", "-p", "/binary" ]
      # Container.withExec [ "cp", "target/" <> args.target <> "/release/tunein", "/binary/tunein" ]
    packaged = Query.container client
      # Container.from "node:22-bookworm-slim"
      # Container.withExec [ "sh", "-c", "apt-get update && apt-get install -y --no-install-recommends dpkg-dev rpm libdigest-sha-perl && rm -rf /var/lib/apt/lists/*" ]
      # Container.withDirectory "/pipeline" (scripts client)
      # withSource args.source
      # Container.withFile "/src/target/tunein" (Container.file "/binary/tunein" built)
      # Container.withWorkdir "/src"
      # Container.withEnvVariable "VERSION" args.version
      # Container.withEnvVariable "TARGET" args.target
      # Container.withEnvVariable "LABEL" label
      # Container.withExec [ "bash", "/pipeline/package.sh" ]
  pure $ Container.directory "/out" packaged

type SourceArgs = { source :: Directory }

sourceArgs :: Module.Arguments SourceArgs
sourceArgs = ado
  source <- Module.argument "source"
  in { source }

test :: Client -> SourceArgs -> Aff String
test client args = do
  let ui = web client args.source # Container.withExec [ "bun", "run", "test" ]
  void $ Container.sync ui
  void $ Container.sync $ Query.container client
    # Container.from "rust:bookworm"
    # Container.withExec [ "sh", "-c", "apt-get update && apt-get install -y --no-install-recommends libasound2-dev libdbus-1-dev pkg-config protobuf-compiler && rm -rf /var/lib/apt/lists/*" ]
    # withSource args.source
    # Container.withDirectory "/src/web/dist" (Container.directory "/web/dist" ui)
    # Container.withWorkdir "/src"
    # withCargoCaches client "test"
    # Container.withExec [ "cargo", "test", "--locked" ]
  pure "Web UI and Rust tests passed"

format :: Client -> SourceArgs -> Aff String
format client args = do
  void $ Container.sync $ Query.container client
    # Container.from "rust:bookworm"
    # Container.withExec [ "rustup", "component", "add", "rustfmt" ]
    # withSource args.source
    # Container.withWorkdir "/src"
    # Container.withExec [ "cargo", "fmt", "--all", "--check" ]
  pure "Rust formatting passed"
