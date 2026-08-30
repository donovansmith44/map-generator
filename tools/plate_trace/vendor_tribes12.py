# Vendor the tribal allotments from the Wikimedia Commons
# "12 Tribes of Israel Map.svg" (CC BY-SA 3.0).
#
# The SVG is georeferenced through its own city dot markers: an affine
# px -> (lon, lat) fitted on 19 identified tells (mean 2.3 km, max
# 6.3 km — svg_georef.py in the job tmp). Each tribe is a distinct
# fill color; regions are read from a painter's-order raster so
# overlaps resolve exactly as drawn. Manasseh spans both banks of the
# Jordan in this map; it is split mechanically by our real Jordan
# corridor (data/osm/rivers.geojson), never by hand.
#
# Shared borders exist ONCE: each ring snap+splices onto previously
# accepted rings, so two tribes' common border is literally the same
# polyline — knife-edge parallels cannot form between tribes.
import io
import json
import math
import sys

import numpy as np
import cv2

sys.stdout.reconfigure(errors='replace', encoding='ascii')
TMP = r'C:/Users/donov/.claude/jobs/c6946bce/tmp'
REPO = r'C:/Users/donov/Documents/the-best-maps-ever'
OUT = f'{REPO}/data/wikimedia/tribes12.geojson'

img = cv2.imread(f'{TMP}/tribes12_raster.png')
H, W = img.shape[:2]
tribe_color = json.load(open(f'{TMP}/tribe_colors.json'))
coef = np.load(f'{TMP}/svg_affine.npy')
cl, ct = coef[0], coef[1]  # lon = cl @ [x,y,1]; lat = ct @ [x,y,1]

# inverse affine (lon,lat) -> px
Afwd = np.array([[cl[0], cl[1]], [ct[0], ct[1]]])
bfwd = np.array([cl[2], ct[2]])
Ainv = np.linalg.inv(Afwd)


def to_px(lon, lat):
    v = Ainv @ (np.array([lon, lat]) - bfwd)
    return float(v[0]), float(v[1])


def to_ll(x, y):
    return float(cl @ [x, y, 1.0]), float(ct @ [x, y, 1.0])


# ---- the real Jordan corridor, rasterized into SVG px space
corridor = np.zeros((H, W), np.uint8)
rj = json.load(open(f'{REPO}/data/osm/rivers.geojson', encoding='utf8'))
for f in rj['features']:
    if not f['properties'].get('corridor'):
        continue
    ring = [[int(round(x)), int(round(y))]
            for x, y in (to_px(c[0], c[1]) for c in f['geometry']['coordinates'][0])]
    cv2.fillPoly(corridor, [np.array(ring, np.int32)], 255)
corridor = cv2.dilate(corridor, np.ones((9, 9), np.uint8))
print('corridor px:', int((corridor > 0).sum()))

# the real Mediterranean ring (shoreline identity target) and the SVG
# water mask whose adjacency DECLARES a border to be shoreline
mj = json.load(open(f'{REPO}/data/natural-earth/med_clip.geojson', encoding='utf8'))
med_ring = [(c[0], c[1]) for c in mj['features'][0]['geometry']['coordinates'][0]]
water = ((img[:, :, 0] == 0xFF) & (img[:, :, 1] == 0xEC) & (img[:, :, 2] == 0xC6)).astype(np.uint8)
n, lab, stats, _ = cv2.connectedComponentsWithStats(water)
big = 1 + int(np.argmax(stats[1:, cv2.CC_STAT_AREA]))  # the Mediterranean

# ---- tribe masks; split Manasseh at the corridor
units = {}  # slug -> (mask, parent)
for tribe, (r, g, b) in tribe_color.items():
    mask = ((img[:, :, 2] == r) & (img[:, :, 1] == g) & (img[:, :, 0] == b)).astype(np.uint8)
    mask = cv2.morphologyEx(mask, cv2.MORPH_CLOSE, np.ones((5, 5), np.uint8))
    slug = tribe.lower()
    if slug == 'manasseh':
        halves = mask & ~(corridor > 0)
        n, lab, stats, cent = cv2.connectedComponentsWithStats(halves)
        comps = sorted(((stats[i, cv2.CC_STAT_AREA], i) for i in range(1, n)), reverse=True)
        keep = [(a, i) for a, i in comps if a >= 4000]
        assert len(keep) == 2, f'manasseh split into {len(keep)} pieces: {keep}'
        (a0, i0), (a1, i1) = keep
        west, east = ((i0, i1) if cent[i0][0] < cent[i1][0] else (i1, i0))
        units['manasseh-west'] = ((lab == west).astype(np.uint8), None)
        units['manasseh-east'] = ((lab == east).astype(np.uint8), None)
    elif slug == 'simeon':
        units[slug] = (mask, 'judah')  # measured: 100% enclosed by Judah
    else:
        # tribes stand on their own data: the allotments legitimately
        # exceed the pre-conquest canaan plate (Asher to Sidon, Judah
        # to Kadesh), so no canaan parent — the partition's smaller-
        # witness-names-the-face law resolves the overlap instead.
        units[slug] = (mask, None)

# ---- water adjacency: a ring vertex next to the SVG's own sea or a
# lake IS that shoreline by the map's declaration — it adopts the real
# water ring by identity, with no distance tolerance to tune.
lakes_by_class = {'sea': [med_ring]}
lj = json.load(open(f'{REPO}/data/natural-earth/ne_10m_lakes.geojson', encoding='utf8'))
for f in lj['features']:
    nm = f['properties'].get('name')
    if nm not in ('Sea of Galilee', 'Dead Sea'):
        continue
    key = 'galilee' if nm == 'Sea of Galilee' else 'dead'
    g = f['geometry']
    polys = g['coordinates'] if g['type'] == 'MultiPolygon' else [g['coordinates']]
    for poly in polys:
        lakes_by_class.setdefault(key, []).append([(c[0], c[1]) for c in poly[0]])

# classify SVG water components by centroid latitude
water_class = np.zeros((H, W), np.uint8)  # 0 none, 1 sea, 2 galilee, 3 dead
CLASS_ID = {'sea': 1, 'galilee': 2, 'dead': 3}
nw, wlab, wstats, wcent = cv2.connectedComponentsWithStats(water)
for i in range(1, nw):
    if wstats[i, cv2.CC_STAT_AREA] < 800:
        continue
    _, clat = to_ll(float(wcent[i][0]), float(wcent[i][1]))
    if i == big:
        water_class[wlab == i] = 1
    elif 32.6 <= clat <= 33.0:
        water_class[wlab == i] = 2
    elif 31.0 <= clat <= 31.8:
        water_class[wlab == i] = 3

ADJ = 8  # px: a fill/stroke gap this thin still means "on the shore"


def vertex_class(x, y):
    x0, x1 = max(0, int(x) - ADJ), min(W, int(x) + ADJ + 1)
    y0, y1 = max(0, int(y) - ADJ), min(H, int(y) + ADJ + 1)
    win = water_class[y0:y1, x0:x1]
    if win.max() == 0 or int(x) <= ADJ:
        return None  # frame-edge contact is not coast
    vals, counts = np.unique(win[win > 0], return_counts=True)
    return {v: k for k, v in CLASS_ID.items()}[int(vals[np.argmax(counts)])]


# ---- trace, transform, snap+splice shared borders
def snap_splice(ll, targets, budget_deg, classes=None, class_targets=None):
    def seg_project(px, py, ax, ay, bx, by):
        dx, dy = bx - ax, by - ay
        L2 = dx * dx + dy * dy
        if L2 == 0:
            return ax, ay, 0.0
        tt = max(0.0, min(1.0, ((px - ax) * dx + (py - ay) * dy) / L2))
        return ax + tt * dx, ay + tt * dy, tt

    # one combined target pool: budget-snapped peers first, then the
    # water rings (reached only through a vertex's declared class)
    class_targets = class_targets or {}
    pool = list(targets)
    class_range = {}
    for key, rings in class_targets.items():
        class_range[key] = (len(pool), len(pool) + len(rings))
        pool.extend(rings)

    def project_onto(lon, lat, lo, hi):
        best = None
        for ti in range(lo, hi):
            ring = pool[ti]
            m = len(ring)
            for s in range(m):
                ax, ay = ring[s]
                bx, by = ring[(s + 1) % m]
                qx, qy, tt = seg_project(lon, lat, ax, ay, bx, by)
                d = math.hypot(lon - qx, lat - qy)
                if best is None or d < best[0]:
                    best = (d, ti, s + tt, qx, qy)
        return best

    snapped = []
    for i, (lon, lat) in enumerate(ll):
        cls = classes[i] if classes else None
        if cls in class_range:
            lo, hi = class_range[cls]
            d, ti, s, qx, qy = project_onto(lon, lat, lo, hi)
            snapped.append((ti, s, qx, qy))  # by identity: no budget
            continue
        best = project_onto(lon, lat, 0, len(targets)) if targets else None
        if best and best[0] <= budget_deg:
            d, ti, s, qx, qy = best
            snapped.append((ti, s, qx, qy))
        else:
            snapped.append((None, None, lon, lat))
    out = []
    n = len(snapped)
    for i in range(n):
        ti, s, x, y = snapped[i]
        out.append((x, y))
        tj, s2, _, _ = snapped[(i + 1) % n]
        if ti is not None and ti == tj:
            ring = pool[ti]
            m = len(ring)
            fwd = (s2 - s) % m
            back = (s - s2) % m
            span = fwd if fwd <= back else -back
            if 0 < abs(span) <= 60:
                step = 1 if span > 0 else -1
                k = math.floor(s) + 1 if step > 0 else math.ceil(s) - 1
                while (k - s) * step > 0 and (k - s) * step < abs(span):
                    out.append(tuple(ring[int(k) % m]))
                    k += step
    clean = []
    for pt in out:
        if not clean or math.hypot(clean[-1][0] - pt[0], clean[-1][1] - pt[1]) > 1e-9:
            clean.append(pt)
    while len(clean) > 1 and math.hypot(clean[0][0] - clean[-1][0], clean[0][1] - clean[-1][1]) <= 1e-9:
        clean.pop()
    return clean


ORDER = ['judah', 'simeon', 'benjamin', 'ephraim', 'manasseh-west', 'dan',
         'issachar', 'zebulun', 'asher', 'naphtali',
         'reuben', 'gad', 'manasseh-east']
accepted = []  # rings already emitted; later rings adopt their vertices
features = []
for slug in ORDER:
    mask, parent = units[slug]
    cnts, _ = cv2.findContours(mask, cv2.RETR_EXTERNAL, cv2.CHAIN_APPROX_NONE)
    cnt = max(cnts, key=cv2.contourArea)
    ap = cv2.approxPolyDP(cnt, 1.0, True).reshape(-1, 2)
    ring = [to_ll(float(x), float(y)) for x, y in ap]
    classes = [vertex_class(float(x), float(y)) for x, y in ap]
    n_shore = sum(1 for c in classes if c)
    ring = snap_splice(ring, accepted, 0.0035,  # ~350 m: 1px offset + DP scale
                       classes=classes, class_targets=lakes_by_class)
    assert len(ring) >= 3, slug
    accepted.append(ring)
    features.append({
        'type': 'Feature',
        'properties': {'tribe': slug, 'parent': parent},
        'geometry': {'type': 'Polygon',
                     'coordinates': [[[round(x, 5), round(y, 5)] for x, y in ring]]},
    })
    print(f'{slug:14s} {len(ring)} pts, parent={parent}, shore vertices {n_shore}')

doc = {
    'type': 'FeatureCollection',
    'note': ('Tribal allotments traced from Wikimedia Commons "12 Tribes of '
             'Israel Map.svg", georeferenced through the map\'s own city '
             'markers against known tell coordinates (mean 2.3 km, max 6.3 km '
             'over 19 cities). Manasseh is split at the real Jordan corridor. '
             'Shared borders are spliced to a single polyline.'),
    'license': 'CC BY-SA 3.0',
    'source': 'https://commons.wikimedia.org/wiki/File:12_Tribes_of_Israel_Map.svg',
    'features': features,
}
import os
os.makedirs(os.path.dirname(OUT), exist_ok=True)
json.dump(doc, io.open(OUT, 'w', encoding='utf8'))
print('written:', OUT, 'features:', len(features),
      'vertices:', sum(len(f['geometry']['coordinates'][0]) for f in features))
