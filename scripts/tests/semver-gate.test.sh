#!/usr/bin/env bash
# The semver rule (spec §4) as an executable test. Four cases, and the
# NEGATIVE ones matter most: a gate that passes everything is not a gate.
set -u
GATE="$(cd "$(dirname "$0")/.." && pwd)/contract-semver-gate.sh"
fail=0
scenario() { # name, expected exit, setup commands
  local name="$1" want="$2"; shift 2
  local d; d="$(mktemp -d)"
  ( cd "$d"
    git init -q . && git config user.email t@t && git config user.name t
    mkdir -p contracts/map-api/fixtures
    printf '0.1.0\n' > contracts/VERSION
    printf '# Changelog\n' > contracts/CHANGELOG.md
    printf 'Feature: f\n  Scenario: s\n    When I GET /a\n' > contracts/map-api/a.feature
    printf '{"x":1}\n' > contracts/map-api/fixtures/a.json
    git add -A && git commit -qm base
    eval "$@"
    git add -A && git commit -qm change
    "$GATE" HEAD~1 ) >/dev/null 2>&1
  local got=$?
  if [ "$got" != "$want" ]; then echo "FAIL $name: want exit $want, got $got"; fail=1
  else echo "ok   $name"; fi
  rm -rf "$d"
}
scenario "edit a feature with no bump -> reject" 1 \
  "printf 'Feature: f\n  Scenario: s2\n    When I GET /b\n' > contracts/map-api/a.feature"
scenario "edit a fixture with no bump -> reject" 1 \
  "printf '{\"x\":2}\n' > contracts/map-api/fixtures/a.json"
scenario "edit a feature WITH bump and changelog -> accept" 0 \
  "printf 'Feature: f\n  Scenario: s2\n    When I GET /b\n' > contracts/map-api/a.feature; \
   printf '0.2.0\n' > contracts/VERSION; \
   printf '# Changelog\n\n## 0.2.0\n- changed a.feature\n' > contracts/CHANGELOG.md"
scenario "ADD a new feature with no bump -> accept (additive)" 0 \
  "printf 'Feature: g\n  Scenario: s\n    When I GET /c\n' > contracts/map-api/b.feature"
exit $fail
