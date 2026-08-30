//! THE SPHERE PARTITION: one closed spherical arrangement is the
//! geometric authority. Every canonical border exists once; every
//! edge has exactly two sides; every side belongs to one closed face
//! cycle; the faces partition the sphere exactly — so the sum of all
//! face areas is 4π (the completeness law), and gaps or overlaps are
//! impossible by construction, not by lapping tricks.
//!
//! Regions, seas, lakes, and the background are faces. Rivers are an
//! attributed OVERLAY of polylines noded into the same vertex pool
//! (the spec's face-splitting alternative was rejected: real rivers
//! end mid-region at their springs, and a dead-ending edge cannot
//! split a face). A river that runs along a border shares the
//! border's canonical vertices, so the two can never disagree.
//!
//! Witness polygons (traced plates, modern datasets) are EVIDENCE
//! presented to the builder, never canonical geometry themselves.

mod build;
#[cfg(test)]
mod tests;

pub use build::{build, BuildError, WitnessPolyline, WitnessRegion};

use map_types::UnitVec;

// ------------------------------------------------------- primitives

/// Neumaier-compensated summation: the completeness law is numerical,
/// and naive accumulation over many faces bleeds precision.
pub fn compensated_sum(xs: impl IntoIterator<Item = f64>) -> f64 {
    let mut s = 0.0f64;
    let mut c = 0.0f64;
    for x in xs {
        let t = s + x;
        if s.abs() >= x.abs() {
            c += (s - t) + x;
        } else {
            c += (x - t) + s;
        }
        s = t;
    }
    s + c
}

/// Signed area of a spherical triangle in steradians, by the robust
/// atan2 form (well-conditioned where acos-based forms are not).
pub fn tri_area(a: &UnitVec, b: &UnitVec, c: &UnitVec) -> f64 {
    let (cx, cy, cz) = b.cross_raw(c);
    let det = a.x() * cx + a.y() * cy + a.z() * cz;
    let denom = 1.0 + a.dot(b) + b.dot(c) + c.dot(a);
    2.0 * det.atan2(denom)
}

/// Signed area enclosed by a closed spherical polygon (first point is
/// NOT repeated), by triangulating as a fan from the first vertex.
/// Positive when traversed with the interior on the left.
pub fn cycle_area(pts: &[UnitVec]) -> f64 {
    if pts.len() < 3 {
        return 0.0;
    }
    compensated_sum((1..pts.len() - 1).map(|i| tri_area(&pts[0], &pts[i], &pts[i + 1])))
}

/// A deterministic tangent frame at p: (e1, e2) orthonormal, both
/// perpendicular to p.
pub fn tangent_frame(p: &UnitVec) -> ([f64; 3], [f64; 3]) {
    let (ax, ay, az) = if p.z().abs() < 0.9 { (0.0, 0.0, 1.0) } else { (1.0, 0.0, 0.0) };
    // e1 = normalize(axis × p)
    let e1 = [ay * p.z() - az * p.y(), az * p.x() - ax * p.z(), ax * p.y() - ay * p.x()];
    let n = (e1[0] * e1[0] + e1[1] * e1[1] + e1[2] * e1[2]).sqrt();
    let e1 = [e1[0] / n, e1[1] / n, e1[2] / n];
    // e2 = p × e1
    let e2 = [
        p.y() * e1[2] - p.z() * e1[1],
        p.z() * e1[0] - p.x() * e1[2],
        p.x() * e1[1] - p.y() * e1[0],
    ];
    (e1, e2)
}

/// Bearing (radians in (-π, π]) of the direction from p toward q, in
/// p's tangent frame. Undefined for q at ±p; callers keep arcs short.
pub fn bearing(p: &UnitVec, q: &UnitVec) -> f64 {
    let (e1, e2) = tangent_frame(p);
    let d = p.dot(q);
    // tangent component of q at p
    let t = [q.x() - d * p.x(), q.y() - d * p.y(), q.z() - d * p.z()];
    let u = t[0] * e1[0] + t[1] * e1[1] + t[2] * e1[2];
    let v = t[0] * e2[0] + t[1] * e2[1] + t[2] * e2[2];
    v.atan2(u)
}

/// Winding number of a closed point cycle around p: +1 when p lies on
/// the cycle's interior-left side, -1 for its antipode, 0 outside.
pub fn winding(cycle: &[UnitVec], p: &UnitVec) -> i32 {
    if cycle.len() < 3 {
        return 0;
    }
    let mut total = 0.0f64;
    let mut prev = bearing(p, &cycle[0]);
    for i in 1..=cycle.len() {
        let b = bearing(p, &cycle[i % cycle.len()]);
        let mut d = b - prev;
        while d > std::f64::consts::PI {
            d -= 2.0 * std::f64::consts::PI;
        }
        while d <= -std::f64::consts::PI {
            d += 2.0 * std::f64::consts::PI;
        }
        total += d;
        prev = b;
    }
    (total / (2.0 * std::f64::consts::PI)).round() as i32
}

// ------------------------------------------------------------ model

/// What a face IS, semantically. Metadata never affects topology.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum FaceKind {
    LandClaim,
    Sea,
    Lake,
    Background,
}

/// The tolerances of ingestion: how far apart two witness points may
/// be and still be the same place. These are WITNESS tolerances —
/// tracing disagreement — and are deliberately far larger than the
/// f64 tolerance of the area law.
#[derive(Clone, Copy, Debug)]
pub struct PartitionConfig {
    /// radians: candidate nodes closer than this are one vertex.
    pub tau_vertex: f64,
    /// radians: a node this close to an edge interior is ON the edge.
    pub tau_edge: f64,
    /// steradians: cells smaller than this are slivers, absorbed
    /// semantically into their longest-boundary neighbor.
    pub sliver_area: f64,
}

impl Default for PartitionConfig {
    fn default() -> Self {
        // ~75 m and ~150 m on Earth's radius; slivers under ~2 km².
        PartitionConfig { tau_vertex: 1.2e-5, tau_edge: 2.4e-5, sliver_area: 5.0e-8 }
    }
}

pub type VertexId = usize;
pub type EdgeId = usize;
pub type HalfId = usize;
pub type FaceId = usize;

/// One canonical undirected edge: a single minor great-circle arc
/// between two canonical vertices, stored exactly once.
#[derive(Clone, Debug)]
pub struct PEdge {
    pub a: VertexId,
    pub b: VertexId,
    /// half-edge a→b and its twin b→a.
    pub half_ab: HalfId,
    pub half_ba: HalfId,
    /// river attribute: this border IS a river (style strokes it).
    pub river: bool,
    /// the witnesses whose geometry this edge carries.
    pub provenance: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct PHalf {
    pub origin: VertexId,
    pub edge: EdgeId,
    pub twin: HalfId,
    pub next: HalfId,
    pub prev: HalfId,
    pub face: FaceId,
}

/// A face: one or more closed half-edge cycles bounding one cell (or
/// one cell-with-holes) of the arrangement.
#[derive(Clone, Debug)]
pub struct PFace {
    pub cycles: Vec<Vec<HalfId>>,
    pub kind: FaceKind,
    /// ids of the witnesses claiming this face (empty = background).
    pub claims: Vec<String>,
    /// witnesses that claimed it but lost reconciliation (diagnostic;
    /// never silently averaged).
    pub conflicts: Vec<String>,
    pub area: f64,
}

/// A river path in the overlay: noded into the partition's vertex
/// pool wherever it meets canonical geometry; free elsewhere. Never
/// a face — a polyline cannot have holes.
#[derive(Clone, Debug)]
pub struct RiverPath {
    pub id: String,
    pub pts: Vec<UnitVec>,
}

pub struct Partition {
    pub vertices: Vec<UnitVec>,
    pub edges: Vec<PEdge>,
    pub halves: Vec<PHalf>,
    pub faces: Vec<PFace>,
    pub rivers: Vec<RiverPath>,
    pub diagnostics: Vec<String>,
}

impl Partition {
    /// The completeness sum: must be 4π for a valid partition.
    pub fn total_area(&self) -> f64 {
        compensated_sum(self.faces.iter().map(|f| f.area))
    }

    /// The residual of the completeness law, in steradians.
    pub fn area_residual(&self) -> f64 {
        (4.0 * std::f64::consts::PI - self.total_area()).abs()
    }

    /// A face's boundary cycles as point rings (origin vertices in
    /// cycle order; the closing point is not repeated).
    pub fn face_rings(&self, f: FaceId) -> Vec<Vec<UnitVec>> {
        self.faces[f]
            .cycles
            .iter()
            .map(|cy| cy.iter().map(|&h| self.vertices[self.halves[h].origin]).collect())
            .collect()
    }

    /// Every structural law, checked. Empty = lawful.
    pub fn validate(&self) -> Vec<String> {
        let mut out = Vec::new();
        for (i, h) in self.halves.iter().enumerate() {
            if self.halves[h.twin].twin != i {
                out.push(format!("half {i}: twin(twin) != self"));
            }
            if self.halves[h.next].prev != i {
                out.push(format!("half {i}: prev(next) != self"));
            }
            if self.halves[h.next].face != h.face {
                out.push(format!("half {i}: next crosses faces"));
            }
            let dest = self.halves[h.twin].origin;
            if self.halves[h.next].origin != dest {
                out.push(format!("half {i}: next does not continue at destination"));
            }
            if self.halves[h.twin].face == h.face {
                out.push(format!("half {i}: edge with the same face on both sides"));
            }
        }
        for (i, e) in self.edges.iter().enumerate() {
            if e.a == e.b {
                out.push(format!("edge {i}: zero-length"));
            }
        }
        // faces: cycles closed (structural via next-integrity) and
        // positive area
        for (i, f) in self.faces.iter().enumerate() {
            if f.area <= 0.0 {
                out.push(format!("face {i}: non-positive area {}", f.area));
            }
        }
        // completeness
        let residual = self.area_residual();
        if residual > 1e-10 {
            out.push(format!("completeness: |4π - Σ areas| = {residual:e}"));
        }
        out
    }

    /// Content hash of the canonical geometry: independent of witness
    /// order, ring winding, and construction path. Styles and claims
    /// ride separately — geometry identity is geometric.
    pub fn content_hash(&self) -> u64 {
        // canonical vertex order: by quantized coordinate bytes
        let mut order: Vec<usize> = (0..self.vertices.len()).collect();
        let key = |v: &UnitVec| {
            let q = |x: f64| ((x * 1e9).round() as i64).to_be_bytes();
            let mut k = [0u8; 24];
            k[..8].copy_from_slice(&q(v.x()));
            k[8..16].copy_from_slice(&q(v.y()));
            k[16..].copy_from_slice(&q(v.z()));
            k
        };
        order.sort_by_key(|&i| key(&self.vertices[i]));
        let mut rank = vec![0usize; self.vertices.len()];
        for (r, &i) in order.iter().enumerate() {
            rank[i] = r;
        }
        let mut h = Fnv::new();
        for &i in &order {
            h.bytes(&key(&self.vertices[i]));
        }
        let mut edge_keys: Vec<(usize, usize, bool)> = self
            .edges
            .iter()
            .map(|e| {
                let (x, y) = (rank[e.a], rank[e.b]);
                (x.min(y), x.max(y), e.river)
            })
            .collect();
        edge_keys.sort();
        for (a, b, r) in edge_keys {
            h.u64(a as u64).u64(b as u64).u64(r as u64);
        }
        h.finish()
    }
}

/// FNV-1a: a small stable hash — DefaultHasher's keys are not
/// guaranteed stable across releases, and this identity must be.
pub struct Fnv(u64);
impl Fnv {
    pub fn new() -> Self {
        Fnv(0xcbf29ce484222325)
    }
    pub fn bytes(&mut self, b: &[u8]) -> &mut Self {
        for &x in b {
            self.0 ^= x as u64;
            self.0 = self.0.wrapping_mul(0x100000001b3);
        }
        self
    }
    pub fn u64(&mut self, x: u64) -> &mut Self {
        self.bytes(&x.to_be_bytes())
    }
    pub fn finish(&self) -> u64 {
        self.0
    }
}

impl Default for Fnv {
    fn default() -> Self {
        Self::new()
    }
}
