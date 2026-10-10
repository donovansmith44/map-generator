use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;
use std::sync::OnceLock;

use map_compile::exclusion::{self, ExcludedGeometry, ExclusionError};
use map_compile::vendor::PolityRow;
use map_types::UnitVec;
use proptest::prelude::*;
use serde::Deserialize;

const BASE: &str = "cf3c348";

#[test]
fn the_lineage_catalogue_covers_every_source_feature_and_recorded_transform() {
    let mut expected = Vec::new();
    for (origin, original) in originals() {
        for feature in 0..original["features"]
            .as_array()
            .expect("source features exist")
            .len()
        {
            for method in [Transform::Raw, Transform::Quantized, Transform::Snapped] {
                let control = Control {
                    origin: origin.clone(),
                    feature,
                    method,
                    excluded: vec![],
                };
                if !points(&control).is_empty() {
                    expected.push((origin.clone(), feature, method));
                }
            }
        }
    }
    let actual: Vec<_> = catalogue()
        .basemap_controls
        .iter()
        .map(|control| (control.origin.clone(), control.feature, control.method))
        .collect();
    assert_eq!(
        actual, expected,
        "every historical feature and recorded transform has a complete lineage control"
    );
}

#[test]
fn every_gpl_feature_and_recorded_transform_is_refused_before_admission() {
    for control in &catalogue().basemap_controls {
        assert_eq!(
            exclusion::check_points(&points(control)),
            Err(expected(control)),
            "each original, quantized and snapped feature retains its complete exclusion provenance: {control:?}"
        );
    }
}

#[test]
fn renamed_gpl_input_is_refused_by_the_real_build_before_any_output() {
    let control = &catalogue().basemap_controls[0];
    let value = originals()[&control.origin]["features"][control.feature]["geometry"].clone();
    let root = std::env::temp_dir().join(format!("maps-x2-build-{}", std::process::id()));
    std::fs::create_dir_all(root.join("data/permitted-looking"))
        .expect("temporary build input directory");
    std::fs::write(
        root.join("data/permitted-looking/renamed.geojson"),
        serde_json::to_vec(&serde_json::json!({"geometry":value})).expect("probe serializes"),
    )
    .expect("temporary renamed input");
    let output = Command::new(env!("CARGO_BIN_EXE_map-compile"))
        .arg("build")
        .current_dir(&root)
        .output()
        .expect("real compiler runs");
    let produced = root.join("data/canon").exists();
    std::fs::remove_dir_all(&root).expect("remove only this test's temporary inputs");
    assert_eq!(
        output.status.code(),
        Some(1),
        "the build refuses the restored GPL input"
    );
    assert_eq!(
        String::from_utf8(output.stderr).expect("compiler diagnostic is UTF-8"),
        format!(
            "map-compile FAILED: excluded input: {:?}\n",
            expected(control)
        ),
        "the real build reports the complete excluded lineage before reading unrelated sources"
    );
    assert_eq!(
        produced, false,
        "a refused GPL input writes no compiled output directory"
    );
}

#[test]
fn gpl_content_split_across_renamed_input_files_is_refused() {
    let control = &catalogue().basemap_controls[0];
    let points = points(control);
    let root = std::env::temp_dir().join(format!("maps-x2-split-{}", std::process::id()));
    std::fs::create_dir_all(&root).expect("temporary split input directory");
    for (index, part) in points.chunks(2).enumerate() {
        let coordinates: Vec<_> = part
            .iter()
            .map(|point| {
                let (lat, lon) = point.to_lat_lon_deg();
                [lon, lat]
            })
            .collect();
        let value = serde_json::json!({"geometry":{"coordinates":coordinates}});
        std::fs::write(
            root.join(format!("licensed-{index}.geojson")),
            serde_json::to_vec(&value).expect("fragment serializes"),
        )
        .expect("temporary renamed fragment");
    }
    let result = exclusion::check_build_inputs(&root);
    std::fs::remove_dir_all(&root).expect("remove only this test's split inputs");
    assert_eq!(
        result,
        Err(expected(control)),
        "the union of renamed input files retains the complete GPL exclusion"
    );
}

#[test]
fn the_gpl_vendor_and_ingest_are_removed() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for path in [
        "data/historical-basemaps",
        "crates/map-adapters/src/basemaps.rs",
        "crates/map-adapters/src/arcs.rs",
    ] {
        assert!(
            !root.join(path).exists(),
            "the excluded source and its retired ingest machinery are removed: {path}"
        );
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]
    #[test]
    fn gpl_descendants_keep_their_refusal_through_composed_inputs_and_outputs(
        index in 0usize..catalogue().basemap_controls.len(),
        rotations in proptest::collection::vec(any::<usize>(), 1..6),
        reverse in any::<bool>(), split in 1usize..12, name in "[a-z]{1,24}",
    ) {
        let control = &catalogue().basemap_controls[index];
        let expected = expected(control);
        let mut points = points(control);
        for rotation in rotations {
            let length = points.len();
            points.rotate_left(rotation % length);
            if reverse { points.reverse(); }
        }
        let mut store = map_canon::CanonStore::default();
        for fragment in points.chunks(split) { store.insert_border(map_canon::Border(fragment.to_vec())); }
        prop_assert_eq!(exclusion::check_compiled(&store), Err(expected.clone()), "split compiled borders preserve the complete GPL refusal");
        prop_assert_eq!(map_compile::compile::append_ways(&mut store, &[]), Err(format!("excluded output: {expected:?}")), "compiler output admission refuses renamed GPL descendants");
        let row = PolityRow {
            id: name.clone(), name: name.clone(), from_year: -1446, to_year: -1399,
            rings: points.chunks(split).map(|part| part.iter().map(UnitVec::to_lat_lon_deg).collect()).collect(),
            color_key: None, transition_verses: vec![], fall_verses: vec![],
        };
        prop_assert_eq!(map_compile::partition_bridge::gather_witnesses(&[row]).err(), Some(format!("excluded input: {expected:?}")), "nonempty vendored polity inputs refuse the complete GPL ancestry before partitioning");
        let mut timeline = map_adapters::promised_land_timeline();
        timeline.boundaries.values_mut().next().expect("a survey boundary exists").versions[0].1.pts = points;
        let mut compiled = map_canon::CanonStore::default();
        let result = map_compile::timeline_bridge::bridge_timeline_regions(&mut compiled, &timeline, map_canon::LayerKind::ScriptureClaims, map_canon::Witness::Authored, &name);
        prop_assert_eq!(result, Err(format!("excluded input: {expected:?}")), "the timeline compiler refuses content-preserving GPL derivations");
        prop_assert_eq!(compiled, map_canon::CanonStore::default(), "a refused GPL timeline leaves the complete compiler output unchanged");
    }

    #[test]
    fn permitted_geometry_keeps_its_identity_independently_of_excluded_names(
        latitude in -60f64..60f64, longitude in -160f64..160f64,
        name in prop_oneof![Just("canaan"), Just("judea"), Just("egypt")],
    ) {
        let points: Vec<_> = [(latitude, longitude), (latitude + 1.0, longitude), (latitude + 1.0, longitude + 1.0), (latitude, longitude + 1.0)]
            .into_iter().map(|(lat, lon)| UnitVec::from_lat_lon_deg(lat, lon)).collect();
        prop_assert_eq!(exclusion::check_points(&points), Ok(()), "independently permitted geometry survives with biblical identity {}", name);
        let mut store = map_canon::CanonStore::default();
        store.insert_border(map_canon::Border(points));
        prop_assert_eq!(exclusion::check_compiled(&store), Ok(()), "permitted compiled geometry is not excluded by an entity name");
    }
}

fn expected(control: &Control) -> ExclusionError {
    ExclusionError::Excluded(
        control
            .excluded
            .iter()
            .map(|index| catalogue().geometries[*index].clone())
            .collect(),
    )
}

fn points(control: &Control) -> Vec<UnitVec> {
    let geometry = &originals()[&control.origin]["features"][control.feature]["geometry"];
    let polygons = match geometry["type"]
        .as_str()
        .expect("source geometry has a type")
    {
        "MultiPolygon" => geometry["coordinates"]
            .as_array()
            .expect("source polygons exist")
            .clone(),
        "Polygon" => vec![geometry["coordinates"].clone()],
        other => panic!("unexpected source geometry: {other}"),
    };
    let mut points = Vec::new();
    for polygon in polygons {
        for ring in polygon.as_array().expect("source rings exist") {
            let coordinates: Vec<_> = ring
                .as_array()
                .expect("source positions exist")
                .iter()
                .map(|point| {
                    let mut lon = point[0].as_f64().expect("source longitude");
                    let mut lat = point[1].as_f64().expect("source latitude");
                    if control.method == Transform::Snapped {
                        lon = (lon / 0.02).round() * 0.02;
                        lat = (lat / 0.02).round() * 0.02;
                    }
                    (lon, lat)
                })
                .collect();
            match control.method {
                Transform::Raw => points.extend(
                    coordinates
                        .into_iter()
                        .map(|(lon, lat)| UnitVec::from_lat_lon_deg(lat, lon)),
                ),
                Transform::Quantized | Transform::Snapped => {
                    if let Some(ring) = map_adapters::quantize::clean_ring(&coordinates) {
                        points.extend(ring.into_iter().map(|point| point.to_unit_vec()));
                    }
                }
            }
        }
    }
    points
}

fn catalogue() -> &'static Catalogue {
    static CATALOGUE: OnceLock<Catalogue> = OnceLock::new();
    CATALOGUE.get_or_init(|| {
        serde_json::from_str(include_str!(
            "../../../data/authored/excluded-geometry-fingerprints.json"
        ))
        .expect("the independently checked catalogue and full outcomes decode")
    })
}

fn originals() -> &'static BTreeMap<String, serde_json::Value> {
    static ORIGINALS: OnceLock<BTreeMap<String, serde_json::Value>> = OnceLock::new();
    ORIGINALS.get_or_init(|| {
        let paths = Command::new("git")
            .args([
                "ls-tree",
                "-r",
                "--name-only",
                BASE,
                "--",
                "data/historical-basemaps",
            ])
            .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
            .output()
            .expect("the original source inventory is readable");
        assert!(
            paths.status.success(),
            "the source inventory exists in history"
        );
        String::from_utf8(paths.stdout)
            .expect("source paths are UTF-8")
            .lines()
            .filter(|path| path.ends_with(".geojson"))
            .map(|path| {
                let output = Command::new("git")
                    .args(["show", &format!("{BASE}:{path}")])
                    .current_dir(env!("CARGO_MANIFEST_DIR"))
                    .output()
                    .expect("the GPL original exists in history");
                assert!(
                    output.status.success(),
                    "the inventoried GPL file exists in history"
                );
                (
                    path.to_string(),
                    serde_json::from_slice(&output.stdout).expect("the historical GeoJSON decodes"),
                )
            })
            .collect()
    })
}

#[derive(Deserialize)]
struct Catalogue {
    geometries: Vec<ExcludedGeometry>,
    basemap_controls: Vec<Control>,
}

#[derive(Debug, Deserialize)]
struct Control {
    origin: String,
    feature: usize,
    method: Transform,
    excluded: Vec<usize>,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
enum Transform {
    #[serde(rename = "raw")]
    Raw,
    #[serde(rename = "quantized")]
    Quantized,
    #[serde(rename = "snap-0.02")]
    Snapped,
}
