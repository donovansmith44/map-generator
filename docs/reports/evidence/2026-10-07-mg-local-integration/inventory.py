import argparse
import hashlib
import json
import pathlib
import re
import subprocess


LOCAL = '089a4bf899f9bd4da08061d710618e7efb2f7c90'
REMOTE = '63bb038897a036a75a9dce050676cc79c7d690aa'
MERGED = '4d3200774a34c91cb4da1c708ec2415047559a09'
ATLAS = '9da8a6c51eeef1319124b2178ff218a49b8097d9'


def git(tree, *arguments):
    return subprocess.check_output(['git', '-C', str(tree), *arguments])


def digest(data):
    return hashlib.sha256(data).hexdigest()


def inventory(map_tree, atlas_tree):
    registry_bytes = git(map_tree, 'show', f'{MERGED}:data/authored/registry.json')
    old_bytes = git(map_tree, 'show', f'{MERGED}:data/atlas-exports/gazetteer.json')
    current_path = atlas_tree / 'data/exports/gazetteer.json'
    current_bytes = current_path.read_bytes()
    old_export = json.loads(old_bytes)
    current_export = json.loads(current_bytes)
    old_places = {row['id']: row for row in old_export['places']}
    current_places = {row['id']: row for row in current_export['places']}
    absent = set(old_places) - set(current_places)
    merge_source = git(atlas_tree, 'show', f'{ATLAS}:server/atlas-core/src/merge.rs')
    pair_headers = re.findall(
        r'survivor: "([^"]+)",\s*absorbed: "([^"]+)"', merge_source.decode()
    )
    source_pairs = {absorbed: survivor for survivor, absorbed in pair_headers}
    unmatched = sorted(absent - set(source_pairs))
    if unmatched:
        raise RuntimeError(f'Export removals have no recorded source merge: {unmatched}')
    removed = [
        {
            'absorbed': prior,
            'survivor': source_pairs[prior],
            'kind': 'Place',
            'proposed_move': 'AbsorbedInto',
            'old_export_row': old_places[prior],
            'survivor_in_current_export': source_pairs[prior] in current_places,
            'reason_source': {
                'commit': '0b29bf129786a43f78ce62cf40aee7dd7a845635',
                'current_revision': ATLAS,
                'path': 'server/atlas-core/src/merge.rs',
            },
        }
        for prior in sorted(absent)
    ]
    if any(not row['survivor_in_current_export'] for row in removed):
        raise RuntimeError('A recorded survivor is absent from the current export')
    registry = json.loads(registry_bytes)
    aliases = registry['unifications']
    canonical_homes = {row['canonical'] for row in aliases}
    minted_aliases = {row['minted'] for row in aliases}
    atlas_place_aliases = sorted(minted_aliases & set(old_places))
    vendor_bytes = git(map_tree, 'show', f'{MERGED}:data/atlas-vendor/polities.json')
    vendor_polities = {row['id'] for row in json.loads(vendor_bytes)['polities']}
    atlas_polity_aliases = sorted(minted_aliases & vendor_polities)
    inherited_local = git(map_tree, 'rev-list', f'{REMOTE}..{LOCAL}').decode().splitlines()
    inherited_remote = git(map_tree, 'rev-list', f'{LOCAL}..{REMOTE}').decode().splitlines()
    ancestor_statuses = {
        commit: subprocess.run(
            ['git', '-C', str(map_tree), 'merge-base', '--is-ancestor', commit, MERGED],
            check=False, capture_output=True,
        ).returncode
        for commit in inherited_local + inherited_remote
    }
    return {
        'scope': 'History preservation and source/export inventory; no graph rebuild, runtime registry validation or served-id successor gate.',
        'local': LOCAL,
        'remote': REMOTE,
        'merge': MERGED,
        'merge_parents': git(map_tree, 'show', '-s', '--format=%P', MERGED).decode().strip().split(),
        'merge_changed_paths': git(map_tree, 'diff', '--name-only', LOCAL, MERGED).decode().splitlines(),
        'all_inherited_commit_ancestor_statuses': ancestor_statuses,
        'local_only_count': len(inherited_local),
        'remote_only_count': len(inherited_remote),
        'registry_unifications': aliases,
        'registry_sanity': {
            'declared_aliases': len(aliases),
            'unique_minted_aliases': len(minted_aliases),
            'self_aliases': [row for row in aliases if row['canonical'] == row['minted']],
            'chained_declared_homes': sorted(canonical_homes & minted_aliases),
            'atlas_place_aliases': atlas_place_aliases,
            'atlas_polity_aliases': atlas_polity_aliases,
        },
        'exports': {
            'old_root': old_export['atlas_version_root'],
            'current_root': current_export['atlas_version_root'],
            'old_place_count': len(old_places),
            'current_place_count': len(current_places),
            'added_place_keys': sorted(set(current_places) - set(old_places)),
            'removed_place_keys': sorted(absent),
        },
        'place_moves': removed,
        'hashes': {
            'merged_registry': digest(registry_bytes),
            'merged_old_export': digest(old_bytes),
            'current_atlas_export': digest(current_bytes),
            'pinned_atlas_merge_source': digest(merge_source),
            'merged_vendor_polities': digest(vendor_bytes),
        },
    }


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--atlas', type=pathlib.Path, required=True)
    options = parser.parse_args()
    directory = pathlib.Path(__file__).resolve().parent
    map_tree = pathlib.Path(git(directory, 'rev-parse', '--show-toplevel').decode().strip())
    print(json.dumps(inventory(map_tree, options.atlas), indent=2))


if __name__ == '__main__':
    main()
