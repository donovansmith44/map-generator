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
