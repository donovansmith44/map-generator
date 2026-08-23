# Map-System Handoff — founding prompt + type sketches
(controller-authored 2026-08-23, for the owner to hand to an
independent Fable session; the map system gets its own repository)

---

## A. The founding prompt (copy-paste, edit to taste)

You are founding a new system: a historical map-generation engine. It
is a SEPARATE repository from the Bible Atlas, solving the general
problem; the atlas (and future exploration tools: world history,
literature) will be consumers that QUERY it.

THE VISION (owner's words, binding): "snapshots of maps with arbitrary
granularity with respect to time (so tons and tons of snapshots) of an
arbitrary chunk of the world with time-accurate borders for the chunk
of the world. you have a set of borders that essentially describe the
world, and a layer where we can add topographic style, another layer
where we can add artistic style, ability to overlay maps in a clean
fashion, ability to elegantly animate transitions between snapshots."

END STATE this serves: a suite of offline exploration tools on an
open-source tablet — no internet — interpreted through a biblical
lens. Offline-first is architecture, not a feature: snapshots must be
pure functions of content-addressed data, so any needed set can be
precomputed and shipped as files.

THE WORKING COVENANT (proven on the sibling Bible Atlas project —
reuse it wholesale):
1. TYPES FIRST. Before any rendering or ingestion code: write the
   domain types as a zero-dependency Rust crate, make it compile, and
   get owner review. The compiled crate is the authority; where prose
   and crate disagree, the crate wins. If monads/typeclasses can't be
   used everywhere without compile failure, the design is wrong.
2. CONSULT FIRST on compile failures in the types crate — never
   "fix" types unilaterally.
3. LAWS AS TESTS. Every design invariant is a test in the crate (see
   §D). If the laws pass, the design composes.
4. JUSTIFICATION EVERYWHERE. A border claim cites scholarship exactly
   like a date claim. Every boundary, every change-event carries
   justification (optional prose + set of grounds) and provenance.
   Imported vs authored confidence classes. No fabricated precision.
5. HONESTY RENDERS. Ancient borders were mostly frontier gradients,
   not lines. Uncertainty is a first-class, RENDERED property (fuzzy
   edges, hatched marches, "unknown" styling) — never a false crisp
   line, never hidden.
6. DELTAS, NOT STATES. Borders change at EVENTS (conquest, treaty,
   exile). Store boundary histories + typed change events; a snapshot
   at time t is a DERIVED, deterministic query result — never a
   hand-maintained artifact. "Tons of snapshots" = tons of QUERIES,
   cached by content hash.
7. REVERSIBILITY BY INTERFACE. Every subsystem (source adapters,
   storage, renderer, serving) sits behind an interface we can back
   out of. Extensibility by adapter. OUTPUT is the archetypal unknown
   (owner: "i don't know what's gonna work") — the scene model is
   semantic and format-free; concrete formats are terminal encoder
   backends (§B, §D law 11), swappable after real tablet testing.
8. STANDING ORDERS: report the LOC delta at the end of each work
   unit; if code isn't needed to make a test pass, delete it;
   traditional chronology (the atlas's dating doctrine) governs any
   dates displayed.
9. Dates use the sibling project's chrono vocabulary (Year as
   non-zero i32, negative = BC; TimePoint { year, month?, day? }) so
   the two systems never argue about time.
10. ALGEBRAIC COMPOSITION OVER INHERITANCE (owner order, 2026-08-23:
   "we value algebraic composition over inheritance - we don't make
   fragile systems and everything has a composable interface"). No
   subtype hierarchies, no downcasting, no god objects. The design
   vocabulary is: SUM AND PRODUCT TYPES for data (EdgeCharacter,
   BoundarySource, ChangeKind are enums, never class families);
   TRAITS WITH LAWS for behavior (a trait without law tests is just
   hope); PURE FUNCTIONS for derivation (materialization = select ∘
   simplify ∘ style — each stage total, each stage testable alone);
   ALGEBRAS for the operations users feel:
     - transitions COMPOSE: identity (t→t is empty), associative,
       endpoint-agreeing (§D law 3) — a category, and the tests say so;
     - scenes form a MONOID under overlay: empty scene is identity,
       overlay is associative (§D law 8) — "overlay maps in a clean
       fashion" IS this monoid, not a special feature;
     - styles are DATA that merge predictably, never subclassed
       renderers;
     - queries compose: a snapshot of a sub-viewport at coarser LOD
       is derivable from the same timeline by the same pure function —
       no privileged code paths.
   The sibling atlas is the proof this works: its exploration state is
   a lawful monad (focus/bind, laws as tests), its relation manifest
   is one macro with a dual that is involutive BY SHAPE, and none of
   it inherits from anything. When a design urge says "base class,"
   the answer is a trait with laws or a sum type. Fragility is what
   this rule exists to kill: pieces must snap together because the
   laws guarantee it, not because a hierarchy hopes it.
11. BIBLE-DRIVEN AUTHORITY (owner order, 2026-08-23): "border
   locations and times and everything needs to be bible driven
   wherever possible." Authority flows FROM Scripture THROUGH the
   Bible Atlas graph INTO the map. Where Scripture speaks to a border,
   a waypoint, or a date, the map system consumes the ATLAS's fact
   (with its Scripture grounds and its traditional chronology) — it
   never re-derives or re-dates independently. Extra-biblical
   scholarship fills only what Scripture is silent on, always carried
   under distinguishable Source grounds. This is enforced by the
   CONTRACT SET in §C and by the Bible-preference law in §D.

FIRST SOURCE: aourednik/historical-basemaps (GitHub) — world border
GeoJSONs across historical epochs; already license-vetted and used by
the atlas demo pipeline. First milestone: ingest it behind an
adapter, serve deterministic snapshots of it, and be HONEST that its
epoch granularity is coarse — finer granularity arrives by adding
sources/events, not by interpolating fiction.

PHASES: (0) brainstorm -> spec (architectural path, owner-gated);
(1) types crate + laws; (2) historical-basemaps adapter + fidelity
law; (3) snapshot materializer + determinism law; (4) THE VIEWER (see
below) over the materializer — from here on, every phase is visible;
(5) style layers; (6) transitions (slerp morphs + typed topology
changes); (7) serving (file artifacts first, HTTP optional); (8) the
atlas handshake — the RenderQuery/MapProvider contract below is
co-owned with the atlas session: FREEZE it with the owner before
serving work begins.

THE VIEWER (owner, 2026-08-23: "we definitely want some kind of UI to
view the outputs though"): the map repo ships its own thin viewer — a
WORKBENCH, not a product (the atlas and its siblings remain the
product UIs). One page: a RenderSubject picker, a time scrubber
driven by changes_between (scrub stops AT change events — the
piecewise-constant timeline made visible), a style switcher, an
encoder switcher, an overlay scratchpad (compose any two outputs and
SEE the monoid). DOGFOOD LAW: the viewer consumes ONLY the public
contract (MapProvider + SceneEncoder) — zero privileged internal
access — so the viewer's very existence continuously proves the
contract an external consumer will live on. Keep a standing local
demo of it running for owner review (the atlas's port-8080 demo
discipline transfers).

---

## B. Type sketches (seed material for phase 1 — sketches, not law;
## the founding session's brainstorm may amend WITH owner review)

```rust
// ---------- time (shared vocabulary with the atlas) ----------
pub struct Year(/* NonZero i32; negative = BC */);
pub struct TimePoint { pub year: Year, pub month: Option<u8>, pub day: Option<u8> }
/// Validity of a fact. `to: None` = open (current edge of knowledge).
pub struct Interval { pub from: TimePoint, pub to: Option<TimePoint> }

// ---------- geometry: slerp-ready from birth ----------
/// Points are UNIT VECTORS on the sphere, not lat/lon. This is the
/// atlas's parked border-morph idea promoted to foundation: two rings
/// resampled to equal point counts interpolate by slerp on the GPU —
/// transitions are a data shape, not an afterthought.
pub struct UnitVec { pub x: f64, pub y: f64, pub z: f64 } // |v|=1, checked at parse
pub struct Ring(pub Vec<UnitVec>);                        // closed; winding = containment

/// BOUNDARIES ARE FIRST-CLASS AND SHARED. Two neighboring polities
/// reference ONE boundary arc. Why: (a) no sliver gaps/overlaps by
/// construction; (b) one edit point when scholarship moves a border;
/// (c) a shared arc morphs ONCE and both regions stay consistent.
/// (Per-region closed rings are the simpler alternative the atlas
/// uses today — fine for ingestion, wrong for the general system.
/// Adapter extracts shared arcs from source polygons; that extraction
/// is real work, priced in.)
pub struct BoundaryId(pub ContentHash);
pub struct Boundary {
    pub pts: Vec<UnitVec>,          // open polyline (an arc)
    pub character: EdgeCharacter,   // §honesty
    pub justification: Justification,
    pub provenance: ProvenanceId,
}

/// Honesty as a type: what KIND of edge is this?
pub enum EdgeCharacter {
    Line,                              // genuinely attested precise line (river, wall)
    Frontier { width_km: f64 },        // gradient of control — render as zone
    Disputed { claimants: Vec<RegionId> },
    Unknown,                           // scholarship silent — render distinctly, never invent
}

/// A region (polity/province/realm) is a labeled area whose geometry
/// is a cycle of oriented boundary references.
pub struct RegionId(pub ContentHash);   // content-addressed like atlas Pids
pub enum Orientation { Forward, Reverse }
pub struct RegionGeom { pub cycle: Vec<(BoundaryId, Orientation)>, pub holes: Vec<Vec<(BoundaryId, Orientation)>> }

// ---------- the temporal model: deltas, not states ----------
pub struct BoundaryHistory { pub versions: Vec<(Interval, Boundary)> }  // piecewise; intervals disjoint
pub struct RegionHistory   { pub label_history: Vec<(Interval, String)>, pub geom_history: Vec<(Interval, RegionGeom)> }

/// Border change IS an event — the same ontology as the atlas's
/// chronology (placements drive dates; deltas drive borders).
pub enum ChangeKind {
    Rise    { region: RegionId },
    Fall    { region: RegionId },
    Shift   { boundary: BoundaryId },                       // geometry moved
    Split   { parent: RegionId, children: Vec<RegionId>, seam: Vec<UnitVec> },
    Merge   { parents: Vec<RegionId>, child: RegionId },
    Rename  { region: RegionId },
}
pub struct ChangeEvent {
    pub at: TimePoint,
    pub kind: ChangeKind,
    /// BIBLE-DRIVEN (C4): when this change corresponds to a
    /// Scripture-attested event, `driver` carries the ATLAS's EventId
    /// and `at` MUST equal the atlas's resolved placement for it —
    /// the map never re-dates what the Word (via the atlas's
    /// traditional chronology) already dates. Extra-biblical changes
    /// leave driver = None and carry Source grounds, disclosed.
    pub driver: Option<AtlasEventRef>,  // atlas event id + version root
    pub justification: Justification,   // Ground::Scripture(range) preferred; Source fallback
    pub provenance: ProvenanceId,
}

/// BIBLE-DRIVEN BORDERS (C3+C4): Scripture contains literal border
/// SURVEYS — NUM 34:1-12 (the promised land's borders, specified by
/// God to Moses), JOS 15:1-12 (Judah's allotment, waypoint by
/// waypoint), JOS 16-19 (the rest), 2KI 14:25 (Jeroboam II RESTORES
/// the border "from the entering of Hamath unto the sea of the
/// plain" — a border CHANGE with attestation, date, and waypoints).
/// A survey-derived boundary is CONSTRUCTED from the text: its
/// waypoints are atlas PlaceIds (the atlas gazetteer owns
/// coordinates), its interpolation between waypoints is the only
/// authored geometry (terrain-following, disclosed method), and its
/// justification grounds are the survey verses themselves.
pub struct BorderSurvey {
    pub verses: BibleLocusRange,          // atlas locus type (C1)
    pub waypoints: Vec<AtlasPlaceRef>,    // atlas PlaceIds, in text order
    pub interpolation: InterpolationMethod, // e.g. Geodesic | TerrainValley | Coast
    pub provenance: ProvenanceId,
}
pub enum BoundarySource {
    Survey(BorderSurvey),                 // Bible-driven: the text IS the border
    Imported { source: SourceId },        // scholarship fills silence, labeled
    Authored { justification: Justification },
}

pub struct WorldTimeline {
    pub boundaries: BTreeMap<BoundaryId, BoundaryHistory>,
    pub regions:    BTreeMap<RegionId, RegionHistory>,
    pub events:     Vec<ChangeEvent>,   // the narrative of the map
}

// ---------- the query contract (THE SEAM — co-owned with the atlas) ----------
pub struct Bbox { /* spherical rectangle or cap */ }
pub struct Lod(pub f64);                 // simplification tolerance; law: higher never ADDS points
bitflags-ish LayerSet { GEOMETRY, TOPOGRAPHY, LABELS }   // style is separate, below

pub struct SnapshotQuery {
    pub at: TimePoint,
    pub viewport: Bbox,
    pub lod: Lod,
    pub layers: LayerSet,
    pub style: StyleId,
}
impl ContentAddressed for SnapshotQuery { /* query hash = cache key = artifact filename (OFFLINE STORY) */ }

/// A snapshot is a SEMANTIC SCENE — styled geometry + labels +
/// attribution with NO commitment to any encoding. Consumers (the
/// atlas) composite their own overlays (markers, arrows) on top, and
/// ALL composition (the overlay monoid, accumulation folds) happens
/// HERE, at the semantic level — never on encoded bytes.
pub struct Snapshot {
    pub regions: Vec<StyledRegion>,      // simplified geometry + style-resolved paint
    pub labels:  Vec<PlacedLabel>,
    pub attribution: BTreeSet<SourceId>, // licensing rides every response
}

// ---------- OUTPUT DECOUPLING: encoders are terminal ----------
// Owner (2026-08-23): "im not sure what kind of output we're
// expecting. we need to be decoupled from particular output formats
// because i don't know what's gonna work."
//
// So the model COMMITS TO NO FORMAT. Scene is the last semantic
// type; every concrete format — SVG, GeoJSON, vector tiles, raster
// PNG/tiles (a weak tablet GPU may well want pre-rasterized output!),
// WebGL buffers, PDF plates for print — is an ENCODER BACKEND behind
// one trait. Adding a format touches nothing upstream; killing one
// loses nothing but itself (P7 at the output edge). Performance
// testing on the actual tablet DECIDES the format later; the
// architecture refuses to decide it now.
pub trait SceneEncoder {
    type Output;
    fn encode(&self, scene: &Snapshot) -> Result<Self::Output, EncodeError>;
}
// impls: SvgEncoder, GeoJsonEncoder, VectorTileEncoder,
// RasterEncoder { dpi }, GlBufferEncoder, ... — each self-contained.
// TransitionScript gets the same treatment: a semantic script,
// encodable to CSS/WAAPI animations, GL interpolation buffers, or a
// pre-rendered frame sequence, per backend.

pub trait MapProvider {
    fn snapshot(&self, q: &SnapshotQuery) -> Result<Snapshot, MapError>;
    fn transition(&self, from: TimePoint, to: TimePoint, viewport: Bbox, lod: Lod)
        -> Result<TransitionScript, MapError>;
    fn changes_between(&self, from: TimePoint, to: TimePoint) -> Vec<&ChangeEvent>; // scrubber UIs
}

// ---------- transitions: slerp + typed topology ----------
pub enum TransitionStep {
    Morph   { boundary: BoundaryId, from_pts: Vec<UnitVec>, to_pts: Vec<UnitVec> }, // equal counts; slerp pairs
    FadeIn  { region: RegionId },
    FadeOut { region: RegionId },
    SplitAlong { parent: RegionId, seam: Vec<UnitVec>, children: Vec<RegionId> },
    MergeAcross{ parents: Vec<RegionId>, child: RegionId },
}
pub struct TransitionScript { pub steps: Vec<TransitionStep> }
// Continuous drift MORPHS; topology change gets its own honest verb —
// Alexander's empire does not "morph" into the Diadochi, it SPLITS.

// ---------- ACCUMULATION VIEWS: long-exposure maps ----------
// Owner (2026-08-23): "overlays of maps illustrate multiple borders:
// visualizing the accumulation of changes over a period of time."
//
// This is the overlay monoid EARNING ITS KEEP. An accumulation over
// an interval is the FOLD of overlay across the snapshots inside it:
//
//     accumulate([t1..tn]) = fold(overlay, empty, snapshots)
//
// Three properties make it cheap and exact:
//   1. The timeline is PIECEWISE-CONSTANT (deltas-not-states), so the
//      only distinct snapshots in an interval are the ones AT its
//      ChangeEvents — sample there, plus the endpoints, and the
//      accumulation is EXACT (uniform time-ticks would alias: miss a
//      short-lived kingdom, or redundantly re-render an unchanged
//      century).
//   2. Content addressing DEDUPS for free: identical snapshots hash
//      identically; the fold touches each distinct scene once.
//   3. Temporal depth is STYLE, not machinery: age->paint mapping
//      (older = fainter / hue-banded / hatched) is a Style parameter,
//      so "the expansion of Rome" and "the shrinking of Judah" are
//      the SAME query with different palettes.
//
// DUALITY worth preserving in the design: an accumulation is the
// STILL form of a transition — same change events, two renderings.
// transition : time-lapse :: accumulation : long-exposure photograph.
//
// Bible-driven showcase figures this exists for: the land PROMISED
// (GEN 15:18) over ALLOTTED (JOS 15-19) over REALIZED (1KI 4:21)
// over DIVIDED over EXILED — covenant history in one still image;
// Daniel's four kingdoms (DAN 2/7) as one accumulated overlay.

pub struct AccumulationQuery {
    pub over: Interval,
    pub sampling: SamplingPolicy,     // AtChangeEvents (default, exact) | Explicit(Vec<TimePoint>)
    pub viewport: Bbox,
    pub lod: Lod,
    pub style: StyleId,               // must define the age->paint mapping
    pub subject: Option<RegionId>,    // focus one polity's story, or the whole viewport's
}
impl ContentAddressed for AccumulationQuery { /* same cache/offline story as SnapshotQuery */ }
// Result is a Snapshot (the monoid is closed — an accumulation IS a
// scene, so accumulations overlay onto other scenes like anything else).

// ---------- THE RENDERABLE UNIVERSE ----------
// Owner (2026-08-23): "any node in the graph can be acted upon to
// give us an output from the system. (from a point in space, to a
// border of one location, to a topographic border of a location, to
// the whole world.)" — and "borders and deltas are style-able as
// well."
//
// This is the atlas's own P4 (presentation = selection) arriving on
// the map side: the system does not have a privileged "draw the
// world" path plus special cases — EVERY node of the map graph is a
// query subject, and every output is a Scene (the monoid carrier), so
// every output overlays with every other. The atlas's Presentable
// (display policy per kind × context) has its mirror here: STYLE
// RULES ARE KEYED PER SUBJECT KIND — a Style carries stroke rules for
// boundaries (per EdgeCharacter), paint for regions, marker/label
// rules for points, emphasis rules for DELTAS (a ChangeEvent renders:
// before-stroke, after-stroke, the seam of a split — "what changed at
// the fall of Samaria" is a scene, not a caption).

pub enum RenderSubject {
    Point(AtlasPlaceRef),          // a place: styled marker in period dress
    RawPoint(UnitVec),             // an arbitrary point in space
    Boundary(BoundaryId),          // ONE border, alone (JOS 15 as a drawn line)
    Region(RegionId),              // one location's border, fill optional
    RegionTerrain(RegionId),       // the region clipped over topography
    Change(ChangeEventId),         // a DELTA, rendered
    World,                         // everything in the viewport
}
pub enum TimeSelector { At(TimePoint), Over(Interval) }   // snapshot vs accumulation, UNIFIED

pub struct RenderQuery {
    pub subject: RenderSubject,
    pub time: TimeSelector,
    pub viewport: Option<Bbox>,    // None = auto-frame to the subject's own extent
    pub lod: Lod,
    pub style: StyleId,
}
impl ContentAddressed for RenderQuery { /* one cache discipline for every granularity */ }

// SnapshotQuery     == RenderQuery { subject: World, time: At(t), .. }
// AccumulationQuery == RenderQuery { subject, time: Over(interval), .. }
// The founding session should collapse the earlier two types into
// THIS one in the real crate — they are kept above as derivations so
// the algebra is visible, but RenderQuery is the one front door.

// ---------- styles as data ----------
pub struct StyleId(pub ContentHash);
pub struct Style {
    // Rules keyed PER SUBJECT KIND (the atlas's Presentable-per-kind
    // discipline, mirrored): stroke rules per EdgeCharacter for
    // boundaries (Frontier renders fuzzy, Unknown renders distinctly),
    // region paint + age->paint mapping (accumulations), point/label
    // typography, DELTA emphasis (before/after strokes, split seams),
    // topo shading params. Artistic layer = parameters, never baked
    // pixels — borders and deltas are as style-able as regions.
}
```

---

## C. THE CONTRACT SET (Bible Atlas <-> map generator; owner order:
## "there needs to be a set of contracts between bible atlas and the
## map generator")

Direction of authority is explicit in each. All six are named,
versioned, and frozen with the owner before serving code exists on
either side.

- **C1 — THE LIBRARY** (owner ruling 2026-08-23: "we'll probably just
  need to create a library that this thing can use... reusing the
  stuff that we have in our system"): the map repo takes
  atlas-graph-types as a dependency (git/path dep; zero-dep crate,
  compiles anywhere, vendorable for offline) and REUSES:
    - chrono: Year/TimePoint/ResolvedPlacement + temporal_order
    - Scripture loci: VerseRef/BibleLocus/BibleLocusRange
    - Justification/Ground (+ Provenance/Confidence from ingest)
    - ContentAddressed/Pid/ContentHash (the identity discipline)
    - the kind_tags! and relations! MACROS — the map system declares
      its OWN node kinds (Region/Boundary/Style/...) and its OWN
      relation manifest (bounded-by/driven-by/...) with the same
      machinery, getting phantom-typed ids and the paired-label laws
      for free.
  What does NOT transfer yet: Graph/Holdings/store machinery (typed
  over the ATLAS's own NodeKind/Position — making them
  domain-generic is a real generics pass, not a file move). RULE OF
  THREE: when a third consumer (the world-history tools) appears, we
  split a domain-generic `graph-core` crate out; deliberately
  deferred (YAGNI + P7 — the seam stays open, the covenant NAMES are
  semver-frozen now). Deep observation for the founding session: the
  map system's own data model IS an explorable graph (regions and
  boundaries are nodes, changes are justified edges, snapshots are
  queries) — build it in that idiom and the eventual core-split is a
  move, not a rewrite.
  ATLAS-SIDE PREREQUISITE (small, rides the polish pass):
  #[macro_export] on kind_tags!/relations! + a `covenant` re-export
  module naming exactly the types above.
- **C2 — Chronology authority** (atlas -> map): the atlas's resolved
  placements (traditional chronology, Ussher-anchored, justification-
  hashed) are THE dates for every Scripture-attested change. The map
  system imports a chronology export (event id -> ResolvedPlacement)
  as an adapter source with provenance "bible-atlas@<version-root>",
  and NEVER dates an attested event independently.
- **C3 — Gazetteer** (atlas -> map): Place nodes (id, canonical name,
  lat/lon, aliases, attestations) are the coordinate authority.
  Survey waypoints reference PlaceIds; a place moving in the atlas
  moves every border built through it (one fact, one home).
- **C4 — Event drivers** (atlas -> map): ChangeEvent.driver links a
  border change to the atlas Event that caused it; date equality with
  the atlas placement is a LAW (§D law 11a). The exile, the conquest,
  the fall of Nineveh — the map narrates the same events the reader
  reads.
- **C5 — Map serving** (map -> atlas): RenderQuery / Snapshot /
  TransitionScript / MapProvider (§B). SEMANTIC scenes, attribution
  riding every response, content-addressed caching. The wire/file
  ENCODING of a scene is negotiated per consumer via SceneEncoder
  backends and may change without touching this contract — format
  names never appear upstream of the encoder boundary (§D law 11).
- **C6 — Version drift** (both): every map artifact records the atlas
  version root it compiled against; an atlas chronology/gazetteer
  change flips a fail-loud "stale against atlas@X" flag on the map
  side rather than silently serving outdated borders (the atlas's own
  verified-cache lesson, applied across repos).

## D. The laws (day-one tests in the types crate)

1. DETERMINISM: identical SnapshotQuery -> byte-identical Snapshot
   (the content-address law; the offline tablet depends on it).
2. PARTITION SANITY at every queried t: no region overlap beyond
   declared Disputed; no sliver gaps on shared arcs (the arc-sharing
   payoff, proven not assumed).
3. TRANSITION COMPOSITION: transition(t1,t2) ++ transition(t2,t3)
   ends in the same state as transition(t1,t3).
4. MORPH SAFETY: slerp of matched rings preserves closure and winding.
5. HISTORY COHERENCE: each history's intervals are disjoint and
   ordered; every geometry change at t has a ChangeEvent at t (no
   silent border moves — changes are NARRATED).
6. PROVENANCE TOTALITY: no boundary or event with empty provenance;
   EdgeCharacter::Unknown renders distinctly from Line (honesty is
   testable).
7. LOD MONOTONICITY: coarser tolerance never adds points, and never
   changes topology (a region never vanishes from simplification
   without a declared threshold).
8. COMPOSITION ALGEBRA: scene overlay is a monoid (empty identity,
   associative — overlay(a, overlay(b, c)) == overlay(overlay(a, b),
   c)); transition composition has identity (t→t = no-op script) and
   agrees with law 3's endpoint rule. If these fail, "clean overlay"
   and "elegant transitions" are marketing, not properties.
9. ACCUMULATION EXACTNESS: accumulate(interval) equals the fold of
   overlay across the snapshots at every ChangeEvent in the interval
   plus its endpoints — and adding any sampling point BETWEEN change
   events changes nothing (piecewise-constancy, proven). Accumulation
   of a single-point interval equals its snapshot (fold identity).
10. SELECTION COHERENCE (P4 as a map law): rendering a subject alone
   agrees with selecting it out of the world — render(Boundary(b),
   At(t), style) equals the b-selection of render(World, At(t),
   style) over any viewport containing b, same lod. No privileged
   code path may make the lone rendering and the in-context rendering
   drift.
11. ENCODER TERMINALITY: no type or function upstream of SceneEncoder
   names a concrete output format (grep-enforceable); every encoder is
   deterministic (same Scene + same encoder config -> same bytes, so
   content-addressed caching survives the encoding boundary); and
   composition never happens post-encoding (overlay/accumulate operate
   on Scenes only — encoded artifacts are leaves).
12. BIBLE PREFERENCE (the owner's authority order, testable): (a) a
   ChangeEvent with driver = Some(atlas event) has `at` EQUAL to the
   atlas's resolved placement — byte equality, no re-derivation; (b) a
   boundary whose region/interval is covered by a Scripture survey
   (BoundarySource::Survey) may not be silently overridden by an
   Imported source — an Imported boundary in survey-covered territory
   is a law violation, not a preference; (c) every Survey waypoint
   resolves to a live atlas PlaceId (referential integrity across
   repos, checked against the pinned atlas version).

## E. The shared seam (both sessions, via the owner)

RenderQuery / Snapshot / MapProvider / TransitionScript /
SceneEncoder are CO-OWNED. The atlas will hold a mirror trait behind
its P7 client seam and swap its current map only when the new system
serves the existing era rings equal-or-better. Freeze the contract
with the owner before either side builds serving code against it.
Chrono vocabulary (Year/TimePoint) must stay field-compatible with
atlas-graph-types::chrono.

ATLAS-SIDE OBLIGATIONS (owner, 2026-08-23: "we'll probably need some
ports and adapters bible-atlas-side to ingest... but that's later" —
recorded, deliberately deferred):
1. NOW-ish (rides the atlas polish pass): #[macro_export] on
   kind_tags!/relations! + the `covenant` re-export module (C1).
2. AT MAP-SYSTEM HANDSHAKE: chronology + gazetteer EXPORTS (C2/C3),
   adapter-shaped, versioned by the atlas graph root.
3. LATER (at swap time, not before): atlas-side INGESTION ports +
   adapters for map outputs — a MapProvider mirror port behind the
   existing P7 client seam, plus a scene-ingestion adapter if map
   artifacts ship as files for offline. Same ports-and-adapters
   discipline as every atlas source; nothing built until the map
   system has something worth ingesting.

## F. Open questions for that session's brainstorm (deliberately
## unresolved here)

1. Arc-sharing vs per-region rings at INGESTION (arcs recommended for
   the core; the extraction cost is the trade).
2. Topography source + licensing (relief data is its own sourcing
   problem; the atlas's demo basemaps are style prototypes only).
3. Storage format for the compiled timeline artifact (the atlas's
   FORMAT_VERSION + fail-loud gate pattern transfers).
4. Label placement: precomputed per-snapshot vs consumer-side.
5. Repo name (owner's call).
