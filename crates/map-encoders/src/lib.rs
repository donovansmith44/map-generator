//! Terminal encoders (law 11): the ONLY place concrete output formats
//! exist. Everything upstream speaks Snapshot; each backend here turns
//! a scene into bytes, deterministically — same scene, same config,
//! same bytes — so content-addressed caching survives the encoding
//! boundary. Composition never happens here: overlay and accumulation
//! are semantic operations; encoded artifacts are leaves.
//!
//! Geometry lives on the sphere; PROJECTION IS ENCODER CONFIG, never
//! architecture. The globe projection is the default: an orthographic
//! view centered on the content, back hemisphere clipped, graticule
//! for the curve of the earth — a region renders as a surface slice of
//! the sphere. The flat (equirectangular) plate remains for whole-world
//! diagnostics; it draws antimeridian-crossing rings naively (disclosed).

use std::fmt::Write as _;

use map_types::scene::LabelSubject;
use map_types::style::{Rgba, StrokePattern};
use map_types::{EncodeError, Ring, SceneEncoder, Snapshot, UnitVec};

fn lat_lon(v: &UnitVec) -> (f64, f64) {
    (v.z().asin().to_degrees(), v.y().atan2(v.x()).to_degrees())
}

fn esc(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

fn rgb(c: Rgba) -> String {
    format!("rgb({},{},{})", c.0, c.1, c.2)
}
fn alpha(c: Rgba) -> f64 {
    f64::from(c.3) / 255.0
}

fn scene_points<'a>(scene: &'a Snapshot) -> impl Iterator<Item = &'a UnitVec> {
    scene
        .regions
        .iter()
        .flat_map(|r| r.outer.iter().chain(&r.holes).flat_map(|ring| ring.points()))
        .chain(scene.boundaries.iter().flat_map(|b| b.pts.iter()))
        .chain(scene.markers.iter().map(|m| &m.at))
        .chain(scene.labels.iter().map(|l| &l.at))
}

// ---------------------------------------------------------- projection

/// How the sphere meets the page. Config, not architecture: adding a
/// projection touches nothing upstream of the encoder.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Projection {
    /// Orthographic globe: the view a sphere actually offers. Centered
    /// on the given (lat, lon) or, with None, on the content itself;
    /// `zoom` is the view's angular radius in degrees (90 = the whole
    /// hemisphere) or, with None, fit to the content's extent — so one
    /// region is a surface slice and the world is the lit hemisphere.
    /// The resolved view rides the svg root as data attributes, so an
    /// interactive consumer can seed its navigation from the artifact.
    Globe { center: Option<(f64, f64)>, zoom: Option<f64> },
    /// Equirectangular plate for diagnostics.
    Flat,
}

pub struct SvgEncoder {
    /// Rendered width in px; height follows (square for the globe).
    pub width: f64,
    /// Padding around the content, in px.
    pub padding: f64,
    pub projection: Projection,
}

impl Default for SvgEncoder {
    fn default() -> Self {
        SvgEncoder {
            width: 1200.0,
            padding: 16.0,
            projection: Projection::Globe { center: None, zoom: None },
        }
    }
}

// -------------------------------------------------- shared svg pieces

fn stroke_attrs(st: map_types::style::Stroke) -> (f64, &'static str, f64) {
    match st.pattern {
        StrokePattern::Solid => (st.width, "", alpha(st.color)),
        StrokePattern::Dashed => (st.width, " stroke-dasharray=\"6 4\"", alpha(st.color)),
        StrokePattern::Hatched => (st.width, " stroke-dasharray=\"2 3\"", alpha(st.color)),
        // A frontier is a zone, not a line: broad and soft.
        StrokePattern::Zonal => (st.width * 6.0, "", alpha(st.color) * 0.35),
    }
}

fn path_from(chunks: &[Vec<(f64, f64)>], close: bool) -> String {
    let mut d = String::new();
    for pts in chunks {
        for (i, (x, y)) in pts.iter().enumerate() {
            let _ = write!(d, "{}{:.3} {:.3}", if i == 0 { "M" } else { "L" }, x, y);
        }
        if close {
            d.push('Z');
        }
    }
    d
}

/// Emit every scene element through a projector. `ring_of` returns the
/// page-space rings of a Ring (empty when fully out of view);
/// `line_of` the visible runs of a polyline; `point_of` a visible point.
fn emit_scene(
    s: &mut String,
    scene: &Snapshot,
    ring_of: &dyn Fn(&Ring) -> Vec<Vec<(f64, f64)>>,
    line_of: &dyn Fn(&[UnitVec]) -> Vec<Vec<(f64, f64)>>,
    point_of: &dyn Fn(&UnitVec) -> Option<(f64, f64)>,
) {
    for r in &scene.regions {
        let mut chunks: Vec<Vec<(f64, f64)>> = Vec::new();
        for ring in r.outer.iter().chain(&r.holes) {
            chunks.extend(ring_of(ring));
        }
        if chunks.is_empty() {
            continue;
        }
        let _ = write!(
            s,
            "<path data-region=\"{:016x}\" d=\"{}\" fill=\"{}\" fill-opacity=\"{:.3}\" fill-rule=\"evenodd\"/>",
            r.region.0 .0,
            path_from(&chunks, true),
            rgb(r.paint.fill),
            alpha(r.paint.fill)
        );
    }
    for b in &scene.boundaries {
        let chunks = line_of(&b.pts);
        if chunks.is_empty() {
            continue;
        }
        let (width, dash, opacity) = stroke_attrs(b.stroke);
        let _ = write!(
            s,
            "<path d=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{:.3}\" stroke-opacity=\"{:.3}\" stroke-linejoin=\"round\" stroke-linecap=\"round\"{}/>",
            path_from(&chunks, false),
            rgb(b.stroke.color),
            width,
            opacity,
            dash
        );
    }
    for m in &scene.markers {
        if let Some((x, y)) = point_of(&m.at) {
            let _ = write!(
                s,
                "<circle cx=\"{:.3}\" cy=\"{:.3}\" r=\"{:.3}\" fill=\"{}\" fill-opacity=\"{:.3}\"/>",
                x,
                y,
                m.style.size,
                rgb(m.style.color),
                alpha(m.style.color)
            );
        }
    }
    for l in &scene.labels {
        if let Some((x, y)) = point_of(&l.at) {
            let _ = write!(
                s,
                "<text x=\"{:.3}\" y=\"{:.3}\" font-size=\"{:.3}\" fill=\"{}\" fill-opacity=\"{:.3}\" text-anchor=\"middle\" font-family=\"ui-monospace,monospace\">{}</text>",
                x,
                y,
                l.style.size,
                rgb(l.style.color),
                alpha(l.style.color),
                esc(&l.text)
            );
        }
    }
}

fn svg_head(width: f64, height: f64, scene: &Snapshot) -> String {
    let sources: Vec<String> = scene.attribution.iter().map(|src| src.0.clone()).collect();
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {:.3} {:.3}\"><desc>sources: {}</desc>",
        width,
        height,
        esc(&sources.join(", "))
    )
}

// ------------------------------------------------------- globe plumbing

struct Globe {
    center: UnitVec,
    east: UnitVec,
    north: UnitVec,
    /// px per projected sin-unit.
    scale: f64,
    cx: f64,
    cy: f64,
}

impl Globe {
    /// Orthographic page coordinates; None when behind the limb.
    fn place(&self, p: &UnitVec) -> Option<(f64, f64)> {
        if p.dot(&self.center) < 0.0 {
            return None;
        }
        Some(self.place_unclipped(p))
    }
    fn place_unclipped(&self, p: &UnitVec) -> (f64, f64) {
        (self.cx + p.dot(&self.east) * self.scale, self.cy - p.dot(&self.north) * self.scale)
    }
    fn radial(&self, p: &UnitVec) -> f64 {
        let (x, y) = (p.dot(&self.east), p.dot(&self.north));
        (x * x + y * y).sqrt()
    }
}

fn front_crossing(a: &UnitVec, b: &UnitVec, c: &UnitVec) -> UnitVec {
    // Where the segment a->b pierces the limb plane dot(p, c) = 0.
    let (da, db) = (a.dot(c), b.dot(c));
    let t = if (da - db).abs() < 1e-12 { 0.5 } else { da / (da - db) };
    UnitVec::normalize(
        a.x() + t * (b.x() - a.x()),
        a.y() + t * (b.y() - a.y()),
        a.z() + t * (b.z() - a.z()),
    )
    .unwrap_or(*a)
}

/// Sutherland–Hodgman against the limb plane: the front-hemisphere
/// part of a closed ring (empty when fully behind). The cut edge runs
/// straight between limb points — a chord of the disc, close enough
/// for plates.
fn clip_ring_front(pts: &[UnitVec], c: &UnitVec) -> Vec<UnitVec> {
    let mut out: Vec<UnitVec> = Vec::new();
    let n = pts.len();
    for i in 0..n {
        let (a, b) = (&pts[i], &pts[(i + 1) % n]);
        let (fa, fb) = (a.dot(c) >= 0.0, b.dot(c) >= 0.0);
        if fa {
            out.push(*a);
        }
        if fa != fb {
            out.push(front_crossing(a, b, c));
        }
    }
    out
}

/// The visible runs of an open polyline, split at the limb.
fn clip_line_front(pts: &[UnitVec], c: &UnitVec) -> Vec<Vec<UnitVec>> {
    let mut runs = Vec::new();
    let mut run: Vec<UnitVec> = Vec::new();
    for i in 0..pts.len() {
        let front = pts[i].dot(c) >= 0.0;
        if front {
            if run.is_empty() && i > 0 {
                run.push(front_crossing(&pts[i - 1], &pts[i], c));
            }
            run.push(pts[i]);
        } else if !run.is_empty() {
            run.push(front_crossing(&pts[i - 1], &pts[i], c));
            runs.push(std::mem::take(&mut run));
        }
    }
    if !run.is_empty() {
        runs.push(run);
    }
    runs
}

fn content_center(scene: &Snapshot) -> UnitVec {
    let (mut x, mut y, mut z) = (0.0, 0.0, 0.0);
    for p in scene_points(scene) {
        x += p.x();
        y += p.y();
        z += p.z();
    }
    UnitVec::normalize(x, y, z).unwrap_or_else(|_| UnitVec::from_lat_lon_deg(20.0, 20.0))
}

fn graticule() -> Vec<Vec<UnitVec>> {
    let mut lines = Vec::new();
    for lon in (-180..180).step_by(15) {
        lines.push(
            (-90..=90)
                .step_by(3)
                .map(|lat| UnitVec::from_lat_lon_deg(f64::from(lat), f64::from(lon)))
                .collect(),
        );
    }
    for lat in (-75..=75).step_by(15) {
        lines.push(
            (-180..=180)
                .step_by(3)
                .map(|lon| UnitVec::from_lat_lon_deg(f64::from(lat), f64::from(lon)))
                .collect(),
        );
    }
    lines
}

fn encode_globe(enc: &SvgEncoder, scene: &Snapshot, center: UnitVec, zoom: Option<f64>) -> String {
    // Basis on the sphere at the view center.
    let east = UnitVec::normalize(-center.y(), center.x(), 0.0)
        .unwrap_or_else(|_| UnitVec::from_lat_lon_deg(0.0, 90.0)); // pole-on view
    let (nx, ny, nz) = center.cross_raw(&east);
    let north = UnitVec::normalize(nx, ny, nz).unwrap_or_else(|_| UnitVec::from_lat_lon_deg(90.0, 0.0));

    // Zoom to the visible content's angular extent; the whole
    // hemisphere when content reaches (or wraps) the limb.
    let mut globe = Globe { center, east, north, scale: 1.0, cx: 0.0, cy: 0.0 };
    let r_view = match zoom {
        Some(deg) => deg.clamp(1.0, 90.0).to_radians().sin(),
        None => {
            let mut r_max: f64 = 0.0;
            let mut any_behind = false;
            for p in scene_points(scene) {
                if p.dot(&center) >= 0.0 {
                    r_max = r_max.max(globe.radial(p));
                } else {
                    any_behind = true;
                }
            }
            if any_behind || r_max <= 0.0 {
                1.0
            } else {
                (r_max * 1.08).min(1.0)
            }
        }
    };
    let inner = enc.width - 2.0 * enc.padding;
    globe.scale = inner / 2.0 / r_view;
    globe.cx = enc.width / 2.0;
    globe.cy = enc.width / 2.0;

    // Report the resolved view on the root, so interactive consumers
    // can pick up navigation exactly where this artifact stands.
    let (clat, clon) = lat_lon(&center);
    let mut s = svg_head(enc.width, enc.width, scene).replace(
        "<svg ",
        &format!(
            "<svg data-clat=\"{:.3}\" data-clon=\"{:.3}\" data-zoom=\"{:.3}\" ",
            clat,
            clon,
            r_view.clamp(-1.0, 1.0).asin().to_degrees()
        ),
    );

    // The sphere itself: limb circle when the full hemisphere is in
    // view, and the graticule always — the curve is the point.
    if r_view >= 1.0 {
        let _ = write!(
            s,
            "<circle cx=\"{:.3}\" cy=\"{:.3}\" r=\"{:.3}\" fill=\"rgb(128,128,128)\" fill-opacity=\"0.06\" stroke=\"rgb(128,128,128)\" stroke-opacity=\"0.5\" stroke-width=\"1\"/>",
            globe.cx, globe.cy, globe.scale
        );
    }
    let mut grat = String::new();
    for line in graticule() {
        for run in clip_line_front(&line, &center) {
            let pts: Vec<(f64, f64)> = run.iter().map(|p| globe.place_unclipped(p)).collect();
            let _ = write!(grat, "{}", path_from(&[pts], false));
        }
    }
    let _ = write!(
        s,
        "<path d=\"{}\" fill=\"none\" stroke=\"rgb(128,128,128)\" stroke-opacity=\"0.22\" stroke-width=\"0.6\"/>",
        grat
    );

    let ring_of = |ring: &Ring| -> Vec<Vec<(f64, f64)>> {
        let clipped = clip_ring_front(ring.points(), &center);
        if clipped.len() < 3 {
            return Vec::new();
        }
        vec![clipped.iter().map(|p| globe.place_unclipped(p)).collect()]
    };
    let line_of = |pts: &[UnitVec]| -> Vec<Vec<(f64, f64)>> {
        clip_line_front(pts, &center)
            .into_iter()
            .map(|run| run.iter().map(|p| globe.place_unclipped(p)).collect())
            .collect()
    };
    let point_of = |p: &UnitVec| globe.place(p);
    emit_scene(&mut s, scene, &ring_of, &line_of, &point_of);
    s.push_str("</svg>");
    s
}

// ------------------------------------------------------------ flat plate

fn flat_bounds(scene: &Snapshot) -> (f64, f64, f64, f64) {
    let mut b: Option<(f64, f64, f64, f64)> = None;
    for v in scene_points(scene) {
        let (lat, lon) = lat_lon(v);
        b = Some(match b {
            None => (lon, lat, lon, lat),
            Some((x0, y0, x1, y1)) => (x0.min(lon), y0.min(lat), x1.max(lon), y1.max(lat)),
        });
    }
    b.unwrap_or((-180.0, -90.0, 180.0, 90.0))
}

fn encode_flat(enc: &SvgEncoder, scene: &Snapshot) -> Result<String, EncodeError> {
    let (x0, _y0, x1, y1) = flat_bounds(scene);
    let span_x = (x1 - x0).max(1e-6);
    let span_y = (y1 - _y0).max(1e-6);
    let inner = enc.width - 2.0 * enc.padding;
    if inner <= 0.0 {
        return Err(EncodeError("width smaller than padding".to_string()));
    }
    let scale = inner / span_x;
    let height = span_y * scale + 2.0 * enc.padding;
    let place = move |p: &UnitVec| -> (f64, f64) {
        let (lat, lon) = lat_lon(p);
        (enc.padding + (lon - x0) * scale, enc.padding + (y1 - lat) * scale)
    };

    let mut s = svg_head(enc.width, height, scene);
    let ring_of = |ring: &Ring| -> Vec<Vec<(f64, f64)>> {
        vec![ring.points().iter().map(place).collect()]
    };
    let line_of =
        |pts: &[UnitVec]| -> Vec<Vec<(f64, f64)>> { vec![pts.iter().map(place).collect()] };
    let point_of = |p: &UnitVec| Some(place(p));
    emit_scene(&mut s, scene, &ring_of, &line_of, &point_of);
    s.push_str("</svg>");
    Ok(s)
}

impl SceneEncoder for SvgEncoder {
    type Output = String;

    fn encode(&self, scene: &Snapshot) -> Result<String, EncodeError> {
        if self.width <= 2.0 * self.padding {
            return Err(EncodeError("width smaller than padding".to_string()));
        }
        match self.projection {
            Projection::Flat => encode_flat(self, scene),
            Projection::Globe { center, zoom } => {
                let c = match center {
                    Some((lat, lon)) => UnitVec::from_lat_lon_deg(lat, lon),
                    None => content_center(scene),
                };
                Ok(encode_globe(self, scene, c, zoom))
            }
        }
    }
}

// ------------------------------------------------------------- GeoJSON

/// Feature-collection output for downstream tooling. Regions become
/// MultiPolygons, strokes LineStrings, markers and labels Points.
pub struct GeoJsonEncoder;

fn ring_coords(ring: &Ring) -> serde_json::Value {
    let mut coords: Vec<serde_json::Value> = ring
        .points()
        .iter()
        .map(|p| {
            let (lat, lon) = lat_lon(p);
            serde_json::json!([lon, lat])
        })
        .collect();
    coords.push(coords[0].clone()); // the closing repeat the format expects
    serde_json::Value::Array(coords)
}

impl SceneEncoder for GeoJsonEncoder {
    type Output = String;

    fn encode(&self, scene: &Snapshot) -> Result<String, EncodeError> {
        let mut features = Vec::new();
        for r in &scene.regions {
            let polys: Vec<serde_json::Value> = r
                .outer
                .iter()
                .map(|o| {
                    let mut rings = vec![ring_coords(o)];
                    rings.extend(r.holes.iter().map(ring_coords));
                    serde_json::Value::Array(rings)
                })
                .collect();
            features.push(serde_json::json!({
                "type": "Feature",
                "properties": {
                    "kind": "region",
                    "id": format!("{:016x}", r.region.0 .0),
                    "fill": format!("#{:02x}{:02x}{:02x}", r.paint.fill.0, r.paint.fill.1, r.paint.fill.2),
                },
                "geometry": { "type": "MultiPolygon", "coordinates": polys }
            }));
        }
        for b in &scene.boundaries {
            let line: Vec<serde_json::Value> = b
                .pts
                .iter()
                .map(|p| {
                    let (lat, lon) = lat_lon(p);
                    serde_json::json!([lon, lat])
                })
                .collect();
            features.push(serde_json::json!({
                "type": "Feature",
                "properties": { "kind": "boundary", "id": format!("{:016x}", b.boundary.0 .0) },
                "geometry": { "type": "LineString", "coordinates": line }
            }));
        }
        for m in &scene.markers {
            let (lat, lon) = lat_lon(&m.at);
            features.push(serde_json::json!({
                "type": "Feature",
                "properties": { "kind": "marker" },
                "geometry": { "type": "Point", "coordinates": [lon, lat] }
            }));
        }
        for l in &scene.labels {
            let (lat, lon) = lat_lon(&l.at);
            let subject = match &l.subject {
                LabelSubject::Region(r) => format!("region:{:016x}", r.0 .0),
                LabelSubject::Boundary(b) => format!("boundary:{:016x}", b.0 .0),
                LabelSubject::Free => "free".to_string(),
            };
            features.push(serde_json::json!({
                "type": "Feature",
                "properties": { "kind": "label", "text": l.text, "subject": subject },
                "geometry": { "type": "Point", "coordinates": [lon, lat] }
            }));
        }
        let sources: Vec<String> = scene.attribution.iter().map(|s| s.0.clone()).collect();
        let doc = serde_json::json!({
            "type": "FeatureCollection",
            "attribution": sources,
            "features": features
        });
        Ok(doc.to_string())
    }
}

#[cfg(test)]
mod tests;
