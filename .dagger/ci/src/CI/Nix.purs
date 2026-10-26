module CI.Nix (nixArgs, flake, package) where

import Prelude
import Dagger (Client)
import Dagger.Container as Container
import Dagger.Modules as Module
import Dagger.Query as Query
import Dagger.Types (Directory, Secret)
import Data.Maybe (Maybe(..))
import Dagger.Secret as Secret
import Data.String (trim)
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
  let
    environment = image
      # Container.withMountedCache "/nix" store
      # Container.withEnvVariable "NIX_REMOTE" "local"
      # Container.withEnvVariable "NIX_CONFIG" "experimental-features = nix-command flakes\nsandbox = false\nbuild-users-group =\n"
      # Container.withEnvVariable "NIX_PATH" "nixpkgs=channel:nixos-unstable"
      # Container.withDirectoryWith "/src" args.source
          (Container.defaultWithDirectoryOpts
            { exclude = Just [ ".git", ".dagger", ".source", "**/target", "**/node_modules", "**/.env", "**/.env.*", "result", "result-*", "*.db*", "covers", "smolsonic.toml" ] })
      # Container.withSecretVariable "CACHIX_AUTH_TOKEN" args.cachixToken
      # Container.withEnvVariable "NIX_RUN" args.runId
  cachix <- trim <$> Container.stdout (environment
    # Container.withWorkdir "/src"
    # Container.withExec [ "nix", "build", "--inputs-from", ".", "nixpkgs#cachix", "--no-link", "--print-out-paths" ])
  let configured = environment
        # Container.withWorkdir "/src"
        # Container.withExec [ cachix <> "/bin/cachix", "use", "tsirysndr" ]
      built = configured # Container.withExec
        (if mode == "package" then [ "nix-build", "-E", "with import <nixpkgs> { }; callPackage ./nix/package.nix { }" ]
         else [ "nix", "build", "-L" ])
  token <- Secret.plaintext args.cachixToken
  void $ Container.sync $ if token == "" then built
    else built # Container.withExec [ cachix <> "/bin/cachix", "push", "tsirysndr", "./result" ]
  pure "Nix build completed"

flake :: Client -> NixArgs -> Aff String
flake = build "flake"

package :: Client -> NixArgs -> Aff String
package = build "package"
