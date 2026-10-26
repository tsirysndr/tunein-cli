module CI.Build (buildArgs, linuxRelease, sourceArgs, test, format) where

import Prelude
import CI.Package as Package
import CI.Cross as Cross
import Dagger (Client)
import Dagger.Container as Container
import Dagger.Directory as Directory
import Dagger.Modules as Module
import Dagger.Query as Query
import Dagger.Types (Container, Directory)
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
  environment <- Cross.environment client args.target
  let
    built = environment
      # withSource args.source
      # Container.withDirectory "/src/web/dist" (Container.directory "/web/dist" (web client args.source))
      # Container.withWorkdir "/src"
      # withCargoCaches client args.target
      -- Preserve the application's protobuf compatibility wrapper for older tags.
      # Container.withExec [ "mv", "/usr/bin/protoc", "/usr/bin/_protoc" ]
      # Container.withExec [ "install", "-m", "0755", "tools/protoc", "/usr/bin/protoc" ]
      # Container.withExec [ "cargo", "build", "--release", "--locked", "--target", args.target, "--bin", "tunein" ]
      -- Cache mounts cannot be exported directly: materialize the executable.
      # Container.withExec [ "mkdir", "-p", "/binary" ]
      # Container.withExec [ "cp", "target/" <> args.target <> "/release/tunein", "/binary/tunein" ]
  Package.packageRelease client
    { source: args.source
    , binary: Container.file "/binary/tunein" built
    , target: args.target
    , label
    , version: args.version
    }

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
    # Container.withExec [ "apt-get", "update" ]
    # Container.withExec [ "apt-get", "install", "-y", "--no-install-recommends", "libasound2-dev", "libdbus-1-dev", "pkg-config", "protobuf-compiler" ]
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
