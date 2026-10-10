import pathlib
import sys

import tomli


root = pathlib.Path(sys.argv[1])
evidence = tomli.loads((root / 'data/authored/surveys/allotments.toml').read_text())
judah = next(row for row in evidence['survey'] if row['lot'] == 'judah')
sequence = next(row for row in judah['sequence'] if row['side'] == 'north')
adummim = next(row for row in sequence['waypoints'] if row['name'] == 'Adummim')
relation = next(row for row in adummim['relations'] if row['kind'] == 'position' and row['predicate'] == 'south_of')
river = next(row for row in sequence['waypoints'] if row['name'] == relation['target']['reference'])
if len(sys.argv) == 3 and sys.argv[2] == 'neutral-reference-control':
    river['name'] = 'unnamed river (Joshua 15:7)'
    river['site'] = {'unlocated': 'unnamed river (Joshua 15:7)'}
    river['relations'][0]['target']['reference'] = river['name']
    relation['target']['reference'] = river['name']
assert river['name'] != 'river south of Adummim', 'the unnamed river reference must not reverse the adopted Adummim-south-of-river relation'
