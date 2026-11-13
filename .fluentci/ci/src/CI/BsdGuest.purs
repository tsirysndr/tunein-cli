module CI.BsdGuest (main) where
import Prelude
import CI.GuestHost as H
import CI.Validation (validVersion)
import Effect (Effect)
import Effect.Exception (throw)

main :: Effect Unit
main = do
  version <- H.env "RELEASE_VERSION"
  target <- H.env "ASSET_TARGET"
  os <- H.platform
  arch <- H.arch
  unless (os == "freebsd" || os == "netbsd") (throw "BSD release requires FreeBSD or NetBSD")
  unless (validVersion version) (throw "Invalid release version")
  unless (arch == "arm64" || arch == "x64") (throw "Unsupported BSD guest CPU")
  path <- H.env "PATH"
  H.setEnv "PATH" ("/usr/pkg/bin:/usr/pkg/sbin:/usr/local/bin:/usr/local/sbin:" <> path)
  let expected = (if arch == "arm64" then "aarch64" else "x86_64") <> "-unknown-" <> os
  unless (target == expected) (throw "BSD guest architecture does not match ASSET_TARGET")
  if os == "freebsd" then H.run [ "pkg", "install", "-y", "rust", "protobuf", "alsa-lib", "pkgconf", "dbus" ]
    else H.run [ "/usr/sbin/pkg_add", "rust", "protobuf", "alsa-lib", "pkgconf", "dbus" ]
  -- Node bindings inherit the VM environment; env is a literal executable, not a shell.
  H.run [ "env", "PKG_CONFIG=pkgconf", "cargo", "build", "--release", "--locked", "--bin", "tunein" ]
  let output = ".fluentci/out/" <> target
      name = "tunein_" <> version <> "_" <> target <> ".tar.gz"
      archive = output <> "/" <> name
  H.run [ "mkdir", "-p", output ]
  H.run [ "tar", "-czf", archive, "-C", "target/release", "tunein" ]
  checksum <- H.digest archive
  H.writeText (archive <> ".sha256") (checksum <> "  " <> name <> "\n")
