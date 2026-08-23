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

use map_adapters::{epoch_year_from_label, ingest, EpochSource, IngestConfig};
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
    /// Scrub stops: the change-event years, preceded by one dawn stop
    /// showing the state before the first recorded change.
    stops: Vec<i32>,
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
    let (p_style, s_style) = (parchment(), slate());
    let styles = vec![("parchment", p_style.id()), ("slate", s_style.id())];
    let provider: Arc<dyn MapProvider + Send + Sync> = Arc::new(TimelineProvider {
        timeline: out.timeline,
        styles: BTreeMap::from([(p_style.id(), p_style), (s_style.id(), s_style)]),
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
    App { provider, styles, stops }
}

// ------------------------------------------------------------ queries

struct Params(BTreeMap<String, String>);

impl Params {
    fn parse(query: &str) -> Params {
        Params(
            query
                .split('&')
                .filter_map(|kv| kv.split_once('='))
                .map(|(k, v)| (k.to_string(), v.to_string()))
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

fn build_query(app: &App, p: &Params, prefix: &str) -> Option<RenderQuery> {
    let subject = parse_subject(p.get(&format!("{prefix}subject")).unwrap_or("world"))?;
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

fn encode(p: &Params, scene: &Snapshot) -> Result<(String, &'static str), String> {
    match p.get("encoder").unwrap_or("svg") {
        "geojson" => GeoJsonEncoder
            .encode(scene)
            .map(|s| (s, "application/geo+json"))
            .map_err(|e| e.0),
        _ => SvgEncoder::default()
            .encode(scene)
            .map(|s| (s, "image/svg+xml"))
            .map_err(|e| e.0),
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
            let body = serde_json::json!({
                "stops": app.stops,
                "styles": styles,
                "encoders": ["svg", "geojson"],
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
            let Some(q) = build_query(app, &p, "") else { return bad("bad query") };
            match app.provider.render(&q) {
                Err(e) => bad(&format!("{e:?}")),
                Ok(scene) => match encode(&p, &scene) {
                    Err(e) => bad(&e),
                    Ok((body, ctype)) => {
                        let attribution: Vec<String> =
                            scene.attribution.iter().map(|s| s.0.clone()).collect();
                        let headers = vec![
                            ("X-Attribution".to_string(), attribution.join(", ")),
                            ("X-Scene-Pid".to_string(), format!("{:016x}", scene.map_pid().hash.0)),
                            ("X-Query-Pid".to_string(), format!("{:016x}", q.map_pid().hash.0)),
                        ];
                        (200, ctype, body, headers)
                    }
                },
            }
        }

        // The overlay scratchpad: TWO scenes composed at the SEMANTIC
        // level (the monoid), then encoded once — never on bytes.
        "/api/overlay" => {
            let (Some(qa), Some(qb)) = (build_query(app, &p, "a_"), build_query(app, &p, "b_"))
            else {
                return bad("bad overlay query");
            };
            let scene = match (app.provider.render(&qa), app.provider.render(&qb)) {
                (Ok(a), Ok(b)) => a.combine(b),
                (Err(e), _) | (_, Err(e)) => return bad(&format!("{e:?}")),
            };
            match encode(&p, &scene) {
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
