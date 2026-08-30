# Vendor the plate's settlements: the atlas gazetteer's curated
# coordinates and verse attestations, joined at VENDOR TIME with
# OpenBible.info's place typing (CC BY 4.0) so only SETTLEMENTS become
# city dots — regions and waters already speak as areas. The join is
# by normalized name, done once here with every miss printed; the
# engine never name-matches at runtime.
#
# Declared thresholds, measured on the data:
#   - attestations >= 20: the in-frame distribution knees there
#     (47 places >= 20, 90 >= 10); the plate look wants ~15-25 dots.
#   - proximity dedup 2 km: "Jerusalem" vs "City of David" vs "Zion"
#     are one dot on a plate; the most-attested name survives.
import io
import json
import math
import sys

sys.stdout.reconfigure(errors='replace', encoding='ascii')
TMP = r'C:/Users/donov/.claude/jobs/c6946bce/tmp'
REPO = r'C:/Users/donov/Documents/the-best-maps-ever'
OUT = f'{REPO}/data/openbible/settlements.geojson'

FRAME = (29.5, 34.0, 33.5, 37.2)  # lat0, lat1, lon0, lon1
MIN_ATTESTATIONS = 20
DEDUP_KM = 2.0


def base_name(s):
    s = s.strip()
    parts = s.rsplit(' ', 1)
    if len(parts) == 2 and parts[1].isdigit():
        s = parts[0]
    return s.lower()


# OpenBible typing: base name -> the types of its DOMINANT sense (the
# sense scripture uses most, measured by verse count) — a name whose
# settlement sense is a footnote to its region sense (Gilead, Galilee)
# is not a city dot.
ob_best = {}
for line in open(f'{TMP}/ob_ancient.jsonl', encoding='utf8'):
    o = json.loads(line)
    key = base_name(o.get('friendly_id', ''))
    weight = len(o.get('verses') or [])
    if key not in ob_best or weight > ob_best[key][0]:
        ob_best[key] = (weight, set(o.get('types') or []))
ob_types = {k: v[1] for k, v in ob_best.items()}

gaz = json.load(open(f'{REPO}/data/atlas-exports/gazetteer.json', encoding='utf8'))
la0, la1, lo0, lo1 = FRAME
candidates = []
misses = []
for p in gaz['places']:
    att = len(p.get('attestations') or [])
    if att < MIN_ATTESTATIONS:
        continue
    if not (la0 <= p['lat'] <= la1 and lo0 <= p['lon'] <= lo1):
        continue
    types = ob_types.get(base_name(p['canonical']))
    if types is None:
        misses.append((p['canonical'], att))
        continue
    if 'settlement' not in types:
        continue
    candidates.append((att, p))

for name, att in misses:
    print(f'  no OpenBible typing for {name!r} ({att} attestations) — skipped')

# proximity dedup: most-attested first claims its ground
candidates.sort(key=lambda t: -t[0])
kept = []
for att, p in candidates:
    close = any(
        math.hypot((p['lat'] - q['lat']) * 111.0,
                   (p['lon'] - q['lon']) * 111.0 * math.cos(math.radians(p['lat'])))
        < DEDUP_KM
        for _, q in kept
    )
    if close:
        print(f"  {p['canonical']} folds into a nearer, better-attested dot")
        continue
    kept.append((att, p))

def display(name):
    parts = name.rsplit(' ', 1)
    return parts[0] if len(parts) == 2 and parts[1].isdigit() else name


features = [{
    'type': 'Feature',
    'properties': {'place': p['id'], 'name': display(p['canonical']), 'attestations': att},
    'geometry': {'type': 'Point', 'coordinates': [p['lon'], p['lat']]},
} for att, p in kept]

json.dump({
    'type': 'FeatureCollection',
    'note': ('Settlements for the plate: atlas gazetteer coordinates and verse '
             'attestations, typed as settlements by OpenBible.info Bible-Geocoding-Data '
             '(CC BY 4.0), joined by normalized name at vendor time. Thresholds '
             'measured and declared in tools/plate_trace/vendor_settlements.py.'),
    'license': 'atlas gazetteer (project data) + OpenBible.info typing, CC BY 4.0',
    'features': features,
}, io.open(OUT, 'w', encoding='utf8'))
print(f'settlements.geojson: {len(features)} dots '
      f'(from {len(candidates)} candidates, {len(misses)} typing misses)')
for att, p in kept:
    print(f"  {att:4d}  {p['canonical']}")
