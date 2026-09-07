{-# OPTIONS_GHC -Wno-orphans #-}
module Main where

import Test.Hspec
import Test.Hspec.QuickCheck (prop)
import Test.QuickCheck
import Gherkin.Ast
import Gherkin.Parse
import Gherkin.Render
import qualified Data.Text as T

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
      -- Tables are capped to a handful of small rows/cells: this doubly-
      -- nested listOf1 shares QuickCheck's ambient size with the
      -- scenario/step lists above it, and uncapped it compounds into
      -- minutes-long runs without adding coverage the law needs (every
      -- shape — one row, one cell, many of each — is still reachable).
      genStepArg = frequency
        [ (2, pure Nothing)
        , (1, Just . Table <$> resize 4 (listOf1 (resize 4 (listOf1 safeText))))
        ]
