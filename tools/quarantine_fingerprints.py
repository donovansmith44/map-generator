import argparse
import hashlib
import json
import os
import subprocess
from functools import lru_cache
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


@lru_cache(maxsize=None)
def historical(path):
    return subprocess.check_output(['git', 'show', BASE + ':' + path])


def source_rows(path, source, geometries):
    for name, points in geometries:
        yield {
            'source': source,
            'geometry': name,
            'origin': path,
            'source_sha256': hashlib.sha256(historical(path)).hexdigest(),
            'vertices': points,
        }


def geojson_geometries(path):
    for index, feature in enumerate(json.loads(historical(path))['features']):
        name = feature['properties'].get('tribe', feature['properties'].get('id', feature['properties'].get('name', feature['properties'].get('NAME', 'unnamed'))))
        geometry = feature['geometry']
        if geometry['type'] == 'LineString':
            yield f'{name}/{index}/0', [(lat, lon) for lon, lat, *rest in geometry['coordinates']]
            continue
        polygons = geometry['coordinates'] if geometry['type'] == 'MultiPolygon' else [geometry['coordinates']]
        for p, polygon in enumerate(polygons):
            for r, ring in enumerate(polygon):
                key = f'{name}/{index}/{p}/{r}' if 'historical-basemaps' in path else f'{name}/{p}/{r}'
                yield key, [(lat, lon) for lon, lat, *rest in ring]


def basemap_geometries(path):
    for name, points in geojson_geometries(path):
        yield name + '/raw', points
        for method, scale in [('quantized', 1e-7), ('snap-0.02', 0.02)]:
            rounded = [(round_away(lat, scale), round_away(lon, scale)) for lat, lon in points]
            cleaned = [point for i, point in enumerate(rounded) if i == 0 or point != rounded[i - 1]]
            while len(cleaned) > 1 and cleaned[0] == cleaned[-1]:
                cleaned.pop()
            if len(cleaned) >= 3:
                yield name + '/' + method, cleaned + cleaned[:1]


def round_away(value, scale):
    import math
    value /= scale
    return (math.floor(value + 0.5) if value >= 0 else math.ceil(value - 0.5)) * scale


def basemap_paths():
    paths = subprocess.check_output(['git', 'ls-tree', '-r', '--name-only', BASE, '--', 'data/historical-basemaps']).decode().splitlines()
    return [path for path in paths if path.endswith('.geojson')]


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


def catalogue():
    rows = []
    for path, source, geometries in [
        ('data/wikimedia/tribes12.geojson', 'Tribes12', geojson_geometries('data/wikimedia/tribes12.geojson')),
        ('data/openbible/regions.geojson', 'SplicedRegions', geojson_geometries('data/openbible/regions.geojson')),
        ('crates/map-adapters/src/surveys.rs', 'KnowingTheBible', rust_geometries('crates/map-adapters/src/surveys.rs', False)),
        ('crates/map-adapters/src/plate_water.rs', 'KnowingTheBible', rust_geometries('crates/map-adapters/src/plate_water.rs', True)),
        ('data/osm/rivers.geojson', 'OsmRivers', geojson_geometries('data/osm/rivers.geojson')),
    ]:
        rows.extend(source_rows(path, source, geometries))
    for path in basemap_paths():
        rows.extend(source_rows(path, 'HistoricalBasemaps', basemap_geometries(path)))
    permitted = []
    for path in [
        'data/natural-earth/ne_10m_land.geojson',
        'data/natural-earth/ne_10m_ocean.geojson',
        'data/natural-earth/ne_10m_lakes.geojson',
        'data/natural-earth/ne_50m_land.geojson',
        'data/natural-earth/ne_50m_lakes.geojson',
        'data/natural-earth/ne_110m_land.geojson',
        'data/natural-earth/ne_110m_ocean.geojson',
        'data/natural-earth/ne_110m_lakes.geojson',
        'data/natural-earth/ne_10m_rivers_lake_centerlines.geojson',
        'data/natural-earth/med_clip.geojson',
    ]:
        raw = historical(path)
        permitted.append({'origin': path, 'source_sha256': hashlib.sha256(raw).hexdigest(), 'license': 'Public Domain', 'value': json.loads(raw)})
    policy = {'tolerance_meters': 100.0, 'maximum_unexplained_meters': 2000.0, 'short_line_fraction': 0.1}
    output = subprocess.check_output([os.environ.get('QUARANTINE_KEYS_EXECUTABLE', str(Path(os.environ['CARGO_TARGET_DIR']) / 'debug/examples/quarantine_keys'))], input=json.dumps({'policy': policy, 'geometries': rows, 'permitted': permitted}).encode())
    return {'base': BASE, 'decision': 'docs/errata/quarantine.md', 'policy': policy, **json.loads(output)}



def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    path = Path('data/authored/excluded-geometry-fingerprints.json')
    expected = json.dumps(catalogue(), separators=(',', ':')) + '\n'
    if args.check:
        if path.read_text() != expected:
            raise SystemExit('quarantine fingerprints differ from their recorded source geometries')
    else:
        path.write_text(expected)
    print(f'{len(json.loads(expected)["geometries"])} excluded geometries catalogued')


if __name__ == '__main__':
    main()
