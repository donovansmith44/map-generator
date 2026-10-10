import hashlib
import json
import pathlib
import subprocess

import tomli


ROOT = pathlib.Path(__file__).resolve().parents[4]
ATLAS = pathlib.Path('/home/donovan/src/bible-atlas')
OUT = pathlib.Path(__file__).resolve().parent
EVIDENCE = ROOT / 'data/authored/surveys/allotments.toml'
KJV = ATLAS / 'data/raw/kjv.json'


def main():
    evidence = tomli.loads(EVIDENCE.read_text())
    kjv = json.loads(KJV.read_text())
    books = {book['name']: book for book in kjv['books']}
    verses = {}
    citations = set()
    for name, tag in [('Numbers', 'NUM'), ('Joshua', 'JOS')]:
        for chapter in books[name]['chapters']:
            rows = chapter['verses']
            for row in rows:
                ref = f"{tag}.{chapter['chapter']}.{row['verse']}"
                verses[ref] = row['text']
                citations.add(ref)
                for last in rows:
                    if last['verse'] >= row['verse']:
                        citations.add(f"{ref}-{last['verse']}")
    surveys = evidence['survey']
    sequences = [sequence for survey in surveys for sequence in survey['sequence']]
    waypoints = [waypoint for sequence in sequences for waypoint in sequence['waypoints']]
    samples = [
        ('promised_land', 'east', 'Riblah', 'NUM.34.11', 'Names and east-of-Ain relation retained; no coordinates.'),
        ('reuben', 'south', 'Aroer by the Arnon', 'JOS.13.16', 'Aroer and Arnon retained; unnamed river city separately unlocated.'),
        ('gad', 'north', 'Mahanaim', 'JOS.13.26', 'Mahanaim-to-Debir reference retained; no located course claimed.'),
        ('judah', 'south', 'Kadesh-barnea', 'JOS.15.3', 'South-of-Kadesh wording retained.'),
        ('judah', 'north', 'Adummim', 'JOS.15.7', 'F-291: before-Adummim qualifies Gilgal; the flattened reference loses that subject.'),
        ('ephraim', 'north', 'Janohah', 'JOS.16.6', 'F-291: east of Taanath-shiloh on the way to Janohah; east_of is attached to Janohah.'),
        ('manasseh_west', 'northeast', 'Asher', 'JOS.17.10', 'Northern tribal neighbour retained without a drawn course.'),
        ('benjamin', 'south', 'Adummim', 'JOS.18.17', 'F-291: over-against-Adummim qualifies Geliloth; the flattened reference loses that subject.'),
        ('naphtali', 'south', 'Adami', 'JOS.19.33', 'Adami and Nekeb remain separate textual names despite their shared atlas id.'),
        ('dan', 'west', 'Japho', 'JOS.19.46', 'Before-Japho limit retained without asserting possession of Joppa.'),
    ]
    checked = []
    for lot, side, name, ref, assessment in samples:
        survey = next(row for row in surveys if row['lot'] == lot)
        sequence = next(row for row in survey['sequence'] if row['side'] == side)
        waypoint = next(row for row in sequence['waypoints'] if row['name'] == name and row['verse'] == ref)
        checked.append({'lot': lot, 'side': side, 'row': waypoint, 'kjv': verses[ref], 'assessment': assessment})
    source = (ROOT / 'crates/map-adapters/src/surveys.rs').read_text()
    fixture = (ROOT / 'crates/map-adapters/src/tests.rs').read_text()
    fixture = fixture.split('fn scripture_surveys_are_lawful_alone_and_merged()', 1)[1].split('\n#[test]', 1)[0]
    new = source.split('mod allotment_laws {', 1)[1]
    count = lambda text: text.count('assert!(') + text.count('assert_eq!(')
    result = {
        'reviewed_head': subprocess.check_output(['git', '-C', str(ROOT), 'rev-parse', 'HEAD'], text=True).strip(),
        'atlas_head': subprocess.check_output(['git', '-C', str(ATLAS), 'rev-parse', 'HEAD'], text=True).strip(),
        'kjv_path': str(KJV),
        'kjv_sha256': hashlib.sha256(KJV.read_bytes()).hexdigest(),
        'evidence_sha256': hashlib.sha256(EVIDENCE.read_bytes()).hexdigest(),
        'survey_count': len(surveys),
        'sequence_count': len(sequences),
        'waypoint_count': len(waypoints),
        'unresolved_identity_count': sum('unlocated' in row['site'] for row in waypoints),
        'survey_sources': sorted({row['source'] for row in surveys}),
        'invalid_survey_citations': [ref for row in surveys for ref in row['verses'] if ref not in citations],
        'invalid_waypoint_citations': [row['verse'] for row in waypoints if row['verse'] not in verses],
        'dating': evidence['dating'],
        'new_test_and_helper_assertion_count': count(new),
        'touched_fixture_assertion_count': count(fixture),
        'assertion_review': 'Manual whole-body review: one fact and a plain behavior message in every macro; no tuple or Boolean-vector observations. Counts use textual inventory only, not an AST parser.',
        'kjv_spot_checks': checked,
    }
    (OUT / 'audit.json').write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps({key: result[key] for key in ['survey_count', 'sequence_count', 'waypoint_count', 'unresolved_identity_count', 'invalid_survey_citations', 'invalid_waypoint_citations', 'new_test_and_helper_assertion_count', 'touched_fixture_assertion_count']}, indent=2))


if __name__ == '__main__':
    main()
