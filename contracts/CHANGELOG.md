# Contract suite changelog

The suite is **pre-release (0.x)**. Breaking change to an existing feature or
blessed fixture = MINOR bump; additive = PATCH. v1.0.0 is the owner's act,
never automatic. Enforced by `scripts/contract-semver-gate.sh`.

## 0.3.0 — in progress (Stage 1 continued; final entry lands when the stage closes)

Breaking: the bless phase (`5609ecd`) re-blessed three published fixtures —
`contract.json`, `scene-1405-full.json`, `scene-1405-nolabels.json` — which
had gone stale against the per-(piece,paint) buffer split of Task 10. The
content of a blessed fixture changed, which is breaking under this file's
rule even though the change is a correction.

- New features, additive: `scene/label-placement.feature` (`b02fcd4`,
  renamed `2674ab1`, gained a `Background:` in `31f46e8`) states that a
  label arrives with the position it is drawn at, so two viewers given the
  same answer draw the same names in the same places. Three of its five
  laws are `@target` — the answer carries no placement yet; the steps that
  judge that work landed in `bda17d5`. `golden-gate/golden-gate.feature`
  (`037123c`) turns the golden gate's own required behaviour into laws: it
  must be able to fail, and must say which way.

- Edited features, no law changed: `camera.feature`, `detail.feature`,
  `scene.feature`, `transition.feature` lost 35 `# Characterization:`
  comments (`2674ab1`) under the owner's ruling that feature files carry no
  comments unless they explain why something exists. One such "why" was
  preserved by moving it into `scene.feature`'s prose. The gate names these
  files as edited; no scenario, step, or hole changed.

- The runner gained `Background:` as a first-class part of the feature AST
  (`31f46e8`, fix rounds `c737e6f`/`ad9b112`/`544bf47`), so shared setup is
  stated once while each law keeps its own scenario and its own verdict.

Bookkeeping note, and a question for Task 17: 0.2.0's entry said the version
was "bumped mid-stage so pushes clear the semver gate". The same pressure
produced this bump, because the gate has no way to express "this version is
still open" — any push editing a published file demands a fresh bump, so the
version advances with push cadence rather than with meaning. Stage 1 was
planned to close at 0.2.0; on this trajectory it closes at 0.3.0 or later.
Task 17 should decide whether to fold these entries into one release and
whether the gate should learn about an open version.

## 0.2.0 — superseded mid-stage by 0.3.0 (Stage 1)

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
