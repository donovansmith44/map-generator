module Steps where

import Data.Aeson (Value (..), eitherDecodeStrict)
import qualified Data.Aeson.Key as K
import qualified Data.Aeson.KeyMap as KM
import Data.Aeson.Encode.Pretty (encodePretty)
import qualified Data.ByteString as BS
import qualified Data.ByteString.Lazy as BL
import Data.Char (isSpace)
import Data.Foldable (asum)
import qualified Data.Map.Strict as Map
import Data.Set (Set, member)
import qualified Data.Set as Set
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
    -- Task 10: the same GET, but binding the response under a name
    -- instead of just "_last" (so a later step can compare two GETs
    -- against each other, e.g. the @property "first equals second"
    -- determinism scenarios). Specific-before-generic: listed right
    -- after the plain GET step it overlaps with on "I GET " — not load-
    -- bearing under R23 (a genuine structural match always wins), but
    -- good practice for error-message quality.
  , mkStep When (lit "I GET " *> ((,) <$> capUntil @UrlPath " as "
                                      <*> capRest @BindName)) $
      \(UrlPath path, BindName n) w -> do
        r <- getUrl (baseUrl w <> path) w
        pure (r >>= bindLast n)
  , mkStep Then (lit "the response equals fixture " *> capRest @FixtureRef) $
      \(FixtureRef f) w -> blessOrCompare f w
    -- Task 10: whole-body fixture equality with a DECLARED, in-scenario
    -- mask (the owner's whole-body law forbids an existential poke —
    -- "some field equals X" — so a don't-care field must be masked
    -- VISIBLY in the scenario text, shape-checked, and the WHOLE
    -- remainder still pinned against the fixture; see MaskShape/
    -- checkShape/setField below). Listed right after the unmasked
    -- fixture-equality step it overlaps with on "the response equals
    -- fixture ".
  , mkStep Then (lit "the response equals fixture " *> ((,,) <$> capUntil @FixtureRef " masking "
                                                             <*> capUntil @FixtureRefFreeText " as "
                                                             <*> capRest @MaskShape)) $
      \(FixtureRef f, FixtureRefFreeText masked, shape) w -> do
        fx <- loadFixture w f
        case Map.lookup "_last" (bound w) of
          Nothing -> pure (Left "no response")
          -- the raw bytes are deliberately unused here: bless writes the
          -- re-serialized, masked VALUE below, never the original raw
          -- bytes (which still contain the real secret this masks).
          Just (_raw, actual) -> case field masked actual of
            Left e -> pure (Left e)
            Right mv -> case checkShape shape masked mv of
              Left e -> pure (Left e)
              Right () -> do
                let actual' = setField masked (String "MASKED") actual
                    path = fixtureDir w </> T.unpack f <> ".json"
                if blessMode w
                  then do
                    -- Bless writes the ACTUAL body with the masked field
                    -- replaced by the literal "MASKED" -- never the raw
                    -- secret value, and never the unmasked body either.
                    BS.writeFile path (BL.toStrict (encodePretty actual'))
                    pure (Right w)
                  else pure $ do
                    expected <- fx
                    if actual' == expected then Right w
                    else Left ("body differs from fixture " <> f <> " outside the mask")
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
      \(ps, y@(Year yInt), st, BindName n) w -> do
        r <- getUrl (sceneUrl (baseUrl w) ps y st) w
        pure $ do
          w' <- r >>= bindLast n
          -- Task 11: the combine step ("combining A and B equals
          -- rendering <someA> plus <someB>") must render its own union
          -- scene at the SAME year the two parts were rendered at, not
          -- a hardcoded one (the plan's own self-review flagged exactly
          -- this bug). Every binding render records the year it used
          -- here; a @property scenario substitutes ONE <someYear> value
          -- across the whole scenario body, so sceneA and sceneB's
          -- renders (and therefore this binding) always agree.
          Right w' { bound = Map.insert "_year" (BS.empty, Number (fromIntegral yInt)) (bound w') }
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
    -- current feature file — do not treat its absence from allSteps' test
    -- coverage or from any .feature as a sign it should be deleted; that's
    -- a deliberate, blessed exception, unlike the now-removed "the response
    -- is a JSON array" step.
    -- (Post-Task-7 review, fix 2 of the second round: this comment used to
    -- say "like the subset step above" as a second example of a blessed-
    -- unused step. That's gone stale from this round's own fix 8 — the
    -- subset step's wording now matches the corpus exactly
    -- ("noWater's resources are a subset of full's resources"), so it is
    -- no longer unused. This labels-are-empty step remains the one
    -- deliberate exception.)
  , mkStep Then (lit "" *> capUntil @BindName "'s labels are empty") $
      \(BindName a) w ->
        pure $ case field "labels" =<< scene a w of
          Right (Array v) | V.null v -> Right w
          Right _ -> Left (a <> " has labels")
          Left e -> Left e
    -- Task 11 (@target): today's manifest carries no per-entry piece
    -- attribution at all -- this is the diagnosis, not a bug in the
    -- step. It MATCHES (so `check` never calls it an orphan) and fails
    -- honestly, naming the v0.1 wart, whenever any entry lacks the field.
  , mkStep Then (lit "every feature entry carries a piece field") $ \() w ->
      pure $ case Map.lookup "_last" (bound w) of
        Nothing -> Left "no response"
        Just (_, v) -> case field "features" v of
          Right (Array fs)
            | V.all (\f -> either (const False) (const True) (field "piece" f)) fs -> Right w
            | otherwise -> Left "manifest entries carry no piece attribution (v0.1 wart)"
          other -> Left (T.pack (show (() <$ other)))
    -- Task 11 (@target @property): v0.1 has no server-side combine
    -- endpoint, so the law is checked indirectly -- render the union of
    -- the two piece sets and assert its resource-id set equals the
    -- UNION of the two parts' resource-id sets, at the SAME year the
    -- parts were rendered at (see the binding render step's "_year"
    -- comment above).
  , mkStep Then (lit "combining " *> ((,,,) <$> capUntil @BindName " and "
                                            <*> capUntil @BindName " equals rendering "
                                            <*> capUntil @PieceSet " plus "
                                            <*> capRest @PieceSet)) $
      \(BindName a, BindName b, PieceSet sa, PieceSet sb) w ->
        case Map.lookup "_year" (bound w) of
          Nothing -> pure (Left "no year recorded for this scene (render a piece set first)")
          Just (_, Number sci) -> do
            let y = Year (round sci)
            r <- getUrl (sceneUrl (baseUrl w) (PieceSet (Set.union sa sb)) y (StyleName "canaan")) w
            pure $ do
              w' <- r
              both <- resourceSet "_last" w'
              ra <- resourceSet a w'
              rb <- resourceSet b w'
              if both == Set.union ra rb then Right w'
              else Left "union scene is not the union of its parts' resources"
          Just _ -> pure (Left "bound _year is not a number")
    -- Task 11 (@target): geometry is content-addressed (an id IS its
    -- bytes -- see resources.feature's own title); dress rides styles,
    -- not payload ids, so restyling one piece must leave every id set
    -- EQUAL, not merely overlapping.
  , mkStep Then (lit "" *> ((,) <$> capUntil @BindName " and "
                                <*> (capUntil @BindName " differ only in dress, never in geometry"))) $
      \(BindName a, BindName b) w ->
        pure $ do
          ra <- resourceSet a w
          rb <- resourceSet b w
          if ra == rb then Right w
          else Left "restyle changed geometry ids: dress is not local"
    -- Task 11: /api/resource returns a BINARY body -- transportRaw, not
    -- the JSON-decoding transport, is used here (see World.hs).
  , mkStep Then (lit "fetching " *> (capUntil @BindName "'s first resource twice yields identical bytes")) $
      \(BindName a) w ->
        case firstResourceId a w of
          Left e -> pure (Left e)
          Right rid -> do
            r1 <- transportRaw w (baseUrl w <> "/api/resource?id=" <> rid)
            r2 <- transportRaw w (baseUrl w <> "/api/resource?id=" <> rid)
            pure $ case (r1, r2) of
              (Right b1, Right b2) | b1 == b2 -> Right w
              (Right _, Right _) -> Left ("id " <> rid <> " served two different payloads")
              (Left e, _) -> Left e
              (_, Left e) -> Left e
    -- Task 11 (@target): a batch is exactly its singles, byte for byte.
  , mkStep Then (lit "fetching " *> (capUntil @BindName "'s first two resources as a batch equals fetching them singly")) $
      \(BindName a) w ->
        case twoResourceIds a w of
          Left e -> pure (Left e)
          Right (i1, i2) -> do
            batch <- transportRaw w (baseUrl w <> "/api/resources?ids=" <> i1 <> "," <> i2)
            s1 <- transportRaw w (baseUrl w <> "/api/resource?id=" <> i1)
            s2 <- transportRaw w (baseUrl w <> "/api/resource?id=" <> i2)
            pure $ case (batch, s1, s2) of
              (Right bb, Right b1, Right b2)
                | bb == b1 <> b2 -> Right w
                | otherwise -> Left "batch bytes differ from concatenated singles"
              (Left e, _, _) -> Left e
              (_, Left e, _) -> Left e
              (_, _, Left e) -> Left e
    -- Task 12: the CDC step -- see `project`/`projections` above.
  , mkStep Then (lit "the consumed projection " *> ((,) <$> capUntil @ProjName " equals fixture "
                                                        <*> capRest @FixtureRef)) $
      \(ProjName pn, FixtureRef f) w ->
        case Map.lookup "_last" (bound w) of
          Nothing -> pure (Left "no response")
          Just (_, actual) -> case Map.lookup pn projections of
            Nothing -> pure (Left ("unknown projection " <> pn))
            Just p -> do
              let got = project p actual
                  path = fixtureDir w </> T.unpack f <> ".json"
              if blessMode w
                then do
                  -- Bless writes the PROJECTED value, not the raw
                  -- response -- the fixture pins what our parsers
                  -- consume, not everything the provider happens to
                  -- send.
                  BS.writeFile path (BL.toStrict (encodePretty got))
                  pure (Right w)
                else do
                  fx <- loadFixture w f
                  pure $ case fx of
                    Left e -> Left e
                    Right expected
                      | got == expected -> Right w
                      | otherwise -> Left ("consumed projection " <> pn <> " differs from fixture " <> f)
  ]
  where
    scene n w = maybe (Left ("unbound " <> n)) (Right . snd) (Map.lookup n (bound w))
    resourceIds v = case field "resources" v of
      Right (Array rs) -> traverse (field "id") (V.toList rs)
      other -> Left ("no resources array: " <> T.pack (show (() <$ other)))

-- ---------- Task 11: content-addressed resource ids ----------
-- Every id in a bound scene's "resources" array, text order, extracted
-- as plain Text (not Value, unlike the local `resourceIds` in
-- `allSteps`'s own `where` above, which the subset step uses for a
-- structural comparison) -- these three are used to build URLs
-- (/api/resource?id=..., /api/resources?ids=...), which need the raw
-- string.
resourceIdsOf :: Text -> World -> Either Text [Text]
resourceIdsOf n w = do
  (_, v) <- maybe (Left ("unbound " <> n)) Right (Map.lookup n (bound w))
  case field "resources" v of
    Right (Array rs) -> traverse idOf (V.toList rs)
    other -> Left ("no resources array: " <> T.pack (show (() <$ other)))
  where
    idOf r = case field "id" r of
      Right (String s) -> Right s
      Right other       -> Left ("resource id is not a string: " <> T.pack (show other))
      Left e            -> Left e

resourceSet :: Text -> World -> Either Text (Set Text)
resourceSet n w = Set.fromList <$> resourceIdsOf n w

firstResourceId :: Text -> World -> Either Text Text
firstResourceId n w = case resourceIdsOf n w of
  Right (i : _) -> Right i
  Right []      -> Left (n <> " has no resources")
  Left e        -> Left e

twoResourceIds :: Text -> World -> Either Text (Text, Text)
twoResourceIds n w = case resourceIdsOf n w of
  Right (i1 : i2 : _) -> Right (i1, i2)
  Right _             -> Left (n <> " has fewer than two resources")
  Left e              -> Left e

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

-- ---------- Task 10: masked whole-body fixture equality ----------
-- The owner's whole-body law forbids an existential poke ("some field
-- equals X"); a genuinely don't-care field (the compiled canon's graph
-- pin, which legitimately varies build to build) must instead be
-- declared VISIBLY in the scenario text and shape-checked, never
-- silently dropped. Enumerated (not Described) on purpose: unlike a
-- free-form field name or expected value, "which shapes exist" IS a
-- small, closed, growable vocabulary a dummy should be able to read off
-- the Vocabulary table — the same reasoning Piece/StyleName/Year get.
data MaskShape = SixteenHex deriving (Eq, Show)
instance FromCapture MaskShape where
  capName _ = "mask-shape"
  universe _ = Enumerated ["sixteen hex characters"]
  renderCap SixteenHex = "sixteen hex characters"
  parseCap t | T.strip t == "sixteen hex characters" = Right SixteenHex
             | otherwise = Left "unknown mask shape (shapes: sixteen hex characters)"

checkShape :: MaskShape -> Text -> Value -> Either Text ()
checkShape SixteenHex k (String s)
  | T.length s == 16 && T.all (`elem` ("0123456789abcdef" :: String)) s = Right ()
  | otherwise = Left (k <> " is not 16 hex chars: " <> s)
checkShape SixteenHex k _ = Left (k <> " is not a string")

setField :: Text -> Value -> Value -> Value
setField k v (Object o) = Object (KM.insert (K.fromText k) v o)
setField _ _ other = other

-- ---------- Task 12: the consumed projection ----------
-- A PROJECTION is a declared field-tree: applied to a provider's raw
-- response, it keeps exactly the fields our parsers (crates/map-compile/
-- src/vendor.rs) actually read -- recursively, through objects and
-- arrays -- and drops everything else. The projected value is then
-- compared WHOLE against the fixture: the CDC principle under the
-- whole-body law (pin the ENTIRE projection, not a poke at one field).
--
-- Two of vendor.rs's fields are not plain "keep this key" lookups:
--   * EventRow.label is `title`, falling back to `label` -- one OUTPUT
--     key sourced from the first of several INPUT keys that exists.
--   * EventRow.verses is not a top-level field at all; it is gathered by
--     walking witnesses[].verse_groups[].verses and concatenating every
--     leaf array found along the way.
-- Rather than special-case these two fields with ad hoc code, both are
-- expressed as instances of two general, reusable extractors
-- (`fieldAlt`, `flattenField`) alongside the ordinary `field1` -- the
-- SAME small vocabulary of combinators describes every endpoint's
-- projection, including the two that aren't plain lookups.
--
-- An `Extractor` is a function from the ENCLOSING object to the value
-- (if any) its declared output key should hold; `Nothing` omits the key
-- entirely (used for a field that is genuinely absent, or present as
-- JSON `null` -- vendor.rs's own `Option`-typed fields, color_key and
-- when, are read with `.and_then`, which treats a JSON `null` exactly
-- the same as a missing key, so the projection does too).
type Extractor = Value -> Maybe Value

data Proj = Keep | Fields [(Text, Extractor)] | Each Proj | Flatten [Text]

project :: Proj -> Value -> Value
project Keep v = v
project (Each p) (Array a) = Array (fmap (project p) a)
project (Each _) v = v
project (Flatten path) v = Array (V.fromList (flattenGather path v))
project (Fields fs) v@(Object _) = Object (KM.fromList
  [ (K.fromText k, pv) | (k, ext) <- fs, Just pv <- [ext v] ])
project (Fields _) v = v

-- The ordinary case: keep the field named `k`, projecting its value
-- through `p`. `Just Null` is treated the same as absent (see the
-- `Extractor` note above).
field1 :: Text -> Proj -> (Text, Extractor)
field1 k = fieldAlt k [k]

-- The general case field1 specializes: the OUTPUT key `outKey` is
-- sourced from the FIRST of `srcKeys` that is present (and non-null) on
-- the enclosing object -- vendor.rs's `label` (from `title`, falling
-- back to `label`) is `fieldAlt "label" ["title", "label"] Keep`.
fieldAlt :: Text -> [Text] -> Proj -> (Text, Extractor)
fieldAlt outKey srcKeys p = (outKey, \v -> case v of
  Object o -> project p <$> asum [ nonNull (KM.lookup (K.fromText k) o) | k <- srcKeys ]
  _        -> Nothing)
  where
    nonNull (Just Null) = Nothing
    nonNull other        = other

-- A field gathered by walking a path of array-valued keys and
-- concatenating every leaf array found at the end of it -- vendor.rs's
-- `verses` (not a top-level field; flattened out of
-- witnesses[].verse_groups[].verses) is
-- `flattenField "verses" ["witnesses", "verse_groups", "verses"]`.
flattenField :: Text -> [Text] -> (Text, Extractor)
flattenField outKey path = (outKey, \v -> Just (Array (V.fromList (flattenGather path v))))

flattenGather :: [Text] -> Value -> [Value]
flattenGather [] v = [v]
flattenGather (k : ks) (Object o) = case KM.lookup (K.fromText k) o of
  Just (Array arr)
    | null ks   -> V.toList arr
    | otherwise -> concatMap (flattenGather ks) (V.toList arr)
  _ -> []
flattenGather _ _ = []

-- The registry: one entry per atlas endpoint, mirroring vendor.rs's
-- parsers field for field (verified directly against
-- crates/map-compile/src/vendor.rs's parse_polities/parse_narratives/
-- parse_event/parse_eras/parse_landmarks/parse_land_mask, and against
-- the live-captured fixtures in crates/map-compile/fixtures/, before
-- being written -- see phase-S-report.md's verification section).
-- Changing a parser without changing its projection here (and
-- re-blessing) fails the CDC suite: the contract and the consumer
-- cannot drift apart.
projections :: Map.Map Text Proj
projections = Map.fromList
  [ ( "polities"
    , Fields
        [ field1 "polities" (Each (Fields
            [ field1 "id" Keep
            , field1 "name" Keep
            , field1 "from" Keep
            , field1 "to" Keep
            , field1 "rings" Keep
            , field1 "color_key" Keep
            , field1 "transition" (Fields [field1 "verses" Keep])
            , field1 "fall" (Fields [field1 "verses" Keep])
            ]))
        ]
    )
  , ( "narratives"
    , Each (Fields
        [ field1 "id" Keep, field1 "name" Keep, field1 "color" Keep, field1 "legs" Keep ])
    )
  , ( "event"
    , Fields
        [ field1 "id" Keep
        , fieldAlt "label" ["title", "label"] Keep
        , field1 "when" (Fields [field1 "from_year" Keep, field1 "to_year" Keep])
        , field1 "places" (Each (Fields [field1 "id" Keep]))
        , flattenField "verses" ["witnesses", "verse_groups", "verses"]
        ]
    )
  , ( "eras"
    , Each (Fields
        [ field1 "id" Keep, field1 "name" Keep, field1 "from_year" Keep, field1 "to_year" Keep ])
    )
  , ( "landmarks"
    , Each (Fields
        [ field1 "name" Keep, field1 "kind" Keep, field1 "lat" Keep, field1 "lon" Keep ])
    )
  , ("land-mask", Fields [ field1 "rings" Keep ])
  ]

-- Its universe IS the registry -- the Vocabulary block lists every
-- projection automatically, and a typo gets a did-you-mean naming the
-- real endpoints.
newtype ProjName = ProjName Text deriving (Eq, Show)
instance FromCapture ProjName where
  capName _ = "projection"
  universe _ = Enumerated (Map.keys projections)
  renderCap (ProjName p) = p
  parseCap t = let s = T.strip t in
    if s `Map.member` projections then Right (ProjName s)
    else Left ("'" <> s <> "' is not a projection."
              <> didYouMean (Map.keys projections) s
              <> "\n  Projections are: " <> T.intercalate ", " (Map.keys projections))
