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
    use crate::vendor::{EventRow, NarrativeRow};
    use atlas_graph_types::covenant::{TimePoint, Year};
    use map_canon::{Feature, LayerKind};

    fn ts(y: i32) -> TimePoint {
        TimePoint::year_only(Year::new(y).unwrap())
    }

    fn ring(lat0: f64, lon0: f64, d: f64) -> Vec<(f64, f64)> {
        vec![(lat0, lon0), (lat0, lon0 + d), (lat0 + d, lon0 + d), (lat0 + d, lon0)]
    }

    /// Polity eras become Territory moments THROUGH THE ALGEBRA now:
    /// PresenceBook derives the eras, the overlay lays each window,
    /// and the world changes exactly at the standing edges — clear
    /// ground after the last era.
    #[test]
    fn polity_eras_become_territory_moments() {
        use map_canon::PresenceBook;
        let mut book = PresenceBook::default();
        book.declare("assyria@-1900", ts(-1900), Some(ts(-911))).unwrap();
        book.declare("neo-assyria@-911", ts(-911), Some(ts(-608))).unwrap();
        let mut store = map_canon::CanonStore::default();
        let mk = |store: &mut map_canon::CanonStore, name: &str, d: f64| {
            let ring_pts: Vec<map_types::UnitVec> = ring(35.0, 42.0, d)
                .into_iter()
                .map(|(lat, lon)| map_types::UnitVec::from_lat_lon_deg(lat, lon))
                .collect();
            let bid = store.insert_border(map_canon::Border(ring_pts));
            store.insert_feature(Feature::Area(map_canon::Area {
                entity: map_canon::EntityId("assyria".into()),
                name: name.into(),
                rings: [bid].into_iter().collect(),
                holes: Default::default(),
                tenure: map_canon::Tenure::Held,
            }))
        };
        let old = mk(&mut store, "Assyria", 3.0);
        let neo = mk(&mut store, "Neo-Assyrian Empire", 5.0);
        for era in book.eras(ts(-1900)) {
            let mut fids = std::collections::BTreeSet::new();
            if !era.absent.contains("assyria@-1900") {
                fids.insert(old);
            }
            if !era.absent.contains("neo-assyria@-911") {
                fids.insert(neo);
            }
            crate::partition_bridge::overlay_features_for_law_span(
                &mut store,
                LayerKind::Territory,
                &fids,
                era.from,
                era.until,
            )
            .unwrap();
        }
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

    #[test]
    fn unsupported_abraham_stand_in_is_refused() {
        let narrative = NarrativeRow {
            id: "abraham-migration".into(),
            name: "Abraham's Migration".into(),
            color: "#D97706".into(),
            legs: vec!["ab_ur".into(), "ab_haran".into(), "ab_shechem".into()],
        };
        let events = vec![
            EventRow {
                id: "ab_ur".into(),
                label: "Ur".into(),
                when: Some((-2095, -2093)),
                places: vec!["ur-1".into()],
                verses: vec!["GEN.11.28".into()],
            },
            EventRow {
                id: "ab_haran".into(),
                label: "Haran".into(),
                when: Some((-2092, -2091)),
                places: vec!["haran".into()],
                verses: vec!["GEN.12.1".into()],
            },
            EventRow {
                id: "ab_shechem".into(),
                label: "Shechem".into(),
                when: Some((-2090, -2090)),
                places: vec!["shechem".into()],
                verses: vec!["GEN.12.6".into()],
            },
        ];
        let places: std::collections::BTreeMap<String, (f64, f64)> = [
            ("ur-1".to_string(), (30.96, 46.10)),
            ("haran".to_string(), (36.87, 39.03)),
            ("shechem".to_string(), (32.21, 35.28)),
        ]
        .into_iter()
        .collect();
        let points: Vec<_> = ["ur-1", "haran", "shechem"]
            .iter()
            .map(|id| {
                let (lat, lon) = places[*id];
                map_types::UnitVec::from_lat_lon_deg(lat, lon)
            })
            .collect();
        let expected = crate::exclusion::check_points(&points)
            .expect_err("the unsupported historical stand-in has excluded lineage");
        let mut store = map_canon::CanonStore::default();
        let actual = compile_narratives(&mut store, &[narrative], &events, &places)
            .expect_err("the unsupported route is refused");
        assert_eq!(
            actual,
            format!("excluded output: {expected:?}"),
            "the narrative compiler returns the complete refusal from the sole geometric owner"
        );
    }

    #[test]
    fn permitted_source_lines_preserve_narrative_leg_intervals() {
        let narrative = NarrativeRow {
            id: "synthetic-native-source".into(),
            name: "Source interval law".into(),
            color: "#D97706".into(),
            legs: vec!["source-0".into(), "source-1".into(), "source-2".into()],
        };
        let events = vec![
            EventRow {
                id: "source-0".into(),
                label: "Source 0".into(),
                when: Some((-2095, -2093)),
                places: vec!["source-0".into()],
                verses: vec![],
            },
            EventRow {
                id: "source-1".into(),
                label: "Source 1".into(),
                when: Some((-2092, -2091)),
                places: vec!["source-1".into()],
                verses: vec![],
            },
            EventRow {
                id: "source-2".into(),
                label: "Source 2".into(),
                when: Some((-2090, -2090)),
                places: vec!["source-2".into()],
                verses: vec![],
            },
        ];
        let source: serde_json::Value = serde_json::from_str(include_str!(
            "../../../data/natural-earth/ne_10m_rivers_lake_centerlines.geojson"
        ))
        .unwrap();
        let line = crate::exclusion::geojson_lines(&source)
            .into_iter()
            .find(|line| line.0.len() >= 3)
            .unwrap();
        let places = line
            .0
            .iter()
            .take(3)
            .enumerate()
            .map(|(index, point)| (format!("source-{index}"), (point.y, point.x)))
            .collect();
        let mut store = map_canon::CanonStore::default();
        let report =
            compile_narratives(&mut store, &[narrative], &events, &places).expect("compiles");
        assert_eq!(report.routes, 1, "one input route produces one route");
        let world = &store.layers()[&LayerKind::Journeys];
        assert!(
            !world.moments().is_empty(),
            "dated legs produce exploration moments"
        );
        let route = store
            .features()
            .values()
            .find_map(|f| match f {
                Feature::Way(r) => Some(r.clone()),
                _ => None,
            })
            .expect("a way was compiled");
        assert_eq!(
            route.entity.0, "synthetic-native-source",
            "the synthetic source identity is preserved"
        );
        assert_eq!(route.legs.len(), 2, "three stations, two walks");
        assert_eq!(
            route.legs[0].span,
            (ts(-2093), ts(-2092)),
            "depart when the first interval ends, arrive when the second starts"
        );
        assert_eq!(
            route.legs[1].span,
            (ts(-2091), ts(-2090)),
            "the second leg preserves its departure and arrival intervals"
        );
        assert_eq!(
            store.validate(),
            vec![],
            "the permitted route and dated layers are lawful"
        );
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

#[test]
fn physical_partition_face_census() {
    let (regions, polylines) =
        crate::partition_bridge::gather_witnesses(&[]).expect("retained witnesses gather");
    let partition = map_partition::build(
        &regions,
        &polylines,
        &map_partition::PartitionConfig::default(),
    )
    .expect("retained physical witnesses form a partition");
    assert!(
        partition.area_residual() < 1e-10,
        "physical faces cover the sphere"
    );
    let cities = crate::partition_bridge::load_settlements_for_law().expect("settlements load");
    assert!(
        cities.len() >= 12,
        "the retained settlement roster is complete"
    );
    assert!(
        cities
            .iter()
            .any(|(place, name, _, _)| place == "jerusalem" && name == "Jerusalem"),
        "Jerusalem retains its recorded settlement identity"
    );
    let waters: Vec<_> = regions
        .iter()
        .filter(|region| {
            matches!(
                region.kind,
                map_partition::FaceKind::Sea | map_partition::FaceKind::Lake
            )
        })
        .flat_map(|region| region.rings.iter())
        .collect();
    let drowned: Vec<_> = cities
        .iter()
        .filter(|(_, _, lat, lon)| {
            let at = map_types::UnitVec::from_lat_lon_deg(*lat, *lon);
            waters
                .iter()
                .any(|ring| map_partition::winding(ring, &at) != 0)
        })
        .map(|(place, _, _, _)| place.as_str())
        .collect();
    assert!(
        drowned.contains(&"sodom"),
        "the recorded submerged site remains detected"
    );
    let jerusalem = map_types::UnitVec::from_lat_lon_deg(31.78, 35.23);
    let mut containing = Vec::new();
    for (index, face) in partition.faces.iter().enumerate() {
        let rings = partition.face_rings(index);
        let signed: f64 = rings
            .iter()
            .map(|ring| map_partition::cycle_area(ring))
            .sum();
        let winding: i32 = rings
            .iter()
            .map(|ring| map_partition::winding(ring, &jerusalem))
            .sum();
        let target = if signed <= 1e-12 { 0 } else { 1 };
        if winding == target {
            containing.push(face.kind.clone());
        }
    }
    assert_eq!(
        containing,
        vec![map_partition::FaceKind::Background],
        "Jerusalem has no territorial claim supplied by the excluded sources"
    );
    assert_eq!(
        partition
            .faces
            .iter()
            .filter(|face| face.kind == map_partition::FaceKind::Background)
            .count(),
        1,
        "retained physical witnesses leave one background face"
    );
}

/// A COHORT ENTERS TIME: features overlaid from a moment join every
/// state at or after it — a rising moment is created if none stands
/// there, inheriting what was already in effect — and every earlier
/// moment is left exactly as it was. The tribes begin at the
/// conquest; before it, the world stands without them.
#[test]
fn a_cohort_enters_time_at_its_moment() {
    use atlas_graph_types::covenant::{TimePoint, Year};
    use map_canon::{CanonStore, EntityId, Feature, LayerKind, Memory, Snapshot, World};
    let ts = |y: i32| TimePoint::year_only(Year::new(y).unwrap());
    let mut store = CanonStore::default();
    let mk = |store: &mut CanonStore, id: &str| {
        store.insert_feature(Feature::Memory(Memory {
            entity: EntityId(id.into()),
            name: id.into(),
            at: map_types::UnitVec::from_lat_lon_deg(31.0, 35.0),
        }))
    };
    let old = mk(&mut store, "old");
    let sid = store.insert_snapshot(Snapshot { features: [old].into() });
    let mut world = World::default();
    world.insert(ts(-4004), sid).unwrap();
    store.set_layer(LayerKind::ScriptureClaims, world);

    let tribe = mk(&mut store, "tribe");
    crate::partition_bridge::overlay_features_for_law(
        &mut store,
        LayerKind::ScriptureClaims,
        &[tribe].into(),
        ts(-1406),
    )
    .unwrap();

    let world = &store.layers()[&LayerKind::ScriptureClaims];
    let at = |y: i32| store.snapshots()[&world.state_at(&ts(y)).unwrap()].features.clone();
    assert!(at(-2000).contains(&old) && !at(-2000).contains(&tribe), "before: no tribes");
    assert!(at(-1406).contains(&old) && at(-1406).contains(&tribe), "the rising moment inherits");
    assert!(at(-1200).contains(&tribe), "within the span: the tribes stand");
    assert_eq!(world.moments().len(), 2, "one world before, one from the rising");

    // and a SPAN closes: overlaid [from, until), the cohort is gone at
    // the closing moment and after, while everything else remains
    let king = mk(&mut store, "king");
    crate::partition_bridge::overlay_features_for_law_span(
        &mut store,
        LayerKind::ScriptureClaims,
        &[king].into(),
        ts(-1050),
        None,
    )
    .unwrap();
    let mut store2 = CanonStore::default();
    let old2 = mk(&mut store2, "old");
    let sid2 = store2.insert_snapshot(Snapshot { features: [old2].into() });
    let mut world2 = World::default();
    world2.insert(ts(-4004), sid2).unwrap();
    store2.set_layer(LayerKind::ScriptureClaims, world2);
    let tribe2 = mk(&mut store2, "tribe");
    crate::partition_bridge::overlay_features_for_law_span(
        &mut store2,
        LayerKind::ScriptureClaims,
        &[tribe2].into(),
        ts(-1406),
        Some(ts(-1050)),
    )
    .unwrap();
    let world2 = &store2.layers()[&LayerKind::ScriptureClaims];
    let at2 = |y: i32| store2.snapshots()[&world2.state_at(&ts(y)).unwrap()].features.clone();
    assert!(at2(-1200).contains(&tribe2), "within the span");
    assert!(!at2(-1050).contains(&tribe2), "the closing moment is without the cohort");
    assert!(at2(-1050).contains(&old2), "what always stood still stands");
    assert!(!at2(-900).contains(&tribe2), "and after");
}

// ------------------------------------------------------- identity

/// The registry file is the one place two minted ids become one real
/// thing, and the loader reads exactly its written rows.
#[test]
fn the_registry_file_declares_every_unification_and_nothing_else() {
    use map_canon::{EntityId, Unification};
    let text = r#"{"unifications":[{"canonical":"phoenicia","minted":"partition:phoenicia","kind":"Polity","reason":"one coast"}]}"#;
    let reg = crate::identity::load_registry(text, "test.json").unwrap();
    let e = |s: &str| EntityId(s.to_string());
    assert_eq!(reg.resolve(&e("partition:phoenicia")), &e("phoenicia"));
    assert_eq!(reg.resolve(&e("phoenicia")), &e("phoenicia"));
    assert_eq!(reg.resolve(&e("judea")), &e("judea"));
    assert!(matches!(reg.registry().why(&e("partition:phoenicia")), Some(Unification::Declared { .. })));
    assert!(crate::identity::load_registry(r#"{"unifications":[{"canonical":"a","minted":"a","kind":"Polity","reason":"x"}]}"#, "t").is_err(), "self-unification is refused");
    assert!(crate::identity::load_registry(r#"{"unifications":[{"canonical":"a","minted":"b","kind":"Nonsense","reason":"x"}]}"#, "t").is_err(), "an unknown kind is refused");
    assert!(crate::identity::load_registry(r#"{"unifications":[{"canonical":"b","minted":"c","kind":"Polity","reason":"x"},{"canonical":"a","minted":"b","kind":"Polity","reason":"x"}]}"#, "t").is_err(), "a chain is refused");
}

/// The compile validates the registry it observed every witness into:
/// a canonical id no witness ever minted is a typo the compile refuses
/// by name, and a witness that has been observed clears it.
#[test]
fn the_compile_refuses_a_registry_no_witness_backs() {
    use map_canon::{EntityId, LayerKind, Witness};
    let text = r#"{"unifications":[{"canonical":"x","minted":"y","kind":"Polity","reason":"one"}]}"#;
    let mut id = crate::identity::load_registry(text, "t").unwrap();
    id.witness(&EntityId("y".into()), "Y", LayerKind::Territory, Witness::Atlas, "area");
    let err = id.check().unwrap_err();
    assert!(err.contains("x"), "{err}");
    id.witness(&EntityId("x".into()), "X", LayerKind::Territory, Witness::Atlas, "area");
    assert!(id.check().is_ok());
    assert_eq!(id.resolve(&EntityId("y".into())), &EntityId("x".into()));
}

/// Within one era, every bundle that resolves to one entity is one
/// area: rings and holes unioned, the held witness's layer kept, and
/// the whole era's other entities untouched.
#[test]
fn one_entity_per_era_however_many_witnesses_claimed_it() {
    use crate::partition_bridge::{unify_era_areas, EraArea};
    use map_canon::{BorderId, EntityId, LayerKind, Witness};
    let bid = |n: u64| BorderId(atlas_graph_types::covenant::ContentHash(n));
    let row = |entity: &str, layer, witness, rings: &[u64]| EraArea {
        witnessed: vec![EntityId(entity.into())],
        entity: EntityId(entity.into()),
        name: "Phoenicia".into(),
        layer,
        witness,
        verses: vec![],
        note: format!("{entity}"),
        rings: rings.iter().map(|n| bid(*n)).collect(),
        holes: Default::default(),
    };
    let rows = vec![
        row("phoenicia", LayerKind::ScriptureClaims, Witness::Authored, &[1, 2]),
        row("egypt", LayerKind::Territory, Witness::Atlas, &[9]),
        row("phoenicia", LayerKind::Territory, Witness::Atlas, &[3]),
    ];
    let out = unify_era_areas(rows);
    assert_eq!(out.len(), 2, "two entities, two areas");
    let ph = out.iter().find(|a| a.entity.0 == "phoenicia").unwrap();
    assert_eq!(ph.rings, [bid(1), bid(2), bid(3)].into());
    assert_eq!(ph.layer, LayerKind::Territory, "the held witness's layer");
    assert_eq!(ph.witness, Witness::Atlas);
    assert!(ph.note.contains("partition") || ph.note.contains("phoenicia"), "both witnesses' notes survive");
    assert_eq!(out.iter().find(|a| a.entity.0 == "egypt").unwrap().rings, [bid(9)].into());
}
