# Emit the traced Canaan contour as a circuit constant + SurveySpec.
import io
import numpy as np

SRC = r'C:\Users\donov\Documents\the-best-maps-ever\crates\map-adapters\src\surveys.rs'
LL = np.load(r'C:/Users/donov/.claude/jobs/c6946bce/tmp/green_lonlat.npy')  # (lon, lat)


# ---- adopt the water rings' own vertices along shared stretches ----
import json as _json
import math as _math

def _load_targets():
    targets = []  # list of rings [(lon, lat), ...]
    rj = _json.load(open(r'C:/Users/donov/Documents/the-best-maps-ever/data/osm/rivers.geojson', encoding='utf8'))
    for f in rj['features']:
        if f['properties'].get('corridor'):
            targets.append([(c[0], c[1]) for c in f['geometry']['coordinates'][0]])
    lj = _json.load(open(r'C:/Users/donov/Documents/the-best-maps-ever/data/natural-earth/ne_10m_lakes.geojson', encoding='utf8'))
    for f in lj['features']:
        if f['properties'].get('name') not in ('Sea of Galilee', 'Dead Sea'):
            continue
        g = f['geometry']
        polys = g['coordinates'] if g['type'] == 'MultiPolygon' else [g['coordinates']]
        for poly in polys:
            targets.append([(c[0], c[1]) for c in poly[0]])
    return targets

def _snap_splice(ll, targets, budget_deg):
    # nearest point on any target ring; then splice target vertices
    # between consecutive same-target snaps (shorter way around)
    def seg_project(px, py, ax, ay, bx, by):
        dx, dy = bx - ax, by - ay
        L2 = dx * dx + dy * dy
        if L2 == 0:
            return ax, ay, 0.0
        tt = max(0.0, min(1.0, ((px - ax) * dx + (py - ay) * dy) / L2))
        return ax + tt * dx, ay + tt * dy, tt
    snapped = []
    for lon, lat in ll:
        best = None
        for ti, ring in enumerate(targets):
            m = len(ring)
            for s in range(m):
                ax, ay = ring[s]
                bx, by = ring[(s + 1) % m]
                qx, qy, tt = seg_project(lon, lat, ax, ay, bx, by)
                d = _math.hypot(lon - qx, (lat - qy))
                if best is None or d < best[0]:
                    best = (d, ti, s + tt, qx, qy)
        d, ti, s, qx, qy = best
        if d <= budget_deg:
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
            ring = targets[ti]
            m = len(ring)
            fwd = (s2 - s) % m
            back = (s - s2) % m
            span = fwd if fwd <= back else -back
            if 0 < abs(span) <= 60:
                step = 1 if span > 0 else -1
                k = _math.floor(s) + 1 if step > 0 else _math.ceil(s) - 1
                while (k - s) * step > 0 and (k - s) * step < abs(span):
                    out.append(tuple(ring[int(k) % m]))
                    k += step
    # dedupe consecutive
    clean = []
    for pt in out:
        if not clean or _math.hypot(clean[-1][0] - pt[0], clean[-1][1] - pt[1]) > 1e-9:
            clean.append(pt)
    while len(clean) > 1 and _math.hypot(clean[0][0] - clean[-1][0], clean[0][1] - clean[-1][1]) <= 1e-9:
        clean.pop()
    return clean

_targets = _load_targets()
LL = _snap_splice([(lon, lat) for lon, lat in LL], _targets, 0.006)  # ~600 m: the tracing+DP scale
print(f"snap+splice: ring now {len(LL)} points over {len(_targets)} water rings")

lines = ["""// ------------------------- the traced plate contour (calibration proof)
//
// One region of the owner's reference plate, georeferenced and traced:
// the pixel->position function is an affine fit over 12 detected city
// dots (mean residual 1.6 km, max 2.8 km), the border is the region's
// color mask contour (~75 m/px), Douglas-Peucker simplified at ~300 m.
// Every waypoint is an interpolation marker of that tracing, not a
// place. This is the precision reference the tribal circuits converge
// to; the method spreads region by region.

const PLATE_CANAAN_CONTOUR: &[Waypoint] = &["""]
for i, (lon, lat) in enumerate(LL):
    lines.append(f'    Waypoint {{ name: "canaan contour {i:03d}", lat: {lat:.5f}, lon: {lon:.5f} }},')
lines.append("];\n")
block = "\n".join(lines)

t = io.open(SRC, encoding='utf8').read()
anchor = "// --------------------------------- the table of nations"
marker = "// ------------------------- the traced plate contour"
if marker in t:
    i = t.index(marker)
    j = t.index("];", t.index("const PLATE_CANAAN_CONTOUR")) + 2
    t = t[:i] + block.rstrip() + t[j:]
else:
    t = t.replace(anchor, block + chr(10) + anchor, 1)

spec_anchor = """const SURVEYS: &[SurveySpec] = &[
    SurveySpec {
        tag: "NUM34","""
spec_new = """const SURVEYS: &[SurveySpec] = &[
    SurveySpec {
        tag: "PLATE-CANAAN",
        label: "Canaan (traced contour)",
        note: "Georeferenced tracing of the reference plate's Canaan region \\
               (affine calibration over 12 city dots, mean residual 1.6 km); \\
               waypoints are tracing markers, not places.",
        book: 4, chapter: 34, verse_from: 1, verse_to: 12,
        year: -2200,
        grade: Grade::CityDerived,
        circuit: PLATE_CANAAN_CONTOUR,
    },
    SurveySpec {
        tag: "NUM34","""
if 'tag: "PLATE-CANAAN"' not in t:
    t = t.replace(spec_anchor, spec_new, 1)
io.open(SRC, 'w', encoding='utf8', newline='').write(t)
print("emitted", len(LL), "waypoints + spec")
