module Check where

import Data.Text (Text)
import qualified Data.Text as T
import qualified Data.Text.IO as TIO
import qualified Data.Map.Strict as Map
import Gherkin.Ast
import Gherkin.Parse (parseFeature)
import qualified Prop
import System.Directory (doesDirectoryExist, listDirectory)
import System.Exit (exitFailure)
import System.FilePath ((</>), takeExtension)
import World (Claim (..), StepDef (..), readFeatureFile)

-- The totality law (spec §4): every step in every feature matches EXACTLY
-- ONE definition, or CI fails naming the offenders. Under R23, a
-- definition's answer for one body is one of THREE outcomes (World.Claim:
-- NoMatch / ClaimError / Matched), and "exactly one" is a law about how
-- many definitions MATCH, not how many merely claim:
--   0 definitions MATCH, 0 claim  -> an orphan (nobody defines it)
--   0 definitions MATCH, 1+ claim -> a value error (recognized shape,
--                                    bad value — a THIRD class, distinct
--                                    from both orphan and ambiguous; see
--                                    `classify` below)
--   1 definition MATCHES          -> fine, the law holds (any number of
--                                    other definitions merely claiming it
--                                    does not matter — a full match wins)
--   2+ definitions MATCH          -> ambiguous (more than one truly
--                                    claims to run this line)
-- `classify` is the one place a step becomes one of these outcomes;
-- `orphans`, `ambiguous`, and `valueErrors` are just filters over it, and
-- `checkDir` reports all three from a single traversal.

-- | How one step fares under the totality law, or Nothing if it holds
-- (exactly one definition matches it, regardless of how many others
-- merely claim it).
data Violation
  = VOrphan
  | VAmbiguous [Text]
    -- ^ the sketches of every definition that fully MATCHED.
  | VValueError [(Text, Text)]
    -- ^ (sketch, parse error) for every definition that claimed the shape
    -- but whose capture didn't parse — and nothing else matched.
  deriving (Eq, Show)

-- `body`'s holes get substituted with a fixed example value before
-- matching (see Prop.substituteExamples) -- but ONLY when `isProperty`
-- says the enclosing scenario is @property-tagged, because that is
-- exactly the condition under which `Prop.runWithProperties` will ever
-- actually substitute anything at run time (see its `run1`: an untagged
-- scenario runs through plain `runScenario`, which never substitutes).
-- Getting this gate wrong in either direction breaks the law:
--   * substitute unconditionally (the pre-fix behavior) and an untagged
--     scenario that accidentally contains a hole is reported CLEAN by
--     `check` while running, for real, on the literal unresolved
--     "<hole>" text -- a static verdict that lies about the dynamic one.
--   * never substitute and every @property scenario's holes reach their
--     captures as raw "<hole>" text, which is only caught when the
--     capture in question happens to validate its input -- exactly the
--     "artifact of which captures happen to validate" trap this whole
--     mechanism exists to close (concretely: "<someYear>" contains no
--     whitespace, so UrlPath accepts it unexamined; once a "GET {url} as
--     {name}" step exists, that would silently fetch a URL containing
--     the literal text "<someYear>", never a real year).
--
-- An UNREGISTERED hole is a stronger, tag-independent law: scanning the
-- RAW body (before any substitution) for a "<hole>" whose name is not in
-- Prop.holeRegistry and forcing VOrphan on it, regardless of what any
-- definition's capture would have done with the literal text, is what
-- makes "never silently pass" actually true -- a hole is either bound to
-- a real generator or the step is unconditionally undefined, the same
-- as any other never-implemented step.
--
-- Review finding (round 1, Important): the gate itself is no longer
-- decided here. It was duplicated -- `Vocab.expectedVocab` re-expressed
-- the identical rule in its own words -- so it now lives in ONE exported
-- function, `Prop.deholeFor`, called by both. This comment keeps the
-- REASONING (it is the fuller of the two, and it belongs with the
-- totality law it was written for); `Prop.deholeFor` holds the decision.
-- The parameter is the enclosing scenario's tags rather than a
-- pre-computed Bool, so the shared function -- not each caller -- is what
-- turns tags into a substitution.
classify :: [Tag] -> [StepDef] -> Step -> Maybe Violation
classify tags defs (Step k body _)
  | not (null unregistered) = Just VOrphan
  | otherwise = case (matched, errored) of
      ([], [])  -> Just VOrphan
      ([], _)   -> Just (VValueError errored)
      ([_], _)  -> Nothing                    -- one real match wins outright
      (ms, _)   -> Just (VAmbiguous ms)
  where
    unregistered = [ h | h <- Prop.holesIn body, h `Map.notMember` Prop.holeRegistry ]
    results = [ (defSketch d, defRun d (Prop.deholeFor tags body)) | d <- defs, defKw d == k ]
    matched = [ sk | (sk, Matched _) <- results ]
    errored = [ (sk, e) | (sk, ClaimError e) <- results ]

-- | Where in a feature a step lives. A background belongs to the FEATURE
-- rather than to any scenario, so it has no scenario name to give — and
-- that is a fact about the corpus, not a formatting detail, so it is a
-- type rather than a sentinel string.
data StepSite = InBackground | InScenario Text deriving (Eq, Show)

-- | How a site is named in a report line.
siteName :: StepSite -> Text
siteName InBackground     = "Background"
siteName (InScenario n)   = n

-- | Every violation across a feature: (where the step lives, step body,
-- kind). Background rows come first — a step that fails there fails for
-- every scenario in the file, so it is the first thing a reader should
-- see.
--
-- ONE traversal and ONE rule (fix round 1, finding 4). The background
-- rows used to be produced only inside `checkDir`, which left
-- `orphans`/`ambiguous`/`valueErrors` — and the dozen tests that use
-- them as the totality law's proxy — unable to see a background at all.
-- That is the totality law stated twice in two places, which is the
-- shape this module's own comment refuses elsewhere. `checkDir` now
-- decides only how to NAME a row, never which rows exist.
violations :: [StepDef] -> Feature -> [(StepSite, Text, Violation)]
violations defs f =
     [ (InBackground, stepBody st, v)
     | st <- ftBackground f
     , Just v <- [backgroundViolation defs f st] ]
  ++ [ (InScenario (scName sc), stepBody st, v)
     | sc <- ftScenarios f
     , st <- scSteps sc
     , Just v <- [classify (scTags sc) defs st] ]

-- | R97 requirement 2: the totality law counts BACKGROUND steps too, and
-- a background step that no definition matches is reported rather than
-- silently skipped.
--
-- Reported ONCE, named by the feature, because that is what it is. (This
-- is also why `Check` does not use `runnableScenarios` for the totality
-- law, unlike `Run`, `Prop` and `Vocab` — under that expansion one
-- undefined background step would be reported once per scenario, N
-- copies of a single defect under N names.)
--
-- The TAGS a background step is classified under are the honest
-- difficulty here. A background runs before every scenario, and
-- `deholeFor` substitutes a hole only for a @property scenario — so the
-- same background step can be a clean match in one scenario and a bad
-- value in the next, and there is no single answer. So it is checked
-- under EVERY scenario's tags and reports the FIRST treatment that
-- fails: a background is only sound if it is sound for every scenario it
-- runs in. That rule is what turns a background hole in a feature with a
-- non-@property scenario into a check-time BAD-VALUE instead of a
-- run-time capture error.
backgroundViolation :: [StepDef] -> Feature -> Step -> Maybe Violation
backgroundViolation defs f st =
  case [ v | tags <- treatments, Just v <- [classify tags defs st] ] of
    (v : _) -> Just v
    []      -> Nothing
  where
    -- Fix round 1, finding 1: a feature with a background and NO
    -- scenarios used to yield an empty comprehension here, so every one
    -- of its background steps came back clean and `check` printed
    -- "every step has exactly one definition" over a file containing an
    -- undefined one. Before R97 a scenario-less feature had no steps at
    -- all, so that claim was vacuously honest; a background makes it a
    -- lie, and requirement 2 carries no "unless nothing runs it"
    -- qualifier.
    --
    -- The honest treatment for a step that nothing will ever substitute
    -- for is the UNTAGGED one: that is exactly what a non-@property
    -- scenario would give it, and it is the strictest of the two, so a
    -- hole-bearing background in a scenario-less feature is reported
    -- rather than excused.
    treatments
      | null (ftScenarios f) = [[]]
      | otherwise            = map scTags (ftScenarios f)

-- | Steps matching zero definitions (and claimed by none either):
-- (site, step body).
orphans :: [StepDef] -> Feature -> [(StepSite, Text)]
orphans defs f = [ (sc, b) | (sc, b, VOrphan) <- violations defs f ]

-- | Steps that truly MATCH two or more definitions: (site, step body,
-- the competing definitions' human-readable sketches).
ambiguous :: [StepDef] -> Feature -> [(StepSite, Text, [Text])]
ambiguous defs f = [ (sc, b, ss) | (sc, b, VAmbiguous ss) <- violations defs f ]

-- | Steps whose shape is recognized but whose value fails to parse, with
-- no other definition matching to fall back on: (site, step body,
-- [(competing definition's sketch, its parse error)]).
valueErrors :: [StepDef] -> Feature -> [(StepSite, Text, [(Text, Text)])]
valueErrors defs f = [ (sc, b, es) | (sc, b, VValueError es) <- violations defs f ]

-- | The degenerate-hole law over one feature: every (scenario name,
-- defect) it finds.
--
-- Fix round 2, blocker 3's residual: ONE traversal, in ONE place.
-- `checkDir` runs this, and so does the corpus-wide pin in the test
-- suite whose whole job is to guard it. Those two used to state the
-- traversal separately, and the moment R97 changed it here the pin kept
-- the old word — going blind to exactly the scenarios that had just
-- gained a background, which is the defect this law exists to catch. A
-- rule written twice is a rule that can drift, and this one drifted the
-- first time it moved. Now a change to the traversal moves both, and the
-- pin cannot silently disagree with the instrument.
--
-- The scenario is read WITH its background: a hole that appears only in
-- the background is still a hole that scenario quantifies over, and one
-- that never varies is the same defect wherever it was written.
holeDefectsIn :: Int -> Feature -> [(Text, Prop.HoleDefect)]
holeDefectsIn = holeDefectsInWith Prop.holeRegistry

-- | The same law over an INJECTED registry — the split
-- `Prop.holeDefectsWith`/`shrinkToMinimalWith`/`bindingsForWith` all
-- make, and for the same reason: every group in the real registry
-- behaves, so nothing in the real corpus can produce a defect, and the
-- only way to exercise this traversal against one is for a test to build
-- a deliberately degenerate group and drive the real function with it.
holeDefectsInWith
  :: Map.Map Text Prop.HoleGroup -> Int -> Feature -> [(Text, Prop.HoleDefect)]
holeDefectsInWith reg n f =
  [ (scName sc, d)
  | sc <- runnableScenarios f
  , Prop.isProperty (scTags sc)
  , d <- Prop.holeDefectsWith reg n sc ]

-- Duplicated from app/Main.hs's `featureFiles`: importing Main from the
-- library would create an import cycle (Main imports Check), and this
-- walker is 8 lines — duplication beats a dependency cycle.
featureFilesLocal :: FilePath -> IO [FilePath]
featureFilesLocal dir = do
  entries <- listDirectory dir
  fmap concat . mapM walk $ [ dir </> e | e <- entries ]
  where
    walk p = do
      isDir <- doesDirectoryExist p
      if isDir then featureFilesLocal p
      else pure [ p | takeExtension p == ".feature" ]

-- Stage 1 Task 5: the hole-distinctness law, added as a SECOND source of
-- `bad` rows alongside the totality violations above. A @property
-- scenario whose holes cannot vary, or two of whose holes are always
-- equal, has no discriminating power on the axis it claims to fuzz --
-- diagnosis §7.0's exact defect (someA/someB drawing the identical value
-- every iteration), made a `check`-time failure instead of a comment.
checkDir :: [StepDef] -> FilePath -> IO ()
checkDir defs dir = do
  files <- featureFilesLocal dir
  bad <- fmap concat . mapM (\p -> do
    -- Fix 4: shared explicit-UTF-8 reader, not a plain TIO.readFile --
    -- see World.readFeatureFile's comment.
    src <- readFeatureFile p
    pure $ case parseFeature p src of
      Left e  -> [(T.pack p, "PARSE", e)]
      Right f ->
        -- R97: `violations` already orders background rows first and
        -- says where each row lives; this only turns a site into a name.
        [ (T.pack p <> " / " <> siteName s, label v, describe b v)
        | (s, b, v) <- violations defs f ]
        ++ [ (T.pack p <> " / " <> sc, "DEGENERATE-HOLE", describeDefect d)
           | (sc, d) <- holeDefectsIn 30 f ]) $ files
  if null bad then TIO.putStrLn
    "totality: every step has exactly one definition; every property hole varies"
  else do
    mapM_ (\(loc, tag, msg) -> TIO.putStrLn (tag <> " " <> loc <> ": " <> msg)) bad
    exitFailure
  where
    label VOrphan          = "ORPHAN"
    label (VAmbiguous _)   = "AMBIGUOUS"
    label (VValueError _)  = "BAD-VALUE"
    describe b VOrphan = b
    describe b (VAmbiguous sketches) =
      b <> " matches " <> T.pack (show (length sketches)) <> " definitions: "
        <> T.intercalate " | " sketches
    describe b (VValueError errs) =
      -- Fix 6 (belt-and-braces, alongside app/Main.hs's hSetEncoding):
      -- plain ASCII here, not an em dash — this string reaches a Windows
      -- console at whatever code page it's running under, and a literal
      -- U+2014 crashed `commitAndReleaseBuffer` before either fix landed.
      b <> " -- " <> T.intercalate "; " [ sk <> ": " <> e | (sk, e) <- errs ]
    describeDefect (Prop.Constant h) =
      "<" <> h <> "> takes one value across the whole run -- a constant "
      <> "wearing a generator's clothes; this scenario fuzzes nothing on that axis"
    describeDefect (Prop.AlwaysEqual a b) =
      "<" <> a <> "> and <" <> b <> "> are bound to the same value on every "
      <> "iteration -- any law comparing them is degenerate (diagnosis 7.0)"
