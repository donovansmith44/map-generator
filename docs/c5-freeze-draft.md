# C5 freeze draft — the map-serving surface (map → atlas)

Status: **DRAFT for sign-off** (owner + atlas session). Nothing here is
frozen until both sign. Once frozen, changes to the listed types are
additive-only behind new optional fields/variants, with a version bump
and a fresh sign-off.

Prepared 2026-08-25 against map-generator `a8f19b4`, atlas exports
pinned at root `a1b93a3b049a1fe0` (atlas commit 1377398).

## What C5 is

C5 is the contract by which the atlas (or any consumer) asks the map
system for scenes and animations. It is SEMANTIC: the consumer receives
meaning (regions, boundaries, labels, transition verbs), never markup.
Concrete bytes are terminal encoder business (law 11) and are explicitly
NOT part of this freeze — a new format never touches this surface.

## The frozen surface

All types live in `map-types` (the zero-dependency contract crate,
importable without pulling adapters, providers, or encoders).

### 1. The front door — `RenderQuery`

```
RenderQuery {
    subject:  RenderSubject,        // World | Region(RegionId) | …
    time:     TimeSelector,         // At(TimePoint) | Over(Interval)
    viewport: Option<Bbox>,
    lod:      Lod,                  // simplification tolerance, degrees
    layers:   LayerSet,             // bit-set, closed vocabulary
    style:    StyleId,              // content-addressed style
}
```

- `RenderQuery` is `MapAddressed`: its pid is the cache key. Same
  query, same world → byte-identical scene (law 1). The `map-cli`
  artifact files are named by exactly this discipline.
- `LayerSet` closed vocabulary as of this draft:
  `GEOMETRY=1, TOPOGRAPHY=2 (water), LABELS=4, RELIEF=8, JOURNEYS=16`.
  **RELIEF and JOURNEYS are additions since the handoff spec** —
  RELIEF (phase 5, hypsometric elevation bands) and JOURNEYS (the
  whole-Bible itinerary layer: Way boundaries plus their gazetteer
  stations); consumers that never set a bit never see that layer.
- Time is `TimePoint` — the atlas covenant's own type (C1); B.C./A.D.
  display is consumer-side, Anno Mundi conversion stays under the hood.

### 2. The answer — `Snapshot`

```
Snapshot {
    regions:     Vec<StyledRegion>,    // rings + paint + sources + class
    boundaries:  Vec<StyledBoundary>,  // pts + stroke + sources
    markers:     Vec<StyledMarker>,
    labels:      Vec<PlacedLabel>,     // subject-linked, never free-floating lies
    attribution: BTreeSet<SourceId>,   // rides EVERY response
}
```

- **Additions since the handoff spec**, all in the shipped types:
  - per-element `sources: BTreeSet<SourceId>` on regions and
    boundaries (honesty at element grain, not just scene grain);
  - `RegionClass` on regions: `Land | Water | Terrain(band)` — the
    seas and the relief are first-class explorable regions, and a
    consumer can filter classes it does not want;
  - labels carry `LabelSubject` (Region/Boundary/Free) so selection
    and hover map back to subjects.
- Provenance and justification stay on the timeline side, reachable
  through region/boundary ids; the scene stays lean.

### 3. The animation — `TransitionScript`

```
TransitionScript { steps: Vec<TransitionStep> }
TransitionStep =
    Morph { boundary, from_pts, to_pts }   // equal counts, slerp pairs
  | FadeIn { region } | FadeOut { region }
  | SplitAlong { parent, seam, children }
  | MergeAcross { parents, child }
```

- Scripts are a monoid: `transition(t,t)` is the empty script,
  sequencing is associative, composed scripts agree with the direct
  one (law 3). Topology change keeps its own verb — a split is never
  animated as a morph.

### 4. The provider — `MapProvider`

```
trait MapProvider {
    fn render(&self, q: &RenderQuery) -> Result<Snapshot, MapError>;
    fn transition(&self, from: TimePoint, to: TimePoint, viewport: Bbox, lod: Lod)
        -> Result<TransitionScript, MapError>;
    fn subjects(&self, at: TimePoint) -> Vec<SubjectListing>;
    fn changes_between(&self, from: TimePoint, to: TimePoint) -> Vec<&ChangeEvent>;
}
```

- **`subjects()` and `changes_between()` are additions since the
  handoff spec** — the workbench's subject picker and scrubber stops
  are built on them and any atlas UI will want the same.

### 5. The encoder boundary — `SceneEncoder` / `TransitionEncoder`

```
trait SceneEncoder      { type Output; fn encode(&self, &Snapshot) -> Result<Output, EncodeError>; }
trait TransitionEncoder { type Output; fn encode_transition(&self, &TransitionScript) -> Result<Output, EncodeError>; }
```

- Terminal (law 11): no format name upstream (grep-enforced),
  deterministic bytes, no post-encode composition. Current backends
  (SVG, GeoJSON, transition JSON) are REFERENCE IMPLEMENTATIONS, not
  contract; the atlas may negotiate any backend without a C5 change.

## Reference serving (non-contractual, for orientation)

The workbench exposes the surface over HTTP —
`/api/{meta,subjects,changes,render,overlay,region_times,transition}` —
and `map-cli` writes the same routes' bytes as content-addressed files
with a manifest. Both are consumers of C5, not part of it.

## C6 rider (already live)

Every timeline carries `AtlasPin { version_root }`; the vendored atlas
exports carry `atlas_version_root`; mismatch fails loud, never serves
stale borders silently. Artifact filenames include the world pin, so a
re-vendored atlas visibly renames every artifact.

## Open items riding this freeze

1. **CHRON-CONV-1** (atlas-side convention ruling on span-end vs
   span-begin adoption for falls/rises): ~11 binding-audit rows wait on
   it; one re-vendor closes them. The freeze does not depend on the
   ruling — dates flow through C2 regardless.
2. **EXPORT-HASH-1** (content_hash on export artifacts): accepted
   low-priority atlas-side; when it lands, `load_exports` verifies it.
3. Stand-in waypoint coordinates remain under standing review (C3
   binding replaces them place-by-place as the gazetteer grows).

## Sign-off

- [ ] Owner (Donovan)
- [ ] Atlas session (C5 consumer)
