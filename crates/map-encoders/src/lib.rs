//! Terminal encoders (law 11): the ONLY place concrete output formats
//! exist. Everything upstream speaks Snapshot; each backend here turns
//! a scene into bytes, deterministically — same scene, same config,
//! same bytes — so content-addressed caching survives the encoding
//! boundary. Composition never happens here: overlay and accumulation
//! are semantic operations; encoded artifacts are leaves.
//!
//! Known M1 limit, disclosed: the flat projection draws rings that
//! cross the antimeridian naively (a horizontal streak); a cut-aware
//! projection is future encoder work and touches nothing upstream.

use std::fmt::Write as _;

use map_types::scene::LabelSubject;
use map_types::style::{Rgba, StrokePattern};
use map_types::{EncodeError, Ring, SceneEncoder, Snapshot, UnitVec};

fn lat_lon(v: &UnitVec) -> (f64, f64) {
    (v.z().asin().to_degrees(), v.y().atan2(v.x()).to_degrees())
}

fn bounds(scene: &Snapshot) -> (f64, f64, f64, f64) {
    // (min_lon, min_lat, max_lon, max_lat), defaulting to the world.
    let mut b: Option<(f64, f64, f64, f64)> = None;
    let mut feed = |v: &UnitVec| {
        let (lat, lon) = lat_lon(v);
        b = Some(match b {
            None => (lon, lat, lon, lat),
            Some((x0, y0, x1, y1)) => (x0.min(lon), y0.min(lat), x1.max(lon), y1.max(lat)),
        });
    };
    for r in &scene.regions {
        for ring in r.outer.iter().chain(&r.holes) {
            ring.points().iter().for_each(&mut feed);
        }
    }
    for bd in &scene.boundaries {
        bd.pts.iter().for_each(&mut feed);
    }
    for m in &scene.markers {
        feed(&m.at);
    }
    for l in &scene.labels {
        feed(&l.at);
    }
    b.unwrap_or((-180.0, -90.0, 180.0, 90.0))
}

fn esc(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

// ---------------------------------------------------------------- SVG

/// Flat (equirectangular) SVG plates. Good enough eyes for the bench;
/// projection choice is config, not architecture.
pub struct SvgEncoder {
    /// Rendered width in px; height follows the content's aspect.
    pub width: f64,
    /// Padding around the content, in px.
    pub padding: f64,
}

impl Default for SvgEncoder {
    fn default() -> Self {
        SvgEncoder { width: 1200.0, padding: 16.0 }
    }
}

struct Frame {
    scale: f64,
    x0: f64,
    y1: f64,
    pad: f64,
}

impl Frame {
    fn place(&self, v: &UnitVec) -> (f64, f64) {
        let (lat, lon) = lat_lon(v);
        (self.pad + (lon - self.x0) * self.scale, self.pad + (self.y1 - lat) * self.scale)
    }
}

fn rgb(c: Rgba) -> String {
    format!("rgb({},{},{})", c.0, c.1, c.2)
}
fn alpha(c: Rgba) -> f64 {
    f64::from(c.3) / 255.0
}

fn path_of(frame: &Frame, rings: &[&Ring]) -> String {
    let mut d = String::new();
    for ring in rings {
        for (i, p) in ring.points().iter().enumerate() {
            let (x, y) = frame.place(p);
            let _ = write!(d, "{}{:.3} {:.3}", if i == 0 { "M" } else { "L" }, x, y);
        }
        d.push('Z');
    }
    d
}

impl SceneEncoder for SvgEncoder {
    type Output = String;

    fn encode(&self, scene: &Snapshot) -> Result<String, EncodeError> {
        let (x0, y0, x1, y1) = bounds(scene);
        let (span_x, span_y) = ((x1 - x0).max(1e-6), (y1 - y0).max(1e-6));
        let inner = self.width - 2.0 * self.padding;
        if inner <= 0.0 {
            return Err(EncodeError("width smaller than padding".to_string()));
        }
        let scale = inner / span_x;
        let height = span_y * scale + 2.0 * self.padding;
        let frame = Frame { scale, x0, y1, pad: self.padding };

        let mut s = String::new();
        let _ = write!(
            s,
            "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {:.3} {:.3}\">",
            self.width, height
        );
        // Licensing rides the artifact itself.
        let sources: Vec<String> = scene.attribution.iter().map(|src| src.0.clone()).collect();
        let _ = write!(s, "<desc>sources: {}</desc>", esc(&sources.join(", ")));

        for r in &scene.regions {
            let rings: Vec<&Ring> = r.outer.iter().chain(&r.holes).collect();
            let _ = write!(
                s,
                "<path d=\"{}\" fill=\"{}\" fill-opacity=\"{:.3}\" fill-rule=\"evenodd\"/>",
                path_of(&frame, &rings),
                rgb(r.paint.fill),
                alpha(r.paint.fill)
            );
        }
        for b in &scene.boundaries {
            let mut d = String::new();
            for (i, p) in b.pts.iter().enumerate() {
                let (x, y) = frame.place(p);
                let _ = write!(d, "{}{:.3} {:.3}", if i == 0 { "M" } else { "L" }, x, y);
            }
            let st = b.stroke;
            let (width, dash, opacity) = match st.pattern {
                StrokePattern::Solid => (st.width, String::new(), alpha(st.color)),
                StrokePattern::Dashed => (st.width, " stroke-dasharray=\"6 4\"".into(), alpha(st.color)),
                StrokePattern::Hatched => (st.width, " stroke-dasharray=\"2 3\"".into(), alpha(st.color)),
                // A frontier is a zone, not a line: broad and soft.
                StrokePattern::Zonal => (st.width * 6.0, String::new(), alpha(st.color) * 0.35),
            };
            let _ = write!(
                s,
                "<path d=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{:.3}\" stroke-opacity=\"{:.3}\" stroke-linejoin=\"round\" stroke-linecap=\"round\"{}/>",
                d,
                rgb(st.color),
                width,
                opacity,
                dash
            );
        }
        for m in &scene.markers {
            let (x, y) = frame.place(&m.at);
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
        for l in &scene.labels {
            let (x, y) = frame.place(&l.at);
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
        s.push_str("</svg>");
        Ok(s)
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
