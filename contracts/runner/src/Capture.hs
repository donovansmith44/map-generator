-- An explicit export list, so the round-trip laws in test/Spec.hs can
-- quantify over the WHOLE type (via Arbitrary) rather than only over
-- values that `parseCap` happened to produce -- which is the set
-- `parseCap` accepts, making the law circular. Constructors are
-- exported deliberately for that reason.
module Capture
  ( Universe (..)
  , FromCapture (..)
  , describeUniverse
  , didYouMean
  , editDistance
  , Piece (..)
  , pieceText
  , PieceSet (..)
  , allPieces
  , Year (..)
  , StyleName (..)
  , styleNames
  , FixtureRef (..)
  ) where

import Data.List (sort, sortOn)
import Data.Proxy (Proxy (..))
import Data.Set (Set)
import qualified Data.Set as Set
import Data.Text (Text)
import qualified Data.Text as T

data Universe = Enumerated [Text] | Ranged Text Text Text | Described Text
  deriving (Eq, Show)

class FromCapture a where
  capName   :: proxy a -> Text
  parseCap  :: Text -> Either Text a
  renderCap :: a -> Text
  universe  :: proxy a -> Universe

-- The one sentence a dummy reads in the Vocabulary table.
describeUniverse :: Universe -> Text
describeUniverse (Enumerated vs)   = "any of: " <> T.intercalate ", " vs
describeUniverse (Ranged lo hi m)  = "whole number from " <> lo <> " to " <> hi <> " (" <> m <> ")"
describeUniverse (Described d)     = d

didYouMean :: [Text] -> Text -> Text
didYouMean vocab w =
  case sortOn (editDistance w) vocab of
    (best : _) | editDistance w best <= 3 -> "  Did you mean: " <> best <> "?"
    _ -> ""

-- Bounded-time edit distance (Final review cleanup, Fix 1): the previous
-- definition was the naive recursive Levenshtein, exponential in input
-- length -- fine for the short garbage a real bad step produces, but a
-- pathological long input (a garbage feature-file value) could hang
-- `check`/`run` outright, and QuickCheck's own size ramp already had to
-- be capped (`resize 8` in Spec.hs) just to keep ONE property test from
-- hitting it. Fixed the algorithm itself, not the caller: this is the
-- textbook Wagner-Fischer dynamic-programming table, built one row per
-- character of `b` via `scanl` (each row reuses the previous row's
-- values, so no cell is ever recomputed) -- O(length a * length b) time,
-- same edit-distance definition (unit insert/delete/substitute cost) as
-- the naive version it replaces, so genuine near-misses score exactly as
-- before.
editDistance :: Text -> Text -> Int
editDistance ta tb = last (foldl transform [0 .. length a] b)
  where
    a = T.unpack ta
    b = T.unpack tb
    -- Every row this produces has length `length a + 1` (never []), but
    -- that's a length invariant GHC can't see -- `head`/`tail` would
    -- compile clean structurally but trip -Wx-partial (this project
    -- builds with -Wall), and a bare `(x:xs')` function-clause pattern
    -- with no `[]` equation trips -Wincomplete-patterns. An exhaustive
    -- case with an unreachable-in-practice `[]` branch satisfies both
    -- warnings honestly, without pretending the empty case is possible.
    transform row c = case row of
      [] -> []
      (x : xs') -> scanl step (x + 1) (zip3 a row xs')
      where
        -- diag (x') carries the substitution cost; above (y, one column
        -- back in the row just finished) carries a plain +1. Getting
        -- these two swapped type-checks fine and still terminates, just
        -- computes the wrong number -- checked against known distances
        -- (e.g. "kitten" -> "sitting" = 3) in the test added alongside
        -- this fix, not just eyeballed.
        step z (ca, diag, above) = minimum [above + 1, z + 1, diag + fromEnum (ca /= c)]

-- ---------- Piece / PieceSet ----------
data Piece = Ground | Water | Fills | Borders | Claims | Labels | Markers
           | Journeys | Chrome | Veil
  deriving (Eq, Ord, Show, Bounded, Enum)

pieceText :: Piece -> Text
pieceText = T.toLower . T.pack . show

instance FromCapture Piece where
  capName _ = "piece"
  -- Sorted, not declaration order: Task 8's expected vocabulary string
  -- is alphabetical ("any of: borders, chrome, claims, fills, ground,
  -- journeys, labels, markers, veil, water"), independent of how the
  -- Piece constructors happen to be declared above.
  universe _ = Enumerated (sort (map pieceText [minBound .. maxBound]))
  renderCap = pieceText
  parseCap t =
    let vs = [ (pieceText p, p) | p <- [minBound .. maxBound] ]
    in case lookup (T.strip t) vs of
         Just p -> Right p
         Nothing ->
           Left $ "'" <> T.strip t <> "' is not a piece."
                <> didYouMean (map fst vs) (T.strip t)
                <> "\n  Pieces are: " <> T.intercalate ", " (sort (map fst vs))

newtype PieceSet = PieceSet (Set Piece) deriving (Eq, Show)

allPieces :: PieceSet
allPieces = PieceSet (Set.fromList [minBound .. maxBound])

instance FromCapture PieceSet where
  capName _ = "pieces"
  universe _ = universe (Proxy @Piece)
  -- R6: the empty set is the monoid identity (Task 9 generates it as a
  -- subset) and must round-trip. "" isn't renderable as a piece list
  -- that parses back (T.splitOn "," "" == [""], which fails as a piece),
  -- so the empty set gets its own token, "none", which parseCap accepts
  -- on the way back in (along with a whitespace-only string).
  -- `Set.toAscList` orders by Piece's derived Ord, which is constructor
  -- declaration order, not alphabetical -- the same trap `universe`
  -- (above, on Piece) had to be fixed for. Sort the *rendered text*
  -- explicitly so there is one canonical "sorted" order in this module,
  -- not two.
  renderCap (PieceSet s)
    | Set.null s = "none"
    | otherwise  = T.intercalate ", " (sort (map pieceText (Set.toList s)))
  parseCap t
    | T.strip t == "none" || T.null (T.strip t) = Right (PieceSet Set.empty)
    | otherwise = PieceSet . Set.fromList <$> traverse parseCap (T.splitOn "," t)

-- ---------- Year ----------
newtype Year = Year Int deriving (Eq, Ord, Show)

-- Final-review Fix 3: year 0 does not exist in this calendar (1 BC is
-- immediately followed by AD 1) -- a domain law, put in the type rather
-- than left for the server to reject at run time. Before this fix,
-- Year's universe silently ADMITTED 0 (both the Ranged description below
-- and Prop.genYear's generator), so a drawn 0 made the (non-@target)
-- subjects/census determinism properties hard-red for a reason that has
-- nothing to do with either law -- a random, toolchain-timing-dependent
-- false failure, not a real one. See MEMORY: types-over-tricks -- the
-- fix belongs in parseCap/the generator/the universe description
-- together, not as a special-cased runtime check bolted onto one call
-- site.
instance FromCapture Year where
  capName _ = "year"
  universe _ = Ranged "-4004" "100" "negative means BC; -1405 is 1405 BC; year 0 does not exist"
  renderCap (Year y) = T.pack (show y)
  parseCap t = case reads (T.unpack (T.strip t)) of
    [(0, "")] -> Left $ "year 0 does not exist in this calendar (1 BC is followed by AD 1). "
                             <> describeUniverse (universe (Proxy @Year))
    [(y, "")] | y >= (-4004) && y <= 100 -> Right (Year y)
    [(y, "")] -> Left $ "year " <> T.pack (show y) <> " is outside the frame. "
                      <> describeUniverse (universe (Proxy @Year))
    _ -> Left $ "'" <> T.strip t <> "' is not a year. "
              <> describeUniverse (universe (Proxy @Year))

-- ---------- StyleName ----------
newtype StyleName = StyleName Text deriving (Eq, Show)

styleNames :: [Text]
styleNames = ["canaan", "parchment", "slate"]

instance FromCapture StyleName where
  capName _ = "style"
  universe _ = Enumerated styleNames
  renderCap (StyleName s) = s
  parseCap t
    | T.strip t `elem` styleNames = Right (StyleName (T.strip t))
    | otherwise = Left $ "'" <> T.strip t <> "' is not a style."
                       <> didYouMean styleNames (T.strip t)
                       <> "\n  Styles are: " <> T.intercalate ", " styleNames

-- ---------- FixtureRef ----------
newtype FixtureRef = FixtureRef Text deriving (Eq, Show)

instance FromCapture FixtureRef where
  capName _ = "fixture"
  universe _ = Described "a named answer in fixtures/, e.g. \"tribes-at-1405\""
  renderCap (FixtureRef f) = "\"" <> f <> "\""
  parseCap t =
    let s = T.strip t
    in if "\"" `T.isPrefixOf` s && "\"" `T.isSuffixOf` s && T.length s >= 2
         then Right (FixtureRef (T.dropEnd 1 (T.drop 1 s)))
         else Left "a fixture reference is quoted, e.g. \"tribes-at-1405\""
