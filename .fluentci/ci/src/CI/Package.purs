module CI.Package (packageRelease) where

import Prelude
import CI.Dependencies as D
import CI.Support as S
import CI.Validation as V
import Data.Either (either)
import Data.Foldable (traverse_)
import Data.Maybe (fromMaybe)
import Data.String as String
import Effect (Effect)
import Effect.Exception (throw)

-- Only TuneIn metadata and file staging are project-specific. Builders are shared components.
packageRelease :: { source :: String, target :: String, version :: String, binary :: String, output :: String } -> Effect (Array String)
packageRelease args = do
  unless (V.validVersion args.version) (throw "Invalid release version")
  info <- either throw pure (V.targetInfo args.target)
  S.run [ "mkdir", "-p", args.output ]
  output <- S.absolute args.output
  stage <- S.fresh
  let bare = fromMaybe args.version (String.stripPrefix (String.Pattern "v") args.version)
      archive = output <> "/tunein_" <> args.version <> "_" <> args.target <> ".tar.gz"
  S.run [ "mkdir", "-p", stage <> "/archive" ]
  S.run [ "cp", args.binary, stage <> "/archive/tunein" ]
  S.run [ "cp", args.source <> "/README.md", args.source <> "/LICENSE", stage <> "/archive" ]
  S.run [ "chmod", "0755", stage <> "/archive/tunein" ]
  S.run [ "tar", "-czf", archive, "-C", stage <> "/archive", "." ]
  deb <- if info.deb == "" then pure [] else do
    S.run [ "cp", "-R", args.source <> "/dist/debian/" <> info.deb, stage <> "/debian" ]
    control <- S.readText (stage <> "/debian/DEBIAN/control")
    S.writeText (stage <> "/debian/DEBIAN/control") (V.replaceVersion bare control)
    S.run [ "mkdir", "-p", stage <> "/debian/usr/bin" ]
    S.run [ "cp", args.binary, stage <> "/debian/usr/bin/tunein" ]
    S.run [ "chmod", "0755", stage <> "/debian/usr/bin/tunein" ]
    D.dpkg { directory: stage <> "/debian", arch: info.deb, output: output <> "/tunein-cli_" <> bare <> "_" <> info.deb <> ".deb" }
  rpm <- if info.rpm == "" then pure [] else do
    let root = stage <> "/rpmbuild"
    traverse_ (\name -> S.run [ "mkdir", "-p", root <> "/" <> name ]) [ "SPECS", "SOURCES", "BUILD", "BUILDROOT", "RPMS", "SRPMS" ]
    S.run [ "cp", "-R", args.source <> "/dist/rpm/amd64", root <> "/SOURCES/amd64" ]
    spec <- S.readText (args.source <> "/dist/rpm/amd64/tunein.spec")
    S.writeText (root <> "/SPECS/tunein.spec") (V.replaceVersion bare spec)
    S.run [ "mkdir", "-p", root <> "/SOURCES/amd64/usr/bin" ]
    S.run [ "cp", args.binary, root <> "/SOURCES/amd64/usr/bin/tunein" ]
    S.run [ "chmod", "0755", root <> "/SOURCES/amd64/usr/bin/tunein" ]
    D.rpmbuild { directory: root, arch: info.rpm, output, spec: "tunein.spec" }
  let assets = [ archive ] <> deb <> rpm
  traverse_ S.checksum assets
  S.run [ "rm", "-rf", "--", stage ]
  pure (assets <> map (_ <> ".sha256") assets)
