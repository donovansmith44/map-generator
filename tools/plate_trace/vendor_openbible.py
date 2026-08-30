# Vendor the attested neighbor regions from OpenBible.info's
# Bible-Geocoding-Data (CC BY 4.0): Philistia, Phoenicia, Geshur,
# Ammon, Moab, Edom. OpenBible publishes each region as nine nested
# confidence isobands (10%..90%); the middle band (50%) is the
# outline. Borders facing a tribe, the sea, or a lake snap+splice onto
# those rings so the shared line exists once — the budget is the
# isoband spacing itself, measured and printed, never invented.
import io
import json
import math
import sys

sys.stdout.reconfigure(errors='replace', encoding='ascii')
TMP = r'C:/Users/donov/.claude/jobs/c6946bce/tmp'
REPO = r'C:/Users/donov/Documents/the-best-maps-ever'
OUT = f'{REPO}/data/openbible/regions.geojson'

REGIONS = [
    # slug, display name, OpenBible geometry id (geometry/<id>.geojson)
    ('philistia', 'Philistia', 'ac71e65'),
    ('phoenicia', 'Phoenicia', 'a33d53e'),
    ('geshur', 'Geshur', 'af6c325'),
    ('ammon', 'Ammon', 'a6046fa'),
    ('moab', 'Moab', 'aa0b1d6'),
    ('edom', 'Edom', 'a2735ff'),
]

# ---- snap targets: the tribal rings, the real sea, the real lakes
targets = []
tj = json.load(open(f'{REPO}/data/wikimedia/tribes12.geojson', encoding='utf8'))
for f in tj['features']:
    targets.append([(c[0], c[1]) for c in f['geometry']['coordinates'][0]])
mj = json.load(open(f'{REPO}/data/natural-earth/med_clip.geojson', encoding='utf8'))
targets.append([(c[0], c[1]) for c in mj['features'][0]['geometry']['coordinates'][0]])
lj = json.load(open(f'{REPO}/data/natural-earth/ne_10m_lakes.geojson', encoding='utf8'))
for f in lj['features']:
    if f['properties'].get('name') not in ('Sea of Galilee', 'Dead Sea'):
        continue
    g = f['geometry']
    polys = g['coordinates'] if g['type'] == 'MultiPolygon' else [g['coordinates']]
    for poly in polys:
        targets.append([(c[0], c[1]) for c in poly[0]])


def seg_project(px, py, ax, ay, bx, by):
    dx, dy = bx - ax, by - ay
    L2 = dx * dx + dy * dy
    if L2 == 0:
        return ax, ay, 0.0
    tt = max(0.0, min(1.0, ((px - ax) * dx + (py - ay) * dy) / L2))
    return ax + tt * dx, ay + tt * dy, tt


def snap_splice(ll, budget_deg):
    n_on = 0
    snapped = []
    for lon, lat in ll:
        best = None
        for ti, ring in enumerate(targets):
            m = len(ring)
            for s in range(m):
                ax, ay = ring[s]
                bx, by = ring[(s + 1) % m]
                qx, qy, tt = seg_project(lon, lat, ax, ay, bx, by)
                d = math.hypot(lon - qx, lat - qy)
                if best is None or d < best[0]:
                    best = (d, ti, s + tt, qx, qy)
        if best and best[0] <= budget_deg:
            d, ti, s, qx, qy = best
            snapped.append((ti, s, qx, qy))
            n_on += 1
        else:
            snapped.append((None, None, lon, lat))
    out = []
    n = len(snapped)
    for i in range(n):
        ti, s, x, y = snapped[i]
        out.append((x, y))
        tj_, s2, _, _ = snapped[(i + 1) % n]
        if ti is not None and ti == tj_:
            ring = targets[ti]
            m = len(ring)
            fwd = (s2 - s) % m
            back = (s - s2) % m
            span = fwd if fwd <= back else -back
            # generous cap: isoband rings are coarse, tribal rings are
            # dense — a long shared stretch may cross many vertices
            if 0 < abs(span) <= 150:
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
    return clean, n_on


features = []
for slug, name, gid in REGIONS:
    d = json.load(open(f'{TMP}/ob_{gid}.geojson', encoding='utf8'))
    iso = next(f for f in d['features'] if f['properties'].get('role') == 'isobands')
    bands = iso['geometry']['coordinates']
    mid = bands[(len(bands) - 1) // 2]
    ring = [(c[0], c[1]) for c in mid[0]]
    # the budget IS the isoband spacing: mean gap between the middle
    # band and its neighbor, measured on this region's own data
    lo = bands[(len(bands) - 1) // 2 - 1][0]
    gaps = []
    for lon, lat in ring[:: max(1, len(ring) // 24)]:
        g = 1e9
        for i in range(len(lo)):
            qx, qy, _ = seg_project(lon, lat, lo[i][0], lo[i][1],
                                    lo[(i + 1) % len(lo)][0], lo[(i + 1) % len(lo)][1])
            g = min(g, math.hypot(lon - qx, lat - qy))
        gaps.append(g)
    # witness-disagreement allowance: this region's own max isoband
    # spacing plus the tribal map's measured max georef error (6.3 km)
    budget = max(gaps) + 6.3 / 111.0
    snapped, n_on = snap_splice(ring, budget)
    print(f'{slug:10s} band pts {len(ring)} -> {len(snapped)}; '
          f'budget {budget * 111:.1f} km (max band spacing + tribal 6.3); '
          f'snapped vertices {n_on}/{len(ring)}')
    features.append({
        'type': 'Feature',
        'properties': {'region': slug, 'name': name, 'openbible_id': gid},
        'geometry': {'type': 'Polygon',
                     'coordinates': [[[round(x, 5), round(y, 5)] for x, y in snapped]]},
    })

doc = {
    'type': 'FeatureCollection',
    'note': ('Attested regions from OpenBible.info Bible-Geocoding-Data '
             '(https://github.com/openbibleinfo/Bible-Geocoding-Data, CC BY 4.0): '
             'the 50% confidence isoband of each region, with borders facing a '
             'tribe, the sea, or a lake spliced onto those rings (budget = the '
             "region's max isoband spacing + the tribal map's 6.3 km max error)."),
    'license': 'CC BY 4.0',
    'source': 'https://github.com/openbibleinfo/Bible-Geocoding-Data',
    'features': features,
}
import os
os.makedirs(os.path.dirname(OUT), exist_ok=True)
json.dump(doc, io.open(OUT, 'w', encoding='utf8'))
print('written:', OUT)
