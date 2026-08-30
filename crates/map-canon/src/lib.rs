//! The canonical data spine (2026-08-27 design): content-addressed
//! borders, features, and snapshots; worlds as timestamp-keyed SETS at
//! covenant granularity; layered canon with fail-loud validators.
//!
//! Laws upheld here (see tests.rs, written first):
//! - one home per fact: identity IS content, dedup by construction;
//! - one world state per instant: a World is a set of (Timestamp,
//!   Snapshot) pairs keyed by Timestamp — a second snapshot at the
//!   same instant is refused, never silently replaced;
//! - closed references: everything a snapshot points at exists;
//! - Territory never self-overlaps at any moment (hard law) — overlap
//!   across layers or across moments is meaning, not contradiction;
//! - a route's legs run forward in time.

use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, BTreeSet};
use std::hash::{Hash, Hasher};

use atlas_graph_types::covenant::{ContentHash, PlaceId, TimePoint};
use map_types::ident::Canon as Bytes;
use map_types::UnitVec;

/// Time, at the covenant's own granularity: year + optional month +
/// optional day, totally ordered, refinable atlas-side. The canon
/// NEVER flattens this to a bare year.
pub type Timestamp = TimePoint;

/// WHO something is, forever distinct from WHAT it looks like. Atlas
/// node ids verbatim ("rome", "egypt", narrative ids); authored and
/// basemap entities carry their witness prefix ("authored:…",
/// "basemap:…").
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntityId(pub String);

macro_rules! content_id {
    ($($(#[$doc:meta])* $name:ident),+ $(,)?) => {$(
        $(#[$doc])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(pub ContentHash);
    )+};
}
content_id!(
    /// Content hash of a Border — the same coordinates are one border.
    BorderId,
    /// Content hash of a Feature.
    FeatureId,
    /// Content hash of a Snapshot.
    SnapshotId,
);

/// An ordered run of coordinates on the unit sphere. Closed rings do
/// not repeat their first point; open ways simply end.
#[derive(Clone, Debug, PartialEq)]
pub struct Border(pub Vec<UnitVec>);

/// A named territorial (or claim) shape: rings + holes by reference.
#[derive(Clone, Debug, PartialEq)]
pub struct Area {
    pub entity: EntityId,
    pub name: String,
    pub rings: BTreeSet<BorderId>,
    pub holes: BTreeSet<BorderId>,
}

/// One leg of a journey: a way between two places, walked over a span
/// of time — day-granular when the text is (the span's endpoints are
/// covenant Timestamps, not years).
#[derive(Clone, Debug, PartialEq)]
pub struct Leg {
    pub from: PlaceId,
    pub to: PlaceId,
    pub border: BorderId,
    pub span: (Timestamp, Timestamp),
}

/// A journey: an entity walking legs in order.
#[derive(Clone, Debug, PartialEq)]
pub struct Route {
    pub entity: EntityId,
    pub name: String,
    pub legs: Vec<Leg>,
}

/// A named point on the sphere (a landmark, a city as a dot).
#[derive(Clone, Debug, PartialEq)]
pub struct Landmark {
    pub entity: EntityId,
    pub name: String,
    pub at: UnitVec,
}

/// A stroked path feature (rivers): the partition's overlay polylines
/// and river-attributed border edges enter the canon as Lines.
#[derive(Clone, Debug, PartialEq)]
pub struct PathLine {
    pub entity: EntityId,
    pub name: String,
    pub border: BorderId,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Feature {
    Area(Area),
    Way(Route),
    Point(Landmark),
    /// An attributed line: a river (or any stroked path) whose
    /// geometry is one border — never a face, never a filled area.
    Line(PathLine),
}

impl Feature {
    pub fn entity(&self) -> &EntityId {
        match self {
            Feature::Area(a) => &a.entity,
            Feature::Way(r) => &r.entity,
            Feature::Point(p) => &p.entity,
            Feature::Line(l) => &l.entity,
        }
    }
    pub fn name(&self) -> &str {
        match self {
            Feature::Area(a) => &a.name,
            Feature::Way(r) => &r.name,
            Feature::Point(p) => &p.name,
            Feature::Line(l) => &l.name,
        }
    }
}

/// One state of one layer's world: a SET of features by id.
#[derive(Clone, Debug, PartialEq)]
pub struct Snapshot {
    pub features: BTreeSet<FeatureId>,
}

/// The layers of the canon. Within Territory, overlap at a moment is a
/// contradiction; across layers, overlap is meaning.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LayerKind {
    Territory,
    ScriptureClaims,
    Journeys,
    Water,
    Relief,
    Background,
}

/// Which witness a feature's truth stands on, with its grounds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Witness {
    Atlas,
    Authored,
    Basemap,
    /// Natural Earth / ETOPO physical base data (coasts, lakes, relief).
    NaturalEarth,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Provenance {
    pub witness: Witness,
    pub verses: Vec<String>,
    pub note: String,
}

// ---------------------------------------------------------- hashing

fn hash_bytes(bytes: &[u8]) -> ContentHash {
    let mut h = DefaultHasher::new();
    bytes.hash(&mut h);
    ContentHash(h.finish())
}

fn canon_timestamp(c: &mut Bytes, t: &Timestamp) {
    c.i32_(t.year.get());
    c.opt(&t.month, |c, m| {
        c.u8_(*m);
    });
    c.opt(&t.day, |c, d| {
        c.u8_(*d);
    });
}

fn border_bytes(b: &Border) -> Vec<u8> {
    let mut c = Bytes::new();
    c.tag("canon-border");
    c.seq(&b.0, |c, p| {
        c.f64_(p.x()).f64_(p.y()).f64_(p.z());
    });
    c.done()
}

fn feature_bytes(f: &Feature) -> Vec<u8> {
    let mut c = Bytes::new();
    match f {
        Feature::Area(a) => {
            c.tag("canon-area");
            c.str_(&a.entity.0).str_(&a.name);
            let rings: Vec<_> = a.rings.iter().collect();
            c.seq(&rings, |c, id| {
                c.u64_(id.0 .0);
            });
            let holes: Vec<_> = a.holes.iter().collect();
            c.seq(&holes, |c, id| {
                c.u64_(id.0 .0);
            });
        }
        Feature::Way(r) => {
            c.tag("canon-way");
            c.str_(&r.entity.0).str_(&r.name);
            c.seq(&r.legs, |c, leg| {
                c.str_(&leg.from.0).str_(&leg.to.0).u64_(leg.border.0 .0);
                canon_timestamp(c, &leg.span.0);
                canon_timestamp(c, &leg.span.1);
            });
        }
        Feature::Point(p) => {
            c.tag("canon-point");
            c.str_(&p.entity.0).str_(&p.name);
            c.f64_(p.at.x()).f64_(p.at.y()).f64_(p.at.z());
        }
        Feature::Line(l) => {
            c.tag("canon-line");
            c.str_(&l.entity.0).str_(&l.name);
            c.u64_(l.border.0 .0);
        }
    }
    c.done()
}

fn snapshot_bytes(s: &Snapshot) -> Vec<u8> {
    let mut c = Bytes::new();
    c.tag("canon-snapshot");
    let ids: Vec<_> = s.features.iter().collect();
    c.seq(&ids, |c, id| {
        c.u64_(id.0 .0);
    });
    c.done()
}

// ---------------------------------------------------------- worlds

/// A world through time: a set of (timestamp, snapshot) pairs KEYED by
/// timestamp. No ordering is baked into the data; iteration order
/// derives from the covenant's total order on Timestamps.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct World {
    moments: BTreeMap<Timestamp, SnapshotId>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WorldError {
    /// Two DIFFERENT snapshots asserted at the same instant.
    ContradictionAt(Timestamp),
}

impl World {
    /// Assert a moment. Re-asserting the identical pair is idempotent;
    /// a different snapshot at an occupied instant is a contradiction.
    pub fn insert(&mut self, at: Timestamp, snapshot: SnapshotId) -> Result<(), WorldError> {
        match self.moments.get(&at) {
            None => {
                self.moments.insert(at, snapshot);
                Ok(())
            }
            Some(existing) if *existing == snapshot => Ok(()),
            Some(_) => Err(WorldError::ContradictionAt(at)),
        }
    }
    pub fn moments(&self) -> &BTreeMap<Timestamp, SnapshotId> {
        &self.moments
    }
    /// The state AT a time: the latest moment at or before it —
    /// piecewise-constant history.
    pub fn state_at(&self, t: &Timestamp) -> Option<SnapshotId> {
        self.moments.range(..=t).next_back().map(|(_, s)| *s)
    }
}

// ---------------------------------------------------------- the store

/// The canonical store: hash → object for every fact, layers of
/// worlds, provenance per feature. Insertion dedups by construction —
/// identity IS content.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CanonStore {
    borders: BTreeMap<BorderId, Border>,
    features: BTreeMap<FeatureId, Feature>,
    snapshots: BTreeMap<SnapshotId, Snapshot>,
    layers: BTreeMap<LayerKind, World>,
    provenance: BTreeMap<FeatureId, Provenance>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum CanonViolation {
    UnresolvedSnapshot { layer: LayerKind, at: Timestamp, snapshot: SnapshotId },
    UnresolvedFeature { layer: LayerKind, snapshot: SnapshotId, feature: FeatureId },
    UnresolvedBorder { feature: FeatureId, border: BorderId },
    /// Two territorial claims on the same ground at the same moment.
    TerritorialOverlap { at: Timestamp, a: EntityId, b: EntityId },
    /// A leg whose span runs backward in time.
    BackwardLeg { feature: FeatureId, index: usize },
}

impl CanonStore {
    pub fn insert_border(&mut self, b: Border) -> BorderId {
        let id = BorderId(hash_bytes(&border_bytes(&b)));
        self.borders.entry(id).or_insert(b);
        id
    }
    pub fn insert_feature(&mut self, f: Feature) -> FeatureId {
        let id = FeatureId(hash_bytes(&feature_bytes(&f)));
        self.features.entry(id).or_insert(f);
        id
    }
    pub fn insert_snapshot(&mut self, s: Snapshot) -> SnapshotId {
        let id = SnapshotId(hash_bytes(&snapshot_bytes(&s)));
        self.snapshots.entry(id).or_insert(s);
        id
    }
    pub fn set_layer(&mut self, kind: LayerKind, world: World) {
        self.layers.insert(kind, world);
    }
    pub fn set_provenance(&mut self, feature: FeatureId, p: Provenance) {
        self.provenance.insert(feature, p);
    }

    pub fn borders(&self) -> &BTreeMap<BorderId, Border> {
        &self.borders
    }
    pub fn features(&self) -> &BTreeMap<FeatureId, Feature> {
        &self.features
    }
    pub fn snapshots(&self) -> &BTreeMap<SnapshotId, Snapshot> {
        &self.snapshots
    }
    pub fn layers(&self) -> &BTreeMap<LayerKind, World> {
        &self.layers
    }
    pub fn provenance(&self) -> &BTreeMap<FeatureId, Provenance> {
        &self.provenance
    }

    /// Every law, checked; an empty vec is a lawful canon.
    pub fn validate(&self) -> Vec<CanonViolation> {
        let mut v = Vec::new();
        let mut seen_features: BTreeSet<FeatureId> = BTreeSet::new();
        for (layer, world) in &self.layers {
            for (at, sid) in world.moments() {
                let Some(snap) = self.snapshots.get(sid) else {
                    v.push(CanonViolation::UnresolvedSnapshot {
                        layer: *layer,
                        at: *at,
                        snapshot: *sid,
                    });
                    continue;
                };
                for fid in &snap.features {
                    if !self.features.contains_key(fid) {
                        v.push(CanonViolation::UnresolvedFeature {
                            layer: *layer,
                            snapshot: *sid,
                            feature: *fid,
                        });
                        continue;
                    }
                    seen_features.insert(*fid);
                }
                if *layer == LayerKind::Territory {
                    self.check_territory_overlaps(*at, snap, &mut v);
                }
            }
        }
        // Per-feature checks, once per referenced feature.
        for fid in &seen_features {
            let f = &self.features[fid];
            for bid in feature_border_refs(f) {
                if !self.borders.contains_key(&bid) {
                    v.push(CanonViolation::UnresolvedBorder { feature: *fid, border: bid });
                }
            }
            if let Feature::Way(r) = f {
                for (i, leg) in r.legs.iter().enumerate() {
                    if leg.span.1 < leg.span.0 {
                        v.push(CanonViolation::BackwardLeg { feature: *fid, index: i });
                    }
                }
            }
        }
        v
    }

    fn check_territory_overlaps(
        &self,
        at: Timestamp,
        snap: &Snapshot,
        v: &mut Vec<CanonViolation>,
    ) {
        let areas: Vec<(&FeatureId, &Area)> = snap
            .features
            .iter()
            .filter_map(|fid| match self.features.get(fid) {
                Some(Feature::Area(a)) => Some((fid, a)),
                _ => None,
            })
            .collect();
        for i in 0..areas.len() {
            for j in (i + 1)..areas.len() {
                let (a, b) = (areas[i].1, areas[j].1);
                if self.areas_overlap(a, b) {
                    v.push(CanonViolation::TerritorialOverlap {
                        at,
                        a: a.entity.clone(),
                        b: b.entity.clone(),
                    });
                }
            }
        }
    }

    fn rings_of<'a>(&'a self, a: &Area) -> Vec<&'a [UnitVec]> {
        a.rings
            .iter()
            .filter_map(|id| self.borders.get(id))
            .map(|b| b.0.as_slice())
            .collect()
    }

    /// Interiors intersect: a vertex of one strictly inside the other,
    /// or a proper edge crossing. Shared edges and shared corners are
    /// peace, not war. Holes are ignored (conservative — documented).
    fn areas_overlap(&self, a: &Area, b: &Area) -> bool {
        for ra in self.rings_of(a) {
            for rb in self.rings_of(b) {
                if rings_overlap(ra, rb) {
                    return true;
                }
            }
        }
        false
    }
}

fn feature_border_refs(f: &Feature) -> Vec<BorderId> {
    match f {
        Feature::Area(a) => a.rings.iter().chain(a.holes.iter()).copied().collect(),
        Feature::Way(r) => r.legs.iter().map(|l| l.border).collect(),
        Feature::Point(_) => Vec::new(),
        Feature::Line(l) => vec![l.border],
    }
}

// ------------------------------------------------- spherical geometry

fn latlon(p: &UnitVec) -> (f64, f64) {
    (p.z().asin().to_degrees(), p.y().atan2(p.x()).to_degrees())
}

fn bbox(pts: &[UnitVec]) -> (f64, f64, f64, f64) {
    let (mut lat0, mut lon0, mut lat1, mut lon1) =
        (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for p in pts {
        let (lat, lon) = latlon(p);
        lat0 = lat0.min(lat);
        lon0 = lon0.min(lon);
        lat1 = lat1.max(lat);
        lon1 = lon1.max(lon);
    }
    (lat0, lon0, lat1, lon1)
}

fn boxes_intersect(a: (f64, f64, f64, f64), b: (f64, f64, f64, f64)) -> bool {
    a.0 <= b.2 && b.0 <= a.2 && a.1 <= b.3 && b.1 <= a.3
}

fn cross(a: &UnitVec, b: &UnitVec) -> (f64, f64, f64) {
    (
        a.y() * b.z() - a.z() * b.y(),
        a.z() * b.x() - a.x() * b.z(),
        a.x() * b.y() - a.y() * b.x(),
    )
}

fn dot3(a: (f64, f64, f64), b: (f64, f64, f64)) -> f64 {
    a.0 * b.0 + a.1 * b.1 + a.2 * b.2
}

fn norm(a: (f64, f64, f64)) -> f64 {
    dot3(a, a).sqrt()
}

fn as_tuple(p: &UnitVec) -> (f64, f64, f64) {
    (p.x(), p.y(), p.z())
}

/// Strictly-inside test by spherical winding: azimuth sweep of the
/// ring's vertices around `p` totals ±2π inside, ~0 outside, ~±π on
/// the boundary — the boundary is deliberately OUT (shared edges are
/// not overlap). Assumes the ring does not enclose p's antipode.
fn point_strictly_in_ring(p: &UnitVec, ring: &[UnitVec]) -> bool {
    if ring.len() < 3 {
        return false;
    }
    // A point coinciding with a ring vertex is ON the boundary, and
    // the boundary is out (shared corners are peace) — it would also
    // make the azimuth sweep degenerate.
    if ring.iter().any(|v| {
        dot3(as_tuple(v), as_tuple(p)) >= 1.0 - 1e-12
    }) {
        return false;
    }
    // Local tangent frame at p.
    let pt = as_tuple(p);
    let seed = if p.x().abs() < 0.9 { (1.0, 0.0, 0.0) } else { (0.0, 1.0, 0.0) };
    let e1 = {
        let d = dot3(seed, pt);
        let raw = (seed.0 - d * pt.0, seed.1 - d * pt.1, seed.2 - d * pt.2);
        let n = norm(raw);
        (raw.0 / n, raw.1 / n, raw.2 / n)
    };
    let e2 = {
        let c = (
            pt.1 * e1.2 - pt.2 * e1.1,
            pt.2 * e1.0 - pt.0 * e1.2,
            pt.0 * e1.1 - pt.1 * e1.0,
        );
        c
    };
    let azimuth = |v: &UnitVec| -> f64 {
        let vt = as_tuple(v);
        let d = dot3(vt, pt);
        let tan = (vt.0 - d * pt.0, vt.1 - d * pt.1, vt.2 - d * pt.2);
        dot3(tan, e2).atan2(dot3(tan, e1))
    };
    let mut total = 0.0f64;
    let mut prev = azimuth(&ring[ring.len() - 1]);
    for v in ring {
        let az = azimuth(v);
        let mut delta = az - prev;
        while delta > std::f64::consts::PI {
            delta -= 2.0 * std::f64::consts::PI;
        }
        while delta < -std::f64::consts::PI {
            delta += 2.0 * std::f64::consts::PI;
        }
        total += delta;
        prev = az;
    }
    total.abs() > 1.5 * std::f64::consts::PI
}

/// Proper great-circle crossing: the segments intersect at a point
/// strictly interior to BOTH arcs. Parallel arcs (shared edges) and
/// endpoint touches (shared corners) are not crossings.
fn segments_cross(a0: &UnitVec, a1: &UnitVec, b0: &UnitVec, b1: &UnitVec) -> bool {
    const EPS: f64 = 1e-12;
    const ANG_EPS: f64 = 1e-9;
    let n1 = cross(a0, a1);
    let n2 = cross(b0, b1);
    let l = (
        n1.1 * n2.2 - n1.2 * n2.1,
        n1.2 * n2.0 - n1.0 * n2.2,
        n1.0 * n2.1 - n1.1 * n2.0,
    );
    let ln = norm(l);
    if ln < EPS {
        return false; // same great circle: collinear touch, not a crossing
    }
    let cand = (l.0 / ln, l.1 / ln, l.2 / ln);
    let strictly_within = |p: (f64, f64, f64), s0: &UnitVec, s1: &UnitVec| -> bool {
        let d0 = dot3(p, as_tuple(s0)).clamp(-1.0, 1.0).acos();
        let d1 = dot3(p, as_tuple(s1)).clamp(-1.0, 1.0).acos();
        let span = dot3(as_tuple(s0), as_tuple(s1)).clamp(-1.0, 1.0).acos();
        d0 > ANG_EPS && d1 > ANG_EPS && (d0 + d1) <= span + ANG_EPS
    };
    for p in [cand, (-cand.0, -cand.1, -cand.2)] {
        if strictly_within(p, a0, a1) && strictly_within(p, b0, b1) {
            return true;
        }
    }
    false
}

/// Interpenetration below this angular depth (degrees) is tracing
/// slop at the sources' own precision, not a contradiction. Measured
/// on real atlas polities 2026-08-27: adjacent-kingdom slivers reach
/// ~0.09 deg; the one real conflict (Babylon/Sumer) penetrates ~1 deg.
pub const TERRITORY_TOLERANCE_DEG: f64 = 0.2;

/// Angular distance (radians) from a point to the nearest point of a
/// closed ring's boundary.
fn distance_to_ring(p: &UnitVec, ring: &[UnitVec]) -> f64 {
    let mut best = f64::MAX;
    for i in 0..ring.len() {
        let (a, b) = (&ring[i], &ring[(i + 1) % ring.len()]);
        let n = cross(a, b);
        let ln = norm(n);
        let d = if ln < 1e-12 {
            p.angle_to(a)
        } else {
            let nn = (n.0 / ln, n.1 / ln, n.2 / ln);
            let off = dot3(as_tuple(p), nn).clamp(-1.0, 1.0).asin().abs();
            // Is the foot of the perpendicular within the arc?
            let d0 = p.angle_to(a);
            let d1 = p.angle_to(b);
            let span = a.angle_to(b);
            let within = (d0 * d0 - off * off).max(0.0).sqrt()
                + (d1 * d1 - off * off).max(0.0).sqrt()
                <= span + 1e-9;
            if within { off } else { d0.min(d1) }
        };
        best = best.min(d);
    }
    best
}

/// Interior intersection of two closed rings, measured by DEPTH: some
/// vertex of one sits inside the other, farther from its boundary
/// than the tracing tolerance. Crossing slivers shallower than the
/// tolerance are peace (disclosed); containment is always deep.
fn rings_overlap(ra: &[UnitVec], rb: &[UnitVec]) -> bool {
    if ra.len() < 3 || rb.len() < 3 {
        return false;
    }
    const MARGIN: f64 = 1e-9;
    let tol_rad = TERRITORY_TOLERANCE_DEG.to_radians();
    let (a0, a1, a2, a3) = bbox(ra);
    let (b0, b1, b2, b3) = bbox(rb);
    if !boxes_intersect((a0 - MARGIN, a1 - MARGIN, a2 + MARGIN, a3 + MARGIN), (b0, b1, b2, b3)) {
        return false;
    }
    let deep = |p: &UnitVec, other: &[UnitVec]| {
        point_strictly_in_ring(p, other) && distance_to_ring(p, other) > tol_rad
    };
    // Probe vertices AND points along each edge: aligned borders put
    // the penetrating vertices exactly ON the other's boundary, where
    // strictness would hide them; edge-interior samples cannot hide.
    let probes = |ring: &[UnitVec]| -> Vec<UnitVec> {
        let mut out: Vec<UnitVec> = ring.to_vec();
        for i in 0..ring.len() {
            let (a, b) = (&ring[i], &ring[(i + 1) % ring.len()]);
            for t in [0.25, 0.5, 0.75] {
                if let Ok(m) = map_types::slerp(a, b, t) {
                    out.push(m);
                }
            }
        }
        out
    };
    probes(ra).iter().any(|p| deep(p, rb)) || probes(rb).iter().any(|p| deep(p, ra))
}

pub mod persist;

#[cfg(test)]
mod tests;
