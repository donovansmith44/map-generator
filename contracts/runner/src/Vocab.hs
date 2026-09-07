module Vocab where

import Data.List (nub, sort)
import qualified Data.ByteString as BS
import Data.Text (Text)
import qualified Data.Text as T
import qualified Data.Text.Encoding as TE
import qualified Data.Text.IO as TIO
import Capture (Universe (..), describeUniverse)
import Check (featureFilesLocal)
import Gherkin.Ast
import Gherkin.Parse (parseFeature)
import System.Exit (exitFailure)
import World (Claim (..), StepDef (..))

-- Reused rather than duplicated: `module Check where` carries no export
-- list, so `featureFilesLocal` is already exposed, and Vocab -> Check
-- introduces no cycle (Check never imports Vocab or Main -- only Main
-- imports both). Task 7's own comment on this walker explains why IT
-- duplicated Main's copy (Main imports Check, so Check -> Main would be
-- a cycle); that reasoning doesn't apply a second time in this direction,
-- so a second copy here would just be needless duplication, not a cycle
-- avoided.

-- The union of universes contributed by the definitions this feature's
-- steps actually MATCH -- THE source the table must equal. "Matched", not
-- merely "claimed": World.Claim has three states (R23) and a definition
-- that only recognizes a line's shape but fails to parse the value
-- (ClaimError) contributes nothing here, exactly as Check.hs's totality
-- law counts only full matches, not mere claims, toward "exactly one".
--
-- Described universes are excluded STRUCTURALLY, by matching on the
-- Universe constructor, not by a hardcoded list of capture names. A
-- Described universe (FixtureRef's "a named answer in fixtures/, ...",
-- BindName's "a scene name to bind, ...", FixtureRefFreeText's free text,
-- and UrlPath's "a URL path with no embedded whitespace, ..." -- new
-- since this task was scoped) is prose the capture's author wrote to
-- explain a free-form value; it is not a finite or bounded vocabulary a
-- dummy must learn before writing a scenario, the way an Enumerated set
-- of pieces/styles or a Ranged year window is. UrlPath is excluded on
-- exactly the same structural grounds as "name" and "text" -- not a
-- special case bolted on for it, an instance of the general rule.
-- FixtureRef's Described universe is handled the SAME way (excluded):
-- its prose ("a named answer in fixtures/, e.g. ...") is worth writing
-- once at the type's definition for a reader who wonders what a
-- @fixture@ capture even is, but it is not a closed vocabulary either --
-- there is no finite list of legal fixture names to enumerate in a
-- feature's table, so a Vocabulary row for it would just repeat the same
-- prose sentence in every feature that uses it, never a real answer set.
expectedVocab :: [StepDef] -> Feature -> [(Text, Text)]
expectedVocab defs f = nub
  [ (name, describeUniverse u)
  | sc <- ftScenarios f, st <- scSteps sc
  , StepDef k _ us m <- defs, k == stepKw st, Matched _ <- [m (stepBody st)]
  , (name, u) <- us, isVocab u ]
  where
    isVocab (Described _) = False
    isVocab _             = True

drift :: [StepDef] -> Feature -> [Text]
drift defs f =
  let want = sort (expectedVocab defs f)
      have = sort (ftVocab f)
  in [ "vocabulary drift: table says " <> T.pack (show have)
       <> " but the types say " <> T.pack (show want) | want /= have ]

-- ---------- surgical --write (controller ruling R34) ----------
-- --write must touch ONLY the Vocabulary block. parseFeature/renderFeature
-- round-trip the WHOLE file: renderFeature reflows every preamble line
-- through its own formatting, and parseFeature drops blank lines within a
-- preamble outright (see Gherkin.Parse's `isComment`) -- both fine for the
-- round-trip LAW those two functions are pinned against (Spec.hs's
-- "round-trips: parse . render == Right"), both hostile to a hand-written
-- corpus a human owns. So writing here never calls renderFeature: it
-- works at the raw-line level, locates the existing "  Vocabulary:" block
-- (if any) using the same line classification the parser itself uses
-- (blank / a tag line / a "Scenario: " line / a "Vocabulary:" header / a
-- table row), replaces exactly that span -- or inserts a fresh block at
-- the same point the parser would have accepted one, if there is no
-- block yet -- and leaves every other line byte-for-byte untouched.

-- A table row, by the same shape Gherkin.Parse's `tableRow` accepts:
-- "|"-wrapped, non-empty inside.
isTableRow :: Text -> Bool
isTableRow l =
  let s = T.strip l in "|" `T.isPrefixOf` s && "|" `T.isSuffixOf` s && T.length s > 1

-- Where a preamble (and therefore any Vocabulary block within it) ends: a
-- tag line or the first Scenario line -- and ONLY those. Mirrors
-- Gherkin.Parse's own `goBody` dispatch exactly: `isComment` treats a
-- blank line as skippable filler, same as a "#" comment, and `goBody`
-- simply continues past it without ever treating it as the end of the
-- preamble -- a blank line can (and, per renderFeature's own convention,
-- normally does) sit BETWEEN the preamble prose and a "Vocabulary:"
-- header, so treating blank-as-boundary would stop the scan before ever
-- reaching a real block that comes right after one.
isBoundary :: Text -> Bool
isBoundary l =
  let s = T.strip l in "@" `T.isPrefixOf` s || "Scenario: " `T.isPrefixOf` s

-- Skip the FEATURE-level header (any leading comment/blank lines, any
-- FEATURE-level tag line such as "@smoke" before "Feature: ...", and the
-- "Feature: " line itself) before hunting for a boundary. Mirrors
-- Gherkin.Parse's `go0`. Without this, a feature-level tag line at the
-- very top of the file (which also starts with "@", same as a scenario
-- tag) would satisfy `isBoundary` immediately at index 0 and make
-- `vocabRegion` insert a fresh block before the tag line -- i.e. before
-- the Feature: line itself.
skipHeader :: [Text] -> Int
skipHeader ls = go 0
  where
    n = length ls
    go i
      | i >= n = i
      | let s = T.strip (ls !! i), T.null s || "#" `T.isPrefixOf` s = go (i + 1)
      | "@" `T.isPrefixOf` T.strip (ls !! i) = go (i + 1)
      | "Feature: " `T.isPrefixOf` T.strip (ls !! i) = i + 1
      | otherwise = i + 1 -- malformed; parseFeature already rejected this

-- Right (start, end): an existing Vocabulary block spans raw line indices
-- [start, end) (the header line through its last contiguous table row).
-- Left i: no block exists yet; one belongs right before index i (the
-- first boundary line reached, after the Feature-level header, without
-- ever seeing a "Vocabulary:" header). The title and any preamble prose
-- lines are neither a vocabulary header nor a boundary, so the scan
-- simply passes over them.
--
-- Both the insertion index (Left) AND an existing header's own index
-- (Right) back up over any contiguous run of blank lines immediately
-- before them: `renderRows` always supplies its OWN leading blank line,
-- so replacing/inserting right AT the anchor would either glue the new
-- block straight onto whatever precedes it with no separating blank at
-- all (if there were none originally), or stack a second blank on top of
-- one that already served that job (the ordinary case in this corpus:
-- prose, blank, "Vocabulary:" -- or, with no block yet, prose, blank,
-- Scenario). Backing up folds that pre-existing blank into the replaced
-- span instead, so the result reads exactly like renderFeature's own
-- convention -- one blank before the header, one blank after the rows --
-- without ever duplicating a line that was already there.
vocabRegion :: [Text] -> Either Int (Int, Int)
vocabRegion ls = classify (skipHeader ls)
  where
    n = length ls
    classify i
      | i >= n = Left (backOverBlanks i)
      | T.strip (ls !! i) == "Vocabulary:" = Right (backOverBlanks i, consumeRows (i + 1))
      | isBoundary (ls !! i) = Left (backOverBlanks i)
      | otherwise = classify (i + 1)
    consumeRows j
      | j < n, isTableRow (ls !! j) = consumeRows (j + 1)
      | otherwise = j
    backOverBlanks i
      | i > 0, T.null (T.strip (ls !! (i - 1))) = backOverBlanks (i - 1)
      | otherwise = i

-- The block's own text, in renderFeature's exact format (blank line, the
-- header, one "    | k | v |" row per entry) -- so a file that never had
-- a block and a file whose block this function just replaced end up
-- indistinguishable. An empty vocabulary renders as no block at all
-- (nothing to insert; an existing block whose replacement is empty is
-- simply deleted -- not reachable from any feature in the current corpus,
-- since none starts with a stamped block that a step change later empties
-- out, but handled the same way renderFeature itself treats an empty
-- ftVocab).
renderRows :: [(Text, Text)] -> [Text]
renderRows [] = []
renderRows rows =
  "" : "  Vocabulary:" : [ "    | " <> k <> " | " <> v <> " |" | (k, v) <- rows ]

spliceVocab :: [(Text, Text)] -> [Text] -> [Text]
spliceVocab rows ls = case vocabRegion ls of
  Left i       -> take i ls ++ renderRows rows ++ drop i ls
  Right (s, e) -> take s ls ++ renderRows rows ++ drop e ls

vocabDir :: [StepDef] -> FilePath -> Bool -> IO ()
vocabDir defs dir writeMode = do
  files <- featureFilesLocal dir
  results <- mapM one files
  let parseErrs = [ e | Left e <- results ]
  if not (null parseErrs)
    then mapM_ TIO.putStrLn parseErrs >> exitFailure
    else if writeMode
      then TIO.putStrLn "vocabulary: rewritten"
      else
        let driftMsgs = concat [ ds | Right ds <- results ]
        in if null driftMsgs
             then TIO.putStrLn "vocabulary: every table matches its types"
             else mapM_ TIO.putStrLn driftMsgs >> exitFailure
  where
    one :: FilePath -> IO (Either Text [Text])
    one p = do
      -- NOT TIO.readFile: verified empirically that this toolchain's
      -- default text-handle decoder is NOT UTF-8 (it silently mangles a
      -- 3-byte UTF-8 em dash into three separate Latin/Cyrillic-ish
      -- characters instead of raising an error), which would corrupt
      -- every non-ASCII prose line in the corpus (titles and scenario
      -- names use em dashes throughout) the moment it round-trips through
      -- a write. Decoding the raw bytes as UTF-8 explicitly is the read-
      -- side half of the same fix as the encodeUtf8 write below.
      raw <- BS.readFile p
      let src = TE.decodeUtf8 raw
      case parseFeature p src of
        Left e -> pure (Left (T.pack p <> ": " <> e))
        Right f
          | writeMode -> do
              let vocab = expectedVocab defs f
                  newSrc = T.unlines (spliceVocab vocab (T.lines src))
              -- Only touch the file when something actually changed: an
              -- already-correct table is left with its original mtime,
              -- and no risk of an accidental no-op rewrite hitting the
              -- newline-translation hazard below for nothing.
              if newSrc /= src
                -- NOT TIO.writeFile: GHC's text handles default to native
                -- newline translation, which on Windows turns every "\n"
                -- into "\r\n" on output -- rewriting every line ending in
                -- the file even though only the vocabulary block's
                -- CONTENT changed (verified empirically: a plain
                -- TIO.writeFile of "a\nb\n" on this toolchain produced
                -- "a\r\nb\r\n" on disk). Binary-mode ByteString.writeFile
                -- performs no translation, so every untouched line stays
                -- byte-for-byte identical.
                then BS.writeFile p (TE.encodeUtf8 newSrc)
                else pure ()
              pure (Right [])
          | otherwise -> pure (Right (map ((T.pack p <> ": ") <>) (drift defs f)))
