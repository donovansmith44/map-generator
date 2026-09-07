module Steps where

import Data.Aeson (Value (..), eitherDecodeStrict)
import qualified Data.Aeson.Key as K
import qualified Data.Aeson.KeyMap as KM
import qualified Data.ByteString as BS
import Data.Char (isSpace)
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

-- Ordering rule (R5, controller ruling) — UPDATED under R23, see below for
-- what changed and what didn't. Originally: SPECIFIC step definitions must
-- be listed before GENERIC ones that could otherwise shadow them, wherever
-- two definitions can both classify the same body as "this is my step, run
-- me" for the same input text. Two concrete cases were found live in this
-- list: "the response field style equals canaan" is a legal body for BOTH
-- the "the response field {text} equals {text}" step above AND the
-- `lit ""`-prefixed "{name} equals {name}" step below (its
-- `capUntil @BindName " equals "` tries to parse "the response field
-- style" as a BindName and fails on the spaces); and separately, ANY
-- "I render pieces ... in style X as sceneName" line is a legal body for
-- BOTH the binding overload above AND the non-binding overload below it
-- (whose `capRest @StyleName` swallows "X as sceneName" whole and fails to
-- parse it as a style).
--
-- R23 (controller ruling) dissolved both of these STRUCTURALLY, not by
-- ordering: World.Claim now has three states (NoMatch / ClaimError /
-- Matched) instead of a single conflated Maybe, and a definition whose
-- capture fails to parse (ClaimError) is no longer treated the same as one
-- that actually matches (Matched). Both cases above have exactly one
-- Matched definition and one ClaimError also-ran; Run.runScenario always
-- runs a Matched action over a mere ClaimError, and Check.hs's totality
-- law counts MATCHES, not claims, so neither case is ambiguous any more —
-- regardless of list order. List order is therefore NOT load-bearing for
-- correctness any more; a later append (Tasks 10-12) inserted in the
-- "wrong" spot cannot silently reintroduce this kind of shadowing, because
-- there is no shadowing left to reintroduce — a true structural match
-- always wins over a mere claim, by construction.
--
-- That "not load-bearing" claim used to rest on nothing but the fact that
-- today's allSteps happens not to contain a genuine double-match — an
-- aspiration, not a guarantee: if a later append DID introduce two
-- definitions that both truly MATCH the same body, Run.runScenario's old
-- `case ... of (f:_) -> ...` silently ran the head of the list and never
-- told anyone, which is exactly the kind of order-dependent shadowing this
-- whole comment claims doesn't happen any more. Fix 2 (post-Task-7 review)
-- closes that gap in CODE: runScenario now refuses to run anything when
-- two or more definitions truly match, failing loudly and naming every
-- competing sketch, regardless of which one is first in this list. So the
-- claim above is no longer aspirational — a later append that collides can
-- reorder itself however it likes and will still be caught, at both
-- `check` time (Check.hs's VAmbiguous) and `run` time (this refusal).
--
-- Specific-before-generic remains good practice for ERROR QUALITY, though:
-- when NO definition matches and only claims remain, runScenario and
-- Check report the FIRST ClaimError in list order, so listing the more
-- specific (usually more informative) definition first still shapes which
-- message a human sees on a genuinely bad value. That's why this list
-- keeps the order it has. Task 7's totality check (Check.hs) is where the
-- law actually lives: a step matched by 0 definitions is an orphan, by 1
-- is fine, by 2+ is ambiguous and fatal, and a step claimed-but-never-
-- matched (a bad value) is its own fatal class, distinct from both.
allSteps :: [StepDef]
allSteps =
  [ -- generic wire steps
    mkStep When (lit "I GET " *> capRest @UrlPath) $
      \(UrlPath path) w -> getUrl (baseUrl w <> path) w
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
                                <*> capUntil @BindName "'s resources")) $
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

-- free-text capture (field names, expected strings): Described universe.
-- NOT used for URLs any more (see UrlPath below) — this capture can never
-- fail to parse, so it has no discriminating power, which is exactly
-- right for a field name or an expected value (spaces are legitimate
-- there) and exactly wrong for a URL (fix 7: it silently swallowed
-- " as first" as part of a GET path, making an undefined "as"-binding
-- step invisible to the totality check).
newtype FixtureRefFreeText = FixtureRefFreeText Text deriving (Eq, Show)
instance FromCapture FixtureRefFreeText where
  capName _ = "text"
  universe _ = Described "free text (a field name or expected value)"
  renderCap (FixtureRefFreeText t) = t
  parseCap = Right . FixtureRefFreeText . T.strip

-- R23 fix 7 (controller ruling): the plain GET step used to capture its
-- URL with FixtureRefFreeText, whose parseCap can NEVER fail — so a line
-- like "I GET /api/subjects?year=<someYear> as first" (meant for a not-
-- yet-written "I GET {url} as {name}" binding step) silently matched the
-- plain GET step instead, with " as first" swallowed into the "url". A
-- capture that cannot fail has no discriminating power, so the totality
-- law had nothing to catch: a check satisfiable by the failure mode is no
-- check at all (see MEMORY: verify-distinct-not-nonnull). Fixed by TYPE:
-- a URL path is, as a genuine property of URLs, a token with no raw
-- whitespace in it. Rejecting whitespace here is not a special case for
-- " as " — any embedded space (a stray "as SOMETHING", a typo, a copy-
-- paste artifact) now fails to parse, making the line a bad-value rather
-- than a clean, silent match. When the real "I GET {url} as {name}" step
-- is added in a later phase, this is what lets it become the line's
-- unique match instead of creating a fresh ambiguity with this one.
newtype UrlPath = UrlPath Text deriving (Eq, Show)
instance FromCapture UrlPath where
  capName _ = "url"
  universe _ = Described "a URL path with no embedded whitespace, e.g. /api/subjects?year=-1405"
  renderCap (UrlPath t) = t
  parseCap t =
    let s = T.strip t
    in if T.any isSpace s
         then Left ("'" <> s <> "' is not a URL path: a URL path cannot contain whitespace")
         else Right (UrlPath s)

newtype BindName = BindName Text deriving (Eq, Show)
instance FromCapture BindName where
  capName _ = "name"
  universe _ = Described "a scene name to bind, e.g. sceneA"
  renderCap (BindName t) = t
  parseCap t = let s = T.strip t in
    if not (T.null s) && T.all (\c -> c `elem` ("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789" :: String)) s
      then Right (BindName s) else Left "a bind name is a single word"
