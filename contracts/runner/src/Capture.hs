module Capture where

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
  case sortOn (dist w) vocab of
    (best : _) | dist w best <= 3 -> "  Did you mean: " <> best <> "?"
    _ -> ""
  where
    dist a b = lev (T.unpack a) (T.unpack b)
    lev [] bs = length bs
    lev as [] = length as
    lev (a:as) (b:bs)
      | a == b = lev as bs
      | otherwise = 1 + minimum [lev as (b:bs), lev (a:as) bs, lev as bs]

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
  renderCap (PieceSet s)
    | Set.null s = "none"
    | otherwise  = T.intercalate ", " (map pieceText (Set.toAscList s))
  parseCap t
    | T.strip t == "none" || T.null (T.strip t) = Right (PieceSet Set.empty)
    | otherwise = PieceSet . Set.fromList <$> traverse parseCap (T.splitOn "," t)

-- ---------- Year ----------
newtype Year = Year Int deriving (Eq, Ord, Show)

instance FromCapture Year where
  capName _ = "year"
  universe _ = Ranged "-4004" "100" "negative means BC; -1405 is 1405 BC"
  renderCap (Year y) = T.pack (show y)
  parseCap t = case reads (T.unpack (T.strip t)) of
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
