module Main where

import Prelude
import Dagger.Container as Container
import Dagger.Modules as Module
import Dagger.Query as Query
import Effect (Effect)

main :: Effect Unit
main = Module.runModule
  [ Module.object_ "__MODULE_NAME__" {}
      [ Module.function "hello" "Greet someone from PureScript."
          (Module.defaultArgument "name" "world")
          (\_ name -> pure ("Hello, " <> name <> "!"))
      , Module.function "container" "Return a Dagger container."
          (Module.defaultArgument "message" "Hello from PureScript!")
          (\client message -> pure $ Query.container client
            # Container.from "alpine:3.21"
            # Container.withExec [ "echo", message ])
      ]
  ]
