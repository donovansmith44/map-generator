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

-- Ordering rule (R5, controller ruling): SPECIFIC step definitions must be
-- listed before GENERIC ones that could otherwise shadow them, wherever two
-- definitions can both classify the same body as "this is my step, run me"
-- (not merely "not this step") for the same input text. R4 makes a
-- terminator-not-found a fall-through, but it does NOT protect against a
-- capture failing to PARSE at all — that's a reportable error under R4/R5,
-- not a fall-through, and it makes two *different* StepDefs able to each
-- claim the same body when they share a literal substring. Concrete case a
-- reviewer found live in this list: "the response field style equals
-- canaan" is a legal body for BOTH the "the response field {text} equals
-- {text}" step above AND the `lit ""`-prefixed "{name} equals {name}" step
-- below (its `capUntil @BindName " equals "` would try to parse "the
-- response field style" as a BindName, fail on the spaces, and — because
-- that's a capture-parse failure rather than a missing literal — report an
-- error rather than falling through). Listing the "the response field ..."
-- step first means firstMatch (or its non-test callers) sees it before the
-- ambiguous generic step. This list-order workaround is NOT a structural
-- fix — it is not verified anywhere that a later append (Tasks 10-12) can't
-- reintroduce the same shadowing by inserting a step in the wrong spot.
-- Task 7's totality check is where the actual guarantee lives: it detects a
-- step body matching two or more definitions and fails, naming both. Do not
-- try to solve the ambiguity here.
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
    -- The plan explicitly sanctions this definition being unused by any
    -- current feature file (like the subset step above) — do not treat its
    -- absence from allSteps' test coverage or from any .feature as a sign
    -- it should be deleted; that's a deliberate, blessed exception, unlike
    -- the now-removed "the response is a JSON array" step.
  , mkStep Then (lit "" *> capUntil @BindName "'s labels are empty") $
      \(BindName a) w ->
        pure $ case field "labels" =<< scene a w of
          Right (Array v) | V.null v -> Right w
          Right _ -> Left (a <> " has labels")
          Left e -> Left e
  ]
  where
    scene n w = maybe (Left ("unbound " <> n)) (Right . snd) (Map.lookup n (bound w))
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
