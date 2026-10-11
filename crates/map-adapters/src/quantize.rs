use map_types::UnitVec;

const GRID: f64 = 1e7;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct QPoint {
    pub lon: i64,
    pub lat: i64,
}

impl QPoint {
    pub fn from_lon_lat(lon: f64, lat: f64) -> Self {
        QPoint {
            lon: (lon * GRID).round() as i64,
            lat: (lat * GRID).round() as i64,
        }
    }
    pub fn to_unit_vec(self) -> UnitVec {
        UnitVec::from_lat_lon_deg(self.lat as f64 / GRID, self.lon as f64 / GRID)
    }
}

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
