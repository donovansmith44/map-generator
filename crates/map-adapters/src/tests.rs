use std::collections::BTreeMap;

use atlas_graph_types::covenant::{TimePoint, Year};
use atlas_graph_types::covenant::{ContentHash, SourceId};

use map_types::{validate_all, ChronologyExport, GazetteerExport};
use serde_json::json;

use crate::quantize::{clean_ring, QPoint};

fn tp(year: i32) -> TimePoint {
    TimePoint::year_only(Year::new(year).unwrap())
}

fn empty_exports() -> (ChronologyExport, GazetteerExport) {
    (
        ChronologyExport { atlas_root: ContentHash(0), placements: BTreeMap::new(), spans: Vec::new() },
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

// --------------------------------------------------- the waters

#[test]
fn waters_are_explorable_regions() {
    use crate::surveys::{merge_timelines, scripture_timeline, stand_in_gazetteer};
    let text = fc(&[
        (Some("Test Sea"), vec![square(10.0, 10.0, 12.0, 12.0)]),
        (None, vec![square(20.0, 10.0, 25.0, 15.0)]),
    ]);
    let water = crate::hydro::ingest_water(
        &SourceId::new("natural-earth"),
        tp(-4004),
        &[crate::hydro::WaterSource {
            label_for_unnamed: "the sea",
            text,
            skip_largest_feature: false,
        }],
    )
    .unwrap();
    assert_eq!(
        water.regions.len(),
        2,
        "both named and unnamed water features become regions"
    );
    assert!(
        water
            .regions
            .values()
            .all(|r| r.class == map_types::RegionClass::Water),
        "every water feature retains the water class"
    );
    let labels: std::collections::BTreeSet<&str> = water
        .regions
        .values()
        .map(|r| r.label_at(&tp(-1000)).unwrap())
        .collect();
    assert_eq!(
        labels,
        ["Test Sea", "the sea"].into(),
        "water names include the source name and the recorded unnamed-water label"
    );
    let (chron, gaz) = empty_exports();
    assert_eq!(
        validate_all(&water, &chron, &gaz),
        vec![],
        "the complete water timeline is lawful"
    );

    let merged = merge_timelines(water, scripture_timeline()).unwrap();
    assert_eq!(
        validate_all(&merged, &chron, &stand_in_gazetteer()),
        vec![],
        "water and Scripture compose into a lawful timeline"
    );
}

#[test]
fn ring_cleaning_normalizes() {
    let cleaned =
        clean_ring(&[(0.0, 0.0), (1.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 0.0)]).unwrap();
    assert_eq!(
        cleaned,
        vec![
            QPoint::from_lon_lat(0.0, 0.0),
            QPoint::from_lon_lat(1.0, 0.0),
            QPoint::from_lon_lat(1.0, 1.0)
        ],
        "ring normalization keeps the complete ordered vertices without repeats"
    );
    assert_eq!(
        clean_ring(&[(0.0, 0.0), (1.0, 0.0), (0.0, 0.0)]),
        None,
        "a ring with fewer than three vertices is refused"
    );
}


#[test]
fn scripture_surveys_are_lawful_alone_and_merged() {
    use crate::surveys::*;

    let survey_tl = scripture_timeline();
    let gaz = stand_in_gazetteer();
    let (chron, _) = empty_exports();
    assert_eq!(map_types::validate_all(&survey_tl, &chron, &gaz), vec![], "the retained Scripture surveys satisfy every timeline law");

    let world = crate::hydro::ingest_water(
        &SourceId::new("natural-earth"),
        tp(-2000),
        &[crate::hydro::WaterSource {
            label_for_unnamed: "sea",
            text: fc(&[(Some("Elsewhere Sea"), vec![square(50.0, 40.0, 51.0, 41.0)])]),
            skip_largest_feature: false,
        }],
    )
    .unwrap();
    let merged = merge_timelines(world, survey_tl.clone()).unwrap();

    assert_eq!(map_types::validate_all(&merged, &chron, &gaz), vec![], "the merged survey and world satisfy every timeline law");
    assert_eq!(merged.regions.len(), 28, "the twenty-seven retained Scripture regions and imported region survive the merge");


    assert!(
        matches!(
            merge_timelines(merged, survey_tl),
            Err(MergeError::DuplicateBoundary(_))
        ),
        "merging the same survey twice refuses its duplicate boundary"
    );

    let all = scripture_timeline();

    assert_eq!(all.regions.len(), 27, "the Scripture set excludes the unlocated Numbers 34 drawing");

    let journeys = all
        .boundaries
        .values()
        .filter(|h| h.versions[0].1.character == map_types::EdgeCharacter::Way)
        .count();
    assert_eq!(
        journeys, 19,
        "the whole-Bible route book, patriarchs to Paul"
    );
    assert_eq!(
        map_types::validate_all(&all, &chron, &gaz),
        vec![],
        "the complete Scripture timeline satisfies every timeline law"
    );

    use map_types::ChangeKind as K;
    let shifts = all
        .events
        .iter()
        .filter(|e| matches!(e.kind, K::Shift { .. }))
        .count();
    let falls = all
        .events
        .iter()
        .filter(|e| matches!(e.kind, K::Fall { .. }))
        .count();
    assert_eq!(
        shifts, 2,
        "the Solomonic dominion and Jeroboam II's restoration"
    );
    assert_eq!(
        falls, 3,
        "the division ends the united kingdom; Samaria; Jerusalem"
    );
    assert!(
        all.events
            .iter()
            .all(|e| !e.justification.grounds.is_empty()),
        "every Scripture event carries its grounds"
    );

    use map_types::EdgeCharacter;
    let characters: Vec<_> = all
        .boundaries
        .values()
        .map(|h| h.versions[0].1.character.clone())
        .collect();
    assert!(
        characters.iter().any(|c| matches!(c, EdgeCharacter::Line)),
        "the Scripture timeline retains attested border lines"
    );
    assert!(
        characters
            .iter()
            .any(|c| matches!(c, EdgeCharacter::Unknown)),
        "the Scripture timeline retains uncertain hull boundaries"
    );
}

// --------------------------- C2/C3: the great stand-in replacement

/// Against the REAL atlas export artifacts: the exports parse, the
/// roots agree, creation resolves, places bind canonical-first, events
/// bind by attestation, dates are ADOPTED from the atlas (authority
/// order), and the whole bound timeline is lawful — law 12a's byte
/// equality enforced by the same validators as everything else.
#[test]
fn atlas_exports_bind_the_scripture_set() {
    use crate::exports::load_exports;
    use crate::surveys::{merged_gazetteer, scripture_timeline_with};

    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/atlas-exports");
    let atlas = load_exports(
        &std::fs::read_to_string(dir.join("gazetteer.json")).expect("vendored gazetteer"),
        &std::fs::read_to_string(dir.join("chronology.json")).expect("vendored chronology"),
    )
    .expect("exports parse");

    assert_eq!(format!("{:016x}", atlas.root.0), "a1b93a3b049a1fe0", "the pinned root");
    assert_eq!(atlas.creation_anchor().map(|(y, _)| y), Some(-4004));
    assert!(atlas.resolve_place("Kadesh-barnea").is_some(), "canonical binding works");
    assert!(atlas.resolve_place("En-rogel").is_some());
    assert!(atlas.resolve_place("entrance of Hamath").is_some(), "GAZ-1 alias binds");
    assert!(atlas.resolve_place("Brook of Egypt").is_some());
    assert!(atlas.resolve_place("oblation southwest corner").is_none(), "descriptive points stay stand-in");

    let bound = scripture_timeline_with(Some(&atlas));
    let driven = bound.events.iter().filter(|e| e.driver.is_some()).count();
    assert!(driven > 0, "some events now carry atlas drivers");
    let atlas_prov = bound
        .boundaries
        .values()
        .filter(|h| h.versions[0].1.provenance.contains("bible-atlas@"))
        .count();
    assert!(atlas_prov > 0, "some circuits now carry atlas-resolved waypoints");

    // The bound set is lawful under the REAL authority: 12a byte
    // equality for every driver, 12c resolution over the merged
    // gazetteer, coherence and narration intact despite adopted dates.
    let violations =
        map_types::validate_all(&bound, &atlas.chronology, &merged_gazetteer(&atlas));
    assert_eq!(violations, vec![], "the bound Scripture set is lawful");

    // Authority is visible: creation-anchored events that bound now sit
    // at ATLAS dates; unbound ones kept their disclosed stand-ins.
    eprintln!(
        "bound: {driven}/{} events driven, {atlas_prov} circuits atlas-resolved",
        bound.events.len()
    );
}

/// The audit table itself: printed for the atlas session, asserted
/// non-trivial (we KNOW at least Samaria disagrees today).
#[test]
fn chronology_audit_reports_disagreements() {
    use crate::exports::load_exports;
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/atlas-exports");
    let atlas = load_exports(
        &std::fs::read_to_string(dir.join("gazetteer.json")).unwrap(),
        &std::fs::read_to_string(dir.join("chronology.json")).unwrap(),
    )
    .unwrap();
    let rows = crate::surveys::binding_report(&atlas);
    for r in &rows {
        eprintln!(
            "AUDIT {} | ours {} vs atlas {} (delta {:+}) | {} \"{}\"",
            r.ours,
            r.our_year,
            r.their_year,
            r.their_year - r.our_year,
            r.atlas_event.0,
            r.atlas_event.1
        );
    }
    assert!(!rows.is_empty(), "the Samaria disagreement at minimum");
}

// ----------------------------------------------------------- terrain

/// A synthetic hill: marching squares closes ONE ring around it, the
/// run is deterministic, and the ingested timeline is lawful with the
/// Terrain class on every band region.
#[test]
fn terrain_contours_close_and_ingest_lawfully() {
    use crate::terrain::{contour_rings, ingest_terrain, ElevationGrid};
    // 12x12 grid, 1-degree cells: a broad flat hill of 300 m in the
    // middle of a 0 m plain.
    let (rows, cols) = (12usize, 12usize);
    let mut data = vec![0i16; rows * cols];
    for i in 4..8 {
        for j in 4..8 {
            data[i * cols + j] = 300;
        }
    }
    let grid = ElevationGrid { rows, cols, lat0: 10.0, lon0: 20.0, step: 1.0, data };
    let rings = contour_rings(&grid, 200.0);
    assert_eq!(rings.len(), 1, "one hill, one contour");
    let ring = &rings[0];
    assert!(ring.len() >= 8);
    for (lat, lon) in ring {
        // Every crossing lies inside the hill's collar, between the
        // plain corners (14,24)..(17,27) padded by one cell.
        assert!((13.0..=18.0).contains(lat) && (23.0..=28.0).contains(lon), "({lat},{lon})");
    }
    assert_eq!(rings, contour_rings(&grid, 200.0), "law 1: identical runs");

    let tl = ingest_terrain(&grid, tp(-4004));
    assert_eq!(tl.regions.len(), 1, "only the 200 m band exists on a 300 m hill");
    let r = tl.regions.values().next().unwrap();
    assert_eq!(r.class, map_types::RegionClass::Terrain(0));
    let (chron, gaz) = empty_exports();
    assert_eq!(validate_all(&tl, &chron, &gaz), vec![]);
    use crate::surveys::{merge_timelines, scripture_timeline, stand_in_gazetteer};
    let merged = merge_timelines(scripture_timeline(), tl).expect("relief merges");
    assert_eq!(validate_all(&merged, &chron, &stand_in_gazetteer()), vec![]);
}

/// The vendored ETOPO grid parses and contours into all five bands —
/// the earth has hills, uplands, highlands, mountains, and peaks.
#[test]
fn terrain_real_grid_smoke() {
    use crate::terrain::{ingest_terrain, ElevationGrid, BANDS};
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/terrain/etopo_15min.bin");
    let bytes = std::fs::read(path).expect("vendored terrain grid");
    let grid = ElevationGrid::from_etopo_bin(&bytes).expect("shape 721x1441");
    let tl = ingest_terrain(&grid, tp(-4004));
    assert_eq!(tl.regions.len(), BANDS.len(), "every band is inhabited on the real earth");
    for r in tl.regions.values() {
        assert!(matches!(r.class, map_types::RegionClass::Terrain(_)));
        assert!(!r.geom_history[0].1.parts.is_empty());
    }
    let (chron, gaz) = empty_exports();
    assert_eq!(validate_all(&tl, &chron, &gaz), vec![]);
}

/// A journey is a WAY, not a border: every scripture route wears
/// EdgeCharacter::Way so no consumer can mistake an itinerary for a
/// territorial claim (and every style dresses it distinctly).
#[test]
fn routes_wear_the_way_character() {
    let tl = crate::surveys::scripture_timeline();
    let ways: Vec<_> = tl
        .boundaries
        .values()
        .flat_map(|h| h.versions.iter())
        .filter(|(_, b)| b.character == map_types::EdgeCharacter::Way)
        .collect();
    assert!(ways.len() >= 5, "exodus + four Pauline routes at least, got {}", ways.len());
    for (_, b) in ways {
        assert!(
            matches!(b.source, map_types::BoundarySource::Survey(_)),
            "a way's stations come from the text"
        );
    }
}

/// The whole Bible travels: journeys span the patriarchs through the
/// apostles, not just the Exodus and Paul.
#[test]
fn journeys_cover_the_whole_bible() {
    let tl = crate::surveys::scripture_timeline();
    let ways: Vec<_> = tl
        .boundaries
        .values()
        .flat_map(|h| h.versions.iter())
        .filter(|(_, b)| b.character == map_types::EdgeCharacter::Way)
        .collect();
    assert!(ways.len() >= 15, "a whole-Bible route book, got {}", ways.len());
    let year = |i: &map_types::Interval| i.from.year.get();
    assert!(
        ways.iter().any(|(i, _)| year(i) <= -1900),
        "the patriarchs walk (Abraham's call and after)"
    );
    assert!(
        ways.iter().any(|(i, _)| (-1200..=-580).contains(&year(i))),
        "the kingdom era walks (ark, prophets, exile)"
    );
    assert!(
        ways.iter().any(|(i, _)| (-10..=40).contains(&year(i))),
        "the Gospels walk (nativity, ministry)"
    );
    assert!(
        ways.iter().filter(|(i, _)| year(i) >= 30).count() >= 6,
        "the church walks (Acts beyond Paul)"
    );
    // Every journey happens IN TIME: a closed span from departure to
    // arrival — a snapshot outside the span shows nothing of it.
    assert!(
        ways.iter().all(|(i, _)| i.to.is_some()),
        "journeys have spans; the calendar, not just the toggle, governs them"
    );
    // And every journey is a scrub stop: departure always, arrival too
    // when the walk crosses years — the scrubber can land on it.
    let journey_events = tl
        .events
        .iter()
        .filter(|e| matches!(e.kind, map_types::ChangeKind::Journey { .. }))
        .count();
    assert!(
        journey_events >= ways.len(),
        "at least one stop per journey, got {journey_events} for {} ways",
        ways.len()
    );
    let multi_year = ways.iter().filter(|(i, _)| {
        i.to.unwrap().year.get() - i.from.year.get() > 1
    }).count();
    assert!(multi_year >= 5, "exodus, Abraham, Jacob, Paul's long roads span years");
}

#[test]
fn river_build_inputs_exclude_osm() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for relative in [
        "crates/map-compile/src/main.rs",
        "crates/map-compile/src/partition_bridge.rs",
    ] {
        let source =
            std::fs::read_to_string(root.join(relative)).expect("compiler source is readable");
        assert!(
            !source.contains("data/osm"),
            "the compiler never reads an OSM river input"
        );
    }
    assert!(
        !root.join("data/osm").exists(),
        "OSM river bytes are not vendored build inputs"
    );
}

#[test]
fn river_adapter_preserves_every_natural_earth_vertex() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let text = std::fs::read_to_string(
        root.join("data/natural-earth/ne_10m_rivers_lake_centerlines.geojson"),
    )
    .expect("Natural Earth rivers are readable");
    let source: serde_json::Value = serde_json::from_str(&text).expect("Natural Earth river JSON");
    let features = source["features"].as_array().expect("river features");
    let rivers = crate::hydro::read_rivers(&text).expect("Natural Earth courses are admitted");
    assert_eq!(
        rivers.len(),
        features.len(),
        "every Natural Earth river feature is retained"
    );
    for (river, feature) in rivers.iter().zip(features) {
        assert_eq!(
            river.number.0,
            feature["properties"]["rivernum"]
                .as_i64()
                .expect("river number"),
            "the source's river identity is retained"
        );
        assert_eq!(
            river.name.as_deref(),
            feature["properties"]["name"].as_str(),
            "the source's name is retained without a guessed name"
        );
        let expected_course = match feature["properties"]["featurecla"].as_str() {
            Some("River") => crate::hydro::RiverCourse::River,
            Some("Lake Centerline") => crate::hydro::RiverCourse::LakeCenterline,
            _ => panic!("the source names a known river course class"),
        };
        assert_eq!(
            river.course, expected_course,
            "the source distinguishes rivers from lake centerlines"
        );
        let paths = feature["geometry"]["coordinates"]
            .as_array()
            .expect("source paths");
        let course = match &river.shape {
            crate::hydro::RiverShape::Course(course) => course.as_slice(),
            crate::hydro::RiverShape::Unlocated => {
                assert!(
                    paths.is_empty(),
                    "a source without vertices is explicitly unlocated"
                );
                &[]
            }
        };
        assert_eq!(course.len(), paths.len(), "every source path is retained");
        for (path, coordinates) in course.iter().zip(paths) {
            let coordinates = coordinates.as_array().expect("source coordinates");
            assert_eq!(
                path.len(),
                coordinates.len(),
                "every source vertex is retained"
            );
            for (point, coordinate) in path.iter().zip(coordinates) {
                assert_eq!(
                    *point,
                    map_types::UnitVec::from_lat_lon_deg(
                        coordinate[1].as_f64().expect("latitude"),
                        coordinate[0].as_f64().expect("longitude")
                    ),
                    "the course uses the source vertex without tuning"
                );
            }
        }
    }
}


fn river_fixture(geometry: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "type": "FeatureCollection",
        "features": [{
            "type": "Feature",
            "properties": {"rivernum": 229, "name": null, "featurecla": "River"},
            "geometry": geometry
        }]
    })
}

#[test]
fn malformed_river_courses_refuse_without_dropping_vertices() {
    for position in [
        serde_json::json!([]),
        serde_json::json!([35.0]),
        serde_json::json!(["35", 32]),
        serde_json::json!([35, null]),
        serde_json::json!([181, 32]),
        serde_json::json!([35, 91]),
    ] {
        let source = river_fixture(
            serde_json::json!({"type": "LineString", "coordinates": [[35, 32], position]}),
        );
        assert_eq!(
            crate::hydro::read_rivers(&source.to_string()),
            Err(crate::hydro::RiverError::Position),
            "a malformed vertex refuses the course instead of shortening it"
        );
    }
    for geometry in [
        serde_json::json!({"type": "Polygon", "coordinates": [[[35, 32], [36, 33]]]}),
        serde_json::json!({"type": "Point", "coordinates": [35, 32]}),
    ] {
        assert_eq!(
            crate::hydro::read_rivers(&river_fixture(geometry).to_string()),
            Err(crate::hydro::RiverError::Geometry),
            "a non-river geometry refuses the river adapter"
        );
    }
    for coordinates in [serde_json::json!([]), serde_json::json!([[35, 32]])] {
        let source =
            river_fixture(serde_json::json!({"type": "LineString", "coordinates": coordinates}));
        assert_eq!(
            crate::hydro::read_rivers(&source.to_string()),
            Err(crate::hydro::RiverError::Path),
            "an incomplete course is recorded as an error rather than drawn"
        );
    }
    for class in [
        serde_json::json!("Canal"),
        serde_json::json!(true),
        serde_json::json!(null),
    ] {
        let mut source = river_fixture(
            serde_json::json!({"type": "LineString", "coordinates": [[35, 32], [36, 33]]}),
        );
        source["features"][0]["properties"]["featurecla"] = class;
        assert_eq!(
            crate::hydro::read_rivers(&source.to_string()),
            Err(crate::hydro::RiverError::Course),
            "an unknown course class cannot masquerade as a river"
        );
    }
}
