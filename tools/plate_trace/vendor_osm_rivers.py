# Vendor OSM rivers v2: rivers plus ONLY the stream chains that
# bridge two river networks (e.g. the course through the Mujib
# reservoir, tagged stream, that reunites the ancient Arnon). A
# tributary stream that merely joins one network stays out — the map
# renders rivers, not every wadi.
import json
import math
from collections import deque

SRC = r'C:/Users/donov/.claude/jobs/c6946bce/tmp/osm_rivers2.json'
OUT = r'C:/Users/donov/Documents/the-best-maps-ever/data/osm/rivers.geojson'

d = json.load(open(SRC, encoding='utf8'))
ways = [e for e in d.get('elements', []) if e['type'] == 'way' and 'geometry' in e and 'nodes' in e]
kind = {w['id']: w.get('tags', {}).get('waterway') for w in ways}
by_id = {w['id']: w for w in ways}

# node -> way ids
node_ways = {}
for w in ways:
    for n in w['nodes']:
        node_ways.setdefault(n, []).append(w['id'])

# river components over shared nodes
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
rivers = [w for w in ways if kind[w['id']] == 'river']
for w in rivers:
    for n in w['nodes'][1:]:
        union((w['nodes'][0], 'n'), (n, 'n'))
def comp_of_way(w):
    return find((w['nodes'][0], 'n'))

# BFS through stream ways from river-way sources; a meeting of two
# different river components marks the bridging chain.
label = {}   # way id -> river component
parent_way = {}
dist = {}
q = deque()
for w in rivers:
    label[w['id']] = comp_of_way(w)
    dist[w['id']] = 0
    q.append(w['id'])
bridge_ways = set()
pairs_done = set()
def chain(wid):
    out = []
    while wid is not None and kind[wid] == 'stream':
        out.append(wid)
        wid = parent_way.get(wid)
    return out
MAX_D = 12  # a bridge is a short chain, not a parallel river system
while q:
    u = q.popleft()
    if dist[u] >= MAX_D:
        continue
    for n in by_id[u]['nodes']:
        for v in node_ways.get(n, []):
            if v == u:
                continue
            if v in label:
                if label[v] != label[u]:
                    pair = (min(label[u], label[v]), max(label[u], label[v]))
                    if pair not in pairs_done:
                        pairs_done.add(pair)
                        bridge_ways.update(chain(u))
                        bridge_ways.update(chain(v))
                continue
            if kind[v] != 'stream':
                continue
            label[v] = label[u]
            parent_way[v] = u if kind[u] == 'stream' else None
            if kind[u] == 'river':
                parent_way[v] = None
            dist[v] = dist[u] + 1
            q.append(v)

# wait: chain() needs the stream itself even when parented to a river
# — parent None ends the chain, and v itself is included by chain(v).
print(f"river ways: {len(rivers)}, bridge stream ways: {len(bridge_ways)}, pairs joined: {len(pairs_done)}")

kept = rivers + [by_id[w] for w in sorted(bridge_ways)]

# networks over the KEPT set
parent.clear()
for w in kept:
    for n in w['nodes'][1:]:
        union((w['nodes'][0], 'k'), (n, 'k'))
def knet(w):
    return find((w['nodes'][0], 'k'))

def length_km(pts):
    s = 0.0
    for (a, b) in zip(pts, pts[1:]):
        la1, lo1, la2, lo2 = map(math.radians, (a[0], a[1], b[0], b[1]))
        s += 6371.0 * math.acos(max(-1, min(1,
            math.sin(la1)*math.sin(la2) + math.cos(la1)*math.cos(la2)*math.cos(lo2-lo1))))
    return s

net_len = {}
for w in kept:
    pts = [(g['lat'], g['lon']) for g in w['geometry']]
    c = knet(w)
    net_len[c] = net_len.get(c, 0.0) + length_km(pts)

def dp(pts, eps_deg):
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
            distl = abs(dy * px - dx * py + bx * ay - by * ax) / L
            if distl > best:
                best, bk = distl, k
        if best > eps_deg:
            keep[bk] = True
            stack += [(i, bk), (bk, j)]
    return [p for p, k in zip(pts, keep) if k]

MIN_NETWORK_KM = 12.0
feats = []
for w in kept:
    c = knet(w)
    if net_len.get(c, 0.0) < MIN_NETWORK_KM:
        continue
    pts = [(g['lat'], g['lon']) for g in w['geometry']]
    if len(pts) < 2:
        continue
    pts = dp(pts, 0.0012)
    nm = w.get('tags', {}).get('name:en') or w.get('tags', {}).get('name') or ''
    feats.append({
        "type": "Feature",
        "properties": {"name": nm, "network": str(c), "osm_way": w['id'],
                       "bridge": w['id'] in bridge_ways},
        "geometry": {
            "type": "LineString",
            "coordinates": [[round(lon, 6), round(lat, 6)] for lat, lon in pts],
        },
    })

# keep the existing corridor polygons
old = json.load(open(OUT, encoding='utf8'))
corridors = [f for f in old['features'] if f['properties'].get('corridor')]
out = {
    "type": "FeatureCollection",
    "note": "OpenStreetMap waterway=river plus bridge streams (chains reuniting "
            "river networks split by modern dams), bbox (29.0,33.5)-(34.6,37.8). "
            "(c) OpenStreetMap contributors, ODbL 1.0 - see LICENSE.md",
    "features": feats + corridors,
}
json.dump(out, open(OUT, 'w', encoding='utf8'))
total = sum(length_km([(la, lo) for lo, la in f['geometry']['coordinates']]) for f in feats)
print(f"{len(feats)} ways kept ({sum(1 for f in feats if f['properties']['bridge'])} bridges), {total:.0f} km")
