module Gherkin.Ast where

import Data.Text (Text)

newtype Tag = Tag Text deriving (Eq, Ord, Show)

data Keyword = Given | When | Then deriving (Eq, Ord, Show, Bounded, Enum)

data StepArg = DocString Text | Table [[Text]] deriving (Eq, Show)

data Step = Step { stepKw :: Keyword, stepBody :: Text, stepArg :: Maybe StepArg }
  deriving (Eq, Show)

data Scenario = Scenario { scName :: Text, scTags :: [Tag], scSteps :: [Step] }
  deriving (Eq, Show)

-- Vocabulary rows are (term, description) pairs — Task 8 gives them law.
data Feature = Feature
  { ftTitle :: Text, ftTags :: [Tag], ftPreamble :: [Text]
  , ftVocab :: [(Text, Text)], ftScenarios :: [Scenario] }
  deriving (Eq, Show)
