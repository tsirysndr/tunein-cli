module CI.Toolchain (ensureExecutable) where

import Prelude
import Data.Either (Either(..))
import Data.String.Common (trim)
import Effect (Effect)
import Effect.Exception (throw, try)

-- Capture is supplied by the host adapter; tests need no processes or installers.
ensureExecutable :: String -> String -> Effect String -> Effect Unit -> Effect Unit
ensureExecutable program expected capture install = do
  existing <- try capture
  case existing of
    Right version | trim version == expected -> pure unit
    _ -> do
      install
      actual <- trim <$> capture
      unless (actual == expected) (throw (program <> " setup did not publish the requested version on PATH"))
