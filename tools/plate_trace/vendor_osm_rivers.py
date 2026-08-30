# Vendor OSM rivers: Overpass ways -> data/osm/rivers.geojson.
# Ways of one river share exact node ids, so junction connectivity is
# in the data; we keep endpoint coordinates exact and only simplify
# interiors. Networks (connected components) below a length floor are
# dropped as ditches.
import json
import math

SRC = r'C:/Users/donov/.claude/jobs/c6946bce/tmp/osm_rivers.json'
OUT = r'C:/Users/donov/Documents/the-best-maps-ever/data/osm/rivers.geojson'

d = json.load(open(SRC, encoding='utf8'))
ways = [e for e in d.get('elements', []) if e['type'] == 'way' and 'geometry' in e]

def length_km(pts):
    s = 0.0
    for (a, b) in zip(pts, pts[1:]):
        la1, lo1, la2, lo2 = map(math.radians, (a[0], a[1], b[0], b[1]))
        s += 6371.0 * math.acos(max(-1, min(1,
            math.sin(la1)*math.sin(la2) + math.cos(la1)*math.cos(la2)*math.cos(lo2-lo1))))
    return s

# connected components over shared node ids
parent = {}
def find(x):
    while parent.setdefault(x, x) != x:
        parent[x] = parent[parent[x]]
        x = parent[x]
    return x
def union(a, b):
    ra, rb = find(a), find(b)
    if ra != rb:
        parent[max(ra, rb)] = min(ra, rb)

for w in ways:
    nodes = w.get('nodes', [])
    for n in nodes[1:]:
        union(nodes[0], n)

comp_len = {}
for w in ways:
    pts = [(g['lat'], g['lon']) for g in w['geometry']]
    c = find(w['nodes'][0]) if w.get('nodes') else id(w)
    comp_len[c] = comp_len.get(c, 0.0) + length_km(pts)

def dp(pts, eps_deg):
    # endpoints always kept
    keep = [False] * len(pts)
    keep[0] = keep[-1] = True
    stack = [(0, len(pts) - 1)]
    while stack:
        i, j = stack.pop()
        if j <= i + 1:
            continue
        ax, ay = pts[i][1], pts[i][0]
        bx, by = pts[j][1], pts[j][0]
        best, bk = -1.0, None
        for k in range(i + 1, j):
            px, py = pts[k][1], pts[k][0]
            dx, dy = bx - ax, by - ay
            L = math.hypot(dx, dy) or 1e-12
            dist = abs(dy * px - dx * py + bx * ay - by * ax) / L
            if dist > best:
                best, bk = dist, k
        if best > eps_deg:
            keep[bk] = True
            stack += [(i, bk), (bk, j)]
    return [p for p, k in zip(pts, keep) if k]

MIN_NETWORK_KM = 12.0
feats = []
kept_names = set()
for w in ways:
    c = find(w['nodes'][0]) if w.get('nodes') else id(w)
    if comp_len.get(c, 0.0) < MIN_NETWORK_KM:
        continue
    pts = [(g['lat'], g['lon']) for g in w['geometry']]
    if len(pts) < 2:
        continue
    pts = dp(pts, 0.0012)  # ~120 m
    nm = w.get('tags', {}).get('name:en') or w.get('tags', {}).get('name') or ''
    if nm:
        kept_names.add(nm)
    feats.append({
        "type": "Feature",
        "properties": {"name": nm, "network": str(c), "osm_way": w['id']},
        "geometry": {
            "type": "LineString",
            "coordinates": [[round(lon, 6), round(lat, 6)] for lat, lon in pts],
        },
    })

out = {
    "type": "FeatureCollection",
    "note": "OpenStreetMap waterway=river, bbox (29.0,33.5)-(34.6,37.8). "
            "(c) OpenStreetMap contributors, ODbL 1.0 - see LICENSE.md",
    "features": feats,
}
import os
os.makedirs(os.path.dirname(OUT), exist_ok=True)
json.dump(out, open(OUT, 'w', encoding='utf8'))
total = sum(length_km([(la, lo) for lo, la in f['geometry']['coordinates']]) for f in feats)
print(f"{len(feats)} ways kept, {len(kept_names)} named rivers, {total:.0f} km total")
open(os.path.join(os.path.dirname(OUT), 'LICENSE.md'), 'w', encoding='utf8').write(
    "Rivers derived from OpenStreetMap (waterway=river).\n"
    "(c) OpenStreetMap contributors, licensed ODbL 1.0.\n"
    "https://www.openstreetmap.org/copyright\n")
