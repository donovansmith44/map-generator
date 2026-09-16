module Sphere
  ( Vec3 (..)
  , dot
  , cross
  , normalize
  , angleBetween
  , insideRing
  , insideRings
  , unitOfLatLon
  , Page (..)
  , Globe (..)
  , globeOf
  , placePx
  , unprojectPx
  , pageDistanceToRings
  ) where

import Data.Text (Text)

data Vec3 = Vec3 !Double !Double !Double deriving (Eq, Show)

dot :: Vec3 -> Vec3 -> Double
dot (Vec3 ax ay az) (Vec3 bx by bz) = ax * bx + ay * by + az * bz

cross :: Vec3 -> Vec3 -> Vec3
cross (Vec3 ax ay az) (Vec3 bx by bz) =
  Vec3 (ay * bz - az * by) (az * bx - ax * bz) (ax * by - ay * bx)

normalize :: Vec3 -> Maybe Vec3
normalize v@(Vec3 x y z)
  | n < 1e-9 = Nothing
  | otherwise = Just (Vec3 (x / n) (y / n) (z / n))
  where n = sqrt (dot v v)

angleBetween :: Vec3 -> Vec3 -> Double
angleBetween a b = acos (max (-1) (min 1 (dot a b)))

unitOfLatLon :: Double -> Double -> Vec3
unitOfLatLon lat lon = Vec3 (cos la * cos lo) (cos la * sin lo) (sin la)
  where
    la = lat * pi / 180
    lo = lon * pi / 180

antipode :: Vec3 -> Vec3
antipode (Vec3 x y z) = Vec3 (-x) (-y) (-z)

side :: Vec3 -> Vec3 -> Bool
side n q = dot n q > 0

arcsCross :: Vec3 -> Vec3 -> Vec3 -> Vec3 -> Vec3 -> Bool
arcsCross p r n1 a b
  | side n1 a == side n1 b = False
  | side n2 p == side n2 r = False
  | otherwise = dAb * dPr > 0
  where
    n2 = cross a b
    x = cross n1 n2
    Vec3 ax ay az = a
    Vec3 bx by bz = b
    Vec3 px py pz = p
    Vec3 rx ry rz = r
    dAb = dot x (Vec3 (ax + bx) (ay + by) (az + bz))
    dPr = dot x (Vec3 (px + rx) (py + ry) (pz + rz))

crossingParity :: Vec3 -> Vec3 -> [Vec3] -> Maybe Int -> Bool
crossingParity p r ring skip = foldl step False (zip [0 ..] edges)
  where
    n1 = cross p r
    edges = zip ring (drop 1 ring ++ take 1 ring)
    step odd_ (i, (a, b))
      | skip == Just i = odd_
      | arcsCross p r n1 a b = not odd_
      | otherwise = odd_

ringCap :: [Vec3] -> Maybe (Vec3, Double)
ringCap ring = do
  axis <- normalize (foldr (\(Vec3 x y z) (Vec3 sx sy sz) -> Vec3 (sx + x) (sy + y) (sz + z)) (Vec3 0 0 0) ring)
  let cosR = minimum (1 : map (dot axis) ring)
  pure (axis, max (-1) (min 1 cosR))

coversSphere :: [Vec3] -> Bool
coversSphere pts = length pts <= 5 && or [ dot a b < -0.99 | a <- pts, b <- pts ]

outsideReference :: Vec3 -> Vec3 -> Double -> Vec3
outsideReference p axis@(Vec3 ax ay az) cosR
  | dot p anti > -0.999 = anti
  | otherwise =
      let e = maybe (unitOfLatLon 0 90) id (normalize (Vec3 (-ay) ax 0))
          phi = (acos cosR + pi) / 2
          (c, s) = (cos phi, sin phi)
          Vec3 ex ey ez = e
      in maybe anti id (normalize (Vec3 (ax * c + ex * s) (ay * c + ey * s) (az * c + ez * s)))
  where anti = antipode axis

insideRing :: Vec3 -> [Vec3] -> Bool
insideRing p ring
  | length ring < 3 = False
  | coversSphere ring = True
  | otherwise = case ringCap ring of
      Just (axis, cosR) | cosR > 0 -> crossingParity p (outsideReference p axis cosR) ring Nothing
      _ -> insideRingGeneral p ring

insideRingGeneral :: Vec3 -> [Vec3] -> Bool
insideRingGeneral p ring0
  | n < 3 = False
  | otherwise = (pParity == leftParity) == leftIsInterior
  where
    dedup = foldl (\acc q -> case reverse acc of
                              (l : _) | dot l q >= 1 - 1e-12 -> acc
                              _ -> acc ++ [q]) []
    pts = trimClose (dedup ring0)
    trimClose xs = case xs of
      (h : _) | length xs > 1 && dot h (last xs) >= 1 - 1e-12 -> trimClose (init xs)
      _ -> xs
    n = length pts
    at i = pts !! (i `mod` n)
    turn = sum
      [ atan2 (dot (cross tIn tOut) v) (dot tIn tOut)
      | i <- [0 .. n - 1]
      , let u = at (i - 1); v = at i; w = at (i + 1)
      , Just tIn <- [normalize (scaleSub v (dot u v) u)]
      , Just tOut <- [normalize (scaleSub w 1 (scale v (dot v w)))]
      ]
    scale (Vec3 x y z) k = Vec3 (x * k) (y * k) (z * k)
    scaleSub (Vec3 x y z) k (Vec3 ux uy uz) = Vec3 (x * k - ux) (y * k - uy) (z * k - uz)
    leftArea = 2 * pi - turn
    leftIsInterior = leftArea <= 2 * pi
    (eBest, _) = foldl (\(bi, bc) i -> let c = dot (at i) (at (i + 1)) in if c < bc then (i, c) else (bi, bc))
                       (0, 2) [0 .. n - 1]
    (a, b) = (at eBest, at (eBest + 1))
    m = maybe a id (normalize (add a b))
    add (Vec3 x y z) (Vec3 x' y' z') = Vec3 (x + x') (y + y') (z + z')
    nHat = maybe (Vec3 0 0 1) id (normalize (cross a b))
    candidates = [Vec3 0 0 1, Vec3 0 0 (-1), Vec3 1 0 0, Vec3 0 1 0, Vec3 (-1) 0 0, Vec3 0 (-1) 0]
    r = case filter (\c -> dot p c > -0.999 && dot m c > -0.999 && abs (dot c nHat) > 1e-6) candidates of
          (c : _) -> c
          [] -> Vec3 0 0 1
    leftParity0 = crossingParity m r pts (Just eBest)
    leftParity = if side nHat r then leftParity0 else not leftParity0
    pParity = crossingParity p r pts Nothing

insideRings :: Vec3 -> [[Vec3]] -> Bool
insideRings p = odd . length . filter (insideRing p)

data Page = Page
  { pageChart :: Text
  , pageLat :: Double
  , pageLon :: Double
  , pageZoom :: Double
  , pageWidth :: Double
  , pageHeight :: Double
  } deriving (Eq, Show)

data Globe = Globe
  { globeCenter :: Vec3
  , globeEast :: Vec3
  , globeNorth :: Vec3
  , globeScale :: Double
  , globeCx :: Double
  , globeCy :: Double
  , globeWidth :: Double
  , globeHeight :: Double
  } deriving (Eq, Show)

globeOf :: Page -> Either Text Globe
globeOf pg
  | pageChart pg /= "globe" =
      Left ("the answer was placed on a " <> pageChart pg <> " chart, which this runner does not mirror")
  | otherwise = Right Globe
      { globeCenter = c
      , globeEast = e
      , globeNorth = n
      , globeScale = (pageWidth pg - 32) / 2 / rView
      , globeCx = pageWidth pg / 2
      , globeCy = pageWidth pg / 2
      , globeWidth = pageWidth pg
      , globeHeight = pageHeight pg
      }
  where
    c@(Vec3 cx cy _) = unitOfLatLon (pageLat pg) (pageLon pg)
    e = maybe (unitOfLatLon 0 90) id (normalize (Vec3 (-cy) cx 0))
    n = maybe (unitOfLatLon 90 0) id (normalize (cross c e))
    rView = sin (max 1 (min 90 (pageZoom pg)) * pi / 180)

placePx :: Globe -> Vec3 -> Maybe (Double, Double)
placePx g p
  | dot p (globeCenter g) < 0 = Nothing
  | otherwise = Just (globeCx g + dot p (globeEast g) * globeScale g, globeCy g - dot p (globeNorth g) * globeScale g)

unprojectPx :: Globe -> (Double, Double) -> Maybe Vec3
unprojectPx g (x, y)
  | rr > 1 = Nothing
  | otherwise = normalize (Vec3 (ex * ex' + ny * nx' + f * cx') (ex * ey' + ny * ny' + f * cy') (ex * ez' + ny * nz' + f * cz'))
  where
    ex = (x - globeCx g) / globeScale g
    ny = (globeCy g - y) / globeScale g
    rr = ex * ex + ny * ny
    f = sqrt (max 0 (1 - rr))
    Vec3 ex' ey' ez' = globeEast g
    Vec3 nx' ny' nz' = globeNorth g
    Vec3 cx' cy' cz' = globeCenter g

pageDistanceToRings :: Globe -> (Double, Double) -> [[Vec3]] -> Double
pageDistanceToRings g q rings = minimum (1 / 0 : concatMap ringEdges rings)
  where
    ringEdges ring =
      let px = map (placePx g) ring
      in [ segDist q a b | (Just a, Just b) <- zip px (drop 1 px ++ take 1 px) ]
    segDist (x, y) (ax, ay) (bx, by) =
      let (dx, dy) = (bx - ax, by - ay)
          l2 = dx * dx + dy * dy
          t = if l2 <= 0 then 0 else max 0 (min 1 (((x - ax) * dx + (y - ay) * dy) / l2))
          (cx, cy) = (ax + t * dx, ay + t * dy)
      in sqrt ((x - cx) * (x - cx) + (y - cy) * (y - cy))
