module World where

import Data.Aeson (Value, eitherDecodeStrict)
import Data.ByteString (ByteString)
import qualified Data.ByteString.Lazy as BL
import Data.Map.Strict (Map)
import qualified Data.Map.Strict as Map
import Data.Text (Text)
import qualified Data.Text as T
import Gherkin.Ast (Keyword)
import Capture (Universe)
import Pattern
import Network.HTTP.Client

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
      <> " }"

httpTransport :: Manager -> Text -> IO (Either Text (ByteString, Value))
httpTransport mgr url = do
  req <- parseRequest (T.unpack url)
  resp <- httpLbs req mgr
  let raw = BL.toStrict (responseBody resp)
  pure $ case eitherDecodeStrict raw of
    Right v -> Right (raw, v)
    Left e  -> Left (T.pack e <> " for " <> url)

-- Same request as `httpTransport`, minus the JSON decode -- for the two
-- steps (Task 11) that fetch /api/resource and /api/resources, whose
-- bodies are binary geometry, not JSON.
httpTransportRaw :: Manager -> Text -> IO (Either Text ByteString)
httpTransportRaw mgr url = do
  req <- parseRequest (T.unpack url)
  resp <- httpLbs req mgr
  pure (Right (BL.toStrict (responseBody resp)))

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
  | Matched (World -> IO (Either Text World))
    -- ^ the pattern matched AND every capture parsed: the runnable
    -- action.

data StepDef = StepDef
  { defKw     :: Keyword
  , defSketch :: Text
  , defUses   :: [(Text, Universe)]
  , defRun    :: Text -> Claim
  }

mkStep :: Keyword -> StepP a -> (a -> World -> IO (Either Text World)) -> StepDef
mkStep k p f = StepDef k (renderP p) (usesOf p) $ \body ->
  case matchP p body of
    Right a -> Matched (f a)
    Left e
      -- a literal mismatch means "not this step" (try the next def);
      -- a CAPTURE failure means "this step, bad value" (report it)
      | "expected literal" `T.isPrefixOf` e -> NoMatch
      | otherwise -> ClaimError e
