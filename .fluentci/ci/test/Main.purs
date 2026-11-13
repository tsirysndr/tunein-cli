module Test.Main (main) where
import Prelude
import CI.Validation as V
import Test.Toolchain as Toolchain
import Data.Either (Either(..))
import Effect (Effect)
import Effect.Exception (throw)

assert :: String -> Boolean -> Effect Unit
assert label condition = unless condition (throw label)

main :: Effect Unit
main = do
  assert "semantic release tag" (V.validVersion "v0.7.1")
  assert "prerelease tag" (V.validVersion "0.8.0-rc.1")
  assert "reject traversal" (not (V.validVersion "../../escape"))
  assert "reject shell injection" (not (V.validVersion "1.0;touch injected"))
  assert "reject empty tag" (not (V.validVersion ""))
  case V.targetInfo "aarch64-unknown-linux-gnu" of
    Right info -> assert "ARM64 package mapping" (info.deb == "arm64" && info.rpm == "" && info.triple == "aarch64-linux-gnu")
    Left _ -> throw "ARM64 target rejected"
  case V.targetInfo "armv7-unknown-linux-gnueabihf" of
    Right info -> assert "ARMv7 archive-only behavior" (info.deb == "" && info.rpm == "")
    Left _ -> throw "ARMv7 target rejected"
  case V.targetInfo "--bad" of
    Left _ -> pure unit
    Right _ -> throw "Invalid target accepted"
  assert "quoted CSV preserves commas, quotes and shell text" (V.csv [ "one,two", "a\"b", "$(false)" ] == "\"one,two\",\"a\"\"b\",\"$(false)\"")
  let metadata = V.replaceVersion "0.7.1" "Name: tunein-cli\nVersion: 0.6.0\nRequires: alsa-lib\n"
  assert "update only the version header" (metadata == "Name: tunein-cli\nVersion: 0.7.1\nRequires: alsa-lib\n")
  Toolchain.run
