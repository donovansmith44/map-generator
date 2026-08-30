# Warp the plate into our render frame and blend, proving the traced
# circuit lands where the plate draws it.
# Chain: plate px --(calibration affine)--> lon/lat --(fitted)--> screen px.
import re
import numpy as np
import cv2
from PIL import Image

TMP = r'C:/Users/donov/.claude/jobs/c6946bce/tmp'
S = r'C:/Users/donov/AppData/Local/Temp/claude/C--Users-donov-Documents-the-best-maps-ever/04071d76-0bf3-41fb-826a-3607dd4d7ef0/scratchpad'
PLATE = r'C:/Users/donov/Documents/the-best-maps-ever/10 Canaan Before the Conquest of Joshua.png'

LL = np.load(f'{TMP}/green_lonlat.npy')            # traced (lon, lat), N=205
calib = np.load(f'{TMP}/affine.npy')               # [x y 1] -> (lon, lat)

# our render, exact pixels
svg = open(f'{S}/canaan_traced.svg', encoding='utf8').read()
# the traced region: the data-region path whose vertex count matches
# the 205-point contour
best = None
for pm in re.finditer(r'<path[^>]*data-region[^>]* d="([^"]+)"', svg):
    pts = re.findall(r'(-?[\d.]+) (-?[\d.]+)', pm.group(1))
    if 195 <= len(pts) <= 215:
        best = pts
if best is None:
    raise SystemExit("no matching path")
scr = np.array([(float(x), float(y)) for x, y in best])
print("screen polygon pts:", len(scr))

# correspond by role: extreme vertices are projection-order invariant
def extremes(P, flipy):
    # returns [westmost, eastmost, northmost, southmost, NE-most, SW-most]
    x, y = P[:, 0], P[:, 1]
    n = (-y if flipy else y)
    return np.array([
        P[np.argmin(x)], P[np.argmax(x)],
        P[np.argmax(n)], P[np.argmin(n)],
        P[np.argmax(x + n)], P[np.argmin(x + n)],
    ])

src = extremes(LL, flipy=False)          # lon/lat, north = +lat
dst = extremes(scr, flipy=True)          # screen, north = -y
A = np.c_[src, np.ones(6)]
fit, res, *_ = np.linalg.lstsq(A, dst, rcond=None)  # lonlat -> screen
pred = A @ fit
err = np.hypot(*(pred - dst).T)
print("lonlat->screen fit residuals (px):", np.round(err, 1))

# total affine: plate px -> screen px  (2x3 for cv2)
# lonlat = [px py 1] @ calib ; screen = [lon lat 1] @ fit
C = np.zeros((3, 3)); C[:, :2] = calib; C[2, 2] = 1.0; C[:2, 2] = 0; C[2, :2] = calib[2, :]
# careful: calib is (3,2) mapping [x y 1]->[lon lat]; build 3x3 homogeneous
Ch = np.eye(3)
Ch[:2, :] = calib.T          # rows: lon = a.x+b.y+c ; lat = ...
Fh = np.eye(3)
Fh[:2, :] = fit.T            # rows: sx = ..lon,lat,1 ; sy = ..
T = Fh @ Ch                  # plate px (homog) -> screen
M = T[:2, :]
plate = cv2.imread(PLATE)
warped = cv2.warpAffine(plate, M.astype(np.float64), (1400, 1400), flags=cv2.INTER_AREA, borderValue=(205, 227, 237))

ours = cv2.imread(f'{S}/shots/canaan_traced_1400.png')
blend = cv2.addWeighted(warped, 0.55, ours, 0.45, 0)
cv2.imwrite(f'{S}/shots/overlay_proof.png', blend)
# also save the warped plate alone for side-by-side
cv2.imwrite(f'{S}/shots/plate_warped.png', warped)
print("overlay written")
