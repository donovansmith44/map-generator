module Steps where

import Data.Aeson (Value (..), eitherDecodeStrict)
import qualified Data.Aeson.Key as K
import qualified Data.Aeson.KeyMap as KM
import qualified Data.ByteString as BS
import qualified Data.Map.Strict as Map
import Data.Set (member)
import Data.Text (Text)
import qualified Data.Text as T
import qualified Data.Vector as V
import System.Directory (doesFileExist)
import System.FilePath ((</>))
import Capture
import Gherkin.Ast (Keyword (..))
import Pattern
import World

sceneUrl :: Text -> PieceSet -> Year -> StyleName -> Text
sceneUrl base (PieceSet ps) (Year y) (StyleName st) =
  base <> "/api/scene?year=" <> tshow y <> "&zoom=90.0000&style=" <> st
       <> flag Labels "labels" <> flag Water "topo" <> flag Journeys "journeys"
       <> (if Ground `member` ps then "&relief=1" else "")
  where
    flag p name = if p `member` ps then "" else "&" <> name <> "=0"
    tshow = T.pack . show

getUrl :: Text -> World -> IO (Either Text World)
getUrl url w = do
  r <- transport w url
  pure $ (\pair -> w { bound = Map.insert "_last" pair (bound w) }) <$> r

bindLast :: Text -> World -> Either Text World
bindLast name w = case Map.lookup "_last" (bound w) of
  Just pair -> Right w { bound = Map.insert name pair (bound w) }
  Nothing   -> Left "no response to bind"

field :: Text -> Value -> Either Text Value
field k (Object o) = maybe (Left ("no field " <> k)) Right (KM.lookup (K.fromText k) o)
field k _          = Left ("not an object, wanted field " <> k)

loadFixture :: World -> Text -> IO (Either Text Value)
loadFixture w name = do
  let path = fixtureDir w </> T.unpack name <> ".json"
  ok <- doesFileExist path
  if not ok
    then pure (Left ("missing fixture " <> name <> " — run with --bless to create it"))
    else do
      raw <- BS.readFile path
      pure (either (Left . T.pack) Right (eitherDecodeStrict raw))

blessOrCompare :: Text -> World -> IO (Either Text World)
blessOrCompare fname w = case Map.lookup "_last" (bound w) of
  Nothing -> pure (Left "no response to compare")
  Just (raw, v) -> do
    let path = fixtureDir w </> T.unpack fname <> ".json"
    if blessMode w
      then BS.writeFile path raw >> pure (Right w)
      else do
        fx <- loadFixture w fname
        pure $ case fx of
          Left e -> Left e
          Right expected
            | expected == v -> Right w
            | otherwise -> Left ("response differs from fixture " <> fname)

allSteps :: [StepDef]
allSteps =
  [ -- generic wire steps
    mkStep When (lit "I GET " *> capRest @FixtureRefFreeText) $
      \(FixtureRefFreeText path) w -> getUrl (baseUrl w <> path) w
  , mkStep Then (lit "the response equals fixture " *> capRest @FixtureRef) $
      \(FixtureRef f) w -> blessOrCompare f w
  , mkStep Then (lit "the response field " *> ((,) <$> capUntil @FixtureRefFreeText " equals "
                                                   <*> capRest @FixtureRefFreeText)) $
      \(FixtureRefFreeText k, FixtureRefFreeText expct) w ->
        pure $ case Map.lookup "_last" (bound w) of
          Nothing -> Left "no response"
          Just (_, v) -> case field k v of
            Right (String s) | s == expct -> Right w
            Right other -> Left ("field " <> k <> " = " <> T.pack (show other)
                                 <> ", wanted " <> expct)
            Left e -> Left e
  , mkStep Then (lit "the response is a JSON array") $ \() w ->
      pure $ case Map.lookup "_last" (bound w) of
        Just (_, Array _) -> Right w
        Just _  -> Left "response is not an array"
        Nothing -> Left "no response"
    -- scene steps (piece vocabulary on the wire)
  , mkStep When (lit "I render pieces "
                 *> ((,,,) <$> capUntil @PieceSet " at year "
                           <*> capUntil @Year " in style "
                           <*> capUntil @StyleName " as "
                           <*> capRest @BindName)) $
      \(ps, y, st, BindName n) w -> do
        r <- getUrl (sceneUrl (baseUrl w) ps y st) w
        pure (r >>= bindLast n)
  , mkStep When (lit "I render pieces "
                 *> ((,,) <$> capUntil @PieceSet " at year "
                          <*> capUntil @Year " in style "
                          <*> capRest @StyleName)) $
      \(ps, y, st) w -> getUrl (sceneUrl (baseUrl w) ps y st) w
  , mkStep Then (lit "" *> ((,) <$> capUntil @BindName " equals " <*> capRest @BindName)) $
      \(BindName a, BindName b) w ->
        pure $ case (Map.lookup a (bound w), Map.lookup b (bound w)) of
          (Just (_, va), Just (_, vb))
            | va == vb  -> Right w
            | otherwise -> Left (a <> " and " <> b <> " differ")
          _ -> Left "unbound scene name"
  , mkStep Then (lit "" *> ((,) <$> capUntil @BindName "'s resources are a subset of "
                                <*> capRest @BindName)) $
      \(BindName a, BindName b) w ->
        pure $ case (resourceIds =<< scene a w, resourceIds =<< scene b w) of
          (Right ia, Right ib)
            | all (`elem` ib) ia -> Right w
            | otherwise -> Left (a <> " has resources absent from " <> b)
          (Left e, _) -> Left e
          (_, Left e) -> Left e
  , mkStep Then (lit "" *> (capUntil @BindName "'s labels are empty" <* pure ())) $
      \(BindName a) w ->
        pure $ case field "labels" =<< scene' a w of
          Right (Array v) | V.null v -> Right w
          Right _ -> Left (a <> " has labels")
          Left e -> Left e
  ]
  where
    scene n w = maybe (Left ("unbound " <> n)) (Right . snd) (Map.lookup n (bound w))
    scene' = scene
    resourceIds v = case field "resources" v of
      Right (Array rs) -> traverse (field "id") (V.toList rs)
      other -> Left ("no resources array: " <> T.pack (show (() <$ other)))

-- free-text capture (paths, field names, expected strings): Described universe
newtype FixtureRefFreeText = FixtureRefFreeText Text deriving (Eq, Show)
instance FromCapture FixtureRefFreeText where
  capName _ = "text"
  universe _ = Described "free text (a path, field name, or expected value)"
  renderCap (FixtureRefFreeText t) = t
  parseCap = Right . FixtureRefFreeText . T.strip

newtype BindName = BindName Text deriving (Eq, Show)
instance FromCapture BindName where
  capName _ = "name"
  universe _ = Described "a scene name to bind, e.g. sceneA"
  renderCap (BindName t) = t
  parseCap t = let s = T.strip t in
    if not (T.null s) && T.all (\c -> c `elem` ("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789" :: String)) s
      then Right (BindName s) else Left "a bind name is a single word"
