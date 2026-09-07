module Check where

import Data.Text (Text)
import qualified Data.Text as T
import qualified Data.Text.IO as TIO
import Gherkin.Ast
import Gherkin.Parse (parseFeature)
import System.Directory (doesDirectoryExist, listDirectory)
import System.Exit (exitFailure)
import System.FilePath ((</>), takeExtension)
import World (StepDef (..))

-- The totality law (spec §4): every step in every feature matches EXACTLY
-- ONE definition, or CI fails naming the offenders. That "exactly one" has
-- two ways to fail, and both are the SAME law, not two separate checks:
--   0 definitions claim the step -> an orphan (nobody defines it)
--   1 definition claims it       -> fine, the law holds
--   2+ definitions claim it      -> ambiguous (more than one claims it)
-- `classify` is the one place that turns a step into one of those three
-- outcomes; `orphans` and `ambiguous` are just filters over it, and
-- `checkDir` reports both from a single traversal.

-- | The definitions (matching the step's keyword) that claim this body —
-- i.e. whose `defRun` returns `Just` rather than falling through. A
-- literal mismatch is "not this step" (Nothing, per R4); anything else,
-- including a capture that fails to PARSE, is a claim (Just) even when
-- the claimed value is bad — that's what makes cross-definition ambiguity
-- representable at all (Task 5 review finding; see Steps.hs's ordering
-- comment for the concrete "the response field style equals canaan" case
-- that only allSteps' list order hides today).
claimants :: [StepDef] -> Keyword -> Text -> [StepDef]
claimants defs k body =
  [ d | d <- defs, defKw d == k, Just _ <- [defRun d (dehole body)] ]
  where
    -- property holes (e.g. <someYear>) must be substituted with a
    -- registered example value before matching, or every @property step
    -- looks like an orphan. Task 9 wires this to Prop.substituteExamples;
    -- until a hole name is registered, the step is an orphan — loudly.
    dehole = id

-- | How one step fares under the totality law, or Nothing if it holds
-- (exactly one definition claims it).
data Violation = VOrphan | VAmbiguous [Text]
  deriving (Eq, Show)

classify :: [StepDef] -> Step -> Maybe Violation
classify defs (Step k body _) = case claimants defs k body of
  []  -> Just VOrphan
  [_] -> Nothing
  ds  -> Just (VAmbiguous (map defSketch ds))

-- | Every violation across a feature: (scenario name, step body, kind).
violations :: [StepDef] -> Feature -> [(Text, Text, Violation)]
violations defs f =
  [ (scName sc, stepBody st, v)
  | sc <- ftScenarios f, st <- scSteps sc
  , Just v <- [classify defs st] ]

-- | Steps matching zero definitions: (scenario, step body).
orphans :: [StepDef] -> Feature -> [(Text, Text)]
orphans defs f = [ (sc, b) | (sc, b, VOrphan) <- violations defs f ]

-- | Steps matching two or more definitions: (scenario, step body, the
-- competing definitions' human-readable sketches).
ambiguous :: [StepDef] -> Feature -> [(Text, Text, [Text])]
ambiguous defs f = [ (sc, b, ss) | (sc, b, VAmbiguous ss) <- violations defs f ]

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
    label VOrphan        = "ORPHAN"
    label (VAmbiguous _) = "AMBIGUOUS"
    describe b VOrphan = b
    describe b (VAmbiguous sketches) =
      b <> " matches " <> T.pack (show (length sketches)) <> " definitions: "
        <> T.intercalate " | " sketches
