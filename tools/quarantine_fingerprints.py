import argparse
import hashlib
import json
import math
import struct
import subprocess
from pathlib import Path

from tree_sitter import Language, Parser
import tree_sitter_rust

PARSER = Parser(Language(tree_sitter_rust.language()))
BASE = '6ac32bfbf67e26b9cfe94560806fda293db05801'


def nodes(node):
    yield node
    for child in node.named_children:
        yield from nodes(child)


def text(node, source):
    return source[node.start_byte:node.end_byte].decode()


def fingerprint(lat, lon):
    lat, lon = math.radians(lat), math.radians(lon)
    values = [math.cos(lat) * math.cos(lon), math.cos(lat) * math.sin(lon), math.sin(lat)]
    keys = [math.floor(v * 1e9 + 0.5) if v >= 0 else math.ceil(v * 1e9 - 0.5) for v in values]
    return hashlib.sha256(struct.pack('>qqq', *keys)).hexdigest()


def historical(path):
    return subprocess.check_output(['git', 'show', BASE + ':' + path])


def source_rows(path, source, geometries):
    checksum = hashlib.sha256(historical(path)).hexdigest()
    for name, points in geometries:
        yield {
            'source': source,
            'geometry': name,
            'origin': path,
            'source_sha256': checksum,
            'vertices': sorted({fingerprint(lat, lon) for lat, lon in points}),
        }


def geojson_geometries(path):
    for feature in json.loads(historical(path))['features']:
        name = feature['properties'].get('tribe', feature['properties'].get('id', feature['properties'].get('name', 'unnamed')))
        geometry = feature['geometry']
        polygons = geometry['coordinates'] if geometry['type'] == 'MultiPolygon' else [geometry['coordinates']]
        for p, polygon in enumerate(polygons):
            for r, ring in enumerate(polygon):
                yield f'{name}/{p}/{r}', [(lat, lon) for lon, lat, *rest in ring]


def rust_geometries(path, plate):
    source = historical(path)
    for node in nodes(PARSER.parse(source).root_node):
        if node.type != 'const_item':
            continue
        name = text(node.child_by_field_name('name'), source)
        if not name.startswith('PLATE_'):
            continue
        value = node.child_by_field_name('value')
        points = []
        for entry in nodes(value):
            if plate and entry.type == 'tuple_expression':
                numbers = [float(text(n, source)) for n in entry.named_children]
                if len(numbers) == 2:
                    x, y = numbers
                    lon = 7.423165730975e-4 * x - 1.392245320025e-5 * y + 3.354108963464e1
                    lat = -1.314773386506e-5 * x - 6.519117998042e-4 * y + 3.407261055596e1
                    points.append((lat, lon))
            elif not plate and entry.type == 'struct_expression':
                fields = {}
                for field in nodes(entry):
                    if field.type == 'field_initializer':
                        fields[text(field.child_by_field_name('field'), source)] = text(field.child_by_field_name('value'), source)
                points.append((float(fields['lat']), float(fields['lon'])))
        if points:
            yield name, points


def rounded(value, scale):
    value /= scale
    return (math.floor(value + 0.5) if value >= 0 else math.ceil(value - 0.5)) * scale


def quantized(value):
    value *= 1e7
    return (math.floor(value + 0.5) if value >= 0 else math.ceil(value - 0.5)) / 1e7


def basemap_geometries(path):
    for index, feature in enumerate(json.loads(historical(path))['features']):
        name = feature['properties'].get('NAME') or 'unnamed'
        geometry = feature['geometry']
        polygons = geometry['coordinates'] if geometry['type'] == 'MultiPolygon' else [geometry['coordinates']]
        raw = [(lat, lon) for polygon in polygons for ring in polygon for lon, lat, *rest in ring]
        if raw:
            yield f'{name}/{index}/raw', raw
        for method, snap in [('quantized', None), ('snap-0.02', 0.02)]:
            points = []
            for polygon in polygons:
                for ring in polygon:
                    cleaned = []
                    for lon, lat, *rest in ring:
                        if snap is not None:
                            lon, lat = rounded(lon, snap), rounded(lat, snap)
                        point = (quantized(lat), quantized(lon))
                        if not cleaned or point != cleaned[-1]:
                            cleaned.append(point)
                    while len(cleaned) > 1 and cleaned[0] == cleaned[-1]:
                        cleaned.pop()
                    if len(cleaned) >= 3:
                        points.extend(cleaned)
            if points:
                yield f'{name}/{index}/{method}', points


def basemap_paths():
    paths = subprocess.check_output(['git', 'ls-tree', '-r', '--name-only', BASE, '--', 'data/historical-basemaps']).decode().splitlines()
    return [path for path in paths if path.endswith('.geojson')]


def catalogue():
    rows = []
    for path, source, geometries in [
        ('data/wikimedia/tribes12.geojson', 'Tribes12', geojson_geometries('data/wikimedia/tribes12.geojson')),
        ('data/openbible/regions.geojson', 'SplicedRegions', geojson_geometries('data/openbible/regions.geojson')),
        ('crates/map-adapters/src/surveys.rs', 'KnowingTheBible', rust_geometries('crates/map-adapters/src/surveys.rs', False)),
        ('crates/map-adapters/src/plate_water.rs', 'KnowingTheBible', rust_geometries('crates/map-adapters/src/plate_water.rs', True)),
    ]:
        rows.extend(source_rows(path, source, geometries))
    for path in basemap_paths():
        rows.extend(source_rows(path, 'HistoricalBasemaps', basemap_geometries(path)))
    controls = []
    indexed = [(index, set(row['vertices'])) for index, row in enumerate(rows)]
    for index, row in enumerate(rows):
        if row['source'] != 'HistoricalBasemaps':
            continue
        _, feature, method = row['geometry'].rsplit('/', 2)
        observed = indexed[index][1]
        controls.append({
            'origin': row['origin'], 'feature': int(feature), 'method': method,
            'excluded': [other for other, vertices in indexed if vertices <= observed],
        })
    return {'base': BASE, 'decision': 'docs/errata/quarantine.md; docs/errata/background.md', 'geometries': rows, 'basemap_controls': controls}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    path = Path('data/authored/excluded-geometry-fingerprints.json')
    expected = json.dumps(catalogue(), indent=2) + '\n'
    if args.check:
        if path.read_text() != expected:
            raise SystemExit('quarantine fingerprints differ from their recorded source geometries')
    else:
        path.write_text(expected)
    print(f'{len(json.loads(expected)["geometries"])} excluded geometries fingerprinted')


if __name__ == '__main__':
    main()
