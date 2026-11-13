module CI.Support where

import Prelude
import Data.Array as A
import Data.Either (either)
import Data.Maybe (Maybe(..), fromMaybe)
import Data.String as S
import Effect (Effect)
import Effect.Exception (throw)
import FluentCI.Host.Files as F
import FluentCI.Host.Plan as H
import FluentCI.Host.Client (RunOutcome(..))
import FluentCI.Plugin as P

unwrap :: forall a. P.Result a -> Effect a
unwrap = either (throw <<< P.errorMessage) pure

captureIn :: String -> Array String -> Effect String
captureIn directory argv = P.stdout (P.pipeline "tunein-ci" # P.withWorkdir directory # P.exec argv) >>= unwrap

runIn :: String -> Array String -> Effect Unit
runIn directory argv = void (captureIn directory argv)

run :: Array String -> Effect Unit
run = runIn "."

env :: String -> String -> Effect String
env key fallback = do
  found <- P.hasEnv key >>= unwrap
  if found then P.getEnv key >>= unwrap else pure fallback

requiredEnv :: String -> Effect String
requiredEnv key = do
  value <- env key ""
  when (S.trim value == "") (throw ("Missing environment variable: " <> key))
  pure value

absolute :: String -> Effect String
absolute directory = S.trim <$> captureIn directory [ "pwd" ]

fresh :: Effect String
fresh = S.trim <$> captureIn "." [ "mktemp", "-d" ]

readText :: String -> Effect String
readText path = captureIn "." [ "cat", path ]

basename :: String -> String
basename path = fromMaybe path (A.last (S.split (S.Pattern "/") path))

writeText :: String -> String -> Effect Unit
writeText path text = do
  directory <- S.trim <$> captureIn "." [ "dirname", path ]
  run [ "mkdir", "-p", directory ]
  root <- fresh
  report <- P.runPlan (root <> "/run") (H.plan [ F.writeText "write" (basename path) text true directory ] # H.withInheritedEnv true) >>= unwrap
  unless (report.outcome == Just RunSucceeded) (throw ("Could not write " <> path))

files :: String -> Effect (Array String)
files directory = do
  result <- captureIn "." [ "find", directory, "-type", "f" ]
  pure (A.filter (_ /= "") (S.split (S.Pattern "\n") (S.trim result)))

checksum :: String -> Effect Unit
checksum path = do
  digest <- S.trim <$> (P.sha256 (P.file path) >>= unwrap)
  writeText (path <> ".sha256") (digest <> "  " <> basename path <> "\n")

releaseAsset :: String -> Boolean
releaseAsset path = A.any (\suffix -> S.endsWith (S.Pattern suffix) path)
  [ ".tar.gz", ".deb", ".rpm", ".sha256" ]

-- Persist typed setup results across GitHub steps without parsing stdout in a shell.
persistEnvironment :: Effect Unit
persistEnvironment = do
  destination <- env "GITHUB_ENV" ""
  when (destination /= "") do
    path <- requiredEnv "PATH"
    when (S.contains (S.Pattern "\n") path || S.contains (S.Pattern "\r") path) (throw "Invalid PATH")
    previous <- readText destination
    writeText destination (previous <> "PATH=" <> path <> "\n")
