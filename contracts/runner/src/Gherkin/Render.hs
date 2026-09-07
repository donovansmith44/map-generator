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
    kw Given = "Given"
    kw When = "When"
    kw Then = "Then"
