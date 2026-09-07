module Check where

import Data.Text (Text)
import qualified Data.Text as T
import qualified Data.Text.IO as TIO
import Gherkin.Ast
import Gherkin.Parse (parseFeature)
import qualified Prop
import System.Directory (doesDirectoryExist, listDirectory)
import System.Exit (exitFailure)
import System.FilePath ((</>), takeExtension)
import World (Claim (..), StepDef (..))

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

classify :: [StepDef] -> Step -> Maybe Violation
classify defs (Step k body _) = case (matched, errored) of
  ([], [])  -> Just VOrphan
  ([], _)   -> Just (VValueError errored)
  ([_], _)  -> Nothing                    -- one real match wins outright
  (ms, _)   -> Just (VAmbiguous ms)
  where
    -- property holes (e.g. <someYear>) must be substituted with a
    -- registered example value before matching, or every @property step
    -- looks wrong. Task 9 wires this to Prop.substituteExamples: an
    -- unregistered hole name is deliberately left as-is here (classify
    -- has no scenario/tag context to build a good "which hole, which
    -- scenario" message from a bare Step body -- that loud, hole-naming
    -- failure lives in Prop.runScenarioProperty, which runs with that
    -- context). Fix 4 (post-Task-7 review):
    -- this was documented as making the step "an orphan" — that's not
    -- what the real corpus shows. A hole like "<somePieces>" still matches
    -- the literal shape of "I render pieces ... at year ... in style ...",
    -- so both render overloads CLAIM it; the hole text just fails to parse
    -- as a Piece. With nothing left to actually MATCH, that's a bad-value
    -- (VValueError), not an orphan (VOrphan is for a shape nobody claims
    -- at all) — still fatal, still loud, but a different, correctly-named
    -- class. See `classify` above for the three-way split.
    --
    -- CAVEAT (post-Task-7 review round 2): the above is true for a hole
    -- landing in a VALIDATING capture (Piece, Year, ...), but it is NOT
    -- the universal outcome — it depends entirely on whether the capture
    -- the hole lands in happens to reject the hole's literal text.
    -- Concretely: "<someYear>" contains no whitespace, so fix 7's UrlPath
    -- accepts it happily. Once the real "I GET {url} as {name}" step
    -- lands, "I GET /api/census?year=<someYear> as first" becomes a
    -- clean single match again — and the four lines fix 7 just made
    -- visible (see Steps.hs's UrlPath comment) go quiet a second time,
    -- silently fetching a URL containing the literal text "<someYear>".
    -- A no-op `dehole` would not be a safety net against that; the actual
    -- protection is substituting the hole with a real registered example
    -- value BEFORE matching, which is exactly what `dehole` now does
    -- (wired below to Prop.substituteExamples). Do not read "bad-value"
    -- above as a universal guarantee that holes get caught on their own —
    -- absent this wiring it would be an artifact of which captures happen
    -- to validate their input, not a law this module enforces.
    dehole = Prop.substituteExamples
    results = [ (defSketch d, defRun d (dehole body)) | d <- defs, defKw d == k ]
    matched = [ sk | (sk, Matched _) <- results ]
    errored = [ (sk, e) | (sk, ClaimError e) <- results ]

-- | Every violation across a feature: (scenario name, step body, kind).
violations :: [StepDef] -> Feature -> [(Text, Text, Violation)]
violations defs f =
  [ (scName sc, stepBody st, v)
  | sc <- ftScenarios f, st <- scSteps sc
  , Just v <- [classify defs st] ]

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
    src <- TIO.readFile p
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
