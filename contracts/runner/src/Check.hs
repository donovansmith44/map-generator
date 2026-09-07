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

-- | Every violation across a feature: (scenario name, step body, kind).
violations :: [StepDef] -> Feature -> [(Text, Text, Violation)]
violations defs f =
  [ (scName sc, stepBody st, v)
  | sc <- ftScenarios f
  , st <- scSteps sc
  , Just v <- [classify (scTags sc) defs st] ]

-- | Steps matching zero definitions (and claimed by none either):
-- (scenario, step body).
orphans :: [StepDef] -> Feature -> [(Text, Text)]
orphans defs f = [ (sc, b) | (sc, b, VOrphan) <- violations defs f ]

-- | Steps that truly MATCH two or more definitions: (scenario, step body,
-- the competing definitions' human-readable sketches).
ambiguous :: [StepDef] -> Feature -> [(Text, Text, [Text])]
ambiguous defs f = [ (sc, b, ss) | (sc, b, VAmbiguous ss) <- violations defs f ]

-- | Steps whose shape is recognized but whose value fails to parse, with
-- no other definition matching to fall back on: (scenario, step body,
-- [(competing definition's sketch, its parse error)]).
valueErrors :: [StepDef] -> Feature -> [(Text, Text, [(Text, Text)])]
valueErrors defs f = [ (sc, b, es) | (sc, b, VValueError es) <- violations defs f ]

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

checkDir :: [StepDef] -> FilePath -> IO ()
checkDir defs dir = do
  files <- featureFilesLocal dir
  bad <- fmap concat . mapM (\p -> do
    -- Fix 4: shared explicit-UTF-8 reader, not a plain TIO.readFile --
    -- see World.readFeatureFile's comment.
    src <- readFeatureFile p
    pure $ case parseFeature p src of
      Left e  -> [(T.pack p, "PARSE", e)]
      Right f -> [ (T.pack p <> " / " <> s, label v, describe b v)
                 | (s, b, v) <- violations defs f ]) $ files
  if null bad then TIO.putStrLn "totality: every step has exactly one definition"
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
