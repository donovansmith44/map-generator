module Vocab where

import World (StepDef)

-- Task 8 implements Vocabulary-block verification (and --write to
-- regenerate a feature's Vocabulary table from the types). This stub
-- exists only so Main compiles and `run`/`check` are usable from Task 6
-- on; invoking `vocab` before Task 8 lands must fail loudly, not
-- silently succeed.
vocabDir :: [StepDef] -> FilePath -> Bool -> IO ()
vocabDir _defs _dir _write = error "Task 8: not yet implemented"
