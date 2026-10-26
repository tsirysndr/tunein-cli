module CI.Nix (nixArgs, flake, package) where

import Prelude
import Dagger (Client)
import Dagger.Container as Container
import Dagger.CurrentModule as CurrentModule
import Dagger.Directory as Directory
import Dagger.Modules as Module
import Dagger.Query as Query
import Dagger.Types (Directory, Secret)
import Data.Maybe (Maybe(..))
import Effect.Aff (Aff)

type NixArgs = { source :: Directory, cachixToken :: Secret, runId :: String }

nixArgs :: Module.Arguments NixArgs
nixArgs = ado
  source <- Module.argument "source"
  cachixToken <- Module.argument "cachixToken"
  runId <- Module.argument "runId"
  in { source, cachixToken, runId }

build :: String -> Client -> NixArgs -> Aff String
build mode client args = do
  let
    image = Query.container client # Container.from "nixos/nix:2.28.5"
    store = Query.cacheVolume "tunein-cli-nix-store" client
    -- The cache also contains Nix itself; seed it before mounting over /nix.
    seed = Query.container client
      # Container.from "node:22-bookworm-slim"
      # Container.withDirectory "/seed" (Container.directory "/nix" image)
      # Container.withMountedCache "/nix" store
      # Container.withEnvVariable "NIX_RUN" args.runId
      # Container.withExec [ "cp", "-an", "/seed/.", "/nix/" ]
  void $ Container.sync seed
  void $ Container.sync $ image
    # Container.withMountedCache "/nix" store
    # Container.withEnvVariable "NIX_REMOTE" "local"
    # Container.withEnvVariable "NIX_CONFIG" "experimental-features = nix-command flakes\nsandbox = false\nbuild-users-group =\n"
    # Container.withEnvVariable "NIX_PATH" "nixpkgs=channel:nixos-unstable"
    # Container.withDirectory "/pipeline" (Query.currentModule client # CurrentModule.source # Directory.directory "scripts")
    # Container.withDirectoryWith "/src" args.source
        (Container.defaultWithDirectoryOpts
          { exclude = Just [ ".git", ".dagger", ".source", "**/target", "**/node_modules", "**/.env", "**/.env.*", "result", "result-*", "*.db*", "covers", "smolsonic.toml" ] })
    # Container.withSecretVariable "CACHIX_AUTH_TOKEN" args.cachixToken
    # Container.withEnvVariable "NIX_RUN" args.runId
    # Container.withExec [ "sh", "/pipeline/nix.sh", mode ]
  pure "Nix build completed"

flake :: Client -> NixArgs -> Aff String
flake = build "flake"

package :: Client -> NixArgs -> Aff String
package = build "package"
