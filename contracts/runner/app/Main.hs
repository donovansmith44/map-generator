module Main where

import qualified Data.Map.Strict as Map
import qualified Data.Text as T
import qualified Data.Text.IO as TIO
import Network.HTTP.Client (newManager, defaultManagerSettings)
import Options.Applicative
import System.Directory (listDirectory, doesDirectoryExist)
import System.Exit (exitFailure, exitSuccess)
import System.FilePath ((</>), takeExtension)
import System.IO (stdout, stderr, hSetEncoding, utf8)
import Run
import Steps (allSteps)
import World
import qualified Check
import qualified Vocab
import qualified Prop

data Cmd
  = CmdRun  { cBase :: String, cDir :: FilePath, cBless :: Bool, cRuns :: Int }
  | CmdCheck FilePath
  | CmdVocab { vDir :: FilePath, vWrite :: Bool }

cmd :: Parser Cmd
cmd = hsubparser
  (  command "run"   (info (CmdRun <$> strOption (long "base-url")
                                   <*> argument str (metavar "DIR")
                                   <*> switch (long "bless")
                                   <*> option auto (long "property-runs" <> value 100))
                       (progDesc "execute a contract directory against a server"))
  <> command "check" (info (CmdCheck <$> argument str (metavar "DIR"))
                       (progDesc "totality: every step matches exactly one definition"))
  <> command "vocab" (info (CmdVocab <$> argument str (metavar "DIR")
                                     <*> switch (long "write"))
                       (progDesc "verify (or --write) Vocabulary blocks against the types"))
  )

featureFiles :: FilePath -> IO [FilePath]
featureFiles dir = do
  entries <- listDirectory dir
  fmap concat . mapM walk $ [ dir </> e | e <- entries ]
  where
    walk p = do
      isDir <- doesDirectoryExist p
      if isDir then featureFiles p
      else pure [ p | takeExtension p == ".feature" ]

main :: IO ()
main = do
  -- Fix 6: this is a Windows-first project, and Windows consoles default
  -- to a legacy code page that can't encode characters like an em dash —
  -- `checkDir`'s BAD-VALUE line crashed with
  -- "commitAndReleaseBuffer: invalid argument (cannot encode character
  -- '\8212')" before this landed. Force UTF-8 on both output handles at
  -- the entry point so ALL of this runner's output (check, run, vocab) is
  -- safe regardless of the console's code page, rather than requiring
  -- every caller to `chcp 65001` first — the Stage 0 diagnosis document is
  -- generated straight from this output.
  hSetEncoding stdout utf8
  hSetEncoding stderr utf8
  c <- execParser (info (cmd <**> helper) fullDesc)
  case c of
    CmdRun base dir bless runs -> do
      mgr <- newManager defaultManagerSettings
      let w = World (T.pack base) (httpTransport mgr) (dir </> "fixtures") Map.empty bless
                    (httpTransportRaw mgr)
      files <- featureFiles dir
      results <- Prop.runWithProperties allSteps w runs files
      TIO.putStrLn (reportTable results)
      let reds = hardReds results
      if null reds then exitSuccess
      else TIO.putStrLn (T.pack (show (length reds)) <> " non-target failures")
           >> exitFailure
    CmdCheck dir -> Check.checkDir allSteps dir
    CmdVocab dir w -> Vocab.vocabDir allSteps dir w
