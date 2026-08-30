# Vendor the real Mediterranean: NE land rasterized through the
# chart; the sea is the frame minus land, largest component; traced
# and emitted as a closed ring. Same family as the NE lakes, so
# rivers, lakes, and coast all agree by construction.
import json

import numpy as np
import cv2

TMP = r'C:/Users/donov/.claude/jobs/c6946bce/tmp'
LAND = r'C:/Users/donov/Documents/the-best-maps-ever/data/natural-earth/ne_10m_land.geojson'
OUT = r'C:/Users/donov/Documents/the-best-maps-ever/data/natural-earth/med_clip.geojson'

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
        # cheap bbox reject in lon/lat before projecting
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

sea = ((land == 0).astype(np.uint8)) * 255
lab_n, lab = cv2.connectedComponents(sea)
sizes = [(int((lab == i).sum()), i) for i in range(1, lab_n)]
sizes.sort(reverse=True)
med = (lab == sizes[0][1]).astype(np.uint8)
print("sea components:", lab_n - 1, "med px:", sizes[0][0])

cnts, _ = cv2.findContours(med, cv2.RETR_EXTERNAL, cv2.CHAIN_APPROX_NONE)
cnt = max(cnts, key=cv2.contourArea).reshape(-1, 2).astype(np.float32)
ap = cv2.approxPolyDP(cnt.reshape(-1, 1, 2), 2.0, True).reshape(-1, 2)
ring = []
for x, y in ap:
    lon, lat = (A @ np.array([float(x), float(y)])) + b
    ring.append([round(float(lon), 6), round(float(lat), 6)])
print("med ring points:", len(ring))

json.dump({
    "type": "FeatureCollection",
    "note": "The Mediterranean within the working frame: Natural Earth 10m land, "
            "complemented and clipped through the plate chart. Same witness family "
            "as ne_10m_lakes, so coast, lakes, and rivers agree by construction.",
    "features": [{
        "type": "Feature",
        "properties": {"name": "the Great Sea", "source": "ne_10m_land complement"},
        "geometry": {"type": "Polygon", "coordinates": [ring]},
    }],
}, open(OUT, 'w', encoding='utf8'))
print("med_clip.geojson written")
