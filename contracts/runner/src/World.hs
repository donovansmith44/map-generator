module World where

import Control.Exception (SomeException, try)
import Data.Aeson (Value, eitherDecodeStrict, encode)
import Data.ByteString (ByteString)
import qualified Data.ByteString as BS
import qualified Data.ByteString.Lazy as BL
import qualified Data.ByteString.Lazy.Char8 as BL8
import Data.List (intercalate)
import Data.Map.Strict (Map)
import qualified Data.Map.Strict as Map
import Data.Text (Text)
import qualified Data.Text as T
import qualified Data.Text.Encoding as TE
import Gherkin.Ast (Keyword)
import Capture (Center, StyleName, Universe, Year, Zoom)
import Pattern
import Network.HTTP.Client
import Network.HTTP.Types.Status (statusCode)
import System.Environment (lookupEnv)
import System.FilePath ((</>), takeDirectory)
import System.Process (CreateProcess (..), proc, readCreateProcessWithExitCode)

-- Final-review Fix 4: the ONE explicit-UTF-8 reader for a .feature file,
-- shared by every call site that reads one (Run.runFeatureFiles,
-- Check.checkDir, Prop.runWithProperties, Vocab.vocabDir) -- there used to
-- be two DISAGREEING readers instead of one. Three sites called plain
-- `TIO.readFile`, whose text-handle decoder uses the process's LOCALE
-- encoding, not UTF-8 -- verified empirically on this toolchain: the
-- locale encoding here is CP437, and `TIO.readFile` silently mangled a
-- 3-byte UTF-8 em dash (U+2014) into three separate CP437 characters
-- instead of raising an error (no exception, no warning -- a silent
-- corruption). Vocab.hs alone got this right, decoding the raw bytes as
-- UTF-8 explicitly, with a comment recording exactly that experiment --
-- correct, not the wrong half of the disagreement. The corpus is full of
-- em dashes (every feature title uses one) and the Stage 0 diagnosis
-- table is generated straight from `Run.runFeatureFiles`'s output, so the
-- three-site version was the live bug: the mojibake actually visible in
-- this review's own console output ("the scene ΓÇö a picture...") is
-- this defect firing, not a cosmetic footnote. One shared function
-- collapses the disagreement structurally -- a future fifth call site
-- gets the correct decode for free instead of a fresh chance to guess
-- wrong.
readFeatureFile :: FilePath -> IO Text
readFeatureFile p = TE.decodeUtf8 <$> BS.readFile p

data World = World
  { baseUrl      :: Text
  , transport    :: Text -> IO (Either Text (ByteString, Value))
  , fixtureDir   :: FilePath
  , bound        :: Map Text (ByteString, Value)
  , blessMode    :: Bool
  -- Task 11: /api/resource and /api/resources return BINARY bodies, not
  -- JSON -- `transport` above decodes JSON and would `Left` on them. A
  -- second, parallel transport that skips the decode entirely, for the
  -- two byte-identity steps that need raw bytes rather than a parsed
  -- Value.
  , transportRaw :: Text -> IO (Either Text ByteString)
  -- Phase S review, fixes 2/3: the combine step ("combining A and B
  -- equals rendering <someA> plus <someB>") must render its own union
  -- scene at the SAME year AND STYLE the two parts were rendered at.
  -- Smuggling the year through the stringly-typed `bound` scene map
  -- under a magic "_year" key (this module's first draft) forced a
  -- `Just (_, Number sci) -> ...; Just _ -> Left "bound _year is not a
  -- number"` branch that could never actually fire, and said nothing
  -- about style at all -- the combine step then hardcoded style to
  -- "canaan", a tuned constant the owner's law forbids. A typed field,
  -- symmetric with `transportRaw` above, removes both problems by
  -- construction: there is no ill-typed value this can hold, and no
  -- style left to hardcode.
  , lastRender   :: Maybe (Year, StyleName)
  -- The step phase: WHICH CAMERA a bound scene was drawn with, per bound
  -- name. Three of camera.feature's laws ("keeps every feature ... in
  -- view", "every marker and label ... still in narrow's view", "every
  -- label ... anchors in view") are stated relative to a camera the
  -- scenario named on an EARLIER line and never repeats, so the Then step
  -- has to be able to ask what camera `viewed`/`narrow` was rendered at.
  --
  -- Typed and per-name, for the same reason `lastRender` is typed
  -- (World.hs's own note on smuggling a year through the stringly-typed
  -- `bound` map): there is no ill-typed value this can hold, and no
  -- camera left to guess. Per-NAME rather than "last", unlike
  -- `lastRender`, because these laws compare TWO scenes drawn at two
  -- different cameras in the same scenario -- a single "last camera"
  -- would answer for the wrong one exactly half the time.
  , cameras      :: Map Text (Center, Zoom)
  -- Fix round 1, finding 4: a transport that does NOT treat a non-2xx as
  -- an error.
  --
  -- `transport` and `transportRaw` both collapse every non-2xx into a
  -- `Left` naming the code (`checkStatus` below). That is exactly right
  -- for a law that needs the PAYLOAD -- a law reading a 404 error page as
  -- if it were geometry is the defect `checkStatus` was added to close.
  -- It is exactly wrong for a law about a REFUSAL, because it destroys
  -- the only two things such a law needs to see: the status class, and
  -- the body that is supposed to name what was refused. Reading a `Left`
  -- as "the server refused, so the law is met" makes a 500, a 502, or a
  -- route that does not exist at all into a satisfied contract -- the
  -- same defect one layer up from the one this module's own comment
  -- records having already been burned by.
  --
  -- So: a third transport, whose `Right` carries the status code and the
  -- raw body whatever the code was, and whose `Left` is reserved for a
  -- genuine transport failure. It is a separate field rather than a
  -- widening of `transportRaw` so that the twenty-odd steps that must
  -- NOT see an error page as data keep the transport that refuses to
  -- hand them one.
  , transportProbe :: Text -> IO (Either Text (Int, ByteString))
  -- R99: the golden gate is not an HTTP endpoint, it is a PROGRAM, and
  -- its laws drive it the way a person does -- with arguments, reading
  -- what it prints. A fourth transport, on exactly the same terms as
  -- the three above: `Right` is what the gate said (its whole stdout),
  -- `Left` is a failure to run it at all (no node, no gate, no
  -- harness). A non-zero EXIT is emphatically not a `Left` -- the gate
  -- exiting 1 is the gate answering, and the laws are about which
  -- answer it gave.
  --
  -- Injected rather than called directly for the reason `transport` is:
  -- the steps that read a verdict are then testable against a gate a
  -- test writes, without a browser, and the one real implementation
  -- (`nodeGate`) has one call site.
  , runGate     :: GateInvocation -> IO (Either Text Text)
  -- The condition the NEXT judging step will run the gate under.
  --
  -- The corpus states a change on one line ("I repaint probe 7 of
  -- levant in the drawn map") and judges the changed map on the next
  -- ("I judge the repainted map as judged"), so the change has to
  -- survive between two steps. Typed and its own field, for the reason
  -- `lastRender` and `cameras` are: the alternative is smuggling a
  -- script path and a JSON blob through the stringly-typed `bound` map
  -- under a magic key, which this module has already been burned by
  -- once.
  , pending      :: Maybe GateCondition
  }

-- ONE declared condition: the script the gate installs in the page, and
-- the argument it hands that script. This mirrors the gate's own single
-- affordance exactly (`--condition` / `--condition-arg`), rather than
-- naming any particular situation -- there is no constructor here for
-- "the renderer died", because there is no flag there for it either.
data GateCondition = GateCondition
  { condScript :: FilePath
    -- ^ the condition's file NAME. Conditions live beside the gate they
    -- drive (they are written against the page's internals, which is the
    -- gate's own business), so where that directory is on disk is
    -- resolved once, in `gateArgv`, instead of being carried around as
    -- an absolute path by every step that names one.
  , condArg    :: Maybe Value
  } deriving (Eq, Show)

-- ONE INVOCATION OF THE GATE, as a value rather than a list of strings.
--
-- Every field is an input the gate declares (`--check`, `--stops`,
-- `--baseline`, `--condition`/`--condition-arg`) and nothing here names
-- a situation: there is no `invRendererDied`, because the gate has no
-- such flag and a step that wanted one would have to write a condition
-- file like everybody else. Turning this into a command line is
-- `gateArgv`, which is pure and therefore pinnable without a browser.
data GateInvocation = GateInvocation
  { invBless     :: Bool
  , invStops     :: [Int]
  , invBaseline  :: Maybe FilePath
  , invCondition :: Maybe GateCondition
  } deriving (Eq, Show)

-- The command line one invocation becomes, given where the gate itself
-- lives. Pure: the argument order, the stop spelling, and the condition
-- path are all things a test can read back without running node.
gateArgv :: FilePath -> GateInvocation -> [String]
gateArgv gateJs inv =
  [ gateJs ]
  ++ [ "--check" | not (invBless inv) ]
  ++ (case invStops inv of
        [] -> []
        ys -> ["--stops", intercalate "," (map show ys)])
  ++ maybe [] (\b -> ["--baseline", b]) (invBaseline inv)
  ++ maybe [] condArgv (invCondition inv)
  where
    condArgv c =
      ["--condition", takeDirectory gateJs </> "conditions" </> condScript c]
      ++ maybe [] (\a -> ["--condition-arg", BL8.unpack (encode a)]) (condArg c)

-- `transport` is a function and has no Show instance, so World cannot
-- derive Show. Tests (Task 5's brief) need `shouldSatisfy` on an
-- `Either Text World`, which requires `Show World` at compile time even
-- though the function value itself is never actually printed on the
-- success path this task exercises. Show everything except the function,
-- and the bound-scene map by its keys only (the values are raw response
-- bytes + parsed JSON, not useful in a failure message).
instance Show World where
  show w =
    "World { baseUrl = " <> show (baseUrl w)
      <> ", fixtureDir = " <> show (fixtureDir w)
      <> ", bound = " <> show (Map.keys (bound w))
      <> ", blessMode = " <> show (blessMode w)
      <> ", lastRender = " <> show (lastRender w)
      <> ", cameras = " <> show (Map.toList (cameras w))
      <> ", pending = " <> show (pending w)
      <> " }"

-- Phase S review, fix 1: a non-2xx status used to come back as `Right`
-- from both transports below -- on a server where /api/resource simply
-- doesn't exist, two fetches of the same 404 (or 500) error page are
-- byte-identical, so "fetching {name}'s first resource twice yields
-- identical bytes" (which is NOT @target -- it is expected to be green)
-- would report GREEN against a server that cannot serve resources at
-- all. A check satisfiable by its own failure mode is no check (see
-- MEMORY: verify-distinct-not-nonnull). Extracted as a pure predicate,
-- not inlined into either IO transport, so the law itself -- "2xx or a
-- Left naming the code and the url" -- is testable without a live
-- server (standing up one is the NEXT task's job, not this one's).
checkStatus :: Text -> Int -> Either Text ()
checkStatus url code
  | code >= 200 && code < 300 = Right ()
  | otherwise = Left ("HTTP " <> T.pack (show code) <> " from " <> url)

httpTransport :: Manager -> Text -> IO (Either Text (ByteString, Value))
httpTransport mgr url = do
  req <- parseRequest (T.unpack url)
  resp <- httpLbs req mgr
  let raw  = BL.toStrict (responseBody resp)
      code = statusCode (responseStatus resp)
  pure $ do
    checkStatus url code
    case eitherDecodeStrict raw of
      Right v -> Right (raw, v)
      Left e  -> Left (T.pack e <> " for " <> url)

-- Same request as `httpTransport`, minus the JSON decode -- for the two
-- steps (Task 11) that fetch /api/resource and /api/resources, whose
-- bodies are binary geometry, not JSON. Shares `httpTransport`'s status
-- check above -- `httpTransport` was only ever accidentally shielded
-- from this by its JSON decode failing on an HTML error page, which is
-- no protection at all for a plain-bytes transport that has no decode
-- step to fail.
httpTransportRaw :: Manager -> Text -> IO (Either Text ByteString)
httpTransportRaw mgr url = do
  req <- parseRequest (T.unpack url)
  resp <- httpLbs req mgr
  let code = statusCode (responseStatus resp)
  pure $ do
    checkStatus url code
    Right (BL.toStrict (responseBody resp))

-- The same request again, with NO status check at all: the code and the
-- body, whatever they are. See `transportProbe`'s field comment for why
-- a law about a refusal needs both and cannot be given either by the two
-- transports above.
httpTransportProbe :: Manager -> Text -> IO (Either Text (Int, ByteString))
httpTransportProbe mgr url = do
  req <- parseRequest (T.unpack url)
  resp <- httpLbs req mgr
  pure (Right (statusCode (responseStatus resp), BL.toStrict (responseBody resp)))

-- THE GATE, RUN. `node golden.js <args>`, from a directory where
-- `require('playwright-core')` resolves -- the gate says so in its own
-- header, and that directory is a fact about this machine (the harness
-- convention keeps node_modules in a job tmp dir, never in the repo),
-- so it is named by the environment rather than guessed at here.
--
-- A missing environment is a LOUD failure, never a skip: a law about
-- the gate that quietly does not run is exactly the vacuous green this
-- whole stage exists to outlaw. `Left` says which variable is unset and
-- what it is for.
--
-- stdout AND stderr both come back, concatenated: the verdict line is
-- on stdout, but a gate that died on its way to printing one said why
-- on stderr, and a diagnosis that threw half of it away would report
-- "no verdict line" over a message that named the problem exactly.
nodeGate :: GateInvocation -> IO (Either Text Text)
nodeGate inv = do
  mHarness <- lookupEnv "GOLDEN_GATE_HARNESS"
  mJs <- lookupEnv "GOLDEN_GATE_JS"
  case (mHarness, mJs) of
    (Just harness, Just js) -> do
      let proc0 = (proc "node" (gateArgv js inv)) { cwd = Just harness }
      r <- try (readCreateProcessWithExitCode proc0 "")
      pure $ case r of
        Left e -> Left ("could not run the gate: " <> T.pack (show (e :: SomeException)))
        Right (_, out, err) -> Right (T.pack out <> T.pack err)
    _ -> pure (Left gateEnvHelp)

gateEnvHelp :: Text
gateEnvHelp =
  "the golden gate's laws need two environment variables: GOLDEN_GATE_JS \
  \(the path to crates/map-viewer/tests/golden.js) and GOLDEN_GATE_HARNESS \
  \(a directory where `require('playwright-core')` resolves). Neither has a \
  \safe default: the gate drives a real browser against the running viewer, \
  \and a law that silently did not run it would be a green tick for no \
  \evidence."

-- R23 (controller ruling): a step definition's answer to "does this line
-- belong to me" is not a yes/no Maybe — it's one of THREE outcomes, and
-- conflating two of them into a single `Just` is exactly what made two
-- genuinely different definitions look "ambiguous" over the same line
-- whenever one of them merely recognized the shape without its capture
-- actually parsing (see Check.hs and Steps.hs's ordering comment for the
-- concrete collisions this dissolves).
data Claim
  = NoMatch
    -- ^ a literal didn't match: this step does not apply to this line at
    -- all (R4's "expected literal"-prefixed case).
  | ClaimError Text
    -- ^ the shape matched (every literal was found) but a capture failed
    -- to PARSE — a bad piece name, a style that isn't a style. This step
    -- recognizes the line but the value in it is bad.
  | Matched (World -> IO StepOutcome)
    -- ^ the pattern matched AND every capture parsed: the runnable
    -- action.

-- The sweep (parameterization): RUNNING a step has THREE outcomes, not
-- two — the same shape `Claim` already has for MATCHING one, and for the
-- same reason. A law with a precondition (a scene with at least one
-- resource to fetch; a non-empty piece set to fold over) meets a drawn
-- binding that cannot exercise it perhaps one iteration in ten. The two-
-- outcome `Either Text World` forces that into either a PASS (a vacuous
-- green: the law didn't run, and a check satisfiable by its own failure
-- mode is no check — MEMORY: verify-distinct-not-nonnull) or a FAILURE
-- (a red for a law that isn't broken, just unexercised). Neither is
-- true. `StepSkipped` is the third, honest answer, and `Run`/`Prop`
-- count them: a law that ran 88 times and skipped 12 says so, and a law
-- that skipped ALL of them is a Failed, because a law that never ran
-- must never report green.
data StepOutcome
  = StepOk World
    -- ^ the step ran and the law held on this iteration.
  | StepFailed Text
    -- ^ the step ran and the law BROKE, with the reason.
  | StepSkipped Text
    -- ^ the step could not run at all on this draw, with the precondition
    -- that went unmet. NOT a pass, NOT a failure — counted separately.

-- The typed shape of "this law needs something the draw may not have
-- given it". Three cases, because there really are three: the world is
-- BROKEN (an unbound scene name, a body with no resources array — a
-- genuine failure that no amount of redrawing fixes), the precondition
-- is merely UNMET on this draw (a legal scene that happens to carry
-- fewer resources than this law needs — skip), or it is MET. Collapsing
-- the first two into one `Left` is exactly how a broken world would come
-- to look like a quiet skip, and how a law would stop being able to
-- fail.
data Precondition a = Broken Text | Unmet Text | Met a
  deriving (Eq, Show)

-- WHAT ONE RUN OF THIS STEP COSTS, in wall-clock seconds — a MEASURED
-- fact about the step, declared where the step is, and the input from
-- which `Prop.iterationsFor` derives how many times a property law may
-- run.
--
-- Almost everything in this corpus is one HTTP call against a server on
-- this machine and rounds to nothing; `Instant` says so, and says it
-- once for the fifty-odd steps it is true of. A step that drives the
-- golden gate launches a browser, boots the map, waits for it to stop
-- moving and samples it twice per view — measured at 48.6 s for a
-- one-stop check run — and a law quantified over that step cannot pay
-- a hundred iterations of it.
--
-- The alternative was a number: a per-law iteration count, written down
-- somewhere, matched to a scenario by name. That is the buried constant
-- the brief forbids, and it goes stale the moment a law gains a step or
-- the machine gets faster. This is the cost itself, stated by the thing
-- that has it, from which the count is computed.
data StepCost = Instant | Seconds Int deriving (Eq, Ord, Show)

costSeconds :: StepCost -> Int
costSeconds Instant     = 0
costSeconds (Seconds s) = s

data StepDef = StepDef
  { defKw     :: Keyword
  , defSketch :: Text
  , defUses   :: [(Text, Universe)]
  , defRun    :: Text -> Claim
  , defCost   :: StepCost
  }

-- The ordinary step: two outcomes, and it CANNOT skip — the lift is
-- total and one-way, so a step written this way can never accidentally
-- report a precondition miss it never reasoned about.
mkStep :: Keyword -> StepP a -> (a -> World -> IO (Either Text World)) -> StepDef
mkStep k p f = mkSkippableStep k p (\a w -> either StepFailed StepOk <$> f a w)

-- The step whose law has a PRECONDITION: it says so in its own type, so
-- "which steps can skip" is answerable by reading their definitions
-- rather than by grepping for a magic string in an error message.
mkSkippableStep :: Keyword -> StepP a -> (a -> World -> IO StepOutcome) -> StepDef
mkSkippableStep k p f = StepDef k (renderP p) (usesOf p) claim Instant
  where
    claim body = case matchP p body of
      Right a -> Matched (f a)
      Left e
        -- a literal mismatch means "not this step" (try the next def);
        -- a CAPTURE failure means "this step, bad value" (report it)
        | "expected literal" `T.isPrefixOf` e -> NoMatch
        | otherwise -> ClaimError e

-- A step that costs real time says so, by wrapping the definition it
-- would otherwise have been. `Instant` is the default because it is
-- true of almost every step here; a step that is not instant has to say
-- what it costs, and says it in the same breath as its pattern.
costing :: Int -> StepDef -> StepDef
costing s d = d { defCost = Seconds s }
