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
  , Center (..)
  , centerLat
  , centerLon
  , latClamp
  , Zoom (..)
  , zoomMin
  , zoomMax
  , zoomClamp
  , zoomDoubled
  , zoomHalved
  , DetailTier (..)
  , detailTierNames
  , lodFloor
  , lodCeiling
  , autoLod
  , canonicalWidth
  , canonicalFineZoom
  , tierLod
  , ScaleQual (..)
  , applyScale
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
  -- R78 (controller ruling): `all` is the DUAL of `none`, and it is an
  -- input alias on exactly the same terms. The Rust side already has
  -- both (crates/map-types/src/piece.rs:103-110: `if s == "none" {
  -- PieceSet::empty() }`, `if s == "all" { PieceSet::all() }`); this
  -- side had only `none`, so every corpus line reading `pieces all`
  -- came back BAD-VALUE ("'all' is not a piece. Did you mean: fills?")
  -- against a server that would have accepted it. That was a genuine
  -- gap between the two implementations of ONE vocabulary, not a
  -- missing step.
  --
  -- Input alias only: `renderCap` is unchanged and still renders the
  -- whole set as its ten sorted names, so the round-trip law
  -- (parseCap . renderCap == Right) is untouched -- `all` is a value
  -- parseCap accepts, never one renderCap produces, exactly as `none`
  -- is for the empty set's OTHER spelling ("" / whitespace). The
  -- universe is likewise unchanged, for the same reason it never
  -- mentioned `none`: `universe (Proxy @PieceSet)` answers "what is a
  -- piece", and neither `all` nor `none` is a piece -- they are
  -- shorthands for a SET of them. (Mirrors Rust exactly: `Piece::ALL`
  -- is what its error message lists too, with neither alias in it.)
  parseCap t
    | T.strip t == "none" || T.null (T.strip t) = Right (PieceSet Set.empty)
    | T.strip t == "all" = Right allPieces
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

-- ---------- Center: where the camera looks ----------
-- A geographic point in degrees, written the way the wire writes it and
-- the way camera.feature writes it: "lat,lon" (e.g. "31.5,35.0").
--
-- The frame is the SERVER'S OWN, not a number chosen here: `build_query`
-- (crates/map-viewer/src/lib.rs:645) clamps latitude to +/-89.9 before
-- building the viewport cap, so +/-89.9 is the whole of what a latitude
-- can mean to this API -- a request at 89.95 and one at 89.9 are the
-- same camera. Longitude has no clamp and wraps, so the half-open
-- (-180, 180] is the full circle written exactly once.
--
-- Described, not Enumerated or Ranged: a lat/lon pair is neither a
-- finite answer set nor a whole-number window, so `describeUniverse`'s
-- Ranged sentence ("whole number from ... to ...") would be a false
-- statement about the type. It therefore contributes no Vocabulary row,
-- on the same structural grounds as UrlPath and BindName.
data Center = Center Double Double deriving (Eq, Ord, Show)

centerLat, centerLon :: Center -> Double
centerLat (Center la _) = la
centerLon (Center _ lo) = lo

-- The server's own latitude clamp, transcribed once (lib.rs:645) and
-- used by both the type's legality check and the visibility predicates
-- that must agree with the server about where the camera actually is.
latClamp :: Double -> Double
latClamp = max (-89.9) . min 89.9

instance FromCapture Center where
  capName _ = "center"
  universe _ = Described
    "a lat,lon point in degrees, e.g. 31.5,35.0 (latitude -89.9 to 89.9 \
    \-- the server's own clamp; longitude -180 to 180)"
  renderCap (Center la lo) = T.pack (show la) <> "," <> T.pack (show lo)
  parseCap t = case T.splitOn "," (T.strip t) of
    [a, b] -> do
      la <- readDouble "latitude" a
      lo <- readDouble "longitude" b
      if la < (-89.9) || la > 89.9
        then Left ("latitude " <> a <> " is outside the frame: "
                   <> describeUniverse (universe (Proxy @Center)))
        -- Fix round 1, finding 14: half-open, as the comment above
        -- declares it. -180 and 180 are ONE meridian, and admitting
        -- both spellings would make them two distinct `Center` values
        -- to `distinctCenterLaw` -- a pair that is "distinct" while
        -- naming the same camera. Unreachable from the generator
        -- today; refused by the type so it stays unreachable.
        else if lo <= (-180) || lo > 180
          then Left ("longitude " <> b <> " is outside the frame: "
                     <> describeUniverse (universe (Proxy @Center)))
          else Right (Center la lo)
    _ -> Left ("'" <> T.strip t <> "' is not a center: "
               <> describeUniverse (universe (Proxy @Center)))

-- One place a decimal is read, so every numeric capture below reports the
-- same shape of error and none of them re-derives "is this a number".
readDouble :: Text -> Text -> Either Text Double
readDouble what raw = case reads (T.unpack (T.strip raw)) of
  [(d, "")] -> Right d
  _         -> Left ("'" <> T.strip raw <> "' is not a " <> what)

-- ---------- Zoom: how wide the camera looks ----------
-- The angular RADIUS of the view in degrees. The frame is again the
-- server's own: `build_query` clamps zoom to [0.05, 90] before turning
-- it into the viewport cap (lib.rs:646), and the characterization
-- confirmed the clamp empirically from both ends (zoom 0.001/0.01/0.05
-- byte-identical; 89.9/90/180/1000 byte-identical). Outside that window
-- a zoom is not a different camera, it is the same camera spelled
-- misleadingly -- so the type refuses it rather than letting a law
-- silently compare a scene with itself.
newtype Zoom = Zoom Double deriving (Eq, Ord, Show)

zoomMin, zoomMax :: Double
zoomMin = 0.05
zoomMax = 90

zoomClamp :: Double -> Double
zoomClamp = max zoomMin . min zoomMax

-- The two derived forms camera.feature asks for by name ("zoom <someZoom>
-- doubled", "... halved"). Derived through the SERVER'S clamp, not around
-- it: doubling 60 gives 90, not 120, because 120 and 90 are the same
-- camera. That is exactly why the someZoom hole draws from the
-- doubling-safe sub-window (Prop.genZoom) -- outside it, "doubled" and
-- "halved" would silently be the same camera as the base and the two
-- nesting laws would be vacuous.
zoomDoubled, zoomHalved :: Zoom -> Zoom
zoomDoubled (Zoom z) = Zoom (zoomClamp (z * 2))
zoomHalved  (Zoom z) = Zoom (zoomClamp (z / 2))

instance FromCapture Zoom where
  capName _ = "zoom"
  universe _ = Described
    "an angular view radius in degrees from 0.05 to 90 (the server's own \
    \clamp), e.g. 4"
  renderCap (Zoom z) = T.pack (show z)
  parseCap t = do
    z <- readDouble "zoom" t
    if z < zoomMin || z > zoomMax
      then Left ("zoom " <> T.strip t <> " is outside the frame: "
                 <> describeUniverse (universe (Proxy @Zoom)))
      else Right (Zoom z)

-- ---------- ScaleQual: the two derived cameras, spoken ----------
-- Enumerated, unlike Center and Zoom: "which scale words exist" IS a
-- small closed vocabulary a reader must learn before writing a camera
-- scenario, exactly like the piece and style lists. So it earns its
-- Vocabulary row.
data ScaleQual = Doubled | Halved deriving (Eq, Ord, Show, Bounded, Enum)

applyScale :: ScaleQual -> Zoom -> Zoom
applyScale Doubled = zoomDoubled
applyScale Halved  = zoomHalved

scaleText :: ScaleQual -> Text
scaleText Doubled = "doubled"
scaleText Halved  = "halved"

instance FromCapture ScaleQual where
  capName _ = "scale"
  universe _ = Enumerated (sort (map scaleText [minBound .. maxBound]))
  renderCap = scaleText
  parseCap t =
    let vs = [ (scaleText s, s) | s <- [minBound .. maxBound] ]
    in case lookup (T.strip t) vs of
         Just s  -> Right s
         Nothing -> Left ("'" <> T.strip t <> "' is not a scale word."
                          <> didYouMean (map fst vs) (T.strip t)
                          <> "\n  Scale words are: "
                          <> T.intercalate ", " (sort (map fst vs)))

-- ---------- DetailTier: how much geometry ----------
-- Three tiers a person actually uses, and their lod numbers are DERIVED
-- from the server's own auto rule -- never chosen to make a scenario
-- pass.
--
-- The rule (crates/map-viewer/src/lib.rs:581-586, `auto_lod` itself --
-- lib.rs:605-616 is where it is CALLED, and where the width default
-- lives; cited separately at `canonicalWidth` below). Verified
-- byte-identical against explicit `lod=` at three zooms in the
-- characterization's section 0):
--
--     lod(zoom, width) = clamp(radians(zoom / width), 1e-6, 0.01)
--
-- `autoLod` below IS that rule. The three tiers are the only three
-- distinguished points the rule has:
--
--   ultra  = the rule's FLOOR (1e-6). The finest tolerance the auto law
--            can ever ask for; leaning all the way in. (Characterization
--            section 2.0 labels 1e-6 "auto floor".)
--   fine   = the rule's value at the canonical working camera,
--            autoLod 8 1200 = radians(8/1200) = 1.1636e-4. (Section 2.0
--            labels this rung "fine"; it is the working map.)
--   coarse = the rule's CEILING (0.01). The coarsest tolerance the auto
--            law can ever ask for, and -- per section 2.0's table -- the
--            exact minimum of the vertex curve; the world at a glance.
--
-- Note what is deliberately NOT a tier: `lod = 6.0`, /api/transition's
-- default, which is 3.3x HEAVIER than the ceiling and collapses every
-- morph to two points. Calling it "coarse" would be a lie about the
-- curve's shape (characterization D7's trap, stated in its own words).
data DetailTier = Coarse | Fine | Ultra deriving (Eq, Ord, Show, Bounded, Enum)

detailTierNames :: [Text]
detailTierNames = sort (map tierText [minBound .. maxBound])

tierText :: DetailTier -> Text
tierText = T.toLower . T.pack . show

lodFloor, lodCeiling :: Double
lodFloor = 1e-6
lodCeiling = 1e-2

-- The page width the auto rule defaults to when none is given
-- (lib.rs:611: `.unwrap_or(1200.0)`), and the zoom at which the
-- characterization's own table names the middle tier. Both are the
-- server's/characterization's numbers, cited, not tuned here.
canonicalWidth, canonicalFineZoom :: Double
canonicalWidth = 1200
canonicalFineZoom = 8

autoLod :: Double -> Double -> Double
autoLod zoom width = max lodFloor (min lodCeiling ((zoom / width) * pi / 180))

tierLod :: DetailTier -> Double
tierLod Ultra  = lodFloor
tierLod Fine   = autoLod canonicalFineZoom canonicalWidth
tierLod Coarse = lodCeiling

instance FromCapture DetailTier where
  capName _ = "detail"
  universe _ = Enumerated detailTierNames
  renderCap = tierText
  parseCap t =
    let vs = [ (tierText d, d) | d <- [minBound .. maxBound] ]
    in case lookup (T.strip t) vs of
         Just d  -> Right d
         Nothing -> Left ("'" <> T.strip t <> "' is not a detail tier."
                          <> didYouMean (map fst vs) (T.strip t)
                          <> "\n  Detail tiers are: "
                          <> T.intercalate ", " detailTierNames)
