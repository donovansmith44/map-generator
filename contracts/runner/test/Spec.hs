{-# OPTIONS_GHC -Wno-orphans #-}
module Main where

import Test.Hspec
import Test.Hspec.QuickCheck (prop)
import Test.QuickCheck
import Gherkin.Ast
import Gherkin.Parse
import Gherkin.Render
import Capture
import Pattern
import World
import Steps
import Run
import qualified Check
import Control.Exception (try)
import Data.Proxy (Proxy (..))
import Data.Either (isLeft, isRight)
import Data.Maybe (fromJust, listToMaybe)
import qualified Data.Aeson as A
import qualified Data.ByteString as BS
import qualified Data.Map.Strict as Map
import qualified Data.Set as Set
import qualified Data.Text as T
import qualified Data.Text.Encoding as TE
import qualified Data.Text.IO as TIO
import GHC.IO.Handle (hDuplicate, hDuplicateTo)
import System.Directory
  (getTemporaryDirectory, createDirectoryIfMissing, removeDirectoryRecursive, removeFile)
import System.Exit (ExitCode)
import System.FilePath ((</>))
import System.IO (stdout, openTempFile, hClose, hFlush, hSetEncoding, utf8)

main :: IO ()
main = hspec $ do
  describe "scaffold" $
    it "keywords enumerate Given/When/Then" $
      [minBound .. maxBound] `shouldBe` [Given, When, Then]

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
          case ftScenarios f of
            [sc] -> do
              scTags sc `shouldBe` [Tag "property"]
              -- And resolves to the PREVIOUS keyword at parse time:
              map stepKw (scSteps sc) `shouldBe` [When, When, Then]
            scs -> expectationFailure ("expected exactly one scenario, got " <> show (length scs))
    it "rejects a malformed vocabulary row instead of silently dropping it" $ do
      let src = T.unlines
            [ "Feature: bad vocab"
            , "  Vocabulary:"
            , "    | pieces | any of | extra |"
            ]
      case parseFeature "badvocab.feature" src of
        Left e -> e `shouldSatisfy` ("badvocab.feature:3" `T.isPrefixOf`)
        Right f -> expectationFailure
          ("expected Left for a 3-column vocabulary row, got Right " <> show f)
    prop "round-trips: parse . render == Right" $ \f ->
      parseFeature "gen.feature" (renderFeature f) === Right f

  describe "capture universes" $ do
    it "every enumerated piece value round-trips" $ do
      case universe (Proxy @Piece) of
        Enumerated vs -> mapM_ (\v -> fmap renderCap (parseCap @Piece v) `shouldBe` Right v) vs
        other -> expectationFailure ("expected an Enumerated universe, got " <> show other)
    it "the piece universe is enumerated in sorted order" $ do
      case universe (Proxy @Piece) of
        Enumerated vs -> vs `shouldBe`
          [ "borders", "chrome", "claims", "fills", "ground"
          , "journeys", "labels", "markers", "veil", "water" ]
        other -> expectationFailure ("expected an Enumerated universe, got " <> show other)
    it "piece sets parse comma-separated, any order, and render sorted" $ do
      (renderCap <$> parseCap @PieceSet "water, ground") `shouldBe` Right "ground, water"
    it "piece sets render in alphabetical order, not declaration order" $ do
      -- Water/Borders/Chrome are declared in that relative order (Water=1,
      -- Borders=3, Chrome=8), so a set derived-Ord (declaration-order)
      -- render would read "water, borders, chrome". Alphabetical is
      -- "borders, chrome, water". These genuinely diverge, unlike the
      -- water/ground pair above (whose two orders happen to coincide),
      -- so this test can actually catch a declaration-order regression.
      let ps = PieceSet (Set.fromList [Water, Borders, Chrome])
      renderCap ps `shouldBe` "borders, chrome, water"
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
    it "the empty piece set renders as none and round-trips" $ do
      renderCap (PieceSet Set.empty) `shouldBe` "none"
      parseCap @PieceSet "none" `shouldBe` Right (PieceSet Set.empty)
      parseCap @PieceSet "   " `shouldBe` Right (PieceSet Set.empty)
    prop "renderCap is a right inverse of parseCap for all piece sets, including empty" $
      \ps -> parseCap @PieceSet (renderCap ps) === Right ps
    it "every style name round-trips" $
      mapM_ (\n -> fmap renderCap (parseCap @StyleName n) `shouldBe` Right n) styleNames

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
    -- R4: capUntil's terminator-not-found error is a LITERAL-class mismatch
    -- (the plan's own comment: "the terminator IS the next literal"), so it
    -- must be classified the same way lit's own mismatch is — by starting
    -- with the exact prefix "expected literal". Task 5's mkStep falls
    -- through to the next step definition on that prefix and reports any
    -- other error. A capture PARSE failure (a value that doesn't parse) is
    -- NOT a literal mismatch and must not carry that prefix, or steps that
    -- should report a bad value would instead silently fall through.
    it "classifies literal-vs-capture failures (R4): terminator-not-found is literal, capture-parse failure is not" $ do
      case matchP p "I render pieces water, ground at yeer -1405" of
        Left e  -> e `shouldSatisfy` T.isPrefixOf "expected literal"
        Right _ -> expectationFailure "matched despite a missing terminator"
      case matchP p "I render pieces water, topografy at year -1405" of
        Left e  -> e `shouldSatisfy` (not . T.isPrefixOf "expected literal")
        Right _ -> expectationFailure "matched garbage"

  describe "world and steps" $ do
    let -- The body echoes the URL it was fetched from (rather than a fixed
        -- string), so any two renders of DIFFERENT URLs produce genuinely
        -- different bound values — needed so the equality step's negative
        -- case (below) actually discriminates instead of trivially passing
        -- against a stub that always returns success.
        fake url = pure (Right (raw, v))
          where raw = TE.encodeUtf8 ("{\"scene\":\"" <> url <> "\",\"labels\":[]}")
                v   = fromJust (A.decodeStrict raw)
        w0 = World "http://x" fake "test/fixtures" mempty False
        sceneVal :: [T.Text] -> A.Value
        sceneVal ids = A.object ["resources" A..= map (\i -> A.object ["id" A..= i]) ids]
        labelsVal :: [T.Text] -> A.Value
        labelsVal ls = A.object ["labels" A..= ls]
    it "sceneUrl maps pieces onto today's toggles" $
      -- Equality against the whole literal URL, not `isInfixOf` on pieces of
      -- it: an isInfixOf-per-flag check would still pass if sceneUrl
      -- duplicated a flag, injected an extra parameter, or reordered the
      -- query string, since none of those change which substrings are
      -- present.
      sceneUrl "http://x" (PieceSet (Set.fromList [Fills, Borders])) (Year (-1405)) (StyleName "canaan")
        `shouldBe` "http://x/api/scene?year=-1405&zoom=90.0000&style=canaan&labels=0&topo=0&journeys=0"
    it "the render step binds a named response" $ do
      let run = fromJust $ firstMatch When
            "I render pieces fills at year -1405 in style canaan as sceneA"
      Right w1 <- run w0
      Map.member "sceneA" (bound w1) `shouldBe` True
    it "the equality step compares two bound scenes as JSON values" $ do
      let run1 = fromJust $ firstMatch When
            "I render pieces fills at year -1405 in style canaan as sceneA"
          run2 = fromJust $ firstMatch When
            "I render pieces fills at year -1405 in style canaan as sceneB"
          run3 = fromJust $ firstMatch Then "sceneA equals sceneB"
      Right w1 <- run1 w0
      Right w2 <- run2 w1
      r <- run3 w2
      r `shouldSatisfy` isRight
    it "the equality step reports Left when the bound scenes come from different URLs" $ do
      -- The negative case the positive test alone can't prove: without it,
      -- a stub "always Right" equality step would also pass the test above.
      let run1 = fromJust $ firstMatch When
            "I render pieces fills at year -1405 in style canaan as sceneA"
          run2 = fromJust $ firstMatch When
            "I render pieces water at year -1300 in style slate as sceneB"
          run3 = fromJust $ firstMatch Then "sceneA equals sceneB"
      Right w1 <- run1 w0
      Right w2 <- run2 w1
      r <- run3 w2
      case r of
        Left e  -> e `shouldSatisfy` T.isInfixOf "differ"
        Right _ -> expectationFailure "expected scenes rendered from different URLs to differ"
    it "the GET step fetches and binds the response under _last" $ do
      let run = fromJust $ firstMatch When "I GET /foo"
      Right w1 <- run w0
      Map.member "_last" (bound w1) `shouldBe` True
    it "the response field step passes when the field matches" $ do
      let getRun = fromJust $ firstMatch When "I GET /foo"
          fieldRun = fromJust $ firstMatch Then "the response field scene equals http://x/foo"
      Right w1 <- getRun w0
      r <- fieldRun w1
      r `shouldSatisfy` isRight
    it "the response field step reports Left naming both the actual and wanted values on a mismatch" $ do
      let getRun = fromJust $ firstMatch When "I GET /foo"
          fieldRun = fromJust $ firstMatch Then "the response field scene equals http://wrong"
      Right w1 <- getRun w0
      r <- fieldRun w1
      case r of
        Left e -> do
          e `shouldSatisfy` T.isInfixOf "wanted"
          e `shouldSatisfy` T.isInfixOf "http://wrong"
        Right _ -> expectationFailure "expected a field mismatch to fail"
    -- Fix 8 (post-Task-7 review): the FEATURE is the law and the step
    -- serves it. The real corpus (map-api's scene.feature) writes this as
    -- "noWater's resources are a subset of full's resources" -- a bind
    -- name on BOTH sides, not a bare name on the right -- so the step's
    -- pattern (and this test) now match that wording exactly.
    it "the subset step passes when one scene's resources are a genuine subset of the other's" $ do
      let wSub = w0 { bound = Map.fromList
            [ ("sceneA", (BS.empty, sceneVal ["r1", "r2"]))
            , ("sceneB", (BS.empty, sceneVal ["r1", "r2", "r3"])) ] }
          run = fromJust $ firstMatch Then "sceneA's resources are a subset of sceneB's resources"
      r <- run wSub
      r `shouldSatisfy` isRight
    it "the subset step reports Left when a resource is genuinely absent from the other scene" $ do
      let wSub = w0 { bound = Map.fromList
            [ ("sceneA", (BS.empty, sceneVal ["r1", "r9"]))
            , ("sceneB", (BS.empty, sceneVal ["r1", "r2", "r3"])) ] }
          run = fromJust $ firstMatch Then "sceneA's resources are a subset of sceneB's resources"
      r <- run wSub
      case r of
        Left e  -> e `shouldSatisfy` T.isInfixOf "absent from"
        Right _ -> expectationFailure "expected r9 (absent from sceneB) to fail the subset check"
    it "the labels-are-empty step passes when the bound scene's labels array is empty" $ do
      let wLab = w0 { bound = Map.fromList [ ("sceneA", (BS.empty, labelsVal [])) ] }
          run = fromJust $ firstMatch Then "sceneA's labels are empty"
      r <- run wLab
      r `shouldSatisfy` isRight
    it "the labels-are-empty step reports Left when labels are present" $ do
      let wLab = w0 { bound = Map.fromList [ ("sceneA", (BS.empty, labelsVal ["x"])) ] }
          run = fromJust $ firstMatch Then "sceneA's labels are empty"
      r <- run wLab
      case r of
        Left e  -> e `shouldSatisfy` T.isInfixOf "has labels"
        Right _ -> expectationFailure "expected non-empty labels to fail"

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
          fake _ = pure (Right ("{\"x\":1}", fromJust (A.decodeStrict "{\"x\":1}")))
          w = World "http://x" fake "test/fixtures" mempty False
      -- (Deviation from the brief's literal `let Right f = ...` / `head`:
      -- both trigger -Wincomplete-uni-patterns / -Wx-partial under this
      -- project's -Wall. Restructured as a case/list-pattern to keep the
      -- same meaning with pristine test output; see task-6-report.md.)
      case parseFeature "t.feature" feat of
        Left e -> expectationFailure (T.unpack e)
        Right f -> do
          [r1, r2] <- mapM (runScenario allSteps w) (ftScenarios f)
          r1 `shouldBe` Passed
          r2 `shouldSatisfy` \v -> case v of Failed _ -> True; _ -> False
    it "an undefined step fails naming the orphan" $ do
      let w = World "http://x" (\_ -> pure (Left "no")) "" mempty False
      case parseFeature "t.feature"
             "Feature: t\n  Scenario: s\n    When I do something nobody defined" of
        Left e -> expectationFailure (T.unpack e)
        Right f -> case ftScenarios f of
          (sc : _) -> do
            v <- runScenario allSteps w sc
            case v of
              Failed e -> e `shouldSatisfy` T.isInfixOf "nobody defined"
              _ -> expectationFailure "should have failed"
          [] -> expectationFailure "expected at least one scenario"
    -- Review finding (Task 6 round 1): the classification this whole stage
    -- exists to produce — a non-@target Failed is a hard red, a @target
    -- Failed is expected-red (reported, not fatal), a @target Passed is
    -- informational (not fatal either) — had ZERO coverage: the test above
    -- only calls runScenario, which never sees tags at all. This test
    -- drives the real pipeline: runFeatureFiles (so tag propagation from
    -- Scenario through ScenarioResult is genuinely exercised, not
    -- reimplemented), the exported Run.hardReds law, and reportTable's
    -- cell rendering, against a real feature file on disk plus a second
    -- file that fails to parse (so the PARSE-failure path — no tags, must
    -- count as a hard red — is covered too).
    it "classifies @target vs. hard-red through runFeatureFiles, hardReds, and reportTable" $ do
      let fake url
            | "/api/ok" `T.isSuffixOf` url =
                pure (Right ("{\"ok\":\"yes\"}", fromJust (A.decodeStrict "{\"ok\":\"yes\"}")))
            | otherwise =
                pure (Right ("{\"x\":1}", fromJust (A.decodeStrict "{\"x\":1}")))
          w = World "http://x" fake "test/fixtures" mempty False
      results <- runFeatureFiles allSteps w
        [ "test/features/classification.feature", "test/features/badparse.feature" ]
      case results of
        [hardRed, targetFail, targetPass, parseFail] -> do
          -- tags and verdicts genuinely came out of the real pipeline
          srScenario hardRed `shouldBe` "hard red"
          srTags hardRed `shouldBe` []
          srVerdict hardRed `shouldSatisfy` \v -> case v of Failed _ -> True; _ -> False
          srScenario targetFail `shouldBe` "expected red"
          srTags targetFail `shouldBe` [Tag "target"]
          srVerdict targetFail `shouldSatisfy` \v -> case v of Failed _ -> True; _ -> False
          srScenario targetPass `shouldBe` "target already met"
          srTags targetPass `shouldBe` [Tag "target"]
          srVerdict targetPass `shouldBe` Passed
          srScenario parseFail `shouldBe` "PARSE"
          srTags parseFail `shouldBe` []
          srVerdict parseFail `shouldSatisfy` \v -> case v of Failed _ -> True; _ -> False
          -- the classification: only the untagged failure and the parse
          -- failure are hard reds; the @target failure is expected, and
          -- the @target pass is informational — neither is fatal
          map srScenario (hardReds results) `shouldBe` ["hard red", "PARSE"]
          -- and the report table must actually SHOW the distinction, not
          -- just compute it silently
          let table = reportTable results
          table `shouldSatisfy` T.isInfixOf "| hard red | \10060 RED"
          table `shouldSatisfy`
            T.isInfixOf "| expected red | \128308 red (expected \8212 @target) |"
          table `shouldSatisfy`
            T.isInfixOf "| target already met | \128994 green (target already met!) |"
          table `shouldSatisfy` T.isInfixOf "| PARSE | \10060 RED"
        rs -> expectationFailure ("expected exactly 4 results, got " <> show (length rs))
    -- Review finding (Task 6 round 1), fix 3: the exception branch of
    -- runScenario dropped the step's keyword and body that the logical-
    -- failure branch includes, even though this stage's whole deliverable
    -- is a legible diagnosis of which step failed.
    it "an exception thrown while running a step is reported with the step's keyword and body" $ do
      let w = World "http://x" (\_ -> error "boom") "" mempty False
      case parseFeature "t.feature" "Feature: t\n  Scenario: s\n    When I GET /boom" of
        Left e -> expectationFailure (T.unpack e)
        Right f -> case ftScenarios f of
          (sc : _) -> do
            v <- runScenario allSteps w sc
            case v of
              Failed e -> do
                e `shouldSatisfy` T.isInfixOf "When"
                e `shouldSatisfy` T.isInfixOf "I GET /boom"
              _ -> expectationFailure "should have failed"
          [] -> expectationFailure "expected at least one scenario"
    -- Fix 2 (post-Task-7 review): runScenario used to silently run the
    -- FIRST Matched action when two or more definitions truly matched --
    -- `case [f | Matched f <- results] of (f:_) -> ...` -- which is
    -- exactly the shadowing the three-state Claim refactor set out to
    -- remove, just relocated from `check` time to `run` time. It survives
    -- wherever `check` hasn't (yet) been run: a developer running `run`
    -- directly against a fresh feature file. This drives that path with a
    -- synthetic ambiguous pair (mirroring the totality test below) and
    -- asserts the scenario fails LOUDLY, naming both competing sketches,
    -- instead of quietly succeeding by picking one.
    it "runScenario refuses to run anything when two or more definitions \
       \truly MATCH the same body, naming the competing sketches instead \
       \of silently picking one" $ do
      let dupDefs =
            [ mkStep When (lit "I do " *> capRest @FixtureRefFreeText) (\_ w' -> pure (Right w'))
            , mkStep When (lit "I do the thing") (\() w' -> pure (Right w'))
            ]
          w = World "http://x" (\_ -> pure (Left "no")) "" mempty False
      case parseFeature "dup2.feature" $ T.unlines
             [ "Feature: dup2"
             , "  Scenario: s"
             , "    When I do the thing" ] of
        Left e -> expectationFailure (T.unpack e)
        Right f -> case ftScenarios f of
          [sc] -> do
            v <- runScenario dupDefs w sc
            case v of
              Failed e -> do
                e `shouldSatisfy` T.isInfixOf "I do {text}"
                e `shouldSatisfy` T.isInfixOf "I do the thing"
                e `shouldSatisfy` T.isInfixOf "ambiguous"
              _ -> expectationFailure "expected ambiguity to fail loudly, not silently pick one"
          scs -> expectationFailure ("expected one scenario, got " <> show (length scs))

  describe "totality" $ do
    it "names the orphan steps" $ do
      -- (Deviation from the brief's literal `let Right f = ...`: an
      -- incomplete pattern binding triggers -Wincomplete-uni-patterns
      -- under this project's -Wall, same issue task-6-report.md already
      -- worked around. Restructured as a case to keep pristine output.)
      case parseFeature "t.feature" $ T.unlines
             [ "Feature: t"
             , "  Scenario: s"
             , "    When I render pieces fills at year -1405 in style canaan"
             , "    Then nobody wrote this step" ] of
        Left e -> expectationFailure (T.unpack e)
        Right f -> map snd (Check.orphans allSteps f) `shouldBe` ["nobody wrote this step"]
    -- R23 (controller ruling): mkStep's old Maybe conflated "I match this
    -- line" and "I recognize this shape but the value is bad" into one
    -- Just, which is what made BOTH real collisions below look ambiguous
    -- and left them protected only by allSteps' list order. World.Claim
    -- now has three states (NoMatch / ClaimError / Matched), and a full
    -- Matched always wins over a mere ClaimError — dissolving both
    -- collisions structurally, with no per-step special-casing.
    it "a line one definition MATCHES and another only CLAIMS resolves to \
       \the matching definition's behavior, and check calls it unambiguous" $ do
      -- Real collision #1 (Task 5 review / R18): "the response field scene
      -- equals http://x/foo" MATCHES the field-equality step; the generic
      -- "{name} equals {name}" step's BindName capture on "the response
      -- field scene" fails to parse (spaces aren't a bind name) and so
      -- only CLAIMS. Driven through runScenario (not just Check) to prove
      -- the MATCHING definition's actual behavior runs — the field
      -- genuinely gets compared and passes — not merely that Check stays
      -- quiet about it.
      let fake url = pure (Right (raw, v))
            where raw = TE.encodeUtf8 ("{\"scene\":\"" <> url <> "\",\"labels\":[]}")
                  v   = fromJust (A.decodeStrict raw)
          w0 = World "http://x" fake "test/fixtures" mempty False
      case parseFeature "c1.feature" $ T.unlines
             [ "Feature: c1"
             , "  Scenario: s"
             , "    When I GET /foo"
             , "    Then the response field scene equals http://x/foo" ] of
        Left e -> expectationFailure (T.unpack e)
        Right f -> do
          Check.orphans allSteps f `shouldBe` []
          Check.ambiguous allSteps f `shouldBe` []
          Check.valueErrors allSteps f `shouldBe` []
          case ftScenarios f of
            [sc] -> do
              v <- runScenario allSteps w0 sc
              v `shouldBe` Passed
            scs -> expectationFailure ("expected one scenario, got " <> show (length scs))
    it "the second real collision (render steps' \"as\"/no-\"as\" overloads) \
       \is also unambiguous under R23" $ do
      -- Real collision #2, found while implementing Task 7 (beyond what
      -- R18 named): "I render pieces fills at year -1405 in style canaan
      -- as sceneA" MATCHES the binding overload ("... as {name}"); the
      -- non-binding overload's capRest @StyleName swallows the whole
      -- remainder "canaan as sceneA" and fails to parse it as a style, so
      -- it only CLAIMS. Asserted against the REAL allSteps and the REAL
      -- line (not a synthetic fixture), and driven through runScenario to
      -- prove the binding overload's action actually ran (the scene gets
      -- bound), matching Task 6's own existing "the render step binds a
      -- named response" behavior.
      let fake url = pure (Right (raw, v))
            where raw = TE.encodeUtf8 ("{\"scene\":\"" <> url <> "\",\"labels\":[]}")
                  v   = fromJust (A.decodeStrict raw)
          w0 = World "http://x" fake "test/fixtures" mempty False
      case parseFeature "c2.feature" $ T.unlines
             [ "Feature: c2"
             , "  Scenario: s"
             , "    When I render pieces fills at year -1405 in style canaan as sceneA" ] of
        Left e -> expectationFailure (T.unpack e)
        Right f -> do
          Check.orphans allSteps f `shouldBe` []
          Check.ambiguous allSteps f `shouldBe` []
          Check.valueErrors allSteps f `shouldBe` []
          case ftScenarios f of
            [sc] -> do
              v <- runScenario allSteps w0 sc
              v `shouldBe` Passed
            scs -> expectationFailure ("expected one scenario, got " <> show (length scs))
    it "genuine ambiguity -- two definitions that both fully MATCH the same \
       \body -- is still detected and fatal, and NAMES the competing \
       \definitions (fix 1)" $ do
      -- No such case survives in the real allSteps after R23 (that's the
      -- whole point), so this constructs a small StepDef list where two
      -- definitions both actually match, to prove real ambiguity is still
      -- caught and not accidentally dissolved along with the two fakes.
      --
      -- Fix 1 (post-Task-7 review): the two definitions here used to share
      -- one IDENTICAL sketch, and the assertion only checked
      -- `length sketches == 2` with the scenario name wildcarded -- a
      -- shape that would still pass if VAmbiguous were built from every
      -- keyword-matching definition, from the errored ones, or from a
      -- constant pair, i.e. it verified a COUNT, not that ambiguity
      -- actually NAMES the competing definitions. These two definitions
      -- now have genuinely DISTINCT sketches (one via a literal, one via
      -- a capture) that both still fully match "I do the thing", and the
      -- whole triple -- scenario name, step body, and both sketches -- is
      -- asserted in one equality.
      let dupDefs =
            [ mkStep When (lit "I do " *> capRest @FixtureRefFreeText) (\_ w -> pure (Right w))
            , mkStep When (lit "I do the thing") (\() w -> pure (Right w))
            ]
      case parseFeature "dup.feature" $ T.unlines
             [ "Feature: dup"
             , "  Scenario: s"
             , "    When I do the thing" ] of
        Left e -> expectationFailure (T.unpack e)
        Right f -> Check.ambiguous dupDefs f `shouldBe`
          [("s", "I do the thing", ["I do {text}", "I do the thing"])]
    it "a step whose shape matches but whose value doesn't parse is its own \
       \value-error class, reported by check and NOT silently skipped by \
       \runScenario when nothing else matches" $ do
      -- "topografy" is not a piece. Both the binding and non-binding "I
      -- render pieces ..." overloads share the same PieceSet capture, so
      -- BOTH only CLAIM this line (with the same error) and NEITHER
      -- matches -- genuinely different from the two collisions above,
      -- where one definition always fully matched. There is no successful
      -- match to fall back on, so this must be reported as its own class,
      -- not folded into "orphan" (nobody's shape matched -- false, two
      -- did) or "ambiguous" (two full matches -- false, zero did).
      let body = "I render pieces water, topografy at year -1405 in style canaan as sceneA"
      case parseFeature "bad.feature" $ T.unlines
             [ "Feature: bad", "  Scenario: s", "    When " <> body ] of
        Left e -> expectationFailure (T.unpack e)
        Right f -> do
          Check.orphans allSteps f `shouldBe` []
          Check.ambiguous allSteps f `shouldBe` []
          case Check.valueErrors allSteps f of
            [(_, b, errs)] -> do
              b `shouldBe` body
              errs `shouldSatisfy` (not . null)
              mapM_ (\(_, e) -> e `shouldSatisfy` T.isInfixOf "not a piece") errs
            other -> expectationFailure
                       ("expected exactly one value-error step, got " <> show other)
          case ftScenarios f of
            [sc] -> do
              v <- runScenario allSteps (World "http://x" (\_ -> pure (Left "no")) "" mempty False) sc
              case v of
                Failed e -> e `shouldSatisfy` T.isInfixOf "not a piece"
                _ -> expectationFailure "expected the bad piece name to fail, not pass"
            scs -> expectationFailure ("expected one scenario, got " <> show (length scs))
    -- Fix 3 (post-Task-7 review): the "real allSteps is unambiguous" pin
    -- only covered four line shapes (the two collisions above plus the
    -- value-error case); four definitions -- the fixture-equality step,
    -- the generic "{name} equals {name}" step, the resources-subset step,
    -- and labels-are-empty -- had NO coverage at all. This feeds one
    -- exemplar body per definition in the real allSteps (built directly
    -- as AST, not parsed text, so keyword sequencing is irrelevant) and
    -- asserts no orphan, ambiguity, or value-error anywhere across the
    -- whole set. The length equality is the actual re-trigger: append a
    -- ninth step definition without adding its exemplar here and this
    -- test fails on the count alone, regardless of which shape the new
    -- step happens to collide with.
    it "every definition in the real allSteps has a genuine, unambiguous \
       \exemplar -- covering the whole step set, not just four shapes \
       \(fix 3)" $ do
      let exemplars =
            [ (When, "I GET /foo")
            , (Then, "the response equals fixture \"foo\"")
            , (Then, "the response field scene equals http://x/foo")
            , (When, "I render pieces fills at year -1405 in style canaan as sceneA")
            , (When, "I render pieces fills at year -1405 in style canaan")
            , (Then, "sceneA equals sceneB")
            , (Then, "sceneA's resources are a subset of sceneB's resources")
            , (Then, "sceneA's labels are empty")
            ]
          steps = [ Step k b Nothing | (k, b) <- exemplars ]
          f = Feature "exemplars" [] [] [] [Scenario "s" [] steps]
      length exemplars `shouldBe` length allSteps
      Check.orphans allSteps f `shouldBe` []
      Check.ambiguous allSteps f `shouldBe` []
      Check.valueErrors allSteps f `shouldBe` []
    -- Fix 5 (post-Task-7 review): checkDir's AMBIGUOUS and BAD-VALUE print
    -- paths (the `label`/`describe` branches other than ORPHAN) had never
    -- actually executed under test -- every existing totality test calls
    -- Check.orphans/ambiguous/valueErrors directly, never checkDir itself.
    -- This drives the real checkDir over a real temp directory (a
    -- synthetic ambiguous pair, reusing the fix-1/2 shape, alongside a
    -- real bad-piece-name line from allSteps) and asserts BOTH tagged
    -- lines actually appear in stdout. checkDir calls exitFailure, so the
    -- resulting ExitCode exception is caught via `try` rather than killing
    -- the test process.
    it "checkDir's AMBIGUOUS and BAD-VALUE print paths actually execute \
       \and are visible in its output (fix 5)" $ do
      tmpBase <- getTemporaryDirectory
      let dir = tmpBase </> "contract-runner-checkdir-test"
          dupDefs =
            [ mkStep When (lit "I do " *> capRest @FixtureRefFreeText) (\_ w -> pure (Right w))
            , mkStep When (lit "I do the thing") (\() w -> pure (Right w))
            ]
          feat = T.unlines
            [ "Feature: probe"
            , "  Scenario: ambiguous case"
            , "    When I do the thing"
            , "  Scenario: bad value case"
            , "    When I render pieces bogus at year -1405 in style canaan" ]
      createDirectoryIfMissing True dir
      TIO.writeFile (dir </> "probe.feature") feat
      (out, result) <- captureStdout (Check.checkDir (dupDefs ++ allSteps) dir)
      removeDirectoryRecursive dir
      result `shouldSatisfy` isLeft
      out `shouldSatisfy` T.isInfixOf "AMBIGUOUS"
      out `shouldSatisfy` T.isInfixOf "I do {text}"
      out `shouldSatisfy` T.isInfixOf "I do the thing"
      out `shouldSatisfy` T.isInfixOf "BAD-VALUE"
      out `shouldSatisfy` T.isInfixOf "not a piece"
    -- Fix 7 (post-Task-7 review, structural -- closes a FALSE GREEN in the
    -- law): the real corpus's "When I GET /api/subjects?year=<someYear>
    -- as first" used to come back CLEAN, even though no "I GET {url} as
    -- {name}" binding step exists yet. Cause: the plain GET step captured
    -- its URL with FixtureRefFreeText, whose parseCap is
    -- `Right . FixtureRefFreeText . T.strip` -- it can NEVER fail, so it
    -- silently swallowed " as first" as part of the URL and registered a
    -- full Matched. A capture that cannot fail has no discriminating
    -- power, so the totality law had nothing to catch -- a check
    -- satisfiable by the failure mode, which this project forbids (see
    -- MEMORY: verify-distinct-not-nonnull). UrlPath fixes this BY TYPE: a
    -- URL path cannot contain a raw space, a genuine property of the
    -- type, not a special case for " as ". This proves the hole closes:
    -- the line is now a value error (nothing fully matches), not a clean
    -- match. When the real GET-as step is added in a later phase, this is
    -- what will make IT the unique match rather than creating a fresh
    -- ambiguity with the plain GET step.
    it "a GET line with a stray \" as name\" is a value error, not a \
       \silent clean match, now that UrlPath rejects embedded whitespace \
       \(fix 7)" $ do
      let body = "I GET /api/subjects?year=-1405 as first"
      case parseFeature "getas.feature" $ T.unlines
             [ "Feature: f", "  Scenario: s", "    When " <> body ] of
        Left e -> expectationFailure (T.unpack e)
        Right f -> do
          Check.orphans allSteps f `shouldBe` []
          Check.ambiguous allSteps f `shouldBe` []
          case Check.valueErrors allSteps f of
            [(_, b, errs)] -> do
              b `shouldBe` body
              errs `shouldSatisfy` (not . null)
              mapM_ (\(_, e) -> e `shouldSatisfy` T.isInfixOf "whitespace") errs
            other -> expectationFailure
                       ("expected exactly one value-error step, got " <> show other)

-- Fix 5's stdout-capture helper: redirects the process's real stdout to a
-- temp file for the duration of `act` (via GHC.IO.Handle's fd-duplication,
-- the same technique `System.IO.Silently` uses), then restores it and
-- returns what was written plus `act`'s own `try` result. Needed because
-- checkDir writes straight to stdout and calls exitFailure -- there is no
-- other way to observe its output from inside a test.
captureStdout :: IO a -> IO (T.Text, Either ExitCode a)
captureStdout act = do
  tmpDir <- getTemporaryDirectory
  (path, h) <- openTempFile tmpDir "capture.txt"
  hSetEncoding h utf8
  old <- hDuplicate stdout
  hDuplicateTo h stdout
  result <- try act
  hFlush stdout
  hDuplicateTo old stdout
  hClose old
  hClose h
  txt <- TIO.readFile path
  removeFile path
  pure (txt, result)

firstMatch :: Keyword -> T.Text -> Maybe (World -> IO (Either T.Text World))
firstMatch k t = listToMaybe
  [ f | StepDef k' _ _ m <- allSteps, k' == k, Matched f <- [m t] ]

-- '|' is deliberately excluded from the alphabet: the renderer emits
-- table rows as "| a | b |" with no escaping, so a cell containing '|'
-- could never round-trip through this format — a genuine representability
-- limit of the (unescaped) table syntax, not a generator convenience.
safeText :: Gen T.Text
safeText = (T.pack <$> listOf1 (elements (['a'..'z'] ++ ['0'..'9'] ++ " -")))
    `suchThat` (\t -> t == T.strip t && not (T.null (T.strip t)))

instance Arbitrary Feature where
  arbitrary = do
    t   <- safeText
    tgs <- sublistOf [Tag "smoke", Tag "wip"]
    pre <- listOf safeText
    vs  <- listOf ((,) <$> safeText <*> safeText)
    ss  <- listOf1 genScenario
    pure (Feature t tgs pre vs ss)
    where
      genScenario = do
        n  <- safeText
        tg <- sublistOf [Tag "property", Tag "target"]
        st <- listOf1 genStep
        pure (Scenario n tg st)
      genStep = Step <$> elements [Given, When, Then] <*> safeText <*> genStepArg
      -- No DocString: the parser deliberately defers DocString support
      -- (see Gherkin.Parse), so it's not part of the round-trip law yet.
      genStepArg = frequency
        [ (2, pure Nothing)
        , (1, Just . Table <$> genTable)
        ]
      -- `listOf1 (listOf1 safeText)` nests three unbounded dimensions
      -- (rows, cells, and cell length, via safeText's own listOf1) under
      -- one shared QuickCheck size parameter, so generation cost grows
      -- roughly cubically with ambient size — at size 100 that's on the
      -- order of a 100-row table of 100 cells of 100 characters each,
      -- which is the actual source of a 78-second run. `scale intSqrt`
      -- at each nesting boundary shrinks the size seen by that level
      -- (roughly: n rows -> sqrt(n) cells -> sqrt(sqrt(n))-length text),
      -- bringing total cost back down to roughly linear in the ambient
      -- size. This changes the *distribution*, not the *domain*: every
      -- table shape, arbitrarily large, is still reachable at a large
      -- enough ambient size — nothing is capped out of what the law
      -- ranges over.
      genTable = scale intSqrt (listOf1 (scale intSqrt (listOf1 safeText)))
      intSqrt = floor . sqrt . (fromIntegral :: Int -> Double)

-- Every subset of the piece universe, including the empty set (the
-- monoid identity that Task 9's subset generation relies on).
instance Arbitrary PieceSet where
  arbitrary = PieceSet . Set.fromList <$> sublistOf [minBound .. maxBound]
