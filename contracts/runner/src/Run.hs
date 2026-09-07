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
            Right (Left e)   -> pure (Failed (kwText k <> " " <> body <> "\n    \10007 " <> e))
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
      (Passed, False)   -> "\9989 green"
      (Passed, True)    -> "\128994 green (target already met!)"
      (Failed _, True)  -> "\128308 red (expected \8212 @target)"
      (Failed e, False) -> "\10060 RED \8212 " <> T.replace "\n" " " (T.take 160 e)
      (Skipped why, _)  -> "\9199 " <> why
