use std::path::Path;
use std::process::Command;

use map_compile::exclusion::{self, ExclusionError};
use map_types::UnitVec;
use proptest::prelude::*;

#[test]
fn every_removed_osm_and_gpl_line_has_registered_ordered_lineage() {
    #[derive(serde::Deserialize)]
    struct Catalogue {
        geometries: Vec<exclusion::ExcludedGeometry>,
    }
    let catalogue: Catalogue = serde_json::from_str(include_str!(
        "../../../data/authored/excluded-geometry-fingerprints.json"
    ))
    .unwrap();
    let paths = Command::new("git")
        .args([
            "ls-tree",
            "-r",
            "--name-only",
            "6ac32bfb",
            "--",
            "data/historical-basemaps",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    let paths = String::from_utf8(paths.stdout).unwrap();
    for path in std::iter::once("data/osm/rivers.geojson")
        .chain(paths.lines().filter(|path| path.ends_with(".geojson")))
    {
        let expected = exclusion::geojson_lines(&original(path)).len();
        let actual = catalogue
            .geometries
            .iter()
            .filter(|geometry| {
                geometry.origin == path
                    && (geometry.source == exclusion::ExcludedSource::OsmRivers
                        || geometry.geometry.ends_with("/raw"))
            })
            .count();
        assert_eq!(
            actual, expected,
            "every ordered source line is registered through the sole geometric owner: {path}"
        );
    }
}

#[test]
fn quarantined_polities_preserve_the_complete_original_evidence_document() {
    let quarantine: serde_json::Value = serde_json::from_str(include_str!(
        "../../../data/authored/quarantined-courses.json"
    ))
    .unwrap();
    let mut expected = original("data/atlas-vendor/polities.json");
    for row in expected["polities"].as_array_mut().unwrap() {
        if quarantine["polities"]
            .as_array()
            .unwrap()
            .iter()
            .any(|record| {
                record["entity"] == row["id"]
                    && record["from"] == row["from"]
                    && record["to"] == row["to"]
            })
        {
            row["rings"] = serde_json::json!([]);
        }
    }
    let actual: serde_json::Value =
        serde_json::from_str(include_str!("../../../data/atlas-vendor/polities.json")).unwrap();
    assert_eq!(actual, expected, "quarantine removes only the recorded rings and preserves every original identity, date, provenance field, verse reference and alternative");
}

#[test]
fn renamed_jordan_vertices_are_refused_by_the_geometric_owner() {
    let value = probe(Probe::Jordan);
    assert!(
        exclusion::check_geojson(&value).is_err(),
        "the 96 renamed OSM vertices are refused before river admission"
    );
}

#[test]
fn three_consecutive_gpl_vertices_are_refused_by_the_geometric_owner() {
    let value = probe(Probe::Egypt);
    assert!(
        exclusion::check_geojson(&value).is_err(),
        "three consecutive renamed GPL vertices exceed the unchanged geometric threshold"
    );
}

#[test]
fn the_real_build_refuses_each_renamed_reviewer_probe_before_reading_the_vendor() {
    for selected in [Probe::Jordan, Probe::Egypt] {
        let value = probe(selected.clone());
        let expected =
            exclusion::check_geojson(&value).expect_err("the excluded source run is refused");
        let root =
            std::env::temp_dir().join(format!("mg-x0-3-{}-{selected:?}", std::process::id()));
        std::fs::create_dir_all(root.join("data/natural-earth")).unwrap();
        std::fs::write(
            root.join("data/natural-earth/renamed.geojson"),
            serde_json::to_vec(&value).unwrap(),
        )
        .unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_map-compile"))
            .arg("build")
            .current_dir(&root)
            .output()
            .unwrap();
        let produced = root.join("data/canon").exists();
        std::fs::remove_dir_all(root).unwrap();
        assert_eq!(
            output.status.code(),
            Some(1),
            "the real build refuses the renamed excluded input"
        );
        assert_eq!(
            String::from_utf8(output.stderr).unwrap(),
            format!("map-compile FAILED: excluded input: {expected:?}\n"),
            "the real compiler reports the complete geometric refusal before a missing vendor read"
        );
        assert_eq!(
            produced, false,
            "a refused input produces no canon directory"
        );
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]
    #[test]
    fn excluded_runs_keep_their_complete_refusal_when_renamed_reversed_and_split(
        selected in prop_oneof![Just(Probe::Jordan), Just(Probe::Egypt)],
        name in any::<String>(), reverse in any::<bool>(),
    ) {
        let original = probe(selected);
        let expected = exclusion::check_geojson(&original).expect_err("the independently retained excluded run is refused");
        let mut points = exclusion::geojson_lines(&original)[0].0.clone();
        if reverse { points.reverse(); }
        let coordinates: Vec<_> = points.iter().map(|point| [point.x, point.y]).collect();
        let parts: Vec<_> = coordinates.windows(2).map(|part| serde_json::json!({"geometry":{"coordinates":part},"properties":{"name":name}})).collect();
        prop_assert_eq!(exclusion::check_geojson(&serde_json::json!(parts)), Err(expected), "the composed ordered fragments retain the complete geometric refusal and its provenance");
    }
}

#[test]
fn retained_polity_courses_have_permitted_explanations() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
    let rows = map_compile::vendor::parse_polities(
        &std::fs::read_to_string(root.join("atlas-vendor/polities.json")).unwrap(),
    )
    .unwrap();
    let mut refused = Vec::new();
    for row in rows {
        let lines: Vec<Vec<_>> = row
            .rings
            .iter()
            .map(|ring| {
                ring.iter()
                    .map(|(lat, lon)| UnitVec::from_lat_lon_deg(*lat, *lon))
                    .collect()
            })
            .collect();
        if let Err(ExclusionError::Excluded(stretch)) =
            exclusion::check_sequences(lines.iter().map(Vec::as_slice))
        {
            refused.push((
                row.id,
                row.from_year,
                row.to_year,
                stretch.geometry.source,
                stretch.geometry.geometry,
            ));
        }
    }
    assert_eq!(
        refused,
        vec![],
        "every retained polity course has a permitted geometric explanation"
    );
}

#[test]
fn retained_scripture_courses_have_permitted_explanations() {
    let timeline = map_adapters::scripture_timeline();
    let mut refused = Vec::new();
    for (id, history) in &timeline.boundaries {
        for (_, boundary) in &history.versions {
            if let Err(ExclusionError::Excluded(stretch)) = exclusion::check_points(&boundary.pts) {
                refused.push((
                    format!("{:016x}", id.0 .0),
                    boundary.source.clone(),
                    stretch.geometry.source,
                    stretch.geometry.geometry,
                ));
            }
        }
    }
    assert_eq!(
        refused,
        vec![],
        "every retained Scripture course has a permitted geometric explanation"
    );
}

#[test]
fn composed_scripture_lineage_remains_admitted() {
    let timeline = map_adapters::scripture_timeline();
    let result = exclusion::check_timeline(&timeline);
    if let Err(ExclusionError::Excluded(stretch)) = &result {
        for (id, history) in &timeline.boundaries {
            for (_, boundary) in &history.versions {
                let meters =
                    exclusion::unexplained_meters(&stretch.geometry, &boundary.pts).unwrap();
                if meters > 0.0 {
                    println!(
                        "contribution={:016x} meters={meters} source={:?}",
                        id.0 .0, boundary.source
                    );
                }
            }
        }
    }
    assert_eq!(
        result,
        Ok(()),
        "the complete Scripture timeline stays admitted when its surviving courses are composed"
    );
}

#[test]
fn retained_input_composition_remains_admitted() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
    let result = exclusion::check_build_inputs(&root);
    if let Err(ExclusionError::Excluded(stretch)) = &result {
        let rows = map_compile::vendor::parse_polities(
            &std::fs::read_to_string(root.join("atlas-vendor/polities.json")).unwrap(),
        )
        .unwrap();
        for row in rows {
            let meters: f64 = row
                .rings
                .iter()
                .map(|ring| {
                    let points: Vec<_> = ring
                        .iter()
                        .map(|(lat, lon)| UnitVec::from_lat_lon_deg(*lat, *lon))
                        .collect();
                    exclusion::unexplained_meters(&stretch.geometry, &points).unwrap()
                })
                .sum();
            if meters > 0.0 {
                println!(
                    "polity={} from={} to={} meters={meters}",
                    row.id, row.from_year, row.to_year
                );
            }
        }
    }
    assert_eq!(
        result,
        Ok(()),
        "the complete retained input corpus stays admitted when its surviving courses are composed"
    );
}

fn original(path: &str) -> serde_json::Value {
    let output = Command::new("git")
        .args(["show", &format!("6ac32bfb:{path}")])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    serde_json::from_slice(&output.stdout).unwrap()
}

#[derive(Clone, Debug)]
enum Probe {
    Jordan,
    Egypt,
}

fn probe(selected: Probe) -> serde_json::Value {
    let coordinates = match selected {
        Probe::Jordan => {
            let source = original("data/osm/rivers.geojson");
            source["features"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|feature| feature["properties"]["name"] == "River Jordan")
                .max_by_key(|feature| feature["geometry"]["coordinates"].as_array().unwrap().len())
                .unwrap()["geometry"]["coordinates"]
                .clone()
        }
        Probe::Egypt => {
            let source = original("data/historical-basemaps/world_bc1500.geojson");
            let egypt = source["features"]
                .as_array()
                .unwrap()
                .iter()
                .find(|feature| feature["properties"]["NAME"] == "Egypt")
                .unwrap();
            serde_json::json!(&egypt["geometry"]["coordinates"][0][0].as_array().unwrap()[..3])
        }
    };
    serde_json::json!({"geometry":{"type":"LineString","coordinates":coordinates},"properties":{"name":"renamed","rivernum":229,"featurecla":"River"}})
}
