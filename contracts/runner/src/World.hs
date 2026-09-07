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
  { baseUrl    :: Text
  , transport  :: Text -> IO (Either Text (ByteString, Value))
  , fixtureDir :: FilePath
  , bound      :: Map Text (ByteString, Value)
  , blessMode  :: Bool
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

data StepDef = StepDef
  { defKw     :: Keyword
  , defSketch :: Text
  , defUses   :: [(Text, Universe)]
  , defRun    :: Text -> Maybe (World -> IO (Either Text World))
  }

mkStep :: Keyword -> StepP a -> (a -> World -> IO (Either Text World)) -> StepDef
mkStep k p f = StepDef k (renderP p) (usesOf p) $ \body ->
  case matchP p body of
    Right a -> Just (f a)
    Left e
      -- a literal mismatch means "not this step" (try the next def);
      -- a CAPTURE failure means "this step, bad value" (report it)
      | "expected literal" `T.isPrefixOf` e -> Nothing
      | otherwise -> Just (\_ -> pure (Left e))
