module Test.Toolchain (run) where

import Prelude
import CI.Toolchain (ensureExecutable)
import Data.Either (isRight)
import Data.Foldable (traverse_)
import Effect (Effect)
import Effect.Exception (throw, try)
import Effect.Ref as Ref

run :: Effect Unit
run = traverse_ scenario
  [ { first: "v24.22.2\n", next: "unused", installs: false, succeeds: true }
  , { first: "v22.23.4", next: "v24.22.2\n", installs: true, succeeds: true }
  , { first: "", next: "v24.22.2", installs: true, succeeds: true }
  , { first: "v22.23.4", next: "v22.23.4", installs: true, succeeds: false }
  , { first: "", next: "", installs: true, succeeds: false }
  , { first: "unrelated output v24.22.2", next: "v24.22.2", installs: true, succeeds: true }
  ]
  where
  scenario input = do
    captures <- Ref.new 0
    installs <- Ref.new 0
    let capture = do
          index <- Ref.read captures
          Ref.modify_ (_ + 1) captures
          let value = if index == 0 then input.first else input.next
          if value == "" then throw "Executable unavailable" else pure value
        install = Ref.modify_ (_ + 1) installs
    result <- try (ensureExecutable "node" "v24.22.2" capture install)
    unless (isRight result == input.succeeds) (throw "Toolchain failure propagation changed")
    count <- Ref.read installs
    unless (count == if input.installs then 1 else 0) (throw "Matching tools must not be reinstalled")
    checked <- Ref.read captures
    unless (checked == if input.installs then 2 else 1) (throw "Fallback installation must be verified")
