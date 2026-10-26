module CI.Cache (cacheArgs, restoreArgs, saveCaches, restoreCaches) where

import Prelude
import Dagger (Client)
import Dagger.Container as Container
import Dagger.Directory as Directory
import Dagger.Types (Directory, ExistsType(..))
import Data.Maybe (Maybe(..))
import Dagger.Modules as Module
import Dagger.Query as Query
import Data.Foldable (all, foldM)
import Data.String (Pattern(..), contains, split)
import Effect.Aff (Aff, throwError)
import Effect.Exception (error)

type CacheArgs = { volumes :: String, runId :: String }

cacheArgs :: Module.Arguments CacheArgs
cacheArgs = ado
  volumes <- Module.argument "volumes"
  runId <- Module.argument "runId"
  in { volumes, runId }

cacheNames :: String -> Aff (Array String)
cacheNames value = do
  let names = split (Pattern ",") value
  if all (\name -> name /= "" && not (contains (Pattern "/") name) && not (contains (Pattern "..") name)) names then
    pure names
  else throwError (error "Expected comma-separated cache volume names without path separators")

saveCaches :: Client -> CacheArgs -> Aff Directory
saveCaches client args = do
  names <- cacheNames args.volumes
  let base = Query.container client
        # Container.from "node:24-bookworm-slim"
        # Container.withEnvVariable "CACHE_SNAPSHOT" args.runId
  snapshot <- foldM (\container name -> pure $ container
    # Container.withMountedCache "/cache" (Query.cacheVolume name client)
    # Container.withExec [ "mkdir", "-p", "/out/" <> name ]
    # Container.withExec [ "cp", "-a", "/cache/.", "/out/" <> name ]) base names
  pure $ Container.directory "/out" snapshot

type RestoreArgs = { source :: Directory, volumes :: String, runId :: String }

restoreArgs :: Module.Arguments RestoreArgs
restoreArgs = ado
  source <- Module.argument "source"
  volumes <- Module.argument "volumes"
  runId <- Module.argument "runId"
  in { source, volumes, runId }

restoreCaches :: Client -> RestoreArgs -> Aff String
restoreCaches client args = do
  names <- cacheNames args.volumes
  let base = Query.container client
        # Container.from "node:24-bookworm-slim"
        # Container.withDirectory "/snapshot" args.source
        # Container.withEnvVariable "CACHE_RESTORE" args.runId
  restored <- foldM (\container name -> do
    exists <- Directory.existsWith name
      (Directory.defaultExistsOpts { expectedType = Just ExistsTypeDirectoryType }) args.source
    pure $ if exists then container
      # Container.withMountedCache "/cache" (Query.cacheVolume name client)
      # Container.withExec [ "cp", "-a", "/snapshot/" <> name <> "/.", "/cache/" ]
      else container) base names
  void $ Container.sync restored
  pure "Dagger cache volumes restored"
