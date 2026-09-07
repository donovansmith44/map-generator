# Stage 0: The Contract Runner and the Red/Green Diagnosis — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the Haskell contract runner, write the full map-api v0.1 and atlas-edge CDC feature suites, add `/api/contract` and `/api/census` to the server, then run everything against the CURRENT server to produce the honest red/green diagnosis of which algebraic laws we already break.

**Architecture:** A standalone cabal project (`contracts/runner`) interprets Gherkin `.feature` files with three owned extensions: typed capture patterns (no regexes), drift-checked `Vocabulary:` blocks, and `@property` holes that turn scenarios into QuickCheck properties. It speaks only HTTP+JSON against a `BASE_URL`. The Rust server gains two read-only routes. Nothing else in production changes — Stage 0 is measurement.

**Tech Stack:** Haskell (GHC ≥ 9.6 via ghcup), cabal, megaparsec, aeson, http-client, QuickCheck, hspec, optparse-applicative. Rust changes: map-canon (census function), map-viewer (two routes).

**Spec:** `docs/superpowers/specs/2026-09-06-map-api-contract-design.md`

## Global Constraints

- TDD is mandatory: every task writes its failing test before its implementation (owner decree).
- The contract suite is **pre-release**: `contracts/VERSION` = `0.1.0`; v1.0 is the owner's declaration, never automatic.
- Features' scenarios ARE laws — no shapes-only feature files anywhere.
- The runner speaks only HTTP+JSON to `BASE_URL`; no imports from the Rust workspace.
- Rust edits follow the render pipeline law: after changing map-viewer, `cargo build --release`, stop the old process, relaunch DETACHED via `Start-Process` on port **8090** (session background tasks get killed). No canon recompile is needed in this plan (no map-compile changes).
- Port **8080** belongs to the atlas API (CDC provider verification target). Never bind to it.
- Stage 0 makes NO visual change; the golden gate (`node crates/map-viewer/tests/golden.js --check`, run from `C:\Users\donov\.claude\jobs\c6946bce\tmp` so playwright-core resolves) must still hold at the end.
- Diagnosis only: scenarios that fail against today's server are RECORDED, not fixed, in this stage. `@target`-tagged scenarios are *expected* red.
- **Whole-body assertions (owner decree):** a scenario pins the ENTIRE answer — fixture equality for our API (don't-cares masked explicitly in the scenario text), the entire CONSUMED PROJECTION for the atlas edge. Existential poke-assertions ("some entry equals…", "is an array", "has at least N") are forbidden: a check satisfiable by the failure mode isn't a check. Relational algebra scenarios (A equals B, subset) compare whole bodies against each other.
- Haskell commands below run from `contracts/runner/` unless stated. If `ghc`/`cabal` are missing: `ghcup install ghc 9.6.7 && ghcup install cabal latest && ghcup set ghc 9.6.7` (ghcup itself from https://www.haskell.org/ghcup/ — its Windows installer works in Git Bash).

---

### Task 1: Runner scaffold — cabal project that builds and runs one trivial hspec test

**Files:**
- Create: `contracts/VERSION`
- Create: `contracts/runner/contract-runner.cabal`
- Create: `contracts/runner/cabal.project`
- Create: `contracts/runner/src/Gherkin/Ast.hs` (stub module, filled in Task 2)
- Create: `contracts/runner/app/Main.hs` (stub CLI)
- Create: `contracts/runner/test/Spec.hs`

**Interfaces:**
- Produces: a `contract-runner` executable and a `spec` test-suite runnable via `cabal test`; module namespace `Gherkin.*`, `Capture`, `World`, `Run`, `Vocab`, `Check`, `Prop` used by all later tasks.

- [ ] **Step 1: Write contracts/VERSION**

```text
0.1.0
```
(one line, trailing newline)

- [ ] **Step 2: Write the cabal files**

`contracts/runner/cabal.project`:
```text
packages: .
```

`contracts/runner/contract-runner.cabal`:
```cabal
cabal-version:      3.0
name:               contract-runner
version:            0.1.0
build-type:         Simple

common warnings
    ghc-options: -Wall -Wincomplete-patterns

library
    import:           warnings
    hs-source-dirs:   src
    exposed-modules:  Gherkin.Ast
                      Gherkin.Parse
                      Gherkin.Render
                      Capture
                      Pattern
                      World
                      Steps
                      Run
                      Check
                      Vocab
                      Prop
    build-depends:    base >=4.17 && <5,
                      text,
                      containers,
                      bytestring,
                      megaparsec >=9,
                      aeson >=2,
                      aeson-pretty,
                      http-client,
                      http-types,
                      QuickCheck >=2.14,
                      directory,
                      filepath
    default-language: GHC2021

executable contract-runner
    import:           warnings
    hs-source-dirs:   app
    main-is:          Main.hs
    build-depends:    base, contract-runner, text, optparse-applicative,
                      directory, filepath
    default-language: GHC2021

test-suite spec
    import:           warnings
    type:             exitcode-stdio-1.0
    hs-source-dirs:   test
    main-is:          Spec.hs
    build-depends:    base, contract-runner, hspec, QuickCheck, text,
                      containers, aeson
    default-language: GHC2021
```

- [ ] **Step 3: Write the stub modules so the failing test COMPILES against real names**

`contracts/runner/src/Gherkin/Ast.hs`:
```haskell
module Gherkin.Ast where

import Data.Text (Text)

newtype Tag = Tag Text deriving (Eq, Ord, Show)

data Keyword = Given | When | Then deriving (Eq, Ord, Show, Bounded, Enum)

data StepArg = DocString Text | Table [[Text]] deriving (Eq, Show)

data Step = Step { stepKw :: Keyword, stepBody :: Text, stepArg :: Maybe StepArg }
  deriving (Eq, Show)

data Scenario = Scenario { scName :: Text, scTags :: [Tag], scSteps :: [Step] }
  deriving (Eq, Show)

-- Vocabulary rows are (term, description) pairs — Task 8 gives them law.
data Feature = Feature
  { ftTitle :: Text, ftTags :: [Tag], ftPreamble :: [Text]
  , ftVocab :: [(Text, Text)], ftScenarios :: [Scenario] }
  deriving (Eq, Show)
```

Create the other listed modules (`Gherkin/Parse.hs`, `Gherkin/Render.hs`, `Capture.hs`, `Pattern.hs`, `World.hs`, `Steps.hs`, `Run.hs`, `Check.hs`, `Vocab.hs`, `Prop.hs`) each containing only `module <Name> where` (plus imports they'll need later can wait). `app/Main.hs`:
```haskell
module Main where
main :: IO ()
main = putStrLn "contract-runner: no mode given (see --help in Task 6)"
```

- [ ] **Step 4: Write the failing smoke test**

`contracts/runner/test/Spec.hs`:
```haskell
module Main where

import Test.Hspec
import Gherkin.Ast

main :: IO ()
main = hspec $
  describe "scaffold" $
    it "keywords enumerate Given/When/Then" $
      [minBound .. maxBound] `shouldBe` [Given, When, Then]
```

- [ ] **Step 5: Run and verify it fails (before Step 3's Ast content exists) / passes (after)**

Run: `cabal test 2>&1 | tail -5` from `contracts/runner/`.
Expected first run (if Ast stub missing): compile error naming `Gherkin.Ast`. After Step 3: `1 example, 0 failures`.

- [ ] **Step 6: Commit**

```bash
git add contracts/
git commit -m "contract-runner scaffold: cabal project, AST skeleton, VERSION 0.1.0"
```

---

### Task 2: Gherkin AST parser + renderer with the round-trip law

**Files:**
- Modify: `contracts/runner/src/Gherkin/Parse.hs`
- Modify: `contracts/runner/src/Gherkin/Render.hs`
- Test: `contracts/runner/test/Spec.hs` (add describe-block)

**Interfaces:**
- Consumes: `Gherkin.Ast` types from Task 1.
- Produces: `parseFeature :: FilePath -> Text -> Either Text Feature` and `renderFeature :: Feature -> Text`; law `parseFeature p (renderFeature f) == Right f` for generated features.

- [ ] **Step 1: Write the failing tests (example + property)**

Append to `test/Spec.hs` (add imports `Gherkin.Parse`, `Gherkin.Render`, `Test.Hspec.QuickCheck (prop)`, `Test.QuickCheck`, `qualified Data.Text as T`):

```haskell
  describe "gherkin parse/render" $ do
    it "parses a feature with vocabulary, tags, and And-resolution" $ do
      let src = T.unlines
            [ "Feature: the scene"
            , "  Any subset renders; absence is identity."
            , ""
            , "  Vocabulary:"
            , "    | pieces | any of: ground, water |"
            , ""
            , "  @property"
            , "  Scenario: composition"
            , "    When I render pieces <someA> as sceneA"
            , "    And I render pieces <someB> as sceneB"
            , "    Then combining sceneA and sceneB equals rendering <someA> plus <someB>"
            ]
      case parseFeature "scene.feature" src of
        Left e -> expectationFailure (T.unpack e)
        Right f -> do
          ftTitle f `shouldBe` "the scene"
          ftVocab f `shouldBe` [("pieces", "any of: ground, water")]
          let [sc] = ftScenarios f
          scTags sc `shouldBe` [Tag "property"]
          -- And resolves to the PREVIOUS keyword at parse time:
          map stepKw (scSteps sc) `shouldBe` [When, When, Then]
    prop "round-trips: parse . render == Right" $ \f ->
      parseFeature "gen.feature" (renderFeature f) === Right f
```

The property needs an `Arbitrary Feature`. Add at the bottom of `Spec.hs` a generator producing SIMPLE well-formed features (titles/bodies from a safe alphabet, since Gherkin is line-oriented):

```haskell
safeText :: Gen T.Text
safeText = T.pack <$> listOf1 (elements (['a'..'z'] ++ ['0'..'9'] ++ " -"))
    `suchThat` (\t -> t == T.strip t && not (T.null (T.strip t)))

instance Arbitrary Feature where
  arbitrary = do
    t  <- safeText
    vs <- listOf ((,) <$> safeText <*> safeText)
    ss <- listOf1 genScenario
    pure (Feature t [] [] vs ss)
    where
      genScenario = do
        n  <- safeText
        tg <- sublistOf [Tag "property", Tag "target"]
        st <- listOf1 genStep
        pure (Scenario n tg st)
      genStep = Step <$> elements [Given, When, Then] <*> safeText <*> pure Nothing
```

- [ ] **Step 2: Run tests, verify they fail**

Run: `cabal test 2>&1 | tail -5`
Expected: FAIL — `parseFeature` not in scope.

- [ ] **Step 3: Implement the renderer (write it first; the parser targets its output)**

`Gherkin/Render.hs`:
```haskell
module Gherkin.Render (renderFeature) where

import Data.Text (Text)
import qualified Data.Text as T
import Gherkin.Ast

renderFeature :: Feature -> Text
renderFeature f = T.unlines $
     [tagLine (ftTags f) | not (null (ftTags f))]
  ++ ["Feature: " <> ftTitle f]
  ++ map ("  " <>) (ftPreamble f)
  ++ (if null (ftVocab f) then []
      else "" : "  Vocabulary:" : [ "    | " <> k <> " | " <> v <> " |" | (k, v) <- ftVocab f ])
  ++ concatMap scenario (ftScenarios f)
  where
    tagLine ts = T.unwords [ "@" <> t | Tag t <- ts ]
    scenario sc =
         [""]
      ++ ["  " <> tagLine (scTags sc) | not (null (scTags sc))]
      ++ ["  Scenario: " <> scName sc]
      ++ concatMap step (scSteps sc)
    step (Step k b arg) =
      ("    " <> kw k <> " " <> b) : maybe [] stepArgLines arg
    stepArgLines (DocString d) =
      ["    \"\"\""] ++ map ("    " <>) (T.lines d) ++ ["    \"\"\""]
    stepArgLines (Table rows) =
      [ "      | " <> T.intercalate " | " r <> " |" | r <- rows ]
    kw Given = "Given"; kw When = "When"; kw Then = "Then"
```

- [ ] **Step 4: Implement the parser**

`Gherkin/Parse.hs` — line-oriented, no megaparsec needed for the outer structure (megaparsec arrives in Task 4 for step patterns):

```haskell
module Gherkin.Parse (parseFeature) where

import Data.Text (Text)
import qualified Data.Text as T
import Gherkin.Ast

-- Line-oriented: classify each stripped line, fold into the AST.
-- And/But resolve to the PREVIOUS keyword here, so the AST never
-- carries them — a law the round-trip property preserves (render
-- never emits And; parse of an And-free file is identity).
parseFeature :: FilePath -> Text -> Either Text Feature
parseFeature path src = go0 (zip [1 :: Int ..] (map T.stripEnd (T.lines src))) []
  where
    err n m = Left (T.pack path <> ":" <> T.pack (show n) <> " " <> m)
    strip = T.strip
    isComment l = "#" `T.isPrefixOf` strip l || T.null (strip l)

    go0 [] _ = Left (T.pack path <> ": no Feature line")
    go0 ((n, l) : rest) tags
      | isComment l = go0 rest tags
      | "@" `T.isPrefixOf` strip l = go0 rest (tags ++ tagsOf l)
      | Just t <- T.stripPrefix "Feature: " (strip l) =
          goBody rest (Feature t tags [] [] [])
      | otherwise = err n ("expected Feature:, got " <> strip l)

    tagsOf l = [ Tag (T.drop 1 w) | w <- T.words (strip l), "@" `T.isPrefixOf` w ]

    goBody [] f = Right (doneFeature f)
    goBody ls@((_, l) : rest) f
      | isComment l = goBody rest f
      | strip l == "Vocabulary:" = let (vs, rest') = vocabRows rest in
          goBody rest' f { ftVocab = ftVocab f ++ vs }
      | "@" `T.isPrefixOf` strip l || "Scenario: " `T.isPrefixOf` strip l =
          goScenarios ls f []
      | otherwise = goBody rest f { ftPreamble = ftPreamble f ++ [strip l] }

    vocabRows ((_, l) : rest)
      | Just row <- tableRow l =
          case row of
            [k, v] -> let (vs, rest') = vocabRows rest in ((k, v) : vs, rest')
            _      -> ([], rest)
    vocabRows ls = ([], ls)

    tableRow l =
      let s = strip l
      in if "|" `T.isPrefixOf` s && "|" `T.isSuffixOf` s && T.length s > 1
           then Just (map T.strip (T.splitOn "|" (T.dropEnd 1 (T.drop 1 s))))
           else Nothing

    goScenarios [] f acc = Right (doneFeature f { ftScenarios = ftScenarios f ++ reverse acc })
    goScenarios ((n, l) : rest) f acc
      | isComment l = goScenarios rest f acc
      | "@" `T.isPrefixOf` strip l =
          goScenario rest f acc (tagsOf l) n
      | Just t <- T.stripPrefix "Scenario: " (strip l) =
          goSteps rest f acc (Scenario t [] []) Nothing
      | otherwise = err n ("expected Scenario or tags, got " <> strip l)
      where
        goScenario ((n', l') : rest') f' acc' tg _
          | isComment l' = goScenario rest' f' acc' tg n'
          | Just t <- T.stripPrefix "Scenario: " (strip l') =
              goSteps rest' f' acc' (Scenario t tg []) Nothing
        goScenario _ _ _ _ n' = err n' "tags must precede a Scenario"

    goSteps [] f acc sc _ =
      Right (doneFeature f { ftScenarios = ftScenarios f ++ reverse (sc : acc) })
    goSteps ls@((n, l) : rest) f acc sc prevKw
      | isComment l = goSteps rest f acc sc prevKw
      | "@" `T.isPrefixOf` strip l || "Scenario: " `T.isPrefixOf` strip l =
          goScenarios ls f (sc : acc)
      | Just row <- tableRow l
      , (s0 : older) <- reverse (scSteps sc) =
          let s0' = s0 { stepArg = Just (addRow (stepArg s0) row) }
          in goSteps rest f acc sc { scSteps = reverse (s0' : older) } prevKw
      | otherwise =
          case kwOf (strip l) prevKw of
            Just (k, b) ->
              goSteps rest f acc sc { scSteps = scSteps sc ++ [Step k b Nothing] } (Just k)
            Nothing -> err n ("not a step: " <> strip l)
      where
        addRow (Just (Table rs)) r = Table (rs ++ [r])
        addRow _ r = Table [r]

    kwOf s prev =
          (,) Given <$> T.stripPrefix "Given " s
      <|> (,) When  <$> T.stripPrefix "When "  s
      <|> (,) Then  <$> T.stripPrefix "Then "  s
      <|> (do b <- T.stripPrefix "And " s <|> T.stripPrefix "But " s
              k <- prev
              pure (k, b))
      where a <|> b = maybe b Just a

    doneFeature = id
```

(DocString parsing is deliberately omitted from v0.1 — no feature in this plan uses it; the renderer's DocString branch stays for AST totality. If a future feature needs it, it arrives with its own test.)

- [ ] **Step 5: Run tests until green**

Run: `cabal test 2>&1 | tail -5`
Expected: PASS including the round-trip property (100 cases). If the property finds a mismatch, the shrunk counterexample IS the bug report — fix parse/render until it holds.

- [ ] **Step 6: Commit**

```bash
git add contracts/runner
git commit -m "gherkin parse/render with round-trip law; And resolves at parse"
```

---

### Task 3: Capture — typed values with self-declared universes

**Files:**
- Modify: `contracts/runner/src/Capture.hs`
- Test: `contracts/runner/test/Spec.hs`

**Interfaces:**
- Produces:
```haskell
data Universe = Enumerated [Text] | Ranged Text Text Text | Described Text
class FromCapture a where
  capName  :: proxy a -> Text          -- vocabulary term, e.g. "pieces"
  parseCap :: Text -> Either Text a
  renderCap :: a -> Text               -- inverse; property-holes render through this
  universe :: proxy a -> Universe
newtype Year = Year Int
newtype PieceSet = PieceSet (Set Piece)
data Piece = Ground | Water | Fills | Borders | Claims | Labels | Markers | Journeys | Chrome | Veil
newtype StyleName = StyleName Text     -- canaan | parchment | slate
newtype FixtureRef = FixtureRef Text
allPieces :: PieceSet
describeUniverse :: Universe -> Text   -- the Vocabulary-table cell text
didYouMean :: [Text] -> Text -> Text   -- nearest by edit distance
```
- Laws produced (tested here, relied on by Tasks 8–9): enumerated round-trip, `parseCap . renderCap ≡ Right`.

- [ ] **Step 1: Failing tests**

Append to `Spec.hs` (import `Capture`, `Data.Proxy`):
```haskell
  describe "capture universes" $ do
    it "every enumerated piece value round-trips" $ do
      let Enumerated vs = universe (Proxy @Piece)
      mapM_ (\v -> fmap renderCap (parseCap @Piece v) `shouldBe` Right v) vs
    it "piece sets parse comma-separated, any order, and render sorted" $ do
      renderCap <$> parseCap @PieceSet "water, ground" `shouldBe` Right "ground, water"
    it "a wrong piece gets a did-you-mean naming the universe" $
      case parseCap @PieceSet "water, topografy" of
        Left e -> do
          e `shouldSatisfy` T.isInfixOf "topografy"
          e `shouldSatisfy` T.isInfixOf "ground"   -- the full universe is listed
        Right _ -> expectationFailure "accepted a non-piece"
    it "years parse within the frame and refuse outside it" $ do
      parseCap @Year "-1405" `shouldBe` Right (Year (-1405))
      parseCap @Year "9999" `shouldSatisfy` isLeft
    prop "renderCap is a right inverse of parseCap for years" $
      \(y :: Int) -> let y' = (-4004) + (abs y `mod` 4105) in
        parseCap @Year (renderCap (Year y')) === Right (Year y')
```
(add `import Data.Either (isLeft)`; derive `Eq`/`Show` on the capture types.)

- [ ] **Step 2: Run, verify FAIL** — `cabal test 2>&1 | tail -5`: `Capture` exports nothing yet.

- [ ] **Step 3: Implement `Capture.hs`**

```haskell
module Capture where

import Data.List (sortOn)
import Data.Proxy (Proxy (..))
import Data.Set (Set)
import qualified Data.Set as Set
import Data.Text (Text)
import qualified Data.Text as T

data Universe = Enumerated [Text] | Ranged Text Text Text | Described Text
  deriving (Eq, Show)

class FromCapture a where
  capName   :: proxy a -> Text
  parseCap  :: Text -> Either Text a
  renderCap :: a -> Text
  universe  :: proxy a -> Universe

-- The one sentence a dummy reads in the Vocabulary table.
describeUniverse :: Universe -> Text
describeUniverse (Enumerated vs)   = "any of: " <> T.intercalate ", " vs
describeUniverse (Ranged lo hi m)  = "whole number from " <> lo <> " to " <> hi <> " (" <> m <> ")"
describeUniverse (Described d)     = d

didYouMean :: [Text] -> Text -> Text
didYouMean vocab w =
  case sortOn (dist w) vocab of
    (best : _) | dist w best <= 3 -> "  Did you mean: " <> best <> "?"
    _ -> ""
  where
    dist a b = lev (T.unpack a) (T.unpack b)
    lev [] bs = length bs
    lev as [] = length as
    lev (a:as) (b:bs)
      | a == b = lev as bs
      | otherwise = 1 + minimum [lev as (b:bs), lev (a:as) bs, lev as bs]

-- ---------- Piece / PieceSet ----------
data Piece = Ground | Water | Fills | Borders | Claims | Labels | Markers
           | Journeys | Chrome | Veil
  deriving (Eq, Ord, Show, Bounded, Enum)

pieceText :: Piece -> Text
pieceText = T.toLower . T.pack . show

instance FromCapture Piece where
  capName _ = "piece"
  universe _ = Enumerated (map pieceText [minBound .. maxBound])
  renderCap = pieceText
  parseCap t =
    let vs = [ (pieceText p, p) | p <- [minBound .. maxBound] ]
    in case lookup (T.strip t) vs of
         Just p -> Right p
         Nothing ->
           Left $ "'" <> T.strip t <> "' is not a piece."
                <> didYouMean (map fst vs) (T.strip t)
                <> "\n  Pieces are: " <> T.intercalate ", " (map fst vs)

newtype PieceSet = PieceSet (Set Piece) deriving (Eq, Show)

allPieces :: PieceSet
allPieces = PieceSet (Set.fromList [minBound .. maxBound])

instance FromCapture PieceSet where
  capName _ = "pieces"
  universe _ = universe (Proxy @Piece)
  renderCap (PieceSet s) = T.intercalate ", " (map pieceText (Set.toAscList s))
  parseCap t = PieceSet . Set.fromList <$> traverse parseCap (T.splitOn "," t)

-- ---------- Year ----------
newtype Year = Year Int deriving (Eq, Ord, Show)

instance FromCapture Year where
  capName _ = "year"
  universe _ = Ranged "-4004" "100" "negative means BC; -1405 is 1405 BC"
  renderCap (Year y) = T.pack (show y)
  parseCap t = case reads (T.unpack (T.strip t)) of
    [(y, "")] | y >= (-4004) && y <= 100 -> Right (Year y)
    [(y, "")] -> Left $ "year " <> T.pack (show y) <> " is outside the frame. "
                      <> describeUniverse (universe (Proxy @Year))
    _ -> Left $ "'" <> T.strip t <> "' is not a year. "
              <> describeUniverse (universe (Proxy @Year))

-- ---------- StyleName ----------
newtype StyleName = StyleName Text deriving (Eq, Show)

styleNames :: [Text]
styleNames = ["canaan", "parchment", "slate"]

instance FromCapture StyleName where
  capName _ = "style"
  universe _ = Enumerated styleNames
  renderCap (StyleName s) = s
  parseCap t
    | T.strip t `elem` styleNames = Right (StyleName (T.strip t))
    | otherwise = Left $ "'" <> T.strip t <> "' is not a style."
                       <> didYouMean styleNames (T.strip t)
                       <> "\n  Styles are: " <> T.intercalate ", " styleNames

-- ---------- FixtureRef ----------
newtype FixtureRef = FixtureRef Text deriving (Eq, Show)

instance FromCapture FixtureRef where
  capName _ = "fixture"
  universe _ = Described "a named answer in fixtures/, e.g. \"tribes-at-1405\""
  renderCap (FixtureRef f) = "\"" <> f <> "\""
  parseCap t =
    let s = T.strip t
    in if "\"" `T.isPrefixOf` s && "\"" `T.isSuffixOf` s && T.length s >= 2
         then Right (FixtureRef (T.dropEnd 1 (T.drop 1 s)))
         else Left "a fixture reference is quoted, e.g. \"tribes-at-1405\""
```

- [ ] **Step 4: Run tests until green** — `cabal test 2>&1 | tail -5`.

- [ ] **Step 5: Commit**

```bash
git add contracts/runner
git commit -m "typed captures with self-declared universes and did-you-mean errors"
```

---

### Task 4: Pattern — typed step patterns, no regexes

**Files:**
- Modify: `contracts/runner/src/Pattern.hs`
- Test: `contracts/runner/test/Spec.hs`

**Interfaces:**
- Consumes: `Capture` (Task 3).
- Produces:
```haskell
newtype StepP a          -- Functor/Applicative
lit      :: Text -> StepP ()
capUntil :: FromCapture a => Text -> StepP a  -- capture up to (and consuming) the literal
capRest  :: FromCapture a => StepP a          -- capture the remainder
matchP   :: StepP a -> Text -> Either Text a
usesOf   :: StepP a -> [(Text, Universe)]     -- vocabulary contributions (Task 8 reads this)
renderP  :: StepP a -> Text                   -- human sketch: "I render pieces {pieces} at {year}"
```

- [ ] **Step 1: Failing tests**

```haskell
  describe "step patterns" $ do
    let p = lit "I render pieces " *> ((,) <$> capUntil @PieceSet " at year " <*> capRest @Year)
    it "matches and yields typed captures" $
      matchP p "I render pieces water, ground at year -1405"
        `shouldBe` Right (PieceSet (Set.fromList [Ground, Water]), Year (-1405))
    it "a bad capture fails with the capture's own error, not a match miss" $
      case matchP p "I render pieces water, topografy at year -1405" of
        Left e  -> e `shouldSatisfy` T.isInfixOf "not a piece"
        Right _ -> expectationFailure "matched garbage"
    it "a literal mismatch says which literal" $
      matchP p "I paint pieces water at year 0" `shouldSatisfy` isLeft
    it "declares its vocabulary uses" $
      map fst (usesOf p) `shouldBe` ["pieces", "year"]
    it "renders a human sketch" $
      renderP p `shouldBe` "I render pieces {pieces} at year {year}"
```
(imports: `Pattern`, `qualified Data.Set as Set`.)

- [ ] **Step 2: Run, verify FAIL.**

- [ ] **Step 3: Implement `Pattern.hs`**

```haskell
module Pattern where

import Data.Proxy (Proxy (..))
import Data.Text (Text)
import qualified Data.Text as T
import Capture

-- A step pattern is an applicative over (remaining text -> result).
-- capUntil names its terminator, so matching is deterministic and
-- ambiguity (adjacent untyped captures) is unrepresentable by
-- construction — the terminator IS the next literal.
data StepP a = StepP
  { runP  :: Text -> Either Text (a, Text)
  , uses  :: [(Text, Universe)]
  , sketch :: [Text]
  }

instance Functor StepP where
  fmap f (StepP r u s) = StepP (\t -> fmap (\(a, rest) -> (f a, rest)) (r t)) u s

instance Applicative StepP where
  pure a = StepP (\t -> Right (a, t)) [] []
  StepP rf uf sf <*> StepP ra ua sa = StepP
    (\t -> do (f, t') <- rf t; (a, t'') <- ra t'; pure (f a, t''))
    (uf ++ ua) (sf ++ sa)

lit :: Text -> StepP ()
lit l = StepP
  (\t -> case T.stripPrefix l t of
      Just rest -> Right ((), rest)
      Nothing   -> Left ("expected literal '" <> l <> "' at: " <> T.take 40 t))
  [] [l]

capUntil :: forall a. FromCapture a => Text -> StepP a
capUntil terminator = StepP
  (\t -> case T.breakOn terminator t of
      (raw, rest) | terminator `T.isPrefixOf` rest -> do
        v <- parseCap raw
        pure (v, T.drop (T.length terminator) rest)
      _ -> Left ("expected '" <> terminator <> "' after {" <> capName (Proxy @a) <> "}"))
  [(capName (Proxy @a), universe (Proxy @a))]
  ["{" <> capName (Proxy @a) <> "}", terminator]

capRest :: forall a. FromCapture a => StepP a
capRest = StepP
  (\t -> do v <- parseCap t; pure (v, ""))
  [(capName (Proxy @a), universe (Proxy @a))]
  ["{" <> capName (Proxy @a) <> "}"]

matchP :: StepP a -> Text -> Either Text a
matchP p t = do
  (a, rest) <- runP p t
  if T.null (T.strip rest) then Right a
  else Left ("unmatched trailing text: " <> rest)

usesOf :: StepP a -> [(Text, Universe)]
usesOf = uses

renderP :: StepP a -> Text
renderP = T.concat . sketch
```

- [ ] **Step 4: Run tests until green.**

- [ ] **Step 5: Commit**

```bash
git add contracts/runner
git commit -m "typed step patterns: capUntil/capRest, vocabulary uses, human sketch"
```

---

### Task 5: World, transport seam, and the step library

**Files:**
- Modify: `contracts/runner/src/World.hs`
- Modify: `contracts/runner/src/Steps.hs`
- Test: `contracts/runner/test/Spec.hs`

**Interfaces:**
- Consumes: `Pattern`, `Capture`.
- Produces:
```haskell
-- World.hs
data World = World
  { baseUrl    :: Text
  , transport  :: Text -> IO (Either Text (ByteString, Value))  -- url -> (raw bytes, parsed)
  , fixtureDir :: FilePath
  , bound      :: Map Text (ByteString, Value)   -- "as sceneA" bindings
  , blessMode  :: Bool
  }
httpTransport :: Manager -> Text -> IO (Either Text (ByteString, Value))
data StepDef = StepDef { defKw :: Keyword, defSketch :: Text
                       , defUses :: [(Text, Universe)]
                       , defRun :: Text -> Maybe (World -> IO (Either Text World)) }
mkStep :: Keyword -> StepP a -> (a -> World -> IO (Either Text World)) -> StepDef
-- Steps.hs
allSteps :: [StepDef]
loadFixture :: World -> Text -> IO (Either Text Value)
sceneUrl :: Text -> PieceSet -> Year -> StyleName -> Text   -- pieces -> today's toggles
```
- The **piece→wire mapping for v0.1** (the honest bridge to today's params; the vocabulary documents it):
  `Labels ∉ set → labels=0` · `Water ∉ set → topo=0` · `Ground ∈ set → relief=1` (relief is opt-in today) · `Journeys ∉ set → journeys=0` · `Fills/Borders/Claims/Markers/Chrome/Veil`: always-on today, ignored by the wire (this is exactly what the diagnosis must expose).

- [ ] **Step 1: Failing tests (pure transport injection — no server needed)**

```haskell
  describe "world and steps" $ do
    let fakeResp = "{\"scene\":\"abc\",\"labels\":[]}"
        fake url = pure (Right (fakeResp, fromJust (A.decodeStrict fakeResp)))
          where _ = url
        w0 = World "http://x" fake "test/fixtures" mempty False
    it "sceneUrl maps pieces onto today's toggles" $ do
      sceneUrl "http://x" (PieceSet (Set.fromList [Fills, Borders])) (Year (-1405)) (StyleName "canaan")
        `shouldSatisfy` (\u -> all (`T.isInfixOf` u)
             ["year=-1405", "labels=0", "topo=0", "journeys=0", "style=canaan"]
             && not ("relief=1" `T.isInfixOf` u))
    it "the render step binds a named response" $ do
      let Just run = firstMatch When
            "I render pieces fills at year -1405 in style canaan as sceneA"
      Right w1 <- run w0
      Map.member "sceneA" (bound w1) `shouldBe` True
    it "the equality step compares two bound scenes as JSON values" $ do
      let Just run1 = firstMatch When
            "I render pieces fills at year -1405 in style canaan as sceneA"
          Just run2 = firstMatch When
            "I render pieces fills at year -1405 in style canaan as sceneB"
          Just run3 = firstMatch Then "sceneA equals sceneB"
      Right w1 <- run1 w0
      Right w2 <- run2 w1
      r <- run3 w2
      r `shouldSatisfy` isRight
  where
    firstMatch k t = listToMaybe
      [ f | StepDef k' _ _ m <- allSteps, k' == k, Just f <- [m t] ]
```
(imports: `World`, `Steps`, `qualified Data.Aeson as A`, `Data.Maybe`, `qualified Data.Map.Strict as Map`, `Data.Either (isRight)`. Note: the `where` on a `describe` needs the helper floated to top level of `Spec.hs` — put `firstMatch` there.)

- [ ] **Step 2: Run, verify FAIL.**

- [ ] **Step 3: Implement `World.hs`**

```haskell
module World where

import Data.Aeson (Value, eitherDecodeStrict)
import Data.ByteString (ByteString)
import qualified Data.ByteString.Lazy as BL
import Data.Map.Strict (Map)
import Data.Text (Text)
import qualified Data.Text as T
import Gherkin.Ast (Keyword)
import Capture (Universe)
import Pattern
import Network.HTTP.Client

data World = World
  { baseUrl    :: Text
  , transport  :: Text -> IO (Either Text (ByteString, Value))
  , fixtureDir :: FilePath
  , bound      :: Map Text (ByteString, Value)
  , blessMode  :: Bool
  }

httpTransport :: Manager -> Text -> IO (Either Text (ByteString, Value))
httpTransport mgr url = do
  req <- parseRequest (T.unpack url)
  resp <- httpLbs req mgr
  let raw = BL.toStrict (responseBody resp)
  pure $ case eitherDecodeStrict raw of
    Right v -> Right (raw, v)
    Left e  -> Left (T.pack e <> " for " <> url)

data StepDef = StepDef
  { defKw     :: Keyword
  , defSketch :: Text
  , defUses   :: [(Text, Universe)]
  , defRun    :: Text -> Maybe (World -> IO (Either Text World))
  }

mkStep :: Keyword -> StepP a -> (a -> World -> IO (Either Text World)) -> StepDef
mkStep k p f = StepDef k (renderP p) (usesOf p) $ \body ->
  case matchP p body of
    Right a -> Just (f a)
    Left e
      -- a literal mismatch means "not this step" (try the next def);
      -- a CAPTURE failure means "this step, bad value" (report it)
      | "expected literal" `T.isPrefixOf` e -> Nothing
      | otherwise -> Just (\_ -> pure (Left e))
```

- [ ] **Step 4: Implement `Steps.hs`** (the generic library; scene-piece steps included)

```haskell
module Steps where

import Data.Aeson (Value (..), eitherDecodeStrict)
import qualified Data.Aeson as A
import qualified Data.Aeson.Key as K
import qualified Data.Aeson.KeyMap as KM
import qualified Data.ByteString as BS
import qualified Data.Map.Strict as Map
import Data.Set (member)
import Data.Text (Text)
import qualified Data.Text as T
import qualified Data.Text.Encoding as TE
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
  , mkStep Then (lit "the response is a JSON array") $ \() w ->
      pure $ case Map.lookup "_last" (bound w) of
        Just (_, Array _) -> Right w
        Just _  -> Left "response is not an array"
        Nothing -> Left "no response"
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
  , mkStep Then (lit "" *> (capUntil @BindName "'s labels are empty" <* pure ())) $
      \(BindName a) w ->
        pure $ case field "labels" =<< scene' a w of
          Right (Array v) | V.null v -> Right w
          Right _ -> Left (a <> " has labels")
          Left e -> Left e
  ]
  where
    scene n w = maybe (Left ("unbound " <> n)) (Right . snd) (Map.lookup n (bound w))
    scene' = scene
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
```

Note for the implementer: the `Then "{name} equals {name}"` pattern starts with `lit ""` so the sketch renders cleanly; `mkStep`'s literal-mismatch rule still applies via the *capture* failing on non-bind-name text, which returns a reported error rather than falling through — that is correct behavior for `Then` bodies that name bindings, and the totality check (Task 7) is the guard against genuinely orphaned steps.

- [ ] **Step 5: Run tests until green.** `cabal test 2>&1 | tail -5`

- [ ] **Step 6: Commit**

```bash
git add contracts/runner
git commit -m "world + transport seam + typed step library with v0.1 piece->wire bridge"
```

---

### Task 6: The runner — execute features, report verdicts, bless fixtures, CLI

**Files:**
- Modify: `contracts/runner/src/Run.hs`
- Modify: `contracts/runner/app/Main.hs`
- Test: `contracts/runner/test/Spec.hs`

**Interfaces:**
- Consumes: everything above.
- Produces:
```haskell
data Verdict = Passed | Failed Text | Skipped Text deriving (Eq, Show)
data ScenarioResult = ScenarioResult { srFeature, srScenario :: Text
                                     , srTags :: [Tag], srVerdict :: Verdict }
runScenario :: [StepDef] -> World -> Scenario -> IO Verdict
runFeatureFiles :: [StepDef] -> World -> [FilePath] -> IO [ScenarioResult]
reportTable :: [ScenarioResult] -> Text   -- the markdown diagnosis table
```
- CLI (Main): `contract-runner run --base-url URL DIR [--bless] [--property-runs N]`, `contract-runner check DIR`, `contract-runner vocab DIR [--write]`. Exit code 0 iff no non-`@target` failures (`@target` reds are expected by design and reported, not fatal).

- [ ] **Step 1: Failing tests (pure transport again)**

```haskell
  describe "runner" $ do
    it "runs a scenario to Passed and reports @target failures as expected-red" $ do
      let feat = T.unlines
            [ "Feature: t"
            , "  Scenario: ok"
            , "    When I render pieces fills at year -1405 in style canaan as a"
            , "    And I render pieces fills at year -1405 in style canaan as b"
            , "    Then a equals b"
            , "  @target"
            , "  Scenario: expected red"
            , "    When I GET /api/nothing"
            , "    Then the response field missing equals nope"
            ]
          Right f = parseFeature "t.feature" feat
          fake _ = pure (Right ("{\"x\":1}", fromJust (A.decodeStrict "{\"x\":1}")))
          w = World "http://x" fake "test/fixtures" mempty False
      [r1, r2] <- mapM (runScenario allSteps w) (ftScenarios f)
      r1 `shouldBe` Passed
      r2 `shouldSatisfy` \v -> case v of Failed _ -> True; _ -> False
    it "an undefined step fails naming the orphan" $ do
      let Right f = parseFeature "t.feature"
            "Feature: t\n  Scenario: s\n    When I do something nobody defined"
          w = World "http://x" (\_ -> pure (Left "no")) "" mempty False
      v <- runScenario allSteps w (head (ftScenarios f))
      case v of
        Failed e -> e `shouldSatisfy` T.isInfixOf "nobody defined"
        _ -> expectationFailure "should have failed"
```

- [ ] **Step 2: Run, verify FAIL.**

- [ ] **Step 3: Implement `Run.hs`**

```haskell
module Run where

import Control.Exception (SomeException, try)
import Data.Text (Text)
import qualified Data.Text as T
import Gherkin.Ast
import Gherkin.Parse (parseFeature)
import World
import qualified Data.Text.IO as TIO

data Verdict = Passed | Failed Text | Skipped Text deriving (Eq, Show)

data ScenarioResult = ScenarioResult
  { srFeature :: Text, srScenario :: Text, srTags :: [Tag], srVerdict :: Verdict }
  deriving (Eq, Show)

runScenario :: [StepDef] -> World -> Scenario -> IO Verdict
runScenario defs w0 sc = go w0 (scSteps sc)
  where
    go _ [] = pure Passed
    go w (Step k body _ : rest) =
      case [ f | StepDef k' _ _ m <- defs, k' == k, Just f <- [m body] ] of
        (f : _) -> do
          r <- try (f w) :: IO (Either SomeException (Either Text World))
          case r of
            Left ex          -> pure (Failed (T.pack (show ex)))
            Right (Left e)   -> pure (Failed (kwText k <> " " <> body <> "\n    ✗ " <> e))
            Right (Right w') -> go w' rest
        [] -> pure (Failed ("undefined step: " <> kwText k <> " " <> body))
    kwText Given = "Given"; kwText When = "When"; kwText Then = "Then"

runFeatureFiles :: [StepDef] -> World -> [FilePath] -> IO [ScenarioResult]
runFeatureFiles defs w paths = fmap concat . mapM one $ paths
  where
    one p = do
      src <- TIO.readFile p
      case parseFeature p src of
        Left e  -> pure [ScenarioResult (T.pack p) "PARSE" [] (Failed e)]
        Right f -> mapM (\sc -> ScenarioResult (ftTitle f) (scName sc) (scTags sc)
                                  <$> runScenario defs w sc)
                        (ftScenarios f)

isTarget :: ScenarioResult -> Bool
isTarget = elem (Tag "target") . srTags

reportTable :: [ScenarioResult] -> Text
reportTable rs = T.unlines $
     "| feature | scenario | verdict |"
   : "|---|---|---|"
   : [ "| " <> srFeature r <> " | " <> srScenario r <> " | " <> cell r <> " |" | r <- rs ]
  where
    cell r = case (srVerdict r, isTarget r) of
      (Passed, False)   -> "✅ green"
      (Passed, True)    -> "🟢 green (target already met!)"
      (Failed _, True)  -> "🔴 red (expected — @target)"
      (Failed e, False) -> "❌ RED — " <> T.replace "\n" " " (T.take 160 e)
      (Skipped why, _)  -> "⏭ " <> why
```

- [ ] **Step 4: Implement the CLI in `app/Main.hs`**

```haskell
module Main where

import qualified Data.Map.Strict as Map
import qualified Data.Text as T
import qualified Data.Text.IO as TIO
import Network.HTTP.Client (newManager, defaultManagerSettings)
import Options.Applicative
import System.Directory (listDirectory, doesDirectoryExist)
import System.Exit (exitFailure, exitSuccess)
import System.FilePath ((</>), takeExtension)
import Run
import Steps (allSteps)
import World
import qualified Check
import qualified Vocab
import qualified Prop

data Cmd
  = CmdRun  { cBase :: String, cDir :: FilePath, cBless :: Bool, cRuns :: Int }
  | CmdCheck FilePath
  | CmdVocab { vDir :: FilePath, vWrite :: Bool }

cmd :: Parser Cmd
cmd = hsubparser
  (  command "run"   (info (CmdRun <$> strOption (long "base-url")
                                   <*> argument str (metavar "DIR")
                                   <*> switch (long "bless")
                                   <*> option auto (long "property-runs" <> value 100))
                       (progDesc "execute a contract directory against a server"))
  <> command "check" (info (CmdCheck <$> argument str (metavar "DIR"))
                       (progDesc "totality: every step matches exactly one definition"))
  <> command "vocab" (info (CmdVocab <$> argument str (metavar "DIR")
                                     <*> switch (long "write"))
                       (progDesc "verify (or --write) Vocabulary blocks against the types"))
  )

featureFiles :: FilePath -> IO [FilePath]
featureFiles dir = do
  entries <- listDirectory dir
  fmap concat . mapM walk $ [ dir </> e | e <- entries ]
  where
    walk p = do
      isDir <- doesDirectoryExist p
      if isDir then featureFiles p
      else pure [ p | takeExtension p == ".feature" ]

main :: IO ()
main = do
  c <- execParser (info (cmd <**> helper) fullDesc)
  case c of
    CmdRun base dir bless runs -> do
      mgr <- newManager defaultManagerSettings
      let w = World (T.pack base) (httpTransport mgr) (dir </> "fixtures") Map.empty bless
      files <- featureFiles dir
      results <- Prop.runWithProperties allSteps w runs files
      TIO.putStrLn (reportTable results)
      let hardReds = [ r | r <- results, not (isTarget r)
                         , Failed _ <- [srVerdict r] ]
      if null hardReds then exitSuccess
      else TIO.putStrLn (T.pack (show (length hardReds)) <> " non-target failures")
           >> exitFailure
    CmdCheck dir -> Check.checkDir allSteps dir
    CmdVocab dir w -> Vocab.vocabDir allSteps dir w
```

(`Prop.runWithProperties` arrives in Task 9; until then give `Prop` a passthrough `runWithProperties defs w _ files = runFeatureFiles defs w files` so Main compiles and the CLI works from this task on. `Check.checkDir`/`Vocab.vocabDir` get `error "Task 7/8"` stubs replaced in their tasks — Main compiles, the modes fail loudly if invoked early.)

- [ ] **Step 5: Run tests until green; smoke the CLI**

Run: `cabal test 2>&1 | tail -5` then `cabal run contract-runner -- run --base-url http://127.0.0.1:8090 ../../contracts/map-api 2>&1 | tail -3` (no features yet — expect an empty table, exit 0).

- [ ] **Step 6: Commit**

```bash
git add contracts/runner
git commit -m "runner: verdicts, report table, bless mode, CLI with run/check/vocab"
```

---

### Task 7: Totality check — every step matches exactly one definition

**Files:**
- Modify: `contracts/runner/src/Check.hs`
- Test: `contracts/runner/test/Spec.hs`

**Interfaces:**
- Consumes: `Run`, `World`, `Gherkin.*`.
- Produces: `orphans :: [StepDef] -> Feature -> [(Text, Text)]` (scenario, step body) and `checkDir :: [StepDef] -> FilePath -> IO ()` (prints orphans, exits non-zero if any).

- [ ] **Step 1: Failing test**

```haskell
  describe "totality" $
    it "names the orphan steps" $ do
      let Right f = parseFeature "t.feature" $ T.unlines
            [ "Feature: t"
            , "  Scenario: s"
            , "    When I render pieces fills at year -1405 in style canaan"
            , "    Then nobody wrote this step" ]
      map snd (Check.orphans allSteps f) `shouldBe` ["nobody wrote this step"]
```

- [ ] **Step 2: Run, verify FAIL.**

- [ ] **Step 3: Implement `Check.hs`**

```haskell
module Check where

import Data.Text (Text)
import qualified Data.Text as T
import qualified Data.Text.IO as TIO
import Gherkin.Ast
import Gherkin.Parse (parseFeature)
import System.Exit (exitFailure)
import World (StepDef (..))

orphans :: [StepDef] -> Feature -> [(Text, Text)]
orphans defs f =
  [ (scName sc, stepBody st)
  | sc <- ftScenarios f, st <- scSteps sc
  , null [ () | StepDef k _ _ m <- defs, k == stepKw st
              , Just _ <- [m (dehole (stepBody st))] ] ]
  where
    -- property holes match totality-wise with a placeholder value per
    -- hole type; Task 9's registry supplies renderable examples. Until
    -- a hole name is registered, the step is an orphan — loudly.
    dehole = id  -- replaced by Prop.substituteExamples in Task 9 wiring

checkDir :: [StepDef] -> FilePath -> IO ()
checkDir defs dir = do
  files <- featureFilesLocal dir
  bad <- fmap concat . mapM (\p -> do
    src <- TIO.readFile p
    pure $ case parseFeature p src of
      Left e  -> [(T.pack p, e)]
      Right f -> [ (T.pack p <> " / " <> s, b) | (s, b) <- orphans defs f ]) $ files
  if null bad then TIO.putStrLn "totality: every step has exactly one definition"
  else do
    mapM_ (\(loc, b) -> TIO.putStrLn ("ORPHAN " <> loc <> ": " <> b)) bad
    exitFailure
```
(`featureFilesLocal` — copy the 8-line `featureFiles` walker from Main; it is 8 lines, duplication beats a dependency cycle, note it in a why-comment.)

- [ ] **Step 4: Run tests green; commit**

```bash
git add contracts/runner
git commit -m "totality check: orphan steps are named, never discovered at runtime"
```

---

### Task 8: Vocabulary drift law — the table in the file must equal the types in use

**Files:**
- Modify: `contracts/runner/src/Vocab.hs`
- Test: `contracts/runner/test/Spec.hs`

**Interfaces:**
- Consumes: `Pattern.usesOf` via `World.defUses`, `Capture.describeUniverse`, `Gherkin.*`.
- Produces: `expectedVocab :: [StepDef] -> Feature -> [(Text, Text)]`, `vocabDir :: [StepDef] -> FilePath -> Bool -> IO ()` (verify, or `--write` to regenerate the block in place via `renderFeature`).

- [ ] **Step 1: Failing test**

```haskell
  describe "vocabulary drift" $ do
    it "derives the expected table from the steps' capture types" $ do
      let Right f = parseFeature "t.feature" $ T.unlines
            [ "Feature: t"
            , "  Scenario: s"
            , "    When I render pieces fills at year -1405 in style canaan" ]
      lookup "pieces" (Vocab.expectedVocab allSteps f)
        `shouldBe` Just "any of: borders, chrome, claims, fills, ground, journeys, labels, markers, veil, water"
      lookup "year" (Vocab.expectedVocab allSteps f)
        `shouldBe` Just "whole number from -4004 to 100 (negative means BC; -1405 is 1405 BC)"
    it "flags drift when the file's table disagrees" $ do
      let Right f = parseFeature "t.feature" $ T.unlines
            [ "Feature: t"
            , "  Vocabulary:"
            , "    | pieces | some old lie |"
            , "  Scenario: s"
            , "    When I render pieces fills at year -1405 in style canaan" ]
      Vocab.drift allSteps f `shouldSatisfy` (not . null)
```
(Note the enumerated list is SORTED — `universe` for `Piece` must emit sorted values; adjust Task 3's instance to `map pieceText (sortOn pieceText [minBound..maxBound])` if the derived order differs. The test states the law; make the code obey it.)

- [ ] **Step 2: Run, verify FAIL.**

- [ ] **Step 3: Implement `Vocab.hs`**

```haskell
module Vocab where

import Data.List (nub, sort)
import Data.Text (Text)
import qualified Data.Text as T
import qualified Data.Text.IO as TIO
import Capture (describeUniverse)
import Gherkin.Ast
import Gherkin.Parse (parseFeature)
import Gherkin.Render (renderFeature)
import System.Exit (exitFailure)
import World (StepDef (..))

-- the union of universes contributed by the definitions this feature's
-- steps actually match — THE source the table must equal
expectedVocab :: [StepDef] -> Feature -> [(Text, Text)]
expectedVocab defs f = nub
  [ (name, describeUniverse u)
  | sc <- ftScenarios f, st <- scSteps sc
  , StepDef k _ us m <- defs, k == stepKw st, Just _ <- [m (stepBody st)]
  , (name, u) <- us, name /= "text", name /= "name" ]  -- free-text/bind carry no vocabulary

drift :: [StepDef] -> Feature -> [Text]
drift defs f =
  let want = sort (expectedVocab defs f)
      have = sort (ftVocab f)
  in [ "vocabulary drift: table says " <> T.pack (show have)
       <> " but the types say " <> T.pack (show want) | want /= have ]

vocabDir :: [StepDef] -> FilePath -> Bool -> IO ()
vocabDir defs dir writeMode = do
  files <- featureFilesLocal dir
  problems <- fmap concat . mapM (one) $ files
  if null problems
    then TIO.putStrLn "vocabulary: every table matches its types"
    else if writeMode
      then TIO.putStrLn "vocabulary: rewritten"
      else mapM_ TIO.putStrLn problems >> exitFailure
  where
    one p = do
      src <- TIO.readFile p
      case parseFeature p src of
        Left e -> pure [T.pack p <> ": " <> e]
        Right f
          | writeMode -> do
              TIO.writeFile p (renderFeature f { ftVocab = expectedVocab defs f })
              pure []
          | otherwise -> pure (map ((T.pack p <> ": ") <>) (drift defs f))
```
(again the 8-line `featureFilesLocal` walker, with its why-comment.)

- [ ] **Step 4: Run tests green; commit**

```bash
git add contracts/runner
git commit -m "vocabulary drift law: the dummy-readable table is proven against the types"
```

---

### Task 9: @property holes — scenarios as QuickCheck properties

**Files:**
- Modify: `contracts/runner/src/Prop.hs`
- Modify: `contracts/runner/src/Check.hs` (wire `dehole` to the example substitution)
- Test: `contracts/runner/test/Spec.hs`

**Interfaces:**
- Consumes: `Run.runScenario`, `Capture` (renderCap laws), `World`.
- Produces:
```haskell
data SomeHole = forall a. FromCapture a => SomeHole (Gen a)
holeRegistry :: Map Text SomeHole      -- "someYear" → Year gen; "someA"/"someB" → PieceSet gen
substitute :: Map Text Text -> Scenario -> Scenario    -- <hole> → rendered value
substituteExamples :: Text -> Text                     -- for the totality check
runWithProperties :: [StepDef] -> World -> Int -> [FilePath] -> IO [ScenarioResult]
  -- plain scenarios once; @property scenarios: N generated bindings, first
  -- failure reported with its (shrunk-enough) binding values in the message
```
- Hole names are an EXPLICIT registry (typed, no inference): `someYear :: Year`, `someA`, `someB`, `somePieces :: PieceSet`, `someStyle :: StyleName`. An unregistered `<hole>` is an orphan in `check` and a `Failed` naming the hole in `run`.

- [ ] **Step 1: Failing tests**

```haskell
  describe "@property scenarios" $ do
    it "substitutes holes and runs N times, all green on a law that holds" $ do
      let feat = T.unlines
            [ "Feature: t"
            , "  @property"
            , "  Scenario: determinism"
            , "    When I render pieces <somePieces> at year <someYear> in style canaan as a"
            , "    And I render pieces <somePieces> at year <someYear> in style canaan as b"
            , "    Then a equals b" ]
          Right f = parseFeature "t.feature" feat
          fake url = pure (Right (bs, fromJust (A.decodeStrict bs)))
            where bs = TE.encodeUtf8 ("{\"echo\":\"" <> url <> "\"}")
          w = World "http://x" fake "" mempty False
      rs <- Prop.runScenarioProperty allSteps w 25 (head (ftScenarios f))
      rs `shouldBe` Passed
    it "reports the failing binding when the law breaks" $ do
      let feat = T.unlines
            [ "Feature: t"
            , "  @property"
            , "  Scenario: falsifiable"
            , "    When I GET /api/echo?y=<someYear>"
            , "    Then the response field neverThere equals nope" ]
          Right f = parseFeature "t.feature" feat
          fake _ = pure (Right ("{}", A.object []))
          w = World "http://x" fake "" mempty False
      v <- Prop.runScenarioProperty allSteps w 25 (head (ftScenarios f))
      case v of
        Failed e -> e `shouldSatisfy` T.isInfixOf "someYear ="
        _ -> expectationFailure "law should have failed with its binding"
```

- [ ] **Step 2: Run, verify FAIL.**

- [ ] **Step 3: Implement `Prop.hs`**

```haskell
{-# LANGUAGE ExistentialQuantification #-}
module Prop where

import Data.IORef
import Data.Map.Strict (Map)
import qualified Data.Map.Strict as Map
import Data.Text (Text)
import qualified Data.Text as T
import qualified Data.Text.IO as TIO
import Test.QuickCheck
import Test.QuickCheck.Gen (Gen (..))
import Test.QuickCheck.Random (mkQCGen)
import Capture
import Gherkin.Ast
import Gherkin.Parse (parseFeature)
import Run
import World (StepDef, World)

data SomeHole = forall a. FromCapture a => SomeHole (Gen a)

genPieces :: Gen PieceSet
genPieces = PieceSet . Data.Set.fromList <$> sublistOf [minBound .. maxBound]
-- (import qualified Data.Set; a possibly-empty subset — the empty scene
-- is the monoid identity and MUST be generated)

genYear :: Gen Year
genYear = Year <$> chooseInt (-4004, 100)

holeRegistry :: Map Text SomeHole
holeRegistry = Map.fromList
  [ ("someYear",   SomeHole genYear)
  , ("somePieces", SomeHole genPieces)
  , ("someA",      SomeHole genPieces)
  , ("someB",      SomeHole genPieces)
  , ("someStyle",  SomeHole (StyleName <$> elements ["canaan", "parchment", "slate"]))
  ]

holesOf :: Scenario -> [Text]
holesOf sc =
  [ h | Step _ b _ <- scSteps sc
      , h <- extract b ]
  where
    extract b = case T.breakOn "<" b of
      (_, rest) | T.null rest -> []
      (_, rest) -> let (h, rest') = T.breakOn ">" (T.drop 1 rest)
                   in h : if T.null rest' then [] else extract (T.drop 1 rest')

substitute :: Map Text Text -> Scenario -> Scenario
substitute env sc = sc { scSteps = map sub (scSteps sc) }
  where
    sub st = st { stepBody = Map.foldrWithKey
                    (\h v b -> T.replace ("<" <> h <> ">") v b)
                    (stepBody st) env }

-- deterministic seed: the diagnosis must be reproducible run to run
runScenarioProperty :: [StepDef] -> World -> Int -> Scenario -> IO Verdict
runScenarioProperty defs w n sc =
  case traverse (\h -> (,) h <$> Map.lookup h holeRegistry) (dedup (holesOf sc)) of
    Nothing -> pure (Failed ("unregistered property hole in: " <> scName sc))
    Just holes -> loop 0
      where
        loop i | i >= n = pure Passed
        loop i = do
          let env = Map.fromList
                [ (h, renderIt s i) | (h, s) <- holes ]
          v <- runScenario defs w (substitute env sc)
          case v of
            Passed -> loop (i + 1)
            Failed e -> pure . Failed $
              e <> "\n    with " <> T.intercalate ", "
                [ h <> " = " <> v' | (h, v') <- Map.toList env ]
            s -> pure s
        renderIt (SomeHole g) i =
          renderCap (unGen g (mkQCGen (fromIntegral i * 7919)) (min 30 (i + 3)))
  where dedup = foldr (\x acc -> if x `elem` acc then acc else x : acc) []

substituteExamples :: Text -> Text
substituteExamples b = foldr rep b (Map.toList holeRegistry)
  where
    rep (h, SomeHole g) = T.replace ("<" <> h <> ">")
      (renderCap (unGen g (mkQCGen 1) 3))

runWithProperties :: [StepDef] -> World -> Int -> [FilePath] -> IO [ScenarioResult]
runWithProperties defs w n files = fmap concat . mapM one $ files
  where
    one p = do
      src <- TIO.readFile p
      case parseFeature p src of
        Left e  -> pure [ScenarioResult (T.pack p) "PARSE" [] (Failed e)]
        Right f -> mapM (run1 (ftTitle f)) (ftScenarios f)
    run1 ft sc
      | Tag "property" `elem` scTags sc =
          ScenarioResult ft (scName sc) (scTags sc) <$> runScenarioProperty defs w n sc
      | otherwise =
          ScenarioResult ft (scName sc) (scTags sc) <$> runScenario defs w sc
```

Then wire Task 7's `dehole = id` to `dehole = Prop.substituteExamples` (move `substituteExamples` import into `Check.hs`; if that creates an import cycle, move `substituteExamples` + `holeRegistry` into `Capture`-adjacent module `Prop` and import `Prop` from `Check` — `Prop` imports `World` but not `Check`, so the direction is clean).

- [ ] **Step 4: Run all tests green.** `cabal test 2>&1 | tail -5`

- [ ] **Step 5: Commit**

```bash
git add contracts/runner
git commit -m "@property holes: registry-typed, deterministic seeds, failing bindings named"
```

---

### Task 10: The map-api v0.1 feature suite — meta and fact tier

**Files:**
- Create: `contracts/map-api/meta/contract.feature`
- Create: `contracts/map-api/fact/subjects.feature`
- Create: `contracts/map-api/fact/changes.feature`
- Create: `contracts/map-api/fact/census.feature`
- Create: `contracts/map-api/fixtures/` (empty dir with `.gitkeep`; blessed in Task 14)

**Interfaces:**
- Consumes: the step library sketches from Task 5 (every step below must match one; `check` proves it).
- Produces: the fact-tier half of contract v0.1.

- [ ] **Step 1: Write the feature files exactly**

`contracts/map-api/meta/contract.feature`:
```gherkin
Feature: the contract endpoint — a server declares what it speaks
  A consumer refuses version skew instead of discovering it. The whole
  body is pinned; the graph pin varies with the compiled canon by
  design, so it is masked HERE, visibly, and shape-checked instead.

  Scenario: the contract declaration is exactly its blessed body
    When I GET /api/contract
    Then the response equals fixture "contract" masking graphPin as sixteen hex characters
```

`contracts/map-api/fact/subjects.feature`:
```gherkin
Feature: subjects — what can be asked about at a moment
  The picker's feed: enumeration precedes every question. The WHOLE
  list is the answer — a subjects feed that also carried a leaked era's
  ghost would pass any existential poke; it cannot pass the fixture.

  Scenario: the twelve tribes era, whole
    When I GET /api/subjects?year=-1405
    Then the response equals fixture "subjects-1405"

  Scenario: the tetrarchies era, whole
    When I GET /api/subjects?year=59
    Then the response equals fixture "subjects-59"

  @property
  Scenario: subjects are deterministic at any year
    When I GET /api/subjects?year=<someYear> as first
    And I GET /api/subjects?year=<someYear> as second
    Then first equals second
```

`contracts/map-api/fact/changes.feature`:
```gherkin
Feature: changes — the narrative between two instants
  The scrubber's stops: the piecewise-constant timeline made visible.

  Scenario: the conquest is a change the timeline knows
    When I GET /api/changes?from=-1407&to=-1405
    Then the response equals fixture "changes-conquest"

  Scenario: an empty span has no changes
    When I GET /api/changes?from=-3000&to=-3000
    Then the response equals fixture "changes-empty"
```

`contracts/map-api/fact/census.feature`:
```gherkin
Feature: the census — every disposition, queryable
  The instrument whose absence let a phantom state ship: for any year,
  every standing feature's tenure in one table, pinned WHOLE. The
  promise-as-claim at 1050 BC and Judea-as-held at AD 59 live inside
  these fixtures — and so does everything else standing in those
  years, which is the point: a leaked feature fails the fixture.

  Scenario: the whole census at 1050 BC
    When I GET /api/census?year=-1050
    Then the response equals fixture "census-1050"

  Scenario: the whole census at AD 59
    When I GET /api/census?year=59
    Then the response equals fixture "census-59"

  Scenario: the whole census at the conquest
    When I GET /api/census?year=-1405
    Then the response equals fixture "census-1405"

  @property
  Scenario: the census is deterministic at any year
    When I GET /api/census?year=<someYear> as first
    And I GET /api/census?year=<someYear> as second
    Then first equals second
```

- [ ] **Step 2: Add the two missing generic steps (GET-as binding; whole-body-with-mask)**

WHY these and only these: the whole-body law forbids existential pokes, so the
fact tier needs exactly (a) binding a response under a name and (b) fixture
equality where declared don't-care fields are masked — with the mask VISIBLE
in the scenario text and shape-checked, never silently dropped.

Failing tests first (append to `Spec.hs`):
```haskell
  describe "fact-tier steps" $ do
    it "GET-as binds under a name" $ do
      let fake _ = pure (Right ("[]", fromJust (A.decodeStrict "[]")))
          w = World "http://x" fake "" mempty False
      Just get <- pure (firstMatch When "I GET /api/subjects?year=-1405 as first")
      Right w1 <- get w
      Map.member "first" (bound w1) `shouldBe` True
    it "masked fixture equality: body pinned whole, mask shape-checked" $ do
      let o = "{\"version\":\"0.1.0\",\"graphPin\":\"0123456789abcdef\"}"
          fake _ = pure (Right (o, fromJust (A.decodeStrict o)))
          w = World "http://x" fake "test/fixtures" mempty False
      -- test/fixtures/contract.json holds {"version":"0.1.0","graphPin":"MASKED"}
      Just get <- pure (firstMatch When "I GET /api/contract")
      Right w1 <- get w
      Just chk <- pure (firstMatch Then
        "the response equals fixture \"contract\" masking graphPin as sixteen hex characters")
      r <- chk w1
      r `shouldSatisfy` isRight
    it "a masked field with the WRONG shape still fails" $ do
      let o = "{\"version\":\"0.1.0\",\"graphPin\":\"nope\"}"
          fake _ = pure (Right (o, fromJust (A.decodeStrict o)))
          w = World "http://x" fake "test/fixtures" mempty False
      Just get <- pure (firstMatch When "I GET /api/contract")
      Right w1 <- get w
      Just chk <- pure (firstMatch Then
        "the response equals fixture \"contract\" masking graphPin as sixteen hex characters")
      r <- chk w1
      r `shouldSatisfy` isLeft
    it "an unmasked difference anywhere in the body fails" $ do
      let o = "{\"version\":\"9.9.9\",\"graphPin\":\"0123456789abcdef\"}"
          fake _ = pure (Right (o, fromJust (A.decodeStrict o)))
          w = World "http://x" fake "test/fixtures" mempty False
      Just get <- pure (firstMatch When "I GET /api/contract")
      Right w1 <- get w
      Just chk <- pure (firstMatch Then
        "the response equals fixture \"contract\" masking graphPin as sixteen hex characters")
      r <- chk w1
      r `shouldSatisfy` isLeft
```
(also create `contracts/runner/test/fixtures/contract.json` with exactly
`{"version":"0.1.0","graphPin":"MASKED"}`.)

Then implement, appending to `allSteps` in `Steps.hs`:
```haskell
  , mkStep When (lit "I GET " *> ((,) <$> capUntil @FixtureRefFreeText " as "
                                      <*> capRest @BindName)) $
      \(FixtureRefFreeText path, BindName n) w -> do
        r <- getUrl (baseUrl w <> path) w
        pure (r >>= bindLast n)
  , mkStep Then (lit "the response equals fixture " *> ((,,) <$> capUntil @FixtureRef " masking "
                                                             <*> capUntil @FixtureRefFreeText " as "
                                                             <*> capRest @MaskShape)) $
      \(FixtureRef f, FixtureRefFreeText masked, shape) w -> do
        fx <- loadFixture w f
        pure $ do
          expected <- fx
          (_, actual) <- maybe (Left "no response") Right (Map.lookup "_last" (bound w))
          -- 1. the masked field must exist and satisfy its declared shape
          mv <- field masked actual
          checkShape shape masked mv
          -- 2. the WHOLE remaining body must equal the fixture, where the
          --    fixture writes the literal string "MASKED" at the masked key
          let actual' = setField masked (String "MASKED") actual
          if actual' == expected then Right w
          else Left ("body differs from fixture " <> f <> " outside the mask")
```
with, at the bottom of `Steps.hs`:
```haskell
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
```

- [ ] **Step 3: Prove the suite is total and undrifted**

Run from `contracts/runner/`:
```
cabal run contract-runner -- check ../map-api
cabal run contract-runner -- vocab ../map-api --write
cabal run contract-runner -- vocab ../map-api
```
Expected: `check` reports totality (fix any orphan by adjusting feature wording to the sketches, or adding the missing step WITH a test); `vocab --write` stamps the tables; the verify pass is then clean.

- [ ] **Step 4: Commit**

```bash
git add contracts/map-api contracts/runner
git commit -m "map-api v0.1 fact+meta features: whole bodies pinned, masks declared in-scenario"
```

---

### Task 11: The scene algebra features — where the diagnosis lives

**Files:**
- Create: `contracts/map-api/scene/scene.feature`
- Create: `contracts/map-api/scene/resources.feature`

**Interfaces:**
- Consumes: scene steps from Task 5, property holes from Task 9.
- Produces: the algebra half of v0.1. `@target` tags mark scenarios expected red against today's server — they ARE the diagnosis.

- [ ] **Step 1: Write `scene.feature` exactly**

```gherkin
Feature: the scene — a picture composed from pieces
  The scene is a monoid over pieces: any subset renders, absence is the
  identity, and adding a piece back changes nothing else. In v0.1 only
  ground, water, labels, and journeys are toggleable on the wire; the
  rest are always present — a wart this contract records rather than
  hides, retired when the pieces parameter lands.

  Scenario: the twelve tribes scene, whole
    When I render pieces ground, water, fills, borders, labels, journeys at year -1405 in style canaan
    Then the response equals fixture "scene-1405-full"

  Scenario: a scene with no labels is still a scene — pinned whole
    When I render pieces ground, water, fills, borders, journeys at year -1405 in style canaan
    Then the response equals fixture "scene-1405-nolabels"

  Scenario: omission is subtractive, not destructive
    When I render pieces ground, water, fills, borders, labels, journeys at year -1405 in style canaan as full
    And I render pieces fills, borders, labels, journeys at year -1405 in style canaan as noWater
    Then noWater's resources are a subset of full's resources

  Scenario: rendering is deterministic
    When I render pieces ground, water, fills, borders, labels, journeys at year -1405 in style canaan as first
    And I render pieces ground, water, fills, borders, labels, journeys at year -1405 in style canaan as second
    Then first equals second

  @property
  Scenario: determinism holds for any piece subset at any year
    When I render pieces <somePieces> at year <someYear> in style canaan as first
    And I render pieces <somePieces> at year <someYear> in style canaan as second
    Then first equals second

  @target
  Scenario: every manifest entry names its piece
    When I render pieces ground, water, fills, borders, labels, journeys at year -1405 in style canaan
    Then every feature entry carries a piece field

  @target @property
  Scenario: composition — pieces render separately and combine to the whole
    When I render pieces <someA> at year <someYear> in style canaan as sceneA
    And I render pieces <someB> at year <someYear> in style canaan as sceneB
    Then combining sceneA and sceneB equals rendering <someA> plus <someB>

  @target
  Scenario: dress-locality — restyling one piece leaves the others untouched
    When I render pieces ground, fills at year -1405 in style canaan as dressed
    And I render pieces ground, fills at year -1405 in style slate as redressed
    Then dressed and redressed differ only in dress, never in geometry
```

- [ ] **Step 2: Write `resources.feature` exactly**

```gherkin
Feature: resources — geometry by content address
  An id IS its bytes: the same id can never serve two payloads, and a
  batch is exactly its singles.

  Scenario: the same id fetched twice is byte-identical
    When I render pieces fills, borders at year -1405 in style canaan as scene
    Then fetching scene's first resource twice yields identical bytes

  @target
  Scenario: a batch equals its singles
    When I render pieces fills, borders at year -1405 in style canaan as scene
    Then fetching scene's first two resources as a batch equals fetching them singly
```

- [ ] **Step 3: Add the four remaining steps (test-first, same pattern as Task 10)**

Tests: pure-transport tests in `Spec.hs` exercising each step's happy path and one failure path (write them in the style of Task 10 Step 2 — for the `@target` steps the test asserts the step MATCHES and produces a `Left` against a manifest lacking piece fields; expected-red needs the step to exist and honestly fail, not to be an orphan). Implementations appended to `allSteps`:

```haskell
  , mkStep Then (lit "every feature entry carries a piece field") $ \() w ->
      pure $ case Map.lookup "_last" (bound w) of
        Nothing -> Left "no response"
        Just (_, v) -> case field "features" v of
          Right (Array fs)
            | V.all (\f -> either (const False) (const True) (field "piece" f)) fs -> Right w
            | otherwise -> Left "manifest entries carry no piece attribution (v0.1 wart)"
          other -> Left (T.pack (show (() <$ other)))
  , mkStep Then (lit "combining " *> ((,,,) <$> capUntil @BindName " and "
                                            <*> capUntil @BindName " equals rendering "
                                            <*> capUntil @PieceSet " plus "
                                            <*> capRest @PieceSet)) $
      \(BindName a, BindName b, psA, psB) w -> do
        let PieceSet sa = psA; PieceSet sb = psB
        r <- getUrl (sceneUrl (baseUrl w) (PieceSet (Set.union sa sb)) (Year (-1405)) (StyleName "canaan")) w
        -- v0.1 has no server-side combine; the law is checked as:
        -- union render's resource set == union of the parts' resource sets
        pure $ do
          w' <- r
          both <- resourceSet "_last" w'
          ra <- resourceSet a w'
          rb <- resourceSet b w'
          if both == Set.union ra rb then Right w'
          else Left "union scene is not the union of its parts' resources"
  , mkStep Then (lit "" *> ((,) <$> capUntil @BindName " and "
                                <*> (capUntil @BindName " differ only in dress, never in geometry"))) $
      \(BindName a, BindName b) w ->
        pure $ do
          ra <- resourceSet a w
          rb <- resourceSet b w
          -- geometry is content-addressed: identical geometry ⇒ identical ids.
          -- dress rides styles, not payload ids ⇒ the id sets must be EQUAL.
          if ra == rb then Right w
          else Left "restyle changed geometry ids: dress is not local"
  , mkStep Then (lit "fetching " *> (capUntil @BindName "'s first resource twice yields identical bytes")) $
      \(BindName a) w -> do
        firstId <- pure (firstResourceId a w)
        case firstId of
          Left e -> pure (Left e)
          Right rid -> do
            r1 <- transport w (baseUrl w <> "/api/resource?id=" <> rid)
            r2 <- transport w (baseUrl w <> "/api/resource?id=" <> rid)
            pure $ case (r1, r2) of
              (Right (b1, _), Right (b2, _)) | b1 == b2 -> Right w
              (Right _, Right _) -> Left ("id " <> rid <> " served two different payloads")
              (Left e, _) -> Left e
              (_, Left e) -> Left e
  , mkStep Then (lit "fetching " *> (capUntil @BindName "'s first two resources as a batch equals fetching them singly")) $
      \(BindName a) w -> do
        case twoResourceIds a w of
          Left e -> pure (Left e)
          Right (i1, i2) -> do
            batch <- transport w (baseUrl w <> "/api/resources?ids=" <> i1 <> "," <> i2)
            s1 <- transport w (baseUrl w <> "/api/resource?id=" <> i1)
            s2 <- transport w (baseUrl w <> "/api/resource?id=" <> i2)
            pure $ case (batch, s1, s2) of
              (Right (bb, _), Right (b1, _), Right (b2, _))
                | bb == b1 <> b2 -> Right w
                | otherwise -> Left "batch bytes differ from concatenated singles"
              _ -> Left "resource fetch failed"
```
with helpers `resourceSet`, `firstResourceId`, `twoResourceIds` (extract `resources[].id` from a bound scene; `resourceSet` returns `Set Text`; on missing binding or shape, `Left` with the name). NOTE for the implementer: `/api/resource` and `/api/resources` return binary bodies, not JSON — the shared `transport` decodes JSON and will `Left` on them. Give `World` a second field `transportRaw :: Text -> IO (Either Text ByteString)` (HTTP impl: same as `httpTransport` minus the decode; pure fakes return the bytes), and use it in these two steps. Add it in this task, test-first, updating Task 5's `World` record and both constructors.

- [ ] **Step 4: Totality + vocab again**

```
cabal run contract-runner -- check ../map-api
cabal run contract-runner -- vocab ../map-api --write && cabal run contract-runner -- vocab ../map-api
```
Fix orphans by aligning wording; commit only when both are clean and `cabal test` is green.

- [ ] **Step 5: Commit**

```bash
git add contracts/map-api contracts/runner
git commit -m "scene algebra features: v0.1 truths green-by-design, target laws honest red"
```

---

### Task 12: The atlas-edge CDC suite

**Files:**
- Create: `contracts/atlas-edge/polities.feature`
- Create: `contracts/atlas-edge/narratives.feature`
- Create: `contracts/atlas-edge/events.feature`
- Create: `contracts/atlas-edge/eras.feature`
- Create: `contracts/atlas-edge/landmarks.feature`
- Create: `contracts/atlas-edge/land-mask.feature`
- Create: `contracts/atlas-edge/fixtures/.gitkeep`

**Interfaces:**
- Consumes: the generic steps (Tasks 5/10) plus two new array-shape steps below.
- Produces: the consumer-driven contract for everything `map-compile/src/vendor.rs` parsers require — nothing more (a CDC states the consumer's needs, not the provider's abilities).

- [ ] **Step 1: Write the features exactly — whole-body over the CONSUMED PROJECTION**

The CDC principle under the whole-body law: we pin the ENTIRE projection of
their response onto the fields our parsers read — every value included — and
nothing outside it. They may add fields or reorder freely; they may not move
one coordinate we consume without the fixture saying so.

`polities.feature`:
```gherkin
Feature: polities — the eras of governed ground we vendor
  Our parse_polities reads: per row, id, name, from, to, and rings.
  The consumed projection of the whole book is pinned — all rows, all
  coordinates. A silently moved border fails here before it can move
  a pixel of ours.

  Scenario: the whole polity book, as we consume it
    When I GET /api/polities?from=-4004&to=2000
    Then the consumed projection polities equals fixture "polities-consumed"
```

`narratives.feature`:
```gherkin
Feature: narratives — the journeys we vendor
  Our parse_narratives reads: per row, id, name, color, and ordered legs.

  Scenario: the whole narrative book, as we consume it
    When I GET /api/narratives
    Then the consumed projection narratives equals fixture "narratives-consumed"
```

`events.feature`:
```gherkin
Feature: events — a leg's when, where, and why
  Our parse_event reads: id; optional when (from_year, to_year);
  places; verses.

  Scenario: a known leg event, as we consume it
    When I GET /api/event/ab_haran
    Then the consumed projection event equals fixture "event-ab-haran-consumed"
```

`eras.feature`:
```gherkin
Feature: eras — the named periods that resolve standings
  Our parse_eras reads id and from_year per era; era ids are how
  vendored data declares WHO STANDS WHEN without hardcoded years.

  Scenario: the whole era table, as we consume it
    When I GET /api/eras
    Then the consumed projection eras equals fixture "eras-consumed"
```

`landmarks.feature`:
```gherkin
Feature: landmarks — named waters and places we label by
  Our parse_landmarks reads name and kind per row.

  Scenario: the whole landmark list, as we consume it
    When I GET /api/landmarks
    Then the consumed projection landmarks equals fixture "landmarks-consumed"
```

`land-mask.feature`:
```gherkin
Feature: the land mask — the coastline our partition builds on
  Our parse_land_mask reads the rings, whole.

  Scenario: the whole mask, as we consume it
    When I GET /api/land-mask
    Then the consumed projection land-mask equals fixture "land-mask-consumed"
```

- [ ] **Step 2: Add the projection step and the typed projection registry (test-first, pure transports)**

A PROJECTION is a declared field-tree: applied to their response, it keeps
exactly the consumed fields (recursively, through arrays) and drops the rest.
The projected value is then compared WHOLE against the fixture. Failing test
first (Task-10 style, pure transport): project a two-field-plus-extras object
through the `eras` projection and assert equality with the filtered fixture;
a second test asserts a CHANGED consumed value fails; a third asserts an
ADDED unconsumed field still passes.

In `Steps.hs`:
```haskell
-- WHAT WE CONSUME, as data — one entry per atlas endpoint, mirroring
-- vendor.rs's parsers field for field. Changing a parser without
-- changing its projection here (and re-blessing) fails the CDC suite:
-- the contract and the consumer cannot drift apart.
data Proj = Keep | Fields [(Text, Proj)] | Each Proj

projections :: Map Text Proj
projections = Map.fromList
  [ ("polities",  Fields [("polities", Each (Fields
                    [("id", Keep), ("name", Keep), ("from", Keep), ("to", Keep), ("rings", Keep)]))])
  , ("narratives", Fields [("narratives", Each (Fields
                    [("id", Keep), ("name", Keep), ("color", Keep), ("legs", Keep)]))])
  , ("event",     Fields [("id", Keep), ("when", Fields [("from_year", Keep), ("to_year", Keep)])
                         ,("places", Keep), ("verses", Keep)])
  , ("eras",      Fields [("eras", Each (Fields [("id", Keep), ("from_year", Keep)]))])
  , ("landmarks", Fields [("landmarks", Each (Fields [("name", Keep), ("kind", Keep)]))])
  , ("land-mask", Fields [("rings", Keep)])
  ]

project :: Proj -> Value -> Value
project Keep v = v
project (Each p) (Array a) = Array (fmap (project p) a)
project (Each _) v = v
project (Fields fs) (Object o) = Object (KM.fromList
  [ (K.fromText k, project p fv)
  | (k, p) <- fs, Just fv <- [KM.lookup (K.fromText k) o] ])
project (Fields _) v = v

  -- appended to allSteps:
  , mkStep Then (lit "the consumed projection " *> ((,) <$> capUntil @ProjName " equals fixture "
                                                        <*> capRest @FixtureRef)) $
      \(ProjName pn, FixtureRef f) w -> do
        fx <- loadFixture w f
        pure $ do
          expected <- fx
          (_, actual) <- maybe (Left "no response") Right (Map.lookup "_last" (bound w))
          p <- maybe (Left ("unknown projection " <> pn)) Right (Map.lookup pn projections)
          let got = project p actual
          if got == expected then Right w
          else Left ("consumed projection " <> pn <> " differs from fixture " <> f)
```
with the projection-name capture (its universe IS the registry — the
Vocabulary block lists every projection automatically):
```haskell
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
```
Blessing writes the PROJECTED value as the fixture (adjust `blessOrCompare`
usage: this step, in bless mode, writes `project p actual`, not the raw body).
IMPORTANT provider note for the implementer: verify the actual atlas response
shapes against the committed live-captured fixtures in
`crates/map-compile/fixtures/` (`polities.json`, `narratives.json`, etc.)
BEFORE finalizing the projection trees — if `narratives.json`'s top level is
a bare array rather than `{"narratives": [...]}`, the projection is
`Each (Fields …)` at the top. `vendor.rs`'s parsers are the authority; the
projections must state what they ACTUALLY read, field for field.

- [ ] **Step 3: Totality + vocab for atlas-edge; runner tests green**

```
cabal run contract-runner -- check ../atlas-edge
cabal run contract-runner -- vocab ../atlas-edge --write && cabal run contract-runner -- vocab ../atlas-edge
cabal test 2>&1 | tail -3
```

- [ ] **Step 4: Commit**

```bash
git add contracts/atlas-edge contracts/runner
git commit -m "atlas-edge CDC: six features stating exactly what our parsers need"
```

---

### Task 13: Rust — the census function and the two routes

**Files:**
- Modify: `crates/map-canon/src/lib.rs` (add `CensusRow`, `census`)
- Test: `crates/map-canon/src/tests.rs`
- Modify: `crates/map-viewer/src/lib.rs` (routes `/api/contract`, `/api/census`; `App` gains `world_etag` if it lacks a field carrying the startup ETag — check near the `world_etag` computation around line 251 first)

**Interfaces:**
- Consumes: existing `CanonStore`, `Tenure`, `LayerKind`, `World::state_at`.
- Produces:
```rust
pub struct CensusRow { pub entity: String, pub name: String,
                       pub layer: &'static str, pub kind: &'static str,
                       pub tenure: &'static str }
pub fn census(store: &CanonStore, at: &Timestamp) -> Vec<CensusRow>  // sorted (layer, entity)
```
Wire shapes: `/api/census?year=Y` → JSON array of `{entity, name, layer, kind, tenure}`; `/api/contract` → `{"version": "<contracts/VERSION>", "lawsVersion": "0.0-preledger", "graphPin": "<16 hex>"}`.

- [ ] **Step 1: Failing Rust test for the census function**

Append to `crates/map-canon/src/tests.rs`:
```rust
/// THE CENSUS LAW: for any instant, every standing feature appears in
/// the table exactly once, with its tenure — the queryable image of
/// the disposition function. (Stage 0 serves the baked tenure; the
/// ledger stages make the whole derivation queryable.)
#[test]
fn the_census_is_total_and_sorted() {
    let mut store = CanonStore::default();
    let held = area(&mut store, "egypt", square(25.0, 26.0, 8.0));
    let ring = store.insert_border(square(30.0, 34.0, 4.0));
    let claim = store.insert_feature(Feature::Area(Area {
        entity: entity("promise"),
        name: "a promise".to_string(),
        rings: BTreeSet::from([ring]),
        holes: BTreeSet::new(),
        tenure: Tenure::Claimed,
    }));
    let sid = store.insert_snapshot(Snapshot { features: BTreeSet::from([held, claim]) });
    let mut world = World::default();
    world.insert(ts(-1450), sid).unwrap();
    store.set_layer(LayerKind::ScriptureClaims, world);

    let rows = census(&store, &ts(-1000));
    assert_eq!(rows.len(), 2, "every standing feature, exactly once");
    let promise = rows.iter().find(|r| r.entity == "promise").expect("the claim is a row");
    assert_eq!(promise.tenure, "claimed");
    let egypt = rows.iter().find(|r| r.entity == "egypt").expect("held ground is a row");
    assert_eq!(egypt.tenure, "held");
    let mut sorted = rows.iter().map(|r| (r.layer, r.entity.clone())).collect::<Vec<_>>();
    sorted.sort();
    assert_eq!(sorted, rows.iter().map(|r| (r.layer, r.entity.clone())).collect::<Vec<_>>(),
               "the table is sorted (layer, entity) — deterministic wire bytes");
    assert!(census(&store, &ts(-2000)).is_empty(), "before the first moment: empty, not error");
}
```

- [ ] **Step 2: Run, verify FAIL** — `cargo test -p map-canon 2>&1 | grep -E "^error|census" | head -5`: `census` not found.

- [ ] **Step 3: Implement in `crates/map-canon/src/lib.rs`**

```rust
/// One row of THE CENSUS: the queryable image of every disposition at
/// an instant — the instrument that makes a policy change reviewable
/// as a table diff before any pixel moves.
pub struct CensusRow {
    pub entity: String,
    pub name: String,
    pub layer: &'static str,
    pub kind: &'static str,
    pub tenure: &'static str,
}

pub fn census(store: &CanonStore, at: &Timestamp) -> Vec<CensusRow> {
    let layer_name = |l: &LayerKind| -> &'static str {
        match l {
            LayerKind::Territory => "territory",
            LayerKind::ScriptureClaims => "scripture-claims",
            LayerKind::Background => "background",
            LayerKind::Water => "water",
            LayerKind::Relief => "relief",
            LayerKind::Journeys => "journeys",
        }
    };
    let mut rows = Vec::new();
    for (lk, world) in store.layers() {
        let Some(sid) = world.state_at(at) else { continue };
        for fid in &store.snapshots()[&sid].features {
            let Some(f) = store.features().get(fid) else { continue };
            let (entity, name, kind, tenure) = match f {
                Feature::Area(a) => (
                    a.entity.0.clone(), a.name.clone(), "area",
                    match a.tenure { Tenure::Held => "held", Tenure::Claimed => "claimed" },
                ),
                Feature::Line(l) => (l.entity.0.clone(), l.name.clone(), "line", "held"),
                Feature::Way(r) => (r.entity.0.clone(), r.name.clone(), "way", "held"),
                Feature::Point(p) => (p.entity.0.clone(), p.name.clone(), "point", "held"),
                Feature::Memory(m) => (m.entity.0.clone(), m.name.clone(), "memory", "held"),
            };
            rows.push(CensusRow { entity, name, layer: layer_name(lk), kind, tenure });
        }
    }
    rows.sort_by(|a, b| (a.layer, &a.entity).cmp(&(b.layer, &b.entity)));
    rows
}
```
(If `LayerKind` has variants beyond these six, the match will fail to compile — extend it with the real set; the compiler is the reviewer.)

- [ ] **Step 4: Run map-canon tests green** — `cargo test -p map-canon 2>&1 | grep "test result"`.

- [ ] **Step 5: Add the routes in `crates/map-viewer/src/lib.rs`**

In `route_text`'s match, alongside the existing `"/api/meta"` arm:
```rust
        "/api/contract" => {
            // the server declares what contract it speaks; consumers
            // refuse skew instead of discovering it (spec §4)
            let version = include_str!("../../../contracts/VERSION").trim();
            (200, "application/json",
             format!("{{\"version\":\"{}\",\"lawsVersion\":\"0.0-preledger\",\"graphPin\":\"{:016x}\"}}",
                     version, app.world_etag),
             Vec::new())
        }
        "/api/census" => {
            let Some(year) = p.year("year") else { return bad("year required") };
            let rows = map_canon::census(&app.canon, &year);
            let mut s = String::from("[");
            for (i, r) in rows.iter().enumerate() {
                if i > 0 { s.push(','); }
                s.push_str(&format!(
                    "{{\"entity\":{},\"name\":{},\"layer\":\"{}\",\"kind\":\"{}\",\"tenure\":\"{}\"}}",
                    serde_json::json!(r.entity), serde_json::json!(r.name),
                    r.layer, r.kind, r.tenure));
            }
            s.push(']');
            (200, "application/json", s, Vec::new())
        }
```
Adaptation notes for the implementer (verify against the real file, do not guess): (1) `app.world_etag` — the ETag u64 is computed at startup around line 251; if it is not already stored on `App`, add `pub world_etag: u64` to the struct and set it where computed. (2) `app.canon` — find how existing routes reach the `CanonStore` (the provider holds it; if the store is not directly on `App`, expose `pub fn store(&self) -> &CanonStore` on the provider — read-only accessor, one line). (3) `p.year("year")` — reuse the same year-param helper `build_query` uses; if its name differs, match it.

- [ ] **Step 6: Rebuild, restart detached, smoke both routes**

```powershell
Get-Process map-viewer -ErrorAction SilentlyContinue | Stop-Process -Force -Confirm:$false
cargo build --release
Start-Process -FilePath "C:\Users\donov\Documents\the-best-maps-ever\target\release\map-viewer.exe" -WorkingDirectory "C:\Users\donov\Documents\the-best-maps-ever"
```
then:
```bash
curl -s http://127.0.0.1:8090/api/contract
curl -s "http://127.0.0.1:8090/api/census?year=59" | head -c 300
```
Expected: version `0.1.0` + 16-hex pin; census rows including `"name":"Judea","tenure":"held"` somewhere (`grep -o '"name":"Judea"[^}]*'` to confirm).

- [ ] **Step 7: Workspace tests + commit**

```bash
cargo test --workspace 2>&1 | grep -cE "test result: ok"   # expect 19
git add crates/map-canon crates/map-viewer contracts/VERSION
git commit -m "the census and the contract declaration: two read-only routes for v0.1"
```

---

### Task 14: Bless the fixtures against the live server

**Files:**
- Create (generated): `contracts/map-api/fixtures/*.json`, `contracts/atlas-edge/fixtures/*.json`

- [ ] **Step 1: Bless map-api fixtures**

From `contracts/runner/` (server from Task 13 still running):
```
cabal run contract-runner -- run --base-url http://127.0.0.1:8090 ../map-api --bless
```
Expected: fixture-referencing scenarios write `contract.json` (with the masked
field holding the literal `"MASKED"`), `subjects-1405.json`, `subjects-59.json`,
`changes-conquest.json`, `changes-empty.json`, `census-1050.json`,
`census-59.json`, `census-1405.json`, `scene-1405-full.json`,
`scene-1405-nolabels.json`; the table prints (reds among `@target` are fine and
expected here). Bless mode for the MASKED step writes the actual body with the
masked field replaced by `"MASKED"` — implement that in `blessOrCompare`'s
masked variant, test-first, when wiring Task 10's step.

- [ ] **Step 2: Bless atlas-edge fixtures (atlas API on :8080 must be up; if it is down, SKIP and record that in the diagnosis instead of failing the task)**

```
cabal run contract-runner -- run --base-url http://127.0.0.1:8080 ../atlas-edge --bless
```
Expected: the six `*-consumed.json` projection fixtures (projected values, not
raw bodies).

- [ ] **Step 3: Eyeball every blessed fixture** — open each; confirm it is the real content (a census with hundreds of rows, a conquest change list naming tribes) and not an error body. A blessed error is a lie that will pass forever.

- [ ] **Step 4: Commit**

```bash
git add contracts/map-api/fixtures contracts/atlas-edge/fixtures
git commit -m "bless v0.1 fixtures from the live servers, eyeballed"
```

---

### Task 15: THE DIAGNOSIS — run everything, write the red/green truth

**Files:**
- Create: `docs/notes/2026-09-06-contract-diagnosis.md`

- [ ] **Step 1: Full runs, captured**

```bash
cd contracts/runner
cabal run contract-runner -- check ../map-api    && cabal run contract-runner -- vocab ../map-api
cabal run contract-runner -- check ../atlas-edge && cabal run contract-runner -- vocab ../atlas-edge
cabal run contract-runner -- run --base-url http://127.0.0.1:8090 ../map-api    | tee /tmp/map-api.out
cabal run contract-runner -- run --base-url http://127.0.0.1:8080 ../atlas-edge | tee /tmp/atlas-edge.out
```

- [ ] **Step 2: Write the diagnosis document**

`docs/notes/2026-09-06-contract-diagnosis.md` containing: (1) both verdict tables verbatim; (2) a **ranked list of every non-`@target` red** — each with the failing law, the observed behavior, and which migration stage (spec §5) retires it; (3) the `@target` reds confirmed as the expected gaps (no piece attribution, no server combine, no batch law); (4) any *green `@target`* rows called out (a target already met is information); (5) the atlas-edge verdict — if `:8080` was down, say so and mark provider verification pending.

- [ ] **Step 3: Golden gate + batteries sanity (Stage 0 must not have moved a pixel)**

```bash
cd /c/Users/donov/.claude/jobs/c6946bce/tmp
node ../../../../Documents/the-best-maps-ever/crates/map-viewer/tests/golden.js --check 2>&1 | tail -2
```
Expected: `ALL GOLDEN VIEWS HOLD`. If not, Stage 0 broke a pixel it had no business touching — STOP and investigate before committing.

- [ ] **Step 4: Commit and push**

```bash
git add docs/notes/2026-09-06-contract-diagnosis.md
git commit -m "the diagnosis: contract v0.1 run against the world as it is"
git push
```

- [ ] **Step 5: Report to the owner** — the diagnosis table, the ranked unexpected reds, and the recommendation for what Stage 1 should absorb first.

---

## After This Plan: STOP AND PLAN AGAIN

This plan implements **Stage 0 only**. Stages 1–4 of the spec (entity
registry → ledger → one arrangement → route retirement) are deliberately
NOT planned yet: the owner's intent is that the rest of the spec gets its
own planning pass AFTER this implementation is done, informed by what the
diagnosis actually finds. When Task 15 is complete and reported, do not
continue into Stage 1 work — return to the writing-plans skill with the
spec (`docs/superpowers/specs/2026-09-06-map-api-contract-design.md`),
the diagnosis document, and the owner in the loop. The diagnosis may
reorder or reshape the stages; the plan for them must be written against
that evidence, not against tonight's guesses.

---

## Self-Review (performed at write time)

- **Spec coverage:** runner with three extensions (Tasks 1–9), map-api v0.1 features with laws-as-scenarios (10–11), CDC suite (12), `/api/contract` + `/api/census` (13), diagnosis (15), pre-release VERSION 0.1.0 (Task 1), golden-gate closure (15.3). Vocabulary drift (8) and property fuzzing (9) both land. ✓
- **Placeholder scan:** no TBDs; the two deliberate deferrals (DocString parsing, `transportRaw`) are named with their arrival points. ✓
- **Type consistency:** `FromCapture`/`Universe` names match across Tasks 3/4/8/9; `StepDef`/`mkStep` across 5/6/7/8; `runWithProperties` stub (6) replaced in 9; `Tenure::{Held,Claimed}` matches the canon as committed today; `census` signature identical in test (13.1) and impl (13.3).
- **Whole-body law (owner correction, applied):** no existential or shape-poke assertions remain in any feature — our API pins whole bodies with in-scenario masks; the atlas edge pins whole consumed projections via the typed `Proj` registry mirroring `vendor.rs`. The `labels-are-empty` and `subset` steps from Task 5 survive only where they serve RELATIONAL algebra scenarios; `labels-are-empty` is currently unused by any feature and may be dropped by the implementer along with its test, or kept for the algebra work — either is acceptable, dead orphan DEFINITIONS are not flagged by totality (only orphan steps are). One knowing simplification: Task 11's `combining` step fixes year -1405 for the union render while holes generate `<someYear>` — the implementer must thread the scenario's bound year through `World.bound` (store `"_year"` on render steps and read it in the combine step); noted here so it is built, not discovered.
