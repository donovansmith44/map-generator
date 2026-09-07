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
import Data.Proxy (Proxy (..))
import Data.Either (isLeft, isRight)
import Data.Maybe (fromJust, listToMaybe)
import qualified Data.Aeson as A
import qualified Data.ByteString as BS
import qualified Data.Map.Strict as Map
import qualified Data.Set as Set
import qualified Data.Text as T
import qualified Data.Text.Encoding as TE

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
    it "the subset step passes when one scene's resources are a genuine subset of the other's" $ do
      let wSub = w0 { bound = Map.fromList
            [ ("sceneA", (BS.empty, sceneVal ["r1", "r2"]))
            , ("sceneB", (BS.empty, sceneVal ["r1", "r2", "r3"])) ] }
          run = fromJust $ firstMatch Then "sceneA's resources are a subset of sceneB"
      r <- run wSub
      r `shouldSatisfy` isRight
    it "the subset step reports Left when a resource is genuinely absent from the other scene" $ do
      let wSub = w0 { bound = Map.fromList
            [ ("sceneA", (BS.empty, sceneVal ["r1", "r9"]))
            , ("sceneB", (BS.empty, sceneVal ["r1", "r2", "r3"])) ] }
          run = fromJust $ firstMatch Then "sceneA's resources are a subset of sceneB"
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

firstMatch :: Keyword -> T.Text -> Maybe (World -> IO (Either T.Text World))
firstMatch k t = listToMaybe
  [ f | StepDef k' _ _ m <- allSteps, k' == k, Just f <- [m t] ]

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
