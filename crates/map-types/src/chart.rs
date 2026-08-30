//! A CHART: a bijective mapping between a planar map and a patch of
//! the sphere. The sphere is the one canonical home of borders; every
//! flat artifact — a georeferenced plate, a rendered frame — is a
//! chart of it, invertible on its own domain, so map-space data
//! DERIVES its 3d and any two charts compose through the sphere.
//! (An atlas, in the mathematician's sense as much as ours.)

use crate::geom::UnitVec;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChartError {
    /// The plane→degrees matrix must be invertible; a chart that
    /// collapses the plane is not a mapping anyone can come back
    /// through.
    Degenerate,
}

/// An affine chart: `[lon, lat] = A·[x, y] + b`, bijective on the
/// planar `domain` rectangle. Affine-in-degrees is exact for plates
/// whose projection is locally flat at their scale (the calibration
/// residual, carried by the producer, is the honesty figure).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Chart {
    a: [[f64; 2]; 2],
    b: [f64; 2],
    /// x0, y0, x1, y1 — the plate frame in its own pixel units.
    domain: (f64, f64, f64, f64),
}

impl Chart {
    pub fn new(a: [[f64; 2]; 2], b: [f64; 2], domain: (f64, f64, f64, f64)) -> Result<Self, ChartError> {
        let det = a[0][0] * a[1][1] - a[0][1] * a[1][0];
        if det.abs() < 1e-18 {
            return Err(ChartError::Degenerate);
        }
        Ok(Chart { a, b, domain })
    }

    pub fn contains(&self, x: f64, y: f64) -> bool {
        let (x0, y0, x1, y1) = self.domain;
        // The frame edge belongs to the chart: float dust from a
        // roundtrip must not evict a corner.
        let eps = 1e-9 * ((x1 - x0).abs() + (y1 - y0).abs()).max(1.0);
        x >= x0 - eps && x <= x1 + eps && y >= y0 - eps && y <= y1 + eps
    }

    /// Map plane → degrees without the domain check (the raw affine).
    fn lon_lat(&self, x: f64, y: f64) -> (f64, f64) {
        (
            self.a[0][0] * x + self.a[0][1] * y + self.b[0],
            self.a[1][0] * x + self.a[1][1] * y + self.b[1],
        )
    }

    /// The chart's forward direction: a point ON the map becomes a
    /// point on the sphere. None outside the domain — a chart claims
    /// nothing beyond its own frame.
    pub fn to_sphere(&self, x: f64, y: f64) -> Option<UnitVec> {
        if !self.contains(x, y) {
            return None;
        }
        let (lon, lat) = self.lon_lat(x, y);
        Some(UnitVec::from_lat_lon_deg(lat, lon))
    }

    /// The inverse: where does a sphere point sit on this map? None
    /// when it falls outside the frame.
    pub fn from_sphere(&self, p: &UnitVec) -> Option<(f64, f64)> {
        let (lat, lon) = p.to_lat_lon_deg();
        let (u, v) = (lon - self.b[0], lat - self.b[1]);
        let det = self.a[0][0] * self.a[1][1] - self.a[0][1] * self.a[1][0];
        let x = (u * self.a[1][1] - v * self.a[0][1]) / det;
        let y = (v * self.a[0][0] - u * self.a[1][0]) / det;
        if self.contains(x, y) {
            Some((x, y))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod chart_laws {
    use super::*;

    fn plate_like() -> Chart {
        // a plausible plate: ~0.00075°/px, slight rotation
        Chart::new(
            [[7.4e-4, 1.1e-5], [-9.0e-6, -7.2e-4]],
            [33.2, 34.1],
            (0.0, 0.0, 4500.0, 6000.0),
        )
        .unwrap()
    }

    #[test]
    fn plane_roundtrips_through_the_sphere() {
        let c = plate_like();
        for (x, y) in [(0.0, 0.0), (4500.0, 6000.0), (2250.0, 3000.0), (17.0, 5990.0)] {
            let p = c.to_sphere(x, y).expect("in domain");
            let (x2, y2) = c.from_sphere(&p).expect("comes back");
            assert!((x - x2).abs() < 1e-6 && (y - y2).abs() < 1e-6, "({x},{y}) -> ({x2},{y2})");
        }
    }

    #[test]
    fn sphere_roundtrips_through_the_plane() {
        let c = plate_like();
        let p = UnitVec::from_lat_lon_deg(31.778, 35.229);
        let (x, y) = c.from_sphere(&p).expect("Jerusalem is on the plate");
        let q = c.to_sphere(x, y).unwrap();
        assert!(p.angle_to(&q) < 1e-12, "bijective where defined");
    }

    #[test]
    fn a_chart_claims_nothing_beyond_its_frame() {
        let c = plate_like();
        assert!(c.to_sphere(-1.0, 10.0).is_none());
        assert!(c.to_sphere(10.0, 6001.0).is_none());
        // Rome is not on this plate
        let rome = UnitVec::from_lat_lon_deg(41.9, 12.5);
        assert!(c.from_sphere(&rome).is_none());
    }

    #[test]
    fn collapsing_charts_are_refused() {
        assert_eq!(
            Chart::new([[1.0, 2.0], [2.0, 4.0]], [0.0, 0.0], (0.0, 0.0, 1.0, 1.0)),
            Err(ChartError::Degenerate)
        );
    }

    #[test]
    fn charts_compose_through_the_sphere() {
        // plate -> sphere -> screen equals one affine, to float precision
        let plate = plate_like();
        let screen = Chart::new(
            [[3.0e-3, 0.0], [0.0, -3.0e-3]],
            [33.0, 36.0],
            (0.0, 0.0, 1400.0, 1400.0),
        )
        .unwrap();
        let p = plate.to_sphere(2000.0, 3000.0).unwrap();
        let (sx, sy) = screen.from_sphere(&p).expect("in frame");
        // and back the whole way
        let q = screen.to_sphere(sx, sy).unwrap();
        let (px, py) = plate.from_sphere(&q).unwrap();
        assert!((px - 2000.0).abs() < 1e-6 && (py - 3000.0).abs() < 1e-6);
    }
}
