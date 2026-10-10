import argparse
import hashlib
import json
import math
import re
import subprocess
from pathlib import Path

import pyclipper

BASE = 'cf3c348'
BBOX = (-10.9, 7.6, 71.4, 48.9)
START = '## Reproducible era inventory\n'
END = '## Replacement evidence by era\n'


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    inventory = generate()
    path = Path('docs/errata/background.md')
    document = path.read_text()
    before, rest = document.split(START, 1)
    _, after = rest.split(END, 1)
    expected = before + START + inventory + END + after
    if args.check:
        if document != expected:
            raise SystemExit('background inventory differs from the pinned GPL input and era/shadow data')
    else:
        path.write_text(expected)
    print('Ten legacy eras inventoried from twelve pinned GPL snapshots')


def generate():
    eras = json.loads(historical('data/atlas-vendor/eras.json'))
    polities = json.loads(historical('data/atlas-vendor/polities.json'))['polities']
    shadows = {}
    for polity in polities:
        for key in [slug(polity['id']), slug(polity['name'])]:
            shadows.setdefault(key, []).append((polity['from'], polity['to']))
    paths = subprocess.check_output(['git', 'ls-tree', '-r', '--name-only', BASE, '--', 'data/historical-basemaps']).decode().splitlines()
    epochs = []
    checksums = []
    for path in paths:
        if not path.endswith('.geojson'):
            continue
        payload = historical(path)
        checksums.append((path, hashlib.sha256(payload).hexdigest()))
        label = Path(path).stem.removeprefix('world_')
        year = -int(label[2:]) if label.startswith('bc') else int(label)
        features = json.loads(payload)['features']
        names = {}
        named_world = set()
        for index, feature in enumerate(features):
            name = feature['properties'].get('NAME')
            if name is None:
                continue
            named_world.add(name)
            geometry = feature['geometry']
            polygons = geometry['coordinates'] if geometry['type'] == 'MultiPolygon' else [geometry['coordinates']]
            cleaned = []
            for polygon in polygons:
                rings = []
                for ring in polygon:
                    points = []
                    for lon, lat, *rest in ring:
                        point = [quantized(rounded(lon, .02)), quantized(rounded(lat, .02))]
                        if not points or point != points[-1]:
                            points.append(point)
                    while len(points) > 1 and points[0] == points[-1]:
                        points.pop()
                    if len(points) >= 3:
                        rings.append(points + [points[0]])
                if rings:
                    cleaned.append(rings)
            if not cleaned:
                continue
            if any(intersects_box(polygon) for polygon in cleaned):
                names.setdefault(name, []).append(f'{index}:{len(cleaned)}p/{sum(map(len, cleaned))}r')
        epochs.append((year, Path(path).stem, names, sorted(named_world)))
    epochs.sort()
    lines = ['\nThe box is the inherited atlas `BIBLICAL_WORLD_BBOX`: west −10.9°, south 7.6°, east 71.4°, north 48.9°. It is an inventory window, not a new drawing boundary. Rows replay the old 0.02° snap and named-feature admission, then subtract only the old polity id/name slug shadow spans. A listed outline survives for at least one year in that era; `suppressed` lists ones hidden for the entire overlapping interval. Feature references `index:Xp/Yr` identify zero-based source features and their admitted polygon/ring counts, so an outline is reproducible without retaining its vertices. Anonymous features were never drawn.\n\n',
             '| Era (inherited years) | GPL epoch | Surviving in-box outlines: label [feature:polygons/rings] | Fully suppressed |\n',
             '|---|---|---|---|\n']
    for era in eras:
        any_epoch = False
        for number, (year, label, names, _) in enumerate(epochs):
            limit = epochs[number + 1][0] - 1 if number + 1 < len(epochs) else era['to_year']
            start, end = max(year, era['from_year']), min(limit, era['to_year'])
            if start > end:
                continue
            any_epoch = True
            shown, hidden = [], []
            for name, references in sorted(names.items()):
                spans = shadows.get(slug(name), [])
                survives = any(not any(a <= at <= b for a, b in spans) for at in range(start, end + 1) if at != 0)
                entry = f'{name} [{", ".join(references)}]'
                (shown if survives else hidden).append(entry)
            lines.append(f'| {era["name"]} ({era["from_year"]}…{era["to_year"]}) | `{label}` ({start}…{end}) | {"; ".join(shown) or "none"} | {"; ".join(hidden) or "none"} |\n')
        if not any_epoch:
            lines.append(f'| {era["name"]} | none | none | none |\n')
    lines.append('\nBefore the first snapshot (4004–4001 BC), the GPL layer had no input. The final AD 100 epoch began at the final legacy era boundary and remained open in the old adapter; this removal also ends that extrapolation. World-camera outlines outside the box are excluded too:\n\n')
    for _, label, _, world_names in epochs:
        lines.append(f'- `{label}` ({len(world_names)} named entities worldwide): {"; ".join(world_names)}.\n')
    lines.append('\nSource bytes pinned in git history; SHA-256 is evidence, not a retained drawing:\n\n| Source | SHA-256 |\n|---|---|\n')
    for path, checksum in checksums:
        lines.append(f'| `{path}` | `{checksum}` |\n')
    lines.append('\n')
    return ''.join(lines)


def intersects_box(polygon):
    rings = [[[round(lon * 1e7), round(lat * 1e7)] for lon, lat in ring] for ring in polygon]
    subject = pyclipper.SimplifyPolygons(rings, pyclipper.PFT_EVENODD)
    if not subject:
        return False
    west, south, east, north = [round(value * 1e7) for value in BBOX]
    clip = [[west, south], [east, south], [east, north], [west, north]]
    operation = pyclipper.Pyclipper()
    operation.AddPaths(subject, pyclipper.PT_SUBJECT, True)
    operation.AddPath(clip, pyclipper.PT_CLIP, True)
    return any(pyclipper.Area(ring) != 0 for ring in operation.Execute(pyclipper.CT_INTERSECTION, pyclipper.PFT_EVENODD, pyclipper.PFT_EVENODD))


def historical(path):
    return subprocess.check_output(['git', 'show', BASE + ':' + path])


def slug(name):
    return re.sub('[^a-z0-9]+', '-', name.lower(), flags=re.ASCII).strip('-')


def quantized(value):
    value *= 1e7
    return (math.floor(value + .5) if value >= 0 else math.ceil(value - .5)) / 1e7


def rounded(value, scale):
    value /= scale
    return (math.floor(value + .5) if value >= 0 else math.ceil(value - .5)) * scale


if __name__ == '__main__':
    main()
