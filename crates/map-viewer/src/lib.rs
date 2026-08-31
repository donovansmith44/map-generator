//! The composition root and its two front ends (phase 7): `serve()`
//! answers HTTP for the workbench; the `map-cli` bin writes the SAME
//! routes' bytes to content-addressed files — serverless artifacts,
//! one shared `load()`. DOGFOOD LAW: request handlers consume ONLY the
//! public contract — `dyn MapProvider` for scenes, `SceneEncoder` for
//! bytes — zero privileged access into the timeline. `load()` is the
//! composition root: it may wire adapter → provider, and nothing else
//! may.

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;

use atlas_graph_types::covenant::{TimePoint, Year};
use atlas_graph_types::covenant::SourceId;

use map_adapters::{load_exports, merged_gazetteer};
use map_provider::SCRIPTURE_SOURCE;
use map_encoders::{GeoJsonEncoder, GpuSceneEncoder, JsonTransitionEncoder, SvgEncoder};
use map_types::style::*;
use map_types::{
    ChangeKind, Interval, LayerSet, Lod, MapAddressed, MapProvider, Monoid, RegionId,
    RenderQuery, RenderSubject, Snapshot, StyleId, TimeSelector,
};
use map_types::SceneEncoder as _;
use map_types::TransitionEncoder as _;

const PAGE: &str = include_str!("page.html");
/// Default port. 8080/8081/8000/5000 belong to the Bible Atlas
/// pipeline on this machine — the workbench stays clear of them.
/// Override with MAP_VIEWER_PORT.
const DEFAULT_PORT: u16 = 8090;

pub struct App {
    pub provider: Arc<dyn MapProvider + Send + Sync>,
    /// One token for the served world: a hash of the canon bytes and
    /// every style template, taken at load. Every response is a pure
    /// function of (this world, the URL), so the pair IS the ETag —
    /// browsers revalidate for free and a recompile rolls the token.
    world_etag: u64,
    styles: Vec<(&'static str, StyleId)>,
    /// Each visible style's ghost twin — the faded dress the rest of
    /// the world wears when one subject is the realized thing.
    ghosts: BTreeMap<StyleId, StyleId>,
    /// Scrub stops: the change-event years, preceded by one dawn stop
    /// showing the state before the first recorded change.
    stops: Vec<i32>,
    /// Per region key ("region:HEX"): its label and the stops that
    /// carry a mapping for it — probed through the contract at startup.
    presence: BTreeMap<String, (String, Vec<i32>)>,
    /// The declared frame this world stands under (read at the
    /// composition root; exposed so the bench can show biblical time,
    /// and so the CLI can pin artifact names to the world).
    pub anchor: Option<(String, i32)>,
    /// Per visible style: the (held, current) tints a comparison
    /// overlay wears — the age ramp's oldest and newest, so an overlay
    /// reads exactly like a range diff.
    overlay_tints: BTreeMap<StyleId, (StyleId, StyleId)>,
    /// The typed canon handle (Some when serving the canon): the
    /// composable API (entities, features, pieces, scaffold) needs
    /// more than the dyn contract exposes.
    canon: Option<Arc<map_provider::canon_provider::CanonProvider>>,
    /// The content-addressed geometry payload store (rendering spec
    /// §63): filled as /api/scene manifests are encoded, read by
    /// /api/resource. Immutable entries — equal id, equal bytes — so
    /// concurrent publishes can never disagree.
    resources: std::sync::Mutex<std::collections::BTreeMap<u64, Vec<u8>>>,
}

mod templates;

// ------------------------------------------------------------- styles

/// Derive a style's ghost: same bones, faded flesh. Patterns survive
/// (honesty renders even in the background), colors thin out.
fn ghosted(base: &Style) -> Style {
    let fade = |s: &Stroke| Stroke {
        color: Rgba(s.color.0, s.color.1, s.color.2, (f64::from(s.color.3) * 0.35) as u8),
        width: s.width * 0.7,
        pattern: s.pattern,
    };
    let fade_paint = |p: Paint| Paint {
        fill: Rgba(p.fill.0, p.fill.1, p.fill.2, (f64::from(p.fill.3) * 0.16) as u8),
    };
    use map_types::EdgeCharacter as E;
    let d = base.delta_emphasis();
    Style::new(
        BoundaryStrokes {
            line: fade(base.stroke_for(&E::Line)),
            frontier: fade(base.stroke_for(&E::Frontier { width_km: 0.0 })),
            disputed: fade(base.stroke_for(&E::Disputed { claimants: Vec::new() })),
            unknown: fade(base.stroke_for(&E::Unknown)),
            way: fade(base.stroke_for(&E::Way)),
        },
        fade_paint(base.region_paint()),
        fade_paint(base.water_paint()),
        AgeRamp {
            newest: fade_paint(base.topo_ramp().newest),
            oldest: fade_paint(base.topo_ramp().oldest),
        },
        None, // the ghost is a uniform disclosure, never colorful
        base.age_ramp(),
        base.labeling(),
        base.marker_style(),
        DeltaEmphasis { before: fade(&d.before), after: fade(&d.after), seam: fade(&d.seam) },
    )
    .expect("a faded honest style is still honest")
}

/// Recolor a style toward one paint — the dress a whole layer wears in
/// a comparison overlay. Patterns and widths survive (honesty), color
/// says WHICH layer.
fn tinted(base: &Style, paint: Paint) -> Style {
    let tint = |s: &Stroke| Stroke {
        color: Rgba(paint.fill.0, paint.fill.1, paint.fill.2, 235),
        width: s.width,
        pattern: s.pattern,
    };
    use map_types::EdgeCharacter as E;
    let d = base.delta_emphasis();
    let mut labeling = base.labeling();
    labeling.base.color = Rgba(paint.fill.0, paint.fill.1, paint.fill.2, 255);
    Style::new(
        BoundaryStrokes {
            line: tint(base.stroke_for(&E::Line)),
            frontier: tint(base.stroke_for(&E::Frontier { width_km: 0.0 })),
            disputed: tint(base.stroke_for(&E::Disputed { claimants: Vec::new() })),
            unknown: tint(base.stroke_for(&E::Unknown)),
            way: tint(base.stroke_for(&E::Way)),
        },
        paint,
        base.water_paint(),
        base.topo_ramp(),
        None, // a comparison layer is ONE tint, that is its meaning
        base.age_ramp(),
        labeling,
        base.marker_style(),
        DeltaEmphasis { before: tint(&d.before), after: tint(&d.after), seam: tint(&d.seam) },
    )
    .expect("a tinted honest style is still honest")
}

// ---------------------------------------------------------- wiring

fn tp(year: i32) -> Option<TimePoint> {
    Year::new(year).ok().map(TimePoint::year_only)
}

pub fn load() -> App {
    // The canon is the only truth store (phase 6): data in, maps out.
    // No canon file means the pipeline has not run — fail loud with
    // the fix, never fall back to a second source of truth.
    let canon_path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/canon/canon.json");
    if !canon_path.exists() {
        panic!(
            "no compiled canon at {} — run: map-compile refresh && map-compile build",
            canon_path.display()
        );
    }
    load_canon(&canon_path)
}

/// The canon-backed composition root: same styles, same App shape,
/// same routes — a different truth store underneath.
fn load_canon(canon_path: &std::path::Path) -> App {
    // PLUG-AND-CHUG: the style book IS templates/*.ron, loaded and
    // validated at startup (a dishonest file refuses to serve, by
    // name). Ghost and tint dresses derive from each loaded base.
    let tpl_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../templates");
    let loaded = templates::load_templates(&tpl_dir);
    let world_etag = {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        use std::hash::{Hash, Hasher};
        std::fs::read(canon_path).expect("canon bytes").hash(&mut h);
        if let Ok(dir) = std::fs::read_dir(&tpl_dir) {
            let mut paths: Vec<_> = dir.filter_map(Result::ok).map(|e| e.path()).collect();
            paths.sort();
            for p in paths {
                if let Ok(b) = std::fs::read(&p) {
                    b.hash(&mut h);
                }
            }
        }
        h.finish()
    };
    let styles: Vec<(&'static str, StyleId)> =
        loaded.iter().map(|(name, s)| (*name, s.id())).collect();
    let mut ghosts = BTreeMap::new();
    let mut style_table = BTreeMap::new();
    for (_, base) in &loaded {
        let ghost = ghosted(base);
        ghosts.insert(base.id(), ghost.id());
        style_table.insert(ghost.id(), ghost);
        style_table.insert(base.id(), *base);
    }
    let mut overlay_tints = BTreeMap::new();
    for (_, base) in &loaded {
        let ramp = base.age_ramp();
        let (held, current) = (tinted(base, ramp.oldest), tinted(base, ramp.newest));
        overlay_tints.insert(base.id(), (held.id(), current.id()));
        style_table.insert(held.id(), held);
        style_table.insert(current.id(), current);
    }
    let exp_dir =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/atlas-exports");
    let atlas = load_exports(
        &std::fs::read_to_string(exp_dir.join("gazetteer.json")).expect("vendored gazetteer"),
        &std::fs::read_to_string(exp_dir.join("chronology.json")).expect("vendored chronology"),
    )
    .expect("atlas exports parse");
    let anchor = atlas
        .creation_anchor()
        .map(|(y, _)| ("biblical (Ussher tradition)".to_string(), y));
    let t_boot = std::time::Instant::now();
    let canon_provider = Arc::new(
        map_provider::canon_provider::CanonProvider::from_canon_file(
            canon_path,
            style_table,
            Some(merged_gazetteer(&atlas)),
        )
        .expect("the compiled canon loads"),
    );
    let provider: Arc<dyn MapProvider + Send + Sync> = canon_provider.clone();
    eprintln!(
        "serving the CANON ({}) — loaded in {:.1}s",
        canon_path.display(),
        t_boot.elapsed().as_secs_f64()
    );

    let (lo, hi) = (tp(-4004).unwrap(), tp(1900).unwrap());
    let mut stops: Vec<i32> = provider.changes_between(lo, hi).iter().map(|e| e.at.year.get()).collect();
    stops.dedup();
    if let Some(&first) = stops.first() {
        stops.insert(0, first - 100);
    }
    let t_presence = std::time::Instant::now();
    let mut presence: BTreeMap<String, (String, Vec<i32>)> = BTreeMap::new();
    for &year in &stops {
        if let Some(at) = tp(year) {
            for s in provider.subjects(at) {
                let key = match &s.subject {
                    RenderSubject::Region(id) => format!("region:{:016x}", id.0 .0),
                    RenderSubject::Point(place) => format!("place:{}", place.0 .0),
                    _ => continue,
                };
                let entry = presence.entry(key).or_insert_with(|| (s.label.clone(), Vec::new()));
                entry.0 = s.label;
                entry.1.push(year);
            }
        }
    }
    eprintln!("presence probed in {:.1}s", t_presence.elapsed().as_secs_f64());
    App {
        provider,
        world_etag,
        styles,
        ghosts,
        stops,
        presence,
        anchor,
        overlay_tints,
        canon: Some(canon_provider),
        resources: std::sync::Mutex::new(std::collections::BTreeMap::new()),
    }
}

/// Keep only Scripture-derived elements: regions and boundaries whose
/// sources carry the scripture id, plus their labels. Semantic
/// selection on the scene — never on encoded bytes.
fn scripture_only(scene: &Snapshot) -> Snapshot {
    let scripture = SourceId::new(SCRIPTURE_SOURCE);
    // The physical stage — seas, lakes, relief — is never a claim to
    // filter: the whole world stays part of the map in bible mode.
    let stage = |srcs: &std::collections::BTreeSet<SourceId>| {
        srcs.iter().any(|s| s.0 == "witness:natural-earth" || s.0 == "natural-earth" || s.0 == "etopo1")
    };
    let regions: Vec<_> = scene
        .regions
        .iter()
        .filter(|r| r.sources.contains(&scripture) || stage(&r.sources))
        .cloned()
        .collect();
    let boundaries: Vec<_> =
        scene.boundaries.iter().filter(|b| b.sources.contains(&scripture)).cloned().collect();
    let kept_regions: std::collections::BTreeSet<_> = regions.iter().map(|r| r.region).collect();
    let kept_bounds: std::collections::BTreeSet<_> = boundaries.iter().map(|b| b.boundary).collect();
    // Markers select by their own sources — a journey's stations are
    // as scripture-grounded as the way through them.
    let markers: Vec<_> =
        scene.markers.iter().filter(|m| m.sources.contains(&scripture)).cloned().collect();
    let kept_places: std::collections::BTreeSet<_> =
        markers.iter().filter_map(|m| m.place.clone()).collect();
    let labels = scene
        .labels
        .iter()
        .filter(|l| match &l.subject {
            map_types::scene::LabelSubject::Region(r) => kept_regions.contains(r),
            map_types::scene::LabelSubject::Boundary(b) => kept_bounds.contains(b),
            map_types::scene::LabelSubject::Place(p) => kept_places.contains(p),
            map_types::scene::LabelSubject::Free => false,
        })
        .cloned()
        .collect();
    let attribution = regions
        .iter()
        .flat_map(|r| r.sources.iter().cloned())
        .chain(boundaries.iter().flat_map(|b| b.sources.iter().cloned()))
        .collect();
    Snapshot { regions, boundaries, markers, labels, attribution }
}

/// Drop from the backdrop whatever the realized scene already carries:
/// a region or boundary drawn in full must not also wear its ghost
/// twin, or every realized border is drawn twice.
fn without_realized(mut backdrop: Snapshot, realized: &Snapshot) -> Snapshot {
    let regions: std::collections::BTreeSet<_> = realized.regions.iter().map(|r| r.region).collect();
    let bounds: std::collections::BTreeSet<_> =
        realized.boundaries.iter().map(|b| b.boundary).collect();
    backdrop.regions.retain(|r| !regions.contains(&r.region));
    backdrop.boundaries.retain(|b| !bounds.contains(&b.boundary));
    backdrop.labels.retain(|l| match &l.subject {
        map_types::scene::LabelSubject::Region(r) => !regions.contains(r),
        map_types::scene::LabelSubject::Boundary(b) => !bounds.contains(b),
        map_types::scene::LabelSubject::Place(_) => true,
        map_types::scene::LabelSubject::Free => true,
    });
    backdrop
}

/// The mean position of a scene's content — where a globe should face
/// to look the subject in the eye.
fn scene_centroid(scene: &Snapshot) -> Option<(f64, f64)> {
    let (mut x, mut y, mut z) = (0.0f64, 0.0f64, 0.0f64);
    let mut feed = |p: &map_types::UnitVec| {
        x += p.x();
        y += p.y();
        z += p.z();
    };
    for r in &scene.regions {
        for ring in r.outer.iter().chain(&r.holes) {
            ring.points().iter().for_each(&mut feed);
        }
    }
    for b in &scene.boundaries {
        b.pts.iter().for_each(&mut feed);
    }
    for m in &scene.markers {
        feed(&m.at);
    }
    let n = (x * x + y * y + z * z).sqrt();
    if n < 1e-9 {
        return None;
    }
    let (x, y, z) = (x / n, y / n, z / n);
    Some((z.asin().to_degrees(), y.atan2(x).to_degrees()))
}

// ------------------------------------------------------------ queries

struct Params(BTreeMap<String, String>);

/// Minimal application/x-www-form-urlencoded decoding: %XX and '+'.
/// (Its absence was a live bug — the page encodes "region:HEX" as
/// "region%3AHEX" and every region query bounced as bad.)
fn url_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                match u8::from_str_radix(&s[i + 1..i + 3], 16) {
                    Ok(b) => {
                        out.push(b);
                        i += 3;
                    }
                    Err(_) => {
                        out.push(b'%');
                        i += 1;
                    }
                }
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

impl Params {
    fn parse(query: &str) -> Params {
        Params(
            query
                .split('&')
                .filter_map(|kv| kv.split_once('='))
                .map(|(k, v)| (url_decode(k), url_decode(v)))
                .collect(),
        )
    }
    fn get(&self, k: &str) -> Option<&str> {
        self.0.get(k).map(String::as_str)
    }
    fn year(&self, k: &str) -> Option<TimePoint> {
        self.get(k)?.parse::<i32>().ok().and_then(tp)
    }
}

/// Timestamps on the wire: "-1450" | "-1450-01" | "-1450-01-14" —
/// year (minding the missing zero), optional month, optional day; the
/// covenant TimePoint verbatim.
fn parse_timestamp(s: &str) -> Option<TimePoint> {
    let s = s.trim();
    let (neg, rest) = match s.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, s),
    };
    let parts: Vec<&str> = rest.split('-').collect();
    if parts.is_empty() || parts.len() > 3 || parts[0].is_empty() {
        return None;
    }
    let year: i32 = parts[0].parse().ok()?;
    let year = if neg { -year } else { year };
    let month: Option<u8> = match parts.get(1) {
        Some(p) => {
            let m: u8 = p.parse().ok()?;
            if m == 0 || m > 12 {
                return None;
            }
            Some(m)
        }
        None => None,
    };
    let day: Option<u8> = match parts.get(2) {
        Some(p) => {
            let d: u8 = p.parse().ok()?;
            if d == 0 || d > 31 {
                return None;
            }
            Some(d)
        }
        None => None,
    };
    TimePoint::new(Year::new(year).ok()?, month, day).ok()
}

fn parse_subject(s: &str) -> Option<RenderSubject> {
    if s == "world" {
        return Some(RenderSubject::World);
    }
    if let Some(id) = s.strip_prefix("place:") {
        return Some(RenderSubject::Point(map_types::AtlasPlaceRef(
            atlas_graph_types::covenant::PlaceId::new(id.to_string()),
        )));
    }
    let hex = s.strip_prefix("region:")?;
    let id = u64::from_str_radix(hex, 16).ok()?;
    Some(RenderSubject::Region(RegionId(atlas_graph_types::covenant::ContentHash(id))))
}

/// A style names itself three ways: by content id (hex), by name
/// ("parchment"), or by omission — the first visible style. Ids stay
/// exact for caching; names serve humans and the CLI.
fn parse_style(app: &App, s: Option<&str>) -> Option<StyleId> {
    let Some(key) = s else {
        return app.styles.first().map(|(_, sid)| *sid);
    };
    if let Some((_, sid)) = app.styles.iter().find(|(name, _)| *name == key) {
        return Some(*sid);
    }
    let id = u64::from_str_radix(key, 16).ok()?;
    let id = StyleId(atlas_graph_types::covenant::ContentHash(id));
    app.styles.iter().any(|(_, sid)| *sid == id).then_some(id)
}

/// Simplification tolerance that follows the camera: ~half an
/// on-screen pixel at the requested zoom (angular radius in degrees
/// over page width in px), converted to the radians Lod speaks.
/// Clamped so hemisphere views stay light and deep zooms never ask
/// for sub-source precision.
fn auto_lod(zoom: Option<f64>, width: f64) -> f64 {
    match zoom {
        Some(z) => (z / width).to_radians().clamp(1e-6, 0.01),
        None => 0.0015,
    }
}

fn build_query(
    app: &App,
    p: &Params,
    prefix: &str,
    subject_override: Option<&str>,
) -> Option<RenderQuery> {
    let subject_key =
        subject_override.unwrap_or_else(|| p.get(&format!("{prefix}subject")).unwrap_or("world"));
    let subject = parse_subject(subject_key)?;
    let at = p.year(&format!("{prefix}year"))?;
    let time = match p.year(&format!("{prefix}to")) {
        Some(to) if to != at => {
            let (a, b) = if at <= to { (at, to) } else { (to, at) };
            TimeSelector::Over(Interval::new(a, Some(b)).ok()?)
        }
        _ => TimeSelector::At(at),
    };
    let lod = match p.get("lod").filter(|v| *v != "auto").and_then(|v| v.parse().ok()) {
        Some(explicit) => Lod(explicit),
        None => {
            let zoom = p.get("zoom").and_then(|v| v.parse::<f64>().ok());
            let width = p
                .get("width")
                .and_then(|v| v.parse::<f64>().ok())
                .unwrap_or(1200.0)
                .clamp(320.0, 8000.0);
            Lod(auto_lod(zoom, width))
        }
    };
    let mut layers = if p.get("labels") == Some("0") {
        LayerSet::GEOMETRY
    } else {
        LayerSet::GEOMETRY.with(LayerSet::LABELS)
    };
    if p.get("topo") != Some("0") {
        layers = layers.with(LayerSet::TOPOGRAPHY); // the seas, on by default
    }
    if p.get("relief") == Some("1") {
        layers = layers.with(LayerSet::RELIEF); // hypsometric bands, opt-in
    }
    if p.get("journeys") != Some("0") {
        layers = layers.with(LayerSet::JOURNEYS); // itineraries, on by default
    }
    // THE VIEWPORT: when the caller pins a camera, the provider can
    // cull the world to it — one spherical cap, generous margin, both
    // charts (the camera law makes the ground span identical).
    let viewport = p.get("center").and_then(|v| {
        let (lat, lon) = v.split_once(',')?;
        let (lat, lon) = (lat.parse::<f64>().ok()?, lon.parse::<f64>().ok()?);
        let zoom = p.get("zoom").and_then(|z| z.parse::<f64>().ok())?;
        Some(map_types::Bbox {
            center: map_types::UnitVec::from_lat_lon_deg(lat.clamp(-89.9, 89.9), lon),
            radius: (zoom.clamp(0.05, 90.0) * 1.8).to_radians().min(std::f64::consts::PI),
        })
    });
    Some(RenderQuery { subject, time, viewport, lod, layers, style: parse_style(app, p.get("style"))? })
}

fn encode(
    p: &Params,
    scene: &Snapshot,
    face: Option<(f64, f64)>,
) -> Result<(String, &'static str), String> {
    match p.get("encoder").unwrap_or("svg") {
        "geojson" => GeoJsonEncoder
            .encode(scene)
            .map(|s| (s, "application/geo+json"))
            .map_err(|e| e.0),
        _ => {
            // Explicit navigation: center=lat,lon and zoom=deg
            // (angular radius). Absent, face the subject if we know
            // where it lives; the encoder auto-frames last.
            let center = p
                .get("center")
                .and_then(|v| {
                    let (lat, lon) = v.split_once(',')?;
                    Some((
                        lat.parse::<f64>().ok()?.clamp(-89.9, 89.9),
                        lon.parse::<f64>().ok()?,
                    ))
                })
                .or(face);
            let zoom = p.get("zoom").and_then(|v| v.parse::<f64>().ok());
            let projection = match p.get("projection") {
                // The flat plate takes the same camera; without one it
                // fits the whole world (zoom drives the crop).
                Some("flat") => map_encoders::Projection::Flat {
                    center: zoom.is_some().then_some(center).flatten(),
                    zoom,
                },
                _ => map_encoders::Projection::Globe { center, zoom },
            };
            // Scalable resolution: the output is vector, so width is a
            // free parameter — strokes, labels, and geodesic precision
            // all scale to it.
            let width = p
                .get("width")
                .and_then(|v| v.parse::<f64>().ok())
                .unwrap_or(1200.0)
                .clamp(320.0, 8000.0);
            let smooth = p.get("smooth") != Some("0");
            SvgEncoder { projection, width, smooth, ..SvgEncoder::default() }
                .encode(scene)
                .map(|s| (s, "image/svg+xml"))
                .map_err(|e| e.0)
        }
    }
}

/// The whole scene-composition pipeline, shared by every consumer of
/// a composed frame (the SVG render and the retained-scene manifest):
/// pieces, the multi-subject overlay monoid, bible-mode selection, and
/// the ghost backdrop. Returns the scene, the content face (globe
/// auto-centering), and — when exactly one query composed it — that
/// query, whose pid rides the response headers.
fn composed_scene(
    app: &App,
    p: &Params,
) -> Result<(Snapshot, Option<(f64, f64)>, Option<RenderQuery>), String> {
    // A PIECES render: a transparent, aligned layer of just the named
    // entities — composable by construction.
    if let Some(ids) = p.get("pieces") {
        let Some(canon) = app.canon.as_ref() else {
            return Err("pieces requires the canon (run map-compile build)".to_string());
        };
        if p.get("center").is_none() || p.get("zoom").is_none() {
            return Err("pieces requires center and zoom (alignment law)".to_string());
        }
        let set: std::collections::BTreeSet<map_canon::EntityId> = ids
            .split(',')
            .filter(|s| !s.is_empty())
            .map(|s| map_canon::EntityId(s.to_string()))
            .collect();
        let Some(q) = build_query(app, p, "", Some("world")) else {
            return Err("bad query".to_string());
        };
        return match canon.render_pieces(&q, &set) {
            Err(e) => Err(format!("{e:?}")),
            Ok(scene) => Ok((scene, None, Some(q))),
        };
    }
    // The subject may be a comma-list: a multi-region map is the
    // overlay of each region's own query (the monoid).
    let keys: Vec<&str> =
        p.get("subject").unwrap_or("world").split(',').filter(|s| !s.is_empty()).collect();
    if keys.is_empty() {
        return Err("no subject".to_string());
    }
    let mut queries = Vec::new();
    for k in &keys {
        let Some(q) = build_query(app, p, "", Some(k)) else {
            return Err("bad query".to_string());
        };
        queries.push(q);
    }
    let mut scenes = Vec::new();
    let mut first_err = None;
    for q in &queries {
        match app.provider.render(q) {
            Ok(s) => scenes.push(s),
            Err(e) => {
                if first_err.is_none() {
                    first_err = Some(e);
                }
            }
        }
    }
    if scenes.is_empty() {
        return Err(format!("{:?}", first_err.expect("some error")));
    }
    let mut subject_scene = Snapshot::empty();
    for s in scenes {
        subject_scene = subject_scene.combine(s);
    }
    // BIBLE MODE: only what is derived from Scripture is realized — a
    // semantic selection by the scripture source. It filters the
    // WORLD, never the user's own selection: a focused Roman Empire
    // stays realized (its sources disclose what grounds it) — only
    // the un-asked-for world reduces to what Scripture says.
    let bible = p.get("bible") == Some("1");
    let is_world_only = keys == ["world"];
    if bible && is_world_only {
        subject_scene = scripture_only(&subject_scene);
    }
    // The whole globe as context, the subject the realized thing:
    // overlay(world in ghost dress, subject in full) — the monoid
    // again, two contract calls and a combine. In bible mode the
    // backdrop is always on: the ghost is the disclosure that the
    // rest is NOT Scripture-derived.
    let q = queries[0].clone();
    let face = scene_centroid(&subject_scene);
    let want_context = bible || (p.get("context") != Some("0") && !is_world_only);
    let scene = if want_context {
        let ghost_style = app.ghosts.get(&q.style).copied();
        let backdrop_at = match q.time {
            TimeSelector::At(t) => t,
            TimeSelector::Over(i) => i.to.unwrap_or(i.from),
        };
        match ghost_style {
            None => subject_scene,
            Some(style) => {
                let ghost_q = RenderQuery {
                    subject: RenderSubject::World,
                    time: TimeSelector::At(backdrop_at),
                    viewport: q.viewport.clone(),
                    lod: q.lod,
                    layers: LayerSet::GEOMETRY,
                    style,
                };
                match app.provider.render(&ghost_q) {
                    Err(e) => return Err(format!("{e:?}")),
                    Ok(backdrop) => {
                        without_realized(backdrop, &subject_scene).combine(subject_scene)
                    }
                }
            }
        }
    } else {
        subject_scene
    };
    let single = if queries.len() == 1 { Some(q) } else { None };
    Ok((scene, face, single))
}

// ------------------------------------------------------------- routes

/// The public route: bytes out, so binary resource payloads (spec
/// §63) and text responses travel the same path.
pub fn route(
    app: &App,
    path: &str,
    query: &str,
) -> (u16, &'static str, Vec<u8>, Vec<(String, String)>) {
    // Content-addressed geometry payloads: the one binary route.
    if path == "/api/resource" {
        let p = Params::parse(query);
        let Some(id) = p.get("id").and_then(|h| u64::from_str_radix(h, 16).ok()) else {
            return (400, "text/plain", b"id required (hex)".to_vec(), Vec::new());
        };
        return match app.resources.lock().expect("resource store").get(&id) {
            Some(bytes) => (200, "application/octet-stream", bytes.clone(), Vec::new()),
            // Not resident is not an error state to hide: the client's
            // acquisition loop re-requests the scene manifest, which
            // re-publishes what this world can produce.
            None => (
                404,
                "text/plain",
                b"resource not resident - re-request /api/scene".to_vec(),
                Vec::new(),
            ),
        };
    }
    let (status, ctype, body, headers) = route_text(app, path, query);
    (status, ctype, body.into_bytes(), headers)
}

fn route_text(app: &App, path: &str, query: &str) -> (u16, &'static str, String, Vec<(String, String)>) {
    let p = Params::parse(query);
    let bad = |msg: &str| (400u16, "text/plain", msg.to_string(), Vec::new());

    match path {
        "/" => (200, "text/html", PAGE.to_string(), Vec::new()),

        "/api/meta" => {
            let styles: Vec<serde_json::Value> = app
                .styles
                .iter()
                .map(|(name, id)| serde_json::json!({ "name": name, "id": format!("{:016x}", id.0 .0) }))
                .collect();
            let anchor = app.anchor.as_ref().map(|(frame, year)| {
                serde_json::json!({ "frame": frame, "year": year })
            });
            let body = serde_json::json!({
                "stops": app.stops,
                "styles": styles,
                "encoders": ["svg", "geojson"],
                "projections": ["globe", "flat"],
                "anchor": anchor,
            });
            (200, "application/json", body.to_string(), Vec::new())
        }

        "/api/subjects" => {
            let Some(at) = p.year("year") else { return bad("year required (no year zero)") };
            let rows: Vec<serde_json::Value> = app
                .provider
                .subjects(at)
                .into_iter()
                .filter_map(|s| {
                    let key = match s.subject {
                        RenderSubject::World => "world".to_string(),
                        RenderSubject::Region(id) => format!("region:{:016x}", id.0 .0),
                        RenderSubject::Point(ref place) => format!("place:{}", place.0 .0),
                        _ => return None,
                    };
                    Some(serde_json::json!({ "key": key, "label": s.label }))
                })
                .collect();
            (200, "application/json", serde_json::Value::Array(rows).to_string(), Vec::new())
        }

        "/api/changes" => {
            let (Some(from), Some(to)) = (p.year("from"), p.year("to")) else {
                return bad("from and to required");
            };
            let rows: Vec<serde_json::Value> = app
                .provider
                .changes_between(from, to)
                .into_iter()
                .map(|e| {
                    let (kind, subject) = match &e.kind {
                        ChangeKind::Rise { region } => ("rise", format!("region:{:016x}", region.0 .0)),
                        ChangeKind::Fall { region } => ("fall", format!("region:{:016x}", region.0 .0)),
                        ChangeKind::Shift { boundary } => ("shift", format!("boundary:{:016x}", boundary.0 .0)),
                        ChangeKind::Split { parent, .. } => ("split", format!("region:{:016x}", parent.0 .0)),
                        ChangeKind::Merge { child, .. } => ("merge", format!("region:{:016x}", child.0 .0)),
                        ChangeKind::Rename { region } => ("rename", format!("region:{:016x}", region.0 .0)),
                        ChangeKind::Journey { boundary } => ("journey", format!("boundary:{:016x}", boundary.0 .0)),
                    };
                    serde_json::json!({
                        "year": e.at.year.get(),
                        "kind": kind,
                        "subject": subject,
                        "id": format!("{:016x}", e.id().0 .0),
                    })
                })
                .collect();
            (200, "application/json", serde_json::Value::Array(rows).to_string(), Vec::new())
        }

        "/api/transition" => {
            // The semantic animation between two instants, in the
            // terminal JSON encoding (phase 6): fades, splits, merges,
            // morphs — never a blurred crossfade of the lot.
            let (Some(from), Some(to)) = (p.year("from"), p.year("to")) else {
                return bad("from and to required");
            };
            let lod = p.get("lod").and_then(|l| l.parse().ok()).map(Lod).unwrap_or(Lod(6.0));
            match app.provider.transition(from, to, map_types::Bbox::whole_world(), lod) {
                Ok(script) => match JsonTransitionEncoder.encode_transition(&script) {
                    Ok(body) => (200, "application/json", body, Vec::new()),
                    Err(e) => bad(&format!("encode: {}", e.0)),
                },
                Err(e) => bad(&format!("transition: {e:?}")),
            }
        }

        // ---- the composable API (canon only) ----
        // The stage alone: land, water, relief — no claims, no ways.
        // Requires an explicit camera: the alignment law is that every
        // artifact with the same camera+width shares its projection.
        "/api/scaffold" => {
            if p.get("center").is_none() || p.get("zoom").is_none() {
                return bad("scaffold requires center and zoom (alignment law)");
            }
            let Some(year) = p.year("year") else { return bad("year required") };
            let mut q = match build_query(app, &p, "", Some("world")) {
                Some(q) => q,
                None => return bad("bad query"),
            };
            q.time = TimeSelector::At(year);
            q.layers = LayerSet::GEOMETRY.with(LayerSet::TOPOGRAPHY).with(LayerSet::RELIEF);
            let scene = match app.provider.render(&q) {
                Ok(s) => s,
                Err(e) => return bad(&format!("{e:?}")),
            };
            // The GEOMETRY bit satisfies the provider; the scaffold
            // then strips every claim, keeping only the physical base.
            let scene = Snapshot {
                regions: scene
                    .regions
                    .into_iter()
                    .filter(|r| {
                        r.sources
                            .iter()
                            .any(|s| s.0 == "witness:natural-earth" || s.0 == "natural-earth")
                    })
                    .collect(),
                boundaries: Vec::new(),
                markers: Vec::new(),
                labels: Vec::new(),
                attribution: scene.attribution,
            };
            match encode(&p, &scene, None) {
                Err(e) => bad(&e),
                Ok((body, ctype)) => (200, ctype, body, Vec::new()),
            }
        }

        // The entity listing: what pieces exist at a timestamp.
        "/api/entities" => {
            let Some(canon) = app.canon.as_ref() else {
                return bad("entities requires the canon (run map-compile build)");
            };
            let Some(at) = p.get("at").and_then(parse_timestamp) else {
                return bad("at required: year[-month[-day]]");
            };
            let rows: Vec<serde_json::Value> = canon
                .entities_at(&at)
                .into_iter()
                .map(|(ent, name, kind, witness)| {
                    serde_json::json!({
                        "entity": ent.0,
                        "name": name,
                        "kind": kind,
                        "witness": witness,
                    })
                })
                .collect();
            (200, "application/json", serde_json::Value::Array(rows).to_string(), Vec::new())
        }

        // Raw composable data: the named entities as GeoJSON.
        "/api/features" => {
            let Some(canon) = app.canon.as_ref() else {
                return bad("features requires the canon (run map-compile build)");
            };
            let Some(at) = p.get("at").and_then(parse_timestamp) else {
                return bad("at required: year[-month[-day]]");
            };
            let Some(ids) = p.get("ids") else { return bad("ids required") };
            let set: std::collections::BTreeSet<map_canon::EntityId> = ids
                .split(',')
                .filter(|s| !s.is_empty())
                .map(|s| map_canon::EntityId(s.to_string()))
                .collect();
            let q = RenderQuery {
                subject: RenderSubject::World,
                time: TimeSelector::At(at),
                viewport: None,
                lod: Lod(0.0),
                layers: LayerSet::GEOMETRY
                    .with(LayerSet::LABELS)
                    .with(LayerSet::TOPOGRAPHY)
                    .with(LayerSet::RELIEF)
                    .with(LayerSet::JOURNEYS),
                style: app.styles.first().map(|(_, sid)| *sid).expect("a style"),
            };
            match canon.render_pieces(&q, &set) {
                Err(e) => bad(&format!("{e:?}")),
                Ok(scene) => match GeoJsonEncoder.encode(&scene) {
                    Err(e) => bad(&e.0),
                    Ok(body) => (200, "application/geo+json", body, Vec::new()),
                },
            }
        }

        "/api/render" => match composed_scene(app, &p) {
            Err(e) => bad(&e),
            Ok((scene, face, single)) => match encode(&p, &scene, face) {
                Err(e) => bad(&e),
                Ok((body, ctype)) => {
                    let attribution: Vec<String> =
                        scene.attribution.iter().map(|s| s.0.clone()).collect();
                    let mut headers = vec![
                        ("X-Attribution".to_string(), attribution.join(", ")),
                        ("X-Scene-Pid".to_string(), format!("{:016x}", scene.map_pid().hash.0)),
                    ];
                    if let Some(q) = single {
                        headers.push((
                            "X-Query-Pid".to_string(),
                            format!("{:016x}", q.map_pid().hash.0),
                        ));
                    }
                    (200, ctype, body, headers)
                }
            },
        },

        // The RETAINED-SCENE protocol (rendering spec, stages 2–3):
        // the same composed scene the SVG path draws, answered as a
        // semantic manifest instead of a picture. Geometry travels
        // separately, content-addressed, via /api/resource — a camera
        // change downstream never re-requests either.
        "/api/scene" => match composed_scene(app, &p) {
            Err(e) => bad(&e),
            Ok((scene, _face, _single)) => {
                match GpuSceneEncoder.encode(&scene) {
                    Err(e) => bad(&e.0),
                    Ok(es) => {
                        // Publish payloads into the content-addressed
                        // store. Immutable by identity: an id already
                        // present IS the same bytes. Eviction is a
                        // declared later stage (spec stage 9) — the
                        // store only grows for now.
                        let mut store = app.resources.lock().expect("resource store");
                        for r in &es.resources {
                            store
                                .entry(r.descriptor.id.0 .0)
                                .or_insert_with(|| r.payload.clone());
                        }
                        (200, "application/json", es.manifest_json(), Vec::new())
                    }
                }
            }
        },

        // The focused subject's own timeline: which scrub stops carry a
        // mapping for it. Derived at startup through the contract's
        // subjects() probe — a first-class per-subject timeline query
        // is noted for the C5 freeze.
        "/api/region_times" => {
            let Some(key) = p.get("subject") else { return bad("subject required") };
            match app.presence.get(key) {
                None => (200, "application/json", "{\"label\":null,\"stops\":[]}".to_string(), Vec::new()),
                Some((label, stops)) => {
                    let body = serde_json::json!({ "label": label, "stops": stops });
                    (200, "application/json", body.to_string(), Vec::new())
                }
            }
        }

        // The overlay scratchpad: TWO scenes composed at the SEMANTIC
        // level (the monoid), then encoded once — never on bytes.
        // Either side may itself be a multi-region list.
        "/api/overlay" => {
            // The held side wears the ramp's OLDEST tint, the current
            // side its NEWEST — an overlay reads exactly like a range
            // diff, never two indistinguishable coats of one style.
            let side = |prefix: &str| -> Result<Snapshot, String> {
                let keys = p.get(&format!("{prefix}subject")).unwrap_or("world").to_string();
                let mut combined = Snapshot::empty();
                let mut any = false;
                for k in keys.split(',').filter(|s| !s.is_empty()) {
                    let mut q = build_query(app, &p, prefix, Some(k)).ok_or("bad overlay query")?;
                    if let Some((held, current)) = app.overlay_tints.get(&q.style) {
                        q.style = if prefix == "a_" { *held } else { *current };
                    }
                    match app.provider.render(&q) {
                        Ok(s) => {
                            combined = combined.combine(s);
                            any = true;
                        }
                        Err(e) => return Err(format!("{e:?}")),
                    }
                }
                if any { Ok(combined) } else { Err("no subject".to_string()) }
            };
            let scene = match (side("a_"), side("b_")) {
                (Ok(a), Ok(b)) => a.combine(b),
                (Err(e), _) | (_, Err(e)) => return bad(&e),
            };
            match encode(&p, &scene, None) {
                Err(e) => bad(&e),
                Ok((body, ctype)) => {
                    let attribution: Vec<String> =
                        scene.attribution.iter().map(|s| s.0.clone()).collect();
                    (200, ctype, body, vec![("X-Attribution".to_string(), attribution.join(", "))])
                }
            }
        }

        _ => (404, "text/plain", "not found".to_string(), Vec::new()),
    }
}

// -------------------------------------------------------------- serve

fn handle(app: &App, mut stream: TcpStream) {
    let mut buf = [0u8; 8192];
    let mut read = 0usize;
    loop {
        match stream.read(&mut buf[read..]) {
            Ok(0) => break,
            Ok(n) => {
                read += n;
                if buf[..read].windows(4).any(|w| w == b"\r\n\r\n") || read == buf.len() {
                    break;
                }
            }
            Err(_) => return,
        }
    }
    let request = String::from_utf8_lossy(&buf[..read]);
    let Some(line) = request.lines().next() else { return };
    let mut parts = line.split(' ');
    let (Some(method), Some(target)) = (parts.next(), parts.next()) else { return };
    if method != "GET" {
        let _ = stream.write_all(b"HTTP/1.1 405 Method Not Allowed\r\nConnection: close\r\n\r\n");
        return;
    }
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    // every response is a pure function of (world, URL): the pair is
    // the ETag, so a browser revisiting a scrub year revalidates in
    // one cheap round-trip instead of re-downloading megabytes
    let etag = {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        app.world_etag.hash(&mut h);
        target.hash(&mut h);
        format!("\"{:016x}\"", h.finish())
    };
    let revalidated = request
        .lines()
        .find(|l| l.to_ascii_lowercase().starts_with("if-none-match:"))
        .map(|l| l.split_once(':').map(|(_, v)| v.trim() == etag).unwrap_or(false))
        .unwrap_or(false);
    if revalidated {
        let _ = stream.write_all(
            format!(
                "HTTP/1.1 304 Not Modified\r\nETag: {etag}\r\nCache-Control: no-cache\r\nConnection: close\r\n\r\n"
            )
            .as_bytes(),
        );
        return;
    }
    let (status, ctype, body, extra) = route(app, path, query);
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        _ => "Not Found",
    };
    let cache = if status == 200 {
        format!("ETag: {etag}\r\nCache-Control: no-cache\r\n")
    } else {
        "Cache-Control: no-store\r\n".to_string()
    };
    let mut head = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {ctype}; charset=utf-8\r\nContent-Length: {}\r\n{cache}Connection: close\r\n",
        body.len()
    );
    for (k, v) in extra {
        head.push_str(&format!("{k}: {v}\r\n"));
    }
    head.push_str("\r\n");
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(&body);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The regression that shipped: URLSearchParams encodes ':' as
    /// %3A, and an undecoded server rejected every region query.
    #[test]
    fn encoded_region_keys_decode_and_parse() {
        let p = Params::parse("subject=region%3A00ff00ff00ff00ff&year=-500");
        assert_eq!(p.get("subject"), Some("region:00ff00ff00ff00ff"));
        assert!(matches!(
            parse_subject(p.get("subject").unwrap()),
            Some(RenderSubject::Region(_))
        ));
        assert_eq!(url_decode("a+b%20c"), "a b c");
        assert_eq!(url_decode("plain"), "plain");
        assert_eq!(url_decode("bad%zz"), "bad%zz");
    }

    /// Detail must follow the camera: about half an on-screen pixel of
    /// tolerance at any zoom, so zooming in never flattens a coast and
    /// zooming out never pays for invisible vertices.
    #[test]
    fn auto_lod_tracks_the_view() {
        // zoom is angular radius in DEGREES; width is the page in px;
        // Lod is RADIANS — the formula must convert.
        let deep = auto_lod(Some(0.5), 2400.0);
        let mid = auto_lod(Some(22.0), 2400.0);
        assert!(deep < mid, "deeper zoom must mean finer tolerance");
        assert!((deep - (0.5f64 / 2400.0).to_radians()).abs() < 1e-12);
        // hemisphere views clamp so whole-globe bytes stay sane
        assert!(auto_lod(Some(90.0), 2400.0) <= 0.01);
        // no zoom (flat / auto-frame): today's default, in radians
        assert!((auto_lod(None, 2400.0) - 0.0015).abs() < 1e-12);
        // never finer than the floor, whatever the numbers say
        assert!(auto_lod(Some(0.001), 8000.0) >= 1e-6);
    }

    /// Timestamps on the wire: "-1450" | "-1450-01" | "-1450-01-14" —
    /// year, optional month, optional day, covenant granularity.
    #[test]
    fn timestamps_parse_at_covenant_granularity() {
        use atlas_graph_types::covenant::{TimePoint, Year};
        let ts = |y: i32, m: Option<u8>, d: Option<u8>| {
            TimePoint::new(Year::new(y).unwrap(), m, d).unwrap()
        };
        assert_eq!(parse_timestamp("-1450"), Some(ts(-1450, None, None)));
        assert_eq!(parse_timestamp("-1450-01"), Some(ts(-1450, Some(1), None)));
        assert_eq!(parse_timestamp("-1450-01-14"), Some(ts(-1450, Some(1), Some(14))));
        assert_eq!(parse_timestamp("33-04-03"), Some(ts(33, Some(4), Some(3))));
        assert_eq!(parse_timestamp("57"), Some(ts(57, None, None)));
        assert_eq!(parse_timestamp("0"), None, "there is no year zero");
        assert_eq!(parse_timestamp("-1450-13"), None, "no thirteenth month");
        assert_eq!(parse_timestamp("nonsense"), None);
    }

    /// Bible mode filters CLAIMS, not the stage: the seas, lakes, and
    /// relief (natural-earth witness) stay realized — the owner's law
    /// is that the whole world is part of the map. Scholarship claims
    /// (basemap witness) still ghost.
    #[test]
    fn bible_mode_keeps_the_stage() {
        use atlas_graph_types::covenant::ContentHash;
        use map_types::{RegionId, Ring, StyledRegion, UnitVec};
        let uv = |lat: f64, lon: f64| UnitVec::from_lat_lon_deg(lat, lon);
        let region = |n: u64, src: &str| StyledRegion {
            region: RegionId(ContentHash(n)),
            entity: None,
            outer: vec![Ring::new(vec![uv(0.0, 0.0), uv(0.0, 10.0), uv(8.0, 5.0)]).unwrap()],
            holes: vec![],
            paint: Paint { fill: Rgba(1, 2, 3, 200) },
            sources: [SourceId::new(src)].into(),
        };
        let mut scene = Snapshot::empty();
        scene.regions.push(region(1, "witness:natural-earth")); // the sea
        scene.regions.push(region(2, "natural-earth")); // legacy tag, same stage
        scene.regions.push(region(3, "witness:basemap")); // a scholarship claim
        let kept = scripture_only(&scene);
        let ids: Vec<u64> = kept.regions.iter().map(|r| r.region.0 .0).collect();
        assert_eq!(ids, vec![1, 2], "the stage stays; the claim ghosts");
    }

    /// Bible mode keeps what Scripture grounds — including a journey's
    /// station markers. (They used to be dropped wholesale: markers
    /// carried no sources to select by.)
    #[test]
    fn scripture_selection_keeps_grounded_markers() {
        use map_types::scene::StyledMarker;
        use map_types::style::MarkerStyle;
        use map_types::UnitVec;
        let mut scene = Snapshot::empty();
        let mk = |src: Option<&str>| StyledMarker {
            at: UnitVec::from_lat_lon_deg(32.0, 35.0),
            style: MarkerStyle { color: map_types::style::Rgba(0, 0, 0, 255), size: 3.0 },
            sources: src.map(SourceId::new).into_iter().collect(),
            place: None,
        };
        scene.markers.push(mk(Some(SCRIPTURE_SOURCE)));
        scene.markers.push(mk(Some("natural-earth")));
        scene.markers.push(mk(None));
        let kept = scripture_only(&scene);
        assert_eq!(kept.markers.len(), 1, "exactly the scripture-grounded marker survives");
        assert!(kept.markers[0].sources.contains(&SourceId::new(SCRIPTURE_SOURCE)));
    }

    /// The ghost backdrop must not re-draw what the subject scene
    /// already realizes — otherwise every scripture region wears a
    /// ghost twin and every border is drawn twice.
    #[test]
    fn backdrop_yields_to_the_realized_scene() {
        use atlas_graph_types::covenant::ContentHash;
        use map_types::style::{Paint, Rgba, Stroke, StrokePattern};
        use map_types::{BoundaryId, Monoid, RegionId, Ring, StyledBoundary, StyledRegion, UnitVec};

        let uv = |lat: f64, lon: f64| UnitVec::from_lat_lon_deg(lat, lon);
        let region = |n: u64| StyledRegion {
            region: RegionId(ContentHash(n)),
            entity: None,
            outer: vec![Ring::new(vec![uv(0.0, 0.0), uv(0.0, 10.0), uv(8.0, 5.0)]).unwrap()],
            holes: vec![],
            paint: Paint { fill: Rgba(210, 190, 150, 255) },
            sources: Default::default(),
        };
        let boundary = |n: u64| StyledBoundary {
            boundary: BoundaryId(ContentHash(n)),
            pts: vec![uv(0.0, 0.0), uv(0.0, 10.0)],
            stroke: Stroke {
                color: Rgba(90, 60, 40, 255),
                width: 1.5,
                pattern: StrokePattern::Dashed,
            },
            sources: Default::default(),
        };

        let mut backdrop = Snapshot::empty();
        backdrop.regions.extend([region(1), region(2)]);
        backdrop.boundaries.extend([boundary(3), boundary(4)]);
        let mut realized = Snapshot::empty();
        realized.regions.push(region(2));
        realized.boundaries.push(boundary(4));

        let scene = without_realized(backdrop, &realized).combine(realized);
        let region_hits =
            scene.regions.iter().filter(|r| r.region == RegionId(ContentHash(2))).count();
        let boundary_hits =
            scene.boundaries.iter().filter(|b| b.boundary == BoundaryId(ContentHash(4))).count();
        assert_eq!(region_hits, 1, "realized region must appear exactly once");
        assert_eq!(boundary_hits, 1, "realized boundary must appear exactly once");
        assert!(scene.regions.iter().any(|r| r.region == RegionId(ContentHash(1))));
        assert!(scene.boundaries.iter().any(|b| b.boundary == BoundaryId(ContentHash(3))));
    }
}

pub fn serve() {
    let port: u16 = std::env::var("MAP_VIEWER_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_PORT);
    eprintln!("loading historical-basemaps…");
    let app = Arc::new(load());
    eprintln!("{} scrub stops, {} styles", app.stops.len(), app.styles.len());
    let listener =
        TcpListener::bind(("127.0.0.1", port)).unwrap_or_else(|e| panic!("port {port}: {e}"));
    eprintln!("workbench on http://127.0.0.1:{port}/");
    for stream in listener.incoming().flatten() {
        let app = Arc::clone(&app);
        std::thread::spawn(move || handle(&app, stream));
    }
}
