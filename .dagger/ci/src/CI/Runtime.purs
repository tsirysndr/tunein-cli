module CI.Runtime (runModule) where

import Prelude
import Dagger as Dagger
import Dagger.Modules (ObjectDefinition)
import Dagger.Module.Runtime as Runtime
import Effect (Effect)
import Effect.Aff (launchAff_)

foreign import sessionCredentials :: Effect { port :: Int, token :: String }

-- Cold image pulls and Rust compilation can exceed the SDK's five-minute default.
-- Each engine request may wait up to 25 minutes for a cold build.
runModule :: Array ObjectDefinition -> Effect Unit
runModule definitions = do
  credentials <- sessionCredentials
  client <- Dagger.connectWith
    { port: credentials.port, token: credentials.token, timeoutMs: 25 * 60 * 1000 }
  launchAff_ (Runtime.serve client definitions)
