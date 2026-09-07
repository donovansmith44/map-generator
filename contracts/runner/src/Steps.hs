module Steps where

import Data.Aeson (Value (..), eitherDecodeStrict)
import qualified Data.Aeson.Key as K
import qualified Data.Aeson.KeyMap as KM
import Data.Aeson.Encode.Pretty (encodePretty)
import qualified Data.ByteString as BS
import qualified Data.ByteString.Lazy as BL
import Data.Char (isSpace)
import Data.Foldable (asum)
import Data.List (sort)
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

-- WHAT A CAMERA IS, on the wire: a center AND a zoom, together. Not two
-- optional parameters -- the server builds a viewport only when BOTH are
-- present (`build_query`, crates/map-viewer/src/lib.rs:643-651, and the
-- characterization confirmed it: `center` alone is byte-identical to no
-- camera at all). A type with one constructor taking both is that fact,
-- written so a step cannot express half a camera.
data CamSpec = CamSpec Center Zoom deriving (Eq, Show)

camSpecCenter :: CamSpec -> Center
camSpecCenter (CamSpec c _) = c

camSpecZoom :: CamSpec -> Zoom
camSpecZoom (CamSpec _ z) = z

-- The one scene URL builder. Four optional things, each of which the
-- corpus writes in some scenarios and omits in others:
--
--   * the STYLE -- scene.feature's default-totality scenario says "in no
--     style" and means it: no `style=` parameter at all, so the server
--     answers with its own declared default rather than one this runner
--     chose. `Just`/`Nothing` is that distinction; a `StyleName
--     "classical"` sentinel would have been this runner inventing a
--     dress name the server never published.
--   * the CAMERA -- see `CamSpec`.
--   * the DETAIL tier, as an explicit `lod=`. Omitting it is not
--     "detail doesn't matter": it hands the choice to the server's auto
--     rule, which detail.feature's implicit/explicit scenario is
--     precisely about. So the absence is a value here too.
--
-- `zoom=` is written whether or not there is a camera, because it has
-- TWO jobs on this API and only one of them is the camera: with no
-- explicit `lod`, `zoom` also drives `auto_lod`. The no-camera default
-- stays the 90.0000 every existing scenario was already pinned at, so
-- no green scenario's URL moves.
sceneUrlFull :: Text -> PieceSet -> Year -> Maybe StyleName
             -> Maybe CamSpec -> Maybe DetailTier -> Text
sceneUrlFull base (PieceSet ps) (Year y) mst mcam mdetail =
  base <> "/api/scene?year=" <> tshow y
       <> "&zoom=" <> maybe "90.0000" (renderCap . camSpecZoom) mcam
       <> maybe "" (\c -> "&center=" <> renderCap (camSpecCenter c)) mcam
       <> maybe "" (\d -> "&lod=" <> tshow (tierLod d)) mdetail
       <> maybe "" (\(StyleName st) -> "&style=" <> st) mst
       <> flag Labels "labels" <> flag Water "topo" <> flag Journeys "journeys"
       <> (if Ground `member` ps then "&relief=1" else "")
  where
    flag p name = if p `member` ps then "" else "&" <> name <> "=0"

sceneUrl :: Text -> PieceSet -> Year -> StyleName -> Text
sceneUrl base ps y st = sceneUrlFull base ps y (Just st) Nothing Nothing

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

-- Final-review Fix 5 (deferred minor, promoted): against a 2.4 MB
-- manifest, "response differs from fixture X" names the fixture but not
-- WHAT differs -- not a diagnosis at all, just a pointer back to a huge
-- file the reader must now diff by hand. `firstDiff` walks both trees
-- together and reports the FIRST point they disagree -- a JSON path plus
-- the two values found there -- so `blessOrCompare`'s failure is
-- actionable on its own. Built to short-circuit: `firstJust` never
-- forces a later sibling once an earlier one has already reported a
-- difference, so a huge manifest that differs near the front is never
-- walked in full just to produce this message.
firstDiff :: Value -> Value -> Maybe Text
firstDiff = go "$"
  where
    go path expected actual
      | expected == actual = Nothing
      | otherwise = case (expected, actual) of
          (Object oe, Object oa) ->
            let ke = sort (map K.toText (KM.keys oe))
                ka = sort (map K.toText (KM.keys oa))
            in if ke /= ka
                 then Just (path <> ": object keys differ -- fixture has ["
                            <> T.intercalate ", " ke <> "], response has ["
                            <> T.intercalate ", " ka <> "]")
                 else firstJust
                        [ go (path <> "." <> k) ve va
                        | k <- ke
                        , Just ve <- [KM.lookup (K.fromText k) oe]
                        , Just va <- [KM.lookup (K.fromText k) oa]
                        ]
          (Array ae, Array aa)
            | V.length ae /= V.length aa ->
                Just (path <> ": array length differs -- fixture has "
                      <> tshow (V.length ae) <> " element(s), response has "
                      <> tshow (V.length aa))
            | otherwise ->
                firstJust
                  [ go (path <> "[" <> tshow i <> "]") (ae V.! i) (aa V.! i)
                  | i <- [0 .. V.length ae - 1] ]
          _ -> Just (path <> ": fixture has " <> bounded expected
                     <> ", response has " <> bounded actual)
    -- Short-circuiting "any": stops at the first Just without forcing the
    -- rest of the list (standard lazy foldr-based `any`/`asum` shape).
    firstJust :: [Maybe Text] -> Maybe Text
    firstJust = foldr (\x acc -> case x of Just _ -> x; Nothing -> acc) Nothing

tshow :: Show a => a -> Text
tshow = T.pack . show

-- A JSON value in an error message, truncated so a 2.4 MB manifest
-- cannot turn one failure line into the whole response body. Shared by
-- `firstDiff` and by the empty-list step (which quotes the body it found
-- instead of [] ) rather than defined twice.
bounded :: Value -> Text
bounded v = let s = tshow v
            in if T.length s > 120 then T.take 120 s <> "..." else s

-- What two resource-id sets disagree about, in both directions -- the
-- diagnosis a bare "not equal" withholds. Both directions matter and
-- mean different things: ids present in the composed side but not the
-- whole are things that appeared from nowhere, ids present in the whole
-- but not the composed side are things the parts failed to account for.
describeSetDiff :: Set Text -> Set Text -> Text
describeSetDiff got want
  | Set.null extra && Set.null missing = "the two sets are equal"
  | otherwise = T.intercalate "; " $
      [ tshow (Set.size extra) <> " id(s) present only in the composed side: "
        <> listIds extra | not (Set.null extra) ]
      ++
      [ tshow (Set.size missing) <> " id(s) present only in the whole: "
        <> listIds missing | not (Set.null missing) ]
  where
    extra   = Set.difference got want
    missing = Set.difference want got
    -- bounded the same way `bounded` bounds a body: five ids is enough
    -- to recognize the pattern, and a scene can carry thousands.
    listIds s = T.intercalate ", " (take 5 (Set.toList s))
                <> (if Set.size s > 5 then ", ..." else "")

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
            | otherwise -> Left ("response differs from fixture " <> fname <> ": "
                                 <> maybe "(no leaf difference found)" id (firstDiff expected v))

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
                    else Left ("body differs from fixture " <> f <> " outside the mask: "
                               <> maybe "(no leaf difference found)" id
                                    (firstDiff expected actual'))
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
    -- The sweep: "when no time passes, nothing changes" is quantified
    -- over every year, so it can no longer be pinned against a blessed
    -- fixture (a whole-body fixture cannot be blessed against a
    -- generated year). WHOLE-BODY equality against the literal empty
    -- list is the honest replacement -- not "the response has no
    -- elements", not "some field is empty", but: the entire body is [].
    -- That is the owner's whole-body law applied to the one body small
    -- enough to write out in full.
  , mkStep Then (lit "the response is the empty list") $ \() w ->
      pure $ case Map.lookup "_last" (bound w) of
        Nothing -> Left "no response"
        Just (_, v)
          | v == Array V.empty -> Right w
          | otherwise -> Left ("the whole response body is not [] -- it is " <> bounded v)
    -- ============ the step phase's Then vocabulary ============
    -- Listed BEFORE the four `lit ""`-prefixed generic steps further
    -- down ("{name} equals {name}", the subset step, labels-are-empty,
    -- and dress-locality), specific before generic. Not load-bearing
    -- under R23 -- a real match always wins over a mere claim -- but it
    -- is what decides which message a human sees when one of these
    -- lines is genuinely malformed: without it, "narrow's markers and
    -- labels are a subset of wide's" reports "a bind name is a single
    -- word" from the dress-locality step's capture, a true statement
    -- about a capture and a useless one about the line.

    -- camera.feature's two-sided culling law (@target). BOTH sides are
    -- computed and both are required, which is the whole point:
    -- characterization C5's named trap is a law that asserts only "far
    -- things are absent", which culling everything satisfies. The
    -- partition comes from ONE predicate and its negation (`inView` /
    -- `outOfView`), so there is no gap between the halves and no overlap.
    --
    -- Two precondition guards, and neither is politeness. A draw whose
    -- camera leaves NOTHING out of view cannot exercise the "omits"
    -- half, and a draw that leaves nothing in view cannot exercise the
    -- "keeps" half; passing either way would be a green earned by the
    -- draw rather than by the server (MEMORY: verify-distinct-not-
    -- nonnull). At zoom 45 the cap radius is 81 degrees, so the first
    -- case is real, not hypothetical.
  , mkSkippableStep Then (lit "" *> ((,,) <$> capUntil @BindName " keeps every feature of "
                                          <*> capUntil @BindName " in view and omits every feature of "
                                          <*> capUntil @BindName " out of view")) $
      \(BindName viewed, BindName ref, BindName ref2) w -> pure $
        either StepFailed id $ do
          () <- if ref == ref2 then Right ()
                else Left ("this law compares one reference scene against " <> viewed
                           <> ", but names two: " <> ref <> " and " <> ref2)
          vw <- cameraOf viewed w
          caps <- featureCaps =<< boundScene ref w
          seen <- featureIdSet =<< boundScene viewed w
          let ins  = [ f | (f, c) <- caps, inView vw c ]
              outs = [ f | (f, c) <- caps, outOfView vw c ]
              missing = [ f | f <- ins, not (f `Set.member` seen) ]
              leaked  = [ f | f <- outs, f `Set.member` seen ]
          pure $ case (null ins, null outs) of
            (True, _) -> StepSkipped
              ("no feature of " <> ref <> " is in " <> viewed
               <> "'s view at this camera: this draw cannot exercise the \"keeps\" half")
            (_, True) -> StepSkipped
              ("every feature of " <> ref <> " is in " <> viewed
               <> "'s view at this camera: this draw cannot exercise the \"omits\" half")
            _ | null missing && null leaked -> StepOk w
              | otherwise -> StepFailed
                  (viewed <> " does not partition " <> ref <> "'s "
                   <> tshow (length caps) <> " features by the view: "
                   <> tshow (length missing) <> " in view but absent ("
                   <> listSome missing <> "); " <> tshow (length leaked)
                   <> " out of view but sent (" <> listSome leaked <> ")")
    -- "moving the camera never redraws what stays visible" -- content
    -- addressing, stated the way it can actually fail: for every id the
    -- two manifests SHARE, the whole published record must agree.
    -- Characterization C3's trap is that this is true by construction of
    -- the hash and therefore certifies nothing on its own; it is worth
    -- pinning anyway because the thing it would catch (an id that is not
    -- a function of its bytes) is catastrophic, and because C4 -- the
    -- law with the real content -- is stated separately as the @target
    -- above. Whole record, not a chosen triple of fields.
  , mkSkippableStep Then (lit "every resource " *> ((,) <$> capUntil @BindName " and "
                                                        <*> capUntil @BindName " share is byte-identical in both")) $
      \(BindName a, BindName b) w -> pure $ either StepFailed id $ do
        ra <- resourceRecords =<< boundScene a w
        rb <- resourceRecords =<< boundScene b w
        let shared = Map.keys (Map.intersection ra rb)
            differ = [ i | i <- shared, Map.lookup i ra /= Map.lookup i rb ]
        pure $ if null shared
          then StepSkipped (a <> " and " <> b <> " share no resource ids at this draw; \
                            \content addressing has nothing to be tested against here")
          else if null differ then StepOk w
          else StepFailed (tshow (length differ) <> " of " <> tshow (length shared)
                           <> " shared resource ids serve different records in " <> a
                           <> " and " <> b <> ": " <> listSome differ)
    -- "zooming out only reveals markers and labels -- it never removes
    -- them". Stated over the two kinds the camera ACTUALLY culls
    -- (characterization 1.0: Points and Memories are the only elements
    -- the viewport removes), by their published ids -- marker place and
    -- label subject -- which is characterization C1/C2's non-vacuous
    -- form. Written over feature ids instead it would pass while the
    -- camera did nothing at all (917 ids at every zoom); that version is
    -- the @target above.
  , mkSkippableStep Then (lit "" *> ((,) <$> capUntil @BindName "'s markers and labels are a subset of "
                                         <*> capUntil @BindName "'s")) $
      \(BindName a, BindName b) w -> pure $ either StepFailed id $ do
        ia <- Set.fromList . map fst <$> (cameraCulled =<< boundScene a w)
        ib <- Set.fromList . map fst <$> (cameraCulled =<< boundScene b w)
        let extra = Set.difference ia ib
        pure $ if Set.null ia
          then StepSkipped (a <> " carries no markers and no labels at this draw; \
                            \the empty set is a subset of anything")
          else if Set.null extra then StepOk w
          else StepFailed (tshow (Set.size extra) <> " marker/label id(s) of " <> a
                           <> " are absent from " <> b <> ": " <> listSome (Set.toList extra))
    -- "zooming in never loses a marker or label you are looking at" --
    -- the converse, and the one that needs the camera: only the things
    -- still inside the NARROWER view are owed.
  , mkSkippableStep Then (lit "every marker and label of " *> ((,,) <$> capUntil @BindName " still in "
                                                                    <*> capUntil @BindName "'s view is kept by "
                                                                    <*> capRest @BindName)) $
      \(BindName wide, BindName narrowView, BindName narrow) w -> pure $
        either StepFailed id $ do
          vw <- cameraOf narrowView w
          ws <- cameraCulled =<< boundScene wide w
          kept <- Set.fromList . map fst <$> (cameraCulled =<< boundScene narrow w)
          let owed = [ i | (i, p) <- ws, pointInView vw p ]
              lost = [ i | i <- owed, not (i `Set.member` kept) ]
          pure $ if null owed
            then StepSkipped ("nothing of " <> wide <> " lies inside " <> narrowView
                              <> "'s view at this draw; this law has nothing to be owed")
            else if null lost then StepOk w
            else StepFailed (tshow (length lost) <> " of " <> tshow (length owed)
                             <> " marker/label id(s) of " <> wide <> " inside " <> narrowView
                             <> "'s view are missing from " <> narrow <> ": " <> listSome lost)
    -- "the far side of the globe is never sent" (@target). The horizon
    -- is a property of the CENTER alone -- no zoom term -- so this step
    -- takes the center literally, from the scenario's own <someCenter>,
    -- rather than looking one up.
  , mkSkippableStep Then (lit "no feature of " *> ((,) <$> capUntil @BindName " is beyond the horizon of "
                                                       <*> capRest @Center)) $
      \(BindName n, c) w -> pure $ either StepFailed id $ do
        caps <- featureCaps =<< boundScene n w
        let eye = unitOf c
            over = [ f | (f, cap) <- caps, beyondHorizon eye cap ]
            reachable = [ f | (f, cap) <- caps, not (coversSphere cap) ]
        pure $ if null reachable
          then StepSkipped (n <> " carries no feature with bounds smaller than the whole \
                            \sphere; nothing here can be beyond any horizon")
          else if null over then StepOk w
          else StepFailed (tshow (length over) <> " of " <> tshow (length caps)
                           <> " features of " <> n <> " lie entirely beyond the horizon of "
                           <> renderCap c <> ": " <> listSome over)
    -- "a label is only sent when the thing it names is in view"
    -- (@target) -- the screenshots' law. Characterization C10's trap is
    -- an existential ("some labels are culled"), which 6 of 833 satisfy;
    -- this is stated over every label, and reports how many of how many.
  , mkSkippableStep Then (lit "every label of " *> capUntil @BindName " anchors in view") $
      \(BindName n) w -> pure $ either StepFailed id $ do
        vw <- cameraOf n w
        ls <- labelAnchors =<< boundScene n w
        let outside = [ i | (i, p) <- ls, not (pointInView vw p) ]
        pure $ if null ls
          then StepSkipped (n <> " carries no labels at this draw; a law about where \
                            \labels anchor has nothing to examine")
          else if null outside then StepOk w
          else StepFailed (tshow (length outside) <> " of " <> tshow (length ls)
                           <> " labels of " <> n <> " anchor outside its own view: "
                           <> listSome outside)
    -- ---------- detail.feature ----------
    -- "detail changes how much is drawn, never what exists" --
    -- characterization D1, which HOLDS exactly (917 ids at all 13 lod
    -- rungs). The whole feature-id set on both sides, compared as sets,
    -- with the difference reported in both directions.
  , mkStep Then (lit "" *> ((,) <$> capUntil @BindName " and "
                                <*> capUntil @BindName " draw the same features")) $
      \(BindName a, BindName b) w -> pure $ do
        ia <- featureIdSet =<< boundScene a w
        ib <- featureIdSet =<< boundScene b w
        if ia == ib then Right w
        else Left (a <> " and " <> b <> " do not draw the same features: "
                   <> describeSetDiff ia ib)
    -- "leaning in never loses geometry" (@target): per shared resource
    -- id, finer must carry at least as many vertices. Per RESOURCE, not
    -- summed -- characterization D5's trap is that the sum hides which
    -- features exploded, and 498 of 917 do at the auto ceiling.
    --
    -- Four names, one chain: `capUntil` breaks on the FIRST occurrence
    -- of its terminator, so " as in " -> ", and in " -> " as in " ->
    -- capRest walks the sentence left to right exactly as a reader does.
    -- The last two names repeat the first two by design ("... in ultra
    -- as in fine"), which is what makes this one ladder of two rungs
    -- rather than two unrelated comparisons.
  , mkSkippableStep Then (lit "every shared resource has at least as many vertices in "
                          *> ((,,,) <$> capUntil @BindName " as in "
                                    <*> capUntil @BindName ", and in "
                                    <*> capUntil @BindName " as in "
                                    <*> capRest @BindName)) $
      \(BindName finer, BindName coarser, BindName finest, BindName mid) w ->
        pure $ either StepFailed id $ do
          l1 <- vertexRung finer coarser w
          l2 <- vertexRung finest mid w
          pure $ case (rungShared l1, rungShared l2) of
            (0, 0) -> StepSkipped
              (finer <> "/" <> coarser <> " and " <> finest <> "/" <> mid
               <> " share no features at this draw; a per-feature comparison \
                  \has nothing to compare")
            _ | null (rungBad l1) && null (rungBad l2) -> StepOk w
              | otherwise -> StepFailed
                  (describeRung finer coarser l1 <> "; " <> describeRung finest mid l2)
    -- "the world at a glance is never heavier than the street corner"
    -- (@target) -- characterization C9/1.5: 490 of 917 features carry
    -- MORE geometry at the deepest zoom than at the widest, because
    -- below-limit rings ship unsimplified.
  , mkSkippableStep Then (lit "no shared resource of " *> ((,) <$> capUntil @BindName " carries more vertices than it does in "
                                                                <*> capRest @BindName)) $
      \(BindName glance, BindName corner) w -> pure $ either StepFailed id $ do
        r <- vertexRung corner glance w
        pure $ if rungShared r == 0
          then StepSkipped (glance <> " and " <> corner <> " share no features \
                            \at this draw; nothing to weigh against anything")
          else if null (rungBad r) then StepOk w
          else StepFailed (describeRung corner glance r)
    -- ---------- transition.feature ----------
    -- "no time passing means nothing moves": the WHOLE steps array is
    -- the empty list, quoted in full when it is not.
  , mkStep Then (lit "" *> capUntil @BindName "'s steps are the empty list") $
      \(BindName n) w -> pure $ do
        sts <- planSteps =<< boundScene n w
        if null sts then Right w
        else Left (n <> "'s plan carries " <> tshow (length sts)
                   <> " step(s), not none: " <> bounded (Array (V.fromList (take 3 sts))))
    -- "the plan and the timeline tell one story, wherever you scrub" --
    -- characterization T6, a BIJECTION ON IDS, both directions, both
    -- kinds. T6's own trap is comparing counts: the timeline also
    -- carries `journey` rows that deliberately produce no step, so
    -- len(steps) /= len(changes) and a count law would be WRONG as well
    -- as weak.
  , mkSkippableStep Then (lit "every fade in " *> ((,,,) <$> capUntil @BindName " is a rise or fall in "
                                                          <*> capUntil @BindName ", and every rise and fall in "
                                                          <*> capUntil @BindName " has a fade in "
                                                          <*> capRest @BindName)) $
      \(BindName plan, BindName story, BindName story2, BindName plan2) w ->
        pure $ either StepFailed id $ do
          () <- sameTwice "plan" plan plan2
          () <- sameTwice "timeline" story story2
          sts <- planSteps =<< boundScene plan w
          ch <- boundScene story w
          ins <- fadeRegions "fade_in" sts
          outs <- fadeRegions "fade_out" sts
          rises <- changeSubjects "rise" "region:" ch
          falls <- changeSubjects "fall" "region:" ch
          pure $ if Set.null ins && Set.null outs && Set.null rises && Set.null falls
            then StepSkipped ("this span carries no fades and no rises or falls; \
                              \a bijection between empty sets demonstrates nothing")
            else if ins == rises && outs == falls then StepOk w
            else StepFailed ("fade_in vs rise: " <> describeSetDiff ins rises
                             <> " -- fade_out vs fall: " <> describeSetDiff outs falls)
    -- "what fades in arrives, what fades out departs" (@target) --
    -- characterization T8, which is PARTIAL today: 56 region ids named
    -- by fades are never a region feature at any stop. T8's trap is
    -- checking only that the id is ABSENT from the other endpoint (a
    -- nonexistent region is absent from both, so it passes); both
    -- directions are required here.
  , mkSkippableStep Then (lit "every fade-in region of " *> ((,,,,) <$> capUntil @BindName " is in "
                                                                    <*> capUntil @BindName " and not "
                                                                    <*> capUntil @BindName ", and every fade-out region is in "
                                                                    <*> capUntil @BindName " and not "
                                                                    <*> capRest @BindName)) $
      \(BindName plan, BindName after, BindName before, BindName before2, BindName after2) w ->
        pure $ either StepFailed id $ do
          () <- sameTwice "later scene" after after2
          () <- sameTwice "earlier scene" before before2
          sts <- planSteps =<< boundScene plan w
          ins <- fadeRegions "fade_in" sts
          outs <- fadeRegions "fade_out" sts
          fa <- featureIdSet =<< boundScene after w
          fb <- featureIdSet =<< boundScene before w
          let regionOf i = "region:" <> i
              badIn  = [ i | i <- Set.toList ins
                           , not (regionOf i `Set.member` fa) || regionOf i `Set.member` fb ]
              badOut = [ i | i <- Set.toList outs
                           , not (regionOf i `Set.member` fb) || regionOf i `Set.member` fa ]
          pure $ if Set.null ins && Set.null outs
            then StepSkipped "this span's plan carries no fades; a law about what fades \
                             \in and out has nothing to examine"
            else if null badIn && null badOut then StepOk w
            else StepFailed (tshow (length badIn) <> " of " <> tshow (Set.size ins)
                             <> " fade-in region(s) are not new in " <> after <> " ("
                             <> listSome badIn <> "); " <> tshow (length badOut) <> " of "
                             <> tshow (Set.size outs) <> " fade-out region(s) are not gone from "
                             <> after <> " (" <> listSome badOut <> ")")
    -- "the road back is the road there, reversed" -- characterization
    -- T5, which HOLDS exactly. Whole-body: the constructed mirror is
    -- compared against the served reverse plan step for step, with
    -- `firstDiff` naming the first disagreement. T5's trap is comparing
    -- step counts or kind multisets, both of which a shuffled order
    -- satisfies.
  , mkSkippableStep Then (lit "" *> ((,) <$> capUntil @BindName " is "
                                         <*> capUntil @BindName " with every morph reversed and every fade inverted")) $
      \(BindName back, BindName there) w -> pure $ either StepFailed id $ do
        bs <- planSteps =<< boundScene back w
        ts <- planSteps =<< boundScene there w
        let want = Array (V.fromList (mirrorPlan ts))
            got  = Array (V.fromList bs)
        pure $ if null ts && null bs
          then StepSkipped ("this span's plan carries no steps in either direction; \
                            \mirroring nothing demonstrates nothing")
          else if want == got then StepOk w
          else StepFailed (back <> " is not the mirror of " <> there <> ": "
                           <> maybe "(no leaf difference found)" id (firstDiff want got))
    -- "a border morphs with its real shape, not a stick figure"
    -- (@target) -- characterization T13/4.9: /api/transition defaults to
    -- Lod(6.0), which collapses every morph in this canon to a two-point
    -- great-circle segment. T13's trap is passing an explicit lod in the
    -- test and never exercising the default; the corpus's URL carries no
    -- lod, on purpose, so this measures the default.
  , mkSkippableStep Then (lit "every morph of " *> ((,) <$> capUntil @BindName " carries at least as many points as its border carries vertices in "
                                                        <*> capRest @BindName)) $
      \(BindName plan, BindName scn) w -> pure $ either StepFailed id $ do
        sts <- planSteps =<< boundScene plan w
        sv <- boundScene scn w
        byId <- resourceRecords sv
        fs <- arrayOf "features" sv
        let morphs = stepsOfKind "morph" sts
        pairs <- traverse (morphPoints byId fs) morphs
        let bad = [ (b, n, v) | (b, n, v) <- pairs, n < v ]
        pure $ if null morphs
          then StepSkipped ("this span's plan carries no morphs; a law about a morph's \
                            \geometry has nothing to examine")
          else if null bad then StepOk w
          else StepFailed (tshow (length bad) <> " of " <> tshow (length morphs)
                           <> " morphs carry fewer points than their border's vertices in "
                           <> scn <> ": "
                           <> T.intercalate ", " [ b <> " " <> tshow n <> "<" <> tshow v
                                                 | (b, n, v) <- take 5 bad ])
    -- ---------- the small edits ----------
    -- subjects.feature: "a question about a span is not answered as a
    -- question about an instant".
    --
    -- On "refused or": a refusal cannot reach this step. The GET-as step
    -- above turns a non-2xx into a Left (World.checkStatus), so a server
    -- that refused the span would fail the scenario at its When line,
    -- naming the status -- red, not green, though the law would be
    -- satisfied. That is a real limitation of the binding step's
    -- contract, recorded here rather than papered over; what this step
    -- can and does decide is the other half, and it is the half today's
    -- server actually exercises.
  , mkStep Then (lit "" *> ((,) <$> capUntil @BindName " is refused or differs from "
                                <*> capRest @BindName)) $
      \(BindName span_, BindName instant) w -> pure $ do
        a <- boundScene span_ w
        b <- boundScene instant w
        if a /= b then Right w
        else Left (span_ <> " was answered with exactly the body " <> instant
                   <> " was answered with: the span parameter was ignored, not refused")
    -- resources.feature (@target): a batch containing an id the store
    -- does not hold must say so. Today /api/resources
    -- (crates/map-viewer/src/lib.rs:838-851) filters the unknown id out
    -- and returns 200 with the rest, so the caller cannot tell a missing
    -- payload from a short one.
  , mkSkippableStep Then (lit "fetching " *> capUntil @BindName "'s first resource alongside a bogus id is refused by name") $
      \(BindName a) w -> case firstResourceId a w of
        Broken e -> pure (StepFailed e)
        Unmet why -> pure (StepSkipped why)
        Met rid -> case resourceRecords =<< boundScene a w of
          Left e -> pure (StepFailed e)
          Right byId
            | Map.member bogusResourceId byId -> pure (StepSkipped
                ("this scene publishes " <> bogusResourceId <> ", the id this law uses \
                 \as its known-absent one; it cannot be used as a bogus id here"))
            | otherwise -> do
                batch <- transportRaw w (baseUrl w <> "/api/resources?ids=" <> rid
                                         <> "," <> bogusResourceId)
                single <- transportRaw w (baseUrl w <> "/api/resource?id=" <> rid)
                pure $ case (batch, single) of
                  -- A refusal IS the law being met: the transport turns
                  -- a non-2xx into a Left, and that is the green case.
                  (Left _, _) -> StepOk w
                  (Right bb, Right b1)
                    | bb == b1 -> StepFailed
                        ("the batch answered 200 with exactly the bytes of the one resident \
                         \id, silently dropping " <> bogusResourceId
                         <> ": a caller cannot tell a missing payload from a short one")
                    | otherwise -> StepFailed
                        ("the batch answered 200 rather than refusing the unknown id "
                         <> bogusResourceId <> " by name")
                  (_, Left e) -> StepFailed e
    -- derivability.feature (@target): the manifest publishes no
    -- disposition and no border attribution per entry, so the scene tier
    -- cannot today be traced back to the fact tier at all. Computed, not
    -- stubbed: it really looks for the two fields, and reports which of
    -- them is missing from how many entries.
  , mkStep Then (lit "every entry in " *> capUntil @BindName " traces to a disposition and a border") $
      \(BindName n) w -> pure $ do
        fs <- arrayOf "features" =<< boundScene n w
        let missing k = length [ () | f <- fs, either (const True) (const False) (field k f) ]
        if null fs then Left (n <> " carries no manifest entries to trace")
        else if missing "disposition" == 0 && missing "border" == 0 then Right w
        else Left (tshow (missing "disposition") <> " of " <> tshow (length fs)
                   <> " entries carry no disposition, and " <> tshow (missing "border")
                   <> " carry no border: the scene tier cannot be traced to the fact \
                      \tier (disposition is Stage 2, borders Stage 3)")
    -- census.feature: a BOUND response against a fixture. The existing
    -- fixture step compares the LAST response; a @property scenario that
    -- binds its response under a name (so the counterexample can report
    -- it) needs to name it here too. Listed before the generic
    -- "{name} equals {name}" step, whose capRest cannot parse
    -- `fixture "census-diff-empty"` and so only ever claims this line.
  , mkStep Then (lit "" *> ((,) <$> capUntil @BindName " equals fixture "
                                <*> capRest @FixtureRef)) $
      \(BindName n, FixtureRef f) w -> case Map.lookup n (bound w) of
        Nothing -> pure (Left ("unbound " <> n))
        Just pair_ -> blessOrCompare f w { bound = Map.insert "_last" pair_ (bound w) }
    -- scene steps (piece vocabulary on the wire)
    -- ============ the camera's render vocabulary ============
    -- Six shapes, all one request with different parts present or
    -- absent (see `sceneUrlFull`). Listed before the two plain render
    -- steps they overlap with on "I render pieces ", specific before
    -- generic: on a camera line the plain steps' `capRest @StyleName`
    -- swallows "canaan looking at 31.5,35.0 zoom 4 detail fine as
    -- viewed" whole and reports "... is not a style" -- a true statement
    -- about a capture and a useless one about the line, and exactly the
    -- misclassification the corpus phase flagged as its finding 3.
  , mkStep When (lit "I render pieces "
                 *> ((,,,,,,) <$> capUntil @PieceSet " at year "
                              <*> capUntil @Year " in style "
                              <*> capUntil @StyleName " looking at "
                              <*> capUntil @Center " zoom "
                              <*> capUntil @Zoom " detail "
                              <*> capUntil @DetailTier " as "
                              <*> capRest @BindName)) $
      \(ps, y, st, c, z, d, BindName n) ->
        renderInto (Just n) ps y (Just st) (Just (CamSpec c z)) (Just d)
  , mkStep When (lit "I render pieces "
                 *> ((,,,,,,,) <$> capUntil @PieceSet " at year "
                               <*> capUntil @Year " in style "
                               <*> capUntil @StyleName " looking at "
                               <*> capUntil @Center " zoom "
                               <*> capUntil @Zoom " "
                               <*> capUntil @ScaleQual " detail "
                               <*> capUntil @DetailTier " as "
                               <*> capRest @BindName)) $
      \(ps, y, st, c, z, sc, d, BindName n) ->
        renderInto (Just n) ps y (Just st) (Just (CamSpec c (applyScale sc z))) (Just d)
  , mkStep When (lit "I render pieces "
                 *> ((,,,,,) <$> capUntil @PieceSet " at year "
                             <*> capUntil @Year " in style "
                             <*> capUntil @StyleName " looking at "
                             <*> capUntil @Center " zoom "
                             <*> capUntil @Zoom " as "
                             <*> capRest @BindName)) $
      \(ps, y, st, c, z, BindName n) ->
        renderInto (Just n) ps y (Just st) (Just (CamSpec c z)) Nothing
  , mkStep When (lit "I render pieces "
                 *> ((,,,,,) <$> capUntil @PieceSet " at year "
                             <*> capUntil @Year " in style "
                             <*> capUntil @StyleName " looking at "
                             <*> capUntil @Center " zoom "
                             <*> capUntil @Zoom " detail "
                             <*> capRest @DetailTier)) $
      \(ps, y, st, c, z, d) ->
        renderInto Nothing ps y (Just st) (Just (CamSpec c z)) (Just d)
  , mkStep When (lit "I render pieces "
                 *> ((,,,,) <$> capUntil @PieceSet " at year "
                            <*> capUntil @Year " in style "
                            <*> capUntil @StyleName " detail "
                            <*> capUntil @DetailTier " as "
                            <*> capRest @BindName)) $
      \(ps, y, st, d, BindName n) ->
        renderInto (Just n) ps y (Just st) Nothing (Just d)
    -- "in no style", meaning it: no `style=` parameter at all, so the
    -- server answers with the dress IT declares as classical rather than
    -- one this runner picked. That absence is the whole content of
    -- scene.feature's default-totality scenario.
  , mkStep When (lit "I render pieces "
                 *> ((,) <$> capUntil @PieceSet " at year "
                         <*> capUntil @Year " in no style")) $
      \(ps, y) -> renderInto Nothing ps y Nothing Nothing Nothing
  , mkStep When (lit "I render pieces "
                 *> ((,,,) <$> capUntil @PieceSet " at year "
                           <*> capUntil @Year " in style "
                           <*> capUntil @StyleName " as "
                           <*> capRest @BindName)) $
      \(ps, y, st, BindName n) w -> do
        r <- getUrl (sceneUrl (baseUrl w) ps y st) w
        pure $ do
          w' <- r >>= bindLast n
          -- Task 11 / Phase S review fixes 2-3: the combine step
          -- ("combining A and B equals rendering <someA> plus <someB>")
          -- must render its own union scene at the SAME year AND STYLE
          -- the two parts were rendered at, not a hardcoded pair (the
          -- plan's own self-review flagged the year half of this bug;
          -- review flagged that a hardcoded style is the same mistake).
          -- Every binding render records the year+style it used, typed,
          -- in `World.lastRender` (see World.hs); a @property scenario
          -- substitutes ONE <someYear> value across the whole scenario
          -- body, so sceneA and sceneB's renders (and therefore this
          -- binding) always agree.
          Right w' { lastRender = Just (y, st) }
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
        pure $ case (resourceIdsIn =<< scene a w, resourceIdsIn =<< scene b w) of
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
    -- Phase S review, cheap fix 4: `V.all` over an EMPTY array is
    -- vacuously True, so an empty manifest would report this @target as
    -- already met -- guarded explicitly rather than trusted to the
    -- vacuous case.
  , mkStep Then (lit "every feature entry carries a piece field") $ \() w ->
      pure $ case Map.lookup "_last" (bound w) of
        Nothing -> Left "no response"
        Just (_, v) -> case field "features" v of
          Right (Array fs)
            | V.null fs -> Left "no features to check piece attribution against (empty manifest)"
            | V.all (\f -> either (const False) (const True) (field "piece" f)) fs -> Right w
            | otherwise -> Left "manifest entries carry no piece attribution (v0.1 wart)"
          -- Phase S review, cheap fix 2: prefixed the way `resourceIds`
          -- (this module's `where` clause) prefixes its own shape
          -- mismatch, naming the field this step actually looked for.
          other -> Left ("no features array: " <> T.pack (show (() <$ other)))
    -- Task 11 (@target @property): v0.1 has no server-side combine
    -- endpoint, so the law is checked indirectly -- render the union of
    -- the two piece sets and assert its resource-id set equals the
    -- UNION of the two parts' resource-id sets, at the SAME year AND
    -- STYLE the parts were rendered at (World.lastRender, set by the
    -- binding render step above).
  , mkStep Then (lit "combining " *> ((,,,) <$> capUntil @BindName " and "
                                            <*> capUntil @BindName " equals rendering "
                                            <*> capUntil @PieceSet " plus "
                                            <*> capRest @PieceSet)) $
      \(BindName a, BindName b, PieceSet sa, PieceSet sb) w ->
        case lastRender w of
          Nothing -> pure (Left "no year/style recorded for this scene (render a piece set first)")
          Just (y, st) -> do
            r <- getUrl (sceneUrl (baseUrl w) (PieceSet (Set.union sa sb)) y st) w
            pure $ do
              w' <- r
              both <- resourceSet "_last" w'
              ra <- resourceSet a w'
              rb <- resourceSet b w'
              if both == Set.union ra rb then Right w'
              else Left "union scene is not the union of its parts' resources"
    -- The sweep: the monoid's IDENTITY law ("an empty map stacked onto
    -- any map changes nothing"), stated in v0.1's terms. "Combining"
    -- keeps exactly the meaning the composition step above gives it --
    -- the union of two scenes' resource sets (owner-ratified design
    -- decision) -- so `combining some and empty equals some` asks
    -- whether resources(some) ∪ resources(empty) is resources(some).
    -- Purely local: all three operands are already-bound scenes, so
    -- unlike the composition step this one renders nothing of its own.
    --
    -- Listed AFTER the "equals rendering {pieces} plus {pieces}"
    -- overload it overlaps with on the "combining " literal: specific
    -- before generic, for error quality (not load-bearing under R23 --
    -- this definition's capRest @BindName cannot parse "rendering fills
    -- plus ground", so it only ever ClaimErrors on that line while the
    -- overload above genuinely Matches).
  , mkStep Then (lit "combining " *> ((,,) <$> capUntil @BindName " and "
                                           <*> capUntil @BindName " equals "
                                           <*> capRest @BindName)) $
      \(BindName a, BindName b, BindName c) w ->
        pure $ do
          ra <- resourceSet a w
          rb <- resourceSet b w
          rc <- resourceSet c w
          let combined = Set.union ra rb
          if combined == rc then Right w
          else Left ("combining " <> a <> " and " <> b <> " is not " <> c
                     <> ": " <> describeSetDiff combined rc)
    -- The sweep (@target): the singleton fold -- draw each piece of the
    -- set ALONE, stack the results, and you must get the whole scene
    -- back. Every single render happens at the SAME year and style the
    -- whole was rendered at (World.lastRender, set by the binding render
    -- step above) -- the same discipline the composition step already
    -- keeps, and for the same reason: a hardcoded year or style would be
    -- a tuned constant, and would silently compare two different maps.
    --
    -- The empty set SKIPS: a fold over no pieces demonstrates nothing
    -- about whether pieces compose (its union is trivially empty, and
    -- an empty scene's resources would have to be empty too for the
    -- comparison to mean anything). Vacuously passing there would be a
    -- law satisfied by its own failure mode.
  , mkSkippableStep Then (lit "rendering each piece of "
                          *> ((,) <$> capUntil @PieceSet " alone and combining them equals "
                                  <*> capRest @BindName)) $
      \(PieceSet ps, BindName whole) w ->
        case lastRender w of
          Nothing -> pure (StepFailed
            "no year/style recorded for this scene (render a piece set first)")
          Just (y, st)
            | Set.null ps -> pure (StepSkipped
                "the piece set drawn for this iteration is empty: a fold over no pieces \
                \demonstrates nothing about how pieces compose")
            | otherwise -> do
                parts <- mapM (renderPieceAlone w y st) (Set.toList ps)
                pure $ either StepFailed StepOk $ do
                  sets <- sequence parts
                  expected <- resourceSet whole w
                  let stacked = Set.unions sets
                  if stacked == expected then Right w
                  else Left ("stacking the " <> tshow (Set.size ps)
                             <> " single-piece renders does not rebuild " <> whole
                             <> ": " <> describeSetDiff stacked expected)
    -- Task 11 (@target): geometry is content-addressed (an id IS its
    -- bytes -- see resources.feature's own title); dress rides styles,
    -- not payload ids, so restyling one piece must leave every id set
    -- EQUAL, not merely overlapping.
    --
    -- Final-review Fix 2: equal geometry ids alone only proves HALF of
    -- this step's own name -- "never in geometry". A server that ignored
    -- `style=` entirely (never re-dressed anything) would ALSO leave the
    -- id sets equal and satisfy the old check, without ever demonstrating
    -- "differ only in dress" at all. Masking the shared geometry id array
    -- OUT of each whole bound body and requiring the REMAINDER to
    -- genuinely differ proves the other half: something besides geometry
    -- really did change between the two renders. This does not weaken
    -- the geometry check -- both conditions are required, `&&`-style
    -- (short-circuited as two sequential Either binds below).
  , mkStep Then (lit "" *> ((,) <$> capUntil @BindName " and "
                                <*> (capUntil @BindName " differ only in dress, never in geometry"))) $
      \(BindName a, BindName b) w ->
        pure $ do
          ra <- resourceSet a w
          rb <- resourceSet b w
          if ra /= rb
            then Left "restyle changed geometry ids: dress is not local"
            else do
              va <- scene a w
              vb <- scene b w
              if setField "resources" Null va == setField "resources" Null vb
                then Left ("dress did not actually change: " <> a <> " and " <> b
                           <> " are identical once geometry ids are masked out")
                else Right w
    -- Task 11: /api/resource returns a BINARY body -- transportRaw, not
    -- the JSON-decoding transport, is used here (see World.hs).
    --
    -- The sweep: quantified over every piece set, this law meets draws
    -- whose scene carries no resources at all. That is not a failure of
    -- byte-identity and it is certainly not a demonstration of it --
    -- it is a precondition this iteration cannot meet, so the iteration
    -- SKIPS and is counted (World.StepOutcome / Precondition).
  , mkSkippableStep Then (lit "fetching " *> (capUntil @BindName "'s first resource twice yields identical bytes")) $
      \(BindName a) w ->
        case firstResourceId a w of
          Broken e -> pure (StepFailed e)
          Unmet why -> pure (StepSkipped why)
          Met rid -> do
            r1 <- transportRaw w (baseUrl w <> "/api/resource?id=" <> rid)
            r2 <- transportRaw w (baseUrl w <> "/api/resource?id=" <> rid)
            pure $ case (r1, r2) of
              (Right b1, Right b2) | b1 == b2 -> StepOk w
              (Right _, Right _) -> StepFailed ("id " <> rid <> " served two different payloads")
              (Left e, _) -> StepFailed e
              (_, Left e) -> StepFailed e
    -- Task 11 (@target): a batch is exactly its singles, byte for byte.
    -- Same precondition story as above, one resource further along: a
    -- scene with fewer than two resources cannot exercise a law about
    -- batching two of them.
  , mkSkippableStep Then (lit "fetching " *> (capUntil @BindName "'s first two resources as a batch equals fetching them singly")) $
      \(BindName a) w ->
        case twoResourceIds a w of
          Broken e -> pure (StepFailed e)
          Unmet why -> pure (StepSkipped why)
          Met (i1, i2) -> do
            batch <- transportRaw w (baseUrl w <> "/api/resources?ids=" <> i1 <> "," <> i2)
            s1 <- transportRaw w (baseUrl w <> "/api/resource?id=" <> i1)
            s2 <- transportRaw w (baseUrl w <> "/api/resource?id=" <> i2)
            pure $ case (batch, s1, s2) of
              (Right bb, Right b1, Right b2)
                | bb == b1 <> b2 -> StepOk w
                | otherwise -> StepFailed "batch bytes differ from concatenated singles"
              (Left e, _, _) -> StepFailed e
              (_, Left e, _) -> StepFailed e
              (_, _, Left e) -> StepFailed e
    -- Task 12: the CDC step -- see `project`/`projections` above.
  , mkStep Then (lit "the consumed projection " *> ((,) <$> capUntil @ProjName " equals fixture "
                                                        <*> capRest @FixtureRef)) $
      \(ProjName pn, FixtureRef f) w ->
        case Map.lookup "_last" (bound w) of
          Nothing -> pure (Left "no response")
          Just (_, actual) -> case Map.lookup pn projections of
            Nothing -> pure (Left ("unknown projection " <> pn))
            -- Phase S review, cheap fix 3: `project` now reports a
            -- SHAPE mismatch (e.g. the tree expects an object and the
            -- provider sent an array) as its own named error, distinct
            -- from an ordinary value mismatch against the fixture --
            -- exactly the provider-drift case this CDC suite exists to
            -- catch, and a strictly worse message when collapsed into
            -- "differs from fixture".
            Just p -> case project p actual of
              Left e -> pure (Left ("consumed projection " <> pn <> ": " <> e))
              Right got -> do
                let path = fixtureDir w </> T.unpack f <> ".json"
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
                        | otherwise -> Left ("consumed projection " <> pn
                                             <> " differs from fixture " <> f <> ": "
                                             <> maybe "(no leaf difference found)" id
                                                  (firstDiff expected got))
  ]
  where
    -- The step phase promoted this to `boundScene` (top level): twenty
    -- more definitions need it, and a `where`-bound copy is not
    -- reachable from any of them.
    scene = boundScene

-- ============ THE CAMERA, AS ARITHMETIC ============
--
-- camera.feature's preamble DEFINES visibility rather than gesturing at
-- it, and these are the definitions, one function each, pure, exported,
-- and unit-tested at their boundaries without a server. The whole point
-- of putting them here rather than inside the steps is that a predicate
-- nobody can call alone is a predicate nobody can falsify alone: each of
-- these has a discriminating case in the test suite (a pair of inputs
-- that a mutation of the predicate would answer differently), which is
-- what "mutation evidence" means in this project.
--
-- Every constant below is the SERVER'S, transcribed with its source, not
-- a number chosen to make anything pass.

-- A point on the unit sphere. The manifest publishes bounds centers,
-- label anchors and marker positions as 3-element unit vectors, so this
-- is the wire's own representation, not a re-encoding of it.
data Vec3 = Vec3 !Double !Double !Double deriving (Eq, Show)

-- A spherical cap: everything within `capRadius` radians of
-- `capCenter`. Both the view and every resource's `bounds` are one of
-- these, which is exactly why the visibility predicates are so short --
-- the wire already speaks in caps.
data Cap = Cap { capCenter :: Vec3, capRadius :: Double } deriving (Eq, Show)

-- crates/map-types/src/geom.rs:52-55 (`UnitVec::from_lat_lon_deg`),
-- with the latitude clamp `build_query` applies before calling it
-- (lib.rs:645). Both halves matter: a request at 89.95 and one at 89.9
-- are the same camera to this server, so a predicate that used the
-- unclamped latitude would disagree with the server about where the
-- camera IS.
unitOf :: Center -> Vec3
unitOf c = Vec3 (cos la * cos lo) (cos la * sin lo) (sin la)
  where
    la = radiansOf (latClamp (centerLat c))
    lo = radiansOf (centerLon c)

radiansOf :: Double -> Double
radiansOf d = d * pi / 180

-- The angle between two unit vectors, in radians. `acos` of a dot
-- product that rounding has pushed a hair outside [-1, 1] is NaN, and a
-- NaN silently makes every comparison below False -- i.e. it would make
-- "is this feature out of view?" answer no for a feature exactly on the
-- boundary. Clamped, so the degenerate case is a real angle (0 or pi)
-- rather than a value that quietly disables the law.
angleBetween :: Vec3 -> Vec3 -> Double
angleBetween (Vec3 ax ay az) (Vec3 bx by bz) =
  acos (max (-1) (min 1 (ax * bx + ay * by + az * bz)))

-- THE MARGIN. `build_query` (lib.rs:646-650):
--
--     radius = min(pi, radians(clamp(zoom, 0.05, 90) * 1.8))
--
-- The 1.8 is the server's own declared margin, and the characterization
-- pinned it empirically to better than 1%: a point at 1.78x the nominal
-- zoom is inside, at 1.82x it is outside. Conflating this cap radius
-- with the query's nominal `zoom` -- they differ by 80% -- is
-- characterization K3's named trap.
viewCap :: Center -> Zoom -> Cap
viewCap c (Zoom z) = Cap (unitOf c) (min pi (radiansOf (zoomClamp z * 1.8)))

-- The whole-sphere sentinel: a cap that covers the globe. It intersects
-- every view cap, is disjoint from none, and lies beyond no horizon --
-- so it can never be a violation of any of the three laws below, and it
-- can never be the HIT that makes one of them look satisfied either.
-- Named rather than left implicit (MEMORY: verify-distinct-not-nonnull
-- -- exclude the whole-sphere sentinel from hit logic): the counting
-- guards below use it to refuse a vacuous pass.
coversSphere :: Cap -> Bool
coversSphere cap = capRadius cap >= pi

-- IN VIEW: the feature's own bounding cap INTERSECTS the view cap. Two
-- caps intersect exactly when the angle between their centers is no
-- more than the sum of their radii -- the whole content of the
-- predicate, and the reason the margin above has to be right.
inView :: Cap -> Cap -> Bool
inView view f = angleBetween (capCenter view) (capCenter f) <= capRadius view + capRadius f

-- OUT OF VIEW: the two caps are disjoint. Stated as the negation, in one
-- place, so the two can never drift into overlapping or leaving a gap --
-- a feature is in view or out of view, never both and never neither,
-- which is what makes camera.feature's "keeps ... and omits ..." a
-- partition rather than two independent claims.
outOfView :: Cap -> Cap -> Bool
outOfView view f = not (inView view f)

-- BEYOND THE HORIZON: the feature's bounds lie ENTIRELY more than a
-- quarter turn from the center -- i.e. even its nearest point is over
-- the edge of the visible hemisphere. Note this is a property of the
-- CENTER alone: no zoom appears, because the horizon of a viewpoint on a
-- sphere does not move when you change how much of it you frame.
beyondHorizon :: Vec3 -> Cap -> Bool
beyondHorizon eye f = angleBetween eye (capCenter f) - capRadius f > pi / 2

-- A published point (a label's anchor, a marker's position) is in view
-- when it lies inside the view cap. The degenerate cap of radius zero,
-- so it is the same predicate as `inView`, not a second one.
pointInView :: Cap -> Vec3 -> Bool
pointInView view p = inView view (Cap p 0)

-- ---------- reading the manifest ----------

vec3Of :: Value -> Either Text Vec3
vec3Of (Array v) = case V.toList v of
  [a, b, c] -> Vec3 <$> num a <*> num b <*> num c
  other     -> Left ("a unit vector needs 3 components, found "
                     <> tshow (length other))
  where
    num (Number n) = Right (realToFrac n)
    num other      = Left ("a unit vector component is not a number: " <> bounded other)
vec3Of other = Left ("not a unit vector: " <> bounded other)

capOf :: Value -> Either Text Cap
capOf v = do
  b <- field "bounds" v
  c <- vec3Of =<< field "center" b
  r <- field "radius" b
  case r of
    Number n -> Right (Cap c (realToFrac n))
    other    -> Left ("bounds radius is not a number: " <> bounded other)

arrayOf :: Text -> Value -> Either Text [Value]
arrayOf k v = case field k v of
  Right (Array a) -> Right (V.toList a)
  Right other     -> Left ("field " <> k <> " is not an array: " <> bounded other)
  Left e          -> Left e

textField :: Text -> Value -> Either Text Text
textField k v = case field k v of
  Right (String s) -> Right s
  Right other      -> Left ("field " <> k <> " is not a string: " <> bounded other)
  Left e           -> Left e

intField :: Text -> Value -> Either Text Int
intField k v = case field k v of
  Right (Number n) -> Right (round n)
  Right other      -> Left ("field " <> k <> " is not a number: " <> bounded other)
  Left e           -> Left e

-- Every resource record of a manifest, by id -- the WHOLE record, not a
-- projection of it: `every resource {a} and {b} share is byte-identical
-- in both` is a whole-body claim about the record, and comparing a
-- hand-picked triple of fields would be the existential poke this
-- project's law forbids.
resourceRecords :: Value -> Either Text (Map.Map Text Value)
resourceRecords v = do
  rs <- arrayOf "resources" v
  Map.fromList <$> traverse (\r -> (,) <$> textField "id" r <*> pure r) rs

-- The distinct feature ids of a manifest.
featureIdSet :: Value -> Either Text (Set Text)
featureIdSet v = Set.fromList <$> (traverse (textField "feature") =<< arrayOf "features" v)

-- Each feature id paired with the bounding cap of the geometry it
-- references. A feature names a `resource`; the resource carries the
-- `bounds`. A feature whose resource is not in the manifest is a broken
-- manifest, not a feature to skip quietly.
featureCaps :: Value -> Either Text [(Text, Cap)]
featureCaps v = do
  byId <- resourceRecords v
  fs <- arrayOf "features" v
  traverse (one byId) fs
  where
    one byId f = do
      fid <- textField "feature" f
      rid <- textField "resource" f
      case Map.lookup rid byId of
        Nothing -> Left ("feature " <> fid <> " references resource " <> rid
                         <> ", which the manifest does not publish")
        Just r  -> (,) fid <$> capOf r

-- Labels by their SUBJECT (the feature they name) and markers by their
-- PLACE, each with the point it is drawn at. Subject and place are the
-- ids the characterization's nesting laws are stated over (C1/C2); the
-- point is what the anchor law needs.
labelAnchors :: Value -> Either Text [(Text, Vec3)]
labelAnchors v = traverse one =<< arrayOf "labels" v
  where one l = (,) <$> textField "subject" l <*> (vec3Of =<< field "anchor" l)

markerPoints :: Value -> Either Text [(Text, Vec3)]
markerPoints v = traverse one =<< arrayOf "markers" v
  where one m = (,) <$> textField "place" m <*> (vec3Of =<< field "at" m)

-- The two kinds together, which is how camera.feature states both
-- nesting laws ("markers and labels"). Ids are namespaced by kind so a
-- marker place and a label subject that happen to share a hex id are two
-- different things, as they are on the wire.
cameraCulled :: Value -> Either Text [(Text, Vec3)]
cameraCulled v = do
  ms <- markerPoints v
  ls <- labelAnchors v
  pure ([ ("marker:" <> i, p) | (i, p) <- ms ] ++ [ ("label:" <> i, p) | (i, p) <- ls ])

-- ---------- reading a transition plan ----------

planSteps :: Value -> Either Text [Value]
planSteps = arrayOf "steps"

stepsOfKind :: Text -> [Value] -> [Value]
stepsOfKind k = filter (\s -> textField "kind" s == Right k)

-- The region ids a plan's fades name, by kind. `fade_in`/`fade_out`
-- publish a bare 16-hex `region`; scene manifests publish the same thing
-- as the feature id `region:HEX`, and `/api/changes` as the subject
-- `region:HEX` -- so one of the three has to be translated to compare
-- them, and it is done here, once, rather than at each of the three call
-- sites.
fadeRegions :: Text -> [Value] -> Either Text (Set Text)
fadeRegions kind sts = Set.fromList <$> traverse (textField "region") (stepsOfKind kind sts)

-- `/api/changes` is a flat array of change rows, each with a `kind` and
-- a namespaced `subject`. The subjects of one kind, with the namespace
-- stripped, are directly comparable with `fadeRegions` above.
changeSubjects :: Text -> Text -> Value -> Either Text (Set Text)
changeSubjects kind ns v = case v of
  Array rows -> Set.fromList . concat <$> traverse one (V.toList rows)
  other      -> Left ("the changes timeline is not an array: " <> bounded other)
  where
    one r = do
      k <- textField "kind" r
      s <- textField "subject" r
      pure [ T.drop (T.length ns) s | k == kind, ns `T.isPrefixOf` s ]

-- THE STRUCTURAL MIRROR of a plan (characterization T5): reverse the
-- step order, swap fade_in with fade_out, and swap each morph's `from`
-- with its `to`. Any other field of any step is carried through
-- untouched, so the comparison the inversion law makes is a WHOLE-BODY
-- one -- a step kind this canon never fires today (split, merge) would
-- be mirrored as itself and would have to match exactly, rather than
-- being silently dropped from the comparison.
mirrorPlan :: [Value] -> [Value]
mirrorPlan = reverse . map flipStep
  where
    flipStep s = case textField "kind" s of
      Right "fade_in"  -> setField "kind" (String "fade_out") s
      Right "fade_out" -> setField "kind" (String "fade_in") s
      Right "morph"    -> case (field "from" s, field "to" s) of
        (Right f, Right t) -> setField "from" t (setField "to" f s)
        _                  -> s
      _ -> s

-- ---------- what the new steps are built from ----------

-- The bound response under a name. Promoted out of `allSteps`'s own
-- `where` clause (where it was called `scene`) because the step phase's
-- twenty-odd new definitions all need it and a second copy inside each
-- would be the same function written twenty-odd times.
boundScene :: Text -> World -> Either Text Value
boundScene n w = maybe (Left ("unbound " <> n)) (Right . snd) (Map.lookup n (bound w))

-- The view cap a bound scene was rendered at. A scene rendered with no
-- camera has none, and that is an ERROR rather than "the whole globe":
-- a law stated about `viewed`'s view cannot be checked against a scene
-- that has no view, and answering "it holds" there would be the exact
-- shape of check this project forbids.
cameraOf :: Text -> World -> Either Text Cap
cameraOf n w = case Map.lookup n (cameras w) of
  Just (c, z) -> Right (viewCap c z)
  Nothing -> Left (n <> " was not rendered with a camera (no center and zoom \
                   \recorded for it), so it has no view for this law to be about")

-- ONE render action, for all six of the step phase's render shapes. The
-- differences between those shapes are entirely in WHICH parts are
-- present (`sceneUrlFull`'s four Maybes), so they are parameters here
-- rather than six near-identical bodies.
--
-- Records the same two things every render records: the (year, style)
-- the combine and fold steps re-render at -- absent when the scenario
-- said "in no style", because there is then no style to re-render at
-- and inventing one would be the hardcoded dress `World.lastRender`
-- exists to prevent -- and, new here, the CAMERA under the bound name.
renderInto :: Maybe Text -> PieceSet -> Year -> Maybe StyleName
           -> Maybe CamSpec -> Maybe DetailTier -> World -> IO (Either Text World)
renderInto mname ps y mst mcam mdet w = do
  r <- getUrl (sceneUrlFull (baseUrl w) ps y mst mcam mdet) w
  pure $ do
    w0 <- r
    w1 <- maybe (Right w0) (`bindLast` w0) mname
    Right w1 { lastRender = (,) y <$> mst
             , cameras = case (mname, mcam) of
                 (Just n, Just (CamSpec c z)) -> Map.insert n (c, z) (cameras w1)
                 _ -> cameras w1 }

-- A handful of ids in a failure message, the same way `describeSetDiff`
-- bounds its own: five is enough to recognize a pattern, and a scene
-- carries thousands.
listSome :: [Text] -> Text
listSome xs = T.intercalate ", " (take 5 xs) <> (if length xs > 5 then ", ..." else "")

-- Several of the transition laws name the same scene twice ("... is in
-- after and not before, and every fade-out region is in before and not
-- after"). Two DIFFERENT names there would be a sentence that no longer
-- states the law, so it is refused with the reason rather than quietly
-- compared against whichever one came last.
sameTwice :: Text -> Text -> Text -> Either Text ()
sameTwice what a b
  | a == b = Right ()
  | otherwise = Left ("this law names one " <> what <> " twice, but was given two: "
                      <> a <> " and " <> b)

-- The geometry each FEATURE carries, in vertices, summed over every
-- resource it references.
--
-- WHAT "SHARED" MEANS HERE, and why it is the feature and not the
-- resource id. Geometry is content-addressed: a resource id IS its
-- bytes. So two scenes rendered at different detail share a resource id
-- exactly when that geometry did not change between them -- and then its
-- vertex count is necessarily EQUAL on both sides. Read literally over
-- resource ids, "every shared resource has at least as many vertices in
-- fine as in coarse" is therefore true by construction of the hash: it
-- cannot fail, for any server, ever. Measured on the live canon at
-- -1405: 470 shared ids between the coarse and fine tiers, 852 between
-- fine and ultra, and ZERO violations possible among them. A check
-- satisfiable by its own failure mode is no check (MEMORY:
-- verify-distinct-not-nonnull), and a vacuous GREEN on a @target is
-- worse than a red -- it retires a law nobody has run.
--
-- The sentence also refutes that reading on its own terms: "no shared
-- resource of glance carries more vertices than IT DOES IN corner"
-- presupposes that the same thing can carry two different counts in the
-- two scenes. Under content addressing nothing can. Under the feature
-- reading everything does, which is what the sentence is about.
--
-- And it is the reading detail.feature's own comments declare. They cite
-- "498/917 violations on the 1.5e-3 -> 1e-2 rung" and "490/917 features
-- carry MORE vertices at zoom 0.05 than at zoom 90": 917 is the count of
-- distinct FEATURE ids in this canon (characterization 1.1/2.1), not of
-- resources, of which there are three to eleven thousand. The corpus
-- author measured features; this measures features.
--
-- Flagged in the step-phase report as an interpretation for the owner to
-- ratify, since it reads "resource" in the feature's sentence as the
-- geometry a shared feature carries. No feature text was changed.
featureVertices :: Value -> Either Text (Map.Map Text Int)
featureVertices v = do
  byId <- resourceRecords v
  fs <- arrayOf "features" v
  rows <- traverse (one byId) fs
  pure (Map.fromListWith (+) rows)
  where
    one byId f = do
      fid <- textField "feature" f
      rid <- textField "resource" f
      case Map.lookup rid byId of
        Nothing -> Left ("feature " <> fid <> " references resource " <> rid
                         <> ", which the manifest does not publish")
        Just r  -> (,) fid <$> intField "vertices" r

-- One rung of a vertex-count comparison: how many features the two
-- scenes share, and which of them the finer side draws with FEWER
-- vertices than the coarser -- the violations, with both counts, so the
-- message can say `region:x 42<108` rather than "something is smaller".
data Rung = Rung { rungShared :: Int, rungBad :: [(Text, Int, Int)] } deriving (Eq, Show)

vertexRung :: Text -> Text -> World -> Either Text Rung
vertexRung finer coarser w = do
  vf <- featureVertices =<< boundScene finer w
  vc <- featureVertices =<< boundScene coarser w
  let shared = Map.toList (Map.intersectionWith (,) vf vc)
  pure (Rung (length shared) [ (i, a, b) | (i, (a, b)) <- shared, a < b ])

describeRung :: Text -> Text -> Rung -> Text
describeRung finer coarser (Rung shared bad)
  | null bad = tshow shared <> " shared feature(s) between " <> finer <> " and " <> coarser
               <> ": none loses vertices"
  | otherwise = tshow (length bad) <> " of " <> tshow shared <> " shared feature(s) carry "
                <> "fewer vertices in " <> finer <> " than in " <> coarser <> ": "
                <> T.intercalate ", " [ i <> " " <> tshow a <> "<" <> tshow b
                                      | (i, a, b) <- take 5 bad ]

-- One morph step, against the border it claims to move: the boundary's
-- id, how many points the morph carries, and how many vertices that
-- boundary's geometry carries in the endpoint scene. `from` and `to`
-- are always resampled to the same length (characterization T10), so
-- the smaller of the two is the honest number to hold the law to.
morphPoints :: Map.Map Text Value -> [Value] -> Value -> Either Text (Text, Int, Int)
morphPoints byId fs s = do
  b <- textField "boundary" s
  from <- arrayOf "from" s
  to <- arrayOf "to" s
  rids <- sequence [ textField "resource" f
                   | f <- fs, textField "feature" f == Right ("boundary:" <> b) ]
  case rids of
    [] -> Left ("the plan morphs boundary " <> b
                <> ", which the endpoint scene does not publish as a feature")
    _ -> do
      vs <- traverse (\i -> maybe (Left ("no resource " <> i)) (intField "vertices")
                              (Map.lookup i byId)) rids
      pure (b, min (length from) (length to), sum vs)

-- The known-absent resource id the batch-refusal law probes with.
--
-- A resource id is the 64-bit content hash of a non-empty geometry
-- payload, rendered as 16 hex digits; an all-zero hash is not a value
-- this canon's hasher produces for any geometry it holds. That is an
-- argument, not a guarantee, so the step does not rest on it: it checks
-- that the scene under test does not in fact publish this id, and skips
-- (naming the reason) rather than passing if it ever does.
bogusResourceId :: Text
bogusResourceId = "0000000000000000"

-- ---------- Task 11: content-addressed resource ids ----------
-- Every id in a bound scene's "resources" array, text order, extracted
-- as plain Text (not Value, unlike the local `resourceIds` in
-- `allSteps`'s own `where` above, which the subset step uses for a
-- structural comparison) -- these three are used to build URLs
-- (/api/resource?id=..., /api/resources?ids=...), which need the raw
-- string.
-- The ids of a scene VALUE. Split out from `resourceIdsOf` (which looks
-- one up by bound name) because the singleton-fold step renders scenes
-- it never binds -- ten single-piece renders per iteration would
-- otherwise need ten throwaway names in the world.
resourceIdsIn :: Value -> Either Text [Text]
resourceIdsIn v = case field "resources" v of
  Right (Array rs) -> traverse idOf (V.toList rs)
  other -> Left ("no resources array: " <> tshow (() <$ other))
  where
    idOf r = case field "id" r of
      Right (String s) -> Right s
      Right other       -> Left ("resource id is not a string: " <> tshow other)
      Left e            -> Left e

resourceIdsOf :: Text -> World -> Either Text [Text]
resourceIdsOf n w = do
  (_, v) <- maybe (Left ("unbound " <> n)) Right (Map.lookup n (bound w))
  resourceIdsIn v

resourceSet :: Text -> World -> Either Text (Set Text)
resourceSet n w = Set.fromList <$> resourceIdsOf n w

-- Render ONE piece on its own, at the year and style the whole scene was
-- rendered at, and report just its resource-id set. Deliberately uses
-- `transport` rather than `getUrl`: this render is an intermediate value
-- in a fold, not an answer any later step should be able to see, so it
-- must not overwrite the world's "_last" binding.
renderPieceAlone :: World -> Year -> StyleName -> Piece -> IO (Either Text (Set Text))
renderPieceAlone w y st p = do
  r <- transport w (sceneUrl (baseUrl w) (PieceSet (Set.singleton p)) y st)
  pure $ do
    (_, v) <- r
    Set.fromList <$> resourceIdsIn v

-- Task 11's two byte-identity laws need at least one / at least two
-- resources to fetch. Under the sweep they are quantified over every
-- piece set, and a perfectly legal draw can produce a scene carrying
-- fewer resources than the law needs. That is a PRECONDITION MISS, not a
-- failure -- and not a pass either. `Precondition` (World.hs) keeps it
-- distinct from the two things that genuinely ARE failures: an unbound
-- scene name, and a body with no resources array. Two specialized
-- accessors rather than one `atLeast k`, so each caller matches a shape
-- that is exactly what it needs and carries no unreachable "not enough
-- after all" branch.
firstResourceId :: Text -> World -> Precondition Text
firstResourceId n w = case resourceIdsOf n w of
  Left e        -> Broken e
  Right (i : _) -> Met i
  Right ids     -> Unmet (tooFew n (length ids) 1)

twoResourceIds :: Text -> World -> Precondition (Text, Text)
twoResourceIds n w = case resourceIdsOf n w of
  Left e              -> Broken e
  Right (i1 : i2 : _) -> Met (i1, i2)
  Right ids           -> Unmet (tooFew n (length ids) 2)

tooFew :: Text -> Int -> Int -> Text
tooFew n have want =
  n <> " carries " <> tshow have <> " resource(s) at this draw; this law needs at least "
    <> tshow want

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
-- (if any) its declared output key should hold: `Right Nothing` omits
-- the key entirely (used for a field that is genuinely absent, or
-- present as JSON `null` -- vendor.rs's own `Option`-typed fields,
-- color_key and when, are read with `.and_then`, which treats a JSON
-- `null` exactly the same as a missing key, so the projection does
-- too), and `Left` reports a SHAPE mismatch found while projecting that
-- field's own value (see `project`'s `Fields`/`Each` fallthroughs
-- below) -- distinct from the field being merely absent.
type Extractor = Value -> Either Text (Maybe Value)

data Proj = Keep | Fields [(Text, Extractor)] | Each Proj

-- Phase S review, cheap fix 3: a shape mismatch (the tree expects an
-- object and the provider sent an array, or vice versa) used to fall
-- through to `v` UNCHANGED, so it surfaced downstream only as an
-- opaque "consumed projection X differs from fixture Y" -- a strictly
-- worse message than naming the actual problem, on exactly the
-- provider-drift case this CDC suite exists to catch. `project` now
-- reports its own shape mismatches as `Left`, distinct from an ordinary
-- value mismatch against the fixture (see the consumed-projection step
-- in `allSteps`, which prefixes this error with the projection's name).
project :: Proj -> Value -> Either Text Value
project Keep v = Right v
project (Each p) (Array a) = Array <$> traverse (project p) a
project (Each _) v = Left ("expected an array, got " <> shapeName v)
project (Fields fs) v@(Object _) = do
  pairs <- traverse (\(k, ext) -> fmap (fmap (\pv -> (K.fromText k, pv))) (ext v)) fs
  Right (Object (KM.fromList [ p | Just p <- pairs ]))
project (Fields _) v = Left ("expected an object, got " <> shapeName v)

shapeName :: Value -> Text
shapeName (Object _) = "an object"
shapeName (Array _)  = "an array"
shapeName (String _) = "a string"
shapeName (Number _) = "a number"
shapeName (Bool _)   = "a boolean"
shapeName Null       = "null"

-- The ordinary case: keep the field named `k`, projecting its value
-- through `p`. `Just Null` is treated the same as absent (see the
-- `Extractor` note above).
field1 :: Text -> Proj -> (Text, Extractor)
field1 k = fieldAlt k [k]

-- The general case field1 specializes: the OUTPUT key `outKey` is
-- sourced from the FIRST of `srcKeys` that is present (and non-null) on
-- the enclosing object -- vendor.rs's `label` (from `title`, falling
-- back to `label`) is `fieldAlt "label" ["title", "label"] Keep`.
--
-- Narrower than the Rust it mirrors, noted rather than silently
-- diverged from: vendor.rs's own fallback
-- (`str_field(&v, &ctx, "title").or_else(|_| str_field(&v, &ctx,
-- "label"))`) also falls back to `label` when `title` is PRESENT but is
-- not a string (str_field's own type check fails, tripping `or_else`);
-- this only falls back on `title` being absent-or-null. A `title` field
-- present with the wrong JSON type is not exercised by any known
-- payload and is not covered by any fixture today.
fieldAlt :: Text -> [Text] -> Proj -> (Text, Extractor)
fieldAlt outKey srcKeys p = (outKey, \v -> case v of
  Object o -> case asum [ nonNull (KM.lookup (K.fromText k) o) | k <- srcKeys ] of
    Nothing -> Right Nothing
    Just fv -> fmap Just (project p fv)
  _        -> Right Nothing)
  where
    nonNull (Just Null) = Nothing
    nonNull other        = other

-- A field gathered by walking a path of array-valued keys and
-- concatenating every leaf array found at the end of it -- vendor.rs's
-- `verses` (not a top-level field; flattened out of
-- witnesses[].verse_groups[].verses) is
-- `flattenField "verses" ["witnesses", "verse_groups", "verses"]`.
flattenField :: Text -> [Text] -> (Text, Extractor)
flattenField outKey path = (outKey, \v -> Right (Just (Array (V.fromList (flattenGather path v)))))

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
