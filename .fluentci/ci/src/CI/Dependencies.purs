module CI.Dependencies where

import Effect (Effect)

type Variable = { name :: String, value :: String }
type SetupResult = { tool :: String, version :: String, binDir :: String, environment :: Array Variable }
type ExecutionResult = { outcome :: String, runDir :: String, reportPath :: String, stdoutPath :: String, stderrPath :: String }
foreign import setupNode :: { version :: String, installDir :: String } -> Effect SetupResult
foreign import setupBun :: { version :: String, installDir :: String } -> Effect SetupResult
foreign import setupRust :: { version :: String, components :: String, targets :: String } -> Effect SetupResult
foreign import setupGo :: { version :: String } -> Effect SetupResult
foreign import setupGh :: Effect SetupResult
foreign import setupNix :: { sudo :: Boolean, trustedUsers :: Array String, extraConf :: Array String } -> Effect SetupResult
foreign import setupCachix :: { cache :: String } -> Effect SetupResult
foreign import setupFlakehub :: Effect SetupResult
foreign import dpkg :: { directory :: String, arch :: String, output :: String } -> Effect (Array String)
foreign import rpmbuild :: { directory :: String, arch :: String, output :: String, spec :: String } -> Effect (Array String)
foreign import uploadRelease :: { project :: String, runDir :: String, tag :: String, files :: String, clobber :: Boolean } -> Effect ExecutionResult
foreign import uploadGemfury :: { files :: String } -> Effect { uploaded :: Boolean, files :: Array String }
foreign import flakehub :: { project :: String, runDir :: String, name :: String, version :: String, visibility :: String } -> Effect ExecutionResult
foreign import flakestry :: { project :: String, runDir :: String, version :: String, ref :: String } -> Effect ExecutionResult
