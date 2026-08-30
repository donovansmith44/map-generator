//! Phase-3 laws against the REAL materializer: determinism (law 1),
//! transition composition endpoints (law 3), accumulation identity and
//! substance (law 9), selection coherence (law 10) — on fixtures and on
//! the real ingested source.

use std::collections::BTreeMap;

use atlas_graph_types::covenant::{TimePoint, Year};
use atlas_graph_types::covenant::Justification;
use atlas_graph_types::covenant::SourceId;

use map_adapters::{ingest, EpochSource, IngestConfig};
use map_types::style::*;
use map_types::{
    Anchor, Boundary, BoundaryHistory, BoundaryId, BoundarySource, ChangeEvent, ChangeKind,
    EdgeCharacter, Interval, LayerSet, Lod, MapAddressed, MapError, MapProvider, Monoid,
    RenderQuery, RenderSubject, StyleId, TimeSelector, TransitionStep, UnitVec,
};
use serde_json::json;

use crate::{resample, TimelineProvider};


fn test_labeling(base: LabelStyle) -> map_types::style::Labeling {
    const TV: map_types::style::TypeVoice = map_types::style::TypeVoice {
    family: "sans-serif",
    weight: 600,
    italic: false,
    uppercase: false,
    tracking_em: 0.0,
    advance_em: 0.62,
};
    map_types::style::Labeling {
        base,
        territory: TV,
        water: TV,
        place: TV,
        scale: map_types::style::LabelScale {
            unit_area_sr: 3.6e-5,
            min: 0.85,
            max: 2.1,
            water_shrink: 0.8,
            water_ink: 0.45,
        },
    }
}

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
            way: stroke(240, StrokePattern::Dashed),
        },
        Paint { fill: Rgba(200, 200, 180, 255) },
        Paint { fill: Rgba(120, 160, 200, 235) },
        AgeRamp {
            newest: Paint { fill: Rgba(150, 110, 80, 200) },
            oldest: Paint { fill: Rgba(225, 214, 180, 200) },
        },
        None,
        AgeRamp {
            newest: Paint { fill: Rgba(220, 40, 40, 255) },
            oldest: Paint { fill: Rgba(220, 40, 40, 40) },
        },
        test_labeling(LabelStyle { color: Rgba(20, 20, 20, 255), halo: Rgba(245, 240, 225, 220), size: 12.0 }),
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
    let config = IngestConfig { source: SourceId::new("historical-basemaps"), anchor: None, snap: None };
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
    let bid = BoundaryId(atlas_graph_types::covenant::ContentHash(777));
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

/// Law 7's second clause, sharpened: when simplification would
/// collapse a ring below three points, the feature keeps a MINIMAL
/// ring — never its full exact geometry. (The old fallback returned
/// every source vertex, so coarser lod made hemisphere scenes BIGGER.)
#[test]
fn collapsed_rings_fall_back_minimal_not_exact() {
    let (mut p, style_id) = provider_from(fixture_epochs());
    let uv = |lat: f64, lon: f64| UnitVec::from_lat_lon_deg(lat, lon);
    // A 64-point island 0.2 degrees across — far below a 0.05 rad
    // tolerance, so DP collapses it and the fallback must kick in.
    let mut pts: Vec<UnitVec> = (0..64)
        .map(|i| {
            let a = i as f64 / 64.0 * std::f64::consts::TAU;
            uv(30.0 + 0.1 * a.sin(), 40.0 + 0.1 * a.cos())
        })
        .collect();
    let first = pts[0];
    pts.push(first); // closed polyline
    let bid = BoundaryId(atlas_graph_types::covenant::ContentHash(4242));
    p.timeline.boundaries.insert(
        bid,
        BoundaryHistory {
            versions: vec![(
                Interval::open_from(tp(-3000)),
                Boundary {
                    pts,
                    character: EdgeCharacter::Line,
                    source: BoundarySource::Authored { justification: Justification::default() },
                    justification: Justification::default(),
                    provenance: "test:island".to_string(),
                },
            )],
        },
    );
    let rid = map_types::RegionId(atlas_graph_types::covenant::ContentHash(4243));
    p.timeline.regions.insert(
        rid,
        map_types::RegionHistory {
            class: map_types::RegionClass::Land,
            label_history: vec![(Interval::open_from(tp(-3000)), "tiny island".to_string())],
            geom_history: vec![(
                Interval::open_from(tp(-3000)),
                map_types::RegionGeom {
                    parts: vec![map_types::RegionPart {
                        cycle: vec![(bid, map_types::Orientation::Forward)],
                        holes: vec![],
                    }],
                },
            )],
        },
    );

    let q = RenderQuery { lod: Lod(0.05), ..world_query(style_id, TimeSelector::At(tp(-1900))) };
    let scene = p.render(&q).unwrap();
    let island = scene.regions.iter().find(|r| r.region == rid).expect("island renders");
    let n = island.outer.iter().map(|ring| ring.len()).sum::<usize>();
    assert!(n >= 3, "topology preserved: a ring survives");
    assert!(n <= 8, "collapse fallback must be minimal, got {n} points");
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
    let acc = p.render(&world_query(style, TimeSelector::Over(interval(-2000, -1000)))).unwrap();

    // A FOCUSED subject's range carries every state's fill, age-ramped:
    // Estia's two distinct geometries in one still image.
    let estia: Vec<_> = p
        .subjects(tp(-1500))
        .into_iter()
        .filter_map(|s| match s.subject {
            RenderSubject::Region(id) if s.label == "Estia" => Some(id),
            _ => None,
        })
        .collect();
    let acc_estia = p
        .render(&RenderQuery {
            subject: RenderSubject::Region(estia[0]),
            ..world_query(style, TimeSelector::Over(interval(-2000, -1000)))
        })
        .unwrap();
    let estia_layers: Vec<_> = acc_estia.regions.iter().filter(|r| r.region == estia[0]).collect();
    assert!(estia_layers.len() >= 2, "both Estia states in one still image");
    assert!(
        estia_layers.windows(2).any(|w| w[0].paint != w[1].paint),
        "age renders as paint"
    );

    // A WORLD range does not stack fills into a blob: each region
    // fills once (its newest state), the lines carry the history.
    let mut per_region: std::collections::BTreeMap<_, usize> = Default::default();
    for r in &acc.regions {
        *per_region.entry(r.region).or_default() += 1;
    }
    assert!(per_region.values().all(|&n| n == 1), "one fill per region in a world range");

    // THE DIFFS ARE LINES: every border that existed in the range is
    // drawn, and age renders in the stroke — at least two distinct
    // tints, with the most recent lines drawn last (on top).
    assert!(!acc.boundaries.is_empty(), "an accumulation draws its borders");
    let strokes: std::collections::BTreeSet<_> =
        acc.boundaries.iter().map(|b| b.stroke.color.3).collect();
    assert!(strokes.len() >= 2, "border age renders as tint: {strokes:?}");
    let alphas: Vec<u8> = acc.boundaries.iter().map(|b| b.stroke.color.3).collect();
    assert!(alphas.windows(2).all(|w| w[0] <= w[1]), "newest lines on top");

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

// -------------------------------- the atlas palette (what's what)

/// With a palette, touching territories never share a slot, the
/// assignment is deterministic, and lone rendering still equals
/// world-selection byte for byte (law 10 survives coloring because
/// colors derive from the TIMELINE, not the scene).
#[test]
fn palette_colors_touching_regions_distinctly() {
    let palette = [
        Paint { fill: Rgba(0x39, 0x87, 0xe5, 205) },
        Paint { fill: Rgba(0xd9, 0x59, 0x26, 205) },
        Paint { fill: Rgba(0x19, 0x9e, 0x70, 205) },
        Paint { fill: Rgba(0xc9, 0x85, 0x00, 205) },
        Paint { fill: Rgba(0xd5, 0x51, 0x81, 205) },
        Paint { fill: Rgba(0x00, 0x83, 0x00, 205) },
        Paint { fill: Rgba(0x90, 0x85, 0xe9, 205) },
        Paint { fill: Rgba(0xe6, 0x67, 0x67, 205) },
    ];
    let stroke = |r, pattern| Stroke { color: Rgba(r, 0, 0, 255), width: 1.0, pattern };
    let style = Style::new(
        BoundaryStrokes {
            line: stroke(0, StrokePattern::Solid),
            frontier: stroke(60, StrokePattern::Zonal),
            disputed: stroke(120, StrokePattern::Hatched),
            unknown: stroke(180, StrokePattern::Dashed),
            way: stroke(240, StrokePattern::Dashed),
        },
        Paint { fill: Rgba(200, 200, 180, 255) },
        Paint { fill: Rgba(120, 160, 200, 235) },
        AgeRamp {
            newest: Paint { fill: Rgba(150, 110, 80, 200) },
            oldest: Paint { fill: Rgba(225, 214, 180, 200) },
        },
        Some(palette),
        AgeRamp {
            newest: Paint { fill: Rgba(220, 40, 40, 255) },
            oldest: Paint { fill: Rgba(220, 40, 40, 40) },
        },
        test_labeling(LabelStyle { color: Rgba(20, 20, 20, 255), halo: Rgba(245, 240, 225, 220), size: 12.0 }),
        MarkerStyle { color: Rgba(0, 0, 0, 255), size: 4.0 },
        DeltaEmphasis {
            before: stroke(90, StrokePattern::Dashed),
            after: stroke(30, StrokePattern::Solid),
            seam: stroke(250, StrokePattern::Solid),
        },
    )
    .unwrap();
    let sid = style.id();
    let config = IngestConfig { source: SourceId::new("historical-basemaps"), anchor: None, snap: None };
    let out = ingest(&config, &fixture_epochs()).unwrap();
    let p = TimelineProvider {
        timeline: out.timeline,
        styles: BTreeMap::from([(sid, style)]),
        gazetteer: None,
    };

    let at = TimeSelector::At(tp(-1800));
    let world = p.render(&world_query(sid, at)).unwrap();
    // Westia and Estia share an arc: their fills must differ.
    let paints: BTreeMap<_, _> = world.regions.iter().map(|r| (r.region, r.paint)).collect();
    let ids: Vec<_> = p
        .subjects(tp(-1800))
        .into_iter()
        .filter_map(|s| match (s.subject, s.label.as_str()) {
            (RenderSubject::Region(id), "Westia") => Some(("w", id)),
            (RenderSubject::Region(id), "Estia") => Some(("e", id)),
            _ => None,
        })
        .collect();
    let w = ids.iter().find(|(t, _)| *t == "w").unwrap().1;
    let e = ids.iter().find(|(t, _)| *t == "e").unwrap().1;
    assert_ne!(paints[&w], paints[&e], "touching territories never match");

    // Deterministic across renders, and law 10 holds under color.
    let again = p.render(&world_query(sid, at)).unwrap();
    assert_eq!(world.canonical_bytes(), again.canonical_bytes());
    let lone = p
        .render(&RenderQuery { subject: RenderSubject::Region(w), ..world_query(sid, at) })
        .unwrap();
    assert_eq!(world.select_region(w).canonical_bytes(), lone.canonical_bytes());
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
        style: StyleId(atlas_graph_types::covenant::ContentHash(0xdead)),
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
        snap: None,
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

    // …and MONOTONICALLY: a coarser lod never emits more points. The
    // collapse fallback must yield a minimal ring, not the full exact
    // geometry, or hemisphere views inflate as tolerance grows.
    let coarse =
        p.render(&RenderQuery { lod: Lod(0.02), ..world_query(sid, TimeSelector::At(tp(-1900))) })
            .unwrap();
    let all_pts = |s: &map_types::Snapshot| -> usize {
        s.regions.iter().flat_map(|r| r.outer.iter().chain(&r.holes)).map(|ring| ring.len()).sum()
    };
    assert!(
        all_pts(&coarse) <= all_pts(&a),
        "coarser lod grew the scene: {} > {}",
        all_pts(&coarse),
        all_pts(&a)
    );

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

/// A Way boundary's stations become markers and labels — the journey
/// is legible, not a bare dashed line — and both carry the scripture
/// source so a semantic selection keeps them.
#[test]
fn way_routes_emit_their_stations() {
    use map_types::scene::LabelSubject;
    let (mut p, style_id) = provider_from(fixture_epochs());
    let uv = |lat: f64, lon: f64| UnitVec::from_lat_lon_deg(lat, lon);
    let stations = [("Antioch of Syria", 36.2, 36.16), ("Seleucia", 36.12, 35.93)];
    let place = |name: &str| atlas_graph_types::covenant::PlaceId::new(format!("place:{name}"));
    let gaz = map_types::GazetteerExport {
        atlas_root: atlas_graph_types::covenant::ContentHash(0),
        places: stations
            .iter()
            .map(|(n, lat, lon)| {
                (
                    place(n),
                    map_types::GazetteerEntry {
                        canonical_name: n.to_string(),
                        position: uv(*lat, *lon),
                        aliases: Vec::new(),
                        provenance: None,
                        attestations: Vec::new(),
                    },
                )
            })
            .collect(),
    };
    p.gazetteer = Some(gaz);
    let bid = BoundaryId(atlas_graph_types::covenant::ContentHash(9001));
    let verses = atlas_graph_types::covenant::LocusRange::new(
        atlas_graph_types::covenant::BibleLocus::whole(atlas_graph_types::covenant::VerseRef {
            book: 44, chapter: 13, verse: 1,
        }),
        atlas_graph_types::covenant::BibleLocus::whole(atlas_graph_types::covenant::VerseRef {
            book: 44, chapter: 14, verse: 28,
        }),
    )
    .unwrap();
    p.timeline.boundaries.insert(
        bid,
        BoundaryHistory {
            versions: vec![(
                Interval::open_from(tp(45)),
                Boundary {
                    pts: stations.iter().map(|(_, lat, lon)| uv(*lat, *lon)).collect(),
                    character: EdgeCharacter::Way,
                    source: BoundarySource::Survey(map_types::BorderSurvey {
                        verses,
                        waypoints: stations
                            .iter()
                            .map(|(n, _, _)| map_types::AtlasPlaceRef(place(n)))
                            .collect(),
                        interpolation: map_types::InterpolationMethod::Geodesic,
                        provenance: "test:route".to_string(),
                    }),
                    justification: Justification::default(),
                    provenance: "test:route".to_string(),
                },
            )],
        },
    );

    // A second way through a SHARED station: the station keeps ONE
    // marker and one name, however many journeys pass through it.
    let bid2 = BoundaryId(atlas_graph_types::covenant::ContentHash(9002));
    let verses2 = atlas_graph_types::covenant::LocusRange::new(
        atlas_graph_types::covenant::BibleLocus::whole(atlas_graph_types::covenant::VerseRef {
            book: 44, chapter: 15, verse: 36,
        }),
        atlas_graph_types::covenant::BibleLocus::whole(atlas_graph_types::covenant::VerseRef {
            book: 44, chapter: 18, verse: 22,
        }),
    )
    .unwrap();
    p.timeline.boundaries.insert(
        bid2,
        BoundaryHistory {
            versions: vec![(
                Interval::open_from(tp(49)),
                Boundary {
                    pts: vec![uv(36.2, 36.16), uv(36.92, 34.9)],
                    character: EdgeCharacter::Way,
                    source: BoundarySource::Survey(map_types::BorderSurvey {
                        verses: verses2,
                        waypoints: vec![map_types::AtlasPlaceRef(place("Antioch of Syria"))],
                        interpolation: map_types::InterpolationMethod::Geodesic,
                        provenance: "test:route2".to_string(),
                    }),
                    justification: Justification::default(),
                    provenance: "test:route2".to_string(),
                },
            )],
        },
    );

    let scene = p
        .render(&RenderQuery {
            layers: LayerSet::GEOMETRY.with(LayerSet::LABELS).with(LayerSet::JOURNEYS),
            ..world_query(style_id, TimeSelector::At(tp(64)))
        })
        .unwrap();
    let scripture = SourceId::new("scripture");
    let station_markers: Vec<_> =
        scene.markers.iter().filter(|m| m.sources.contains(&scripture)).collect();
    assert_eq!(station_markers.len(), 2, "one marker per station, shared stations deduped");
    let station_labels: Vec<_> = scene
        .labels
        .iter()
        .filter(|l| matches!(&l.subject, LabelSubject::Place(_)))
        .collect();
    assert_eq!(station_labels.len(), 2, "stations are named, attached to their places");
    assert!(station_labels.iter().any(|l| l.text == "Seleucia"));
    let _ = bid; // the way itself is still on the scene as a boundary
}

/// A journey's stations are SUBJECTS: they appear in subjects() while
/// their way is active (click-to-focus needs a listing to bind to),
/// and a focused station renders scripture-grounded, so bible mode
/// keeps the very place the text walks through.
#[test]
fn way_stations_are_subjects_and_scripture_grounded() {
    let (mut p, style_id) = provider_from(fixture_epochs());
    let uv = |lat: f64, lon: f64| UnitVec::from_lat_lon_deg(lat, lon);
    let pid = atlas_graph_types::covenant::PlaceId::new("place:Ephesus".to_string());
    p.gazetteer = Some(map_types::GazetteerExport {
        atlas_root: atlas_graph_types::covenant::ContentHash(0),
        places: [(
            pid.clone(),
            map_types::GazetteerEntry {
                canonical_name: "Ephesus".to_string(),
                position: uv(37.94, 27.34),
                aliases: Vec::new(),
                provenance: None,
                attestations: Vec::new(),
            },
        )]
        .into_iter()
        .collect(),
    });
    let verses = atlas_graph_types::covenant::LocusRange::new(
        atlas_graph_types::covenant::BibleLocus::whole(atlas_graph_types::covenant::VerseRef {
            book: 44, chapter: 18, verse: 23,
        }),
        atlas_graph_types::covenant::BibleLocus::whole(atlas_graph_types::covenant::VerseRef {
            book: 44, chapter: 21, verse: 17,
        }),
    )
    .unwrap();
    p.timeline.boundaries.insert(
        BoundaryId(atlas_graph_types::covenant::ContentHash(9010)),
        BoundaryHistory {
            versions: vec![(
                Interval::open_from(tp(53)),
                Boundary {
                    pts: vec![uv(37.94, 27.34), uv(39.75, 26.15)],
                    character: EdgeCharacter::Way,
                    source: BoundarySource::Survey(map_types::BorderSurvey {
                        verses,
                        waypoints: vec![map_types::AtlasPlaceRef(pid.clone())],
                        interpolation: map_types::InterpolationMethod::Geodesic,
                        provenance: "test:route".to_string(),
                    }),
                    justification: Justification::default(),
                    provenance: "test:route".to_string(),
                },
            )],
        },
    );

    // Listed while the way is active, absent before it.
    let listed = |year: i32| {
        p.subjects(tp(year)).into_iter().any(|s| {
            matches!(&s.subject, RenderSubject::Point(r) if r.0 == pid) && s.label == "Ephesus"
        })
    };
    assert!(listed(64), "an active way's station is a subject");
    assert!(!listed(-1000), "a station is not listed before its way exists");

    // A focused station is scripture-grounded (bible mode keeps it).
    let q = RenderQuery {
        subject: RenderSubject::Point(map_types::AtlasPlaceRef(pid)),
        ..world_query(style_id, TimeSelector::At(tp(64)))
    };
    let scene = p.render(&q).unwrap();
    assert_eq!(scene.markers.len(), 1);
    assert!(
        scene.markers[0].sources.contains(&SourceId::new("scripture")),
        "a station the text walks through is scripture-grounded"
    );
}

/// Journeys are their own LAYER: a query without LayerSet::JOURNEYS
/// gets a map with no ways and no stations — maps with and without
/// them are both first-class artifacts.
#[test]
fn journeys_are_a_toggleable_layer() {
    let (mut p, style_id) = provider_from(fixture_epochs());
    let uv = |lat: f64, lon: f64| UnitVec::from_lat_lon_deg(lat, lon);
    let pid = atlas_graph_types::covenant::PlaceId::new("place:Derbe".to_string());
    p.gazetteer = Some(map_types::GazetteerExport {
        atlas_root: atlas_graph_types::covenant::ContentHash(0),
        places: [(
            pid.clone(),
            map_types::GazetteerEntry {
                canonical_name: "Derbe".to_string(),
                position: uv(37.35, 33.25),
                aliases: Vec::new(),
                provenance: None,
                attestations: Vec::new(),
            },
        )]
        .into_iter()
        .collect(),
    });
    let verses = atlas_graph_types::covenant::LocusRange::new(
        atlas_graph_types::covenant::BibleLocus::whole(atlas_graph_types::covenant::VerseRef {
            book: 44, chapter: 13, verse: 1,
        }),
        atlas_graph_types::covenant::BibleLocus::whole(atlas_graph_types::covenant::VerseRef {
            book: 44, chapter: 14, verse: 28,
        }),
    )
    .unwrap();
    let bid = BoundaryId(atlas_graph_types::covenant::ContentHash(9020));
    p.timeline.boundaries.insert(
        bid,
        BoundaryHistory {
            versions: vec![(
                Interval::open_from(tp(45)),
                Boundary {
                    pts: vec![uv(37.35, 33.25), uv(37.58, 32.45)],
                    character: EdgeCharacter::Way,
                    source: BoundarySource::Survey(map_types::BorderSurvey {
                        verses,
                        waypoints: vec![map_types::AtlasPlaceRef(pid)],
                        interpolation: map_types::InterpolationMethod::Geodesic,
                        provenance: "test:route".to_string(),
                    }),
                    justification: Justification::default(),
                    provenance: "test:route".to_string(),
                },
            )],
        },
    );

    let has_way = |layers: LayerSet| {
        let q = RenderQuery {
            layers,
            ..world_query(style_id, TimeSelector::At(tp(64)))
        };
        let scene = p.render(&q).unwrap();
        let way = scene.boundaries.iter().any(|b| b.boundary == bid);
        (way, scene.markers.len())
    };
    let with = has_way(LayerSet::GEOMETRY.with(LayerSet::LABELS).with(LayerSet::JOURNEYS));
    let without = has_way(LayerSet::GEOMETRY.with(LayerSet::LABELS));
    assert!(with.0 && with.1 == 1, "journeys layer carries the way and its station");
    assert!(!without.0 && without.1 == 0, "without the layer, no way and no stations");
}

/// A journey mid-walk shows THE ROAD SO FAR: at each stop inside its
/// span the way is truncated to the time-proportional piece, stations
/// appear as they are reached, outside the span nothing shows, and a
/// range bracketing the whole walk carries the whole way.
#[test]
fn journeys_render_partially_in_time() {
    let (mut p, style_id) = provider_from(fixture_epochs());
    let uv = |lat: f64, lon: f64| UnitVec::from_lat_lon_deg(lat, lon);
    let stations = [("Alpha", 0.0, 30.0), ("Bravo", 0.0, 35.0), ("Charlie", 0.0, 40.0)];
    let place = |name: &str| atlas_graph_types::covenant::PlaceId::new(format!("place:{name}"));
    p.gazetteer = Some(map_types::GazetteerExport {
        atlas_root: atlas_graph_types::covenant::ContentHash(0),
        places: stations
            .iter()
            .map(|(n, lat, lon)| {
                (
                    place(n),
                    map_types::GazetteerEntry {
                        canonical_name: n.to_string(),
                        position: uv(*lat, *lon),
                        aliases: Vec::new(),
                        provenance: None,
                        attestations: Vec::new(),
                    },
                )
            })
            .collect(),
    });
    let verses = atlas_graph_types::covenant::LocusRange::new(
        atlas_graph_types::covenant::BibleLocus::whole(atlas_graph_types::covenant::VerseRef {
            book: 44, chapter: 13, verse: 1,
        }),
        atlas_graph_types::covenant::BibleLocus::whole(atlas_graph_types::covenant::VerseRef {
            book: 44, chapter: 14, verse: 28,
        }),
    )
    .unwrap();
    let bid = BoundaryId(atlas_graph_types::covenant::ContentHash(9030));
    p.timeline.boundaries.insert(
        bid,
        BoundaryHistory {
            versions: vec![(
                // departure 45, arrival 47, half-open end
                Interval::new(tp(45), Some(tp(48))).unwrap(),
                Boundary {
                    pts: stations.iter().map(|(_, lat, lon)| uv(*lat, *lon)).collect(),
                    character: EdgeCharacter::Way,
                    source: BoundarySource::Survey(map_types::BorderSurvey {
                        verses,
                        waypoints: stations
                            .iter()
                            .map(|(n, _, _)| map_types::AtlasPlaceRef(place(n)))
                            .collect(),
                        interpolation: map_types::InterpolationMethod::Geodesic,
                        provenance: "test:route".to_string(),
                    }),
                    justification: Justification::default(),
                    provenance: "test:route".to_string(),
                },
            )],
        },
    );

    // The journey's own scrub stops, as the adapter would write them.
    for year in [45, 47] {
        p.timeline.events.push(ChangeEvent {
            at: tp(year),
            kind: ChangeKind::Journey { boundary: bid },
            driver: None,
            justification: Justification::default(),
            provenance: "test:route".to_string(),
        });
    }

    let render_at = |time: TimeSelector| {
        let q = RenderQuery {
            layers: LayerSet::GEOMETRY.with(LayerSet::LABELS).with(LayerSet::JOURNEYS),
            ..world_query(style_id, time)
        };
        let scene = p.render(&q).unwrap();
        let way = scene.boundaries.iter().find(|b| b.boundary == bid).cloned();
        let max_lon = way.as_ref().map(|b| {
            b.pts
                .iter()
                .map(|pt| pt.y().atan2(pt.x()).to_degrees())
                .fold(f64::NEG_INFINITY, f64::max)
        });
        (way.is_some(), max_lon, scene.markers.len())
    };

    // Outside the span: nothing.
    assert_eq!(render_at(TimeSelector::At(tp(44))).0, false, "not yet departed");
    assert_eq!(render_at(TimeSelector::At(tp(48))).0, false, "long since arrived");

    // Year one of three: a third of the road, first station only.
    let (there, lon, stations_seen) = render_at(TimeSelector::At(tp(45)));
    assert!(there);
    let lon = lon.unwrap();
    assert!(
        (lon - 33.33).abs() < 0.5,
        "one third of the ten-degree road, got as far as {lon:.2}"
    );
    assert_eq!(stations_seen, 1, "only Alpha is behind them");

    // Year two: two thirds, Bravo reached.
    let (_, lon, stations_seen) = render_at(TimeSelector::At(tp(46)));
    assert!((lon.unwrap() - 36.66).abs() < 0.5);
    assert_eq!(stations_seen, 2);

    // Arrival year: the whole way.
    let (_, lon, stations_seen) = render_at(TimeSelector::At(tp(47)));
    assert!((lon.unwrap() - 40.0).abs() < 0.01);
    assert_eq!(stations_seen, 3);

    // A range bracketing the walk: the whole way.
    let over = TimeSelector::Over(Interval::new(tp(44), Some(tp(48))).unwrap());
    let (there, lon, _) = render_at(over);
    assert!(there, "the range carries the journey");
    assert!((lon.unwrap() - 40.0).abs() < 0.01, "…all of it");
}

/// The JOURNEYS layer rides EVERY scene that asks for it: focusing a
/// region must not silently drop the ways walked through it.
#[test]
fn journeys_ride_focused_subjects() {
    let (mut p, style_id) = provider_from(fixture_epochs());
    let uv = |lat: f64, lon: f64| UnitVec::from_lat_lon_deg(lat, lon);
    let pid = atlas_graph_types::covenant::PlaceId::new("place:Waystop".to_string());
    p.gazetteer = Some(map_types::GazetteerExport {
        atlas_root: atlas_graph_types::covenant::ContentHash(0),
        places: [(
            pid.clone(),
            map_types::GazetteerEntry {
                canonical_name: "Waystop".to_string(),
                position: uv(2.0, 3.0),
                aliases: Vec::new(),
                provenance: None,
                attestations: Vec::new(),
            },
        )]
        .into_iter()
        .collect(),
    });
    let verses = atlas_graph_types::covenant::LocusRange::new(
        atlas_graph_types::covenant::BibleLocus::whole(atlas_graph_types::covenant::VerseRef {
            book: 44, chapter: 13, verse: 1,
        }),
        atlas_graph_types::covenant::BibleLocus::whole(atlas_graph_types::covenant::VerseRef {
            book: 44, chapter: 14, verse: 28,
        }),
    )
    .unwrap();
    let bid = BoundaryId(atlas_graph_types::covenant::ContentHash(9040));
    p.timeline.boundaries.insert(
        bid,
        BoundaryHistory {
            versions: vec![(
                Interval::open_from(tp(-1800)),
                Boundary {
                    pts: vec![uv(2.0, 3.0), uv(4.0, 7.0)],
                    character: EdgeCharacter::Way,
                    source: BoundarySource::Survey(map_types::BorderSurvey {
                        verses,
                        waypoints: vec![map_types::AtlasPlaceRef(pid)],
                        interpolation: map_types::InterpolationMethod::Geodesic,
                        provenance: "test:route".to_string(),
                    }),
                    justification: Justification::default(),
                    provenance: "test:route".to_string(),
                },
            )],
        },
    );

    let westia = p
        .subjects(tp(-1800))
        .into_iter()
        .find_map(|s| match s.subject {
            RenderSubject::Region(id) if s.label == "Westia" => Some(id),
            _ => None,
        })
        .expect("Westia exists");
    let q = RenderQuery {
        subject: RenderSubject::Region(westia),
        layers: LayerSet::GEOMETRY.with(LayerSet::LABELS).with(LayerSet::JOURNEYS),
        ..world_query(style_id, TimeSelector::At(tp(-1800)))
    };
    let scene = p.render(&q).unwrap();
    assert!(
        scene.boundaries.iter().any(|b| b.boundary == bid),
        "the way renders under a focused region"
    );
    assert_eq!(scene.markers.len(), 1, "and its station comes along");
}

// ==================== the canon provider (phase 4, tests first)

mod canon_provider_laws {
    use std::collections::{BTreeMap, BTreeSet};

    use atlas_graph_types::covenant::{PlaceId, SourceId, TimePoint, Year};
    use map_canon::{
        Area, Border, CanonStore, EntityId, Feature, LayerKind, Leg, Provenance, Route, Snapshot,
        Witness, World,
    };
    use map_types::style::*;
    use map_types::{
        GazetteerEntry, GazetteerExport, LayerSet, Lod, MapAddressed, MapProvider, RenderQuery,
        RenderSubject, StyleId, TimeSelector, UnitVec,
    };

    use crate::canon_provider::CanonProvider;

    fn ts(y: i32) -> TimePoint {
        TimePoint::year_only(Year::new(y).unwrap())
    }
    fn uv(lat: f64, lon: f64) -> UnitVec {
        UnitVec::from_lat_lon_deg(lat, lon)
    }

    fn style() -> Style {
        let stroke = |r, pattern| Stroke { color: Rgba(r, 0, 0, 255), width: 1.0, pattern };
        Style::new(
            BoundaryStrokes {
                line: stroke(0, StrokePattern::Solid),
                frontier: stroke(60, StrokePattern::Zonal),
                disputed: stroke(120, StrokePattern::Hatched),
                unknown: stroke(180, StrokePattern::Dashed),
                way: stroke(240, StrokePattern::Dashed),
            },
            Paint { fill: Rgba(200, 200, 180, 255) },
            Paint { fill: Rgba(120, 160, 200, 235) },
            AgeRamp {
                newest: Paint { fill: Rgba(150, 110, 80, 200) },
                oldest: Paint { fill: Rgba(225, 214, 180, 200) },
            },
            Some([
                Paint { fill: Rgba(1, 1, 1, 205) },
                Paint { fill: Rgba(2, 2, 2, 205) },
                Paint { fill: Rgba(3, 3, 3, 205) },
                Paint { fill: Rgba(4, 4, 4, 205) },
                Paint { fill: Rgba(5, 5, 5, 205) },
                Paint { fill: Rgba(6, 6, 6, 205) },
                Paint { fill: Rgba(7, 7, 7, 205) },
                Paint { fill: Rgba(8, 8, 8, 205) },
            ]),
            AgeRamp {
                newest: Paint { fill: Rgba(220, 40, 40, 255) },
                oldest: Paint { fill: Rgba(220, 40, 40, 40) },
            },
            super::test_labeling(LabelStyle { color: Rgba(20, 20, 20, 255), halo: Rgba(245, 240, 225, 220), size: 12.0 }),
            MarkerStyle { color: Rgba(0, 0, 0, 255), size: 4.0 },
            DeltaEmphasis {
                before: stroke(90, StrokePattern::Dashed),
                after: stroke(30, StrokePattern::Solid),
                seam: stroke(250, StrokePattern::Solid),
            },
        )
        .unwrap()
    }

    /// A tiny real canon: Assyria in Territory (-1900..-911 exclusive),
    /// a two-leg journey in Journeys (45..47, 47..49), one sea in Water.
    fn fixture() -> CanonStore {
        let mut store = CanonStore::default();
        let square = |lat0: f64, lon0: f64, d: f64| {
            Border(vec![uv(lat0, lon0), uv(lat0, lon0 + d), uv(lat0 + d, lon0 + d), uv(lat0 + d, lon0)])
        };
        // Territory: Assyria
        let b = store.insert_border(square(35.0, 42.0, 3.0));
        let assyria = store.insert_feature(Feature::Area(Area {
            entity: EntityId("assyria".into()),
            name: "Assyria".into(),
            rings: BTreeSet::from([b]),
            holes: BTreeSet::new(),
        }));
        store.set_provenance(assyria, Provenance {
            witness: Witness::Atlas,
            verses: vec!["2KI.15.19".into()],
            note: "t".into(),
        });
        let s1 = store.insert_snapshot(Snapshot { features: BTreeSet::from([assyria]) });
        let s0 = store.insert_snapshot(Snapshot { features: BTreeSet::new() });
        let mut territory = World::default();
        territory.insert(ts(-1900), s1).unwrap();
        territory.insert(ts(-911), s0).unwrap();
        store.set_layer(LayerKind::Territory, territory);

        // Journeys: two legs, 45..47 and 47..49
        let road1 = store.insert_border(Border(vec![uv(36.2, 36.16), uv(37.9, 27.3)]));
        let road2 = store.insert_border(Border(vec![uv(37.9, 27.3), uv(41.89, 12.49)]));
        let way = store.insert_feature(Feature::Way(Route {
            entity: EntityId("test-walk".into()),
            name: "a test walk".into(),
            legs: vec![
                Leg { from: PlaceId::new("antioch".to_string()), to: PlaceId::new("ephesus".to_string()),
                      border: road1, span: (ts(45), ts(47)) },
                Leg { from: PlaceId::new("ephesus".to_string()), to: PlaceId::new("rome".to_string()),
                      border: road2, span: (ts(47), ts(49)) },
            ],
        }));
        store.set_provenance(way, Provenance {
            witness: Witness::Atlas,
            verses: vec!["ACT.19.1".into()],
            note: "t".into(),
        });
        let sj = store.insert_snapshot(Snapshot { features: BTreeSet::from([way]) });
        let sj0 = store.insert_snapshot(Snapshot { features: BTreeSet::new() });
        let mut journeys = World::default();
        journeys.insert(ts(45), sj).unwrap();
        journeys.insert(ts(50), sj0).unwrap();
        store.set_layer(LayerKind::Journeys, journeys);

        // Water: one static sea
        let sea = store.insert_border(square(31.0, 30.0, 4.0));
        let water = store.insert_feature(Feature::Area(Area {
            entity: EntityId("natural-earth:the-sea".into()),
            name: "the sea".into(),
            rings: BTreeSet::from([sea]),
            holes: BTreeSet::new(),
        }));
        store.set_provenance(water, Provenance {
            witness: Witness::NaturalEarth,
            verses: vec![],
            note: "t".into(),
        });
        let sw = store.insert_snapshot(Snapshot { features: BTreeSet::from([water]) });
        let mut w = World::default();
        w.insert(ts(-4004), sw).unwrap();
        store.set_layer(LayerKind::Water, w);
        store
    }

    fn provider() -> (CanonProvider, StyleId) {
        let s = style();
        let sid = s.id();
        let gaz = GazetteerExport {
            atlas_root: atlas_graph_types::covenant::ContentHash(0),
            places: [
                ("antioch", 36.2, 36.16, "Antioch"),
                ("ephesus", 37.9, 27.3, "Ephesus"),
                ("rome", 41.89, 12.49, "Rome"),
            ]
            .into_iter()
            .map(|(id, lat, lon, name)| {
                (PlaceId::new(id.to_string()), GazetteerEntry {
                    canonical_name: name.to_string(),
                    position: uv(lat, lon),
                    aliases: vec![],
                    provenance: None,
                    attestations: vec![],
                })
            })
            .collect(),
        };
        (CanonProvider::new(fixture(), BTreeMap::from([(sid, s)]), Some(gaz)), sid)
    }

    fn world_q(sid: StyleId, y: i32) -> RenderQuery {
        RenderQuery {
            subject: RenderSubject::World,
            time: TimeSelector::At(ts(y)),
            viewport: None,
            lod: Lod::exact(),
            layers: LayerSet::GEOMETRY
                .with(LayerSet::LABELS)
                .with(LayerSet::TOPOGRAPHY)
                .with(LayerSet::JOURNEYS),
            style: sid,
        }
    }

    /// The canon renders through the SAME contract: areas appear only
    /// within their moments, water rides the TOPOGRAPHY bit, sources
    /// carry the witness AND the scripture tag for atlas/authored
    /// truth (bible mode needs no name-matching ever again).
    #[test]
    fn canon_scenes_respect_time_layers_and_witness() {
        let (p, sid) = provider();
        let scene = p.render(&world_q(sid, -1000)).unwrap();
        let assyria = scene
            .regions
            .iter()
            .find(|r| r.sources.contains(&SourceId::new("witness:atlas")))
            .expect("assyria realized");
        assert!(assyria.sources.contains(&SourceId::new("scripture")), "atlas truth is scripture-grounded");
        assert!(
            scene.labels.iter().any(|l| l.text == "Assyria"),
            "areas carry their names"
        );
        assert!(
            scene.regions.iter().any(|r| r.sources.contains(&SourceId::new("witness:natural-earth"))),
            "water rides along"
        );

        // After the era: gone. Before: gone. Water (static) stays.
        let after = p.render(&world_q(sid, -500)).unwrap();
        assert!(!after.labels.iter().any(|l| l.text == "Assyria"));
        assert!(!after.regions.is_empty(), "the sea remains");
        let before = p.render(&world_q(sid, -2000)).unwrap();
        assert!(!before.labels.iter().any(|l| l.text == "Assyria"));

        // Determinism (law 1) holds through the canon.
        let a = p.render(&world_q(sid, -1000)).unwrap();
        let b = p.render(&world_q(sid, -1000)).unwrap();
        assert_eq!(a.canonical_bytes(), b.canonical_bytes());
    }

    /// A lake is never buried by a claim: water areas paint after
    /// every land-claim area regardless of size, so a territory that
    /// overhangs a shoreline is clipped by the sea, never the other
    /// way round. (The fixture's sea is LARGER than Assyria, so a
    /// pure size-sort would paint it first and bury it.)
    #[test]
    fn water_is_never_buried_by_a_claim() {
        let (p, sid) = provider();
        let scene = p.render(&world_q(sid, -1000)).unwrap();
        let idx_of = |witness: &str| {
            scene
                .regions
                .iter()
                .position(|r| r.sources.contains(&SourceId::new(witness)))
                .unwrap_or_else(|| panic!("{witness} region present"))
        };
        let land = idx_of("witness:atlas");
        let water = idx_of("witness:natural-earth");
        assert!(
            water > land,
            "water (idx {water}) must paint after the land claim (idx {land})"
        );
    }

    /// Partial journeys, typed: mid-first-leg the road shows clipped;
    /// stations appear as reached, named from the gazetteer; outside
    /// the span, no way at all.
    #[test]
    fn canon_journeys_walk_in_time() {
        let (p, sid) = provider();
        let way_pts = |y: i32| -> Option<usize> {
            let scene = p.render(&world_q(sid, y)).unwrap();
            scene
                .boundaries
                .iter()
                .find(|b| b.sources.contains(&SourceId::new("witness:atlas")))
                .map(|b| b.pts.len())
        };
        assert_eq!(way_pts(44), None, "not yet departed");
        assert_eq!(way_pts(51), None, "long arrived");
        assert!(way_pts(46).is_some(), "mid-first-leg the road shows");
        let stations = |y: i32| -> usize {
            p.render(&world_q(sid, y)).unwrap().markers.len()
        };
        assert_eq!(stations(46), 1, "only Antioch is behind them");
        assert_eq!(stations(49), 3, "arrived: all three stations");
        let named = p.render(&world_q(sid, 49)).unwrap();
        assert!(named.labels.iter().any(|l| l.text == "Ephesus"), "stations named from the gazetteer");
    }

    /// A pieces render carries ONLY the asked entities — the caller
    /// arranges layers as they choose; everything else stays home.
    #[test]
    fn pieces_filter_to_named_entities() {
        let (p, sid) = provider();
        let q = world_q(sid, -1000);
        let all = p.render(&q).unwrap();
        assert!(all.regions.len() >= 2, "assyria + the sea");
        let only = p
            .render_pieces(&q, &[map_canon::EntityId("assyria".into())].into_iter().collect())
            .unwrap();
        assert_eq!(only.regions.len(), 1, "just assyria");
        assert!(only.labels.iter().any(|l| l.text == "Assyria"));
        assert!(only.regions[0]
            .sources
            .contains(&atlas_graph_types::covenant::SourceId::new("witness:atlas")));
    }

    /// The scrubber lives: subjects at a time list the entities, and
    /// changes_between yields the canon's moment edges as stops.
    #[test]
    fn canon_subjects_and_stops() {
        let (p, _sid) = provider();
        let subs = p.subjects(ts(-1000));
        assert!(subs.iter().any(|s| s.label == "Assyria"));
        let stops = p.changes_between(ts(-4004), ts(100));
        let years: Vec<i32> = stops.iter().map(|e| e.at.year.get()).collect();
        assert!(years.contains(&-1900) && years.contains(&-911) && years.contains(&45));
        assert!(stops.windows(2).all(|w| w[0].at <= w[1].at));
    }
}


// ==================== palette graph coloring (style honesty made true)

mod palette_coloring_laws {
    use std::collections::{BTreeMap, BTreeSet};

    use map_canon::EntityId;

    use crate::canon_provider::color_shared_border_graph;

    fn e(s: &str) -> EntityId {
        EntityId(s.to_string())
    }

    fn graph(edges: &[(&str, &str)]) -> (Vec<EntityId>, BTreeMap<EntityId, BTreeSet<EntityId>>) {
        let mut ids: BTreeSet<EntityId> = BTreeSet::new();
        let mut adj: BTreeMap<EntityId, BTreeSet<EntityId>> = BTreeMap::new();
        for (a, b) in edges {
            ids.insert(e(a));
            ids.insert(e(b));
            adj.entry(e(a)).or_default().insert(e(b));
            adj.entry(e(b)).or_default().insert(e(a));
        }
        (ids.into_iter().collect(), adj)
    }

    /// The law the style comment promises: touching territories never
    /// share a slot.
    #[test]
    fn touching_territories_never_match() {
        let (ids, adj) = graph(&[
            ("asher", "manasseh-west"),
            ("asher", "phoenicia"),
            ("asher", "zebulun"),
            ("manasseh-west", "zebulun"),
            ("manasseh-west", "ephraim"),
            ("zebulun", "issachar"),
        ]);
        let slot = color_shared_border_graph(&ids, &adj);
        for (a, ns) in &adj {
            for b in ns {
                assert_ne!(slot[a], slot[b], "{a:?} and {b:?} touch and match");
            }
        }
    }

    /// More neighbors than slots: the fallback still answers, and
    /// deterministically — same input, same coloring, every time.
    #[test]
    fn oversubscribed_hub_is_deterministic() {
        // a hub touching nine spokes that all touch each other: the
        // spokes exhaust the eight slots, the hub takes the least-worn
        let mut edges = Vec::new();
        let spokes: Vec<String> = (0..9).map(|i| format!("spoke-{i}")).collect();
        for i in 0..spokes.len() {
            edges.push(("hub".to_string(), spokes[i].clone()));
            for j in i + 1..spokes.len() {
                edges.push((spokes[i].clone(), spokes[j].clone()));
            }
        }
        let borrowed: Vec<(&str, &str)> =
            edges.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
        let (ids, adj) = graph(&borrowed);
        let one = color_shared_border_graph(&ids, &adj);
        let two = color_shared_border_graph(&ids, &adj);
        assert_eq!(one, two, "coloring is a function of the graph, nothing else");
        assert!(one.values().all(|&s| s < 8), "every slot is a real slot");
    }

    /// An entity keeps its slot regardless of which pieces render:
    /// the assignment is computed once over the whole graph, so a
    /// subset render cannot recolor the survivors.
    #[test]
    fn color_follows_the_entity() {
        let (ids, adj) = graph(&[("judah", "benjamin"), ("benjamin", "ephraim")]);
        let full = color_shared_border_graph(&ids, &adj);
        // rendering only judah+ephraim later still reads THIS map —
        // the law is that the assignment exists once; spot-check it
        // is total over the ids given
        for id in &ids {
            assert!(full.contains_key(id));
        }
    }
}
