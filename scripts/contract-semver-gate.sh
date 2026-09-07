#!/usr/bin/env bash
# Spec §4: "a diff editing an existing feature or blessed fixture
# without a bump fails". ADDING a file is additive and needs no bump;
# EDITING or DELETING one is a change to a published promise and needs
# both a VERSION move and a CHANGELOG entry. Deliberately dumb about
# WHAT changed -- the version bump is the owner's declaration, and this
# gate's whole job is to make sure the declaration happens.
set -u
base="${1:-origin/master}"
changed() { git diff --name-only --diff-filter="$1" "$base"..HEAD -- contracts/; }

breaking="$( { changed M; changed D; changed R; } \
  | grep -E '(\.feature$|/fixtures/)' || true )"
[ -z "$breaking" ] && exit 0

bumped=0
git diff --name-only "$base"..HEAD -- contracts/VERSION      | grep -q . && bumped=1
logged=0
git diff --name-only "$base"..HEAD -- contracts/CHANGELOG.md | grep -q . && logged=1

if [ "$bumped" = 1 ] && [ "$logged" = 1 ]; then exit 0; fi

echo "semver gate: these published contract files were edited or removed:"
echo "$breaking" | sed 's/^/  /'
[ "$bumped" = 0 ] && echo "  MISSING: a change to contracts/VERSION"
[ "$logged" = 0 ] && echo "  MISSING: an entry in contracts/CHANGELOG.md"
echo "The suite is pre-release: breaking = MINOR bump, additive = PATCH."
exit 1
