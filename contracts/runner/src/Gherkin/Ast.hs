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
--
-- R97: `ftBackground` is the steps stated ONCE for the whole feature and
-- run before every scenario in it. It is a first-class field rather than
-- an expansion performed at parse time, and the difference is not
-- cosmetic — see `runnableScenarios` below, and the report's design
-- section, for what expansion would have cost.
data Feature = Feature
  { ftTitle :: Text, ftTags :: [Tag], ftPreamble :: [Text]
  , ftVocab :: [(Text, Text)]
  , ftBackground :: [Step]
  , ftScenarios :: [Scenario] }
  deriving (Eq, Show)

-- THE ONE PLACE the background becomes part of a scenario's body.
--
-- Every consumer that RUNS or SUBSTITUTES steps goes through this, so
-- the background is part of the scenario for exactly the things it must
-- be part of, and separate for exactly the things it must be separate
-- for:
--
--   * `Run.runFeatureFiles` and `Prop.runWithProperties` take these, so
--     the background runs first and -- decisively, R97 requirement 1 --
--     a hole in the background is the SAME hole as the one in the
--     scenario. `Prop.holesOf` scans `scSteps`, so once the background
--     is in there, one draw per iteration covers both, and `substitute`
--     and the shrinker reach both. No separate preamble draw exists to
--     get out of step with the body.
--   * `Vocab.expectedVocab` takes these, so a background step
--     contributes its universes to the feature's table, deholed under
--     the tags of the scenario it is running in.
--   * `Check` deliberately does NOT take these for the totality law: a
--     background step is checked and reported ONCE, named by the
--     feature, rather than once per scenario. Reporting one defect N
--     times under N scenario names is the "same defect under two
--     labels" this codebase already refuses elsewhere.
--
-- Order is background-then-own, and it is load-bearing: a scenario that
-- re-binds a name the background bound must win, which only happens if
-- its own steps run second.
runnableScenarios :: Feature -> [Scenario]
runnableScenarios f =
  [ sc { scSteps = ftBackground f ++ scSteps sc } | sc <- ftScenarios f ]
