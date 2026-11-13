module CI.Cross (build) where

import Prelude
import CI.Support as S
import CI.Validation as V
import Data.Array as A
import Data.Either (either)
import Data.Foldable (traverse_)
import Data.String as String
import Effect (Effect)
import Effect.Exception (catchException, throw, throwException, try)

-- Use the Debian Trixie runtime baseline and literal cross-linker/sysroot setup.
build :: { source :: String, target :: String, output :: String } -> Effect String
build args = do
  info <- either throw pure (V.targetInfo args.target)
  source <- S.absolute args.source
  S.run [ "docker", "info" ]
  work <- S.fresh
  S.run ([ "tar", "-cf", work <> "/source.tar" ] <> A.concatMap (\path -> [ "--exclude=" <> path ])
    [ ".git", ".ci-tooling", ".fluentci", "target", "web/node_modules", ".env", "*/.env", "*/.env.*", "dist/*.tar.gz*", "dist/*.deb", "dist/*.rpm" ] <> [ "-C", source, "." ])
  id <- String.trim <$> S.captureIn "." ([ "docker", "create", "--platform", "linux/amd64"
    , "--mount", "type=volume,source=tunein-cargo-registry,target=/usr/local/cargo/registry"
    , "--mount", "type=volume,source=tunein-cargo-git,target=/usr/local/cargo/git"
    , "--mount", "type=volume,source=tunein-target-" <> args.target <> ",target=/src/target"
    , "rust:trixie@sha256:0b0ea86881537b3f645aace7e8e39556289f8787db057317c8d4978548213b9e", "sleep", "infinity" ])
  let exec argv = S.run ([ "docker", "exec", id ] <> argv)
      clean = do
        catchException (const (pure unit)) (S.run [ "docker", "rm", "-f", id ])
        catchException (const (pure unit)) (S.run [ "rm", "-rf", "--", work ])
  result <- try do
    S.run [ "docker", "start", id ]
    exec [ "mkdir", "-p", "/src", "/binary" ]
    S.run [ "docker", "cp", work <> "/source.tar", id <> ":/source.tar" ]
    exec [ "tar", "-xf", "/source.tar", "-C", "/src" ]
    when (info.arch /= "") (exec [ "dpkg", "--add-architecture", info.arch ])
    exec [ "apt-get", "update" ]
    exec [ "apt-get", "install", "-y", "--no-install-recommends", "build-essential", "libasound2-dev", "libdbus-1-dev", "protobuf-compiler", "pkg-config" ]
    exec [ "rustup", "target", "add", args.target ]
    when (info.arch /= "") do
      exec [ "apt-get", "install", "-y", "--no-install-recommends", "gcc-" <> info.triple, "libc6-dev-" <> info.arch <> "-cross", "symlinks" ]
      exec [ "mkdir", "-p", "/build/sysroot", "/tmp/debs" ]
      let packages = [ "libasound2t64", "libasound2-dev", "libdbus-1-dev", "libdbus-1-3", "libsystemd-dev", "libsystemd0", "libcap2", "libcap-dev", "libpsx2", "libgcrypt20", "libgcrypt20-dev", "libgpg-error0", "libgpg-error-dev", "liblz4-1", "liblz4-dev", "libxxhash0", "libxxhash-dev", "liblzma5", "liblzma-dev", "libzstd1", "libzstd-dev" ]
      S.run ([ "docker", "exec", "--workdir", "/tmp/debs", id, "apt-get", "download" ] <> map (_ <> ":" <> info.arch) packages)
      files <- S.captureIn "." [ "docker", "exec", id, "find", "/tmp/debs", "-name", "*.deb", "-type", "f" ]
      traverse_ (\file -> exec [ "dpkg", "-x", file, "/build/sysroot" ]) (A.filter (_ /= "") (String.split (String.Pattern "\n") (String.trim files)))
      exec [ "symlinks", "-cr", "/build/sysroot" ]
    let paths = [ "/usr/" <> info.triple <> "/lib", "/build/sysroot/usr/lib/" <> info.triple, "/build/sysroot/lib/" <> info.triple ]
        flags = String.joinWith " " (map ("-Clink-arg=-l" <> _) [ "systemd", "cap", "gcrypt", "gpg-error", "lz4", "lzma", "psx", "xxhash", "zstd" ] <> [ "-C", "linker=" <> info.triple <> "-gcc" ] <> map ("-L" <> _) paths <> map ("-Clink-arg=-Wl,-rpath-link," <> _) paths)
        variables = if info.arch == "" then [] else A.concatMap (\value -> [ "--env", value ])
          [ "PKG_CONFIG_ALLOW_CROSS=1", "PKG_CONFIG_LIBDIR=/build/sysroot/usr/lib/" <> info.triple <> "/pkgconfig:/build/sysroot/usr/share/pkgconfig", "PKG_CONFIG_SYSROOT_DIR=/build/sysroot", "C_INCLUDE_PATH=/build/sysroot/usr/include", "RUSTFLAGS=" <> flags ]
    S.run ([ "docker", "exec", "--workdir", "/src" ] <> variables <> [ id, "cargo", "build", "--release", "--locked", "--target", args.target, "--bin", "tunein" ])
    exec [ "cp", "/src/target/" <> args.target <> "/release/tunein", "/binary/tunein" ]
    S.run [ "mkdir", "-p", args.output ]
    output <- S.absolute args.output
    S.run [ "docker", "cp", id <> ":/binary/tunein", output <> "/tunein" ]
    pure (output <> "/tunein")
  clean
  either throwException pure result
