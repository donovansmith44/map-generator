{-# LANGUAGE ExistentialQuantification #-}
module Prop where

import Data.Map.Strict (Map)
import qualified Data.Map.Strict as Map
import qualified Data.Set as Set
import Data.Text (Text)
import qualified Data.Text as T
import qualified Data.Text.IO as TIO
import Test.QuickCheck
import Test.QuickCheck.Gen (Gen (..), unGen)
import Test.QuickCheck.Random (mkQCGen)
import Capture
import Gherkin.Ast
import Gherkin.Parse (parseFeature)
import Run
import World (StepDef, World)

-- An existential wrapper so `holeRegistry` can hold generators of
-- different result types (Year, PieceSet, StyleName, ...) in one Map,
-- while still being able to render whatever comes out — `FromCapture`
-- is captured alongside the generator, not inferred at use site.
data SomeHole = forall a. FromCapture a => SomeHole (Gen a)

-- A possibly-empty subset of every piece. R6/the scene's own algebra:
-- the empty scene is the monoid identity, and `sublistOf` genuinely
-- includes `[]` in its codomain, so the identity IS reachable here —
-- see the "genPieces can generate the empty piece set" test, which
-- samples this generator directly to prove it, not just trust the docs.
genPieces :: Gen PieceSet
genPieces = PieceSet . Set.fromList <$> sublistOf [minBound .. maxBound]

genYear :: Gen Year
genYear = Year <$> chooseInt (-4004, 100)

-- The one and only place a hole name is given meaning. EXPLICIT and
-- TYPED, no name-sniffing/inference: a hole not listed here has no
-- generator, and `runScenarioProperty`/`substituteExamples` must treat
-- that as a loud failure, never a silent pass (requirement 1).
holeRegistry :: Map Text SomeHole
holeRegistry = Map.fromList
  [ ("someYear",   SomeHole genYear)
  , ("somePieces", SomeHole genPieces)
  , ("someA",      SomeHole genPieces)
  , ("someB",      SomeHole genPieces)
  , ("someStyle",  SomeHole (StyleName <$> elements styleNames))
  ]

-- Every "<hole>" token appearing in one raw step body, left to right.
-- Exported (not just used internally by `holesOf`) so `Check.classify`
-- can scan a bare `Step`'s body for holes too, without going through a
-- whole `Scenario` -- that's what lets an UNREGISTERED hole be caught at
-- `check` time regardless of which capture it happens to land in.
holesIn :: Text -> [Text]
holesIn b = case T.breakOn "<" b of
  (_, rest) | T.null rest -> []
  (_, rest) ->
    let (h, rest') = T.breakOn ">" (T.drop 1 rest)
    in h : if T.null rest' then [] else holesIn (T.drop 1 rest')

-- Every "<hole>" token appearing in any step body of a scenario, in
-- first-seen order (duplicates removed by `dedupe` at the call site).
holesOf :: Scenario -> [Text]
holesOf sc = concatMap (holesIn . stepBody) (scSteps sc)

-- First-seen order, genuinely -- a naive `foldr (\x acc -> if x \`elem\`
-- acc then acc else x : acc) []` looks right but actually REVERSES
-- relative order among the first occurrences (try it on [a,b,a,c]: it
-- yields [b,a,c], not [a,b,c]), because foldr conses the LATEST
-- not-yet-seen element it encounters (scanning right-to-left) onto the
-- front. Harmless everywhere this is currently used -- binding lookup is
-- by Map key, and the failing-binding report sorts by key too -- but the
-- name promises an order this shape doesn't deliver, so it earns an
-- explicit accumulator instead.
dedupe :: [Text] -> [Text]
dedupe = go []
  where
    go _ [] = []
    go seen (x : xs)
      | x `elem` seen = go seen xs
      | otherwise     = x : go (x : seen) xs

-- Every "<hole>" token in every step of a scenario gets textually
-- replaced by its bound rendering. `env` need not cover every hole in
-- `sc` for this to typecheck (it's just Text -> Text substitution) --
-- callers (runScenarioProperty, below) are the ones responsible for
-- ensuring every hole actually used has a binding first.
substitute :: Map Text Text -> Scenario -> Scenario
substitute env sc = sc { scSteps = map sub (scSteps sc) }
  where
    sub st = st { stepBody = Map.foldrWithKey
                    (\h v b -> T.replace ("<" <> h <> ">") v b)
                    (stepBody st) env }

-- Render one hole's generator at a given iteration index. The seed is
-- derived ONLY from the iteration index (never from wall-clock time or
-- any other source of entropy), which is what makes a whole property
-- run reproducible: the same scenario run twice generates the exact
-- same sequence of bindings and therefore the exact same verdict,
-- including a failing run's counterexample text (requirement 2 — see
-- the "same scenario run twice" test, which asserts this by actually
-- running it twice and comparing, not by inspection).
renderHole :: SomeHole -> Int -> Text
renderHole (SomeHole g) i =
  renderCap (unGen g (mkQCGen (fromIntegral i * 7919)) (min 30 (i + 3)))

-- Run one @property-tagged scenario N times over generated bindings. A
-- scenario with no holes at all is a harmless (if wasteful) degenerate
-- case: every iteration substitutes nothing and re-runs the same body.
runScenarioProperty :: [StepDef] -> World -> Int -> Scenario -> IO Verdict
runScenarioProperty defs w n sc =
  case unregistered of
    (h : _) -> pure . Failed $
      "unregistered property hole <" <> h <> "> in scenario \""
        <> scName sc <> "\" -- add it to Prop.holeRegistry"
    [] -> loop 0
  where
    hs = dedupe (holesOf sc)
    unregistered = [ h | h <- hs, h `Map.notMember` holeRegistry ]
    holes = [ (h, holeRegistry Map.! h) | h <- hs ]
    loop i
      | i >= n = pure Passed
      | otherwise = do
          let env = Map.fromList [ (h, renderHole s i) | (h, s) <- holes ]
          v <- runScenario defs w (substitute env sc)
          case v of
            Passed -> loop (i + 1)
            Failed e -> pure . Failed $
              e <> "\n    with " <> T.intercalate ", "
                [ h <> " = " <> val | (h, val) <- Map.toList env ]
            s -> pure s

-- For the totality check ONLY (Check.dehole, applied only when the
-- enclosing scenario is @property-tagged — see Check.hs): every
-- registered hole in a raw step body is replaced with ONE fixed,
-- deterministic example value (seed 1, size 3 — arbitrary but stable),
-- so a capture that validates its input (Piece, Year, Style) sees a real
-- value instead of literal "<hole>" text, and a capture that does NOT
-- validate (UrlPath, which only rejects whitespace) is not fooled into
-- matching on a hole that merely happens to contain no whitespace of its
-- own (ruling R33). An UNREGISTERED hole is deliberately left untouched
-- here — it is not this function's job to fail loudly (it has no
-- scenario to name in an error). `Check.classify` scans for that case
-- itself, BEFORE calling this function, using `holesIn` above; the
-- run-time equivalent (naming the scenario too) is
-- `runScenarioProperty`'s own check, just above.
substituteExamples :: Text -> Text
substituteExamples b0 = foldr rep b0 (Map.toList holeRegistry)
  where
    rep :: (Text, SomeHole) -> Text -> Text
    rep (h, SomeHole g) =
      T.replace ("<" <> h <> ">") (renderCap (unGen g (mkQCGen 1) 3))

-- Plain scenarios run once; @property scenarios run `n` times over
-- generated bindings (first failure's Verdict already carries its
-- binding values, from runScenarioProperty above).
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
