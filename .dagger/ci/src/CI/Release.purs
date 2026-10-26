module CI.Release (releaseArgs, publishRelease, uploadRelease, furyArgs, publishGemfury) where

import Prelude
import Dagger (Client)
import Dagger.Container as Container
import Dagger.CurrentModule as CurrentModule
import Dagger.Directory as Directory
import Dagger.Modules as Module
import Dagger.Query as Query
import Dagger.Types (Container, Directory, Secret)
import Effect.Aff (Aff)

type ReleaseArgs = { assets :: Directory, tag :: String, token :: Secret, runId :: String }

releaseArgs :: Module.Arguments ReleaseArgs
releaseArgs = ado
  assets <- Module.argument "assets"
  tag <- Module.argument "tag"
  token <- Module.argument "token"
  runId <- Module.argument "runId"
  in { assets, tag, token, runId }

publisher :: Client -> Container
publisher client = Query.container client
  # Container.from "node:22-bookworm-slim"
  # Container.withExec [ "sh", "-c", "apt-get update && apt-get install -y --no-install-recommends gh curl ca-certificates && rm -rf /var/lib/apt/lists/*" ]
  # Container.withDirectory "/pipeline" (Query.currentModule client # CurrentModule.source # Directory.directory "scripts")

release :: String -> Client -> ReleaseArgs -> Aff String
release mode client args = do
  void $ Container.sync $ publisher client
    # Container.withDirectory "/assets" args.assets
    # Container.withWorkdir "/assets"
    # Container.withSecretVariable "GH_TOKEN" args.token
    # Container.withEnvVariable "GH_REPO" "tsirysndr/tunein-cli"
    # Container.withEnvVariable "RELEASE_TAG" args.tag
    # Container.withEnvVariable "WORKFLOW_RUN" args.runId
    # Container.withExec [ "bash", "/pipeline/release.sh", mode ]
  pure "Release assets published"

publishRelease :: Client -> ReleaseArgs -> Aff String
publishRelease = release "create"

uploadRelease :: Client -> ReleaseArgs -> Aff String
uploadRelease = release "upload"

type FuryArgs = { assets :: Directory, account :: Secret, token :: Secret, runId :: String }

furyArgs :: Module.Arguments FuryArgs
furyArgs = ado
  assets <- Module.argument "assets"
  account <- Module.argument "account"
  token <- Module.argument "token"
  runId <- Module.argument "runId"
  in { assets, account, token, runId }

publishGemfury :: Client -> FuryArgs -> Aff String
publishGemfury client args = do
  void $ Container.sync $ publisher client
    # Container.withDirectory "/assets" args.assets
    # Container.withSecretVariable "FURY_TOKEN" args.token
    # Container.withSecretVariable "FURY_ACCOUNT" args.account
    # Container.withEnvVariable "WORKFLOW_RUN" args.runId
    # Container.withExec [ "bash", "/pipeline/gemfury.sh" ]
  pure "Gemfury packages published"
