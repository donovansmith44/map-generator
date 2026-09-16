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
/// The view cap's margin over the nominal zoom: a scene is culled to
/// this cap, generous enough that a pan within it re-demands nothing.
pub const VIEW_MARGIN: f64 = 1.8;
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

    pub fn cap(&self) -> Bbox {
        Bbox {
            center: self.center(),
            radius: (self.zoom.clamp(ZOOM_MIN, ZOOM_MAX) * VIEW_MARGIN)
                .to_radians()
                .min(std::f64::consts::PI),
        }
    }
}
