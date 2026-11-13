module CI.Validation (validVersion, targetInfo, csv, replaceVersion) where

import Prelude
import Data.Array as A
import Data.Either (Either(..))
import Data.Foldable (all)
import Data.Maybe (Maybe(..), fromMaybe)
import Data.String as S
import Data.String.CodeUnits as CU

type Target = { deb :: String, rpm :: String, triple :: String, arch :: String }

targetInfo :: String -> Either String Target
targetInfo = case _ of
  "x86_64-unknown-linux-gnu" -> Right { deb: "amd64", rpm: "x86_64", triple: "", arch: "" }
  "aarch64-unknown-linux-gnu" -> Right { deb: "arm64", rpm: "", triple: "aarch64-linux-gnu", arch: "arm64" }
  "armv7-unknown-linux-gnueabihf" -> Right { deb: "", rpm: "", triple: "arm-linux-gnueabihf", arch: "armhf" }
  _ -> Left "Unsupported Linux release target"

validVersion :: String -> Boolean
validVersion value =
  let bare = fromMaybe value (S.stripPrefix (S.Pattern "v") value)
  in case A.head (CU.toCharArray bare) of
    Nothing -> false
    Just first -> first >= '0' && first <= '9' && all (\c ->
      (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || (c >= '0' && c <= '9') || A.elem c [ '.', '+', '~', '-' ]) (CU.toCharArray bare)

-- Internal arrays are encoded as public comma-separated module arguments.
csv :: Array String -> String
csv = S.joinWith "," <<< map (\value -> "\"" <> S.replaceAll (S.Pattern "\"") (S.Replacement "\"\"") value <> "\"")

replaceVersion :: String -> String -> String
replaceVersion version = S.joinWith "\n" <<< map (\line ->
  if S.startsWith (S.Pattern "Version:") line then "Version: " <> version else line) <<< S.split (S.Pattern "\n")
