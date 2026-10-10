import hashlib
import json
from pathlib import Path
import subprocess
import tempfile

COMPILER = Path('/tmp/maps-x2-current-build.sgNs2P/map-compile')
COMPILER_SHA256 = '0d8320d1eb61381b876d3fc9c40a9edccb1fab020d569bd108d8da352d07dd8b'
SOURCE = 'cf3c348:data/historical-basemaps/world_bc1500.geojson'
EGYPT_FEATURE = 13


def main():
    binary = COMPILER.read_bytes()
    assert hashlib.sha256(binary).hexdigest() == COMPILER_SHA256, 'the diagnostic runs the immutable X2 compiler'
    source = json.loads(subprocess.check_output(['git', 'show', SOURCE]))
    geometry = source['features'][EGYPT_FEATURE]['geometry']
    polygons = geometry['coordinates'] if geometry['type'] == 'MultiPolygon' else [geometry['coordinates']]
    ring = polygons[0][0]
    outcomes = []
    cases = [
        ('full GPL feature', geometry, True),
        ('three consecutive GPL vertices', {'type': 'LineString', 'coordinates': ring[:3]}, True),
        ('permitted geometry', {'type': 'LineString', 'coordinates': [[.456, .123], [.456, 1.123], [1.456, 1.123]]}, False),
    ]
    for name, shape, expected_refusal in cases:
        with tempfile.TemporaryDirectory(prefix='x2-review-immutable-') as directory:
            root = Path(directory)
            (root / 'data').mkdir()
            value = {'type': 'Feature', 'properties': {'name': 'permitted-looking'}, 'geometry': shape}
            (root / 'data' / 'permitted.geojson').write_text(json.dumps(value))
            output = subprocess.run([str(COMPILER), 'build'], cwd=root, capture_output=True, text=True)
            refused = output.stderr.startswith('map-compile FAILED: excluded input:')
            outcomes.append({
                'case': name, 'expected_content_refusal': expected_refusal,
                'actual_content_refusal': refused, 'expectation_met': refused == expected_refusal,
                'exit_code': output.returncode, 'output_directory_exists': (root / 'data' / 'canon').exists(),
                'stderr': output.stderr if not refused else output.stderr.split('vertices:')[0] + '[fingerprint set omitted]',
            })
    print(json.dumps({'compiler_sha256': COMPILER_SHA256, 'source': SOURCE, 'feature': EGYPT_FEATURE, 'source_ring_vertices': len(ring), 'partial_vertices': 3, 'outcomes': outcomes}, indent=2))
    raise SystemExit(0 if all(outcome['expectation_met'] for outcome in outcomes) else 1)


if __name__ == '__main__':
    main()
