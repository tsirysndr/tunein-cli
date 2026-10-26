module CI.Release (releaseArgs, publishRelease, uploadRelease, furyArgs, publishGemfury) where

import Prelude
import Dagger (Client)
import Dagger.Container as Container
import Dagger.Modules as Module
import Dagger.Query as Query
import Dagger.Types (Container, Directory, Secret, ReturnType(..))
import CI.Assets as Assets
import Data.Array (null)
import Data.Foldable (foldM)
import Data.Maybe (Maybe(..))
import Dagger.Secret as Secret
import Effect.Aff (Aff, throwError)
import Effect.Exception (error)

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
  # Container.withExec [ "apt-get", "update" ]
  # Container.withExec [ "apt-get", "install", "-y", "--no-install-recommends", "gh", "ca-certificates" ]

release :: Boolean -> Client -> ReleaseArgs -> Aff String
release create client args = do
  files <- Assets.releaseFiles args.assets
  when (null files) $ throwError (error "No release assets supplied")
  let base = publisher client
        # Container.withDirectory "/assets" args.assets
        # Container.withWorkdir "/assets"
        # Container.withSecretVariable "GH_TOKEN" args.token
        # Container.withEnvVariable "GH_REPO" "tsirysndr/tunein-cli"
        # Container.withEnvVariable "WORKFLOW_RUN" args.runId
  ready <- if create then do
    status <- Container.exitCode $ base
      # Container.withExecWith [ "gh", "release", "view", args.tag ]
          (Container.defaultWithExecOpts { expect = Just ReturnTypeAny })
    pure $ base # Container.withExec
      (if status == 0 then [ "gh", "release", "edit", args.tag, "--draft=false", "--prerelease=false" ]
       else [ "gh", "release", "create", args.tag, "--verify-tag", "--generate-notes", "--draft=false", "--prerelease=false" ])
    else pure base
  void $ Container.sync $ ready # Container.withExec
    ([ "gh", "release", "upload", args.tag, "--clobber" ] <> map ("./" <> _) files)
  pure "Release assets published"

publishRelease :: Client -> ReleaseArgs -> Aff String
publishRelease = release true

uploadRelease :: Client -> ReleaseArgs -> Aff String
uploadRelease = release false

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
  -- Only inspect whether credentials were supplied; never put them in arguments.
  token <- Secret.plaintext args.token
  account <- Secret.plaintext args.account
  if token == "" || account == "" then pure "Gemfury credentials absent; upload skipped"
  else do
    files <- Assets.packageFiles args.assets
    let base = Query.container client
          # Container.from "curlimages/curl:8.12.1"
          # Container.withDirectory "/assets" args.assets
          # Container.withSecretVariable "FURY_TOKEN" args.token
          # Container.withSecretVariable "FURY_ACCOUNT" args.account
          # Container.withEnvVariable "WORKFLOW_RUN" args.runId
    uploaded <- foldM (\container name -> pure $ container # Container.withExec
      [ "curl", "--fail", "--silent", "--show-error"
      , "--variable", "%FURY_TOKEN", "--variable", "%FURY_ACCOUNT"
      , "--expand-user", "{{FURY_TOKEN}}:"
      , "--form", "package=@/assets/" <> name
      , "--expand-url", "https://push.fury.io/{{FURY_ACCOUNT:url}}/"
      ]) base files
    void $ Container.sync uploaded
    pure "Gemfury packages published"
