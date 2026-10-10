//! THE CAMERA, as one value: which chart, where it looks, how wide,
//! and the page it draws on. Every law that reads a camera — the view
//! cap a scene is culled to, the page a name is placed on — reads it
//! from here, so the wire's clamps and margins are stated once.

use crate::geom::{Bbox, UnitVec};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChartKind {
    Globe,
    Flat,
}

impl ChartKind {
    pub fn name(self) -> &'static str {
        match self {
            ChartKind::Globe => "globe",
            ChartKind::Flat => "flat",
        }
    }
}

/// The wire's own latitude limit: a camera at the pole is the camera
/// at 89.9, so a predicate on the unclamped latitude would disagree
/// with the server about where the camera is.
pub const LAT_LIMIT: f64 = 89.9;
/// The wire's zoom frame in degrees of angular radius.
pub const ZOOM_MIN: f64 = 0.05;
pub const ZOOM_MAX: f64 = 90.0;
/// THE PAGE'S DEMAND ENVELOPE, declared here because the cap a scene
/// is cut to must cover everything the page can show at the camera it
/// asks with. The page asks at a zoom rounded to the nearest
/// half-octave, so its true zoom is at most 2^(1/4) times the asked
/// one, and at a centre rounded onto a grid of this share of the zoom,
/// never finer than the floor, so its true centre is at most half the
/// grid's diagonal away.
pub const ZOOM_LADDER_STEP: f64 = 0.5;
pub const CENTER_GRID_SHARE: f64 = 0.4;
pub const CENTER_GRID_FLOOR_DEG: f64 = 0.1;

/// The centre grid's pitch at a zoom, in degrees.
pub fn center_grid_deg(zoom: f64) -> f64 {
    (CENTER_GRID_SHARE * zoom).max(CENTER_GRID_FLOOR_DEG)
}

/// The farthest a visible point can lie from the asked centre, in
/// degrees: the true page's half-diagonal plus half the grid's.
pub fn demand_reach_deg(zoom: f64) -> f64 {
    let true_zoom = zoom * 2f64.powf(ZOOM_LADDER_STEP / 2.0);
    (true_zoom + center_grid_deg(zoom) / 2.0) * 2f64.sqrt()
}

/// YOU CAN PAN AT ANY ZOOM: the cap a scene is cut to reaches the
/// demand envelope of every neighbouring grid cell as well, so one
/// pan step re-demands a manifest but never geometry.
pub fn served_reach_deg(zoom: f64) -> f64 {
    demand_reach_deg(zoom) + center_grid_deg(zoom) * 2f64.sqrt()
}
/// The page every label size is stated against, in px.
pub const DESIGN_WIDTH: f64 = 1200.0;
/// The rendered page's own width frame, in px.
pub const WIDTH_MIN: f64 = 320.0;
pub const WIDTH_MAX: f64 = 8000.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    pub chart: ChartKind,
    pub lat: f64,
    pub lon: f64,
    pub zoom: f64,
    pub width: f64,
}

impl Camera {
    pub fn new(chart: ChartKind, lat: f64, lon: f64, zoom: f64, width: f64) -> Camera {
        Camera {
            chart,
            lat: lat.clamp(-LAT_LIMIT, LAT_LIMIT),
            lon,
            zoom,
            width: width.clamp(WIDTH_MIN, WIDTH_MAX),
        }
    }

    pub fn center(&self) -> UnitVec {
        UnitVec::from_lat_lon_deg(self.lat, self.lon)
    }

    pub fn canon(&self, c: &mut crate::ident::Canon) {
        c.u8_(match self.chart {
            ChartKind::Globe => 0,
            ChartKind::Flat => 1,
        });
        c.f64_(self.lat).f64_(self.lon).f64_(self.zoom).f64_(self.width);
    }

    /// The cap a scene is cut to: the page's demand envelope at this
    /// zoom, never more than the whole sphere.
    pub fn cap(&self) -> Bbox {
        Bbox {
            center: self.center(),
            radius: served_reach_deg(self.zoom.clamp(ZOOM_MIN, ZOOM_MAX))
                .to_radians()
                .min(std::f64::consts::PI),
        }
    }
}
