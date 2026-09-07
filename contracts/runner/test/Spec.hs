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
import Control.Exception (try, bracket, evaluate, finally)
import Data.Proxy (Proxy (..))
import Data.Either (isLeft, isRight)
import Data.List (isInfixOf, sort)
import Data.IORef (modifyIORef, newIORef, readIORef)
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
import System.IO
  (stdout, openTempFile, hClose, hFlush, hSetEncoding, utf8,
   hSetNewlineMode, noNewlineTranslation)
import System.Timeout (timeout)

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
    -- Final-review cleanup, Fix 1: `lev` (Capture.hs) was rewritten from a
    -- naive exponential recursion to a polynomial DP table. This pins the
    -- one thing that fix must NOT change -- a genuine one-letter typo
    -- still names the real word it's closest to, by the same <=3 gate.
    it "a genuine one-letter typo still gets did-you-mean naming the \
       \intended word (the rewritten lev must score exactly as before)" $
      case parseCap @Piece "watre" of
        Left e  -> e `shouldSatisfy` T.isInfixOf "Did you mean: water?"
        Right _ -> expectationFailure "accepted a typo as a real piece"
    it "a long garbage capture returns promptly instead of hanging \
       \(Final review cleanup, Fix 1: lev is now O(n*m), not exponential, \
       \in input length -- the exponential version could hang `check`/`run` \
       \outright on a long garbage feature-file value, not merely run slow)" $ do
      -- 300 chars is instant for the O(n*m) replacement and nowhere near
      -- computable in any realistic time for the old exponential `lev`
      -- (which QuickCheck's own size ramp already had to be capped, at a
      -- mere `resize 8`, to avoid hitting -- see the property test above).
      let garbage = T.replicate 300 "q"
      result <- timeout (10 * 1000 * 1000) (evaluate (didYouMean styleNames garbage))
      case result of
        Nothing -> expectationFailure
          "didYouMean did not return within 10s on a 300-char garbage input"
        Just r  -> r `shouldBe` ""  -- nowhere near any real style name
    it "years parse within the frame and refuse outside it" $ do
      parseCap @Year "-1405" `shouldBe` Right (Year (-1405))
      parseCap @Year "9999" `shouldSatisfy` isLeft
    -- Final-review Fix 3: year 0 does not exist in this calendar (1 BC is
    -- immediately followed by AD 1) -- excluded from Year's universe BY
    -- TYPE, not merely rejected by the server at run time. A drawn 0 used
    -- to slip through parseCap and the generator both, making the
    -- (non-@target) subjects/census determinism properties hard-red for a
    -- reason unrelated to either law whenever a run happened to draw it.
    it "year 0 is rejected -- there is no year zero in this calendar" $ do
      case parseCap @Year "0" of
        Left e  -> e `shouldSatisfy` T.isInfixOf "year 0 does not exist"
        Right _ -> expectationFailure "accepted year 0, which does not exist"
    prop "renderCap is a right inverse of parseCap for years, excluding \
         \year 0 (there is no year zero)" $
      \(y :: Int) ->
        -- Map an arbitrary Int onto -4004..100 EXCLUDING 0: candidate
        -- ranges over the 4104 consecutive values -4004..99, and every
        -- non-negative candidate is shifted up by one to skip 0 --
        -- covering -4004..-1 and 1..100 exactly, with no year-0 case ever
        -- generated (unlike a plain `mod 4105`, which would hit 0 about
        -- 1 run in 4105 and intermittently break this law once Year
        -- genuinely excludes it).
        let candidate = (-4004) + (abs y `mod` 4104)
            y' = if candidate >= 0 then candidate + 1 else candidate
        in parseCap @Year (renderCap (Year y')) === Right (Year y')
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

  describe "capture hardening (Stage 1 Task 2)" $ do
    it "editDistance agrees with the textbook answer on known pairs" $ do
      editDistance "kitten" "sitting" `shouldBe` 3
      editDistance "" "abc"           `shouldBe` 3
      editDistance "abc" ""           `shouldBe` 3
      editDistance "abc" "abc"        `shouldBe` 0
      editDistance "topografy" "topography" `shouldBe` 2
    prop "editDistance is symmetric" $ \a b ->
      editDistance (T.pack a) (T.pack b) === editDistance (T.pack b) (T.pack a)
    prop "editDistance is bounded by the longer string" $ \a b ->
      editDistance (T.pack a) (T.pack b) <= max (length a) (length b)
    it "didYouMean answers on a long garbage value instead of hanging" $ do
      -- 400 chars of junk against the piece universe. The old triple
      -- recursion is exponential in the shorter string and never
      -- returns; one second is three orders of magnitude of headroom.
      let junk = T.replicate 400 "q"
      r <- timeout 1000000 (evaluate (T.length (didYouMean (map pieceText [minBound .. maxBound]) junk)))
      r `shouldSatisfy` \x -> case x of Just _ -> True; Nothing -> False
    prop "every Piece round-trips through renderCap/parseCap" $ \(p :: Piece) ->
      parseCap (renderCap p) === Right p
    prop "every PieceSet round-trips, INCLUDING the empty set" $ \(ps :: PieceSet) ->
      parseCap (renderCap ps) === Right ps
    prop "every StyleName round-trips" $ \(s :: StyleName) ->
      parseCap (renderCap s) === Right s
    prop "every Year in the frame round-trips" $ \(y :: Year) ->
      parseCap (renderCap y) === Right y

  describe "step patterns" $ do
    let p = lit "I render pieces " *> ((,) <$> capUntil @PieceSet " at year " <*> capRest @Year)
    it "matches and yields typed captures" $
      matchP p "I render pieces water, ground at year -1405"
        `shouldBe` Right (PieceSet (Set.fromList [Ground, Water]), Year (-1405))
    it "a bad capture fails with the capture's own error, not a match miss" $
      case matchP p "I render pieces water, topografy at year -1405" of
        Left e  -> e `shouldSatisfy` T.isInfixOf "not a piece"
        Right _ -> expectationFailure "matched garbage"
    -- Final-review Fix 5 (deferred minor, promoted): this used to assert
    -- only `isLeft` -- a test promising more than it delivers ("says
    -- which literal") while checking nothing about WHICH literal, inside
    -- the very suite built to outlaw exactly that gap (see MEMORY:
    -- verify-distinct-not-nonnull). `lit`'s own error text already names
    -- the literal it expected (Pattern.hs's `lit`); the test now checks
    -- the message actually names it.
    it "a literal mismatch says which literal" $
      case matchP p "I paint pieces water at year 1" of
        Left e  -> e `shouldSatisfy` T.isInfixOf "I render pieces"
        Right _ -> expectationFailure "expected a literal mismatch to fail"
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
    -- Final-review Fix 5 (deferred minor, promoted): the "expected
    -- literal" prefix is load-bearing for the ENTIRE three-state Claim
    -- classification (World.mkStep, R4) -- it is the one signal that
    -- separates NoMatch ("not this step, try the next one") from
    -- ClaimError ("this step, but the value is bad"). Nothing pinned that
    -- a genuine capture failure (parseCap, not a missing literal) never
    -- accidentally produces that exact prefix; a future FromCapture
    -- instance that did would silently misclassify a bad value as a step
    -- that "doesn't apply" instead of reporting it, with nothing going
    -- red to say so. Probed over arbitrary text against every
    -- FromCapture instance actually registered in this runner.
    -- `resize 8`: a bad piece/style/projection name's parseCap runs it
    -- through Capture.didYouMean, whose Levenshtein distance (`lev`) is
    -- the naive, unmemoized recursive definition -- fine for the short
    -- garbage names a real bad step actually produces, but exponential
    -- in input length, so QuickCheck's default size ramp (up to ~99,
    -- growing every test) would eventually hand it a string long enough
    -- to hang the whole suite. Bounding the generated string's size
    -- keeps this property fast while still covering the space of
    -- realistic bad captures (empty, single-char, short garbage,
    -- comma-separated junk) that the law actually needs to hold over.
    prop "no registered FromCapture instance's parseCap error ever begins \
         \\"expected literal\" -- capRest's Left must never be mistaken \
         \for a NoMatch (R4)" $ forAll (resize 8 arbitrary) $ \s ->
      let t = T.pack (s :: String)
          voidR :: Either T.Text a -> Either T.Text ()
          voidR = either Left (const (Right ()))
          leftOf (Left e)  = [e]
          leftOf (Right ()) = []
          errs = concatMap leftOf
            [ voidR (parseCap @Piece t)
            , voidR (parseCap @PieceSet t)
            , voidR (parseCap @Year t)
            , voidR (parseCap @StyleName t)
            , voidR (parseCap @FixtureRef t)
            , voidR (parseCap @FixtureRefFreeText t)
            , voidR (parseCap @UrlPath t)
            , voidR (parseCap @BindName t)
            , voidR (parseCap @MaskShape t)
            , voidR (parseCap @ProjName t)
            ]
      in all (not . T.isPrefixOf "expected literal") errs

  describe "world and steps" $ do
    let -- The body echoes the URL it was fetched from (rather than a fixed
        -- string), so any two renders of DIFFERENT URLs produce genuinely
        -- different bound values — needed so the equality step's negative
        -- case (below) actually discriminates instead of trivially passing
        -- against a stub that always returns success.
        fake url = pure (Right (raw, v))
          where raw = TE.encodeUtf8 ("{\"scene\":\"" <> url <> "\",\"labels\":[]}")
                v   = fromJust (A.decodeStrict raw)
        w0 = mkWorld "http://x" fake "test/fixtures"
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

  describe "HTTP status law (Phase S review, fix 1): a non-2xx status is a Left, \
           \naming the code and the url" $ do
    -- `checkStatus` is extracted specifically so this law is testable
    -- without a live server (this task must not start or restart one --
    -- blessing fixtures against a live server is the NEXT task's job).
    -- Both `httpTransport` and `httpTransportRaw` now call it before
    -- doing anything else with a response; without it, "fetching
    -- {name}'s first resource twice yields identical bytes" (NOT
    -- @target -- expected GREEN) would report green against a server
    -- whose /api/resource always 404s, since two fetches of the same
    -- error page are byte-identical.
    it "every 2xx code counts as success" $ do
      checkStatus "http://x/foo" 200 `shouldSatisfy` isRight
      checkStatus "http://x/foo" 204 `shouldSatisfy` isRight
      checkStatus "http://x/foo" 299 `shouldSatisfy` isRight
    it "a 404 fails, naming the status code and the exact url" $
      case checkStatus "http://x/api/resource?id=r1" 404 of
        Left e  -> do
          e `shouldSatisfy` T.isInfixOf "404"
          e `shouldSatisfy` T.isInfixOf "http://x/api/resource?id=r1"
        Right _ -> expectationFailure "expected a 404 to fail"
    it "a 500 fails, naming the status code" $
      case checkStatus "http://x/foo" 500 of
        Left e  -> e `shouldSatisfy` T.isInfixOf "500"
        Right _ -> expectationFailure "expected a 500 to fail"
    it "a 3xx redirect also fails -- not silently treated as success" $
      case checkStatus "http://x/foo" 301 of
        Left e  -> e `shouldSatisfy` T.isInfixOf "301"
        Right _ -> expectationFailure "expected a 3xx to fail"

  describe "fact-tier steps (Task 10): GET-as binding, masked whole-body fixture equality" $ do
    it "GET-as binds under a name" $ do
      let fake _ = pure (Right ("[]", fromJust (A.decodeStrict "[]")))
          w = mkWorld "http://x" fake ""
      Just get <- pure (firstMatch When "I GET /api/subjects?year=-1405 as first")
      Right w1 <- get w
      Map.member "first" (bound w1) `shouldBe` True
    it "masked fixture equality: body pinned whole, mask shape-checked" $ do
      let o = "{\"version\":\"0.1.0\",\"graphPin\":\"0123456789abcdef\"}"
          fake _ = pure (Right (o, fromJust (A.decodeStrict o)))
          w = mkWorld "http://x" fake "test/fixtures"
      Just get <- pure (firstMatch When "I GET /api/contract")
      Right w1 <- get w
      Just chk <- pure (firstMatch Then
        "the response equals fixture \"contract\" masking graphPin as sixteen hex characters")
      r <- chk w1
      r `shouldSatisfy` isRight
    it "a masked field with the WRONG shape still fails, naming the shape violation" $ do
      let o = "{\"version\":\"0.1.0\",\"graphPin\":\"nope\"}"
          fake _ = pure (Right (o, fromJust (A.decodeStrict o)))
          w = mkWorld "http://x" fake "test/fixtures"
      Just get <- pure (firstMatch When "I GET /api/contract")
      Right w1 <- get w
      Just chk <- pure (firstMatch Then
        "the response equals fixture \"contract\" masking graphPin as sixteen hex characters")
      r <- chk w1
      case r of
        Left e  -> e `shouldSatisfy` T.isInfixOf "not 16 hex"
        Right _ -> expectationFailure "expected a wrongly-shaped mask value to fail"
    it "an unmasked difference anywhere in the body fails, naming the fixture" $ do
      let o = "{\"version\":\"9.9.9\",\"graphPin\":\"0123456789abcdef\"}"
          fake _ = pure (Right (o, fromJust (A.decodeStrict o)))
          w = mkWorld "http://x" fake "test/fixtures"
      Just get <- pure (firstMatch When "I GET /api/contract")
      Right w1 <- get w
      Just chk <- pure (firstMatch Then
        "the response equals fixture \"contract\" masking graphPin as sixteen hex characters")
      r <- chk w1
      case r of
        Left e  -> e `shouldSatisfy` T.isInfixOf "differs from fixture"
        Right _ -> expectationFailure "expected an unmasked body difference to fail"
    it "a masked field that is simply missing fails, naming the field" $ do
      let o = "{\"version\":\"0.1.0\"}"
          fake _ = pure (Right (o, fromJust (A.decodeStrict o)))
          w = mkWorld "http://x" fake "test/fixtures"
      Just get <- pure (firstMatch When "I GET /api/contract")
      Right w1 <- get w
      Just chk <- pure (firstMatch Then
        "the response equals fixture \"contract\" masking graphPin as sixteen hex characters")
      r <- chk w1
      case r of
        Left e  -> e `shouldSatisfy` T.isInfixOf "no field graphPin"
        Right _ -> expectationFailure "expected a missing masked field to fail"
    it "bless mode writes the body with the masked field replaced by MASKED, never the actual secret value" $ do
      tmpBase <- getTemporaryDirectory
      (uniqueFile, uh) <- openTempFile tmpBase "contract-runner-bless-mask-test"
      hClose uh
      removeFile uniqueFile
      let dir = uniqueFile <> "-dir"
          o = "{\"version\":\"0.1.0\",\"graphPin\":\"0123456789abcdef\"}"
          fake _ = pure (Right (o, fromJust (A.decodeStrict o)))
          w = (mkWorld "http://x" fake dir) { blessMode = True }
      createDirectoryIfMissing True dir
      (`finally` removeDirectoryRecursive dir) $ do
        Just get <- pure (firstMatch When "I GET /api/contract")
        Right w1 <- get w
        Just chk <- pure (firstMatch Then
          "the response equals fixture \"contract\" masking graphPin as sixteen hex characters")
        r <- chk w1
        r `shouldSatisfy` isRight
        written <- BS.readFile (dir </> "contract.json")
        (A.decodeStrict written :: Maybe A.Value) `shouldBe`
          A.decodeStrict "{\"version\":\"0.1.0\",\"graphPin\":\"MASKED\"}"

  describe "blessOrCompare diagnostics (Final review Fix 5): report WHAT \
           \differs, not just the fixture's name" $ do
    -- Against a 2.4 MB manifest, "response differs from fixture X" names
    -- the fixture but not what actually differs -- not a diagnosis, just
    -- a pointer back at a huge file. `firstDiff` (Steps.hs) is exercised
    -- here through the real step, over real JSON, at three shapes of
    -- disagreement: a plain top-level field, a value nested inside an
    -- array of objects, and a genuine shape (key-set) mismatch.
    it "names the first differing top-level field, with both values" $ do
      tmpBase <- getTemporaryDirectory
      (uniqueFile, uh) <- openTempFile tmpBase "contract-runner-blesscompare-field-test"
      hClose uh
      removeFile uniqueFile
      let dir = uniqueFile <> "-dir"
          fixtureBytes = "{\"a\":1,\"b\":2}"
          actualBytes  = "{\"a\":1,\"b\":3}"
          fake _ = pure (Right (actualBytes, fromJust (A.decodeStrict actualBytes)))
          w = mkWorld "http://x" fake dir
      createDirectoryIfMissing True dir
      BS.writeFile (dir </> "foo.json") fixtureBytes
      (`finally` removeDirectoryRecursive dir) $ do
        Just get <- pure (firstMatch When "I GET /api/foo")
        Right w1 <- get w
        Just chk <- pure (firstMatch Then "the response equals fixture \"foo\"")
        r <- chk w1
        case r of
          Left e -> do
            e `shouldSatisfy` T.isInfixOf "foo"
            e `shouldSatisfy` T.isInfixOf "$.b"
            e `shouldSatisfy` T.isInfixOf "2"
            e `shouldSatisfy` T.isInfixOf "3"
          Right _ -> expectationFailure "expected a genuine field difference to fail"
    it "names the first differing path inside a nested array, not merely \
       \the fixture's name" $ do
      tmpBase <- getTemporaryDirectory
      (uniqueFile, uh) <- openTempFile tmpBase "contract-runner-blesscompare-array-test"
      hClose uh
      removeFile uniqueFile
      let dir = uniqueFile <> "-dir"
          fixtureBytes = "{\"resources\":[{\"id\":\"r1\"},{\"id\":\"r2\"}]}"
          actualBytes  = "{\"resources\":[{\"id\":\"r1\"},{\"id\":\"rX\"}]}"
          fake _ = pure (Right (actualBytes, fromJust (A.decodeStrict actualBytes)))
          w = mkWorld "http://x" fake dir
      createDirectoryIfMissing True dir
      BS.writeFile (dir </> "scene.json") fixtureBytes
      (`finally` removeDirectoryRecursive dir) $ do
        Just get <- pure (firstMatch When "I GET /api/scene")
        Right w1 <- get w
        Just chk <- pure (firstMatch Then "the response equals fixture \"scene\"")
        r <- chk w1
        case r of
          Left e -> do
            e `shouldSatisfy` T.isInfixOf "$.resources[1].id"
            e `shouldSatisfy` T.isInfixOf "r2"
            e `shouldSatisfy` T.isInfixOf "rX"
          Right _ -> expectationFailure "expected a genuine nested-array difference to fail"
    it "names the keys that differ when the two shapes themselves disagree" $ do
      tmpBase <- getTemporaryDirectory
      (uniqueFile, uh) <- openTempFile tmpBase "contract-runner-blesscompare-shape-test"
      hClose uh
      removeFile uniqueFile
      let dir = uniqueFile <> "-dir"
          fixtureBytes = "{\"a\":1}"
          actualBytes  = "{\"a\":1,\"extra\":true}"
          fake _ = pure (Right (actualBytes, fromJust (A.decodeStrict actualBytes)))
          w = mkWorld "http://x" fake dir
      createDirectoryIfMissing True dir
      BS.writeFile (dir </> "shape.json") fixtureBytes
      (`finally` removeDirectoryRecursive dir) $ do
        Just get <- pure (firstMatch When "I GET /api/shape")
        Right w1 <- get w
        Just chk <- pure (firstMatch Then "the response equals fixture \"shape\"")
        r <- chk w1
        case r of
          Left e -> do
            e `shouldSatisfy` T.isInfixOf "keys differ"
            e `shouldSatisfy` T.isInfixOf "extra"
          Right _ -> expectationFailure "expected a genuine key-set difference to fail"

  describe "scene algebra steps (Task 11): piece attribution, composition, dress-locality, resource identity" $ do
    it "every feature entry carries a piece field: passes when every entry has one" $ do
      let v = A.object ["features" A..= ([ A.object ["piece" A..= ("ground" :: T.Text)]
                                          , A.object ["piece" A..= ("water" :: T.Text)] ] :: [A.Value])]
          w = (mkWorld "http://x" (\_ -> pure (Left "no")) "")
                { bound = Map.fromList [("_last", (BS.empty, v))] }
          run = fromJust $ firstMatch Then "every feature entry carries a piece field"
      r <- run w
      r `shouldSatisfy` isRight
    it "every feature entry carries a piece field: an entry missing it fails honestly \
       \(the v0.1 wart), naming it -- @target: matched and honestly red, not an orphan" $ do
      let v = A.object ["features" A..= ([A.object ["kind" A..= ("x" :: T.Text)]] :: [A.Value])]
          w = (mkWorld "http://x" (\_ -> pure (Left "no")) "")
                { bound = Map.fromList [("_last", (BS.empty, v))] }
          run = fromJust $ firstMatch Then "every feature entry carries a piece field"
      r <- run w
      case r of
        Left e  -> e `shouldSatisfy` T.isInfixOf "piece attribution"
        Right _ -> expectationFailure "expected the v0.1 wart to fail honestly, not silently pass"
    -- Phase S review, cheap fix 4: `V.all` over an empty Vector is
    -- vacuously True -- without an explicit guard, an empty manifest
    -- would report this @target step as already met, which is false:
    -- there is nothing here to demonstrate piece attribution AT ALL.
    it "every feature entry carries a piece field: an EMPTY features array \
       \fails, not vacuously passes" $ do
      let v = A.object ["features" A..= ([] :: [A.Value])]
          w = (mkWorld "http://x" (\_ -> pure (Left "no")) "")
                { bound = Map.fromList [("_last", (BS.empty, v))] }
          run = fromJust $ firstMatch Then "every feature entry carries a piece field"
      r <- run w
      case r of
        Left e  -> e `shouldSatisfy` T.isInfixOf "empty manifest"
        Right _ -> expectationFailure "expected an empty manifest to fail, not vacuously pass"
    -- Phase S review, cheap fix 2: this used to render as the BARE
    -- string "Right ()" (`() <$ other` erases the payload before
    -- `show`), naming neither the field nor the problem. Now prefixed
    -- the way `resourceIds` (this module's own `where` clause) prefixes
    -- its own shape mismatch -- at minimum naming the field this step
    -- actually looked for, matching the established house style.
    it "every feature entry carries a piece field: a non-array 'features' \
       \field fails, naming the field it looked for" $ do
      let v = A.object ["features" A..= ("not an array" :: T.Text)]
          w = (mkWorld "http://x" (\_ -> pure (Left "no")) "")
                { bound = Map.fromList [("_last", (BS.empty, v))] }
          run = fromJust $ firstMatch Then "every feature entry carries a piece field"
      r <- run w
      case r of
        Left e  -> e `shouldSatisfy` T.isInfixOf "no features array"
        Right _ -> expectationFailure "expected a non-array features field to fail"

    it "combining threads the SAME bound year into the union render -- not a \
       \hardcoded one (the plan's own self-review flag)" $ do
      -- The fake REJECTS any request not carrying "year=-77": if the
      -- combine step hardcoded a different year (as the plan's own
      -- sketch originally did, at -1405), this test would fail on that
      -- mismatch rather than merely coincidentally passing.
      let fake url
            | "year=-77" `T.isInfixOf` url = pure (Right (raw, val))
            | otherwise = pure (Left ("wrong year threaded into union render url: " <> url))
            where
              raw = TE.encodeUtf8 url
              active :: [T.Text]
              active = [ "ground" | "relief=1" `T.isInfixOf` url ]
                    ++ [ "water"  | not ("topo=0" `T.isInfixOf` url) ]
              val = A.object ["resources" A..= [ A.object ["id" A..= p] | p <- active ]]
          w0 = mkWorld "http://x" fake ""
      Just r1 <- pure (firstMatch When "I render pieces ground at year -77 in style canaan as sceneA")
      Right w1 <- r1 w0
      Just r2 <- pure (firstMatch When "I render pieces water at year -77 in style canaan as sceneB")
      Right w2 <- r2 w1
      Just chk <- pure (firstMatch Then "combining sceneA and sceneB equals rendering ground plus water")
      r <- chk w2
      r `shouldSatisfy` isRight
    it "combining threads the SAME bound STYLE into the union render, too -- \
       \not the hardcoded \"canaan\" of the original sketch (Phase S review \
       \fixes 2/3)" $ do
      -- The fake REJECTS any request not carrying "style=slate": both
      -- parts are rendered in slate here, and a combine step that still
      -- hardcoded "canaan" for the union render would request the wrong
      -- style and fail this test, rather than merely coincidentally
      -- passing (the real corpus scenario never exercises this because
      -- it always uses canaan for both parts).
      let fake url
            | "style=slate" `T.isInfixOf` url = pure (Right (raw, val))
            | otherwise = pure (Left ("wrong style threaded into union render url: " <> url))
            where
              raw = TE.encodeUtf8 url
              active :: [T.Text]
              active = [ "ground" | "relief=1" `T.isInfixOf` url ]
                    ++ [ "water"  | not ("topo=0" `T.isInfixOf` url) ]
              val = A.object ["resources" A..= [ A.object ["id" A..= p] | p <- active ]]
          w0 = mkWorld "http://x" fake ""
      Just r1 <- pure (firstMatch When "I render pieces ground at year -77 in style slate as sceneA")
      Right w1 <- r1 w0
      Just r2 <- pure (firstMatch When "I render pieces water at year -77 in style slate as sceneB")
      Right w2 <- r2 w1
      Just chk <- pure (firstMatch Then "combining sceneA and sceneB equals rendering ground plus water")
      r <- chk w2
      r `shouldSatisfy` isRight
    it "combining reports Left naming the algebra failure when the union's \
       \resources are genuinely not the union of its parts'" $ do
      let fake url
            | not ("year=-77" `T.isInfixOf` url) = pure (Left "wrong year")
            | otherwise = pure (Right (TE.encodeUtf8 url, val))
            where
              isUnion = "relief=1" `T.isInfixOf` url && not ("topo=0" `T.isInfixOf` url)
              activePieces :: [T.Text]
              activePieces = [ "ground" | "relief=1" `T.isInfixOf` url ]
                          ++ [ "water"  | not ("topo=0" `T.isInfixOf` url) ]
              -- the union render alone gets an extra id absent from
              -- either part -- a genuine, detectable violation of the
              -- union law, not merely a coincidental fixture mismatch.
              ids = if isUnion then activePieces ++ ["bogus"] else activePieces
              val = A.object ["resources" A..= [ A.object ["id" A..= p] | p <- ids ]]
          w0 = mkWorld "http://x" fake ""
      Just r1 <- pure (firstMatch When "I render pieces ground at year -77 in style canaan as sceneA")
      Right w1 <- r1 w0
      Just r2 <- pure (firstMatch When "I render pieces water at year -77 in style canaan as sceneB")
      Right w2 <- r2 w1
      Just chk <- pure (firstMatch Then "combining sceneA and sceneB equals rendering ground plus water")
      r <- chk w2
      case r of
        Left e  -> e `shouldSatisfy` T.isInfixOf "not the union of its parts"
        Right _ -> expectationFailure "expected a genuinely non-union result to fail"
    it "combining without a prior render fails, naming that no year/style \
       \was recorded (no ill-typed \"bound _year is not a number\" branch \
       \is reachable any more -- lastRender is typed)" $ do
      let w = mkWorld "http://x" (\_ -> pure (Left "no")) ""
          run = fromJust $ firstMatch Then
            "combining sceneA and sceneB equals rendering ground plus water"
      r <- run w
      case r of
        Left e  -> e `shouldSatisfy` T.isInfixOf "no year/style recorded"
        Right _ -> expectationFailure "expected combining with no prior render to fail"

    -- Final-review Fix 2: equal geometry ids alone is satisfiable by a
    -- server that ignores `style=` outright, so the step must also prove
    -- the remainder genuinely changed once geometry ids are masked out
    -- (see Steps.hs's own comment on this definition). The old version of
    -- this test used byte-IDENTICAL bodies for "dressed" and "redressed"
    -- (same ids, nothing else either) and asserted `isRight` -- exactly
    -- the case the fix closes, so it now needs a genuinely differing
    -- dress (a "dress" field standing in for the real payload's
    -- style-dependent fields) to still pass.
    it "dress-locality: two restyled scenes with identical resource ids \
       \but genuinely different dress pass" $ do
      let sceneVal :: T.Text -> [T.Text] -> A.Value
          sceneVal dress ids = A.object
            [ "resources" A..= map (\i -> A.object ["id" A..= i]) ids
            , "dress" A..= dress ]
          w = (mkWorld "http://x" (\_ -> pure (Left "no")) "")
                { bound = Map.fromList
                    [ ("dressed", (BS.empty, sceneVal "canaan" ["r1", "r2"]))
                    , ("redressed", (BS.empty, sceneVal "slate" ["r1", "r2"])) ] }
          run = fromJust $ firstMatch Then "dressed and redressed differ only in dress, never in geometry"
      r <- run w
      r `shouldSatisfy` isRight
    -- The case the fix actually closes: equal geometry ids AND an
    -- otherwise byte-identical body (nothing restyled at all) must now
    -- FAIL -- a server that silently ignored `style=` used to satisfy
    -- this step's old, ids-only check.
    it "dress-locality: identical bodies (dress unchanged too) fail -- \
       \equal geometry ids alone must not be enough to pass" $ do
      let sceneVal :: [T.Text] -> A.Value
          sceneVal ids = A.object
            [ "resources" A..= map (\i -> A.object ["id" A..= i]) ids
            , "dress" A..= ("same" :: T.Text) ]
          w = (mkWorld "http://x" (\_ -> pure (Left "no")) "")
                { bound = Map.fromList
                    [ ("dressed", (BS.empty, sceneVal ["r1", "r2"]))
                    , ("redressed", (BS.empty, sceneVal ["r1", "r2"])) ] }
          run = fromJust $ firstMatch Then "dressed and redressed differ only in dress, never in geometry"
      r <- run w
      case r of
        Left e  -> e `shouldSatisfy` T.isInfixOf "did not actually change"
        Right _ -> expectationFailure
          "expected an unchanged body (dress included) to fail, not pass on ids alone"
    it "dress-locality: a genuine geometry-id difference fails, naming that dress is not local" $ do
      let sceneVal :: [T.Text] -> A.Value
          sceneVal ids = A.object ["resources" A..= map (\i -> A.object ["id" A..= i]) ids]
          w = (mkWorld "http://x" (\_ -> pure (Left "no")) "")
                { bound = Map.fromList
                    [ ("dressed", (BS.empty, sceneVal ["r1", "r2"]))
                    , ("redressed", (BS.empty, sceneVal ["r1", "r9"])) ] }
          run = fromJust $ firstMatch Then "dressed and redressed differ only in dress, never in geometry"
      r <- run w
      case r of
        Left e  -> e `shouldSatisfy` T.isInfixOf "dress is not local"
        Right _ -> expectationFailure "expected differing geometry ids to fail"

    -- ---------- the sweep: the identity law ----------
    it "the identity law: combining a scene with an EMPTY scene is the \
       \scene itself (v0.1 combining = resource-set union)" $ do
      let ids ns = idsValue (ns :: [T.Text])
          w = (mkWorld "http://x" (\_ -> pure (Left "no")) "")
                { bound = Map.fromList
                    [ ("some",  (BS.empty, ids ["r1", "r2"]))
                    , ("empty", (BS.empty, ids [])) ] }
          run = fromJust $ firstMatch Then "combining some and empty equals some"
      r <- run w
      r `shouldSatisfy` isRight
    it "the identity law fails, naming the ids that differ, when the union \
       \genuinely is not the scene it claims to be" $ do
      let ids ns = idsValue (ns :: [T.Text])
          w = (mkWorld "http://x" (\_ -> pure (Left "no")) "")
                { bound = Map.fromList
                    [ ("some",  (BS.empty, ids ["r1", "r2"]))
                      -- not empty at all: stacking it ADDS r9, so the
                      -- identity law is genuinely violated here
                    , ("empty", (BS.empty, ids ["r9"])) ] }
          run = fromJust $ firstMatch Then "combining some and empty equals some"
      r <- run w
      case r of
        Left e -> do
          e `shouldSatisfy` T.isInfixOf "r9"
          e `shouldSatisfy` T.isInfixOf "only in the composed side"
        Right _ -> expectationFailure
          "expected a scene that adds a resource to break the identity law"
    it "the identity law fails naming an unbound scene name rather than \
       \treating it as an empty resource set" $ do
      let w = mkWorld "http://x" (\_ -> pure (Left "no")) ""
          run = fromJust $ firstMatch Then "combining some and empty equals some"
      r <- run w
      case r of
        Left e  -> e `shouldSatisfy` T.isInfixOf "unbound some"
        Right _ -> expectationFailure "expected an unbound scene name to fail"

    -- ---------- the sweep: the singleton fold ----------
    it "the singleton fold: rendering each piece alone and stacking them \
       \rebuilds the whole map" $ do
      let fake url = pure (Right (TE.encodeUtf8 url, wireResources url))
          w0' = mkWorld "http://x" fake ""
      Just r1 <- pure (firstMatch When
        "I render pieces ground, water at year -77 in style canaan as whole")
      Right w1 <- r1 w0'
      Just chk <- pure (firstMatch Then
        "rendering each piece of ground, water alone and combining them equals whole")
      r <- chk w1
      r `shouldSatisfy` isRight
    it "the singleton fold reports Left, naming the ids that went missing, \
       \when the whole carries something no single piece produced" $ do
      -- The fake smuggles an extra id into any render of two or more
      -- pieces -- something that appears only when pieces are combined,
      -- which is exactly what this law exists to catch.
      let fake url = pure (Right (TE.encodeUtf8 url, idsValue (bonus (activeInUrl url))))
          bonus as = if length as >= 2 then as ++ ["bonus"] else as
          w0' = mkWorld "http://x" fake ""
      Just r1 <- pure (firstMatch When
        "I render pieces ground, water at year -77 in style canaan as whole")
      Right w1 <- r1 w0'
      Just chk <- pure (firstMatch Then
        "rendering each piece of ground, water alone and combining them equals whole")
      r <- chk w1
      case r of
        Left e -> do
          e `shouldSatisfy` T.isInfixOf "does not rebuild whole"
          e `shouldSatisfy` T.isInfixOf "bonus"
        Right _ -> expectationFailure
          "expected an id that appears only in the combined render to fail the fold"
    it "the singleton fold threads the SAME year AND style into every \
       \single-piece render -- neither is hardcoded" $ do
      -- The fake REJECTS any request not carrying both, so a fold that
      -- rendered its singles at a different year or in a different style
      -- fails here instead of coincidentally passing.
      let fake url
            | "year=-77" `T.isInfixOf` url && "style=slate" `T.isInfixOf` url =
                pure (Right (TE.encodeUtf8 url, wireResources url))
            | otherwise = pure (Left ("wrong year/style in single-piece render url: " <> url))
          w0' = mkWorld "http://x" fake ""
      Just r1 <- pure (firstMatch When
        "I render pieces ground, water at year -77 in style slate as whole")
      Right w1 <- r1 w0'
      Just chk <- pure (firstMatch Then
        "rendering each piece of ground, water alone and combining them equals whole")
      r <- chk w1
      r `shouldSatisfy` isRight
    it "the singleton fold SKIPS an empty piece set -- a fold over no \
       \pieces demonstrates nothing, and must not pass vacuously" $ do
      let fake url = pure (Right (TE.encodeUtf8 url, wireResources url))
          w0' = mkWorld "http://x" fake ""
      Just r1 <- pure (firstMatch When
        "I render pieces none at year -77 in style canaan as whole")
      Right w1 <- r1 w0'
      Just chk <- pure (firstOutcome Then
        "rendering each piece of none alone and combining them equals whole")
      r <- chk w1
      case r of
        StepSkipped why -> why `shouldSatisfy` T.isInfixOf "empty"
        StepOk _ -> expectationFailure
          "an empty fold must SKIP, not pass vacuously (its union is trivially empty)"
        StepFailed e -> expectationFailure
          ("an empty fold must SKIP, not fail: " <> T.unpack e)
    it "the singleton fold without a prior render fails, naming that no \
       \year/style was recorded" $ do
      let w = mkWorld "http://x" (\_ -> pure (Left "no")) ""
          run = fromJust $ firstMatch Then
            "rendering each piece of ground, water alone and combining them equals whole"
      r <- run w
      case r of
        Left e  -> e `shouldSatisfy` T.isInfixOf "no year/style recorded"
        Right _ -> expectationFailure "expected a fold with no prior render to fail"

    -- ---------- the sweep: the empty-list law ----------
    it "the empty-list law passes when the WHOLE body is []" $ do
      let fake _ = pure (Right ("[]", fromJust (A.decodeStrict "[]")))
          w = mkWorld "http://x" fake ""
      Just get <- pure (firstMatch When "I GET /api/changes?from=-3000&to=-3000")
      Right w1 <- get w
      Just chk <- pure (firstMatch Then "the response is the empty list")
      r <- chk w1
      r `shouldSatisfy` isRight
    it "the empty-list law fails on a NON-empty list, quoting the body it \
       \actually found" $ do
      let o = "[{\"id\":\"c1\"}]"
          fake _ = pure (Right (o, fromJust (A.decodeStrict o)))
          w = mkWorld "http://x" fake ""
      Just get <- pure (firstMatch When "I GET /api/changes?from=-1407&to=-1405")
      Right w1 <- get w
      Just chk <- pure (firstMatch Then "the response is the empty list")
      r <- chk w1
      case r of
        Left e -> do
          e `shouldSatisfy` T.isInfixOf "is not []"
          e `shouldSatisfy` T.isInfixOf "c1"
        Right _ -> expectationFailure "expected a non-empty list to fail"
    it "the empty-list law fails on a body that is not a list at all -- \
       \whole-body equality with [], not a poke at emptiness" $ do
      -- An "is it empty?" check would happily accept {} (an empty
      -- object) or "" ; whole-body equality with [] accepts exactly one
      -- body and says so.
      let o = "{}"
          fake _ = pure (Right (o, fromJust (A.decodeStrict o)))
          w = mkWorld "http://x" fake ""
      Just get <- pure (firstMatch When "I GET /api/changes?from=-1&to=-1")
      Right w1 <- get w
      Just chk <- pure (firstMatch Then "the response is the empty list")
      r <- chk w1
      case r of
        Left e  -> e `shouldSatisfy` T.isInfixOf "is not []"
        Right _ -> expectationFailure "expected an empty OBJECT to fail an empty-LIST law"

    it "fetching a scene's first resource twice: identical bytes pass" $ do
      let sceneVal = A.object ["resources" A..= ([A.object ["id" A..= ("r1" :: T.Text)]] :: [A.Value])]
          rawGet _ = pure (Right "same-bytes")
          w = (mkWorld "http://x" (\_ -> pure (Left "no")) "")
                { bound = Map.fromList [("scene", (BS.empty, sceneVal))], transportRaw = rawGet }
          run = fromJust $ firstMatch Then "fetching scene's first resource twice yields identical bytes"
      r <- run w
      r `shouldSatisfy` isRight
    it "fetching a scene's first resource twice: two different payloads for the \
       \same id fails, naming the id" $ do
      let sceneVal = A.object ["resources" A..= ([A.object ["id" A..= ("r1" :: T.Text)]] :: [A.Value])]
      counter <- newIORef (0 :: Int)
      let rawGet _ = do
            n <- readIORef counter
            modifyIORef counter (+ 1)
            pure (Right (if n == (0 :: Int) then "bytes-A" else "bytes-B"))
          w = (mkWorld "http://x" (\_ -> pure (Left "no")) "")
                { bound = Map.fromList [("scene", (BS.empty, sceneVal))], transportRaw = rawGet }
          run = fromJust $ firstMatch Then "fetching scene's first resource twice yields identical bytes"
      r <- run w
      case r of
        Left e  -> do
          e `shouldSatisfy` T.isInfixOf "r1"
          e `shouldSatisfy` T.isInfixOf "two different payloads"
        Right _ -> expectationFailure "expected two different payloads for the same id to fail"
    it "fetching a scene's first resource twice: an unbound scene fails, naming it" $ do
      let w = mkWorld "http://x" (\_ -> pure (Left "no")) ""
          run = fromJust $ firstMatch Then "fetching ghost's first resource twice yields identical bytes"
      r <- run w
      case r of
        Left e  -> e `shouldSatisfy` T.isInfixOf "ghost"
        Right _ -> expectationFailure "expected an unbound scene name to fail"

    it "a batch fetch equal to its concatenated singles passes" $ do
      let sceneVal = A.object ["resources" A..=
            ([A.object ["id" A..= ("r1" :: T.Text)], A.object ["id" A..= ("r2" :: T.Text)]] :: [A.Value])]
          rawGet url
            | "ids=r1,r2" `T.isInfixOf` url = pure (Right "AB")
            | "id=r1" `T.isInfixOf` url     = pure (Right "A")
            | "id=r2" `T.isInfixOf` url     = pure (Right "B")
            | otherwise                     = pure (Left ("unexpected url: " <> url))
          w = (mkWorld "http://x" (\_ -> pure (Left "no")) "")
                { bound = Map.fromList [("scene", (BS.empty, sceneVal))], transportRaw = rawGet }
          run = fromJust $ firstMatch Then
            "fetching scene's first two resources as a batch equals fetching them singly"
      r <- run w
      r `shouldSatisfy` isRight
    it "a batch fetch that genuinely differs from its concatenated singles fails, \
       \naming the mismatch" $ do
      let sceneVal = A.object ["resources" A..=
            ([A.object ["id" A..= ("r1" :: T.Text)], A.object ["id" A..= ("r2" :: T.Text)]] :: [A.Value])]
          rawGet url
            | "ids=r1,r2" `T.isInfixOf` url = pure (Right "WRONG")
            | "id=r1" `T.isInfixOf` url     = pure (Right "A")
            | "id=r2" `T.isInfixOf` url     = pure (Right "B")
            | otherwise                     = pure (Left ("unexpected url: " <> url))
          w = (mkWorld "http://x" (\_ -> pure (Left "no")) "")
                { bound = Map.fromList [("scene", (BS.empty, sceneVal))], transportRaw = rawGet }
          run = fromJust $ firstMatch Then
            "fetching scene's first two resources as a batch equals fetching them singly"
      r <- run w
      case r of
        Left e  -> e `shouldSatisfy` T.isInfixOf "batch bytes differ"
        Right _ -> expectationFailure "expected a genuine batch/singles mismatch to fail"

  describe "consumed projection (Task 12): a declared field-tree over what we consume" $ do
    it "projects a two-field-plus-extras object through the eras projection, \
       \keeping exactly what parse_eras reads and dropping the rest" $ do
      let raw = A.toJSON ([ A.object
              [ "id" A..= ("e1" :: T.Text), "name" A..= ("Alpha" :: T.Text)
              , "from_year" A..= (1 :: Int), "to_year" A..= (9 :: Int)
              , "notes" A..= ("drop me" :: T.Text) ] ] :: [A.Value])
          expected = A.toJSON ([ A.object
              [ "id" A..= ("e1" :: T.Text), "name" A..= ("Alpha" :: T.Text)
              , "from_year" A..= (1 :: Int), "to_year" A..= (9 :: Int) ] ] :: [A.Value])
      case Map.lookup "eras" projections of
        Just p  -> project p raw `shouldBe` Right expected
        Nothing -> expectationFailure "no 'eras' projection registered"
    it "a CHANGED consumed value changes the projection -- it is not blind \
       \to the fields it keeps" $ do
      let mk fy = A.toJSON ([ A.object
              [ "id" A..= ("e1" :: T.Text), "name" A..= ("Alpha" :: T.Text)
              , "from_year" A..= (fy :: Int), "to_year" A..= (9 :: Int) ] ] :: [A.Value])
      case Map.lookup "eras" projections of
        Just p  -> project p (mk (1 :: Int)) `shouldNotBe` project p (mk 2)
        Nothing -> expectationFailure "no 'eras' projection registered"
    it "an ADDED unconsumed field still passes -- the projection drops what \
       \it doesn't read, not merely what happens to be absent" $ do
      let row extra = A.object $
            [ "id" A..= ("e1" :: T.Text), "name" A..= ("Alpha" :: T.Text)
            , "from_year" A..= (1 :: Int), "to_year" A..= (9 :: Int) ] ++ extra
      case Map.lookup "eras" projections of
        Just p  -> project p (A.toJSON [row []])
                     `shouldBe` project p (A.toJSON [row ["brand-new-field" A..= True]])
        Nothing -> expectationFailure "no 'eras' projection registered"
    -- Corrects the plan's own sketch (R9): vendor.rs derives EventRow's
    -- `label` from `title`, falling back to `label` -- it is not two
    -- separate raw fields both kept as-is -- and `verses` is not a
    -- top-level field at all; it is gathered by flattening
    -- witnesses[].verse_groups[].verses. Verified directly against
    -- crates/map-compile/src/vendor.rs's parse_event (lines 163-198).
    it "the event projection derives 'label' from 'title' and flattens \
       \verses out of witnesses[].verse_groups[].verses" $ do
      let raw = A.object
            [ "id" A..= ("ab_haran" :: T.Text)
            , "title" A..= ("Sojourn in Haran; the call of Abram" :: T.Text)
            , "kind" A..= ("event" :: T.Text)
            , "when" A..= A.object [ "from_year" A..= ((-2092) :: Int)
                                    , "to_year" A..= ((-2091) :: Int) ]
            , "places" A..= [ A.object [ "id" A..= ("haran" :: T.Text)
                                        , "name" A..= ("Haran" :: T.Text) ] ]
            , "witnesses" A..=
                [ A.object [ "book" A..= ("GEN" :: T.Text), "verse_groups" A..=
                    [ A.object [ "chapter" A..= (11 :: Int)
                               , "verses" A..= (["GEN.11.31"] :: [T.Text]) ]
                    , A.object [ "chapter" A..= (12 :: Int)
                               , "verses" A..= (["GEN.12.1", "GEN.12.4"] :: [T.Text]) ]
                    ] ] ]
            ]
          expected = A.object
            [ "id" A..= ("ab_haran" :: T.Text)
            , "label" A..= ("Sojourn in Haran; the call of Abram" :: T.Text)
            , "when" A..= A.object [ "from_year" A..= ((-2092) :: Int)
                                    , "to_year" A..= ((-2091) :: Int) ]
            , "places" A..= [A.object ["id" A..= ("haran" :: T.Text)]]
            , "verses" A..= (["GEN.11.31", "GEN.12.1", "GEN.12.4"] :: [T.Text])
            ]
      case Map.lookup "event" projections of
        Just p  -> project p raw `shouldBe` Right expected
        Nothing -> expectationFailure "no 'event' projection registered"
    it "the event projection falls back to 'label' when 'title' is absent, \
       \and an event with no witnesses projects empty verses (not an \
       \absent key)" $ do
      let raw = A.object
            [ "id" A..= ("e2" :: T.Text)
            , "label" A..= ("Fallback Label" :: T.Text)
            , "places" A..= ([] :: [A.Value])
            ]
          expected = A.object
            [ "id" A..= ("e2" :: T.Text)
            , "label" A..= ("Fallback Label" :: T.Text)
            , "places" A..= ([] :: [A.Value])
            , "verses" A..= ([] :: [T.Text])
            ]
      case Map.lookup "event" projections of
        Just p  -> project p raw `shouldBe` Right expected
        Nothing -> expectationFailure "no 'event' projection registered"
    it "a projection name outside the registry fails to parse, naming the \
       \real endpoints" $
      case parseCap @ProjName "erass" of
        Left e  -> do
          e `shouldSatisfy` T.isInfixOf "not a projection"
          e `shouldSatisfy` T.isInfixOf "eras"
        Right _ -> expectationFailure "accepted a non-projection name"
    -- Phase S review, cheap fix 3: a shape mismatch used to fall through
    -- to the value UNCHANGED, so it surfaced only as an opaque "differs
    -- from fixture" once compared -- a worse message than naming the
    -- actual problem, on exactly the provider-drift case this suite
    -- exists to catch.
    it "project reports a named shape mismatch (object tree, array \
       \payload) instead of silently passing the array through" $
      case Map.lookup "eras" projections of
        Just p  -> project p (A.object ["oops" A..= True]) `shouldBe`
          Left "expected an array, got an object"
        Nothing -> expectationFailure "no 'eras' projection registered"
    it "project reports a named shape mismatch (array tree, object \
       \payload) instead of silently passing the object through" $
      case Map.lookup "land-mask" projections of
        Just p  -> project p (A.toJSON ([1 :: Int] :: [Int])) `shouldBe`
          Left "expected an object, got an array"
        Nothing -> expectationFailure "no 'land-mask' projection registered"
    it "the consumed-projection step names the shape mismatch, prefixed \
       \with the projection, when the provider's top-level shape drifts" $ do
      let o = "{\"id\":\"e9\"}"  -- an object where 'eras' expects a bare array
          fake _ = pure (Right (o, fromJust (A.decodeStrict o)))
          w = mkWorld "http://x" fake "test/fixtures"
      Just get <- pure (firstMatch When "I GET /api/eras")
      Right w1 <- get w
      Just chk <- pure (firstMatch Then
        "the consumed projection eras equals fixture \"eras-single-test-consumed\"")
      r <- chk w1
      case r of
        Left e  -> do
          e `shouldSatisfy` T.isInfixOf "eras"
          e `shouldSatisfy` T.isInfixOf "expected an array, got an object"
        Right _ -> expectationFailure "expected the top-level shape drift to fail, named"

    it "the consumed-projection step passes when the response's consumed \
       \fields equal the fixture, with every unconsumed extra filtered" $ do
      let o = "[{\"id\":\"e9\",\"name\":\"Beta\",\"from_year\":10,\"to_year\":90,\"extra\":true}]"
          fake _ = pure (Right (o, fromJust (A.decodeStrict o)))
          w = mkWorld "http://x" fake "test/fixtures"
      Just get <- pure (firstMatch When "I GET /api/eras")
      Right w1 <- get w
      Just chk <- pure (firstMatch Then
        "the consumed projection eras equals fixture \"eras-single-test-consumed\"")
      r <- chk w1
      r `shouldSatisfy` isRight
    it "the consumed-projection step fails, naming the projection and \
       \fixture, when a CONSUMED value genuinely differs" $ do
      let o = "[{\"id\":\"e9\",\"name\":\"Beta\",\"from_year\":11,\"to_year\":90}]"
          fake _ = pure (Right (o, fromJust (A.decodeStrict o)))
          w = mkWorld "http://x" fake "test/fixtures"
      Just get <- pure (firstMatch When "I GET /api/eras")
      Right w1 <- get w
      Just chk <- pure (firstMatch Then
        "the consumed projection eras equals fixture \"eras-single-test-consumed\"")
      r <- chk w1
      case r of
        Left e  -> do
          e `shouldSatisfy` T.isInfixOf "eras"
          e `shouldSatisfy` T.isInfixOf "eras-single-test-consumed"
        Right _ -> expectationFailure "expected a genuinely changed consumed value to fail"
    it "bless mode writes the PROJECTED value, not the raw response -- the \
       \unconsumed extra field must not survive into the fixture" $ do
      tmpBase <- getTemporaryDirectory
      (uniqueFile, uh) <- openTempFile tmpBase "contract-runner-bless-projection-test"
      hClose uh
      removeFile uniqueFile
      let dir = uniqueFile <> "-dir"
          o = "[{\"id\":\"e9\",\"name\":\"Beta\",\"from_year\":10,\"to_year\":90,\"extra\":true}]"
          fake _ = pure (Right (o, fromJust (A.decodeStrict o)))
          w = (mkWorld "http://x" fake dir) { blessMode = True }
      createDirectoryIfMissing True dir
      (`finally` removeDirectoryRecursive dir) $ do
        Just get <- pure (firstMatch When "I GET /api/eras")
        Right w1 <- get w
        Just chk <- pure (firstMatch Then
          "the consumed projection eras equals fixture \"eras-consumed-bless-test\"")
        r <- chk w1
        r `shouldSatisfy` isRight
        written <- BS.readFile (dir </> "eras-consumed-bless-test.json")
        (A.decodeStrict written :: Maybe A.Value) `shouldBe`
          A.decodeStrict "[{\"id\":\"e9\",\"name\":\"Beta\",\"from_year\":10,\"to_year\":90}]"

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
          w = mkWorld "http://x" fake "test/fixtures"
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
      let w = mkWorld "http://x" (\_ -> pure (Left "no")) ""
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
          w = mkWorld "http://x" fake "test/fixtures"
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
          -- (the sweep: the table now carries a skip-count column too --
          -- every one of these laws ran in full, so each shows 0)
          let table = reportTable results
          table `shouldSatisfy` T.isInfixOf "| hard red | \10060 RED"
          table `shouldSatisfy`
            T.isInfixOf "| expected red | \128308 red (expected \8212 @target) | 0 |"
          table `shouldSatisfy`
            T.isInfixOf "| target already met | \128994 green (target already met!) | 0 |"
          table `shouldSatisfy` T.isInfixOf "| PARSE | \10060 RED"
        rs -> expectationFailure ("expected exactly 4 results, got " <> show (length rs))
    -- Review finding (Task 6 round 1), fix 3: the exception branch of
    -- runScenario dropped the step's keyword and body that the logical-
    -- failure branch includes, even though this stage's whole deliverable
    -- is a legible diagnosis of which step failed.
    it "an exception thrown while running a step is reported with the step's keyword and body" $ do
      let w = mkWorld "http://x" (\_ -> error "boom") ""
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
          w = mkWorld "http://x" (\_ -> pure (Left "no")) ""
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

  describe "Fix 4: one shared UTF-8 reader across every .feature read path" $ do
    -- Verified empirically (this toolchain's locale encoding is CP437, not
    -- UTF-8, via World.readFeatureFile's own comment): a plain
    -- `TIO.readFile` silently mangles a 3-byte em dash instead of raising
    -- an error. `Run.runFeatureFiles`, `Check.checkDir`, and
    -- `Prop.runWithProperties` all used to read a feature file that way;
    -- only `Vocab.vocabDir` decoded explicitly. Each of the three is
    -- exercised here against a REAL file on disk containing a real em
    -- dash, so this pins the actual IO read path (`World.readFeatureFile`),
    -- not just `parseFeature`'s in-memory behavior on text that was
    -- already correctly decoded some other way.
    it "Run.runFeatureFiles preserves an em dash in a feature title read \
       \from disk" $ do
      tmpBase <- getTemporaryDirectory
      (uniqueFile, uh) <- openTempFile tmpBase "contract-runner-fix4-run-test"
      hClose uh
      removeFile uniqueFile
      let dir = uniqueFile <> "-dir"
          path = dir </> "emdash.feature"
          feat = T.unlines
            [ "Feature: the scene \8212 a picture"
            , "  Scenario: s"
            , "    When I GET /api/whatever" ]
      createDirectoryIfMissing True dir
      BS.writeFile path (TE.encodeUtf8 feat)
      (`finally` removeDirectoryRecursive dir) $ do
        let w = mkWorld "http://x" (\_ -> pure (Right ("{}", A.object []))) dir
        [r] <- runFeatureFiles allSteps w [path]
        srFeature r `shouldBe` "the scene \8212 a picture"
    it "Prop.runWithProperties preserves an em dash in a feature title \
       \read from disk" $ do
      tmpBase <- getTemporaryDirectory
      (uniqueFile, uh) <- openTempFile tmpBase "contract-runner-fix4-prop-test"
      hClose uh
      removeFile uniqueFile
      let dir = uniqueFile <> "-dir"
          path = dir </> "emdash.feature"
          feat = T.unlines
            [ "Feature: the scene \8212 a picture"
            , "  Scenario: s"
            , "    When I GET /api/whatever" ]
      createDirectoryIfMissing True dir
      BS.writeFile path (TE.encodeUtf8 feat)
      (`finally` removeDirectoryRecursive dir) $ do
        let w = mkWorld "http://x" (\_ -> pure (Right ("{}", A.object []))) dir
        [r] <- Prop.runWithProperties allSteps w 5 [path]
        srFeature r `shouldBe` "the scene \8212 a picture"
    it "Check.checkDir preserves an em dash in an orphan step's reported \
       \body, not just in the file's title" $ do
      tmpBase <- getTemporaryDirectory
      (uniqueFile, uh) <- openTempFile tmpBase "contract-runner-fix4-check-test"
      hClose uh
      removeFile uniqueFile
      let dir = uniqueFile <> "-dir"
          path = dir </> "emdash.feature"
          feat = T.unlines
            [ "Feature: t"
            , "  Scenario: s"
            , "    When nobody wrote this step \8212 a fixture-worthy orphan" ]
      createDirectoryIfMissing True dir
      BS.writeFile path (TE.encodeUtf8 feat)
      (out, result) <- (`finally` removeDirectoryRecursive dir)
        (captureStdout (Check.checkDir allSteps dir))
      result `shouldSatisfy` isLeft
      out `shouldSatisfy` T.isInfixOf "nobody wrote this step \8212 a fixture-worthy orphan"

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
          w0 = mkWorld "http://x" fake "test/fixtures"
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
          w0 = mkWorld "http://x" fake "test/fixtures"
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
              v <- runScenario allSteps (mkWorld "http://x" (\_ -> pure (Left "no")) "") sc
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
            , (When, "I GET /foo as bar")
            , (Then, "the response equals fixture \"foo\"")
            , (Then, "the response equals fixture \"foo\" masking bar as sixteen hex characters")
            , (Then, "the response field scene equals http://x/foo")
            , (When, "I render pieces fills at year -1405 in style canaan as sceneA")
            , (When, "I render pieces fills at year -1405 in style canaan")
            , (Then, "sceneA equals sceneB")
            , (Then, "sceneA's resources are a subset of sceneB's resources")
            , (Then, "sceneA's labels are empty")
            , (Then, "every feature entry carries a piece field")
            , (Then, "combining sceneA and sceneB equals rendering fills plus ground")
              -- the sweep's three new definitions
            , (Then, "combining sceneA and sceneB equals sceneC")
            , (Then, "rendering each piece of fills, ground alone and combining them equals sceneA")
            , (Then, "the response is the empty list")
            , (Then, "sceneA and sceneB differ only in dress, never in geometry")
            , (Then, "fetching sceneA's first resource twice yields identical bytes")
            , (Then, "fetching sceneA's first two resources as a batch equals fetching them singly")
            , (Then, "the consumed projection eras equals fixture \"foo\"")
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
    -- {name}" binding step existed yet. Cause: the plain GET step captured
    -- its URL with FixtureRefFreeText, whose parseCap is
    -- `Right . FixtureRefFreeText . T.strip` -- it can NEVER fail, so it
    -- silently swallowed " as first" as part of the URL and registered a
    -- full Matched. A capture that cannot fail has no discriminating
    -- power, so the totality law had nothing to catch -- a check
    -- satisfiable by the failure mode, which this project forbids (see
    -- MEMORY: verify-distinct-not-nonnull). UrlPath fixes this BY TYPE: a
    -- URL path cannot contain a raw space, a genuine property of the
    -- type, not a special case for " as ". Before Task 10 added the real
    -- "I GET {url} as {name}" step, this line was a value error (nothing
    -- fully matched); Task 10 now supplies that step, so it is time for
    -- this test to prove the OTHER half of fix 7's own prediction: the
    -- new step becomes the line's unique, clean match (not a fresh
    -- ambiguity with the plain GET step, whose UrlPath capture still
    -- rejects the embedded space and only ever ClaimErrors here) --
    -- driven through runScenario, not just Check, to prove the binding
    -- actually happens.
    it "a GET line with \" as name\" is the GET-as step's unique, clean \
       \match (Task 10 closes the hole fix 7 predicted), and the binding \
       \genuinely happens" $ do
      let body = "I GET /api/subjects?year=-1405 as first"
          fake _ = pure (Right ("[]", fromJust (A.decodeStrict "[]")))
          w = mkWorld "http://x" fake ""
      case parseFeature "getas.feature" $ T.unlines
             [ "Feature: f", "  Scenario: s", "    When " <> body ] of
        Left e -> expectationFailure (T.unpack e)
        Right f -> do
          Check.orphans allSteps f `shouldBe` []
          Check.ambiguous allSteps f `shouldBe` []
          Check.valueErrors allSteps f `shouldBe` []
          case ftScenarios f of
            [sc] -> do
              v <- runScenario allSteps w sc
              v `shouldBe` Passed
            scs -> expectationFailure ("expected one scenario, got " <> show (length scs))

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
            `shouldBe` Just "whole number from -4004 to 100 (negative means BC; -1405 is 1405 BC; year 0 does not exist)"
    -- The sweep: a feature whose EVERY scenario is quantified still has a
    -- vocabulary. Before this, `expectedVocab` matched the raw body, so
    -- "<somePieces>" matched no definition, the derived table came out
    -- EMPTY, and `vocab --write` deleted the block outright from
    -- scene/resources.feature -- generalizing a law over all pieces
    -- silently removed the sentence explaining what a piece is. Same
    -- substitution and same @property gate as Check.classify's.
    it "derives the table from a scenario whose captures are all HOLES, \
       \under the same @property gate the totality law uses" $
      case parseFeature "t.feature" $ T.unlines
             [ "Feature: t"
             , "  @property"
             , "  Scenario: s"
             , "    When I render pieces <somePieces> at year <someYear> in style <someStyle> as scene"
             , "    Then fetching scene's first resource twice yields identical bytes" ] of
        Left e -> expectationFailure (T.unpack e)
        Right f -> map fst (Vocab.expectedVocab allSteps f)
                     `shouldBe` ["pieces", "year", "style"]
    -- The other direction of the same gate: an UNTAGGED scenario is never
    -- substituted at run time, so its literal "<somePieces>" text really
    -- does match nothing, and the table must not claim otherwise.
    it "does NOT substitute holes for an untagged scenario -- the table \
       \must describe what will actually run" $
      case parseFeature "t.feature" $ T.unlines
             [ "Feature: t"
             , "  Scenario: s"
             , "    When I render pieces <somePieces> at year <someYear> in style <someStyle>" ] of
        Left e -> expectationFailure (T.unpack e)
        Right f -> Vocab.expectedVocab allSteps f `shouldBe` []
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
          w = mkWorld "http://x" fake ""
      case parseFeature "t.feature" feat of
        Left e -> expectationFailure (T.unpack e)
        Right f -> case ftScenarios f of
          (sc : _) -> do
            v <- Prop.runScenarioProperty allSteps w 25 sc
            -- (the sweep: a law's run is now its verdict AND how many
            -- iterations never ran -- this one has no precondition, so
            -- all 25 genuinely ran)
            v `shouldBe` LawRun Passed 0
          [] -> expectationFailure "expected at least one scenario"
    it "reports the failing binding when the law breaks" $ do
      let feat = T.unlines
            [ "Feature: t"
            , "  @property"
            , "  Scenario: falsifiable"
            , "    When I GET /api/echo?y=<someYear>"
            , "    Then the response field neverThere equals nope" ]
          fake _ = pure (Right ("{}", A.object []))
          w = mkWorld "http://x" fake ""
      case parseFeature "t.feature" feat of
        Left e -> expectationFailure (T.unpack e)
        Right f -> case ftScenarios f of
          (sc : _) -> do
            v <- Prop.runScenarioProperty allSteps w 25 sc
            case lawVerdict v of
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
          w = mkWorld "http://x" (\_ -> pure (Left "no")) ""
      case parseFeature "t.feature" feat of
        Left e -> expectationFailure (T.unpack e)
        Right f -> case ftScenarios f of
          (sc : _) -> do
            v <- Prop.runScenarioProperty allSteps w 5 sc
            case lawVerdict v of
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
          w = mkWorld "http://x" fake ""
      case parseFeature "t.feature" feat of
        Left e -> expectationFailure (T.unpack e)
        Right f -> case ftScenarios f of
          (sc : _) -> do
            v1 <- Prop.runScenarioProperty allSteps w 25 sc
            v2 <- Prop.runScenarioProperty allSteps w 25 sc
            v1 `shouldBe` v2
          [] -> expectationFailure "expected at least one scenario"
    -- Final-review Fix 1 (Critical): scene.feature's composition property
    -- registers `someA` and `someB` against the SAME generator
    -- (`genPieces`). Before this fix, `renderHole`'s seed depended ONLY
    -- on the iteration index, so `unGen` (pure) returned the identical
    -- PieceSet for someA and someB on every single iteration -- the
    -- composition scenario's "combine the parts, get the whole" assertion
    -- degenerated to `x == x \`union\` x`, true for ANY server behaviour,
    -- and its "target already met" green was an artifact of the
    -- generator wiring, not a fact about the server. This is a
    -- DISTINCTNESS assertion, not a non-nullity one (MEMORY:
    -- verify-distinct-not-nonnull): it directly demonstrates that two
    -- holes sharing a generator draw independently across a run, by
    -- finding at least one iteration where they genuinely differ -- the
    -- exact property whose absence made the composition law
    -- unfalsifiable.
    -- (The sweep: restated against the real registry rather than against
    -- a hand-built SomeHole, now that `Prop.bindingsFor` is the one
    -- function the runner uses to bind a scenario's holes -- so this
    -- pins the actual wiring someA/someB have, not a reconstruction of
    -- it that could drift from the registry.)
    it "someA and someB draw independently: two holes registered against \
       \the SAME generator, in SEPARATE groups, genuinely differ at least \
       \once across a run of iterations" $ do
      let draws = [ (Map.lookup "someA" b, Map.lookup "someB" b)
                  | i <- [0 .. 49], let b = Prop.bindingsFor ["someA", "someB"] i ]
      draws `shouldSatisfy` any (uncurry (/=))
      -- and neither is silently missing, which `any (/=)` alone could
      -- not tell apart from one of them being absent
      draws `shouldSatisfy` all (\(a, b) -> a /= Nothing && b /= Nothing)
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
            , "    When I render pieces <somePieces> at year 1 in style canaan as a" ]
          fake url = pure (Right (bs, fromJust (A.decodeStrict bs)))
            where bs = TE.encodeUtf8 ("{\"echo\":\"" <> url <> "\"}")
          w = mkWorld "http://x" fake ""
      case parseFeature "t.feature" feat of
        Left e -> expectationFailure (T.unpack e)
        Right f -> case ftScenarios f of
          (sc : _) -> do
            let sc' = Prop.substitute (Map.fromList [("somePieces", "none")]) sc
            case scSteps sc' of
              (Step _ b _ : _) ->
                b `shouldBe` "I render pieces none at year 1 in style canaan as a"
              [] -> expectationFailure "expected a step"
            v <- runScenario allSteps w sc'
            v `shouldBe` Passed
          [] -> expectationFailure "expected at least one scenario"

  -- ---------- the sweep: holes drawn in correlated groups ----------
  -- This project's worst Stage 0 defect was a law that COULD NOT FAIL:
  -- someA and someB drew the identical value, so the composition law
  -- degenerated to `x == x ∪ x`. The sweep now introduces correlation ON
  -- PURPOSE, which is the same trap approached from the other side --
  -- so every relationship a group establishes is pinned here by a test
  -- that goes red if the relationship breaks, and every DISTINCTNESS
  -- likewise. A correlation nobody checks is indistinguishable from the
  -- accident that produced 7cd31bd.
  describe "the sweep: correlated hole groups" $ do
    let styleHoles  = ["someStyle", "someOtherStyle"]
        nestedHoles = ["someSubset", "someSuperset"]
        -- The two drawn sets of one iteration, as raw Sets.
        nestedAt i = (,) <$> (unwrap <$> drawnAs nestedHoles "someSubset" i)
                         <*> (unwrap <$> drawnAs nestedHoles "someSuperset" i)
          where unwrap (PieceSet s) = s
        stylesAt i = (,) <$> drawnAs @StyleName styleHoles "someStyle" i
                         <*> drawnAs @StyleName styleHoles "someOtherStyle" i
        iterations = [0 .. 199]
    it "the registry is a PARTITION of the hole names: every hole belongs \
       \to exactly one group, and that group is the one the registry \
       \hands back for it" $ do
      let members = concatMap Prop.groupMembers Prop.holeGroups
      -- no hole is claimed by two groups (which would make its value
      -- depend on which other holes a scenario happened to mention)
      length members `shouldBe` Set.size (Set.fromList members)
      Set.fromList members `shouldBe` Set.fromList (Map.keys Prop.holeRegistry)
      sequence_
        [ fmap Prop.groupName (Map.lookup h Prop.holeRegistry)
            `shouldBe` Just (Prop.groupName g)
        | g <- Prop.holeGroups, h <- Prop.groupMembers g ]
    -- Review finding (round 1, Important): `groupName` uniqueness was
    -- declared in prose and pinned by nothing. Two groups sharing a name
    -- share ONE seed stream and one entry in `bindingsFor`'s
    -- name-keyed map, so the loser's holes silently get no binding at
    -- all -- literally the seed-collision family that produced the
    -- composition bug (7cd31bd), just at the group level instead of the
    -- hole level.
    it "every group's NAME is unique: two groups sharing a name would \
       \share one seed stream, and the loser's holes would silently go \
       \unbound" $ do
      let names = map Prop.groupName Prop.holeGroups
      length names `shouldBe` Set.size (Set.fromList names)
    -- Review finding (round 1, Important): nothing forced a group to
    -- actually DRAW what it declares. A group whose generator omits a
    -- declared member produces no binding for it, the hole survives
    -- substitution as literal "<someHole>" text, and the failure
    -- surfaces downstream as an obscure capture parse error that names
    -- the capture rather than the real cause.
    it "every group DRAWS exactly the members it DECLARES, at every \
       \iteration -- a declared-but-undrawn member would leave its hole \
       \as literal text and fail as an unrelated capture error" $
      sequence_
        [ Map.keys (Prop.drawGroup g i) `shouldBe` sort (Prop.groupMembers g)
        | g <- Prop.holeGroups, i <- [0 .. 24] ]
    it "<someSubset> is nested inside <someSuperset> at EVERY iteration -- \
       \the correlation the subtractive law needs, asserted over the whole \
       \run rather than sampled" $
      -- Whole-body: the full list of 200 answers, not "any" or "most".
      -- Independent draws would fail this within a handful of
      -- iterations, which is exactly the point of asserting it.
      traverse (fmap (uncurry Set.isSubsetOf) . nestedAt) iterations
        `shouldBe` Right (replicate (length iterations) True)
    it "the nested pair is a PROPER subset in at least a quarter of \
       \iterations -- a DISTRIBUTION FLOOR, not a tuned constant" $ do
      -- The floor exists because a law tested only on EQUAL sets tests
      -- nothing about subtraction: `fewer ⊆ more` is trivially true when
      -- they are the same set, so a generator that drifted toward
      -- equality would quietly hollow the law out while staying green.
      -- Equality is still a legal draw (⊆ is reflexive and the law
      -- claims the reflexive case too), so this is a floor on the
      -- distribution, not a ban on a value.
      --
      -- A quarter is deliberately far below what the construction
      -- delivers (`sublistOf` over a mean-size-5 superset coincides with
      -- it about 5.6% of the time, so ~94% of draws are proper) -- it is
      -- a floor chosen to be unmistakably clear of zero and unmistakably
      -- clear of today's number, not a threshold fitted to the numbers
      -- this generator happens to produce.
      let proper = traverse (fmap (uncurry (/=)) . nestedAt) iterations
      fmap (length . filter id) proper
        `shouldSatisfy` either (const False) (>= length iterations `div` 4)
    it "<someStyle> and <someOtherStyle> are DISTINCT at every iteration -- \
       \a restyle to the same style would make dress-locality vacuous" $
      traverse (fmap (uncurry (/=)) . stylesAt) iterations
        `shouldBe` Right (replicate (length iterations) True)
    it "both style slots reach EVERY style across a run -- distinctness \
       \must not be bought by pinning one slot to a constant" $ do
      -- The cheap way to pass the distinctness test above is to always
      -- draw ("canaan", "slate"); that would leave the dress-locality
      -- law quantified over one pair of dresses while claiming to range
      -- over all of them. Coverage on BOTH slots is what rules it out.
      let drawsE = traverse stylesAt iterations
      fmap (Set.fromList . map (renderCap . fst)) drawsE
        `shouldBe` Right (Set.fromList styleNames)
      fmap (Set.fromList . map (renderCap . snd)) drawsE
        `shouldBe` Right (Set.fromList styleNames)
    it "asking for a DIFFERENT style is only meaningful because more than \
       \one dress exists -- genStylePair's non-zero rotation depends on it" $
      length styleNames `shouldSatisfy` (>= 2)
    it "a group member drawn ALONE gets the same value it gets alongside \
       \its partner: a group's draw depends on the GROUP, not on how many \
       \of its members a scenario happens to mention" $
      [ Map.lookup "someStyle" (Prop.bindingsFor ["someStyle"] i) | i <- [0 .. 49] ]
        `shouldBe`
      [ Map.lookup "someStyle" (Prop.bindingsFor ["someStyle", "someOtherStyle", "someYear"] i)
      | i <- [0 .. 49] ]
    it "bindings are restricted to the holes the scenario actually \
       \mentions -- a counterexample must not report a partner hole the \
       \scenario never used" $
      Map.keys (Prop.bindingsFor ["someStyle"] 3) `shouldBe` ["someStyle"]
    it "the subtractive law runs GREEN over 100 iterations against a \
       \server that honours it -- end-to-end evidence that the drawn \
       \pairs really are nested (independent draws redden this)" $ do
      -- The fake answers with exactly the pieces the URL turned ON, so
      -- its resource sets are nested exactly when the drawn piece sets
      -- are. Nothing about the step is faked: this is the corpus's own
      -- wording, run through the real property runner.
      let fake url = pure (Right (TE.encodeUtf8 url, wireResources url))
          w = mkWorld "http://x" fake ""
          feat = T.unlines
            [ "Feature: t"
            , "  @property"
            , "  Scenario: subtractive"
            , "    When I render pieces <someSubset> at year <someYear> in style <someStyle> as fewer"
            , "    And I render pieces <someSuperset> at year <someYear> in style <someStyle> as more"
            , "    Then fewer's resources are a subset of more's resources" ]
      case parseFeature "t.feature" feat of
        Left e -> expectationFailure (T.unpack e)
        Right f -> case ftScenarios f of
          (sc : _) -> do
            r <- Prop.runScenarioProperty allSteps w 100 sc
            r `shouldBe` LawRun Passed 0
          [] -> expectationFailure "expected at least one scenario"
    it "the dress-locality law runs GREEN over 100 iterations against a \
       \server that repaints without moving anything -- end-to-end \
       \evidence that the two drawn styles really differ (a collision \
       \reddens this as \"dress did not actually change\")" $ do
      let fake url = pure (Right (TE.encodeUtf8 url, A.object
            [ "resources" A..= ([A.object ["id" A..= ("r1" :: T.Text)]] :: [A.Value])
            , "dress" A..= styleInUrl url ]))
          w = mkWorld "http://x" fake ""
          feat = T.unlines
            [ "Feature: t"
            , "  @property"
            , "  Scenario: dress locality"
            , "    When I render pieces <somePieces> at year <someYear> in style <someStyle> as dressed"
            , "    And I render pieces <somePieces> at year <someYear> in style <someOtherStyle> as redressed"
            , "    Then dressed and redressed differ only in dress, never in geometry" ]
      case parseFeature "t.feature" feat of
        Left e -> expectationFailure (T.unpack e)
        Right f -> case ftScenarios f of
          (sc : _) -> do
            r <- Prop.runScenarioProperty allSteps w 100 sc
            r `shouldBe` LawRun Passed 0
          [] -> expectationFailure "expected at least one scenario"
    it "a scenario over CORRELATED holes still produces byte-identical \
       \verdicts run to run, counterexample text included" $ do
      let fake url = pure (Right (TE.encodeUtf8 url, wireResources url))
          w = mkWorld "http://x" fake ""
          feat = T.unlines
            [ "Feature: t"
            , "  @property"
            , "  Scenario: falsifiable over correlated holes"
            , "    When I render pieces <someSuperset> at year <someYear> in style <someStyle> as fewer"
            , "    And I render pieces <someSubset> at year <someYear> in style <someOtherStyle> as more"
            , "    Then fewer's resources are a subset of more's resources" ]
      case parseFeature "t.feature" feat of
        Left e -> expectationFailure (T.unpack e)
        Right f -> case ftScenarios f of
          (sc : _) -> do
            r1 <- Prop.runScenarioProperty allSteps w 100 sc
            r2 <- Prop.runScenarioProperty allSteps w 100 sc
            r1 `shouldBe` r2
            -- and it really did fail (a law swapped end for end is not
            -- true): two identical PASSES would satisfy the equality
            -- above without proving anything about counterexample text.
            lawVerdict r1 `shouldSatisfy` \v -> case v of Failed _ -> True; _ -> False
            case lawVerdict r1 of
              Failed e -> e `shouldSatisfy` T.isInfixOf "someSubset = "
              _ -> pure ()
          [] -> expectationFailure "expected at least one scenario"

  -- ---------- Stage 1 Task 4: forAllShrink's semantics ----------
  -- Spec §4 asks for `forAllShrink`; §3.2 is the bill for not having it
  -- (a composition counterexample naming fifteen pieces where the true
  -- answer was one word, narrowed by hand). Shrinking here attaches to
  -- the GROUP, not the hole: a candidate replaces every member of one
  -- group at once, because a candidate that replaced only half of a
  -- correlated pair could hand the law a binding the DRAW could never
  -- have produced -- a counterexample that is not an example of
  -- anything. So the correlation laws are pinned over shrink candidates
  -- with exactly the bar the draw laws are pinned with.
  describe "property shrinking (Stage 1 Task 4)" $ do
    let pieces  = Prop.holeRegistry Map.! "somePieces"
        piece   = Prop.holeRegistry Map.! "somePiece"
        years   = Prop.holeRegistry Map.! "someYear"
        nested  = Prop.holeRegistry Map.! "someSubset"
        styles  = Prop.holeRegistry Map.! "someStyle"
        iterations = [0 .. 49 :: Int]
        -- One solo group's candidate renderings, in the shrinker's own
        -- order -- the order the greedy loop will try them in.
        soloCands g h rendered =
          [ m Map.! h | m <- Prop.groupShrink g (Map.singleton h rendered) ]
        setOf i who = (\(PieceSet s) -> s) <$> drawnAs @PieceSet
                        ["someSubset", "someSuperset"] who i
    it "a solo group's shrink offers strictly smaller renderings, in its \
       \own order, and eventually none" $ do
      -- Whole-body: the complete candidate list, not "some candidate"
      -- (MEMORY: whole-body-assertions). Drop-one-at-a-time means every
      -- subset is reachable by repetition and every step is strictly
      -- smaller, which is what makes the greedy loop terminate.
      soloCands pieces "somePieces" "borders, ground, water"
        `shouldBe` ["borders, water", "borders, ground", "ground, water"]
      soloCands pieces "somePieces" "none" `shouldBe` []
    it "somePiece is registered as its own solo group and shrinks toward \
       \the earliest piece" $ do
      -- Registered here for Task 12's strengthened omission law; pinned
      -- now so it cannot be registered without a working shrinker.
      Prop.groupName piece `shouldBe` "somePiece"
      Prop.groupMembers piece `shouldBe` ["somePiece"]
      soloCands piece "somePiece" "fills" `shouldBe` ["ground", "water"]
      soloCands piece "somePiece" "ground" `shouldBe` []
    it "every year candidate is a legal year of THIS calendar: never 0, \
       \never outside the frame, never equal to its input, and either \
       \-1405 or strictly nearer zero" $ do
      -- Year's shrink is the one that moves toward a landmark (-1405)
      -- rather than only toward zero, so "strictly smaller" is stated as
      -- the disjunction the shrinker actually guarantees -- not a weaker
      -- "is different" that a looping shrinker would satisfy too.
      let bad = [ (y, v)
                | y <- [-4004, -1405, -703, -100, -1, 1, 54, 100 :: Int]
                , v <- map (read . T.unpack)
                         (soloCands years "someYear" (T.pack (show y))) :: [Int]
                , not (v /= 0 && v >= -4004 && v <= 100 && v /= y
                       && (v == -1405 || abs v < abs y)) ]
      bad `shouldBe` []
      -- and it is not the empty shrinker dressed up as a lawful one
      soloCands years "someYear" "54" `shouldSatisfy` (not . null)
      soloCands years "someYear" "-4004" `shouldSatisfy` (not . null)
      -- -1405 is the frame's landmark and the BOTTOM of the year order:
      -- nothing is smaller, so nothing is offered. See the cycle law
      -- below for why offering anything here is a bug, not a courtesy.
      soloCands years "someYear" "-1405" `shouldBe` []
    it "no year, anywhere in the frame, shrinks to a year of equal or \
       \greater RANK -- the well-founded order that rules out the \
       \-1405 <-> -1400 cycle, checked over the WHOLE frame rather than \
       \over a few sampled years" $ do
      -- The cycle was real, not hypothetical: the first draft of this
      -- shrinker offered -1405 from every year AND ordinary
      -- integer-shrinks from -1405, so -1405 offered -1400 and -1400
      -- offered -1405 back. Against the live server, where both years
      -- genuinely fail the identity law, the greedy loop ping-ponged
      -- until the FUEL ran out and reported wherever it stopped as
      -- "minimal". A shrinker that needs the fuel bound to terminate is
      -- the mis-written shrinker the fuel bound exists to survive.
      let ranked y = Prop.yearRank (Year y)
      [ (y, y')
        | y <- [-4004 .. 100 :: Int], y /= 0
        , y' <- map (read . T.unpack) (soloCands years "someYear" (T.pack (show y)))
        , ranked y' >= ranked y ] `shouldBe` []
    it "every shrink candidate of every group RANKS strictly below the \
       \bindings it came from -- the one law that makes the greedy loop \
       \terminate, owed by every group, present and future" $
      [ (Prop.groupName g, i, Prop.groupRank g cand, Prop.groupRank g drawn)
      | g <- Prop.holeGroups
      , i <- iterations
      , let drawn = Prop.drawGroup g i
      , cand <- Prop.groupShrink g drawn
      , Prop.groupRank g cand >= Prop.groupRank g drawn ] `shouldBe` []
    prop "the nested pair's order is well-founded over the WHOLE \
         \generator, not only over the iterations the runner happens to \
         \draw" $
      forAll Prop.genNestedPieces $ \(sub, super) ->
        let env = Map.fromList [ ("someSubset", renderCap sub)
                               , ("someSuperset", renderCap super) ]
        in [ Prop.groupRank nested c
           | c <- Prop.groupShrink nested env
           , Prop.groupRank nested c >= Prop.groupRank nested env ] == []
    it "every shrink candidate of every group still satisfies that \
       \group's own correlation law -- a candidate the DRAW could never \
       \have produced is not a counterexample, it is a bug in the \
       \shrinker" $ do
      let violations =
            [ (Prop.groupName g, i, cand)
            | g <- Prop.holeGroups
            , i <- iterations
            , cand <- Prop.groupShrink g (Prop.drawGroup g i)
            , not (Prop.groupLaw g cand) ]
      violations `shouldBe` []
    it "every DRAW satisfies its group's law too -- the same predicate, \
       \stated once on the group and owed by both the generator and the \
       \shrinker" $
      [ (Prop.groupName g, i)
      | g <- Prop.holeGroups, i <- iterations
      , not (Prop.groupLaw g (Prop.drawGroup g i)) ] `shouldBe` []
    it "a shrunk (subset, superset) pair is STILL nested, and strictly \
       \smaller, at every iteration" $ do
      let cands i = [ ( fmap (\(PieceSet s) -> s) (parseCap (m Map.! "someSubset"))
                      , fmap (\(PieceSet s) -> s) (parseCap (m Map.! "someSuperset")) )
                    | m <- Prop.groupShrink nested (Prop.drawGroup nested i) ]
          drawnSize i = (+) <$> (Set.size <$> setOf i "someSubset")
                            <*> (Set.size <$> setOf i "someSuperset")
      -- every candidate parses back through the very captures the corpus
      -- parses it with (an unparseable rendering is not a candidate)
      [ (i, pr) | i <- iterations, pr@(lft, rgt) <- cands i
                , isLeft lft || isLeft rgt ] `shouldBe` []
      [ (i, sub, sup)
        | i <- iterations, (Right sub, Right sup) <- cands i
        , not (sub `Set.isSubsetOf` sup) ] `shouldBe` []
      -- and the total size strictly decreases, which is why the greedy
      -- loop over this group terminates
      [ (i, Set.size sub + Set.size sup)
        | i <- iterations, (Right sub, Right sup) <- cands i
        , not (either (const False) (> Set.size sub + Set.size sup) (drawnSize i)) ]
        `shouldBe` []
    it "the nested pair offers candidates exactly when there is something \
       \left to remove -- so \"every candidate is nested\" above is not \
       \vacuously true of an empty list" $
      [ (i, null (Prop.groupShrink nested (Prop.drawGroup nested i)))
      | i <- iterations ]
        `shouldBe`
      [ (i, either (const True) Set.null (setOf i "someSuperset")) | i <- iterations ]
    it "the style pair does not shrink: a dress has no smaller dress, and \
       \shrinking one half of the pair would make the two equal -- which \
       \is exactly the collision dress-locality exists to rule out" $
      [ Prop.groupShrink styles (Prop.drawGroup styles i) | i <- iterations ]
        `shouldBe` map (const []) iterations
    it "a property failing on ONE piece shrinks to that one piece" $ do
      -- A fake transport that fails only when `journeys` is actually ON
      -- in the rendered URL. NOTE (and the brief's own warning): today's
      -- Steps.sceneUrl expresses journeys NEGATIVELY -- "&journeys=0"
      -- appears when the piece is ABSENT -- so the predicate is written
      -- against what the URL really says, not against the piece name.
      -- Task 11 changes this wire; re-check the predicate then.
      --
      -- The generated piece sets are large; the MINIMAL failing set is
      -- exactly {journeys}, and that is what the report must name.
      let src = T.unlines
            [ "Feature: t"
            , "  @property"
            , "  Scenario: s"
            , "    When I render pieces <somePieces> at year -1405 in style canaan as only"
            , "    Then only equals only" ]
          fake url
            | "journeys=0" `T.isInfixOf` url =
                pure (Right (okBody, fromJust (A.decodeStrict okBody)))
            | otherwise = pure (Left "boom: journeys")
          w = mkWorld "http://x" fake ""
      case parseFeature "t.feature" src of
        Left e -> expectationFailure (T.unpack e)
        Right f -> case ftScenarios f of
          (sc : _) -> do
            r <- Prop.runScenarioProperty allSteps w 50 sc
            case lawVerdict r of
              -- the whole binding line, not a prefix of it: "somePieces
              -- = journeys" is an infix of "somePieces = journeys,
              -- labels" too, so a prefix test would pass on an
              -- unshrunken report.
              Failed e -> do
                e `shouldSatisfy` T.isInfixOf "\n    with somePieces = journeys\n"
                -- shrinking SHARPENS the report, it never hides what was
                -- actually generated
                e `shouldSatisfy` T.isInfixOf "\n    (shrunk from somePieces = "
              other -> expectationFailure ("expected a failure, got " <> show other)
          [] -> expectationFailure "expected at least one scenario"
    it "shrinking does not turn a PASSING property into a failure" $ do
      let src = T.unlines
            [ "Feature: t"
            , "  @property"
            , "  Scenario: s"
            , "    When I render pieces <somePieces> at year <someYear> in style canaan as only"
            , "    Then only equals only" ]
          fake _ = pure (Right (okBody, fromJust (A.decodeStrict okBody)))
      case parseFeature "t.feature" src of
        Left e -> expectationFailure (T.unpack e)
        Right f -> case ftScenarios f of
          (sc : _) -> do
            r <- Prop.runScenarioProperty allSteps (mkWorld "http://x" fake "") 20 sc
            r `shouldBe` LawRun Passed 0
          [] -> expectationFailure "expected at least one scenario"
    it "shrinking consumes no fresh randomness: the same failing run twice \
       \yields the byte-identical minimal counterexample" $ do
      -- The diagnosis depends on reproducible runs. Candidates are
      -- derived purely from the CURRENT rendered values (parse ->
      -- shrink -> re-render), never from a new draw, so a second run of
      -- the same seeds walks the identical path to the identical
      -- minimum.
      let src = T.unlines
            [ "Feature: t"
            , "  @property"
            , "  Scenario: s"
            , "    When I render pieces <somePieces> at year <someYear> in style canaan as only"
            , "    Then only equals only" ]
          fake url
            | "journeys=0" `T.isInfixOf` url =
                pure (Right (okBody, fromJust (A.decodeStrict okBody)))
            | otherwise = pure (Left "boom: journeys")
          w = mkWorld "http://x" fake ""
      case parseFeature "t.feature" src of
        Left e -> expectationFailure (T.unpack e)
        Right f -> case ftScenarios f of
          (sc : _) -> do
            r1 <- Prop.runScenarioProperty allSteps w 30 sc
            r2 <- Prop.runScenarioProperty allSteps w 30 sc
            r1 `shouldBe` r2
            -- and it really did shrink: two identical PASSES would
            -- satisfy the equality above while proving nothing
            case lawVerdict r1 of
              Failed e -> e `shouldSatisfy` T.isInfixOf "(shrunk from "
              other -> expectationFailure ("expected a failure, got " <> show other)
          [] -> expectationFailure "expected at least one scenario"
    it "the report omits the \"(shrunk from ...)\" tail when nothing \
       \shrank -- the tail states a FACT about the run, it is not \
       \decoration" $ do
      -- Over the style pair alone there is no smaller binding, so the
      -- minimal env IS the drawn env and there is nothing to quote.
      let src = T.unlines
            [ "Feature: t"
            , "  @property"
            , "  Scenario: s"
            , "    When I render pieces none at year -1405 in style <someStyle> as a"
            , "    And I render pieces none at year -1405 in style <someOtherStyle> as b"
            , "    Then a equals b" ]
          fake url = pure (Right (bs, fromJust (A.decodeStrict bs)))
            where bs = TE.encodeUtf8 ("{\"echo\":\"" <> url <> "\"}")
          w = mkWorld "http://x" fake ""
      case parseFeature "t.feature" src of
        Left e -> expectationFailure (T.unpack e)
        Right f -> case ftScenarios f of
          (sc : _) -> do
            r <- Prop.runScenarioProperty allSteps w 10 sc
            case lawVerdict r of
              Failed e -> do
                e `shouldSatisfy` T.isInfixOf "\n    with someOtherStyle = "
                e `shouldSatisfy` (not . T.isInfixOf "(shrunk from")
              other -> expectationFailure ("expected a failure, got " <> show other)
          [] -> expectationFailure "expected at least one scenario"
    it "a counterexample never grows a hole the scenario did not mention: \
       \a group can only be shrunk when the env carries ALL of its \
       \members, because half a correlated pair cannot be re-bound \
       \lawfully" $ do
      let src = T.unlines
            [ "Feature: t"
            , "  @property"
            , "  Scenario: s"
            , "    When I render pieces <someSubset> at year -1405 in style canaan as only"
            , "    Then only equals nothing" ]
          fake url = pure (Right (bs, fromJust (A.decodeStrict bs)))
            where bs = TE.encodeUtf8 ("{\"echo\":\"" <> url <> "\"}")
          w = mkWorld "http://x" fake ""
      case parseFeature "t.feature" src of
        Left e -> expectationFailure (T.unpack e)
        Right f -> case ftScenarios f of
          (sc : _) -> do
            r <- Prop.runScenarioProperty allSteps w 5 sc
            case lawVerdict r of
              Failed e -> e `shouldSatisfy` (not . T.isInfixOf "someSuperset")
              other -> expectationFailure ("expected a failure, got " <> show other)
          [] -> expectationFailure "expected at least one scenario"

  -- ---------- the sweep: the skip discipline ----------
  describe "the sweep: preconditioned laws skip, and a law that never ran \
           \is never green" $ do
    let noResources = A.object ["resources" A..= ([] :: [A.Value])]
        oneResource = A.object
          ["resources" A..= ([A.object ["id" A..= ("r1" :: T.Text)]] :: [A.Value])]
        constBytes _ = pure (Right "same-bytes")
    it "a step whose precondition the draw cannot meet reports Skipped -- \
       \neither Passed (a vacuous green) nor Failed (a red for an \
       \unbroken law)" $ do
      let w = (mkWorld "http://x" (\_ -> pure (Right ("{}", noResources))) "")
                { transportRaw = constBytes }
      case parseFeature "t.feature" $ T.unlines
             [ "Feature: t"
             , "  Scenario: s"
             , "    When I render pieces fills at year -1405 in style canaan as scene"
             , "    Then fetching scene's first resource twice yields identical bytes" ] of
        Left e -> expectationFailure (T.unpack e)
        Right f -> case ftScenarios f of
          (sc : _) -> do
            v <- runScenario allSteps w sc
            case v of
              Skipped why -> why `shouldSatisfy` T.isInfixOf "carries 0 resource(s)"
              other -> expectationFailure
                ("expected a precondition miss to Skip, got " <> show other)
          [] -> expectationFailure "expected at least one scenario"
    it "a plain scenario whose ONLY run skipped is Failed, not green: the \
       \n = 1 instance of the same discipline the property runner applies" $
      case lawOnce (Skipped "nothing to fetch") of
        LawRun (Failed e) 1 -> do
          e `shouldSatisfy` T.isInfixOf "law never ran"
          e `shouldSatisfy` T.isInfixOf "nothing to fetch"
        other -> expectationFailure
          ("expected a single skipped run to be a named failure, got " <> show other)
    it "a property that skips SOME iterations passes, with the skip count \
       \visible in its LawRun" $ do
      -- The fake serves a scene with no resources at every EVEN year and
      -- one resource at every odd year, so the drawn <someYear> decides
      -- whether the law can run at all -- some iterations skip, the rest
      -- genuinely exercise byte-identity.
      let fake url = pure (Right (TE.encodeUtf8 url,
                                  if evenYearInUrl url then noResources else oneResource))
          w = (mkWorld "http://x" fake "") { transportRaw = constBytes }
          feat = T.unlines
            [ "Feature: t"
            , "  @property"
            , "  Scenario: byte identity"
            , "    When I render pieces fills at year <someYear> in style canaan as scene"
            , "    Then fetching scene's first resource twice yields identical bytes" ]
      case parseFeature "t.feature" feat of
        Left e -> expectationFailure (T.unpack e)
        Right f -> case ftScenarios f of
          (sc : _) -> do
            LawRun v skips <- Prop.runScenarioProperty allSteps w 25 sc
            v `shouldBe` Passed
            skips `shouldSatisfy` (> 0)
            skips `shouldSatisfy` (< 25)
          [] -> expectationFailure "expected at least one scenario"
    it "a property whose iterations ALL skip is Failed, naming the count -- \
       \a law that never ran must not report green" $ do
      let w = (mkWorld "http://x" (\_ -> pure (Right ("{}", noResources))) "")
                { transportRaw = constBytes }
          feat = T.unlines
            [ "Feature: t"
            , "  @property"
            , "  Scenario: byte identity"
            , "    When I render pieces fills at year <someYear> in style canaan as scene"
            , "    Then fetching scene's first resource twice yields identical bytes" ]
      case parseFeature "t.feature" feat of
        Left e -> expectationFailure (T.unpack e)
        Right f -> case ftScenarios f of
          (sc : _) -> do
            LawRun v skips <- Prop.runScenarioProperty allSteps w 25 sc
            skips `shouldBe` 25
            case v of
              Failed e -> do
                e `shouldSatisfy` T.isInfixOf "law never ran"
                e `shouldSatisfy` T.isInfixOf "all 25 iteration(s) skipped"
                -- and it still says WHY, with the binding that failed
                -- the precondition: a bare count is a number nobody can
                -- act on.
                e `shouldSatisfy` T.isInfixOf "someYear = "
              other -> expectationFailure
                ("an all-skipped law must be Failed, got " <> show other)
          [] -> expectationFailure "expected at least one scenario"
    -- Review finding (round 1): `--property-runs 0` is a green run that
    -- checked nothing -- `lawTally`'s `iterations > 0` guard cannot fire,
    -- so every @property law reports Passed having examined no draw at
    -- all. Refused at parse time; the law itself lives in the library so
    -- it can be pinned here rather than only in the option parser.
    it "a property-runs count below 1 is REFUSED, naming why -- zero \
       \iterations does not check a law less thoroughly, it does not \
       \check it at all" $ do
      case Prop.checkPropertyRuns 0 of
        Left e -> do
          e `shouldSatisfy` isInfixOf "at least 1"
          e `shouldSatisfy` isInfixOf "does not check it at all"
        Right n -> expectationFailure
          ("--property-runs 0 must be refused, got " <> show n)
      Prop.checkPropertyRuns (-5) `shouldSatisfy` isLeft
      Prop.checkPropertyRuns 1 `shouldBe` Right 1
      Prop.checkPropertyRuns 100 `shouldBe` Right 100
    -- And the reason it must be refused, demonstrated rather than
    -- asserted: at n = 0 the runner really does report a green verdict
    -- for a law whose every real iteration fails.
    it "the reason n = 0 is refused: the property runner would otherwise \
       \report Passed on a law that fails at every genuine iteration" $ do
      let fake _ = pure (Right ("{}", A.object []))
          w = mkWorld "http://x" fake ""
          feat = T.unlines
            [ "Feature: t"
            , "  @property"
            , "  Scenario: falsifiable"
            , "    When I GET /api/echo?y=<someYear>"
            , "    Then the response field neverThere equals nope" ]
      case parseFeature "t.feature" feat of
        Left e -> expectationFailure (T.unpack e)
        Right f -> case ftScenarios f of
          (sc : _) -> do
            atOne <- Prop.runScenarioProperty allSteps w 1 sc
            lawVerdict atOne `shouldSatisfy` \v -> case v of Failed _ -> True; _ -> False
            atZero <- Prop.runScenarioProperty allSteps w 0 sc
            atZero `shouldBe` LawRun Passed 0   -- the vacuous green, unreachable via the CLI
          [] -> expectationFailure "expected at least one scenario"
    it "the report table carries the skip count in its own column, so a \
       \green law with thin coverage is visible as such" $ do
      let rs = [ ScenarioResult "f" "thinly covered" [] Passed 12
               , ScenarioResult "f" "fully covered" [] Passed 0 ]
      reportTable rs `shouldSatisfy` T.isInfixOf "| thinly covered | \9989 green | 12 |"
      reportTable rs `shouldSatisfy` T.isInfixOf "| fully covered | \9989 green | 0 |"
      reportTable rs `shouldSatisfy` T.isInfixOf "| verdict | skipped |"

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
    -- Review finding (round 1, Important): `check` and `vocab` each had
    -- their OWN copy of the dehole gate. Same predicate, same
    -- substitution, no shared function, nothing pinning them together --
    -- so a future change to one (a feature-level tag, a scenario-aware
    -- substitution) would diverge silently, and the symptom would be a
    -- deleted or wrong Vocabulary block found by a human reading the
    -- corpus, not a red test. Now one exported function,
    -- `Prop.deholeFor`, called by both.
    --
    -- These three tests are the cross-pin: the gate's OWN behaviour in
    -- both directions, and then -- the part that actually matters -- the
    -- two consumers agreeing about the SAME body, so a change that
    -- reached only one of them cannot stay quiet.
    it "the dehole gate substitutes for @property tags and not otherwise" $ do
      let body = "I render pieces <somePieces> at year <someYear> in style <someStyle>"
      Prop.deholeFor [Tag "property"] body `shouldNotBe` body
      Prop.deholeFor [Tag "property"] body `shouldBe` Prop.substituteExamples body
      Prop.deholeFor [Tag "target", Tag "property"] body
        `shouldBe` Prop.substituteExamples body
      Prop.deholeFor [] body `shouldBe` body
      Prop.deholeFor [Tag "target"] body `shouldBe` body
    it "check and vocab agree about the same body because they call the \
       \SAME gate -- a hole-bearing @property step is clean to `check` \
       \AND contributes its universes to `vocab`" $
      case parseFeature "t.feature" $ T.unlines
             [ "Feature: t"
             , "  @property"
             , "  Scenario: s"
             , "    When I render pieces <somePieces> at year <someYear> in style <someStyle>" ] of
        Left e -> expectationFailure (T.unpack e)
        Right f -> do
          Check.orphans allSteps f `shouldBe` []
          Check.valueErrors allSteps f `shouldBe` []
          map fst (Vocab.expectedVocab allSteps f) `shouldBe` ["pieces", "year", "style"]
    it "check and vocab agree in the OTHER direction too: the same body \
       \UNTAGGED is a value error to `check` and contributes nothing to \
       \`vocab` -- neither tool may claim a scenario runs differently \
       \than it does" $
      case parseFeature "t.feature" $ T.unlines
             [ "Feature: t"
             , "  Scenario: s"
             , "    When I render pieces <somePieces> at year <someYear> in style <someStyle>" ] of
        Left e -> expectationFailure (T.unpack e)
        Right f -> do
          Check.valueErrors allSteps f `shouldSatisfy` (not . null)
          Vocab.expectedVocab allSteps f `shouldBe` []
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

  describe "fixture diffs on every comparison path (Stage 1 Task 3)" $ do
    it "firstDiff names the path and both values on a nested leaf" $ do
      let e = fromJust (A.decodeStrict "{\"a\":{\"b\":[1,2,3]}}")
          g = fromJust (A.decodeStrict "{\"a\":{\"b\":[1,9,3]}}")
      case firstDiff e g of
        Nothing -> expectationFailure "no difference found in differing values"
        Just m  -> do
          m `shouldSatisfy` T.isInfixOf "$.a.b[1]"
          m `shouldSatisfy` T.isInfixOf "2"
          m `shouldSatisfy` T.isInfixOf "9"
    it "firstDiff is Nothing on equal values (it does not invent differences)" $
      let v = fromJust (A.decodeStrict "{\"a\":[1,2]}")
      in firstDiff v v `shouldBe` Nothing
    it "the masked whole-body failure names WHERE it differs" $ do
      -- masked compare against a fixture that differs outside the mask
      msg <- maskedFailureMessage
      msg `shouldSatisfy` T.isInfixOf "outside the mask"
      msg `shouldSatisfy` T.isInfixOf "$."
    it "the consumed-projection failure names WHERE it differs" $ do
      msg <- projectionFailureMessage
      msg `shouldSatisfy` T.isInfixOf "consumed projection"
      msg `shouldSatisfy` T.isInfixOf "$."

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
      -- Both settings on BOTH handles, defensively: `hDuplicateTo`
      -- redirects the live process `stdout` onto `h`'s underlying OS
      -- handle, but empirically does not reliably carry over `h`'s own
      -- TextEncoding/NewlineMode onto the now-redirected `stdout` (a real
      -- em dash written by `act` through `stdout` crashed with
      -- "commitAndReleaseBuffer: invalid argument" before `stdout` itself
      -- was also forced to UTF-8 here) -- and `noNewlineTranslation`
      -- keeps this test harness's own LF bytes from being silently
      -- rewritten to CRLF on Windows the same way Vocab.hs's write side
      -- had to guard against for the real corpus.
      hSetEncoding h utf8
      hSetNewlineMode h noNewlineTranslation
      bracket (hDuplicate stdout)
              (\old -> hDuplicateTo old stdout >> hClose old)
              (\_ -> do
                  hDuplicateTo h stdout
                  hSetEncoding stdout utf8
                  hSetNewlineMode stdout noNewlineTranslation
                  try act `finally` hFlush stdout)
    -- Read-side counterpart of Fix 4: the write side above is forced to
    -- UTF-8, but a plain `TIO.readFile` here would read those same bytes
    -- back through the LOCALE decoder (verified elsewhere in this file to
    -- be CP437 on this toolchain, not UTF-8), silently mangling any em
    -- dash a captured diagnosis line contains -- the identical defect
    -- Fix 4 closes for feature-file reads, just inside this test helper
    -- instead of the library.
    raw <- BS.readFile path
    pure (TE.decodeUtf8 raw, result)

-- Task 11 added `transportRaw` to `World`; touching all ~14 existing
-- `World base transport dir mempty False` construction sites with a new
-- positional sixth argument would be pure mechanical churn with no
-- test-relevant content. This smart constructor supplies the two fields
-- every one of those sites already left at the same values (`bound =
-- mempty`, `blessMode = False`), plus a `transportRaw` that fails
-- loudly, naming itself, so a test that starts exercising the two
-- binary-resource steps without being updated gets a clear signal
-- rather than a silent wrong answer.
mkWorld :: T.Text -> (T.Text -> IO (Either T.Text (BS.ByteString, A.Value))) -> FilePath -> World
mkWorld base tr dir = World base tr dir mempty False
  (\_ -> pure (Left "no raw transport configured for this test"))
  Nothing

-- The step action a body resolves to, in its full three-outcome form
-- (World.StepOutcome) -- used directly by the tests that are ABOUT
-- skipping.
firstOutcome :: Keyword -> T.Text -> Maybe (World -> IO StepOutcome)
firstOutcome k t = listToMaybe
  [ f | StepDef k' _ _ m <- allSteps, k' == k, Matched f <- [m t] ]

-- The two-outcome view, for the great majority of steps that cannot skip
-- at all. An UNEXPECTED skip is surfaced as a loud, named failure rather
-- than folded into either outcome: a step that quietly started skipping
-- would otherwise look exactly like a step that still runs (a pass) or
-- like a broken one (a failure), and the whole point of the third
-- outcome is that it is neither.
firstMatch :: Keyword -> T.Text -> Maybe (World -> IO (Either T.Text World))
firstMatch k t = fmap (\f w -> asEither <$> f w) (firstOutcome k t)
  where
    asEither (StepOk w)        = Right w
    asEither (StepFailed e)    = Left e
    asEither (StepSkipped why) = Left ("UNEXPECTED SKIP: " <> why)

-- Stage 1 Task 3: two small end-to-end helpers, each exercising a REAL
-- comparison step from allSteps (not a direct call to firstDiff) against
-- a fixture that differs at a known path -- same fake-transport idiom as
-- the masked-fixture and consumed-projection describe blocks above (a
-- fake transport returning a known body, the relevant step run via
-- firstMatch, the Left text handed back instead of asserted on inline).
-- fixtureDir points at the real test/fixtures directory where an
-- existing checked-in fixture already differs at a usable path, or at a
-- fresh temp directory where it doesn't (see projectionFailureMessage).
maskedFailureMessage :: IO T.Text
maskedFailureMessage = do
  let o = "{\"version\":\"9.9.9\",\"graphPin\":\"0123456789abcdef\"}"
      fake _ = pure (Right (o, fromJust (A.decodeStrict o)))
      w = mkWorld "http://x" fake "test/fixtures"
  Just get <- pure (firstMatch When "I GET /api/contract")
  Right w1 <- get w
  Just chk <- pure (firstMatch Then
    "the response equals fixture \"contract\" masking graphPin as sixteen hex characters")
  r <- chk w1
  case r of
    Left e  -> pure e
    Right _ -> error "expected the unmasked body difference to fail"

-- Uses a temp fixture directory rather than test/fixtures's checked-in
-- "eras" fixtures: those are top-level ARRAYS, whose first differing
-- path never contains a literal "$." (an array index appends as "[0]",
-- not ".something") -- an accident of that fixture's own shape, not a
-- limit of firstDiff. land-mask's Fields wrapper is a top-level OBJECT,
-- so a diff nested inside its "rings" field names a path of the "$.
-- something" shape the failure-message law actually asks for.
projectionFailureMessage :: IO T.Text
projectionFailureMessage = do
  tmpBase <- getTemporaryDirectory
  (uniqueFile, uh) <- openTempFile tmpBase "contract-runner-projection-diff-test"
  hClose uh
  removeFile uniqueFile
  let dir = uniqueFile <> "-dir"
      o = "{\"rings\":[1,2,9],\"extra\":true}"
      fake _ = pure (Right (o, fromJust (A.decodeStrict o)))
      w = mkWorld "http://x" fake dir
  createDirectoryIfMissing True dir
  (`finally` removeDirectoryRecursive dir) $ do
    BS.writeFile (dir </> "land.json") "{\"rings\":[1,2,3]}"
    Just get <- pure (firstMatch When "I GET /api/land")
    Right w1 <- get w
    Just chk <- pure (firstMatch Then
      "the consumed projection land-mask equals fixture \"land\"")
    r <- chk w1
    case r of
      Left e  -> pure e
      Right _ -> error "expected a genuinely differing consumed value to fail"

-- ---------- fake-server helpers for the sweep's property tests ----------
-- A fake scene body carrying exactly the pieces the URL turned ON. v0.1's
-- wire expresses four of the ten pieces (Steps.sceneUrl), so this is what
-- an HONEST server would answer: nested piece sets produce nested
-- resource sets, and a piece with no wire toggle contributes nothing.
-- Deliberately derived from the URL rather than from a canned response,
-- so a step that built the wrong URL (wrong year, wrong style, wrong
-- pieces) shows up as a wrong ANSWER instead of passing anyway.
wireResources :: T.Text -> A.Value
wireResources = idsValue . activeInUrl

-- Which of v0.1's four wire-expressible pieces a scene URL turned on.
activeInUrl :: T.Text -> [T.Text]
activeInUrl url =
     [ "ground"   | "relief=1" `T.isInfixOf` url ]
  ++ [ "water"    | not ("topo=0" `T.isInfixOf` url) ]
  ++ [ "labels"   | not ("labels=0" `T.isInfixOf` url) ]
  ++ [ "journeys" | not ("journeys=0" `T.isInfixOf` url) ]

-- The smallest scene-shaped body the scene steps will accept: every
-- top-level collection present and empty. Used by the shrinking tests,
-- where WHICH body comes back is irrelevant -- what is under test is
-- whether the transport answered at all, and the fake decides that from
-- the URL.
okBody :: BS.ByteString
okBody = "{\"features\":[],\"resources\":[],\"labels\":[],\"markers\":[]}"

-- A scene body carrying exactly these resource ids and nothing else.
idsValue :: [T.Text] -> A.Value
idsValue ns = A.object ["resources" A..= [ A.object ["id" A..= n] | n <- ns ]]

-- The style a scene URL asked for. Stands in for every style-dependent
-- field of a real payload: a fake that echoes it "repaints" exactly when
-- the URL says to, which is what lets the dress-locality law be exercised
-- for real against a fake.
styleInUrl :: T.Text -> T.Text
styleInUrl = T.takeWhile (/= '&') . snd . T.breakOnEnd "style="

-- Whether a scene URL's year is even -- an arbitrary but deterministic
-- way for a fake to make a law's precondition hold on some draws and not
-- others, so "some iterations skip" is exercised without depending on a
-- rare draw.
evenYearInUrl :: T.Text -> Bool
evenYearInUrl url =
  case reads (T.unpack (T.takeWhile (/= '&') (snd (T.breakOnEnd "year=" url)))) of
    [(y :: Int, "")] -> even y
    _                -> False

-- The binding a scenario mentioning exactly `hs` gets for hole `h` at
-- iteration `i`, parsed back through the very capture the corpus parses
-- it with. Asks `Prop.bindingsFor` -- the function the runner itself
-- calls -- rather than reconstructing the drawing rule here, so these
-- laws are pinned against the real registry and not against a copy of it
-- that could drift.
drawnAs :: FromCapture a => [T.Text] -> T.Text -> Int -> Either T.Text a
drawnAs hs h i =
  maybe (Left ("no binding for " <> h)) Right (Map.lookup h (Prop.bindingsFor hs i))
    >>= parseCap

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

-- Stage 1 Task 2: quantify the round-trip laws over the WHOLE type (via
-- these Arbitrary instances), not merely over parseCap's image -- see
-- Capture.hs's export-list comment. Piece, StyleName, and Year are new;
-- PieceSet already existed (Task 9's subset generation, above) and keeps
-- its `arbitrary` exactly as it was -- only `shrink` is added here, so
-- the distribution every existing PieceSet-quantified property draws
-- from (the round-trip law just above, and any future use) is unchanged.
instance Arbitrary Piece where
  arbitrary = elements [minBound .. maxBound]
  shrink p = takeWhile (< p) [minBound .. maxBound]

instance Arbitrary StyleName where
  arbitrary = StyleName <$> elements styleNames
  shrink _ = []

instance Arbitrary Year where
  arbitrary = Year <$> chooseInt (-4004, 100) `suchThat` (/= 0)
  shrink (Year y) = [ Year y' | y' <- shrink y, y' /= 0, y' >= -4004, y' <= 100 ]

-- Every subset of the piece universe, including the empty set (the
-- monoid identity that Task 9's subset generation relies on).
instance Arbitrary PieceSet where
  arbitrary = PieceSet . Set.fromList <$> sublistOf [minBound .. maxBound]
  shrink (PieceSet s) = [ PieceSet (Set.delete p s) | p <- Set.toList s ]
