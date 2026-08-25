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
use map_types::{EncodeError, Ring, SceneEncoder, Snapshot, TransitionEncoder, UnitVec};
use map_types::{TransitionScript, TransitionStep};

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
    /// Cartographer's finish: paths render as curves THROUGH every
    /// attested vertex (an interpolating spline — no data point moves,
    /// corners soften). Off = raw polylines.
    pub smooth: bool,
}

impl Default for SvgEncoder {
    fn default() -> Self {
        SvgEncoder {
            width: 1200.0,
            padding: 16.0,
            projection: Projection::Globe { center: None, zoom: None },
            smooth: true,
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
            let _ = write!(d, "{}{:.1} {:.1}", if i == 0 { "M" } else { "L" }, x, y);
        }
        if close {
            d.push('Z');
        }
    }
    d
}

/// Catmull-Rom rendered as cubic curves: an INTERPOLATING spline — the
/// path passes through every real vertex, only the corners between
/// them soften. Endpoints stay pinned; short chunks fall back straight.
fn path_smooth(chunks: &[Vec<(f64, f64)>], close: bool) -> String {
    let mut d = String::new();
    for pts in chunks {
        let n = pts.len();
        if n < 3 {
            let _ = write!(d, "{}", path_from(std::slice::from_ref(pts), close));
            continue;
        }
        let at = |i: isize| -> (f64, f64) {
            if close {
                pts[i.rem_euclid(n as isize) as usize]
            } else {
                pts[i.clamp(0, n as isize - 1) as usize]
            }
        };
        let last = if close { n } else { n - 1 };
        let (x0, y0) = pts[0];
        let _ = write!(d, "M{x0:.1} {y0:.1}");
        for i in 0..last {
            let p0 = at(i as isize - 1);
            let p1 = at(i as isize);
            let p2 = at(i as isize + 1);
            let p3 = at(i as isize + 2);
            let c1 = (p1.0 + (p2.0 - p0.0) / 6.0, p1.1 + (p2.1 - p0.1) / 6.0);
            let c2 = (p2.0 - (p3.0 - p1.0) / 6.0, p2.1 - (p3.1 - p1.1) / 6.0);
            let _ = write!(
                d,
                "C{:.1} {:.1} {:.1} {:.1} {:.1} {:.1}",
                c1.0, c1.1, c2.0, c2.1, p2.0, p2.1
            );
        }
        if close {
            d.push('Z');
        }
    }
    d
}

fn content_path(chunks: &[Vec<(f64, f64)>], close: bool, smooth: bool) -> String {
    if smooth {
        path_smooth(chunks, close)
    } else {
        path_from(chunks, close)
    }
}

/// Subdivide along great circles so no chord strays visibly from the
/// sphere: precision by construction, adapted to the render scale.
fn densify(pts: &[UnitVec], max_step: f64, closed: bool) -> Vec<UnitVec> {
    let n = pts.len();
    if n < 2 {
        return pts.to_vec();
    }
    let edges = if closed { n } else { n - 1 };
    let mut out = Vec::with_capacity(n * 2);
    for i in 0..edges {
        let (p, q) = (&pts[i], &pts[(i + 1) % n]);
        out.push(*p);
        let angle = p.angle_to(q);
        if angle > max_step {
            let steps = (angle / max_step).ceil() as usize;
            for k in 1..steps {
                if let Ok(mid) = map_types::slerp(p, q, k as f64 / steps as f64) {
                    out.push(mid);
                }
            }
        }
    }
    if !closed {
        out.push(pts[n - 1]);
    }
    out
}

type Bounds = (f64, f64, f64, f64);

fn grow(b: &mut Option<Bounds>, (x, y): (f64, f64)) {
    *b = Some(match *b {
        None => (x, y, x, y),
        Some((x0, y0, x1, y1)) => (x0.min(x), y0.min(y), x1.max(x), y1.max(y)),
    });
}

/// Culling against the page (performance honesty: a path the page
/// cannot show is pure parse cost downstream). A chunk survives when
/// its bbox touches the padded page OR swallows it whole — a ring
/// around the viewport still fills the viewport; one entirely beside
/// it cannot paint a single visible pixel.
fn chunk_matters(b: &Bounds, page: &Option<Bounds>) -> bool {
    let Some(p) = page else { return true };
    let touches = b.0 <= p.2 && p.0 <= b.2 && b.1 <= p.3 && p.1 <= b.3;
    let swallows = b.0 <= p.0 && b.1 <= p.1 && b.2 >= p.2 && b.3 >= p.3;
    touches || swallows
}

fn bbox_of(pts: &[(f64, f64)]) -> Option<Bounds> {
    let mut b = None;
    for pt in pts {
        grow(&mut b, *pt);
    }
    b
}

/// Below this size on the page a shape cannot resolve; emitting it is
/// bytes without pixels.
const SUBPIXEL: f64 = 0.7;

fn subpixel(b: &Bounds) -> bool {
    b.2 - b.0 < SUBPIXEL && b.3 - b.1 < SUBPIXEL
}

/// Emit fills, strokes, and markers through a projector; returns each
/// region's projected extent for the label pass. `ring_of` returns the
/// page-space rings of a Ring (empty when fully out of view);
/// `line_of` the visible runs of a polyline; `point_of` a visible point.
/// `page` is the padded viewport for culling (None = emit everything).
fn emit_scene(
    s: &mut String,
    scene: &Snapshot,
    ring_of: &dyn Fn(&Ring) -> Vec<Vec<(f64, f64)>>,
    line_of: &dyn Fn(&[UnitVec]) -> Vec<Vec<(f64, f64)>>,
    point_of: &dyn Fn(&UnitVec) -> Option<(f64, f64)>,
    smooth: bool,
    page: &Option<Bounds>,
) -> std::collections::BTreeMap<u64, Bounds> {
    let mut extents: std::collections::BTreeMap<u64, Bounds> = Default::default();
    for r in &scene.regions {
        let mut chunks: Vec<Vec<(f64, f64)>> = Vec::new();
        for ring in r.outer.iter().chain(&r.holes) {
            // Per-ring culling is parity-safe under evenodd: a ring
            // that neither touches nor swallows the page contributes
            // zero winding to every visible pixel.
            for chunk in ring_of(ring) {
                if bbox_of(&chunk).is_some_and(|cb| chunk_matters(&cb, page)) {
                    chunks.push(chunk);
                }
            }
        }
        if chunks.is_empty() {
            continue;
        }
        let mut b: Option<Bounds> = None;
        for pt in chunks.iter().flatten() {
            grow(&mut b, *pt);
        }
        if b.as_ref().is_some_and(subpixel) {
            continue;
        }
        if let Some(b) = b {
            // Later layers of the same region only widen its extent.
            extents
                .entry(r.region.0 .0)
                .and_modify(|e| {
                    *e = (e.0.min(b.0), e.1.min(b.1), e.2.max(b.2), e.3.max(b.3));
                })
                .or_insert(b);
        }
        let _ = write!(
            s,
            "<path data-region=\"{:016x}\" d=\"{}\" fill=\"{}\" fill-opacity=\"{:.3}\" fill-rule=\"evenodd\"/>",
            r.region.0 .0,
            content_path(&chunks, true, smooth),
            rgb(r.paint.fill),
            alpha(r.paint.fill)
        );
    }
    for b in &scene.boundaries {
        // A densified run that crosses the page has points ON the
        // page, so a bbox-disjoint run is genuinely invisible; and a
        // subpixel run resolves to nothing under its own stroke.
        let chunks: Vec<Vec<(f64, f64)>> = line_of(&b.pts)
            .into_iter()
            .filter(|chunk| {
                bbox_of(chunk)
                    .is_some_and(|cb| chunk_matters(&cb, page) && !subpixel(&cb))
            })
            .collect();
        if chunks.is_empty() {
            continue;
        }
        let (width, dash, opacity) = stroke_attrs(b.stroke);
        let _ = write!(
            s,
            "<path d=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{:.3}\" stroke-opacity=\"{:.3}\" stroke-linejoin=\"round\" stroke-linecap=\"round\"{}/>",
            content_path(&chunks, false, smooth),
            rgb(b.stroke.color),
            width,
            opacity,
            dash
        );
    }
    for m in &scene.markers {
        if let Some((x, y)) = point_of(&m.at) {
            if !chunk_matters(&(x, y, x, y), page) {
                continue;
            }
            let _ = write!(
                s,
                "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"{:.1}\" fill=\"{}\" fill-opacity=\"{:.3}\"/>",
                x,
                y,
                m.style.size,
                rgb(m.style.color),
                alpha(m.style.color)
            );
        }
    }
    extents
}

/// The label pass: a label must FIT what it names and never collide.
/// Region labels shrink to their territory's projected extent and are
/// DROPPED when even the minimum readable size will not fit; every
/// candidate then tries its anchor and small vertical nudges, and
/// yields rather than overlap an earlier label. Deterministic: scene
/// order is placement priority.
fn emit_labels(
    s: &mut String,
    scene: &Snapshot,
    extents: &std::collections::BTreeMap<u64, Bounds>,
    point_of: &dyn Fn(&UnitVec) -> Option<(f64, f64)>,
    page_width: f64,
    page: &Option<Bounds>,
) {
    const CHAR_WIDTH: f64 = 0.62; // monospace em fraction
    let min_size = (page_width / 260.0).max(4.0);
    let mut placed: Vec<Bounds> = Vec::new();
    let collides = |b: &Bounds, placed: &[Bounds]| {
        placed.iter().any(|p| b.0 < p.2 && p.0 < b.2 && b.1 < p.3 && p.1 < b.3)
    };
    for l in &scene.labels {
        let Some((x, y)) = point_of(&l.at) else { continue };
        if !chunk_matters(&(x, y, x, y), page) {
            continue; // anchored off the page: unreadable by definition
        }
        let chars = l.text.chars().count().max(1) as f64;
        let mut size = l.style.size;
        if let map_types::scene::LabelSubject::Region(rid) = &l.subject {
            if let Some((x0, y0, x1, y1)) = extents.get(&rid.0 .0) {
                let fit_w = (x1 - x0) * 0.92 / (chars * CHAR_WIDTH);
                let fit_h = (y1 - y0) * 0.8;
                size = size.min(fit_w).min(fit_h);
            }
        }
        if size < min_size {
            continue; // the territory cannot hold a readable label
        }
        let (w, h) = (chars * CHAR_WIDTH * size, size * 1.25);
        let mut spot = None;
        for dy in [0.0, -h, h, -2.0 * h, 2.0 * h] {
            let b = (x - w / 2.0, y + dy - h * 0.75, x + w / 2.0, y + dy + h * 0.35);
            if !collides(&b, &placed) {
                spot = Some((y + dy, b));
                break;
            }
        }
        let Some((y, b)) = spot else { continue };
        placed.push(b);
        let _ = write!(
            s,
            "<text x=\"{:.1}\" y=\"{:.1}\" font-size=\"{:.1}\" fill=\"{}\" fill-opacity=\"{:.3}\" stroke=\"{}\" stroke-opacity=\"{:.3}\" stroke-width=\"{:.3}\" paint-order=\"stroke\" text-anchor=\"middle\" font-family=\"ui-monospace,monospace\">{}</text>",
            x,
            y,
            size,
            rgb(l.style.color),
            alpha(l.style.color),
            rgb(l.style.halo),
            alpha(l.style.halo),
            size * 0.28,
            esc(&l.text)
        );
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

/// Clip a closed ring to the front hemisphere. Where the ring passes
/// behind, the cut edge follows the LIMB, sweeping the same way around
/// the view axis as the hidden stretch actually travels (accumulated
/// azimuth) — exact for any simple ring, including the world ocean's
/// sphere-wrapping envelope, with no special cases.
fn clip_ring_front(pts: &[UnitVec], c: &UnitVec) -> Vec<UnitVec> {
    let n = pts.len();
    if n < 3 {
        return Vec::new();
    }
    // Azimuth basis around the view axis (pole-safe fallback).
    let east = UnitVec::normalize(-c.y(), c.x(), 0.0)
        .unwrap_or_else(|_| UnitVec::from_lat_lon_deg(0.0, 90.0));
    let ncr = c.cross_raw(&east);
    let north = UnitVec::normalize(ncr.0, ncr.1, ncr.2)
        .unwrap_or_else(|_| UnitVec::from_lat_lon_deg(90.0, 0.0));
    let azimuth = |p: &UnitVec| -> f64 { p.dot(&north).atan2(p.dot(&east)) };
    let wrap = |d: f64| -> f64 {
        let mut d = d % std::f64::consts::TAU;
        if d > std::f64::consts::PI {
            d -= std::f64::consts::TAU;
        }
        if d < -std::f64::consts::PI {
            d += std::f64::consts::TAU;
        }
        d
    };
    let limb_point = |theta: f64| -> UnitVec {
        UnitVec::normalize(
            east.x() * theta.cos() + north.x() * theta.sin(),
            east.y() * theta.cos() + north.y() * theta.sin(),
            east.z() * theta.cos() + north.z() * theta.sin(),
        )
        .expect("limb basis is orthonormal")
    };

    let front = |p: &UnitVec| p.dot(c) >= 0.0;
    if pts.iter().all(front) {
        return pts.to_vec();
    }
    let Some(start) = (0..n).position(|i| front(&pts[i])) else {
        return Vec::new(); // wholly behind
    };

    let mut out: Vec<UnitVec> = Vec::with_capacity(n);
    let mut i = start;
    let mut walked = 0usize;
    while walked < n {
        let p = pts[i % n];
        if front(&p) {
            out.push(p);
            i += 1;
            walked += 1;
            continue;
        }
        // A hidden stretch begins: exit crossing, azimuth sweep of the
        // hidden path, entry crossing, limb arc between them.
        let exit = front_crossing(&pts[(i + n - 1) % n], &pts[i % n], c);
        let mut sweep = 0.0;
        let mut prev = azimuth(&exit);
        while walked < n && !front(&pts[i % n]) {
            let a = azimuth(&pts[i % n]);
            sweep += wrap(a - prev);
            prev = a;
            i += 1;
            walked += 1;
        }
        let entry = front_crossing(&pts[(i + n - 1) % n], &pts[i % n], c);
        sweep += wrap(azimuth(&entry) - prev);
        out.push(exit);
        let start_az = azimuth(&exit);
        let steps = (sweep.abs() / 0.06).ceil().max(1.0) as usize;
        for k in 1..steps {
            out.push(limb_point(start_az + sweep * k as f64 / steps as f64));
        }
        out.push(entry);
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
    // Chord error stays under ~0.75px at this scale: precision follows
    // resolution, so a wider plate is genuinely finer.
    let max_step = (6.0 / globe.scale).sqrt().clamp(0.0015, 0.05);

    let mut grat = String::new();
    for line in graticule() {
        for run in clip_line_front(&densify(&line, max_step, false), &center) {
            let pts: Vec<(f64, f64)> = run.iter().map(|p| globe.place_unclipped(p)).collect();
            let _ = write!(grat, "{}", path_from(&[pts], false));
        }
    }
    let _ = write!(
        s,
        "<path d=\"{}\" fill=\"none\" stroke=\"rgb(128,128,128)\" stroke-opacity=\"0.22\" stroke-width=\"0.6\"/>",
        grat
    );

    // The whole-sphere sentinel (a ring with near-antipodal points —
    // see RegionPart's empty-cycle convention) projects as the LIMB
    // DISC: everything in view; its holes cut the land out (evenodd).
    let covers_sphere = |pts: &[UnitVec]| -> bool {
        pts.len() <= 4 && pts.iter().any(|a| pts.iter().any(|b| a.dot(b) < -0.99))
    };
    let disc: Vec<(f64, f64)> = (0..=127)
        .map(|i| {
            let a = std::f64::consts::TAU * f64::from(i) / 128.0;
            (globe.cx + globe.scale * a.cos(), globe.cy + globe.scale * a.sin())
        })
        .collect();
    let ring_of = |ring: &Ring| -> Vec<Vec<(f64, f64)>> {
        if covers_sphere(ring.points()) {
            return vec![disc.clone()];
        }
        let clipped = clip_ring_front(&densify(ring.points(), max_step, true), &center);
        if clipped.len() < 3 {
            return Vec::new();
        }
        vec![clipped.iter().map(|p| globe.place_unclipped(p)).collect()]
    };
    let line_of = |pts: &[UnitVec]| -> Vec<Vec<(f64, f64)>> {
        clip_line_front(&densify(pts, max_step, false), &center)
            .into_iter()
            .map(|run| run.iter().map(|p| globe.place_unclipped(p)).collect())
            .collect()
    };
    let point_of = |p: &UnitVec| globe.place(p);
    // Cull to the page plus a stroke-and-label margin: zoomed views
    // stop paying (in bytes and in the consumer's parse time) for the
    // rest of the hemisphere.
    let pad = (enc.width * 0.05).max(24.0);
    let page = Some((-pad, -pad, enc.width + pad, enc.width + pad));
    let extents = emit_scene(&mut s, scene, &ring_of, &line_of, &point_of, enc.smooth, &page);
    emit_labels(&mut s, scene, &extents, &point_of, enc.width, &page);
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
    // The flat plate fits the whole world to the page — nothing to cull.
    let extents = emit_scene(&mut s, scene, &ring_of, &line_of, &point_of, enc.smooth, &None);
    emit_labels(&mut s, scene, &extents, &point_of, enc.width, &None);
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

// ---------------------------------------------------- transition JSON

/// The transition backend for a web player: each semantic step becomes
/// one JSON object, ids in stable hex, points as [lon, lat] pairs. The
/// verbs pass through untranslated — a consumer that animates a Split
/// as a Morph would be lying about topology, so the format refuses to
/// blur them.
pub struct JsonTransitionEncoder;

fn pts_json(pts: &[UnitVec]) -> serde_json::Value {
    serde_json::Value::Array(
        pts.iter()
            .map(|p| {
                let (lat, lon) = lat_lon(p);
                serde_json::json!([lon, lat])
            })
            .collect(),
    )
}

impl TransitionEncoder for JsonTransitionEncoder {
    type Output = String;
    fn encode_transition(&self, script: &TransitionScript) -> Result<String, EncodeError> {
        let steps: Vec<serde_json::Value> = script
            .steps
            .iter()
            .map(|s| match s {
                TransitionStep::Morph { boundary, from_pts, to_pts } => serde_json::json!({
                    "kind": "morph",
                    "boundary": format!("{:016x}", boundary.0 .0),
                    "from": pts_json(from_pts),
                    "to": pts_json(to_pts),
                }),
                TransitionStep::FadeIn { region } => serde_json::json!({
                    "kind": "fade_in",
                    "region": format!("{:016x}", region.0 .0),
                }),
                TransitionStep::FadeOut { region } => serde_json::json!({
                    "kind": "fade_out",
                    "region": format!("{:016x}", region.0 .0),
                }),
                TransitionStep::SplitAlong { parent, seam, children } => serde_json::json!({
                    "kind": "split",
                    "parent": format!("{:016x}", parent.0 .0),
                    "seam": pts_json(seam),
                    "children": children
                        .iter()
                        .map(|c| format!("{:016x}", c.0 .0))
                        .collect::<Vec<_>>(),
                }),
                TransitionStep::MergeAcross { parents, child } => serde_json::json!({
                    "kind": "merge",
                    "parents": parents
                        .iter()
                        .map(|p| format!("{:016x}", p.0 .0))
                        .collect::<Vec<_>>(),
                    "child": format!("{:016x}", child.0 .0),
                }),
            })
            .collect();
        Ok(serde_json::json!({ "steps": steps }).to_string())
    }
}

#[cfg(test)]
mod tests;
