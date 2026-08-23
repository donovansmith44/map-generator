//! Phase-3 laws against the REAL materializer: determinism (law 1),
//! transition composition endpoints (law 3), accumulation identity and
//! substance (law 9), selection coherence (law 10) — on fixtures and on
//! the real ingested source.

use std::collections::BTreeMap;

use atlas_graph_types::chrono::{TimePoint, Year};
use atlas_graph_types::edge::Justification;
use atlas_graph_types::id::SourceId;

use map_adapters::{ingest, EpochSource, IngestConfig};
use map_types::style::*;
use map_types::{
    Anchor, Boundary, BoundaryHistory, BoundaryId, BoundarySource, ChangeEvent, ChangeKind,
    EdgeCharacter, Interval, LayerSet, Lod, MapAddressed, MapError, MapProvider, Monoid,
    RenderQuery, RenderSubject, StyleId, TimeSelector, TransitionStep, UnitVec,
};
use serde_json::json;

use crate::{resample, TimelineProvider};

fn tp(year: i32) -> TimePoint {
    TimePoint::year_only(Year::new(year).unwrap())
}

fn honest_style() -> Style {
    let stroke = |r, pattern| Stroke { color: Rgba(r, 0, 0, 255), width: 1.0, pattern };
    Style::new(
        BoundaryStrokes {
            line: stroke(0, StrokePattern::Solid),
            frontier: stroke(60, StrokePattern::Zonal),
            disputed: stroke(120, StrokePattern::Hatched),
            unknown: stroke(180, StrokePattern::Dashed),
        },
        Paint { fill: Rgba(200, 200, 180, 255) },
        AgeRamp {
            newest: Paint { fill: Rgba(220, 40, 40, 255) },
            oldest: Paint { fill: Rgba(220, 40, 40, 40) },
        },
        LabelStyle { color: Rgba(20, 20, 20, 255), size: 12.0 },
        MarkerStyle { color: Rgba(0, 0, 0, 255), size: 4.0 },
        DeltaEmphasis {
            before: stroke(90, StrokePattern::Dashed),
            after: stroke(30, StrokePattern::Solid),
            seam: stroke(250, StrokePattern::Solid),
        },
    )
    .unwrap()
}

fn fc(features: &[(&str, Vec<Vec<(f64, f64)>>)]) -> String {
    let feats: Vec<serde_json::Value> = features
        .iter()
        .map(|(name, polys)| {
            let coords: Vec<Vec<Vec<[f64; 2]>>> = polys
                .iter()
                .map(|outer| vec![outer.iter().map(|&(lon, lat)| [lon, lat]).collect()])
                .collect();
            json!({
                "type": "Feature",
                "properties": { "NAME": name },
                "geometry": { "type": "MultiPolygon", "coordinates": coords }
            })
        })
        .collect();
    json!({ "type": "FeatureCollection", "features": feats }).to_string()
}

fn square(lon0: f64, lat0: f64, lon1: f64, lat1: f64) -> Vec<(f64, f64)> {
    vec![(lon0, lat0), (lon1, lat0), (lon1, lat1), (lon0, lat1), (lon0, lat0)]
}

fn fixture_epochs() -> Vec<EpochSource> {
    vec![
        EpochSource {
            year: -2000,
            label: "fx_bc2000".to_string(),
            text: fc(&[
                ("Westia", vec![square(0.0, 0.0, 5.0, 10.0)]),
                ("Estia", vec![square(5.0, 0.0, 10.0, 10.0)]),
                ("Goneland", vec![square(50.0, 0.0, 51.0, 1.0)]),
            ]),
        },
        EpochSource {
            year: -1500,
            label: "fx_bc1500".to_string(),
            text: fc(&[
                ("Westia", vec![square(0.0, 0.0, 5.0, 10.0)]),
                ("Estia", vec![square(5.0, 0.0, 12.0, 10.0)]),
                ("Newland", vec![square(60.0, 0.0, 61.0, 1.0)]),
            ]),
        },
        EpochSource {
            year: -1000,
            label: "fx_bc1000".to_string(),
            text: fc(&[
                ("Westia", vec![square(0.0, 0.0, 5.0, 10.0)]),
                ("Newland", vec![square(60.0, 0.0, 62.0, 2.0)]),
            ]),
        },
    ]
}

fn provider_from(epochs: Vec<EpochSource>) -> (TimelineProvider, StyleId) {
    let config = IngestConfig { source: SourceId::new("historical-basemaps"), anchor: None };
    let out = ingest(&config, &epochs).unwrap();
    let style = honest_style();
    let id = style.id();
    let provider = TimelineProvider {
        timeline: out.timeline,
        styles: BTreeMap::from([(id, style)]),
        gazetteer: None,
    };
    (provider, id)
}

fn world_query(style: StyleId, time: TimeSelector) -> RenderQuery {
    RenderQuery {
        subject: RenderSubject::World,
        time,
        viewport: None,
        lod: Lod::exact(),
        layers: LayerSet::GEOMETRY.with(LayerSet::LABELS),
        style,
    }
}

// ------------------------------------------------- law 1: determinism

/// Identical queries against independently constructed providers give
/// BYTE-IDENTICAL scenes — the whole cache/offline story.
#[test]
fn law01_byte_identical_across_constructions() {
    let (p1, s1) = provider_from(fixture_epochs());
    let (p2, s2) = provider_from(fixture_epochs());
    assert_eq!(s1, s2, "same style content, same id");
    for time in [TimeSelector::At(tp(-1800)), TimeSelector::Over(interval(-2000, -1000))] {
        let q = world_query(s1, time);
        let a = p1.render(&q).unwrap();
        let b = p2.render(&q).unwrap();
        assert_eq!(a.canonical_bytes(), b.canonical_bytes());
        assert_eq!(a.map_pid(), b.map_pid());
    }
}

fn interval(from: i32, to: i32) -> Interval {
    Interval::new(tp(from), Some(tp(to))).unwrap()
}

// ------------------------------- law 3: transition composition, ends

#[test]
fn law03_transitions_compose_and_invert() {
    let (p, _) = provider_from(fixture_epochs());
    let vp = map_types::Bbox::whole_world();
    let lod = Lod::exact();

    let t12 = p.transition(tp(-2000), tp(-1500), vp, lod).unwrap();
    let t23 = p.transition(tp(-1500), tp(-1000), vp, lod).unwrap();
    let t13 = p.transition(tp(-2000), tp(-1000), vp, lod).unwrap();
    assert!(!t12.steps.is_empty() && !t23.steps.is_empty());
    // Composed parts ARE the direct journey (endpoint agreement, the
    // strong form: identical scripts, not merely identical outcomes).
    assert_eq!(t12.clone().combine(t23), t13);

    // t -> t is the identity.
    assert!(p.transition(tp(-1500), tp(-1500), vp, lod).unwrap().steps.is_empty());

    // Backwards is the inverse walk.
    let back = p.transition(tp(-1000), tp(-2000), vp, lod).unwrap();
    assert_eq!(back.steps.len(), t13.steps.len());
    let fade_ins = |s: &map_types::TransitionScript| {
        s.steps.iter().filter(|x| matches!(x, TransitionStep::FadeIn { .. })).count()
    };
    let fade_outs = |s: &map_types::TransitionScript| {
        s.steps.iter().filter(|x| matches!(x, TransitionStep::FadeOut { .. })).count()
    };
    assert_eq!(fade_ins(&back), fade_outs(&t13));
    assert_eq!(fade_outs(&back), fade_ins(&t13));

    // The scrubber's stops are the events, ordered.
    let stops = p.changes_between(tp(-2000), tp(-1000));
    assert!(!stops.is_empty());
    assert!(stops.windows(2).all(|w| w[0].at <= w[1].at));
}

// ---------------------- law 4/3: same-arc shifts morph, lawfully

/// A hand-authored history where one arc keeps its identity across a
/// shift: the transition MORPHS it — equal counts by resampling.
#[test]
fn same_arc_shift_morphs() {
    let (mut p, style_id) = provider_from(fixture_epochs());
    let uv = |lat: f64, lon: f64| UnitVec::from_lat_lon_deg(lat, lon);
    let bid = BoundaryId(atlas_graph_types::id::ContentHash(777));
    let mk = |pts: Vec<UnitVec>| Boundary {
        pts,
        character: EdgeCharacter::Line,
        source: BoundarySource::Authored { justification: Justification::default() },
        justification: Justification::default(),
        provenance: "test:authored".to_string(),
    };
    p.timeline.boundaries.insert(
        bid,
        BoundaryHistory {
            versions: vec![
                (
                    Interval::new(tp(-2000), Some(tp(-1500))).unwrap(),
                    mk(vec![uv(0.0, 20.0), uv(1.0, 21.0), uv(2.0, 22.0)]),
                ),
                (
                    Interval::open_from(tp(-1500)),
                    mk(vec![uv(0.0, 20.0), uv(0.5, 20.5), uv(1.5, 21.5), uv(2.5, 22.0), uv(3.0, 22.0)]),
                ),
            ],
        },
    );
    p.timeline.events.push(ChangeEvent {
        at: tp(-1500),
        kind: ChangeKind::Shift { boundary: bid },
        driver: None,
        justification: Justification::default(),
        provenance: "test:authored".to_string(),
    });

    let script = p
        .transition(tp(-1600), tp(-1400), map_types::Bbox::whole_world(), Lod::exact())
        .unwrap();
    let morph = script
        .steps
        .iter()
        .find_map(|s| match s {
            TransitionStep::Morph { boundary, from_pts, to_pts } if *boundary == bid => {
                Some((from_pts.clone(), to_pts.clone()))
            }
            _ => None,
        })
        .expect("same-arc shift must morph");
    assert_eq!(morph.0.len(), morph.1.len(), "equal counts: slerp-pairable");
    assert!(morph.0.len() >= 5);

    // The style registry still resolves; the world still renders.
    let q = world_query(style_id, TimeSelector::At(tp(-1450)));
    assert!(p.render(&q).is_ok());
}

#[test]
fn resample_keeps_endpoints_and_count() {
    let uv = |lat: f64, lon: f64| UnitVec::from_lat_lon_deg(lat, lon);
    let pts = vec![uv(0.0, 0.0), uv(0.0, 10.0), uv(5.0, 20.0)];
    for n in [2usize, 3, 7, 50] {
        let r = resample(&pts, n);
        assert_eq!(r.len(), n);
        assert!(r[0].angle_to(&pts[0]) < 1e-12);
        assert!(r[n - 1].angle_to(&pts[2]) < 1e-12);
    }
}

// -------------------------------- law 9: accumulation on the provider

#[test]
fn law09_single_instant_accumulation_is_the_snapshot() {
    let (p, style) = provider_from(fixture_epochs());
    let at = p.render(&world_query(style, TimeSelector::At(tp(-1800)))).unwrap();
    let over = p
        .render(&world_query(
            style,
            TimeSelector::Over(Interval::new(tp(-1800), Some(tp(-1800))).unwrap()),
        ))
        .unwrap();
    assert_eq!(at.canonical_bytes(), over.canonical_bytes());
}

#[test]
fn law09_accumulation_carries_every_state_age_ramped() {
    let (p, style) = provider_from(fixture_epochs());
    let q = world_query(style, TimeSelector::Over(interval(-2000, -1000)));
    let acc = p.render(&q).unwrap();

    // Estia's two distinct geometries both appear (the long exposure),
    // painted differently by age.
    let estia: Vec<_> = p
        .subjects(tp(-1500))
        .into_iter()
        .filter_map(|s| match s.subject {
            RenderSubject::Region(id) if s.label == "Estia" => Some(id),
            _ => None,
        })
        .collect();
    let estia_layers: Vec<_> = acc.regions.iter().filter(|r| r.region == estia[0]).collect();
    assert!(estia_layers.len() >= 2, "both Estia states in one still image");
    assert!(
        estia_layers.windows(2).any(|w| w[0].paint != w[1].paint),
        "age renders as paint"
    );

    // The newest state is drawn last — on top.
    let last_estia = estia_layers.last().unwrap();
    let newest_alone = p
        .render(&RenderQuery {
            subject: RenderSubject::Region(estia[0]),
            ..world_query(style, TimeSelector::At(tp(-1400)))
        })
        .unwrap();
    assert_eq!(last_estia.outer, newest_alone.regions[0].outer);
}

// ------------------------------- law 10: selection coherence, for real

#[test]
fn law10_lone_rendering_agrees_with_world_selection() {
    let (p, style) = provider_from(fixture_epochs());
    let at = TimeSelector::At(tp(-1800));
    let world = p.render(&world_query(style, at)).unwrap();

    for listing in p.subjects(tp(-1800)) {
        let RenderSubject::Region(id) = listing.subject else { continue };
        let lone = p
            .render(&RenderQuery {
                subject: RenderSubject::Region(id),
                ..world_query(style, at)
            })
            .unwrap();
        assert_eq!(
            world.select_region(id).canonical_bytes(),
            lone.canonical_bytes(),
            "region {} must not drift between lone and in-context rendering",
            listing.label
        );
    }

    // And for a boundary subject (one alive at the queried instant).
    let bid = *p
        .timeline
        .boundaries
        .iter()
        .find(|(_, h)| h.at(&tp(-1800)).is_some())
        .map(|(id, _)| id)
        .unwrap();
    let lone = p
        .render(&RenderQuery {
            subject: RenderSubject::Boundary(bid),
            ..world_query(style, at)
        })
        .unwrap();
    assert_eq!(world.select_boundary(bid).canonical_bytes(), lone.canonical_bytes());
}

// ------------------------------------------------ contract edges

#[test]
fn contract_errors_fail_loud() {
    let (p, style) = provider_from(fixture_epochs());
    // Terrain is phase 5: loud, not degraded.
    let rid = p
        .subjects(tp(-1800))
        .into_iter()
        .find_map(|s| match s.subject {
            RenderSubject::Region(id) => Some(id),
            _ => None,
        })
        .unwrap();
    let q = RenderQuery {
        subject: RenderSubject::RegionTerrain(rid),
        ..world_query(style, TimeSelector::At(tp(-1800)))
    };
    assert_eq!(p.render(&q), Err(MapError::TerrainUnavailable));

    // Unknown style is unknown, not defaulted.
    let q = RenderQuery {
        style: StyleId(atlas_graph_types::id::ContentHash(0xdead)),
        ..world_query(style, TimeSelector::At(tp(-1800)))
    };
    assert!(matches!(p.render(&q), Err(MapError::UnknownStyle(_))));

    // A region before its rise: the subject exists, the time is empty.
    let gone = p
        .subjects(tp(-2000))
        .into_iter()
        .find_map(|s| (s.label == "Goneland").then_some(s.subject))
        .unwrap();
    let q = RenderQuery { subject: gone, ..world_query(style, TimeSelector::At(tp(-1200))) };
    assert!(matches!(p.render(&q), Err(MapError::NothingAtTime(_))));
}

// ----------------------------------- a delta renders (P4 for changes)

#[test]
fn a_change_event_renders_as_a_scene() {
    let (p, style) = provider_from(fixture_epochs());
    let event = p.changes_between(tp(-2000), tp(-1000))[0];
    let q = RenderQuery {
        subject: RenderSubject::Change(event.id()),
        ..world_query(style, TimeSelector::At(event.at))
    };
    let scene = p.render(&q).unwrap();
    assert!(!scene.boundaries.is_empty(), "a delta is a drawing, not a caption");
}

// ------------------------------------------- the real source, whole

#[test]
fn real_source_renders_deterministically() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/historical-basemaps");
    let mut epochs = Vec::new();
    for entry in std::fs::read_dir(&dir).expect("vendored data present") {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("geojson") {
            continue;
        }
        let label = path.file_stem().unwrap().to_string_lossy().to_string();
        let year = map_adapters::epoch_year_from_label(&label).unwrap();
        epochs.push(EpochSource { year, label, text: std::fs::read_to_string(&path).unwrap() });
    }
    let config = IngestConfig {
        source: SourceId::new("historical-basemaps"),
        anchor: Some(Anchor {
            frame: "biblical (Ussher tradition)".to_string(),
            at: tp(-4004),
            justification: Justification::default(),
            provenance: "owner-config:ussher-tradition (pending atlas C2 export)".to_string(),
        }),
    };
    let out = ingest(&config, &epochs).unwrap();
    let style = honest_style();
    let sid = style.id();
    let p = TimelineProvider {
        timeline: out.timeline,
        styles: BTreeMap::from([(sid, style)]),
        gazetteer: None,
    };

    // Determinism on the real world, with lod engaged.
    let q = RenderQuery { lod: Lod(0.002), ..world_query(sid, TimeSelector::At(tp(-1900))) };
    let a = p.render(&q).unwrap();
    let b = p.render(&q).unwrap();
    assert_eq!(a.canonical_bytes(), b.canonical_bytes());
    assert!(a.regions.len() > 10);
    assert!(!a.attribution.is_empty(), "licensing rides every response");

    // Lod actually thins geometry (law 7 doing real work).
    let exact = p.render(&world_query(sid, TimeSelector::At(tp(-1900)))).unwrap();
    let pts = |s: &map_types::Snapshot| -> usize {
        s.regions.iter().flat_map(|r| &r.outer).map(|ring| ring.len()).sum()
    };
    assert!(pts(&a) < pts(&exact), "{} !< {}", pts(&a), pts(&exact));

    // Selection coherence holds on real data too.
    let some_region = p
        .subjects(tp(-1900))
        .into_iter()
        .find_map(|s| match s.subject {
            RenderSubject::Region(id) => Some(id),
            _ => None,
        })
        .unwrap();
    let lone = p
        .render(&RenderQuery {
            subject: RenderSubject::Region(some_region),
            ..world_query(sid, TimeSelector::At(tp(-1900)))
        })
        .unwrap();
    assert_eq!(
        exact.select_region(some_region).canonical_bytes(),
        lone.canonical_bytes()
    );
}
