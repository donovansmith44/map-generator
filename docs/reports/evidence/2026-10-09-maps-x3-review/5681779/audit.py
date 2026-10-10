import hashlib
import json
import pathlib
import subprocess
import sys

import tomli


ROOT = pathlib.Path(sys.argv[1])
OUT = pathlib.Path(sys.argv[2])
ATLAS = pathlib.Path('/home/donovan/src/bible-atlas')
KJV = ATLAS / 'data/raw/kjv.json'


def main():
    source = ROOT / 'data/authored/surveys/allotments.toml'
    evidence = tomli.loads(source.read_text())
    bible = json.loads(KJV.read_text())
    verses = {}
    for book, tag in [('Numbers', 'NUM'), ('Joshua', 'JOS')]:
        content = next(row for row in bible['books'] if row['name'] == book)
        for chapter in content['chapters']:
            for row in chapter['verses']:
                verses[f"{tag}.{chapter['chapter']}.{row['verse']}"] = row['text']
    samples = [
        ('promised_land', 'east', 'Riblah', 'NUM.34.11', 'Riblah and Ain are separate; the recorded grammatical alternative is Medium.'),
        ('reuben', 'south', 'Aroer by the Arnon', 'JOS.13.16', 'The Arnon bank is the target; the unnamed river city remains separately unlocated.'),
        ('gad', 'north', 'Mahanaim', 'JOS.13.26', 'Mahanaim-to-Debir references remain undrawn.'),
        ('judah', 'north', 'Adummim', 'JOS.15.7', 'Gilgal/ascent attachment repaired; F-291 remains because the unnamed river label reverses the recorded primary south-of relation.'),
        ('judah', 'north', 'hilltop west of Hinnom', 'JOS.15.8', 'The hill and its relations to both valleys are explicit; the alternative remains Medium.'),
        ('ephraim', 'north', 'Janohah', 'JOS.16.6', 'Janohah is the destination; Taanath-shiloh is the east-of target.'),
        ('manasseh_west', 'northeast', 'Asher', 'JOS.17.10', 'Manasseh is the subject and Asher the northern neighbour; no course inferred.'),
        ('benjamin', 'south', 'Geliloth', 'JOS.18.17', 'Geliloth is the subject of over-against the ascent of Adummim.'),
        ('naphtali', 'south', 'Adami', 'JOS.19.33', 'Adami and Nekeb remain separate textual names despite the shared atlas identity.'),
        ('dan', 'west', 'Japho', 'JOS.19.46', 'The border-before-Japho relation retains its subject without a possession claim.'),
    ]
    checked = []
    for lot, side, name, ref, assessment in samples:
        survey = next(row for row in evidence['survey'] if row['lot'] == lot)
        sequence = next(row for row in survey['sequence'] if row['side'] == side)
        row = next(row for row in sequence['waypoints'] if row['name'] == name and row['verse'] == ref)
        checked.append({'lot': lot, 'side': side, 'row': row, 'kjv': verses[ref], 'assessment': assessment})
    sequences = [sequence for survey in evidence['survey'] for sequence in survey['sequence']]
    waypoints = [row for sequence in sequences for row in sequence['waypoints']]
    relations = [relation for row in waypoints for relation in row['relations']]
    missing_targets = []
    for survey in evidence['survey']:
        names = {row['name'] for sequence in survey['sequence'] for row in sequence['waypoints']}
        for sequence in survey['sequence']:
            for row in sequence['waypoints']:
                for relation in row['relations']:
                    references = [relation['target']['reference']]
                    if isinstance(relation.get('subject'), dict) and relation['subject']['kind'] == 'reference':
                        references.append(relation['subject']['reference'])
                    missing_targets.extend({'lot': survey['lot'], 'name': reference} for reference in references if reference not in names)
    result = {
        'head': git(ROOT, 'rev-parse', 'HEAD'),
        'repair_base': git(ROOT, 'rev-parse', 'ecd3563'),
        'requested_x0': git(ROOT, 'rev-parse', 'origin/lane/claude/MAPS-X0'),
        'inherited_x0': git(ROOT, 'rev-parse', '7bcbaba'),
        'mg_base': git(ROOT, 'rev-parse', 'origin/lane/claude/MG-BASE'),
        'atlas_head': git(ATLAS, 'rev-parse', 'HEAD'),
        'kjv_sha256': hashlib.sha256(KJV.read_bytes()).hexdigest(),
        'survey_sha256': hashlib.sha256(source.read_bytes()).hexdigest(),
        'survey_count': len(evidence['survey']),
        'sequence_count': len(sequences),
        'waypoint_count': len(waypoints),
        'unresolved_identity_count': sum('unlocated' in row['site'] for row in waypoints),
        'sources': sorted({row['source'] for row in evidence['survey']}),
        'invalid_survey_citations': [ref for row in evidence['survey'] for ref in row['verses'] if not valid_citation(ref, verses)],
        'invalid_waypoint_citations': [row['verse'] for row in waypoints if not valid_citation(row['verse'], verses)],
        'missing_relation_references': missing_targets,
        'relation_kind_counts': {kind: sum(row['kind'] == kind for row in relations) for kind in sorted({row['kind'] for row in relations})},
        'dating': evidence['dating'],
        'relation_readings': evidence['relation_reading'],
        'forbidden_geometry_fields': forbidden_fields(evidence),
        'kjv_spot_checks': checked,
    }
    (OUT / 'audit.json').write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps({key: result[key] for key in ['head', 'survey_count', 'sequence_count', 'waypoint_count', 'unresolved_identity_count', 'sources', 'invalid_survey_citations', 'invalid_waypoint_citations', 'missing_relation_references', 'relation_kind_counts', 'forbidden_geometry_fields']}, indent=2))


def git(root, *args):
    return subprocess.check_output(['git', '-C', str(root), *args], text=True).strip()


def valid_citation(ref, verses):
    first, separator, last = ref.partition('-')
    if first not in verses:
        return False
    book, chapter, verse = first.split('.')
    chapter, verse = int(chapter), int(verse)
    if not (book == 'NUM' and chapter == 34 and verse <= 15 or book == 'JOS' and 13 <= chapter <= 19):
        return False
    end = int(last) if separator else verse
    if book == 'JOS' and chapter == 19 and verse <= 47 <= end:
        return False
    return end >= verse and all(f'{book}.{chapter}.{number}' in verses for number in range(verse, end + 1))


def forbidden_fields(value):
    if isinstance(value, dict):
        return [key for key in value if key in ['lat', 'lon', 'coordinates', 'pts', 'polygon', 'ring']] + [key for child in value.values() for key in forbidden_fields(child)]
    if isinstance(value, list):
        return [key for child in value for key in forbidden_fields(child)]
    return []


if __name__ == '__main__':
    main()
