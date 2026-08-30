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
# BRIDGE TOWARD REAL WATER: the region must overlap every water body
# it borders so classification clips it back to the water's own line.
# The water here is the REAL water — NE lakes, the OSM Jordan corridor,
# and the plate sea — rasterized into the plate frame through the
# chart. A symmetric dilate-intersect bridges every region-to-water
# gap, wrapping lake tips with no directionality and no chords.
import cv2
import json
import numpy as np2

def disk(rr):
    return cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (2*rr+1, 2*rr+1))

coefA = np.load(r'C:/Users/donov/.claude/jobs/c6946bce/tmp/affine.npy')
_A = coefA[:2, :].T
_b = coefA[2, :]
_Ainv = np.linalg.inv(_A)
def to_px(lon, lat):
    v = _Ainv @ (np.array([lon, lat]) - _b)
    return float(v[0]), float(v[1])

# real lakes
lakes_m = np.zeros((H, W), np.uint8)
lj = json.load(open(r'C:/Users/donov/Documents/the-best-maps-ever/data/natural-earth/ne_10m_lakes.geojson', encoding='utf8'))
for f in lj['features']:
    if f['properties'].get('name') not in ('Sea of Galilee', 'Dead Sea'):
        continue
    geom = f['geometry']
    polys = geom['coordinates'] if geom['type'] == 'MultiPolygon' else [geom['coordinates']]
    for poly in polys:
        ring = [[int(round(x)), int(round(y))] for x, y in
                (to_px(c[0], c[1]) for c in poly[0])]
        if len(ring) >= 3:
            cv2.fillPoly(lakes_m, [np.array(ring, np.int32)], 255)

# jordan corridor (vendored polygons)
corr_m = np.zeros((H, W), np.uint8)
rj = json.load(open(r'C:/Users/donov/Documents/the-best-maps-ever/data/osm/rivers.geojson', encoding='utf8'))
for f in rj['features']:
    if not f['properties'].get('corridor'):
        continue
    ring = [[int(round(x)), int(round(y))] for x, y in
            (to_px(c[0], c[1]) for c in f['geometry']['coordinates'][0])]
    if len(ring) >= 3:
        cv2.fillPoly(corr_m, [np.array(ring, np.int32)], 255)

# plate sea (wide blue only: opening kills the drawn rivers)
blue_m = (b > r + 25) & (b > g + 18) & (b > 130)
sea_m_all = cv2.morphologyEx(blue_m.astype(np.uint8) * 255, cv2.MORPH_OPEN, disk(8))
# the SEA is the largest wide-water component only; the plate's
# stylized lakes are NOT real water — they carry the plate's intent
# and are attributed below, not treated as geography
sl, sc = ndi.label(sea_m_all > 0)
if sc > 0:
    sizes_s = ndi.sum(sea_m_all > 0, sl, index=range(1, sc + 1))
    sea_m = ((sl == (1 + int(np.argmax(sizes_s)))) * 255).astype(np.uint8)
else:
    sea_m = sea_m_all

# drawn rivers (networks >= 30 km, the same set the map renders) are
# water too: the border wadis south of the Dead Sea must attract the
# region exactly like the Jordan does
rivers_m = np.zeros((H, W), np.uint8)
net_len = {}
def _len_km(cs):
    s = 0.0
    import math as _m
    for (lo1, la1), (lo2, la2) in zip(cs, cs[1:]):
        a1, b1, a2, b2 = map(_m.radians, (la1, lo1, la2, lo2))
        s += 6371.0 * _m.acos(max(-1, min(1,
            _m.sin(a1)*_m.sin(a2) + _m.cos(a1)*_m.cos(a2)*_m.cos(b2-b1))))
    return s
for f in rj['features']:
    if f['properties'].get('corridor'):
        continue
    cs = [(c[0], c[1]) for c in f['geometry']['coordinates']]
    net = f['properties'].get('network', '')
    net_len[net] = net_len.get(net, 0.0) + _len_km(cs)
for f in rj['features']:
    if f['properties'].get('corridor'):
        continue
    if net_len.get(f['properties'].get('network', ''), 0.0) < 30.0:
        continue
    pts = [[int(round(x)), int(round(y))] for x, y in
           (to_px(c[0], c[1]) for c in f['geometry']['coordinates'])]
    if len(pts) >= 2:
        cv2.polylines(rivers_m, [np.array(pts, np.int32)], False, 255, thickness=7)
water_real = np.maximum(np.maximum(np.maximum(lakes_m, corr_m), sea_m), rivers_m) > 0
# THE PLATE'S INTENT: where the plate drew water that reality moved
# (its stylized Dead Sea vs the real basins), the difference area
# belongs to whichever side the plate's drawing adjoins. Real water
# (corridor, wadis) severs the difference into per-side components,
# so component-touches-region attribution is exact — no distances.
plate_wide = sea_m_all > 0
strip = plate_wide & ~water_real
slab, sn = ndi.label(strip)
touch_ids = set(np.unique(slab[(cv2.dilate(region.astype(np.uint8), disk(2)) > 0) & (slab > 0)]))
keep_strip = np.isin(slab, [i for i in touch_ids if i > 0])
print("strip px:", int(strip.sum()), "components:", sn, "kept px:", int(keep_strip.sum()))
region = region | keep_strip
region_u8 = region.astype(np.uint8) * 255
bridge = (cv2.dilate(region_u8, disk(40)) > 0) & (cv2.dilate(water_real.astype(np.uint8) * 255, disk(40)) > 0)
# the bridge may not cross a water body: keep the in-water part, and
# of the dry part keep only what touches the region without crossing
# water (the far bank stays the far bank)
# Only BOUNDING water severs (sea, lakes, corridor): a tributary is
# interior water, and treating it as a wall dropped every wedge at a
# confluence, leaving holes that turned the corridor annex into a
# thin finger the simplifier then collapsed.
barriers = (np.maximum(np.maximum(lakes_m, corr_m), sea_m) > 0)
conn = bridge & ~barriers
dlab, dn = ndi.label(conn)
touch = set(np.unique(dlab[region & (dlab > 0)]))
keep_conn = np.isin(dlab, [i for i in touch if i > 0])
region = region | (bridge & water_real) | keep_conn
region = ndi.binary_closing(region, structure=np.ones((9, 9)))
region = ndi.binary_fill_holes(region)
print("green component px:", int(sizes.max()), "of", int(mask.sum()))

ov = int((region & (corr_m > 0)).sum())
print("corridor px:", int((corr_m > 0).sum()), "covered by region:", ov)
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
