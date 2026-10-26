module CI.Package (PackageArgs, packageRelease, validVersion) where

import Prelude
import CI.Assets as Assets
import Dagger (Client)
import Dagger.Container as Container
import Dagger.Directory as Directory
import Dagger.File as File
import Dagger.Query as Query
import Dagger.Types (Container, Directory, File)
import Data.Array (head, last)
import Data.Foldable (all, foldM)
import Data.Maybe (Maybe(..), fromMaybe)
import Data.String (Pattern(..), contains, joinWith, split, stripPrefix)
import Data.String.CodeUnits (singleton, toCharArray)
import Effect.Aff (Aff, throwError)
import Effect.Exception (error)

validVersion :: String -> Boolean
validVersion version =
  let bare = fromMaybe version (stripPrefix (Pattern "v") version)
  in case head (toCharArray bare) of
    Just first -> first >= '0' && first <= '9' && all
      (\c -> contains (Pattern (singleton c)) "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789.+~-")
      (toCharArray bare)
    Nothing -> false

replaceVersion :: String -> String -> String
replaceVersion version = split (Pattern "\n") >>> map
  (\line -> case stripPrefix (Pattern "Version:") line of
    Just _ -> "Version:        " <> version
    Nothing -> line) >>> joinWith "\n"

type PackageArgs =
  { source :: Directory
  , binary :: File
  , target :: String
  , label :: String
  , version :: String
  }

packageRelease :: Client -> PackageArgs -> Aff Directory
packageRelease client { source, binary, target, label: _, version } = do
  unless (validVersion version) $ throwError (error "Invalid package version")
  arch <- case target of
    "x86_64-unknown-linux-gnu" -> pure "amd64"
    "aarch64-unknown-linux-gnu" -> pure "arm64"
    "armv7-unknown-linux-gnueabihf" -> pure ""
    _ -> throwError (error "Unsupported Linux release target")
  let
    bare = fromMaybe version (stripPrefix (Pattern "v") version)
    archive = "tunein_" <> version <> "_" <> target <> ".tar.gz"
    stage = Query.directory client
      # Directory.withFile "tunein" binary
      # Directory.withFile "README.md" (Directory.file "README.md" source)
      # Directory.withFile "LICENSE" (Directory.file "LICENSE" source)

    base = Query.container client
      # Container.from "node:22-bookworm-slim"
      # Container.withExec [ "apt-get", "update" ]
      # Container.withExec [ "apt-get", "install", "-y", "--no-install-recommends", "dpkg-dev", "rpm", "libdigest-sha-perl" ]
      # Container.withDirectory "/stage" stage
      # Container.withExec [ "chmod", "0755", "/stage/tunein" ]
      # Container.withExec [ "mkdir", "-p", "/out" ]
      # Container.withExec [ "tar", "-C", "/stage", "-czf", "/out/" <> archive, "." ]
  deb <- if arch == "" then pure base else do
    let metadata = Directory.directory ("dist/debian/" <> arch) source
    control <- File.contents (Directory.file "DEBIAN/control" metadata)
    pure $ base
      # Container.withDirectory "/debian" (metadata
          # Directory.withNewFile "DEBIAN/control" (replaceVersion bare control)
          # Directory.withFile "usr/bin/tunein" binary)
      # Container.withExec [ "chmod", "0755", "/debian/usr/bin/tunein" ]
      # Container.withExec [ "dpkg-deb", "--build", "--root-owner-group", "/debian", "/out/tunein-cli_" <> bare <> "_" <> arch <> ".deb" ]
  rpm <- if target /= "x86_64-unknown-linux-gnu" then pure deb else do
    let metadata = Directory.directory "dist/rpm/amd64" source
    spec <- File.contents (Directory.file "tunein.spec" metadata)
    let built = deb
          # Container.withDirectory "/rpmbuild/SOURCES/amd64" (metadata
              # Directory.withNewFile "tunein.spec" (replaceVersion bare spec)
              # Directory.withFile "usr/bin/tunein" binary)
          # Container.withExec [ "chmod", "0755", "/rpmbuild/SOURCES/amd64/usr/bin/tunein" ]
          # Container.withExec ([ "mkdir", "-p" ] <> map ("/rpmbuild/" <> _) [ "BUILD", "BUILDROOT", "RPMS", "SOURCES", "SPECS", "SRPMS" ])
          # Container.withExec [ "rpmbuild", "--define", "_topdir /rpmbuild", "--define", "_sourcedir /rpmbuild/SOURCES", "--target", "x86_64", "-bb", "/rpmbuild/SOURCES/amd64/tunein.spec" ]
    files <- Directory.glob "**/*.rpm" (Container.directory "/rpmbuild/RPMS" built)
    foldM (\container path -> pure $ container # Container.withFile
      ("/out/" <> fromMaybe path (last (split (Pattern "/") path)))
      (Container.file ("/rpmbuild/RPMS/" <> path) built)) built files
  files <- Assets.releaseFiles (Container.directory "/out" rpm)
  checksummed <- foldM checksum rpm files
  pure $ Container.directory "/out" checksummed
  where
  checksum :: Container -> String -> Aff Container
  checksum container name = pure $ container
    # Container.withWorkdir "/out"
    # Container.withExecWith [ "shasum", "-a", "256", name ]
        (Container.defaultWithExecOpts { redirectStdout = Just ("/out/" <> name <> ".sha256") })
