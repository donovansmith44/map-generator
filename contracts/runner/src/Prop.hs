module Prop where

import Run (ScenarioResult, runFeatureFiles)
import World (StepDef, World)

-- Task 9 replaces this with real property-based scenario replication
-- (running `@property` scenarios `runs` times over generated instances).
-- Until then this is a straight passthrough so Main compiles and the CLI
-- is usable from Task 6 on: it just runs every feature file once.
runWithProperties :: [StepDef] -> World -> Int -> [FilePath] -> IO [ScenarioResult]
runWithProperties defs w _runs files = runFeatureFiles defs w files
