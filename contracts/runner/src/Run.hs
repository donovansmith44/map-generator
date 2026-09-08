module Run where

import Control.Exception (SomeException, try)
import Data.Text (Text)
import qualified Data.Text as T
import Gherkin.Ast
import Gherkin.Parse (parseFeature)
import World

data Verdict = Passed | Failed Text | Skipped Text deriving (Eq, Show)

-- What ONE law's whole run came to: its verdict, plus how many of its
-- iterations never actually ran. The count is not part of the verdict on
-- purpose — a law that passed 88 of 100 iterations and skipped 12 PASSED,
-- and the 12 are a fact about the run's coverage, not about the law's
-- truth. Keeping them in one value (rather than returning a bare Verdict
-- and reporting skips out of band) is what lets `runWithProperties`,
-- `reportTable`, and the tests all see the same number.
data LawRun = LawRun { lawVerdict :: Verdict, lawSkips :: Int }
  deriving (Eq, Show)

-- The skip discipline, in the ONE place that states it: a law that never
-- ran is not green. `skipped` of `iterations` attempts skipped, with the
-- last precondition miss quoted, is a Failed when they ALL skipped —
-- naming the count, so the diagnosis says "this law was never exercised"
-- instead of quietly showing a green tick for zero evidence.
--
-- Stated over `iterations`, not specialized to the property runner: a
-- plain scenario is the n = 1 instance of exactly the same rule (see
-- `lawOnce`), so a single-run law that skips is red for the same reason
-- and by the same code, not by a second, separately-maintained rule.
lawTally :: Int -> Int -> Text -> Verdict -> LawRun
lawTally iterations skipped lastWhy v
  | iterations > 0 && skipped >= iterations =
      LawRun (Failed ("law never ran: all " <> tshow iterations
                      <> " iteration(s) skipped -- last precondition unmet: " <> lastWhy))
             skipped
  | otherwise = LawRun v skipped
  where tshow = T.pack . show

-- A plain, un-quantified scenario runs exactly once, and the same
-- discipline applies to it: its one run skipping means the law never ran.
lawOnce :: Verdict -> LawRun
lawOnce (Skipped why) = lawTally 1 1 why (Skipped why)
lawOnce v             = lawTally 1 0 "" v

data ScenarioResult = ScenarioResult
  { srFeature :: Text, srScenario :: Text, srTags :: [Tag], srVerdict :: Verdict
    -- ^ how many iterations of this law never ran (a precondition the
    -- drawn binding could not meet). 0 for a law with no preconditions.
  , srSkips :: Int }
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
          r <- try (f w) :: IO (Either SomeException StepOutcome)
          case r of
            Left ex                   -> pure (Failed (kwText k <> " " <> body <> "\n    \10007 "
                                                       <> T.pack (show ex)))
            Right (StepFailed e)      -> pure (Failed (kwText k <> " " <> body
                                                       <> "\n    \10007 " <> e))
            -- A precondition this draw cannot meet stops the scenario
            -- here: the steps after it exist to check a law that has
            -- nothing to check. Reported as its own verdict, never
            -- folded into Passed (which would be a vacuous green) or
            -- into Failed (which would be a red for an unbroken law).
            Right (StepSkipped why)   -> pure (Skipped why)
            Right (StepOk w')         -> go w' rest
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
        Left e  -> pure [ScenarioResult (T.pack p) "PARSE" [] (Failed e) 0]
        -- R97: `runnableScenarios`, not `ftScenarios` -- the feature's
        -- Background is prepended to every scenario here, at the one
        -- boundary that turns a parsed feature into things to run.
        Right f -> mapM (\sc -> mk (ftTitle f) sc . lawOnce <$> runScenario defs w sc)
                        (runnableScenarios f)
    mk ft sc (LawRun v s) = ScenarioResult ft (scName sc) (scTags sc) v s

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

-- The skip count gets its OWN column rather than being folded into the
-- verdict's prose: it is a number the diagnosis reads (how much of the
-- quantified space this law actually covered), not a decoration on the
-- verdict. A green law with a large skip count is a real finding — the
-- law holds, but over far less of its claimed domain than the run
-- suggests — and it is invisible unless the number is on the page.
reportTable :: [ScenarioResult] -> Text
reportTable rs = T.unlines $
     "| feature | scenario | verdict | skipped |"
   : "|---|---|---|---|"
   : [ "| " <> srFeature r <> " | " <> srScenario r <> " | " <> cell r
       <> " | " <> T.pack (show (srSkips r)) <> " |" | r <- rs ]
  where
    cell r = case (srVerdict r, isTarget r) of
      (Passed, False)   -> "\9989 green"
      (Passed, True)    -> "\128994 green (target already met!)"
      (Failed _, True)  -> "\128308 red (expected \8212 @target)"
      (Failed e, False) -> "\10060 RED \8212 " <> T.replace "\n" " " (T.take 160 e)
      (Skipped why, _)  -> "\9197 " <> why
