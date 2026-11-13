module CI.GuestHost where
import Prelude (Unit)
import Effect (Effect)
foreign import env :: String -> Effect String
foreign import platform :: Effect String
foreign import arch :: Effect String
foreign import run :: Array String -> Effect Unit
foreign import digest :: String -> Effect String
foreign import writeText :: String -> String -> Effect Unit
foreign import setEnv :: String -> String -> Effect Unit
foreign import chdir :: String -> Effect Unit
