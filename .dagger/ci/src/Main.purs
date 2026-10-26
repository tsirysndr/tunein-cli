module Main where

import Prelude
import CI.Runtime as Runtime
import CI.Cache as Cache
import CI.Release as Release
import Dagger.Modules as Module
import Effect (Effect)
import CI.Build as Build
import CI.Nix as Nix

main :: Effect Unit
main = Runtime.runModule
  [ Module.object_ "ProjectCi" {}
      [ Module.function "publishRelease" "Create or update a public GitHub release and upload its assets." Release.releaseArgs Release.publishRelease
      , Module.function "uploadRelease" "Attach assets to an existing release." Release.releaseArgs Release.uploadRelease
      , Module.function "linuxRelease" "Build and package a Linux release." Build.buildArgs Build.linuxRelease
      , Module.function "nixBuild" "Build the Linux Nix flake and upload to Cachix when configured." Nix.nixArgs Nix.flake
      , Module.function "publishGemfury" "Publish Debian and RPM packages to Gemfury." Release.furyArgs Release.publishGemfury
      , Module.function "test" "Run web UI and Rust tests." Build.sourceArgs Build.test
      , Module.function "format" "Check Rust formatting." Build.sourceArgs Build.format
      , Module.function "saveCaches" "Export named volumes for GitHub Actions cache." Cache.cacheArgs Cache.saveCaches
      , Module.function "restoreCaches" "Restore named volumes before a build." Cache.restoreArgs Cache.restoreCaches
      ]
  ]
