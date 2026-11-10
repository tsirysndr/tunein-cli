module CI.Assets (releaseFiles, packageFiles, isReleaseAsset) where

import Effect (Effect)
import Prelude
import Native.Directory as Directory
import Native.Types (Directory, ExistsType(..))
import Data.Array (filter, filterA)
import Data.Foldable (any)
import Data.Maybe (Maybe(..), isJust)
import Data.String (Pattern(..), stripSuffix)

isReleaseAsset :: String -> Boolean
isReleaseAsset name = any (\suffix -> endsWith (Pattern suffix) name)
  [ ".tar.gz", ".zip", ".deb", ".rpm", ".sha256" ]

releaseFiles :: Directory -> Effect (Array String)
releaseFiles directory = do
  entries <- Directory.entries directory
  filterA (\name -> Directory.existsWith name
    (Directory.defaultExistsOpts { expectedType = Just ExistsTypeRegularType }) directory)
    (filter isReleaseAsset entries)

packageFiles :: Directory -> Effect (Array String)
packageFiles directory = filter (\name -> endsWith (Pattern ".deb") name || endsWith (Pattern ".rpm") name)
  <$> releaseFiles directory

endsWith :: Pattern -> String -> Boolean
endsWith suffix value = isJust (stripSuffix suffix value)
