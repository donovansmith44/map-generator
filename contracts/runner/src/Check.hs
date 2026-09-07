module Check where

import World (StepDef)

-- Task 7 implements the totality check (every step body matches exactly
-- one StepDef, naming ambiguous/undefined bodies). This stub exists only
-- so Main compiles and `run`/`vocab` are usable from Task 6 on; invoking
-- `check` before Task 7 lands must fail loudly, not silently succeed.
checkDir :: [StepDef] -> FilePath -> IO ()
checkDir _defs _dir = error "Task 7: not yet implemented"
