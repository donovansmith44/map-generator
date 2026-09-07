module World where

import Data.Aeson (Value, eitherDecodeStrict)
import Data.ByteString (ByteString)
import qualified Data.ByteString as BS
import qualified Data.ByteString.Lazy as BL
import Data.Map.Strict (Map)
import qualified Data.Map.Strict as Map
import Data.Text (Text)
import qualified Data.Text as T
import qualified Data.Text.Encoding as TE
import Gherkin.Ast (Keyword)
import Capture (StyleName, Universe, Year)
import Pattern
import Network.HTTP.Client
import Network.HTTP.Types.Status (statusCode)

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
  }

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

data StepDef = StepDef
  { defKw     :: Keyword
  , defSketch :: Text
  , defUses   :: [(Text, Universe)]
  , defRun    :: Text -> Claim
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
mkSkippableStep k p f = StepDef k (renderP p) (usesOf p) $ \body ->
  case matchP p body of
    Right a -> Matched (f a)
    Left e
      -- a literal mismatch means "not this step" (try the next def);
      -- a CAPTURE failure means "this step, bad value" (report it)
      | "expected literal" `T.isPrefixOf` e -> NoMatch
      | otherwise -> ClaimError e
