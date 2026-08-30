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

/// The integration law (L50): the real plate witnesses build one
/// lawful partition — Canaan is a face, Jerusalem is on land, and
/// the world sums to 4π.
#[test]
fn plate_partition_face_census() {
    let (regions, polylines) =
        crate::partition_bridge::gather_witnesses().expect("witnesses gather");
    let p = map_partition::build(&regions, &polylines, &map_partition::PartitionConfig::default())
        .expect("plate partition builds");
    for (i, f) in p.faces.iter().enumerate() {
        eprintln!(
            "face {i}: kind={:?} area={:.3e} cycles={} claims={:?} conflicts={:?}",
            f.kind, f.area, f.cycles.len(), f.claims, f.conflicts
        );
    }
    for d in &p.diagnostics {
        eprintln!("diag: {d}");
    }
    eprintln!("residual: {:.2e}", p.area_residual());
    assert!(p.area_residual() < 1e-10, "the 4π law on real data");
    assert!(
        p.faces.iter().any(|f| f.kind == map_partition::FaceKind::LandClaim
            && f.claims == vec!["canaan".to_string()]),
        "the pure Canaan face exists"
    );
    // every tribe names at least one face: the open-data allotments
    // survive snapping, healing, and precedence end to end.
    for tribe in [
        "judah", "simeon", "benjamin", "ephraim", "manasseh-west", "dan", "issachar",
        "zebulun", "asher", "naphtali", "reuben", "gad", "manasseh-east",
        // the attested neighbors ride the same guarantee
        "philistia", "phoenicia", "geshur", "ammon", "moab", "edom",
    ] {
        assert!(
            p.faces.iter().any(|f| f.claims.first().map(String::as_str) == Some(tribe)),
            "tribe {tribe} names no face"
        );
    }
    // the settlement roster made it through the bridge
    let cities = crate::partition_bridge::load_settlements_for_law().expect("settlements load");
    assert!(cities.len() >= 12, "a plate's worth of cities");
    assert!(cities.iter().any(|(p, n, _, _)| p == "jerusalem" && n == "Jerusalem"));
    // A CITY STANDS ON LAND: any rostered settlement whose site lies
    // beneath a water witness never becomes a canon point. Sodom's
    // traditional site is under the Dead Sea's south basin — the law
    // holds for whoever is drowned, by measurement, not by name.
    let waters: Vec<&Vec<map_types::UnitVec>> = regions
        .iter()
        .filter(|r| {
            matches!(r.kind, map_partition::FaceKind::Sea | map_partition::FaceKind::Lake)
        })
        .flat_map(|r| r.rings.iter())
        .collect();
    let drowned: Vec<&str> = cities
        .iter()
        .filter(|(_, _, lat, lon)| {
            let at = map_types::UnitVec::from_lat_lon_deg(*lat, *lon);
            waters.iter().any(|ring| map_partition::winding(ring, &at) != 0)
        })
        .map(|(p, _, _, _)| p.as_str())
        .collect();
    assert!(drowned.contains(&"sodom"), "the roster's known drowned site is caught");
    // and a drowned site is not erased — it becomes a MEMORY feature
    // (asserted at the type level: the bridge emits Feature::Memory
    // for it; see the canon provider law for its inscription dress)
    let jer = map_types::UnitVec::from_lat_lon_deg(31.78, 35.23);
    let mut jer_face = None;
    for (i, _f) in p.faces.iter().enumerate() {
        let rings = p.face_rings(i);
        let signed: f64 = rings.iter().map(|r| map_partition::cycle_area(r)).sum();
        let w: i32 = rings.iter().map(|r| map_partition::winding(r, &jer)).sum();
        let target = if signed <= 1e-12 { 0 } else { 1 };
        if w == target {
            eprintln!("JERUSALEM lives in face {i} ({:?})", p.faces[i].kind);
            jer_face = Some(p.faces[i].kind.clone());
        }
    }
    assert_eq!(jer_face, Some(map_partition::FaceKind::LandClaim), "Jerusalem is on land");
    for (i, f) in p.faces.iter().enumerate() {
        if f.kind == map_partition::FaceKind::Background && f.area < 1.0 {
            let r0 = &p.face_rings(i)[0];
            let (la, lo) = r0[0].to_lat_lon_deg();
            let mut nbs: Vec<String> = Vec::new();
            for cy in &f.cycles {
                for &h in cy {
                    let nb = p.halves[p.halves[h].twin].face;
                    let s = format!("{}:{:?}{:?}", nb, p.faces[nb].kind, p.faces[nb].claims);
                    if !nbs.contains(&s) {
                        nbs.push(s);
                    }
                }
            }
            eprintln!("POCKET {i} {:.1e} sr at ({la:.3},{lo:.3}) nbs={nbs:?}", f.area);
        }
    }
    // stem probe: which faces cross lat 32.0 in the stem window?
    for (i, f) in p.faces.iter().enumerate() {
        let mut xs: Vec<f64> = Vec::new();
        for ring in p.face_rings(i) {
            let n = ring.len();
            for k in 0..n {
                let (la1, lo1) = ring[k].to_lat_lon_deg();
                let (la2, lo2) = ring[(k + 1) % n].to_lat_lon_deg();
                if (la1 - 32.0) * (la2 - 32.0) <= 0.0 && la1 != la2 {
                    let x = lo1 + (32.0 - la1) / (la2 - la1) * (lo2 - lo1);
                    if x > 35.2 && x < 35.75 {
                        xs.push(x);
                    }
                }
            }
        }
        if !xs.is_empty() {
            xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let xs4: Vec<String> = xs.iter().map(|x| format!("{x:.5}")).collect();
            eprintln!("STEM face {i} {:?} claims={:?} xs={:?}", f.kind, f.claims, xs4);
        }
    }
    {
        use std::collections::BTreeMap;
        let mut firsts: BTreeMap<String, usize> = BTreeMap::new();
        let mut anywhere: BTreeMap<String, usize> = BTreeMap::new();
        for f in &p.faces {
            if let Some(w) = f.claims.first() {
                *firsts.entry(w.clone()).or_insert(0) += 1;
            }
            for w in &f.claims {
                *anywhere.entry(w.clone()).or_insert(0) += 1;
            }
        }
        eprintln!("CLAIMS first: {firsts:?}");
        let probe = map_types::UnitVec::from_lat_lon_deg(32.1, 35.15);
        for (i, f) in p.faces.iter().enumerate() {
            let rings = p.face_rings(i);
            let signed: f64 = rings.iter().map(|r| map_partition::cycle_area(r)).sum();
            let wnum: i32 = rings.iter().map(|r| map_partition::winding(r, &probe)).sum();
            let target = if signed <= 1e-12 { 0 } else { 1 };
            if wnum == target {
                eprintln!("EPHRAIM-PROBE face {i} {:?} claims={:?}", f.kind, f.claims);
            }
        }
        for r in &regions {
            if ["ephraim", "simeon", "benjamin"].contains(&r.id.as_str()) {
                let probe = match r.id.as_str() {
                    "ephraim" => map_types::UnitVec::from_lat_lon_deg(32.1, 35.15),
                    "simeon" => map_types::UnitVec::from_lat_lon_deg(31.2, 34.75),
                    _ => map_types::UnitVec::from_lat_lon_deg(31.9, 35.2),
                };
                eprintln!(
                    "RING {}: {} pts, winding at interior probe = {}",
                    r.id,
                    r.rings[0].len(),
                    map_partition::winding(&r.rings[0], &probe)
                );
            }
        }
        for w in ["ephraim", "simeon"] {
            eprintln!(
                "CLAIMS {w}: first={} anywhere={}",
                firsts.get(w).copied().unwrap_or(0),
                anywhere.get(w).copied().unwrap_or(0)
            );
        }
    }
    let backgrounds =
        p.faces.iter().filter(|f| f.kind == map_partition::FaceKind::Background).count();
    assert_eq!(backgrounds, 1, "one Background face: every other cell is claimed — no wedges");
    for (i, f) in p.faces.iter().enumerate() {
        if f.kind == map_partition::FaceKind::Background && f.area < 1.0 {
            let r0 = &p.face_rings(i)[0];
            let (la, lo) = r0[0].to_lat_lon_deg();
            eprintln!("BG WEDGE face {i}: {:.2e} sr at ({la:.3},{lo:.3}) {} pts", f.area, r0.len());
        }
    }
    for (i, _f) in p.faces.iter().enumerate() {
        for (ci, ring) in p.face_rings(i).iter().enumerate() {
            let (la, lo) = ring[0].to_lat_lon_deg();
            eprintln!(
                "  face {i} ring {ci}: {} pts, starts ({la:.2},{lo:.2}), signed {:.3e}",
                ring.len(),
                map_partition::cycle_area(ring)
            );
        }
    }
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
    // the world before: one feature standing from -4004
    let old = mk(&mut store, "old");
    let sid = store.insert_snapshot(Snapshot { features: [old].into() });
    let mut world = World::default();
    world.insert(ts(-4004), sid).unwrap();
    store.set_layer(LayerKind::ScriptureClaims, world);

    // the cohort rises at -1406
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
    assert!(at(-1000).contains(&tribe), "after: the tribes stand");
    assert_eq!(world.moments().len(), 2, "one world before, one from the rising");
}
