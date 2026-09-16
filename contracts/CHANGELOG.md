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

- Breaking, wire: `Snapshot` gains an `inscriptions` collection, and
  `LabelSubject` gains a `Memory` variant wire-prefixed `memory:`
  (`6d706df`). A remembered place — Sodom beneath the south basin — is
  its own kind all the way to the wire, never a marker (a living city's
  dot on a destroyed site) and never mistaken for one. The `/api/scene`
  body carries a new top-level `inscriptions` array. The four
  `scene-1405-*` fixtures are re-blessed for it.

- Behavioral, colors: two territories that touch never wear the same
  paint (`7d96be7`). The palette was assigned once against every
  neighbor an entity ever had across the whole timeline, exhausting
  eight slots and silently reusing a neighbor's; a single moment's map
  never needs more than four. Each entity keeps its home color, repaired
  per moment only where a neighbor present at that moment already wears
  it. Pinned by `scene/region-coloring.feature` — correctness and
  stability both, against the live wire. The same four fixtures move
  with it (4–5 features restyled at 1405 BC).

- Breaking, wire: placement moves into the answer. `/api/scene` asked
  with a camera answers `view` (the chart, camera and page it placed
  for) and every label it sends carries `placement` (its box as page
  fractions) and, for a city, `ground` (`region:HEX` or `unclaimed`);
  labels that cannot be drawn at that view are not sent. Asked with no
  camera it answers `view: null` and places nothing. `dress` gains
  `labelOverflowEm`, the style's declared budget for a land name to
  spill past its shore. `label-placement.feature`'s three placement
  laws go green and lose their `@target`; two new laws, a land's name
  sits on its land and a city names its ground, are stated and met.
  The layout law is written once (map-encoders `layout`) and both the
  SVG frame and the manifest draw from it. The four `scene-1405-*`
  fixtures and `contract.json` (stale at 0.2.0) are re-blessed.
  Breaking, corpus: `camera.feature`'s two nesting laws are restated
  over markers alone (zooming out only reveals markers; zooming in
  never loses a marker you are looking at). Names are not nested this
  way once the answer draws them: a name drawn at one zoom may yield
  at another to a neighbour that grew. `camera.feature`'s "a label is
  only sent when the thing it names is in view" is met by the same
  change and loses its `@target`.

- Behavioral, wire: `/api/resource` and `/api/resources` refuse an id
  the store does not hold BY NAME (404, the body naming the hex id),
  and a batch with one unknown id is refused whole rather than
  shortened in silence. `resources.feature`'s refusal law goes green
  and loses its `@target`.

- Breaking, wire: a scene asked at a camera is cut to the view. A
  geometry entry whose published bounds miss the view cap, or lie
  wholly beyond the horizon, is not sent, nor is a resource nothing
  references; markers and inscriptions are cut per point at the view
  cap clipped to the horizon. The provider drops borders whose own cap
  cannot reach the view under a margin derived from the encoder's
  bounds and ships the rest at the query's detail (the hemisphere
  detail floor is retired: the far world no longer travels at all).
  `camera.feature`'s two-sided culling law and its far-side law go
  green and lose `@target`; the two-sided law is stated over geometry
  entries (feature, resource) since a region's rings may straddle the
  view. Two marker laws are added, one at the Levant camera where this
  canon's markers stand and one anywhere on the globe, so both halves
  of the partition are exercised; the marker nesting laws move to the
  Levant camera for the same reason. `scene-1405-levant-cam` is
  re-blessed.

- Breaking, wire and canon: one thing, one name. Every bridge now
  resolves the id it mints through the entity registry
  (`data/authored/registry.json`, the written unifications the
  registry module was built for; the compile refuses a chain, a
  self-unification or an unknown kind by name), and within one era
  every bundle that resolves to one entity is one area, the held
  witness's layer kept. Phoenicia, Judea and Canaan each become one
  entity where two witnesses drew them. The provider names each region
  entity once per scene however many layers draw it, and each place
  once however many roads pass through it; a settlement's place id is
  spoken without the canon's `place:` namespace (`place:gaza`, marker
  `gaza`, never `place:place:gaza`). `naming.feature`'s two laws go
  green and lose `@target`. Every fixture that carries a region id or
  a place id is re-blessed.

- Behavioral, wire: detail is monotone. A ring below the resolvable
  limit that must still ship for its feature's identity ships as its
  three-point stand-in (the chord's ends and the point farthest from
  it, the first shape simplification itself would draw), never its
  unsimplified ring, so leaning out never carries more vertices than
  leaning in. `detail.feature`'s two monotonicity laws go green and
  lose `@target`; its implicit-detail law now looks through zoom 8,
  the fine tier's own zoom, and goes green. Scene fixtures re-blessed.

- Breaking, wire and corpus: the animation is the scene delta (R84).
  `/api/transition` fades in the regions the destination draws and the
  origin does not, fades out the converse, and morphs a border both
  moments draw differently along its real path: the ring simplified at
  the request's own detail (the zoom's half-pixel rule, the same law
  scenes use, replacing the route's `Lod(6.0)` default) and densified
  by the wire's edge-step law, resampled to no fewer points than the
  border is drawn with. `transition.feature`'s delta law and its
  real-shape law go green and lose `@target`; the plan-versus-timeline
  scenario is removed, as R84 said the delta law replaces it, and the
  runner's step for it goes with it. The conquest and exile plans are
  re-blessed.

- Additive: `scene/wire-flags.feature` and `scene/region-scope.feature`
  characterize what the wire actually does today — four working piece
  switches out of ten named, and `subject=` narrowing only labels.

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
