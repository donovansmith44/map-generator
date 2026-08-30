# Trace the green region's outer contour, map pixels -> (lat, lon)
# through the calibrated affine, simplify, verify by drawing the
# polyline back onto the plate, and dump the coordinate list.
import numpy as np
from PIL import Image, ImageDraw
import scipy.ndimage as ndi

IMG = r'C:/Users/donov/Documents/the-best-maps-ever/10 Canaan Before the Conquest of Joshua.png'
TMP = r'C:/Users/donov/.claude/jobs/c6946bce/tmp'
im = np.asarray(Image.open(IMG).convert('RGB')).astype(np.int16)
H, W, _ = im.shape
r, g, b = im[..., 0], im[..., 1], im[..., 2]

# Green fill: g is the top channel, b clearly below g, not cream-bright.
mask = (g >= r - 6) & (g - b > 14) & (b < 190) & (g > 90) & (r < 210)
# clean speckle, close text/river holes a bit
mask = ndi.binary_closing(mask, structure=np.ones((25, 25)))
mask = ndi.binary_opening(mask, structure=np.ones((7, 7)))
lab, n = ndi.label(mask)
sizes = ndi.sum(mask, lab, index=range(1, n + 1))
big = 1 + int(np.argmax(sizes))
region = ndi.binary_fill_holes(lab == big)
# LAP TOWARD WATER: the plate paints a dark shoreline stroke between a
# fill and its water; that strip belongs to neither mask and renders
# as background. Extend the region across it wherever water is near —
# water paints on top, so the lap is invisible and gaps are impossible.
import cv2
blue_m = (b > r + 25) & (b > g + 18) & (b > 130)
def disk(rr):
    return cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (2*rr+1, 2*rr+1))
# WIDE water only (sea, lakes): opening kills river strokes, so the
# lap never chases a river. The lap runs DEEP (~2 km) into the water,
# far beyond the partition's merge tolerance — crossings happen at
# clean lens tips, never as tangles along the shoreline.
wide_m = cv2.morphologyEx(blue_m.astype(np.uint8), cv2.MORPH_OPEN, disk(8)) > 0
lap = (cv2.dilate(region.astype(np.uint8), disk(30)) > 0)     & (cv2.dilate(wide_m.astype(np.uint8), disk(26)) > 0)
region = region | lap
region = ndi.binary_closing(region, structure=np.ones((9, 9)))
print("green component px:", int(sizes.max()), "of", int(mask.sum()))

# outer contour via OpenCV border following
import cv2
cnts, _ = cv2.findContours(region.astype(np.uint8), cv2.RETR_EXTERNAL, cv2.CHAIN_APPROX_NONE)
cnt = max(cnts, key=cv2.contourArea).reshape(-1, 2)   # (x, y)
contour = cnt[:, ::-1]                                # (y, x)
print("boundary px:", len(contour))

# pixel -> (lon, lat)
coef = np.load(f'{TMP}/affine.npy')
P = np.c_[contour[:, 1], contour[:, 0], np.ones(len(contour))]
LL = P @ coef  # (lon, lat)

# Douglas-Peucker in pixel space (uniform metric), eps in pixels.
def dp(pts, eps):
    keep = np.zeros(len(pts), bool)
    keep[0] = keep[-1] = True
    stack = [(0, len(pts) - 1)]
    while stack:
        i, j = stack.pop()
        if j <= i + 1:
            continue
        seg = pts[j] - pts[i]
        L = np.hypot(*seg) + 1e-12
        d = np.abs(np.cross(seg, pts[i + 1:j] - pts[i])) / L
        k = int(np.argmax(d))
        if d[k] > eps:
            m = i + 1 + k
            keep[m] = True
            stack += [(i, m), (m, j)]
    return keep

keep = dp(contour[:, ::-1].astype(float), eps=4.0)   # (x,y), 4 px ~ 300 m
simple = LL[keep]
simple_px = contour[keep]
print("simplified points:", len(simple))

# proof: draw the simplified polyline back on the plate
proof = Image.open(IMG).convert('RGB')
dr = ImageDraw.Draw(proof)
pts = [(int(x), int(y)) for y, x in simple_px]
dr.line(pts + [pts[0]], fill=(255, 0, 60), width=9)
proof.resize((1125, 1500)).save(f'{TMP}/trace_proof.png')

np.save(f'{TMP}/green_lonlat.npy', simple)
print("lat range %.3f..%.3f lon range %.3f..%.3f" % (
    simple[:, 1].min(), simple[:, 1].max(), simple[:, 0].min(), simple[:, 0].max()))
