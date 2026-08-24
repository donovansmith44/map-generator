//! Phase-2 laws: adapter FIDELITY (what came out is what went in),
//! honest deltas (epoch differences are narrated), typed exemptions
//! (nothing excluded silently) — proven on fixtures AND on the real
//! vendored source, whose ingested timeline must satisfy every
//! map-types data validator.

use std::collections::BTreeMap;

use atlas_graph_types::chrono::{TimePoint, Year};
use atlas_graph_types::edge::{Ground, Justification};
use atlas_graph_types::id::{ContentHash, SourceId};
use atlas_graph_types::text::{BibleLocus, LocusRange, VerseRef};

use map_types::{
    validate_all, Anchor, ChangeKind, ChronologyExport, GazetteerExport, TimeSelector,
};
use serde_json::json;

use crate::basemaps::*;
use crate::quantize::{clean_ring, QPoint};

fn tp(year: i32) -> TimePoint {
    TimePoint::year_only(Year::new(year).unwrap())
}

fn creation_anchor() -> Anchor {
    let gen11 = BibleLocus::whole(VerseRef { book: 1, chapter: 1, verse: 1 });
    Anchor {
        frame: "biblical (Ussher tradition)".to_string(),
        at: tp(-4004),
        justification: Justification {
            text: Some("In the beginning God created the heaven and the earth.".to_string()),
            grounds: [Ground::Scripture(LocusRange::new(gen11.clone(), gen11).unwrap())].into(),
        },
        provenance: "owner-config:ussher-tradition (pending atlas C2 export)".to_string(),
    }
}

fn config() -> IngestConfig {
    IngestConfig { source: SourceId::new("historical-basemaps"), anchor: Some(creation_anchor()), snap: None }
}

fn empty_exports() -> (ChronologyExport, GazetteerExport) {
    (
        ChronologyExport { atlas_root: ContentHash(0), placements: BTreeMap::new() },
        GazetteerExport { atlas_root: ContentHash(0), places: BTreeMap::new() },
    )
}

/// Build a feature collection from (name, polygons) pairs; a None name
/// makes an anonymous feature.
fn fc(features: &[(Option<&str>, Vec<Vec<(f64, f64)>>)]) -> String {
    let feats: Vec<serde_json::Value> = features
        .iter()
        .map(|(name, polys)| {
            let coords: Vec<Vec<Vec<[f64; 2]>>> = polys
                .iter()
                .map(|outer| vec![outer.iter().map(|&(lon, lat)| [lon, lat]).collect()])
                .collect();
            json!({
                "type": "Feature",
                "properties": { "NAME": name, "BORDERPRECISION": 1 },
                "geometry": { "type": "MultiPolygon", "coordinates": coords }
            })
        })
        .collect();
    json!({ "type": "FeatureCollection", "features": feats }).to_string()
}

fn square(lon0: f64, lat0: f64, lon1: f64, lat1: f64) -> Vec<(f64, f64)> {
    vec![(lon0, lat0), (lon1, lat0), (lon1, lat1), (lon0, lat1), (lon0, lat0)]
}

// ---------------------------------------------------------- fidelity

/// Two squares sharing one full edge, plus an island: the shared edge
/// is stored ONCE, and every source ring reconstructs exactly.
#[test]
fn fidelity_and_arc_sharing_on_fixture() {
    let epoch = EpochSource {
        year: -2000,
        label: "fixture_bc2000".to_string(),
        text: fc(&[
            (Some("Westia"), vec![square(0.0, 0.0, 5.0, 10.0)]),
            (Some("Estia"), vec![square(5.0, 0.0, 10.0, 10.0)]),
            (Some("Isla"), vec![square(20.0, 0.0, 21.0, 1.0)]),
        ]),
    };
    let out = ingest(&config(), &[epoch.clone()]).unwrap();

    // Sharing: Westia-private + shared + Estia-private + Isla = 4 arcs,
    // where naive per-region storage would hold 3 full rings.
    assert_eq!(out.timeline.boundaries.len(), 4);
    assert_eq!(out.timeline.regions.len(), 3);

    // The shared arc is the two-point vertical edge, referenced by both.
    let shared: Vec<_> = out
        .timeline
        .boundaries
        .values()
        .filter(|h| h.versions[0].1.pts.len() == 2)
        .collect();
    assert_eq!(shared.len(), 1, "exactly one two-point shared arc");

    // The fidelity law, ring for ring.
    assert_eq!(fidelity_violations(&out, &epoch).unwrap(), Vec::<String>::new());

    // And the ingested world is lawful under every data validator.
    let (chron, gaz) = empty_exports();
    assert_eq!(validate_all(&out.timeline, &chron, &gaz), vec![]);
}

// ------------------------------------------------------ honest deltas

#[test]
fn epoch_differences_are_narrated() {
    let e1 = EpochSource {
        year: -2000,
        label: "fixture_bc2000".to_string(),
        text: fc(&[
            (Some("Stayland"), vec![square(0.0, 0.0, 5.0, 5.0)]),
            (Some("Growland"), vec![square(30.0, 0.0, 31.0, 1.0)]),
            (Some("Goneland"), vec![square(50.0, 0.0, 51.0, 1.0)]),
        ]),
    };
    let e2 = EpochSource {
        year: -1500,
        label: "fixture_bc1500".to_string(),
        text: fc(&[
            (Some("Stayland"), vec![square(0.0, 0.0, 5.0, 5.0)]),
            (Some("Growland"), vec![square(30.0, 0.0, 32.0, 1.0)]),
            (Some("Newland"), vec![square(60.0, 0.0, 61.0, 1.0)]),
        ]),
    };
    let out = ingest(&config(), &[e1.clone(), e2.clone()]).unwrap();
    let tl = &out.timeline;

    // Unchanged geometry merges into ONE version spanning both epochs —
    // and therefore needs no narration.
    let stay = tl
        .regions
        .values()
        .find(|r| r.label_at(&tp(-1600)) == Some("Stayland"))
        .expect("Stayland present");
    assert_eq!(stay.geom_history.len(), 1);
    assert_eq!(stay.geom_history[0].0.to, None, "open: the source's edge of knowledge");

    // Changes are narrated: a Rise, a Fall, and Shifts, all at -1500.
    let kinds: Vec<&ChangeKind> = tl.events.iter().map(|e| &e.kind).collect();
    assert!(tl.events.iter().all(|e| e.at == tp(-1500)));
    assert!(kinds.iter().any(|k| matches!(k, ChangeKind::Rise { .. })));
    assert!(kinds.iter().any(|k| matches!(k, ChangeKind::Fall { .. })));
    assert!(kinds.iter().any(|k| matches!(k, ChangeKind::Shift { .. })));

    // Fidelity holds at BOTH epochs, and the whole result is lawful.
    assert_eq!(fidelity_violations(&out, &e1).unwrap(), Vec::<String>::new());
    assert_eq!(fidelity_violations(&out, &e2).unwrap(), Vec::<String>::new());
    let (chron, gaz) = empty_exports();
    assert_eq!(validate_all(tl, &chron, &gaz), vec![]);

    // The scrubber's view: exactly one stop between the epochs.
    let stops: Vec<_> = tl.events.iter().filter(|e| e.at > tp(-2000)).collect();
    assert!(!stops.is_empty());
}

// ----------------------------------------------- typed exemptions

#[test]
fn pre_anchor_epochs_are_excluded_with_exemption() {
    let deep_time = EpochSource {
        year: -10000,
        label: "fixture_bc10000".to_string(),
        text: fc(&[(Some("Prehistoria"), vec![square(0.0, 0.0, 1.0, 1.0)])]),
    };
    let in_frame = EpochSource {
        year: -2000,
        label: "fixture_bc2000".to_string(),
        text: fc(&[(Some("Historia"), vec![square(0.0, 0.0, 1.0, 1.0)])]),
    };
    let out = ingest(&config(), &[deep_time, in_frame]).unwrap();
    assert!(out
        .exemptions
        .iter()
        .any(|e| matches!(e, Exemption::PreAnchorEpoch { year: -10000, .. })));
    // Nothing from the excluded epoch leaked into the timeline.
    for hist in out.timeline.boundaries.values() {
        for (iv, _) in &hist.versions {
            assert!(iv.from >= tp(-4004));
        }
    }
    // Without an anchor, the same epochs all ingest — the anchor is a
    // parameter, not a hardcode.
    let unanchored = IngestConfig { source: SourceId::new("historical-basemaps"), anchor: None, snap: None };
    let out2 = ingest(
        &unanchored,
        &[EpochSource {
            year: -10000,
            label: "fixture_bc10000".to_string(),
            text: fc(&[(Some("Prehistoria"), vec![square(0.0, 0.0, 1.0, 1.0)])]),
        }],
    )
    .unwrap();
    assert!(out2.exemptions.is_empty());
    assert_eq!(out2.timeline.regions.len(), 1);
}

#[test]
fn unnamed_features_are_counted_not_silently_dropped() {
    let epoch = EpochSource {
        year: -2000,
        label: "fixture_bc2000".to_string(),
        text: fc(&[
            (Some("Namedia"), vec![square(0.0, 0.0, 1.0, 1.0)]),
            (None, vec![square(10.0, 0.0, 11.0, 1.0)]),
            (None, vec![square(20.0, 0.0, 21.0, 1.0)]),
        ]),
    };
    let out = ingest(&config(), &[epoch]).unwrap();
    assert!(out
        .exemptions
        .iter()
        .any(|e| matches!(e, Exemption::UnnamedFeatures { year: -2000, count: 2 })));
    assert_eq!(out.timeline.regions.len(), 1);
}

// --------------------------------------------------- the waters

/// Water bodies ingest as first-class Water regions — lawful, labeled,
/// and mergeable with the rest of the world.
#[test]
fn waters_are_explorable_regions()  {
    use crate::surveys::{merge_timelines, scripture_timeline, stand_in_gazetteer};
    let text = fc(&[
        (Some("Test Sea"), vec![square(10.0, 10.0, 12.0, 12.0)]),
        (None, vec![square(20.0, 10.0, 25.0, 15.0)]),
    ]);
    let water = crate::hydro::ingest_water(
        &SourceId::new("natural-earth"),
        tp(-4004),
        &[crate::hydro::WaterSource { label_for_unnamed: "the sea", text, skip_largest_feature: false }],
    )
    .unwrap();
    assert_eq!(water.regions.len(), 2);
    assert!(water
        .regions
        .values()
        .all(|r| r.class == map_types::RegionClass::Water));
    let labels: Vec<&str> =
        water.regions.values().map(|r| r.label_at(&tp(-1000)).unwrap()).collect();
    assert!(labels.contains(&"Test Sea") && labels.contains(&"the sea"));
    let (chron, gaz) = empty_exports();
    assert_eq!(validate_all(&water, &chron, &gaz), vec![]);

    // And the whole world composes: land + Scripture + water, lawful.
    let land = ingest(&config(), &[EpochSource {
        year: -2000,
        label: "fx".to_string(),
        text: fc(&[(Some("Terra"), vec![square(0.0, 0.0, 5.0, 5.0)])]),
    }])
    .unwrap();
    let merged = merge_timelines(land.timeline, water).unwrap();
    let merged = merge_timelines(merged, scripture_timeline()).unwrap();
    assert_eq!(validate_all(&merged, &chron, &stand_in_gazetteer()), vec![]);
}

// ------------------------------------------- topology closure (snap)

/// Two neighbors whose shared border misses by 0.004 degrees: exact
/// matching sees two private rings; a disclosed 0.02-degree snap makes
/// them MEET — one shared arc, no sliver gap. Fidelity still holds,
/// against the source as snapped.
#[test]
fn snap_closes_near_miss_borders() {
    let epoch = |label: &str| EpochSource {
        year: -2000,
        label: label.to_string(),
        text: fc(&[
            (Some("Westia"), vec![square(0.0, 0.0, 5.0, 10.0)]),
            (Some("Estia"), vec![square(5.004, 0.0, 10.0, 10.0)]),
        ]),
    };
    let exact = ingest(&config(), &[epoch("fx")]).unwrap();
    assert_eq!(exact.timeline.boundaries.len(), 2, "near-miss stays private when exact");

    let snapped_config = IngestConfig { snap: Some(0.02), ..config() };
    let e = epoch("fx");
    let snapped = ingest(&snapped_config, &[e.clone()]).unwrap();
    assert_eq!(snapped.timeline.boundaries.len(), 3, "snapped neighbors share one arc");
    assert_eq!(fidelity_violations(&snapped, &e).unwrap(), Vec::<String>::new());
    let (chron, gaz) = empty_exports();
    assert_eq!(validate_all(&snapped.timeline, &chron, &gaz), vec![]);
}

// ------------------------------------------------- helpers and units

#[test]
fn epoch_labels_parse() {
    assert_eq!(epoch_year_from_label("world_bc2000"), Some(-2000));
    assert_eq!(epoch_year_from_label("world_100"), Some(100));
    assert_eq!(epoch_year_from_label("world_bc1"), Some(-1));
    assert_eq!(epoch_year_from_label("nonsense"), None);
}

#[test]
fn ring_cleaning_normalizes() {
    // Closing repeat dropped, consecutive duplicates collapsed.
    let cleaned =
        clean_ring(&[(0.0, 0.0), (1.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 0.0)], None).unwrap();
    assert_eq!(cleaned.len(), 3);
    assert_eq!(cleaned[0], QPoint::from_lon_lat(0.0, 0.0));
    // Degenerate rings refuse to exist.
    assert_eq!(clean_ring(&[(0.0, 0.0), (1.0, 0.0), (0.0, 0.0)], None), None);
}

// ------------------------------------- the Scripture survey source

/// The first Bible-driven borders: the NUM 34 circuit is a lawful
/// timeline on its own AND merged into the imported world — and law 12
/// holds: survey waypoints resolve against the (stand-in) gazetteer,
/// and no import overrides the survey.
#[test]
fn promised_land_survey_is_lawful_alone_and_merged() {
    use crate::surveys::*;
    use map_types::BoundarySource;

    let survey_tl = promised_land_timeline();
    let gaz = stand_in_gazetteer();
    let (chron, _) = empty_exports();
    assert_eq!(map_types::validate_all(&survey_tl, &chron, &gaz), vec![]);

    // The boundary really is Survey-sourced, closed, and justified by
    // the verses themselves.
    let (_, hist) = survey_tl.boundaries.iter().next().unwrap();
    let b = &hist.versions[0].1;
    assert!(matches!(b.source, BoundarySource::Survey(_)));
    assert_eq!(b.pts.first(), b.pts.last(), "the circuit closes");
    assert!(!b.justification.grounds.is_empty(), "the text is the ground");

    // Merged with the imported world, everything stays lawful.
    let e1 = EpochSource {
        year: -2000,
        label: "fixture_bc2000".to_string(),
        text: fc(&[(Some("Elsewhere"), vec![square(50.0, 40.0, 51.0, 41.0)])]),
    };
    let world = ingest(&config(), &[e1]).unwrap().timeline;
    let merged = merge_timelines(world, survey_tl.clone()).unwrap();
    assert_eq!(map_types::validate_all(&merged, &chron, &gaz), vec![]);
    assert_eq!(merged.regions.len(), 2);

    // Merging the same source twice is a loud duplicate, never a
    // silent preference.
    assert!(matches!(
        merge_timelines(merged, survey_tl),
        Err(MergeError::DuplicateBoundary(_))
    ));

    // The full Scripture set — the promise plus all twelve tribal
    // allotments (Levi has none, JOS 13:33) — is lawful as a whole:
    // every waypoint of every survey resolves, every rise narrated.
    let all = scripture_timeline();
    assert_eq!(all.regions.len(), 14, "promise + 13 allotment territories");
    assert_eq!(map_types::validate_all(&all, &chron, &gaz), vec![]);

    // The honesty grades render differently by construction: walked
    // borders are Lines, city-derived hulls are Unknown.
    use map_types::EdgeCharacter;
    let characters: Vec<_> = all
        .boundaries
        .values()
        .map(|h| h.versions[0].1.character.clone())
        .collect();
    assert!(characters.iter().any(|c| matches!(c, EdgeCharacter::Line)));
    assert!(characters.iter().any(|c| matches!(c, EdgeCharacter::Unknown)));
}

// --------------------------------------------- the real source, whole

/// The milestone proof: ingest every vendored epoch of the real source
/// under the biblical anchor; the result is lawful under every data
/// validator, and fidelity holds ring-for-ring at every epoch.
#[test]
fn real_source_ingests_lawfully() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/historical-basemaps");
    let mut epochs = Vec::new();
    for entry in std::fs::read_dir(&dir).expect("vendored data present") {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("geojson") {
            continue;
        }
        let label = path.file_stem().unwrap().to_string_lossy().to_string();
        let year = epoch_year_from_label(&label)
            .unwrap_or_else(|| panic!("unparseable epoch label {label}"));
        epochs.push(EpochSource {
            year,
            label,
            text: std::fs::read_to_string(&path).unwrap(),
        });
    }
    assert_eq!(epochs.len(), 12, "the vendored set");

    let out = ingest(&config(), &epochs).unwrap();
    let tl = &out.timeline;

    // How much sharing the source's vertex agreement actually allows —
    // reported, not assumed (arcs share only where neighbors agree
    // exactly; slivers stay private and that is the honest answer).
    let mut arc_users: BTreeMap<map_types::BoundaryId, std::collections::BTreeSet<_>> =
        BTreeMap::new();
    for (rid, hist) in &tl.regions {
        for (_, geom) in &hist.geom_history {
            for part in &geom.parts {
                for (b, _) in part.cycle.iter().chain(part.holes.iter().flatten()) {
                    arc_users.entry(*b).or_default().insert(*rid);
                }
            }
        }
    }
    let shared = arc_users.values().filter(|s| s.len() > 1).count();
    eprintln!(
        "real ingest: {} regions, {} boundaries ({} shared by 2+ regions), {} events, {} exemption records",
        tl.regions.len(),
        tl.boundaries.len(),
        shared,
        tl.events.len(),
        out.exemptions.len()
    );

    // Substance: the world is actually in there.
    assert!(tl.regions.len() > 100, "{} regions", tl.regions.len());
    assert!(tl.boundaries.len() > 500, "{} boundaries", tl.boundaries.len());
    assert!(!tl.events.is_empty());
    assert!(tl.anchor.is_some());

    // Under the biblical anchor, all twelve epochs are in-frame: the
    // atlas demo pipeline vendored nothing before 4000 BC. Unnamed
    // land is exempted, disclosed, at every epoch.
    assert!(!out.exemptions.iter().any(|e| matches!(e, Exemption::PreAnchorEpoch { .. })));
    assert!(out.exemptions.iter().any(|e| matches!(e, Exemption::UnnamedFeatures { .. })));

    // Every data law holds over the whole ingested timeline.
    let (chron, gaz) = empty_exports();
    let violations = validate_all(tl, &chron, &gaz);
    assert!(
        violations.is_empty(),
        "{} violations, first: {:?}",
        violations.len(),
        violations.first()
    );

    // Fidelity at every single epoch.
    for e in &epochs {
        let v = fidelity_violations(&out, e).unwrap();
        assert!(v.is_empty(), "{}: {} violations, first: {:?}", e.label, v.len(), v.first());
    }

    // The scrubber has stops at every inter-epoch boundary.
    let mut stop_years: Vec<TimePoint> = tl.events.iter().map(|e| e.at).collect();
    stop_years.sort();
    stop_years.dedup();
    assert_eq!(stop_years.len(), 11, "eleven transitions between twelve epochs");

    // A snapshot-shaped question answers coherently: some region is
    // present at Abraham's era under the traditional frame.
    let t = TimeSelector::At(tp(-1900));
    if let TimeSelector::At(at) = t {
        let present = tl.regions.values().filter(|r| r.geom_at(&at).is_some()).count();
        assert!(present > 10, "{present} regions present at 1900 BC");
    }
}
