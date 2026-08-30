# Vendor the dry land within the working frame: Natural Earth 10m
# land rasterized through the plate chart — the SAME derivation as
# med_clip.geojson, so land and sea share one coastline by
# construction. Components above a small floor survive (the mainland
# and Cyprus); specks do not.
import json

import numpy as np
import cv2

TMP = r'C:/Users/donov/.claude/jobs/c6946bce/tmp'
LAND = r'C:/Users/donov/Documents/the-best-maps-ever/data/natural-earth/ne_10m_land.geojson'
OUT = r'C:/Users/donov/Documents/the-best-maps-ever/data/natural-earth/land_clip.geojson'

coef = np.load(f'{TMP}/affine.npy')
A = coef[:2, :].T
b = coef[2, :]
Ainv = np.linalg.inv(A)


def to_px(lon, lat):
    v = Ainv @ (np.array([lon, lat]) - b)
    return float(v[0]), float(v[1])


H, W = 6000, 4500
land = np.zeros((H, W), np.uint8)
lj = json.load(open(LAND, encoding='utf8'))
for f in lj['features']:
    g = f['geometry']
    polys = g['coordinates'] if g['type'] == 'MultiPolygon' else [g['coordinates']]
    for poly in polys:
        outer = poly[0]
        lons = [c[0] for c in outer]
        lats = [c[1] for c in outer]
        if max(lons) < 30 or min(lons) > 40 or max(lats) < 27 or min(lats) > 37:
            continue
        ring = [[int(round(x)), int(round(y))] for x, y in (to_px(c[0], c[1]) for c in outer)]
        if len(ring) >= 3:
            cv2.fillPoly(land, [np.array(ring, np.int32)], 255)
        for hole in poly[1:]:
            hring = [[int(round(x)), int(round(y))] for x, y in (to_px(c[0], c[1]) for c in hole)]
            if len(hring) >= 3:
                cv2.fillPoly(land, [np.array(hring, np.int32)], 0)

n, lab, stats, _ = cv2.connectedComponentsWithStats(land)
features = []
for i in range(1, n):
    if stats[i, cv2.CC_STAT_AREA] < 20000:  # ~ a few hundred km^2
        continue
    cm = (lab == i).astype(np.uint8)
    cnts, _ = cv2.findContours(cm, cv2.RETR_EXTERNAL, cv2.CHAIN_APPROX_NONE)
    cnt = max(cnts, key=cv2.contourArea).reshape(-1, 2).astype(np.float32)
    ap = cv2.approxPolyDP(cnt.reshape(-1, 1, 2), 2.0, True).reshape(-1, 2)
    ring = []
    for x, y in ap:
        lon, lat = (A @ np.array([float(x), float(y)])) + b
        ring.append([round(float(lon), 6), round(float(lat), 6)])
    if len(ring) >= 3:
        features.append({
            'type': 'Feature',
            'properties': {'component_px': int(stats[i, cv2.CC_STAT_AREA])},
            'geometry': {'type': 'Polygon', 'coordinates': [ring]},
        })
    print(f'component {i}: {stats[i, cv2.CC_STAT_AREA]} px -> {len(ring)} pts')

json.dump({
    'type': 'FeatureCollection',
    'note': 'The dry land within the working frame: Natural Earth 10m land '
            'rasterized through the plate chart — the same derivation as '
            'med_clip.geojson, so land and sea share one coastline by '
            'construction.',
    'features': features,
}, open(OUT, 'w', encoding='utf8'))
print('land_clip.geojson written:', len(features), 'components')
