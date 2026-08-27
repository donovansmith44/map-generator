# The Canon refactor — data model, compiler, dumb renderer, composable API

Status: DESIGN, owner-approved in conversation 2026-08-27; this document
is the written form for review. Supersedes the interval-timeline data
model and every load-time name-matching bind. The rendering LOOK is
kept; the data spine is replaced.

## Why

The engine grew inconsistencies because identity and truth were
scattered: regions were identified by display strings, three witnesses
(atlas exports, hand-authored scripture data, historical-basemaps) made
overlapping claims with no shared spine, and each gap was patched with
another bolt-on filter (bible-mode source filters, attested-nation name
matching, per-feature special cases). Meanwhile the bible-atlas already
curates most of this data as typed, verse-grounded graph nodes.

The owner's ruling: the map app should be **generally dumb** — data in,
beautiful time-stamped map artifacts out, styles as plug-and-chug
templates — and the data it eats must be **canonical sets that cannot
contradict**.

## Shape: compile, then render

```
bible-atlas API  ──refresh──▶  data/atlas-vendor/     (typed, pinned, offline)
historical-basemaps / natural-earth / etopo            (vendored, as today)
data/authored/                                         (surveys, routes,
        │                                               reconciliations — data, not code)
        ▼
   map-compile  ──▶  data/canon/        the content-addressed canonical store
                          │
                          ▼
   map-render(Canon, RenderRequest) ──▶ SVG / GeoJSON   (pure, deterministic)
```

- **map-compile** holds all intelligence: source merging, identity,
  reconciliation, validation. Runs before serving, never during.
- **map-render** knows nothing about the Bible, basemaps, or witnesses.
  It draws a Canon.

Crate plan: new `map-canon` (types + store + validators), new
`map-compile` (replaces map-adapters' role), `map-render` (the current
provider + encoders, re-pointed at Canon), `map-viewer` (workbench +
CLI, unchanged from the outside).

## Canon, in strict types

```rust
/// Identity: WHO something is, forever distinct from WHAT it looks like.
/// Atlas node ids verbatim ("rome", "egypt", narrative ids); authored
/// and basemap entities carry their witness as a prefix.
struct EntityId(String);            // "rome" | "authored:promise-num34" | "basemap:hittites"

enum FeatureKind { Area, Way, Point }

// Geometry has ONE home. A border is stored once, ever.
struct Vec3   { x: f64, y: f64, z: f64 }     // unit sphere
struct Border(Vec<Vec3>);
struct BorderId(Hash);                        // content hash of Border

struct Area  { entity: EntityId, name: String,
               rings: BTreeSet<BorderId>, holes: BTreeSet<BorderId> }
struct Route { entity: EntityId, name: String, legs: Vec<Leg> }
struct Leg   { from: PlaceId, to: PlaceId, border: BorderId,
               span: (Timestamp, Timestamp) }   // day-granular walks welcome
struct Point { entity: EntityId, name: String, at: Vec3 }

enum Feature { Area(Area), Way(Route), Point(Point) }
struct FeatureId(Hash);                       // content hash of Feature

// TIME: the covenant's own TimePoint — year + optional month + optional
// day, totally ordered, arbitrarily refinable (the crucifixion week and
// a border that moves in two days are representable; the atlas can
// refine granularity further and this type follows). The map side NEVER
// flattens a Timestamp to a bare year again — bare-year fields are a
// compile error in canon types.
type Timestamp = atlas_graph_types::covenant::TimePoint;

// The world through time: a SET of (timestamp, snapshot) pairs — not a
// list; no ordering is baked into the data. It is keyed by Timestamp
// (iteration order derives from Timestamp's total order), and the key
// carries a law: ONE world state per instant — two snapshots at the
// same timestamp is a contradiction the type cannot express.
struct Snapshot { features: BTreeSet<FeatureId> }
struct SnapshotId(Hash);
struct World    { moments: BTreeMap<Timestamp, SnapshotId> }

// Canonical sets that cannot contradict: layers.
enum LayerKind { Territory, ScriptureClaims, Journeys, Water, Relief, Background }
struct Canon {
    layers:     BTreeMap<LayerKind, World>,
    features:   BTreeMap<FeatureId, Feature>,   // hash → object
    borders:    BTreeMap<BorderId, Border>,     // hash → object
    provenance: BTreeMap<FeatureId, Provenance>,// witness, verses, notes
    pin:        AtlasPin,                       // C6: stale data fails loud
}
```

Partial journeys are a **typed time filter**: each `Leg` carries its
span; rendering `At(t)` draws the legs whose span has begun, clipping
the in-progress leg proportionally at the span's own granularity (a
three-day leg clips by days, a forty-year wandering by years);
`Over(a, b)` draws the legs the range covers. No fraction arithmetic
smeared through the renderer.

### Laws (compile-time validators, all fail loud)

1. **No self-contradiction:** within `Territory`, no two Areas overlap
   at any moment. Overlap across layers is meaning, not contradiction
   (the promise over the kingdoms; a journey over an empire).
2. **One home:** every Border and Feature is stored once, addressed by
   content hash; a Snapshot references, never contains.
3. **Closed references:** every id in every snapshot resolves.
4. **Reconciled witnesses:** an entity claimed by more than one witness
   without a reconciliation row fails the compile (below).
5. **Determinism:** identical inputs produce a byte-identical
   `data/canon/`; rendering is a pure function of (Canon, request).

## Sources and reconciliation — nothing discarded silently

```rust
enum Witness { Atlas, Authored, Basemap }
struct Reconcile { entity: EntityId, choose: Witness, note: String }
// data/authored/reconcile.ron — owner-reviewable data, not code
```

- **Atlas (canonical wherever it speaks):** polities with era-ranged
  border rings and verse-grounded transition/fall deltas, narratives
  (journeys) with ordered events, places with attestations, eras,
  land-mask — pulled by `map-cli refresh` from `/api/polities`,
  `/api/narratives`, `/api/place/{id}`, `/api/eras`, `/api/land-mask`
  into `data/atlas-vendor/`, pinned by the atlas version root.
- **Authored (kept where it is genuinely ours):** the NUM 34 / JOS
  15–19 border-survey constructions and any route the atlas lacks.
  Moves out of Rust constants into `data/authored/`.
- **Basemap (demoted to Background):** historical-basemaps keeps
  painting the whole world — the owner's "whole world is explorable"
  rule — but in its own `Background` layer, never mixed into
  `Territory`, so it cannot contradict atlas polities.

The compiler emits a **reconciliation report**: every entity, its
witnesses, their disagreements. The in-flight attested-nations
name-matching work is dropped (stashed); identity-by-EntityId makes
name matching unrepresentable.

## The dumb renderer and plug-and-chug templates

```rust
struct RenderRequest {
    time:     TimeSel,               // At(Timestamp) | Over(Timestamp, Timestamp)
    camera:   Camera,                // Globe { center, zoom } | Flat { center, zoom }
    layers:   BTreeSet<LayerKind>,
    pieces:   Option<BTreeSet<EntityId>>, // None = whole scene; Some = just these
    template: TemplateId,
    width:    f64,
    detail:   Detail,                // Auto (camera-tracked) | Exact(radians)
}
fn render(canon: &Canon, req: &RenderRequest) -> Bytes   // svg or geojson
```

Style templates become **data files** (`templates/parchment.ron`,
`templates/slate.ron`, …): palette slots, per-FeatureKind strokes,
relief ramp, label rules, ghost/tint derivations. A new look is a new
file, no recompile. All current rendering behavior carries over: globe
math, viewport culling, camera-tracked detail, smoothing, label
fitting, journey dress, hit targets.

## The composable public API

The point (owner's words): all these pieces of data, anyone can do what
they want with; we generate interactable images that are also
composable.

```
GET /api/meta                                  # pin, layers, moments, templates, entities count
GET /api/scaffold?camera&width&template        # the STAGE: land, water, relief, graticule — no claims
GET /api/entities?layer&at                     # listing: EntityId, name, kind, active spans
GET /api/features?ids=…&at|from,to&format=geojson|canon   # raw composable DATA
# Timestamps on the wire: "-1450" | "-1450-01" | "-1450-01-14" —
# year, optional month, optional day, matching covenant TimePoint.
GET /api/render?…                              # whole-world snapshot image (as today)
GET /api/render?pieces=rome,egypt&…            # an image LAYER: transparent, only those entities
```

**Composability law:** for a fixed (camera, width), every artifact —
scaffold, piece, whole snapshot — shares the same projection and
viewBox, byte-stable, so stacked piece-SVGs align geometrically over
the scaffold. Every element keeps its interactivity contract:
`data-entity`, `data-place`, and the `data-clat/clon/zoom` view stamp,
documented as part of the API. Callers arrange pieces however they
choose; our own workbench becomes just another consumer.

## Migration plan (each phase lands green before the next)

1. **map-canon**: types + validators + store, test-first.
2. **refresh**: atlas pull → `data/atlas-vendor/`, C6 pin, offline builds.
3. **map-compile**: all witnesses → Canon + reconciliation report; the
   report drives the owner's reconcile.ron pass.
4. **map-render**: current provider/encoders re-pointed at Canon behind
   the same external query surface; workbench and map-cli work
   unchanged; visual parity check against today's plates.
5. **API growth**: scaffold, entities, features, pieces endpoints.
6. **Deletion**: interval-timeline internals, name-matching binds, and
   in-code authored constants removed once parity is confirmed.

## Out of scope

Anything atlas-side (new endpoints, aliases, richer narrative data) —
requested through the atlas session, never edited here. The C5 freeze
draft gets a rider once the new surface stabilizes.
