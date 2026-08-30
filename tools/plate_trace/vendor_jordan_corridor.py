# Corridor v2: bridge the Jordan corridor caps into the NE lakes so
# no background channel can sneak between river and lake tips.
import json

import numpy as np
import cv2

TMP = r'C:/Users/donov/.claude/jobs/c6946bce/tmp'
SRC = f'{TMP}/osm_rivers.json'
OUT = r'C:/Users/donov/Documents/the-best-maps-ever/data/osm/rivers.geojson'
LAKES = r'C:/Users/donov/Documents/the-best-maps-ever/data/natural-earth/ne_10m_lakes.geojson'

coef = np.load(f'{TMP}/affine.npy')
A = coef[:2, :].T
b = coef[2, :]
Ainv = np.linalg.inv(A)

def to_px(lon, lat):
    v = Ainv @ (np.array([lon, lat]) - b)
    return float(v[0]), float(v[1])

H, W = 6000, 4500
corridor = np.zeros((H, W), np.uint8)
d = json.load(open(SRC, encoding='utf8'))
ways = [e for e in d.get('elements', []) if e['type'] == 'way' and 'geometry' in e]
jordan = [w for w in ways
          if 'Jordan' in (w.get('tags', {}).get('name:en') or w.get('tags', {}).get('name') or '')]
for w in jordan:
    pts = []
    for g in w['geometry']:
        x, y = to_px(g['lon'], g['lat'])
        pts.append([int(round(x)), int(round(y))])
    if len(pts) >= 2:
        cv2.polylines(corridor, [np.array(pts, np.int32)], False, 255, thickness=7)

# NE lakes (Galilee + Dead Sea) rasterized
lakes = np.zeros((H, W), np.uint8)
lj = json.load(open(LAKES, encoding='utf8'))
for f in lj['features']:
    if f['properties'].get('name') not in ('Sea of Galilee', 'Dead Sea'):
        continue
    g = f['geometry']
    polys = g['coordinates'] if g['type'] == 'MultiPolygon' else [g['coordinates']]
    for poly in polys:
        ring = []
        for lon, lat in [(c[0], c[1]) for c in poly[0]]:
            x, y = to_px(lon, lat)
            ring.append([int(round(x)), int(round(y))])
        if len(ring) >= 3:
            cv2.fillPoly(lakes, [np.array(ring, np.int32)], 255)

# closing across the cap gaps, then remove the lakes themselves
combined = cv2.morphologyEx(
    np.maximum(corridor, lakes), cv2.MORPH_CLOSE,
    cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (17, 17)))
corridor_final = combined & ~lakes

cnts, _ = cv2.findContours(corridor_final, cv2.RETR_EXTERNAL, cv2.CHAIN_APPROX_NONE)
corridors = []
for c in cnts:
    if cv2.contourArea(c) < 200:
        continue
    ap = cv2.approxPolyDP(c.astype(np.float32), 2.0, True).reshape(-1, 2)
    ring = []
    for x, y in ap:
        lon, lat = (A @ np.array([x, y])) + b
        ring.append([round(float(lon), 6), round(float(lat), 6)])
    if len(ring) >= 3:
        corridors.append(ring)
print("corridor rings:", len(corridors), "pts:", sum(len(r) for r in corridors))

gj = json.load(open(OUT, encoding='utf8'))
gj['features'] = [f for f in gj['features'] if not f['properties'].get('corridor')]
for i, ring in enumerate(corridors):
    gj['features'].append({
        "type": "Feature",
        "properties": {"corridor": True, "name": "Jordan corridor", "part": i},
        "geometry": {"type": "Polygon", "coordinates": [ring]},
    })
json.dump(gj, open(OUT, 'w', encoding='utf8'))
print("corridors written")
