//! The workbench. DOGFOOD LAW: request handlers consume ONLY the
//! public contract — `dyn MapProvider` for scenes, `SceneEncoder` for
//! bytes — zero privileged access into the timeline. main() is the
//! composition root: it may wire adapter → provider, and nothing else
//! may.

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;

use atlas_graph_types::chrono::{TimePoint, Year};
use atlas_graph_types::edge::{Ground, Justification};
use atlas_graph_types::id::SourceId;
use atlas_graph_types::text::{BibleLocus, LocusRange, VerseRef};

use map_adapters::{
    epoch_year_from_label, ingest, merge_timelines, scripture_timeline, stand_in_gazetteer,
    EpochSource, IngestConfig,
};
use map_provider::SCRIPTURE_SOURCE;
use map_encoders::{GeoJsonEncoder, SvgEncoder};
use map_provider::TimelineProvider;
use map_types::style::*;
use map_types::{
    Anchor, ChangeKind, Interval, LayerSet, Lod, MapAddressed, MapProvider, Monoid, RegionId,
    RenderQuery, RenderSubject, Snapshot, StyleId, TimeSelector,
};
use map_types::SceneEncoder as _;

const PAGE: &str = include_str!("page.html");
/// Default port. 8080/8081/8000/5000 belong to the Bible Atlas
/// pipeline on this machine — the workbench stays clear of them.
/// Override with MAP_VIEWER_PORT.
const DEFAULT_PORT: u16 = 8090;

struct App {
    provider: Arc<dyn MapProvider + Send + Sync>,
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
    /// composition root; exposed so the bench can show biblical time).
    anchor: Option<(String, i32)>,
}

// ------------------------------------------------------------- styles

fn parchment() -> Style {
    let s = |c, w, p| Stroke { color: c, width: w, pattern: p };
    Style::new(
        BoundaryStrokes {
            line: s(Rgba(74, 52, 34, 255), 1.2, StrokePattern::Solid),
            frontier: s(Rgba(150, 110, 60, 255), 1.2, StrokePattern::Zonal),
            disputed: s(Rgba(140, 50, 40, 255), 1.2, StrokePattern::Hatched),
            unknown: s(Rgba(120, 116, 105, 255), 1.1, StrokePattern::Dashed),
        },
        Paint { fill: Rgba(221, 204, 161, 235) },
        AgeRamp {
            newest: Paint { fill: Rgba(174, 60, 40, 220) },
            oldest: Paint { fill: Rgba(174, 60, 40, 40) },
        },
        LabelStyle { color: Rgba(56, 40, 26, 255), size: 11.0 },
        MarkerStyle { color: Rgba(56, 40, 26, 255), size: 3.5 },
        DeltaEmphasis {
            before: s(Rgba(120, 116, 105, 255), 1.6, StrokePattern::Dashed),
            after: s(Rgba(174, 60, 40, 255), 1.8, StrokePattern::Solid),
            seam: s(Rgba(220, 90, 40, 255), 2.2, StrokePattern::Solid),
        },
    )
    .expect("parchment style is honest")
}

fn slate() -> Style {
    let s = |c, w, p| Stroke { color: c, width: w, pattern: p };
    Style::new(
        BoundaryStrokes {
            line: s(Rgba(214, 211, 200, 255), 1.1, StrokePattern::Solid),
            frontier: s(Rgba(150, 140, 110, 255), 1.1, StrokePattern::Zonal),
            disputed: s(Rgba(196, 90, 70, 255), 1.1, StrokePattern::Hatched),
            unknown: s(Rgba(120, 125, 135, 255), 1.0, StrokePattern::Dashed),
        },
        Paint { fill: Rgba(58, 66, 80, 235) },
        AgeRamp {
            newest: Paint { fill: Rgba(196, 90, 70, 220) },
            oldest: Paint { fill: Rgba(196, 90, 70, 40) },
        },
        LabelStyle { color: Rgba(214, 211, 200, 255), size: 11.0 },
        MarkerStyle { color: Rgba(214, 211, 200, 255), size: 3.5 },
        DeltaEmphasis {
            before: s(Rgba(120, 125, 135, 255), 1.6, StrokePattern::Dashed),
            after: s(Rgba(196, 90, 70, 255), 1.8, StrokePattern::Solid),
            seam: s(Rgba(240, 140, 80, 255), 2.2, StrokePattern::Solid),
        },
    )
    .expect("slate style is honest")
}

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
        },
        fade_paint(base.region_paint()),
        base.age_ramp(),
        base.label_style(),
        base.marker_style(),
        DeltaEmphasis { before: fade(&d.before), after: fade(&d.after), seam: fade(&d.seam) },
    )
    .expect("a faded honest style is still honest")
}

// ---------------------------------------------------------- wiring

fn tp(year: i32) -> Option<TimePoint> {
    Year::new(year).ok().map(TimePoint::year_only)
}

fn load() -> App {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/historical-basemaps");
    let mut epochs = Vec::new();
    for entry in std::fs::read_dir(&dir).expect("vendored data present") {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("geojson") {
            continue;
        }
        let label = path.file_stem().unwrap().to_string_lossy().to_string();
        let year = epoch_year_from_label(&label).expect("epoch label");
        epochs.push(EpochSource { year, label, text: std::fs::read_to_string(&path).unwrap() });
    }
    let gen11 = BibleLocus::whole(VerseRef { book: 1, chapter: 1, verse: 1 });
    let config = IngestConfig {
        source: SourceId::new("historical-basemaps"),
        anchor: Some(Anchor {
            frame: "biblical (Ussher tradition)".to_string(),
            at: tp(-4004).unwrap(),
            justification: Justification {
                text: Some("In the beginning God created the heaven and the earth.".to_string()),
                grounds: [Ground::Scripture(LocusRange::new(gen11.clone(), gen11).unwrap())].into(),
            },
            provenance: "owner-config:ussher-tradition (pending atlas C2 export)".to_string(),
        }),
    };
    let out = ingest(&config, &epochs).expect("real source ingests");
    // The first Bible-driven borders join the imported world, and the
    // merged whole must be lawful — fail loud at the door, not in a
    // render (validated against the stand-in gazetteer until C3 lands).
    let timeline = merge_timelines(out.timeline, scripture_timeline())
        .expect("scripture surveys merge cleanly");
    let violations = map_types::validate_all(
        &timeline,
        &map_types::ChronologyExport {
            atlas_root: atlas_graph_types::id::ContentHash(0),
            placements: BTreeMap::new(),
        },
        &stand_in_gazetteer(),
    );
    assert!(violations.is_empty(), "merged world unlawful: {:?}", violations.first());
    let anchor = timeline.anchor.as_ref().map(|a| (a.frame.clone(), a.at.year.get()));
    let out = map_adapters::Ingest { timeline, exemptions: out.exemptions };
    let (p_style, s_style) = (parchment(), slate());
    let (p_ghost, s_ghost) = (ghosted(&p_style), ghosted(&s_style));
    let styles = vec![("parchment", p_style.id()), ("slate", s_style.id())];
    let ghosts = BTreeMap::from([(p_style.id(), p_ghost.id()), (s_style.id(), s_ghost.id())]);
    let provider: Arc<dyn MapProvider + Send + Sync> = Arc::new(TimelineProvider {
        timeline: out.timeline,
        styles: BTreeMap::from([
            (p_style.id(), p_style),
            (s_style.id(), s_style),
            (p_ghost.id(), p_ghost),
            (s_ghost.id(), s_ghost),
        ]),
        gazetteer: None,
    });

    // Scrub stops through the contract: probe the widest sensible span.
    let (lo, hi) = (tp(-4004).unwrap(), tp(1900).unwrap());
    let mut stops: Vec<i32> = provider
        .changes_between(lo, hi)
        .iter()
        .map(|e| e.at.year.get())
        .collect();
    stops.dedup();
    if let Some(&first) = stops.first() {
        stops.insert(0, first - 100); // dawn: the state before the first change
    }
    let mut presence: BTreeMap<String, (String, Vec<i32>)> = BTreeMap::new();
    for &year in &stops {
        if let Some(at) = tp(year) {
            for s in provider.subjects(at) {
                if let RenderSubject::Region(id) = s.subject {
                    let key = format!("region:{:016x}", id.0 .0);
                    let entry = presence.entry(key).or_insert_with(|| (s.label.clone(), Vec::new()));
                    entry.0 = s.label;
                    entry.1.push(year);
                }
            }
        }
    }
    App { provider, styles, ghosts, stops, presence, anchor }
}

/// Keep only Scripture-derived elements: regions and boundaries whose
/// sources carry the scripture id, plus their labels. Semantic
/// selection on the scene — never on encoded bytes.
fn scripture_only(scene: &Snapshot) -> Snapshot {
    let scripture = SourceId::new(SCRIPTURE_SOURCE);
    let regions: Vec<_> =
        scene.regions.iter().filter(|r| r.sources.contains(&scripture)).cloned().collect();
    let boundaries: Vec<_> =
        scene.boundaries.iter().filter(|b| b.sources.contains(&scripture)).cloned().collect();
    let kept_regions: std::collections::BTreeSet<_> = regions.iter().map(|r| r.region).collect();
    let kept_bounds: std::collections::BTreeSet<_> = boundaries.iter().map(|b| b.boundary).collect();
    let labels = scene
        .labels
        .iter()
        .filter(|l| match &l.subject {
            map_types::scene::LabelSubject::Region(r) => kept_regions.contains(r),
            map_types::scene::LabelSubject::Boundary(b) => kept_bounds.contains(b),
            map_types::scene::LabelSubject::Free => false,
        })
        .cloned()
        .collect();
    let attribution = regions
        .iter()
        .flat_map(|r| r.sources.iter().cloned())
        .chain(boundaries.iter().flat_map(|b| b.sources.iter().cloned()))
        .collect();
    Snapshot { regions, boundaries, markers: Vec::new(), labels, attribution }
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

fn parse_subject(s: &str) -> Option<RenderSubject> {
    if s == "world" {
        return Some(RenderSubject::World);
    }
    let hex = s.strip_prefix("region:")?;
    let id = u64::from_str_radix(hex, 16).ok()?;
    Some(RenderSubject::Region(RegionId(atlas_graph_types::id::ContentHash(id))))
}

fn parse_style(app: &App, s: Option<&str>) -> Option<StyleId> {
    let hex = s?;
    let id = u64::from_str_radix(hex, 16).ok()?;
    let id = StyleId(atlas_graph_types::id::ContentHash(id));
    app.styles.iter().any(|(_, sid)| *sid == id).then_some(id)
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
    let lod = Lod(p.get("lod").and_then(|v| v.parse().ok()).unwrap_or(0.0015));
    let layers = if p.get("labels") == Some("0") {
        LayerSet::GEOMETRY
    } else {
        LayerSet::GEOMETRY.with(LayerSet::LABELS)
    };
    Some(RenderQuery { subject, time, viewport: None, lod, layers, style: parse_style(app, p.get("style"))? })
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
            let projection = match p.get("projection") {
                Some("flat") => map_encoders::Projection::Flat,
                _ => {
                    // Explicit navigation: center=lat,lon and zoom=deg
                    // (angular radius). Absent, face the subject if we
                    // know where it lives; the encoder auto-frames last.
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
                    map_encoders::Projection::Globe { center, zoom }
                }
            };
            SvgEncoder { projection, ..SvgEncoder::default() }
                .encode(scene)
                .map(|s| (s, "image/svg+xml"))
                .map_err(|e| e.0)
        }
    }
}

// ------------------------------------------------------------- routes

fn route(app: &App, path: &str, query: &str) -> (u16, &'static str, String, Vec<(String, String)>) {
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

        "/api/render" => {
            // The subject may be a comma-list: a multi-region map is
            // the overlay of each region's own query (the monoid).
            let keys: Vec<&str> = p
                .get("subject")
                .unwrap_or("world")
                .split(',')
                .filter(|s| !s.is_empty())
                .collect();
            if keys.is_empty() {
                return bad("no subject");
            }
            let mut queries = Vec::new();
            for k in &keys {
                let Some(q) = build_query(app, &p, "", Some(k)) else { return bad("bad query") };
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
                return bad(&format!("{:?}", first_err.expect("some error")));
            }
            let mut subject_scene = Snapshot::empty();
            for s in scenes {
                subject_scene = subject_scene.combine(s);
            }
            // BIBLE MODE: only what is derived from Scripture is
            // realized — a semantic selection by the scripture source.
            let bible = p.get("bible") == Some("1");
            if bible {
                subject_scene = scripture_only(&subject_scene);
            }
            // The whole globe as context, the subject the realized
            // thing: overlay(world in ghost dress, subject in full) —
            // the monoid again, two contract calls and a combine. In
            // bible mode the backdrop is always on: the ghost is the
            // disclosure that the rest is NOT Scripture-derived.
            let q = queries[0].clone();
            let face = scene_centroid(&subject_scene);
            let is_world_only = keys == ["world"];
            let want_context =
                bible || (p.get("context") != Some("0") && !is_world_only);
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
                            viewport: None,
                            lod: q.lod,
                            layers: LayerSet::GEOMETRY,
                            style,
                        };
                        match app.provider.render(&ghost_q) {
                            Err(e) => return bad(&format!("{e:?}")),
                            Ok(backdrop) => backdrop.combine(subject_scene),
                        }
                    }
                }
            } else {
                subject_scene
            };
            match encode(&p, &scene, face) {
                Err(e) => bad(&e),
                Ok((body, ctype)) => {
                    let attribution: Vec<String> =
                        scene.attribution.iter().map(|s| s.0.clone()).collect();
                    let mut headers = vec![
                        ("X-Attribution".to_string(), attribution.join(", ")),
                        ("X-Scene-Pid".to_string(), format!("{:016x}", scene.map_pid().hash.0)),
                    ];
                    if queries.len() == 1 {
                        headers.push((
                            "X-Query-Pid".to_string(),
                            format!("{:016x}", q.map_pid().hash.0),
                        ));
                    }
                    (200, ctype, body, headers)
                }
            }
        }

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
            let side = |prefix: &str| -> Result<Snapshot, String> {
                let keys = p.get(&format!("{prefix}subject")).unwrap_or("world").to_string();
                let mut combined = Snapshot::empty();
                let mut any = false;
                for k in keys.split(',').filter(|s| !s.is_empty()) {
                    let q = build_query(app, &p, prefix, Some(k)).ok_or("bad overlay query")?;
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
    let (status, ctype, body, extra) = route(app, path, query);
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        _ => "Not Found",
    };
    let mut head = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {ctype}; charset=utf-8\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n",
        body.len()
    );
    for (k, v) in extra {
        head.push_str(&format!("{k}: {v}\r\n"));
    }
    head.push_str("\r\n");
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(body.as_bytes());
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
}

fn main() {
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
