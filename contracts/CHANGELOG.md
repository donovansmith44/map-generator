# Contract suite changelog

The suite is **pre-release (0.x)**. Breaking change to an existing feature or
blessed fixture = MINOR bump; additive = PATCH. v1.0.0 is the owner's act,
never automatic. Enforced by `scripts/contract-semver-gate.sh`.

## Unreleased

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
