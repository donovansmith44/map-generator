use std::process::Command;

use map_compile::vendor::PolityRow;
use map_types::UnitVec;

#[test]
#[ignore]
fn original_judah_med_restoration_is_refused() {
    let root = std::env::temp_dir().join(format!("review-x0-med-{}", std::process::id()));
    let directory = root.join("data/natural-earth");
    std::fs::create_dir_all(&directory).expect("own diagnostic directory opens");
    let source = historical_judah();
    let input = serde_json::json!({"type":"FeatureCollection", "features":[{"type":"Feature", "properties":{"name":"renamed-water"}, "geometry":source["features"][0]["geometry"]}]});
    std::fs::write(directory.join("med_clip.geojson"), serde_json::to_vec(&input).expect("diagnostic input serializes")).expect("own diagnostic input writes");
    let output = Command::new(std::env::current_exe().expect("diagnostic executable resolves"))
        .args(["--exact", "med_restoration_child", "--ignored", "--nocapture"])
        .current_dir(&root).output().expect("real loader diagnostic starts");
    std::fs::remove_dir_all(&root).expect("own diagnostic input is removed");
    println!("{}", String::from_utf8_lossy(&output.stdout));
    assert!(output.status.success(), "the real compiler refuses the original full Judah ring through the med input");
}

#[test]
#[ignore]
fn med_restoration_child() {
    let expected = expected_refusal();
    let result = map_compile::partition_bridge::gather_witnesses(&[]);
    assert_eq!(result.err(), Some(expected), "the full restored Judah ring reports its complete excluded-source provenance");
}

#[test]
#[ignore]
fn judah_with_one_unique_vertex_removed_is_refused_as_a_polity() {
    let source = historical_judah();
    let original = source["features"][0]["geometry"]["coordinates"][0].as_array().expect("Judah outer ring exists");
    assert_eq!(original.len(), 156, "the historical source contains every original Judah vertex");
    let removed = original.iter().enumerate().find(|(_, candidate)| original.iter().filter(|point| *point == *candidate).count() == 1).expect("a unique Judah vertex exists").0;
    let ring: Vec<_> = original.iter().enumerate().filter(|(index, _)| *index != removed)
        .map(|(_, point)| (point[1].as_f64().expect("latitude exists"), point[0].as_f64().expect("longitude exists"))).collect();
    let expected_ring: Vec<_> = ring.iter().map(|(lat, lon)| UnitVec::from_lat_lon_deg(*lat, *lon)).collect();
    let row = PolityRow {id:"review-renamed".into(), name:"review-renamed".into(), from_year:-1000, to_year:-900, rings:vec![ring], color_key:None, transition_verses:vec![], fall_verses:vec![]};
    let result = map_compile::partition_bridge::gather_witnesses(&[row]);
    if let Ok((regions, _)) = &result {
        let admitted = regions.iter().find(|region| region.id == "review-renamed@-1000").expect("restored descendant retains its renamed identity");
        assert_eq!(admitted.rings, vec![expected_ring], "the real compiler admits the entire excluded Judah descendant unchanged");
        println!("admitted={} vertices={} removed_index={removed}", admitted.id, admitted.rings[0].len());
    }
    assert_eq!(result.err(), Some(expected_refusal()), "removing one unique vertex must not erase the excluded Judah ancestor");
}

fn historical_judah() -> serde_json::Value {
    let output = Command::new("git").args(["show", "6ac32bfbf67e26b9cfe94560806fda293db05801:data/wikimedia/tribes12.geojson"])
        .current_dir(env!("CARGO_MANIFEST_DIR")).output().expect("historical source opens");
    assert!(output.status.success(), "the excluded source exists at the pinned base");
    serde_json::from_slice(&output.stdout).expect("the historical source decodes")
}

fn expected_refusal() -> String {
    let catalogue: serde_json::Value = serde_json::from_str(include_str!("../../../data/authored/excluded-geometry-fingerprints.json")).expect("catalogue decodes");
    let geometry = serde_json::from_value(catalogue["geometries"][0].clone()).expect("independent complete refusal decodes");
    format!("excluded input: {:?}", map_compile::exclusion::ExclusionError::Excluded(vec![geometry]))
}
