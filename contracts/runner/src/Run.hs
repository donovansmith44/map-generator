module Run where

import Control.Exception (SomeException, try)
import Data.Text (Text)
import qualified Data.Text as T
import Gherkin.Ast
import Gherkin.Parse (parseFeature)
import World

data Verdict = Passed | Failed Text | Skipped Text deriving (Eq, Show)

data ScenarioResult = ScenarioResult
  { srFeature :: Text, srScenario :: Text, srTags :: [Tag], srVerdict :: Verdict }
  deriving (Eq, Show)

runScenario :: [StepDef] -> World -> Scenario -> IO Verdict
runScenario defs w0 sc = go w0 (scSteps sc)
  where
    go _ [] = pure Passed
    go w (Step k body _ : rest) =
      -- R23: three outcomes, not two. Run the first full Matched action
      -- if there is one — a Matched always wins over a mere ClaimError,
      -- which is what lets two overlapping definitions coexist without
      -- either being taught about the other (Check.hs has the concrete
      -- collisions this resolves). Only when NOTHING matches do we fall
      -- back to the best claimed error; only when nothing matches OR
      -- claims is the step genuinely undefined.
      --
      -- Fix 2 (post-Task-7 review): two or more definitions truly
      -- MATCHING the same body is exactly the ambiguity Check.hs's
      -- totality law is fatal about at `check` time — silently running
      -- the head of `matches` here would let that same shadowing back in
      -- at `run` time, on any feature file `check` hasn't (yet) been run
      -- against. So `run` refuses it too, naming every competing sketch,
      -- instead of picking one.
      let cands   = [ d | d <- defs, defKw d == k ]
          results = [ (defSketch d, defRun d body) | d <- cands ]
          matches = [ (sk, f) | (sk, Matched f) <- results ]
      in case matches of
        [(_, f)] -> do
          r <- try (f w) :: IO (Either SomeException (Either Text World))
          case r of
            Left ex          -> pure (Failed (kwText k <> " " <> body <> "\n    \10007 "
                                              <> T.pack (show ex)))
            Right (Left e)   -> pure (Failed (kwText k <> " " <> body <> "\n    \10007 " <> e))
            Right (Right w') -> go w' rest
        [] -> case [ e | (_, ClaimError e) <- results ] of
          (e : _) -> pure (Failed (kwText k <> " " <> body <> "\n    \10007 " <> e))
          []      -> pure (Failed ("undefined step: " <> kwText k <> " " <> body))
        _ -> pure (Failed (kwText k <> " " <> body <> "\n    \10007 ambiguous: matches "
                          <> T.pack (show (length matches)) <> " definitions: "
                          <> T.intercalate " | " (map fst matches)))
    kwText Given = "Given"; kwText When = "When"; kwText Then = "Then"

runFeatureFiles :: [StepDef] -> World -> [FilePath] -> IO [ScenarioResult]
runFeatureFiles defs w paths = fmap concat . mapM one $ paths
  where
    one p = do
      -- Fix 4: the shared explicit-UTF-8 reader (World.readFeatureFile),
      -- not a plain TIO.readFile -- see its own comment for why that
      -- silently corrupted this corpus's em dashes on this toolchain.
      src <- readFeatureFile p
      case parseFeature p src of
        Left e  -> pure [ScenarioResult (T.pack p) "PARSE" [] (Failed e)]
        Right f -> mapM (\sc -> ScenarioResult (ftTitle f) (scName sc) (scTags sc)
                                  <$> runScenario defs w sc)
                        (ftScenarios f)

isTarget :: ScenarioResult -> Bool
isTarget = elem (Tag "target") . srTags

-- The classification this whole stage exists to produce: a non-@target
-- scenario whose verdict is Failed is a hard red (fails the run); a
-- @target scenario's Failed verdict is EXPECTED red (reported, not
-- fatal), and a @target scenario's Passed verdict is information (a met
-- target), not fatal either. Exported so `main` calls one law instead of
-- duplicating this list comprehension into its own test.
hardReds :: [ScenarioResult] -> [ScenarioResult]
hardReds rs = [ r | r <- rs, not (isTarget r), Failed _ <- [srVerdict r] ]

reportTable :: [ScenarioResult] -> Text
reportTable rs = T.unlines $
     "| feature | scenario | verdict |"
   : "|---|---|---|"
   : [ "| " <> srFeature r <> " | " <> srScenario r <> " | " <> cell r <> " |" | r <- rs ]
  where
    cell r = case (srVerdict r, isTarget r) of
      (Passed, False)   -> "\9989 green"
      (Passed, True)    -> "\128994 green (target already met!)"
      (Failed _, True)  -> "\128308 red (expected \8212 @target)"
      (Failed e, False) -> "\10060 RED \8212 " <> T.replace "\n" " " (T.take 160 e)
      (Skipped why, _)  -> "\9197 " <> why
