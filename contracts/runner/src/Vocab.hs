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
import World (Claim (..), StepDef (..), readFeatureFile)

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

-- A table row, by the same shape Gherkin.Parse's (private, unexported)
-- `tableRow` accepts: "|"-wrapped, non-empty inside. Deliberately
-- duplicated rather than shared: `tableRow` lives in `Gherkin.Parse`'s own
-- `where` clause, and widening that module's exports just to share an
-- 8-character predicate would be a worse trade than the duplication.
-- The coupling runs BOTH ways and is currently inert, not just one-way:
-- if `Gherkin.Parse`'s row grammar ever changes, this must change with
-- it, or `vocabRegion` could mis-locate an existing block's extent; and
-- conversely, this module can't unilaterally recognize a WIDER row shape
-- than the parser does, since `spliceVocab` only ever runs on text that
-- `parseFeature` has already accepted (see `vocabDir`'s `one`) — a row
-- shape the parser wouldn't recognize can't reach here parsed as a
-- feature in the first place, which is what makes today's duplication
-- inert rather than a live drift risk.
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

-- `T.lines`/`T.unlines` are not inverse: `T.lines "a\nb"` and
-- `T.lines "a\nb\n"` are BOTH `["a", "b"]` (the trailing-newline-or-not
-- distinction is thrown away on the way in), while `T.unlines` always
-- reappends one on the way out. Splicing through `T.lines` and
-- reassembling with a blanket `T.unlines` would therefore silently ADD a
-- trailing newline to any corpus file that lacked one, which is exactly
-- the kind of stray byte the surgical requirement forbids -- true today
-- only because all twelve corpus files happen to already end in "\n"
-- (verified directly, and now pinned by a test below rather than left as
-- an untested aside). `joinLines` carries that one bit explicitly instead
-- of relying on `T.unlines`'s fixed opinion, so the reassembled text
-- has a trailing newline if and only if the ORIGINAL did.
joinLines :: Bool -> [Text] -> Text
joinLines hadTrailingNewline ls =
  T.intercalate "\n" ls <> (if hadTrailingNewline then "\n" else "")

-- Review finding (round 3, controller ruling): this used to be a SINGLE
-- pass -- `mapM one files`, where `one` read, parsed, and (in write mode)
-- WROTE each file in the same breath, with the aggregate parse-error
-- check only happening after every file's `one` action had already run.
-- That meant a directory with one malformed feature among several good
-- ones would genuinely rewrite the good files' bytes on disk BEFORE
-- reporting the parse failure and exiting non-zero -- a partial write on
-- a failed run, which is exactly the half-applied state the surgical
-- requirement exists to prevent (a tool that sometimes stamps some of the
-- owner's hand-written corpus and sometimes doesn't, depending on which
-- OTHER file happens to be broken, is not trustworthy to run at all).
--
-- Fixed structurally, not just with a test: `vocabDir` is now genuinely
-- two phases, sequenced by Haskell's own data dependency (phase 2 cannot
-- run without phase 1's result) rather than by convention. Phase 1
-- (`readAndParse`) reads and parses EVERY file and produces nothing but
-- data -- no write, ever, happens here. Only after ALL of phase 1 has
-- completed does `vocabDir` decide whether ANY file failed to parse; if
-- so, it reports every failure and exits, having touched no file's bytes
-- at all. Phase 2 (`writeOne` / the drift computation) runs only on the
-- `goods` list, and only after that all-clear -- so a single malformed
-- feature now makes --write a strict no-op across the WHOLE directory,
-- not a partial one.
vocabDir :: [StepDef] -> FilePath -> Bool -> IO ()
vocabDir defs dir writeMode = do
  files <- featureFilesLocal dir
  parsed <- mapM readAndParse files
  let parseErrs = [ e | Left e <- parsed ]
  if not (null parseErrs)
    -- Fatal in BOTH modes, not just verify: a malformed feature file must
    -- never let --write silently rewrite every OTHER file in the
    -- directory and then report success anyway (the brief's original
    -- stub did exactly that -- see task-8-report.md's "Deviations"), NOR
    -- rewrite them and then report FAILURE (the bug this phase split just
    -- closed) -- either way, no byte of any file moves when any file in
    -- the directory fails to parse.
    then mapM_ TIO.putStrLn parseErrs >> exitFailure
    else do
      let goods = [ (p, src, f) | Right (p, src, f) <- parsed ]
      if writeMode
        then do
          changes <- mapM (\(p, src, f) -> writeOne p src f) goods
          -- Say what actually happened, not what the command merely
          -- attempted: an unconditional "rewritten" is itself a small
          -- false claim on the (common, e.g. a second consecutive run)
          -- case where every table was already correct and nothing on
          -- disk changed.
          let changed = length (filter id changes)
              total = length goods
          TIO.putStrLn $ if changed == 0
            then "vocabulary: already matches its types across "
                 <> T.pack (show total) <> " file(s); nothing rewritten"
            else "vocabulary: rewrote " <> T.pack (show changed) <> " of "
                 <> T.pack (show total) <> " file(s)"
        else do
          let driftMsgs = concat
                [ map ((T.pack p <> ": ") <>) (drift defs f) | (p, _, f) <- goods ]
          if null driftMsgs
            then TIO.putStrLn "vocabulary: every table matches its types"
            else mapM_ TIO.putStrLn driftMsgs >> exitFailure
  where
    -- Phase 1: read and parse ONE file. Pure data in, data out -- no
    -- write, no side effect beyond the read itself, so running this over
    -- every file before deciding anything is always safe to do.
    readAndParse :: FilePath -> IO (Either Text (FilePath, Text, Feature))
    readAndParse p = do
      -- Final-review Fix 4: this used to decode the raw bytes as UTF-8
      -- itself (verified empirically that this toolchain's default
      -- text-handle decoder is NOT UTF-8 -- it silently mangles a 3-byte
      -- em dash rather than raising an error, corrupting every em-dash
      -- prose line in the corpus the moment it round-tripped through a
      -- write). That experiment was correct, but doing it only HERE left
      -- Run.hs/Check.hs/Prop.hs's plain `TIO.readFile` calls looking
      -- fine by omission when they were not -- the very corruption this
      -- comment warned about, just at the other three call sites instead
      -- of this one. Now shared: World.readFeatureFile carries this
      -- reasoning once, for all four readers, so there is one correct
      -- decode instead of one right answer and three accidental wrong
      -- ones. The write side (encodeUtf8 below) still needs its own
      -- explicit encode; only the read side collapses into the shared
      -- function.
      src <- readFeatureFile p
      pure $ case parseFeature p src of
        Left e  -> Left (T.pack p <> ": " <> e)
        Right f -> Right (p, src, f)

    -- Phase 2 (write mode only), and only ever called after EVERY file in
    -- the directory has already parsed cleanly: splice this one file's
    -- vocabulary and write it back if -- and only if -- something
    -- actually changed. Returns whether it did, for the summary above.
    writeOne :: FilePath -> Text -> Feature -> IO Bool
    writeOne p src f = do
      let vocab = expectedVocab defs f
          hadTrailingNewline = "\n" `T.isSuffixOf` src
          newSrc = joinLines hadTrailingNewline (spliceVocab vocab (T.lines src))
          changed = newSrc /= src
      -- Only touch the file when something actually changed: an
      -- already-correct table is left with its original mtime, and no
      -- risk of an accidental no-op rewrite hitting the newline-
      -- translation hazard below for nothing.
      if changed
        -- NOT TIO.writeFile: GHC's text handles default to native
        -- newline translation, which on Windows turns every "\n" into
        -- "\r\n" on output -- rewriting every line ending in the file
        -- even though only the vocabulary block's CONTENT changed
        -- (verified empirically: a plain TIO.writeFile of "a\nb\n" on
        -- this toolchain produced "a\r\nb\r\n" on disk). Binary-mode
        -- ByteString.writeFile performs no translation, so every
        -- untouched line stays byte-for-byte identical.
        then BS.writeFile p (TE.encodeUtf8 newSrc)
        else pure ()
      pure changed
