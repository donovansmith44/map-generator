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
import qualified Vocab
import qualified Prop
import Control.Exception (try, bracket, finally)
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
    -- Review finding (post-Task-7 review round 2), fix 5: UrlPath (fix 7's
    -- new capture type) had no direct law test where the other capture
    -- laws are pinned -- only an indirect check via the GET step's
    -- totality classification. A type introduced specifically as "the fix
    -- is by type, with a law" belongs alongside Piece/PieceSet/Year's own
    -- round-trip laws: a valid path is accepted and round-trips, and any
    -- embedded whitespace is rejected outright (the whole point of the
    -- type, per fix 7's comment in Steps.hs).
    it "UrlPath accepts a whitespace-free path and round-trips" $
      fmap renderCap (parseCap @UrlPath "/api/subjects?year=-1405")
        `shouldBe` Right "/api/subjects?year=-1405"
    it "UrlPath rejects any embedded whitespace" $
      case parseCap @UrlPath "/api/subjects?year=-1405 as first" of
        Left e  -> e `shouldSatisfy` T.isInfixOf "whitespace"
        Right _ -> expectationFailure "accepted a path containing a space"

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
    -- whole set.
    --
    -- Review finding (post-Task-7 review round 2), fix 3: the original
    -- version of this test asserted only `length exemplars == length
    -- allSteps` -- a CARDINALITY check, not a bijection. Append a ninth
    -- definition that duplicates an existing shape, plus a ninth exemplar
    -- for it, and that count-only check stays green while the genuinely
    -- new definition is never actually exercised by anything. Fixed by
    -- collecting, for every exemplar, the sketch of the ONE definition
    -- that actually MATCHED it (there's exactly one, since ambiguous and
    -- valueErrors are already asserted empty), and asserting that the SET
    -- of matched sketches equals the set of every definition's sketch --
    -- a real bijection: every definition gets its own distinct exemplar,
    -- not just the right count of exemplars.
    it "every definition in the real allSteps has a genuine, unambiguous \
       \exemplar -- covering the whole step set as a bijection, not just \
       \a count (fix 3)" $ do
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
          matchedSketchFor (k, b) =
            [ defSketch d | d <- allSteps, defKw d == k, Matched _ <- [defRun d b] ]
      Check.orphans allSteps f `shouldBe` []
      Check.ambiguous allSteps f `shouldBe` []
      Check.valueErrors allSteps f `shouldBe` []
      Set.fromList (concatMap matchedSketchFor exemplars) `shouldBe`
        Set.fromList (map defSketch allSteps)
    -- Fix 5 (post-Task-7 review): checkDir's AMBIGUOUS and BAD-VALUE print
    -- paths (the `label`/`describe` branches other than ORPHAN) had never
    -- actually executed under test -- every existing totality test calls
    -- Check.orphans/ambiguous/valueErrors directly, never checkDir itself.
    -- This drives the real checkDir over a real temp directory (a
    -- synthetic ambiguous pair, reusing the fix-1/2 shape, alongside a
    -- real bad-piece-name line from allSteps) and asserts the output.
    -- checkDir calls exitFailure, so the resulting ExitCode exception is
    -- caught via `try` (inside captureStdout) rather than killing the
    -- test process.
    --
    -- Review finding (post-Task-7 review round 2), fix 4: asserting five
    -- substrings is satisfiable by output where the labels are attached
    -- to the wrong steps, or the "loc: msg" line format is broken (e.g.
    -- both tags and both bodies present, but swapped, or concatenated
    -- into one garbled line) -- and this printed format is exactly what
    -- the Stage 0 diagnosis document is generated from. checkDir's output
    -- is short and fully deterministic once the piece name can't trigger
    -- Capture.hs's did-you-mean hint, so this now asserts the ENTIRE
    -- captured text against two fully-spelled-out expected lines. Picked
    -- "qqqqqqqqqq" (not "bogus") as the bad piece name specifically
    -- because it is Levenshtein-far from every real piece name (no
    -- shared letters, ten characters against the longest piece name's
    -- eight), so didYouMean's <=3 gate never fires and the error text is
    -- exactly the two-part "'x' is not a piece." / "Pieces are: ..."
    -- message with no extra suggestion clause to predict.
    --
    -- Review finding (post-Task-7 review round 2), fix 6: the temp
    -- directory used to have a fixed name
    -- (tmpBase </> "contract-runner-checkdir-test"), so two concurrent
    -- runs (two checkouts, or CI shards sharing a machine's temp
    -- directory) could collide and clobber each other's fixture. Made
    -- unique by reserving a uniquely-named temp FILE first (openTempFile
    -- guarantees no collision with anything else live at that moment),
    -- then using that reserved, guaranteed-unique name as the basis for
    -- the directory this test actually creates.
    it "checkDir's AMBIGUOUS and BAD-VALUE print paths actually execute \
       \and produce the exact expected output (fix 5)" $ do
      tmpBase <- getTemporaryDirectory
      (uniqueFile, uh) <- openTempFile tmpBase "contract-runner-checkdir-test"
      hClose uh
      removeFile uniqueFile
      let dir = uniqueFile <> "-dir"
          featPath = dir </> "probe.feature"
          dupDefs =
            [ mkStep When (lit "I do " *> capRest @FixtureRefFreeText) (\_ w -> pure (Right w))
            , mkStep When (lit "I do the thing") (\() w -> pure (Right w))
            ]
          feat = T.unlines
            [ "Feature: probe"
            , "  Scenario: ambiguous case"
            , "    When I do the thing"
            , "  Scenario: bad value case"
            , "    When I render pieces qqqqqqqqqq at year -1405 in style canaan" ]
          -- Both the binding ("... as {name}") and non-binding render
          -- overloads share `capUntil @PieceSet " at year "` as their
          -- FIRST capture, so a bad piece name fails identically for both
          -- before either overload's own "as"/no-"as" tail is ever
          -- reached -- same precedent as the pre-existing "a step whose
          -- shape matches but whose value doesn't parse" test above
          -- (the "topografy" line), which asserts exactly two claimants
          -- for the same reason. Both entries carry the identical error
          -- text, differing only by sketch.
          piecesErr = "'qqqqqqqqqq' is not a piece.\n"
                   <> "  Pieces are: borders, chrome, claims, fills, ground, journeys, "
                   <> "labels, markers, veil, water"
          expected = T.concat
            [ "AMBIGUOUS ", T.pack featPath, " / ambiguous case: I do the thing matches 2 "
            , "definitions: I do {text} | I do the thing\n"
            , "BAD-VALUE ", T.pack featPath, " / bad value case: I render pieces qqqqqqqqqq "
            , "at year -1405 in style canaan -- "
            , "I render pieces {pieces} at year {year} in style {style} as {name}: ", piecesErr
            , "; I render pieces {pieces} at year {year} in style {style}: ", piecesErr
            , "\n"
            ]
      createDirectoryIfMissing True dir
      TIO.writeFile featPath feat
      (out, result) <- captureStdout (Check.checkDir (dupDefs ++ allSteps) dir)
      removeDirectoryRecursive dir
      result `shouldSatisfy` isLeft
      out `shouldBe` expected
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

  describe "vocabulary drift" $ do
    it "derives the expected table from the steps' capture types" $ do
      case parseFeature "t.feature" $ T.unlines
             [ "Feature: t"
             , "  Scenario: s"
             , "    When I render pieces fills at year -1405 in style canaan" ] of
        Left e -> expectationFailure (T.unpack e)
        Right f -> do
          lookup "pieces" (Vocab.expectedVocab allSteps f)
            `shouldBe` Just "any of: borders, chrome, claims, fills, ground, journeys, labels, markers, veil, water"
          lookup "year" (Vocab.expectedVocab allSteps f)
            `shouldBe` Just "whole number from -4004 to 100 (negative means BC; -1405 is 1405 BC)"
    it "flags drift when the file's table disagrees" $ do
      case parseFeature "t.feature" $ T.unlines
             [ "Feature: t"
             , "  Vocabulary:"
             , "    | pieces | some old lie |"
             , "  Scenario: s"
             , "    When I render pieces fills at year -1405 in style canaan" ] of
        Left e -> expectationFailure (T.unpack e)
        Right f -> Vocab.drift allSteps f `shouldSatisfy` (not . null)
    -- Task 8's `url` decision: UrlPath (Steps.hs's GET-path capture) is a
    -- Described universe, same shape as FixtureRefFreeText's "text" and
    -- BindName's "name" -- free-form prose describing a value, not a
    -- finite/bounded vocabulary a dummy needs to learn. expectedVocab
    -- excludes ALL THREE the same structural way (by Universe constructor,
    -- not by a hardcoded capName list), so a step using only Described
    -- captures contributes nothing to the table.
    it "excludes Described captures (url, name, text) from the table -- structurally, by Universe, not by a hardcoded name list" $ do
      case parseFeature "t.feature" $ T.unlines
             [ "Feature: t"
             , "  Scenario: s"
             , "    When I GET /api/subjects?year=-1405 as first" ] of
        Left e -> expectationFailure (T.unpack e)
        Right f -> Vocab.expectedVocab allSteps f `shouldBe` []
    -- R34 (controller ruling): --write must be SURGICAL -- it must never
    -- go through parseFeature/renderFeature's whole-file round trip (which
    -- reformats every preamble line and even drops blank lines within a
    -- preamble), because the corpus is hand-written prose the owner cares
    -- about. This proves the property empirically against a file with
    -- prose, a leading tag, a blank line, and a DELIBERATELY WRONG
    -- existing Vocabulary block (fewer rows than the truth, so the block
    -- must both change content AND grow) -- every line that is not a
    -- table row must survive byte-for-byte, in the same order, and the
    -- rewritten table must equal the types.
    it "--write rewrites only the Vocabulary block; every other line survives untouched" $ do
      tmpBase <- getTemporaryDirectory
      (uniqueFile, uh) <- openTempFile tmpBase "contract-runner-vocab-write-test"
      hClose uh
      removeFile uniqueFile
      let dir = uniqueFile <> "-dir"
          path = dir </> "surgical.feature"
          original = T.unlines
            [ "@smoke"
            , "Feature: t \8212 a hand-written law"
            , "  Prose the owner wrote by hand, deliberately kept."
            , "  A second prose line, also deliberate."
            , ""
            , "  Vocabulary:"
            , "    | pieces | some old lie |"
            , ""
            , "  @property"
            , "  Scenario: s"
            , "    When I render pieces fills at year -1405 in style canaan"
            ]
          isRow l = "    | " `T.isPrefixOf` l
          nonRow ls = [ l | l <- ls, not (isRow l) ]
      createDirectoryIfMissing True dir
      -- BS.writeFile, not TIO.writeFile: this test's own fixture setup hit
      -- the same console/handle-encoding trap the em dash below is meant
      -- to exercise (Main.hs's UTF-8 fix, fix 6, is only wired into the
      -- executable's entry point, not this test process) -- writing raw
      -- UTF-8 bytes sidesteps it entirely, the same way Vocab.vocabDir's
      -- own --write path does for the real corpus.
      BS.writeFile path (TE.encodeUtf8 original)
      (`finally` removeDirectoryRecursive dir) $ do
        -- Review round 2, fix 3: --write must say what actually happened,
        -- not a fixed "rewritten" string -- this file genuinely changes on
        -- the first run (the table was wrong) and genuinely does NOT
        -- change on a second consecutive run (idempotent), and the two
        -- messages must say so honestly rather than claiming the same
        -- "rewritten" both times.
        (out1, _) <- captureStdout (Vocab.vocabDir allSteps dir True)
        out1 `shouldBe` "vocabulary: rewrote 1 of 1 file(s)\n"
        -- decodeUtf8, not TIO.readFile: same encoding trap as the write
        -- side above -- this toolchain's default text-handle decoder is
        -- not UTF-8, so reading the em dash back through it would show a
        -- false failure (the file is fine; the read would not be).
        rewritten <- TE.decodeUtf8 <$> BS.readFile path
        rewritten `shouldSatisfy` T.isSuffixOf "\n"
        nonRow (T.lines rewritten) `shouldBe` nonRow (T.lines original)
        case parseFeature path rewritten of
          Left e -> expectationFailure (T.unpack e)
          Right f -> ftVocab f `shouldBe` Vocab.expectedVocab allSteps f
        (out2, _) <- captureStdout (Vocab.vocabDir allSteps dir True)
        out2 `shouldBe`
          "vocabulary: already matches its types across 1 file(s); nothing rewritten\n"
        -- and re-running in verify mode against the now-correct file
        -- reports clean, with no exception (exitFailure) along the way
        (out, result) <- captureStdout (Vocab.vocabDir allSteps dir False)
        result `shouldSatisfy` isRight
        out `shouldBe` "vocabulary: every table matches its types\n"
    -- Review finding (round 2), gap 1: the REMOVAL path had no test.
    -- `renderRows []` deletes an existing block entirely, and the same
    -- backOverBlanks folding used for insertion/replacement must also
    -- apply here so the file ends up with exactly the SAME shape it would
    -- have had if the block had never been stamped in the first place --
    -- not proven anywhere until now. This step's only capture is UrlPath
    -- (Described, excluded), so expectedVocab is [], and the fixture
    -- starts with a stale, WRONG block that must be removed outright.
    it "--write REMOVES an existing block entirely when the steps' true \
       \vocabulary is empty, leaving the file exactly as if no block had \
       \ever been stamped" $ do
      tmpBase <- getTemporaryDirectory
      (uniqueFile, uh) <- openTempFile tmpBase "contract-runner-vocab-removal-test"
      hClose uh
      removeFile uniqueFile
      let dir = uniqueFile <> "-dir"
          path = dir </> "removal.feature"
          withStaleBlock = T.unlines
            [ "@smoke"
            , "Feature: t \8212 removal case"
            , "  Prose the owner wrote by hand."
            , ""
            , "  Vocabulary:"
            , "    | pieces | stale nonsense left over from a deleted step |"
            , ""
            , "  Scenario: s"
            , "    When I GET /api/whatever"
            ]
          -- what the file would look like if it had never had a block:
          -- a single blank line between the prose and the Scenario, same
          -- as `withStaleBlock` minus the block and its extra blank.
          withoutBlock = T.unlines
            [ "@smoke"
            , "Feature: t \8212 removal case"
            , "  Prose the owner wrote by hand."
            , ""
            , "  Scenario: s"
            , "    When I GET /api/whatever"
            ]
      createDirectoryIfMissing True dir
      BS.writeFile path (TE.encodeUtf8 withStaleBlock)
      (`finally` removeDirectoryRecursive dir) $ do
        Vocab.vocabDir allSteps dir True
        rewritten <- TE.decodeUtf8 <$> BS.readFile path
        -- full ordered-line equality against the "never had a block"
        -- canonical text, not just a "no row survives" spot check: this
        -- pins the exact blank-line count too, not merely the rows' fate.
        T.lines rewritten `shouldBe` T.lines withoutBlock
        case parseFeature path rewritten of
          Left e -> expectationFailure (T.unpack e)
          Right f -> ftVocab f `shouldBe` []
    -- Review finding (round 2), gap 2: a feature with NO preamble prose at
    -- all -- "Scenario:" immediately after "Feature:" -- had no test.
    -- `backOverBlanks` walks back from the boundary and finds nothing
    -- blank immediately before it (the previous line IS "Feature: ..."),
    -- so insertion should land directly after the Feature line with
    -- exactly the block's own one leading blank, and nothing after the
    -- rows (there was no blank there to reuse).
    it "--write inserts the block directly after Feature: when there is no \
       \preamble at all, with one leading blank and none trailing" $ do
      tmpBase <- getTemporaryDirectory
      (uniqueFile, uh) <- openTempFile tmpBase "contract-runner-vocab-nopreamble-test"
      hClose uh
      removeFile uniqueFile
      let dir = uniqueFile <> "-dir"
          path = dir </> "nopreamble.feature"
          original = T.unlines
            [ "Feature: t"
            , "  Scenario: s"
            , "    When I render pieces fills at year -1405 in style canaan"
            ]
      createDirectoryIfMissing True dir
      BS.writeFile path (TE.encodeUtf8 original)
      (`finally` removeDirectoryRecursive dir) $ do
        Vocab.vocabDir allSteps dir True
        rewritten <- TE.decodeUtf8 <$> BS.readFile path
        case parseFeature path rewritten of
          Left e -> expectationFailure (T.unpack e)
          Right f -> do
            let vocab = Vocab.expectedVocab allSteps f
            vocab `shouldSatisfy` (not . null)
            T.lines rewritten `shouldBe`
              [ "Feature: t"
              , ""
              , "  Vocabulary:" ]
              ++ [ "    | " <> k <> " | " <> v <> " |" | (k, v) <- vocab ]
              ++ [ "  Scenario: s"
                 , "    When I render pieces fills at year -1405 in style canaan"
                 ]
    -- Review finding (round 2), gap 3: `T.lines`/`joinLines` faithfully
    -- preserving (rather than always adding) a trailing newline was
    -- documented as true of today's corpus but never actually pinned by a
    -- test. This fixture has NO trailing newline at all; the rewritten
    -- file must not gain one it never had.
    it "--write never ADDS a trailing newline to a file that didn't have one" $ do
      tmpBase <- getTemporaryDirectory
      (uniqueFile, uh) <- openTempFile tmpBase "contract-runner-vocab-notrailingnl-test"
      hClose uh
      removeFile uniqueFile
      let dir = uniqueFile <> "-dir"
          path = dir </> "notrailingnl.feature"
          -- built with intercalate, deliberately with NO trailing "\n"
          original = T.intercalate "\n"
            [ "Feature: t"
            , "  Scenario: s"
            , "    When I render pieces fills at year -1405 in style canaan"
            ]
      original `shouldSatisfy` (not . T.isSuffixOf "\n")
      createDirectoryIfMissing True dir
      BS.writeFile path (TE.encodeUtf8 original)
      (`finally` removeDirectoryRecursive dir) $ do
        Vocab.vocabDir allSteps dir True
        rewritten <- TE.decodeUtf8 <$> BS.readFile path
        rewritten `shouldSatisfy` (not . T.isSuffixOf "\n")
        case parseFeature path rewritten of
          Left e -> expectationFailure (T.unpack e)
          Right f -> ftVocab f `shouldBe` Vocab.expectedVocab allSteps f
    -- Review finding (round 2), gap 4: the deviation making a parse error
    -- fatal in BOTH modes (not just verify, unlike the brief's literal
    -- stub -- see task-8-report.md) had no test proving --write actually
    -- exits non-zero on a bad file, rather than silently rewriting every
    -- OTHER file in the directory and reporting success anyway. Mixes one
    -- genuinely valid feature (which COULD be rewritten -- its
    -- expectedVocab is non-empty, so a single-phase implementation WOULD
    -- have touched it) with one that fails to parse at all (no
    -- "Feature:" line), and asserts the run fails loudly, naming the bad
    -- file, instead of printing any "rewritten" success message.
    --
    -- Review finding (round 3, controller ruling): this test originally
    -- only checked the exit code and message, never re-reading
    -- good.feature -- so it never actually caught that `vocabDir` used to
    -- be a SINGLE pass (`mapM one files`, writing as it went) that wrote
    -- good.feature's real bytes to disk BEFORE the aggregate parse-error
    -- check ran and reported failure: a partial write on a failed run,
    -- exactly the half-applied state the surgical requirement forbids.
    -- Fixed structurally in Vocab.hs (`vocabDir` is now two phases: parse
    -- EVERY file first, and only write -- ANY file -- once all of them
    -- have parsed cleanly). This test now closes the loop by asserting
    -- good.feature's bytes on disk are BYTE-IDENTICAL to what was written
    -- before the run, not merely that the command as a whole failed.
    it "--write exits non-zero and names the bad file when one feature in \
       \the directory fails to parse, rather than reporting success -- \
       \and leaves the OTHER, valid file's bytes on disk untouched" $ do
      tmpBase <- getTemporaryDirectory
      (uniqueFile, uh) <- openTempFile tmpBase "contract-runner-vocab-parseerr-test"
      hClose uh
      removeFile uniqueFile
      let dir = uniqueFile <> "-dir"
          goodPath = dir </> "good.feature"
          badPath = dir </> "bad.feature"
          good = T.unlines
            [ "Feature: t"
            , "  Scenario: s"
            , "    When I render pieces fills at year -1405 in style canaan"
            ]
          bad = T.unlines [ "this is not a feature file at all" ]
          goodBytes = TE.encodeUtf8 good
      createDirectoryIfMissing True dir
      BS.writeFile goodPath goodBytes
      BS.writeFile badPath (TE.encodeUtf8 bad)
      (`finally` removeDirectoryRecursive dir) $ do
        (out, result) <- captureStdout (Vocab.vocabDir allSteps dir True)
        result `shouldSatisfy` isLeft
        out `shouldSatisfy` T.isInfixOf "bad.feature"
        out `shouldSatisfy` (not . T.isInfixOf "rewrote")
        out `shouldSatisfy` (not . T.isInfixOf "nothing rewritten")
        -- the assertion the fixture was implicitly promising all along:
        -- good.feature -- which DOES have a non-empty expectedVocab, so
        -- there was real content a buggy single-phase write could have
        -- stamped onto it -- must be exactly the bytes it was before this
        -- failed run, not merely "the command reported failure".
        afterBytes <- BS.readFile goodPath
        afterBytes `shouldBe` goodBytes

  describe "@property scenarios" $ do
    it "substitutes holes and runs N times, all green on a law that holds" $ do
      let feat = T.unlines
            [ "Feature: t"
            , "  @property"
            , "  Scenario: determinism"
            , "    When I render pieces <somePieces> at year <someYear> in style canaan as a"
            , "    And I render pieces <somePieces> at year <someYear> in style canaan as b"
            , "    Then a equals b" ]
          fake url = pure (Right (bs, fromJust (A.decodeStrict bs)))
            where bs = TE.encodeUtf8 ("{\"echo\":\"" <> url <> "\"}")
          w = World "http://x" fake "" mempty False
      case parseFeature "t.feature" feat of
        Left e -> expectationFailure (T.unpack e)
        Right f -> case ftScenarios f of
          (sc : _) -> do
            v <- Prop.runScenarioProperty allSteps w 25 sc
            v `shouldBe` Passed
          [] -> expectationFailure "expected at least one scenario"
    it "reports the failing binding when the law breaks" $ do
      let feat = T.unlines
            [ "Feature: t"
            , "  @property"
            , "  Scenario: falsifiable"
            , "    When I GET /api/echo?y=<someYear>"
            , "    Then the response field neverThere equals nope" ]
          fake _ = pure (Right ("{}", A.object []))
          w = World "http://x" fake "" mempty False
      case parseFeature "t.feature" feat of
        Left e -> expectationFailure (T.unpack e)
        Right f -> case ftScenarios f of
          (sc : _) -> do
            v <- Prop.runScenarioProperty allSteps w 25 sc
            case v of
              Failed e -> e `shouldSatisfy` T.isInfixOf "someYear ="
              _ -> expectationFailure "law should have failed with its binding"
          [] -> expectationFailure "expected at least one scenario"
    -- Requirement 1: an unregistered `<hole>` must never silently pass --
    -- it must fail loudly, naming the specific hole that has no generator.
    it "an unregistered hole fails loudly, naming the hole" $ do
      let feat = T.unlines
            [ "Feature: t"
            , "  @property"
            , "  Scenario: mystery"
            , "    When I GET /api/echo?y=<someMysteryHole>" ]
          w = World "http://x" (\_ -> pure (Left "no")) "" mempty False
      case parseFeature "t.feature" feat of
        Left e -> expectationFailure (T.unpack e)
        Right f -> case ftScenarios f of
          (sc : _) -> do
            v <- Prop.runScenarioProperty allSteps w 5 sc
            case v of
              Failed e -> e `shouldSatisfy` T.isInfixOf "someMysteryHole"
              _ -> expectationFailure "an unregistered hole should fail loudly, naming it"
          [] -> expectationFailure "expected at least one scenario"
    -- Requirement 2: the diagnosis is reproducible run to run -- same
    -- inputs must generate the same bindings and the same verdict,
    -- including the exact text of a failing binding's counterexample.
    it "the same scenario run twice produces byte-identical verdicts \
       \(deterministic per-iteration seeding)" $ do
      let feat = T.unlines
            [ "Feature: t"
            , "  @property"
            , "  Scenario: falsifiable"
            , "    When I GET /api/echo?y=<someYear>"
            , "    Then the response field neverThere equals nope" ]
          fake _ = pure (Right ("{}", A.object []))
          w = World "http://x" fake "" mempty False
      case parseFeature "t.feature" feat of
        Left e -> expectationFailure (T.unpack e)
        Right f -> case ftScenarios f of
          (sc : _) -> do
            v1 <- Prop.runScenarioProperty allSteps w 25 sc
            v2 <- Prop.runScenarioProperty allSteps w 25 sc
            v1 `shouldBe` v2
          [] -> expectationFailure "expected at least one scenario"
    -- Requirement 4: the empty piece set (the scene monoid's identity) MUST
    -- be in genPieces' codomain, and MUST round-trip through substitution
    -- into a body the real step vocabulary still parses ("none", not "").
    it "genPieces can generate the empty piece set (the monoid identity)" $ do
      -- sublistOf independently keeps/drops each of the 10 pieces, so the
      -- empty set has probability (1/2)^10 = 1/1024 per sample -- a large
      -- sample count is needed for this to be reliable rather than flaky
      -- (20000 samples puts the odds of missing it entirely below 1e-8).
      --
      -- Deliberately exempt from the determinism law pinned above (the
      -- "same scenario run twice" test): this uses `generate`, QuickCheck's
      -- real-entropy driver, not `renderHole`'s fixed per-iteration seed.
      -- That's correct FOR THIS TEST -- it asks a codomain question
      -- ("can genPieces ever produce []?"), not a diagnosis-reproducibility
      -- one ("does the SAME run always report the SAME counterexample?").
      -- The actual property runner never calls `generate`; only this one
      -- test does, to sample the generator's range directly.
      samples <- generate (vectorOf 20000 Prop.genPieces)
      samples `shouldSatisfy` any (== PieceSet Set.empty)
    it "substituting the empty piece set renders 'none', which the real \
       \pieces step still parses and runs to Passed" $ do
      let feat = T.unlines
            [ "Feature: t"
            , "  Scenario: s"
            , "    When I render pieces <somePieces> at year 0 in style canaan as a" ]
          fake url = pure (Right (bs, fromJust (A.decodeStrict bs)))
            where bs = TE.encodeUtf8 ("{\"echo\":\"" <> url <> "\"}")
          w = World "http://x" fake "" mempty False
      case parseFeature "t.feature" feat of
        Left e -> expectationFailure (T.unpack e)
        Right f -> case ftScenarios f of
          (sc : _) -> do
            let sc' = Prop.substitute (Map.fromList [("somePieces", "none")]) sc
            case scSteps sc' of
              (Step _ b _ : _) ->
                b `shouldBe` "I render pieces none at year 0 in style canaan as a"
              [] -> expectationFailure "expected a step"
            v <- runScenario allSteps w sc'
            v `shouldBe` Passed
          [] -> expectationFailure "expected at least one scenario"

  describe "Check.dehole wired to Prop.substituteExamples" $ do
    -- Task 7 shipped `dehole = id`, documented as a placeholder Task 9
    -- would replace. This proves the wiring: a bare hole in an
    -- @property-tagged scenario (which some captures, like UrlPath,
    -- would otherwise accept unexamined) now reaches `classify` already
    -- substituted with a real, registered example value.
    it "a @property scene line whose captures are holes classifies by \
       \its substituted example, not the literal hole text" $ do
      case parseFeature "t.feature" $ T.unlines
             [ "Feature: t"
             , "  @property"
             , "  Scenario: s"
             , "    When I render pieces <somePieces> at year <someYear> in style canaan" ] of
        Left e -> expectationFailure (T.unpack e)
        Right f -> do
          Check.orphans allSteps f `shouldBe` []
          Check.ambiguous allSteps f `shouldBe` []
          Check.valueErrors allSteps f `shouldBe` []
    -- Review finding (Important, round 2): substitution used to run
    -- unconditionally, regardless of scenario tags -- but
    -- Prop.runWithProperties only ever substitutes for an @property
    -- scenario (see its `run1`); an UNTAGGED scenario runs through plain
    -- `runScenario`, which never substitutes. Substituting for `check`
    -- regardless of the tag would make a hole in an untagged scenario
    -- classify clean while it would actually run, for real, on the
    -- literal unresolved "<hole>" text -- a static verdict that lies
    -- about the dynamic one. This is the same body as the test above,
    -- MINUS the @property tag: it must NOT come back clean.
    it "the same hole, in a scenario NOT tagged @property, is a bad \
       \value -- check must not substitute a scenario run will not" $ do
      case parseFeature "t.feature" $ T.unlines
             [ "Feature: t"
             , "  Scenario: s"
             , "    When I render pieces <somePieces> at year <someYear> in style canaan" ] of
        Left e -> expectationFailure (T.unpack e)
        Right f -> do
          Check.orphans allSteps f `shouldBe` []
          Check.ambiguous allSteps f `shouldBe` []
          case Check.valueErrors allSteps f of
            [(_, b, errs)] -> do
              b `shouldBe` "I render pieces <somePieces> at year <someYear> in style canaan"
              errs `shouldSatisfy` (not . null)
              mapM_ (\(_, e) -> e `shouldSatisfy` T.isInfixOf "'<somePieces>' is not a piece") errs
            other -> expectationFailure
                       ("expected exactly one value-error step, got " <> show other)
    -- Requirement 1, closed: an UNREGISTERED hole must be an orphan in
    -- `check`, unconditionally -- even landing in a capture (UrlPath)
    -- that would otherwise accept its literal text unexamined, and even
    -- in an untagged scenario (where no other law here would have
    -- caught it at all, since dehole = id there and UrlPath rejects only
    -- whitespace, not "<...>" text).
    it "an unregistered hole is an orphan in check, naming the hole, \
       \even where the literal <hole> text would otherwise silently \
       \satisfy the capture it lands in (ruling R33)" $ do
      case parseFeature "t.feature" $ T.unlines
             [ "Feature: t"
             , "  Scenario: s"
             , "    When I GET /api/echo?y=<someMysteryHole>" ] of
        Left e -> expectationFailure (T.unpack e)
        Right f -> do
          Check.ambiguous allSteps f `shouldBe` []
          Check.valueErrors allSteps f `shouldBe` []
          case Check.orphans allSteps f of
            [(_, b)] -> b `shouldSatisfy` T.isInfixOf "someMysteryHole"
            other -> expectationFailure
                       ("expected exactly one orphan step, got " <> show other)
    -- Same shape, but registered AND unregistered holes both appear in
    -- one step: the unregistered one must still win (force VOrphan),
    -- not get silently ignored because its sibling hole resolves fine.
    it "one unregistered hole among several makes the whole step an \
       \orphan, even when its sibling holes are registered" $ do
      case parseFeature "t.feature" $ T.unlines
             [ "Feature: t"
             , "  @property"
             , "  Scenario: s"
             , "    When I render pieces <somePieces> at year <someMysteryHole> in style canaan" ] of
        Left e -> expectationFailure (T.unpack e)
        Right f -> do
          Check.ambiguous allSteps f `shouldBe` []
          Check.valueErrors allSteps f `shouldBe` []
          case Check.orphans allSteps f of
            [(_, b)] -> b `shouldSatisfy` T.isInfixOf "someMysteryHole"
            other -> expectationFailure
                       ("expected exactly one orphan step, got " <> show other)

-- Fix 5's stdout-capture helper: redirects the process's real stdout to a
-- temp file for the duration of `act` (via GHC.IO.Handle's fd-duplication,
-- the same technique `System.IO.Silently` uses), then restores it and
-- returns what was written plus `act`'s own `try` result. Needed because
-- checkDir writes straight to stdout and calls exitFailure -- there is no
-- other way to observe its output from inside a test.
--
-- Review finding (post-Task-7 review round 2), fix 1 (Important): this
-- used to be a happy-path sequence (redirect; try act; restore; close;
-- read; remove), with the restore/close/remove lines only reached if
-- `act` threw nothing but an ExitCode (the one thing `try`'s signature
-- catches). ANY other exception -- an IOException from a malformed
-- feature file, an ErrorCall from a partial pattern, an async exception
-- from a user interrupt -- would skip straight past every one of those
-- lines, leaving the process's real stdout permanently pointed at a now-
-- orphaned temp file for the rest of the hspec run: every later test's
-- output, and hspec's own failure report, would silently vanish into that
-- file instead of the terminal, and the handle plus the temp file would
-- leak. That is the worst failure mode available to a project whose gate
-- is evidence-before-assertions, and it fires exactly when something has
-- already gone wrong. Restructured so the restore and both handle closes
-- run UNCONDITIONALLY: `bracket` around the stdout duplicate/restore
-- (its release always runs, exception or not), nested inside a `finally`
-- that always closes the temp handle, nested inside a `finally` that
-- always removes the temp file. `try` still only catches `ExitCode` (that
-- part of the design is intentional and unchanged -- checkDir's only
-- non-local exit is exitFailure); a genuinely different exception now
-- still propagates out of captureStdout (so the test correctly reports it
-- as a failure, rather than being silently swallowed), but every layer of
-- cleanup below it has already run by the time it does.
captureStdout :: IO a -> IO (T.Text, Either ExitCode a)
captureStdout act = do
  tmpDir <- getTemporaryDirectory
  (path, h) <- openTempFile tmpDir "capture.txt"
  (`finally` removeFile path) $ do
    result <- (`finally` hClose h) $ do
      hSetEncoding h utf8
      bracket (hDuplicate stdout)
              (\old -> hDuplicateTo old stdout >> hClose old)
              (\_ -> do
                  hDuplicateTo h stdout
                  try act `finally` hFlush stdout)
    txt <- TIO.readFile path
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
