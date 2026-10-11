import argparse
import json
from pathlib import Path


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    inventory = json.loads(Path('data/authored/river-requirements.json').read_text())
    path = Path('docs/errata/rivers.md')
    text = path.read_text()
    start = text.index('| River | Outcome |')
    end = text.index('\n## Kept, lost detail and removed', start)
    rows = ['| River | Outcome | Stop / grounds / source identification and alternatives |', '|---|---|---|']
    for requirement in inventory['requirements']:
        identities = ', '.join(f"{source['name']} NE {source['number']}" for source in requirement['sources'])
        details = ' '.join([requirement['role'], identities, *requirement['alternatives']]).strip()
        rows.append(f"| {requirement['name']} | {requirement['outcome']} | {details} |")
    expected = text[:start] + '\n'.join(rows) + '\n' + text[end:]
    if args.check:
        if expected != text:
            raise SystemExit('river census differs from its golden-authority requirements inventory')
    else:
        path.write_text(expected)
    print(f"{len(inventory['requirements'])} golden river requirements recorded")


if __name__ == '__main__':
    main()
