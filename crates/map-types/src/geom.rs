//! Geometry: slerp-ready from birth. Points are unit vectors on the
//! sphere, not lat/lon — two matched point runs interpolate by slerp,
//! so transitions are a data shape, not an afterthought (spec §B).

use crate::ident::Canon;

const UNIT_EPS: f64 = 1e-9;

/// A point on the unit sphere; |v| = 1 is checked at construction, so
/// downstream math never renormalizes defensively.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct UnitVec {
    x: f64,
    y: f64,
    z: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GeomError {
    NotUnitLength,
    ZeroVector,
    /// Slerp between antipodes has no unique path — the caller must
    /// resample, never guess.
    Antipodal,
    /// Rings need at least three distinct points.
    DegenerateRing,
    /// Morphing requires equal point counts (resampling is upstream work).
    PointCountMismatch,
}

impl UnitVec {
    pub fn new(x: f64, y: f64, z: f64) -> Result<Self, GeomError> {
        let n2 = x * x + y * y + z * z;
        if (n2.sqrt() - 1.0).abs() > UNIT_EPS {
            Err(GeomError::NotUnitLength)
        } else {
            Ok(UnitVec { x, y, z })
        }
    }

    /// Normalizing constructor for computed vectors.
    pub fn normalize(x: f64, y: f64, z: f64) -> Result<Self, GeomError> {
        let n = (x * x + y * y + z * z).sqrt();
        if n < UNIT_EPS {
            Err(GeomError::ZeroVector)
        } else {
            Ok(UnitVec { x: x / n, y: y / n, z: z / n })
        }
    }

    /// From geographic coordinates in degrees (adapter convenience;
    /// pure math, no source format knowledge).
    pub fn from_lat_lon_deg(lat: f64, lon: f64) -> Self {
        let (la, lo) = (lat.to_radians(), lon.to_radians());
        UnitVec { x: la.cos() * lo.cos(), y: la.cos() * lo.sin(), z: la.sin() }
    }

    /// Back to geographic degrees — the inverse of `from_lat_lon_deg`
    /// (total: every unit vector has a latitude and, off the poles, a
    /// longitude; at the poles longitude collapses to 0).
    pub fn to_lat_lon_deg(&self) -> (f64, f64) {
        (self.z.clamp(-1.0, 1.0).asin().to_degrees(), self.y.atan2(self.x).to_degrees())
    }

    pub fn x(&self) -> f64 {
        self.x
    }
    pub fn y(&self) -> f64 {
        self.y
    }
    pub fn z(&self) -> f64 {
        self.z
    }

    pub fn dot(&self, o: &UnitVec) -> f64 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }

    pub fn cross_raw(&self, o: &UnitVec) -> (f64, f64, f64) {
        (
            self.y * o.z - self.z * o.y,
            self.z * o.x - self.x * o.z,
            self.x * o.y - self.y * o.x,
        )
    }

    /// Angular distance in radians — the sphere's native metric.
    pub fn angle_to(&self, o: &UnitVec) -> f64 {
        self.dot(o).clamp(-1.0, 1.0).acos()
    }

    pub fn canon(&self, c: &mut Canon) {
        c.f64_(self.x).f64_(self.y).f64_(self.z);
    }

    /// The diametrically opposite point. Exactly unit by construction.
    pub fn antipode(&self) -> UnitVec {
        UnitVec { x: -self.x, y: -self.y, z: -self.z }
    }
}

/// Spherical linear interpolation. Total except between antipodes,
/// where no unique great-circle path exists.
pub fn slerp(a: &UnitVec, b: &UnitVec, t: f64) -> Result<UnitVec, GeomError> {
    let cos_omega = a.dot(b).clamp(-1.0, 1.0);
    let omega = cos_omega.acos();
    if omega.sin() < UNIT_EPS {
        if cos_omega > 0.0 {
            return Ok(*a); // coincident: the path is a point
        }
        return Err(GeomError::Antipodal);
    }
    let (sa, sb) = (((1.0 - t) * omega).sin() / omega.sin(), (t * omega).sin() / omega.sin());
    UnitVec::normalize(
        sa * a.x + sb * b.x,
        sa * a.y + sb * b.y,
        sa * a.z + sb * b.z,
    )
}

/// A closed ring: consecutive points are edges, last connects to first
/// (the first point is NOT repeated). Winding encodes containment.
#[derive(Clone, Debug, PartialEq)]
pub struct Ring(Vec<UnitVec>);

impl Ring {
    pub fn new(pts: Vec<UnitVec>) -> Result<Self, GeomError> {
        if pts.len() < 3 {
            Err(GeomError::DegenerateRing)
        } else {
            Ok(Ring(pts))
        }
    }
    pub fn points(&self) -> &[UnitVec] {
        &self.0
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn is_empty(&self) -> bool {
        false // by construction: >= 3 points
    }

    /// Winding sign relative to the ring's own centroid direction.
    /// Convention: Ccw (positive circulation seen from outside the
    /// sphere above the centroid) = containment. Valid for rings that
    /// do not enclose the centroid's antipode — fine for polity-scale
    /// geometry; hemispheric+ rings are a declared future problem.
    pub fn winding(&self) -> Winding {
        let (mut cx, mut cy, mut cz) = (0.0, 0.0, 0.0);
        for p in &self.0 {
            cx += p.x();
            cy += p.y();
            cz += p.z();
        }
        let mut s = 0.0;
        for i in 0..self.0.len() {
            let a = &self.0[i];
            let b = &self.0[(i + 1) % self.0.len()];
            let (x, y, z) = a.cross_raw(b);
            s += x * cx + y * cy + z * cz;
        }
        if s >= 0.0 {
            Winding::Ccw
        } else {
            Winding::Cw
        }
    }

    pub fn canon(&self, c: &mut Canon) {
        c.seq(&self.0, |c, p| p.canon(c));
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Winding {
    Ccw,
    Cw,
}

/// Morph two matched rings: pairwise slerp. Law 4 (morph safety) says
/// this preserves closure and winding — the tests prove it.
pub fn morph_rings(a: &Ring, b: &Ring, t: f64) -> Result<Ring, GeomError> {
    if a.len() != b.len() {
        return Err(GeomError::PointCountMismatch);
    }
    let pts = a
        .points()
        .iter()
        .zip(b.points())
        .map(|(p, q)| slerp(p, q, t))
        .collect::<Result<Vec<_>, _>>()?;
    Ring::new(pts)
}

/// Level of detail: an angular simplification tolerance in radians.
/// Law 7 (LOD monotonicity): a coarser (larger) tolerance never ADDS
/// points.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Lod(pub f64);

impl Lod {
    pub fn exact() -> Self {
        Lod(0.0)
    }
    pub fn canon(&self, c: &mut Canon) {
        c.f64_(self.0);
    }
}

/// Angular distance from `p` to the great-circle arc from `a` to `b`.
fn arc_distance(p: &UnitVec, a: &UnitVec, b: &UnitVec) -> f64 {
    let n = a.cross_raw(b);
    let n_len = (n.0 * n.0 + n.1 * n.1 + n.2 * n.2).sqrt();
    if n_len < UNIT_EPS {
        return p.angle_to(a); // a and b coincide
    }
    let n = (n.0 / n_len, n.1 / n_len, n.2 / n_len);
    // Is p's closest great-circle point within the arc segment?
    let ap = a.cross_raw(p);
    let pb = p.cross_raw(b);
    let within = (ap.0 * n.0 + ap.1 * n.1 + ap.2 * n.2) >= 0.0
        && (pb.0 * n.0 + pb.1 * n.1 + pb.2 * n.2) >= 0.0;
    if within {
        (p.x() * n.0 + p.y() * n.1 + p.z() * n.2).asin().abs()
    } else {
        p.angle_to(a).min(p.angle_to(b))
    }
}

/// Douglas–Peucker on the sphere for an open polyline. Endpoints are
/// always kept. The kept set at a larger tolerance is a subset of the
/// kept set at a smaller one — which is exactly law 7.
/// The WHOLE-SPHERE SENTINEL (RegionPart's empty-cycle convention): a
/// ring of at most five stored points containing a near-antipodal
/// pair. Its interior is everything — encoders dress it as the page
/// or the limb, and no viewport may cull it.
pub fn covers_sphere(pts: &[UnitVec]) -> bool {
    pts.len() <= 5 && pts.iter().any(|a| pts.iter().any(|b| a.dot(b) < -0.99))
}

pub fn simplify_polyline(pts: &[UnitVec], lod: Lod) -> Vec<UnitVec> {
    if pts.len() <= 2 || lod.0 <= 0.0 {
        return pts.to_vec();
    }
    let mut keep = vec![false; pts.len()];
    keep[0] = true;
    keep[pts.len() - 1] = true;
    dp_mark(pts, 0, pts.len() - 1, lod.0, &mut keep);
    pts.iter()
        .zip(&keep)
        .filter_map(|(p, k)| if *k { Some(*p) } else { None })
        .collect()
}

fn dp_mark(pts: &[UnitVec], i: usize, j: usize, tol: f64, keep: &mut [bool]) {
    if j <= i + 1 {
        return;
    }
    let (mut worst, mut worst_d) = (i + 1, -1.0);
    for k in (i + 1)..j {
        let d = arc_distance(&pts[k], &pts[i], &pts[j]);
        if d > worst_d {
            worst = k;
            worst_d = d;
        }
    }
    if worst_d > tol {
        keep[worst] = true;
        dp_mark(pts, i, worst, tol, keep);
        dp_mark(pts, worst, j, tol, keep);
    }
}

// -------------------------------------------- the containment law
//
// THE MISSING PRIMITIVE both renderers' limb bugs reduce to: does a
// point lie inside a closed spherical ring? A closed SIMPLE curve
// divides the sphere into two components, and — unlike the plane —
// neither holds a point at infinity to call "outside". The law below
// speaks about simple (non-self-intersecting) rings, which is what the
// geometry pipeline stores; a self-crossing ring has no two-component
// decomposition for it to speak about. The convention, stated once and
// tested as law:
//
//   THE INTERIOR OF A RING IS ITS SMALLER-AREA COMPONENT
//   (an exact half-sphere tie goes to the left of traversal).
//
// Consequences, each a property test in tests.rs:
//   - traversal-invariant: reversing the ring changes nothing, so the
//     even-odd fill rule (which ignores winding) composes over rings;
//   - for a ring bounded by a spherical cap smaller than a hemisphere
//     the interior is the cap-side component, so the antipode of the
//     cap's axis is provably OUTSIDE — the fast path;
//   - the whole-sphere sentinel (covers_sphere) has NO smaller side;
//     its interior is everything, by the existing decree above.
//
// Membership is decided by even-odd crossing counts: the geodesic from
// the query point to a reference point of known status crosses the
// ring an odd number of times exactly when the two lie in different
// components. Crossings use the same half-open sign convention as the
// classic 2D ray cast, so an arc through a ring vertex counts once,
// and a vertex merely touched counts twice (parity unchanged).

/// Signed-side test with the half-open convention: a point exactly on
/// the great circle counts as the negative side, the same tie-break
/// the 2D even-odd cast uses at vertices.
fn side(n: (f64, f64, f64), q: &UnitVec) -> bool {
    n.0 * q.x + n.1 * q.y + n.2 * q.z > 0.0
}

/// Does the geodesic p->r cross the edge a->b? Both arcs are the
/// shorter great-circle arcs.
fn arcs_cross(p: &UnitVec, r: &UnitVec, n1: (f64, f64, f64), a: &UnitVec, b: &UnitVec) -> bool {
    if side(n1, a) == side(n1, b) {
        return false; // edge does not straddle the test arc's circle
    }
    let n2 = a.cross_raw(b);
    if side(n2, p) == side(n2, r) {
        return false; // test arc does not straddle the edge's circle
    }
    // The two great circles meet at the antipodal pair +/-(n1 x n2).
    // The straddles above guarantee each shorter arc crosses the
    // OTHER's circle exactly once — at whichever of +/-x lies nearer
    // that arc's midpoint. The arcs cross each other iff those are the
    // same point: the midpoint dots agree in sign. (An endpoint-exact
    // crossing keeps a robust margin here — dot(x, a+b) is 1+cos(edge)
    // when x IS a vertex — where an alpha/beta arc-membership test
    // sits on a rounding knife-edge.)
    let x = (
        n1.1 * n2.2 - n1.2 * n2.1,
        n1.2 * n2.0 - n1.0 * n2.2,
        n1.0 * n2.1 - n1.1 * n2.0,
    );
    let d_ab = x.0 * (a.x + b.x) + x.1 * (a.y + b.y) + x.2 * (a.z + b.z);
    let d_pr = x.0 * (p.x + r.x) + x.1 * (p.y + r.y) + x.2 * (p.z + r.z);
    d_ab * d_pr > 0.0
}

/// Crossing parity of the geodesic p->r against every ring edge
/// (excluding index `skip`, for the labelling walk that starts ON that
/// edge). True = odd = p and r in different components.
fn crossing_parity(p: &UnitVec, r: &UnitVec, ring: &[UnitVec], skip: Option<usize>) -> bool {
    let n1 = p.cross_raw(r);
    let mut odd = false;
    let n = ring.len();
    for i in 0..n {
        if skip == Some(i) {
            continue;
        }
        if arcs_cross(p, r, n1, &ring[i], &ring[(i + 1) % n]) {
            odd = !odd;
        }
    }
    odd
}

/// The ring's bounding cap: vertex-mean axis and the cosine of the
/// angular radius (the minimum dot with the axis). None when the
/// vertices cancel to no meaningful centre.
fn ring_cap(ring: &[UnitVec]) -> Option<(UnitVec, f64)> {
    let (mut x, mut y, mut z) = (0.0, 0.0, 0.0);
    for p in ring {
        x += p.x;
        y += p.y;
        z += p.z;
    }
    let axis = UnitVec::normalize(x, y, z).ok()?;
    let cos_r = ring.iter().map(|p| p.dot(&axis)).fold(1.0_f64, f64::min);
    Some((axis, cos_r.clamp(-1.0, 1.0)))
}

/// Is `p` inside the closed ring, under the smaller-component law
/// documented above? Rings are cyclic (no repeated closing point).
/// Fewer than three distinct points have no interior; a covers_sphere
/// sentinel contains everything.
pub fn inside_ring(p: &UnitVec, ring: &[UnitVec]) -> bool {
    if ring.len() < 3 {
        return false;
    }
    if covers_sphere(ring) {
        return true;
    }
    if let Some((axis, cos_r)) = ring_cap(ring) {
        if cos_r > 0.0 {
            // The ring fits in a cap smaller than a hemisphere: every
            // point beyond the cap — the axis's antipode in particular
            // — is in the outside component, and the interior (the
            // cap-side component, within an open hemisphere) is the
            // smaller side. Odd crossings = inside.
            let r = outside_reference(p, &axis, cos_r);
            return crossing_parity(p, &r, ring, None);
        }
    }
    inside_ring_general(p, ring)
}

/// A reference point provably outside the cap (axis, cos_r), chosen so
/// it is never near-antipodal to `p` (a geodesic between antipodes is
/// ill-defined). The axis's antipode serves unless `p` sits near the
/// axis; the fallbacks stay outside the cap by construction — any
/// point at more than the cap's angle from the axis is outside.
fn outside_reference(p: &UnitVec, axis: &UnitVec, cos_r: f64) -> UnitVec {
    let anti = axis.antipode();
    if p.dot(&anti) > -0.999 {
        return anti;
    }
    // p is essentially the axis: step off the antipode along a
    // perpendicular, staying more than the cap's angle away from the
    // axis. Halfway between the cap rim and the antipode is exact.
    let e = UnitVec::normalize(-axis.y, axis.x, 0.0)
        .unwrap_or_else(|_| UnitVec::from_lat_lon_deg(0.0, 90.0));
    let phi = (cos_r.acos() + std::f64::consts::PI) / 2.0;
    let (c, s) = (phi.cos(), phi.sin());
    UnitVec::normalize(
        axis.x * c + e.x * s,
        axis.y * c + e.y * s,
        axis.z * c + e.z * s,
    )
    .expect("axis and perpendicular are orthonormal")
}

/// The general path, no cap assumption: label the two components by
/// area (Gauss–Bonnet turning angles) and decide membership by parity
/// against the labelled side. pub(crate) so the law tests can drive it
/// directly against the fast path.
pub(crate) fn inside_ring_general(p: &UnitVec, ring: &[UnitVec]) -> bool {
    // Drop consecutive duplicates: zero-length edges carry no turning
    // angle and no crossing, only conditioning trouble.
    let mut pts: Vec<UnitVec> = Vec::with_capacity(ring.len());
    for q in ring {
        if pts.last().map_or(true, |l: &UnitVec| l.dot(q) < 1.0 - 1e-12) {
            pts.push(*q);
        }
    }
    while pts.len() > 1 && pts[0].dot(pts.last().unwrap()) >= 1.0 - 1e-12 {
        pts.pop();
    }
    if pts.len() < 3 {
        return false;
    }
    let n = pts.len();

    // Area of the LEFT-of-traversal component: 2*pi minus the summed
    // signed turning angles (spherical Gauss–Bonnet; left turns
    // positive about the outward normal).
    let mut turn = 0.0;
    for i in 0..n {
        let u = &pts[(i + n - 1) % n];
        let v = &pts[i];
        let w = &pts[(i + 1) % n];
        // Unit tangents at v: arriving from u, departing toward w.
        let t_in = UnitVec::normalize(
            v.x * u.dot(v) - u.x,
            v.y * u.dot(v) - u.y,
            v.z * u.dot(v) - u.z,
        );
        let t_out = UnitVec::normalize(
            w.x - v.x * v.dot(w),
            w.y - v.y * v.dot(w),
            w.z - v.z * v.dot(w),
        );
        let (Ok(t_in), Ok(t_out)) = (t_in, t_out) else {
            continue; // an antipodal edge pair: no defined tangent, no turn
        };
        let cr = t_in.cross_raw(&t_out);
        turn += (cr.0 * v.x + cr.1 * v.y + cr.2 * v.z).atan2(t_in.dot(&t_out));
    }
    let left_area = std::f64::consts::TAU - turn;
    let left_is_interior = left_area <= std::f64::consts::TAU;

    // The longest edge anchors the labelling: its midpoint m lies ON
    // the ring, and the geodesic m->r meets that edge's great circle
    // only at m itself (two great circles cross exactly at +/-m), so
    // skipping the edge in the count is exact, and whether a point
    // just LEFT of m reaches r without crossing the edge is decided
    // by which side of the edge's circle r lies on.
    let mut e_best = 0;
    let mut c_best = 2.0;
    for i in 0..n {
        let c = pts[i].dot(&pts[(i + 1) % n]);
        if c < c_best {
            c_best = c;
            e_best = i;
        }
    }
    let (a, b) = (&pts[e_best], &pts[(e_best + 1) % n]);
    let m = UnitVec::normalize(a.x + b.x, a.y + b.y, a.z + b.z)
        .expect("the longest edge of a deduplicated ring is not antipodal");
    let ncr = a.cross_raw(b);
    let n_hat = UnitVec::normalize(ncr.0, ncr.1, ncr.2)
        .expect("the longest edge spans a defined great circle");

    // A reference not near-antipodal to p or m and decisively off the
    // labelling edge's great circle. Six spread candidates: p can veto
    // at most one, m one, the circle two.
    let candidates = [
        UnitVec { x: 0.0, y: 0.0, z: 1.0 },
        UnitVec { x: 0.0, y: 0.0, z: -1.0 },
        UnitVec { x: 1.0, y: 0.0, z: 0.0 },
        UnitVec { x: 0.0, y: 1.0, z: 0.0 },
        UnitVec { x: -1.0, y: 0.0, z: 0.0 },
        UnitVec { x: 0.0, y: -1.0, z: 0.0 },
    ];
    let r = candidates
        .iter()
        .find(|r| p.dot(r) > -0.999 && m.dot(r) > -0.999 && r.dot(&n_hat).abs() > 1e-6)
        .copied()
        .unwrap_or(candidates[0]);

    // Parity of the LEFT class: crossings from m to r over the other
    // edges, plus one for the labelling edge itself when r lies to its
    // right (a just-left point must cross the edge to depart right).
    let mut left_parity = crossing_parity(&m, &r, &pts, Some(e_best));
    if !side((n_hat.x, n_hat.y, n_hat.z), &r) {
        left_parity = !left_parity;
    }
    let p_parity = crossing_parity(p, &r, &pts, None);
    // p is in the left class iff its parity matches the left class's;
    // inside iff that class is the interior.
    (p_parity == left_parity) == left_is_interior
}

/// Viewport: a spherical cap — the simplest sound "arbitrary chunk of
/// the world". A rectangle-in-projection is a consumer concern.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bbox {
    pub center: UnitVec,
    /// Angular radius in radians; PI covers the whole sphere.
    pub radius: f64,
}

impl Bbox {
    pub fn whole_world() -> Self {
        Bbox { center: UnitVec { x: 0.0, y: 0.0, z: 1.0 }, radius: std::f64::consts::PI }
    }
    pub fn contains(&self, p: &UnitVec) -> bool {
        self.center.angle_to(p) <= self.radius
    }
    pub fn canon(&self, c: &mut Canon) {
        self.center.canon(c);
        c.f64_(self.radius);
    }
}
