module CI.Cross (environment) where

import Prelude
import Dagger (Client)
import Dagger.Container as Container
import Dagger.Directory as Directory
import Dagger.Query as Query
import Dagger.Types (Container, Platform(..))
import Data.Foldable (foldl)
import Data.Maybe (Maybe(..))
import Data.String (joinWith)
import Effect.Aff (Aff, throwError)
import Effect.Exception (error)

type CrossTarget = { arch :: String, triple :: String }

environment :: Client -> String -> Aff Container
environment client target = do
  cross <- case target of
    "x86_64-unknown-linux-gnu" -> pure Nothing
    "aarch64-unknown-linux-gnu" -> pure $ Just { arch: "arm64", triple: "aarch64-linux-gnu" }
    "armv7-unknown-linux-gnueabihf" -> pure $ Just { arch: "armhf", triple: "arm-linux-gnueabihf" }
    _ -> throwError (error "Unsupported Linux release target")
  let
    image = Query.containerWith
      (Query.defaultContainerOpts { platform = Just (Platform "linux/amd64") }) client
      # Container.from "rust:1.89-bullseye"
    configured = case cross of
      Nothing -> image
      Just platform -> image # Container.withExec [ "dpkg", "--add-architecture", platform.arch ]
    base = configured
      # Container.withExec [ "apt-get", "update" ]
      # Container.withExec [ "apt-get", "install", "-y", "--no-install-recommends", "build-essential", "libasound2-dev", "libdbus-1-dev", "protobuf-compiler", "pkg-config" ]
      # Container.withExec [ "rustup", "target", "add", target ]
  case cross of
    Nothing -> pure base
    Just platform -> prepareSysroot platform base

prepareSysroot :: CrossTarget -> Container -> Aff Container
prepareSysroot platform base = do
  let
    packages =
      [ "libasound2", "libasound2-dev", "libdbus-1-dev", "libdbus-1-3"
      , "libsystemd-dev", "libsystemd0", "libcap2", "libcap-dev"
      , "libgcrypt20", "libgcrypt20-dev", "libgpg-error0", "libgpg-error-dev"
      , "liblz4-1", "liblz4-dev", "libxxhash0", "libxxhash-dev"
      , "liblzma5", "liblzma-dev", "libzstd1", "libzstd-dev"
      ]
    downloaded = base
      # Container.withExec [ "apt-get", "install", "-y", "--no-install-recommends", "gcc-" <> platform.triple, "libc6-dev-" <> platform.arch <> "-cross", "symlinks" ]
      # Container.withExec [ "mkdir", "-p", "/build/sysroot", "/tmp/debs" ]
      # Container.withWorkdir "/tmp/debs"
      # Container.withExec ([ "apt-get", "download" ] <> map (_ <> ":" <> platform.arch) packages)
  debs <- Directory.glob "*.deb" (Container.directory "/tmp/debs" downloaded)
  let extracted = foldl (\container file -> container
        # Container.withExec [ "dpkg", "-x", "/tmp/debs/" <> file, "/build/sysroot" ]) downloaded debs
  pure $ extracted
    # Container.withExec [ "symlinks", "-cr", "/build/sysroot" ]
    # Container.withEnvVariable "PKG_CONFIG_ALLOW_CROSS" "1"
    # Container.withEnvVariable "PKG_CONFIG_LIBDIR" ("/build/sysroot/usr/lib/" <> platform.triple <> "/pkgconfig:/build/sysroot/usr/share/pkgconfig")
    # Container.withEnvVariable "PKG_CONFIG_SYSROOT_DIR" "/build/sysroot"
    # Container.withEnvVariable "C_INCLUDE_PATH" "/build/sysroot/usr/include"
    # Container.withEnvVariable "RUSTFLAGS" (rustFlags platform.triple)

rustFlags :: String -> String
rustFlags triple = joinWith " " $
  map ("-Clink-arg=-l" <> _) [ "systemd", "cap", "gcrypt", "gpg-error", "lz4", "lzma", "psx", "xxhash", "zstd" ]
  <> [ "-C", "linker=" <> triple <> "-gcc" ]
  <> map ("-L" <> _) paths
  <> map ("-Clink-arg=-Wl,-rpath-link," <> _) paths
  where
  paths = [ "/usr/" <> triple <> "/lib", "/build/sysroot/usr/lib/" <> triple, "/build/sysroot/lib/" <> triple ]
