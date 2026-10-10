use std::path::Path;
use std::process::Command;

use map_compile::{exclusion, partition_bridge, vendor::PolityRow};
use map_types::UnitVec;

const SOURCE: &str = "cf3c348:data/historical-basemaps/world_bc1500.geojson";
const EGYPT_FEATURE: usize = 13;
const FIRST_RUN: std::ops::Range<usize> = 0..3;

#[test]
fn a_complete_gpl_feature_is_refused() {
    let points = source_ring();
    assert!(exclusion::check_points(&points).is_err(), "the complete historical Egypt ring is refused");
}

#[test]
fn independently_permitted_points_are_admitted() {
    let points: Vec<_> = [(0.123, 0.456), (1.123, 0.456), (1.123, 1.456)]
        .into_iter().map(|(lat, lon)| UnitVec::from_lat_lon_deg(lat, lon)).collect();
    assert_eq!(exclusion::check_points(&points), Ok(()), "independently permitted points pass the content door");
}

#[test]
fn three_consecutive_gpl_vertices_are_refused_as_renamed_input() {
    let points = source_ring()[FIRST_RUN].to_vec();
    let coordinates: Vec<_> = points.iter().map(|point| {
        let (lat, lon) = point.to_lat_lon_deg();
        [lon, lat]
    }).collect();
    let root = std::env::temp_dir().join(format!("x2-review-input-{}", std::process::id()));
    std::fs::create_dir_all(&root).expect("temporary input directory exists");
    let value = serde_json::json!({"type":"Feature","properties":{"name":"permitted-looking"},"geometry":{"type":"LineString","coordinates":coordinates}});
    std::fs::write(root.join("permitted.geojson"), serde_json::to_vec(&value).expect("input serializes")).expect("temporary input is written");
    let actual = exclusion::check_build_inputs(&root);
    std::fs::remove_dir_all(&root).expect("only this probe's temporary directory is removed");
    eprintln!("renamed partial input: {actual:?}");
    assert!(actual.is_err(), "three consecutive GPL vertices are refused before input admission");
}

#[test]
fn three_consecutive_gpl_vertices_are_refused_as_compiled_output() {
    let mut store = map_canon::CanonStore::default();
    store.insert_border(map_canon::Border(source_ring()[FIRST_RUN].to_vec()));
    let actual = exclusion::check_compiled(&store);
    eprintln!("partial compiled border: {actual:?}");
    assert!(actual.is_err(), "three consecutive GPL vertices are refused at the compiled output door");
}

#[test]
fn a_renamed_partial_gpl_polity_is_refused_by_actual_witness_gathering() {
    let row = PolityRow {
        id: "permitted-looking".into(), name: "independent-looking".into(),
        from_year: -1446, to_year: -1399,
        rings: vec![source_ring()[FIRST_RUN].iter().map(UnitVec::to_lat_lon_deg).collect()],
        color_key: None, transition_verses: vec![], fall_verses: vec![],
    };
    let actual = partition_bridge::gather_witnesses(&[row]);
    if let Ok((regions, _)) = &actual {
        let admitted: Vec<_> = regions.iter().filter(|region| region.id == "permitted-looking@-1446").collect();
        eprintln!("actual gathering admitted {} renamed polity with {} GPL vertices", admitted.len(), admitted.iter().flat_map(|region| &region.rings).map(Vec::len).sum::<usize>());
    }
    assert!(actual.is_err(), "actual witness gathering refuses a renamed polity made from three consecutive GPL vertices");
}

fn source_ring() -> Vec<UnitVec> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output = Command::new("git").args(["show", SOURCE]).current_dir(root).output().expect("pinned excluded source is readable for review");
    assert!(output.status.success(), "the review reads the exact historical source");
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("historical source decodes");
    value["features"][EGYPT_FEATURE]["geometry"]["coordinates"][0]
        .as_array().expect("the historical Egypt exterior ring exists").iter()
        .map(|point| UnitVec::from_lat_lon_deg(point[1].as_f64().expect("latitude exists"), point[0].as_f64().expect("longitude exists"))).collect()
}
