# Contract suite changelog

The suite is **pre-release (0.x)**. Breaking change to an existing feature or
blessed fixture = MINOR bump; additive = PATCH. v1.0.0 is the owner's act,
never automatic. Enforced by `scripts/contract-semver-gate.sh`.

## 0.2.0 — in progress (Stage 1; final entry lands when the stage closes)

Breaking: the coverage corpus (`982817b`) rewrote scenarios in the four
published features the gate names below, quantifying pinned examples over
pieces × year × style and adding camera, detail, transition, and
derivability laws. Steps and hole groups making the corpus runnable landed
in `c86fd0a` + fix round `69224d5`. Census diff instrument and the
absent-vs-malformed `to=` rule: `2cd12dc`, `4d24d1b`. The version is bumped
mid-stage so pushes clear the semver gate; Task 17 finalizes this entry.

- Edited features: `scene.feature`, `census.feature`, `subjects.feature`,
  `resources.feature` (strengthened, breaking under the gate's rule); new
  `camera.feature`, `detail.feature`, `transition.feature`,
  `derivability.feature`.

- The sweep (`8c09f8b`, `2baf81f`) quantified the totality, dehole, and
  composition laws over every piece, year, and style they claim, rather than
  one fixed corner. Quantifying over year (not just pieces) surfaced a fifth
  red the 0.1.0 freeze did not know about: the identity law for map-api's
  scene composition (`someSubset = ∅` at `someYear = 54`) fails today, and
  stays red, untagged, by owner ruling — not `@target`, because it is a
  present defect the corpus can now see, not a predicted future gap. See
  `docs/notes/2026-09-06-contract-diagnosis.md`, Addendum (2026-09-07):
  "the parameterization sweep — every law over every dimension it claims."

## 0.1.0 — 2026-09-06

The freeze. 12 features (`map-api` 6, `atlas-edge` 6), 16 blessed fixtures,
the Haskell runner with the totality law, the vocabulary drift law, and
`@property` scenarios. Two `@target` reds recorded, not fixed:
"composition" and "every manifest entry names its piece".

Corrected in `7cd31bd` before this entry was written: the composition
property's two holes drew from one seed and were always equal, so the law
could not fail. See `docs/notes/2026-09-06-contract-diagnosis.md` §7.0.
