module Pattern where

import Data.Proxy (Proxy (..))
import Data.Text (Text)
import qualified Data.Text as T
import Capture

-- A step pattern is an applicative over (remaining text -> result).
-- capUntil names its terminator, so matching is deterministic and
-- ambiguity (adjacent untyped captures) is unrepresentable by
-- construction — the terminator IS the next literal.
data StepP a = StepP
  { runP  :: Text -> Either Text (a, Text)
  , uses  :: [(Text, Universe)]
  , sketch :: [Text]
  }

instance Functor StepP where
  fmap f (StepP r u s) = StepP (\t -> fmap (\(a, rest) -> (f a, rest)) (r t)) u s

instance Applicative StepP where
  pure a = StepP (\t -> Right (a, t)) [] []
  StepP rf uf sf <*> StepP ra ua sa = StepP
    (\t -> do (f, t') <- rf t; (a, t'') <- ra t'; pure (f a, t''))
    (uf ++ ua) (sf ++ sa)

lit :: Text -> StepP ()
lit l = StepP
  (\t -> case T.stripPrefix l t of
      Just rest -> Right ((), rest)
      Nothing   -> Left ("expected literal '" <> l <> "' at: " <> T.take 40 t))
  [] [l]

-- R4 (controller ruling): the terminator IS the next literal, so a
-- terminator-not-found here is a LITERAL-class mismatch, not a capture
-- failure — its error MUST start with the exact prefix "expected literal",
-- matching `lit`'s own mismatch. Task 5's mkStep falls through to the next
-- step definition on that prefix; a capture PARSE failure (from parseCap)
-- must NOT carry it, since that case should be reported, not skipped.
capUntil :: forall a. FromCapture a => Text -> StepP a
capUntil terminator = StepP
  (\t -> case T.breakOn terminator t of
      (raw, rest) | terminator `T.isPrefixOf` rest -> do
        v <- parseCap raw
        pure (v, T.drop (T.length terminator) rest)
      _ -> Left ("expected literal '" <> terminator <> "' after {" <> capName (Proxy @a) <> "}"))
  [(capName (Proxy @a), universe (Proxy @a))]
  ["{" <> capName (Proxy @a) <> "}", terminator]

capRest :: forall a. FromCapture a => StepP a
capRest = StepP
  (\t -> do v <- parseCap t; pure (v, ""))
  [(capName (Proxy @a), universe (Proxy @a))]
  ["{" <> capName (Proxy @a) <> "}"]

matchP :: StepP a -> Text -> Either Text a
matchP p t = do
  (a, rest) <- runP p t
  if T.null (T.strip rest) then Right a
  else Left ("unmatched trailing text: " <> rest)

usesOf :: StepP a -> [(Text, Universe)]
usesOf = uses

renderP :: StepP a -> Text
renderP = T.concat . sketch
