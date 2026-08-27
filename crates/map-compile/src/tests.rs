//! Phase-2 laws, written first: the atlas API's payloads parse into
//! typed rows (shape drift fails loud, never vendors garbage), and the
//! vendor writer is deterministic — same payloads, same bytes, same pin.

use crate::vendor::*;

fn fx(name: &str) -> String {
    std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures").join(name),
    )
    .expect("fixture exists")
}

// ------------------------------------------------------- typed parses

#[test]
fn polities_parse_typed_and_sane() {
    let rows = parse_polities(&fx("polities.json")).expect("live-captured fixture parses");
    assert!(rows.len() >= 10, "a real polity book, got {}", rows.len());
    let assyria = rows.iter().find(|p| p.id == "assyria").expect("assyria present");
    assert_eq!(assyria.name, "Assyria");
    assert_eq!((assyria.from_year, assyria.to_year), (-1900, -912));
    assert!(!assyria.rings.is_empty() && assyria.rings[0].len() >= 4);
    for p in &rows {
        assert!(p.from_year <= p.to_year, "{}: era runs forward", p.id);
        assert!(p.rings.iter().all(|r| r.len() >= 3), "{}: rings are areas", p.id);
    }
}

#[test]
fn narratives_parse_with_ordered_legs() {
    let rows = parse_narratives(&fx("narratives.json")).expect("parses");
    let abraham = rows.iter().find(|n| n.id == "abraham-migration").expect("present");
    assert_eq!(abraham.name, "Abraham's Migration");
    assert_eq!(abraham.color, "#D97706");
    assert_eq!(abraham.legs.len(), 6);
    assert_eq!(abraham.legs[1], "ab_haran");
    assert!(rows.len() >= 5, "the Bible walks in many narratives");
}

#[test]
fn events_parse_with_time_places_verses() {
    let e = parse_event(&fx("event-ab_haran.json")).expect("parses");
    assert_eq!(e.id, "ab_haran");
    assert_eq!(e.when, Some((-2092, -2091)));
    assert_eq!(e.places, vec!["haran".to_string()]);
    assert!(e.verses.iter().any(|v| v == "GEN.12.1"), "verses flatten: {:?}", e.verses);
}

#[test]
fn eras_landmarks_landmask_parse() {
    let eras = parse_eras(&fx("eras.json")).expect("parses");
    assert!(eras.iter().any(|e| e.id == "patriarchs" && e.from_year == -2166));
    let lm = parse_landmarks(&fx("landmarks.json")).expect("parses");
    assert!(lm.iter().any(|l| l.name == "Sea of Galilee" && l.kind == "water"));
    let mask = parse_land_mask(&fx("land-mask.json")).expect("parses");
    assert!(!mask.rings.is_empty() && mask.rings[0].len() >= 10);
}

/// Shape drift fails loud: a payload missing required fields is an
/// error naming the field, never a silently-empty vendor file.
#[test]
fn shape_drift_is_a_named_error() {
    let err = parse_polities(r#"{"polities":[{"id":"x"}]}"#).unwrap_err();
    assert!(err.contains("name"), "the missing field is named: {err}");
    assert!(parse_event(r#"{"nonsense":true}"#).is_err());
}

// ------------------------------------------- deterministic vendoring

#[test]
fn vendor_writes_are_deterministic_and_pinned() {
    let payloads = vec![
        ("polities.json".to_string(), fx("polities.json").into_bytes()),
        ("narratives.json".to_string(), fx("narratives.json").into_bytes()),
    ];
    let dir1 = std::env::temp_dir().join("canon-vendor-test-1");
    let dir2 = std::env::temp_dir().join("canon-vendor-test-2");
    let _ = std::fs::remove_dir_all(&dir1);
    let _ = std::fs::remove_dir_all(&dir2);
    let pin1 = write_vendor(&dir1, &payloads).expect("writes");
    let pin2 = write_vendor(&dir2, &payloads).expect("writes");
    assert_eq!(pin1, pin2, "same payloads, same pin");
    let m1 = std::fs::read(dir1.join("manifest.json")).unwrap();
    let m2 = std::fs::read(dir2.join("manifest.json")).unwrap();
    assert_eq!(m1, m2, "same payloads, byte-identical manifest");

    // A changed payload moves the pin — staleness is visible (C6 spirit).
    let mut changed = payloads.clone();
    changed[0].1.push(b' ');
    let dir3 = std::env::temp_dir().join("canon-vendor-test-3");
    let _ = std::fs::remove_dir_all(&dir3);
    let pin3 = write_vendor(&dir3, &changed).expect("writes");
    assert_ne!(pin1, pin3, "a changed world changes the pin");
}

// ---------------------------------------- witnesses become the canon

mod compile_laws {
    use crate::compile::*;
    use crate::vendor::{EventRow, NarrativeRow, PolityRow};
    use atlas_graph_types::covenant::{TimePoint, Year};
    use map_canon::{Feature, LayerKind};

    fn ts(y: i32) -> TimePoint {
        TimePoint::year_only(Year::new(y).unwrap())
    }

    fn ring(lat0: f64, lon0: f64, d: f64) -> Vec<(f64, f64)> {
        vec![(lat0, lon0), (lat0, lon0 + d), (lat0 + d, lon0 + d), (lat0 + d, lon0)]
    }

    /// Polity eras become Territory moments: the world changes exactly
    /// at era boundaries, and after the last era the ground is clear.
    #[test]
    fn polity_eras_become_territory_moments() {
        let rows = vec![
            PolityRow {
                id: "assyria".into(), name: "Assyria".into(),
                from_year: -1900, to_year: -912,
                rings: vec![ring(35.0, 42.0, 3.0)],
                color_key: Some(1), transition_verses: vec![], fall_verses: vec![],
            },
            PolityRow {
                id: "neo-assyria".into(), name: "Neo-Assyrian Empire".into(),
                from_year: -911, to_year: -609,
                rings: vec![ring(35.0, 42.0, 5.0)],
                color_key: Some(1), transition_verses: vec!["2KI.15.19".into()], fall_verses: vec![],
            },
        ];
        let mut store = map_canon::CanonStore::default();
        compile_polities(&mut store, &rows).expect("compiles");
        let world = &store.layers()[&LayerKind::Territory];
        let moments: Vec<_> = world.moments().keys().copied().collect();
        assert_eq!(moments, vec![ts(-1900), ts(-911), ts(-608)], "era edges, then clear ground");
        let at = |y: i32| {
            let sid = world.state_at(&ts(y)).unwrap();
            store.snapshots()[&sid].features.len()
        };
        assert_eq!(at(-1000), 1);
        assert_eq!(at(-700), 1);
        assert_eq!(at(-500), 0, "after the fall the layer is empty");
        assert_eq!(store.validate(), vec![], "lawful, including no-overlap");
    }

    /// A narrative's dated leg events become a Route whose legs span the
    /// gaps between events, walked place to place.
    #[test]
    fn narratives_become_routes_with_dated_legs() {
        let narrative = NarrativeRow {
            id: "abraham-migration".into(), name: "Abraham's Migration".into(),
            color: "#D97706".into(),
            legs: vec!["ab_ur".into(), "ab_haran".into(), "ab_shechem".into()],
        };
        let events = vec![
            EventRow { id: "ab_ur".into(), label: "Ur".into(), when: Some((-2095, -2093)),
                       places: vec!["ur-1".into()], verses: vec!["GEN.11.28".into()] },
            EventRow { id: "ab_haran".into(), label: "Haran".into(), when: Some((-2092, -2091)),
                       places: vec!["haran".into()], verses: vec!["GEN.12.1".into()] },
            EventRow { id: "ab_shechem".into(), label: "Shechem".into(), when: Some((-2090, -2090)),
                       places: vec!["shechem".into()], verses: vec!["GEN.12.6".into()] },
        ];
        let places: std::collections::BTreeMap<String, (f64, f64)> = [
            ("ur-1".to_string(), (30.96, 46.10)),
            ("haran".to_string(), (36.87, 39.03)),
            ("shechem".to_string(), (32.21, 35.28)),
        ]
        .into_iter()
        .collect();
        let mut store = map_canon::CanonStore::default();
        let report = compile_narratives(&mut store, &[narrative], &events, &places)
            .expect("compiles");
        assert_eq!(report.routes, 1);
        let world = &store.layers()[&LayerKind::Journeys];
        assert!(!world.moments().is_empty());
        let route = store
            .features()
            .values()
            .find_map(|f| match f {
                Feature::Way(r) => Some(r.clone()),
                _ => None,
            })
            .expect("a way was compiled");
        assert_eq!(route.entity.0, "abraham-migration");
        assert_eq!(route.legs.len(), 2, "three stations, two walks");
        assert_eq!(route.legs[0].span, (ts(-2093), ts(-2092)), "depart when Ur ends, arrive when Haran begins");
        assert_eq!(route.legs[1].span, (ts(-2091), ts(-2090)));
        assert_eq!(store.validate(), vec![]);
    }

    /// A narrative leg whose place the gazetteer cannot resolve is a
    /// NAMED compile error — never a silently skipped station.
    #[test]
    fn unresolvable_places_fail_loud() {
        let narrative = NarrativeRow {
            id: "n".into(), name: "n".into(), color: "#fff".into(),
            legs: vec!["e1".into(), "e2".into()],
        };
        let events = vec![
            EventRow { id: "e1".into(), label: "a".into(), when: Some((-10, -10)),
                       places: vec!["known".into()], verses: vec![] },
            EventRow { id: "e2".into(), label: "b".into(), when: Some((-9, -9)),
                       places: vec!["ghost-town".into()], verses: vec![] },
        ];
        let places: std::collections::BTreeMap<String, (f64, f64)> =
            [("known".to_string(), (30.0, 30.0))].into_iter().collect();
        let mut store = map_canon::CanonStore::default();
        let err = compile_narratives(&mut store, &[narrative], &events, &places).unwrap_err();
        assert!(err.contains("ghost-town"), "the missing place is named: {err}");
    }
}

// ------------------------------ the old model crosses the bridge

mod bridge_laws {
    use crate::timeline_bridge::*;
    use atlas_graph_types::covenant::{Justification, SourceId, TimePoint, Year};
    use map_canon::{Feature, LayerKind, Witness};
    use map_types::{
        Boundary, BoundaryHistory, BoundaryId, BoundarySource, EdgeCharacter, Interval,
        Orientation, RegionClass, RegionGeom, RegionHistory, RegionPart, UnitVec, WorldTimeline,
    };

    fn ts(y: i32) -> TimePoint {
        TimePoint::year_only(Year::new(y).unwrap())
    }
    fn uv(lat: f64, lon: f64) -> UnitVec {
        UnitVec::from_lat_lon_deg(lat, lon)
    }

    fn tl_with_region(from: i32, to: Option<i32>) -> WorldTimeline {
        let mut tl = WorldTimeline::default();
        let bid = BoundaryId(atlas_graph_types::covenant::ContentHash(7));
        let iv = Interval { from: ts(from), to: to.map(ts) };
        tl.boundaries.insert(bid, BoundaryHistory {
            versions: vec![(iv, Boundary {
                pts: vec![uv(10.0, 10.0), uv(10.0, 15.0), uv(15.0, 15.0), uv(15.0, 10.0), uv(10.0, 10.0)],
                character: EdgeCharacter::Unknown,
                source: BoundarySource::Imported { source: SourceId::new("historical-basemaps") },
                justification: Justification::default(),
                provenance: "t".to_string(),
            })],
        });
        tl.regions.insert(map_types::RegionId(atlas_graph_types::covenant::ContentHash(8)), RegionHistory {
            class: RegionClass::Land,
            label_history: vec![(iv, "Westia".to_string())],
            geom_history: vec![(iv, RegionGeom {
                parts: vec![RegionPart { cycle: vec![(bid, Orientation::Forward)], holes: vec![] }],
            })],
        });
        tl
    }

    /// A closed-interval region appears at its from and is gone at its
    /// to (the old model's `to` was already exclusive); entities carry
    /// the witness prefix; labels become names.
    #[test]
    fn old_regions_become_layer_areas_with_moments() {
        let tl = tl_with_region(-2000, Some(-1500));
        let mut store = map_canon::CanonStore::default();
        bridge_timeline_regions(
            &mut store, &tl, LayerKind::Background, Witness::Basemap, "basemap",
        )
        .expect("bridges");
        let world = &store.layers()[&LayerKind::Background];
        let moments: Vec<_> = world.moments().keys().copied().collect();
        assert_eq!(moments, vec![ts(-2000), ts(-1500)]);
        let at = |y: i32| store.snapshots()[&world.state_at(&ts(y)).unwrap()].features.len();
        assert_eq!(at(-1800), 1);
        assert_eq!(at(-1400), 0, "the old exclusive `to` empties the layer");
        let area = store.features().values().find_map(|f| match f {
            Feature::Area(a) => Some(a.clone()),
            _ => None,
        }).expect("an area crossed the bridge");
        assert_eq!(area.entity.0, "basemap:westia");
        assert_eq!(area.name, "Westia");
        assert_eq!(store.validate(), vec![]);
    }
}

// ------------------------------ reconciliation: no silent precedence

mod reconcile_laws {
    use crate::reconcile::*;

    fn atlas_narratives() -> Vec<String> {
        vec!["exodus".to_string(), "paul-first-journey".to_string()]
    }

    /// Every authored route must be reconciled by NAME — a route the
    /// file does not mention fails the compile; superseded routes are
    /// dropped; kept routes stay.
    #[test]
    fn authored_routes_require_reconciliation_rows() {
        let json = r#"{"routes": [
            {"authored": "R-EXODUS", "superseded_by": "exodus"},
            {"authored": "R-SPIES", "keep": true}
        ]}"#;
        let rec = parse_reconcile(json).expect("parses");
        let verdicts = reconcile_routes(
            &rec,
            &["R-EXODUS".to_string(), "R-SPIES".to_string()],
            &atlas_narratives(),
        )
        .expect("all rows present");
        assert_eq!(verdicts.dropped, vec!["R-EXODUS".to_string()]);
        assert_eq!(verdicts.kept, vec!["R-SPIES".to_string()]);

        let err = reconcile_routes(
            &rec,
            &["R-EXODUS".to_string(), "R-SPIES".to_string(), "R-JONAH".to_string()],
            &atlas_narratives(),
        )
        .unwrap_err();
        assert!(err.contains("R-JONAH"), "the unreconciled route is named: {err}");
    }

    /// A supersession must point at a REAL atlas narrative — a typo'd
    /// id is an error, not a silent drop.
    #[test]
    fn supersessions_must_resolve() {
        let json = r#"{"routes": [{"authored": "R-EXODUS", "superseded_by": "exodsu"}]}"#;
        let rec = parse_reconcile(json).expect("parses");
        let err =
            reconcile_routes(&rec, &["R-EXODUS".to_string()], &atlas_narratives()).unwrap_err();
        assert!(err.contains("exodsu"), "the ghost narrative is named: {err}");
    }
}

mod waiver_laws {
    use crate::reconcile::*;

    /// A REAL territorial conflict can be acknowledged (never silently
    /// dropped): a waiver row names the pair and the reason, and only
    /// listed pairs are downgraded to warnings.
    #[test]
    fn territory_waivers_parse_and_match() {
        let json = r#"{"territory_conflicts": [
            {"a": "babylon", "b": "sumer", "note": "atlas era ruling requested"}
        ]}"#;
        let rec = parse_reconcile(json).expect("parses");
        assert!(is_waived(&rec, "babylon", "sumer"));
        assert!(is_waived(&rec, "sumer", "babylon"), "order does not matter");
        assert!(!is_waived(&rec, "babylon", "elam"));
    }
}
