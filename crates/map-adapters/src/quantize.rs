//! Coordinate quantization — the disclosed matching method for shared-
//! arc extraction. Coordinates snap to a 1e-7-degree grid (about a
//! centimeter at the equator, four orders of magnitude below the
//! source's own precision), so vertices that agree in the source text
//! agree exactly as integers and arc matching is exact arithmetic, not
//! epsilon guesswork.

use map_types::UnitVec;

const GRID: f64 = 1e7;

/// A grid point: quantized (lon, lat) in 1e-7-degree units.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct QPoint {
    pub lon: i64,
    pub lat: i64,
}

impl QPoint {
    pub fn from_lon_lat(lon: f64, lat: f64) -> Self {
        QPoint { lon: (lon * GRID).round() as i64, lat: (lat * GRID).round() as i64 }
    }
    pub fn to_unit_vec(self) -> UnitVec {
        UnitVec::from_lat_lon_deg(self.lat as f64 / GRID, self.lon as f64 / GRID)
    }
}

/// Quantize a source ring and normalize it: consecutive duplicates
/// collapse, and the closing repeat of the first point (the input
/// convention) is dropped — our rings leave closure implicit.
/// Returns None for rings degenerate after cleaning (< 3 points).
pub fn clean_ring(src: &[(f64, f64)]) -> Option<Vec<QPoint>> {
    let mut pts: Vec<QPoint> = Vec::with_capacity(src.len());
    for &(lon, lat) in src {
        let q = QPoint::from_lon_lat(lon, lat);
        if pts.last() != Some(&q) {
            pts.push(q);
        }
    }
    while pts.len() > 1 && pts.first() == pts.last() {
        pts.pop();
    }
    if pts.len() < 3 {
        None
    } else {
        Some(pts)
    }
}

/// Rotate a ring to its lexicographically least phase — the canonical
/// form under which two traversals of the same ring compare equal.
pub fn canonical_rotation(pts: &[QPoint]) -> Vec<QPoint> {
    let start = (0..pts.len())
        .min_by(|&a, &b| {
            let ra = pts[a..].iter().chain(&pts[..a]);
            let rb = pts[b..].iter().chain(&pts[..b]);
            ra.cmp(rb)
        })
        .unwrap_or(0);
    let mut out = Vec::with_capacity(pts.len());
    out.extend_from_slice(&pts[start..]);
    out.extend_from_slice(&pts[..start]);
    out
}
