use map_compile::exclusion;
use serde_json::{json, Value};

#[test]
fn nested_ne_metadata_and_wrapped_split_osm_geometry_are_refused() {
    let catalogue: Value = serde_json::from_str(include_str!("../../../data/authored/excluded-geometry-fingerprints.json")).unwrap();
    let record = catalogue["geometries"].as_array().unwrap().iter().find(|row| row["source"] == "OsmRivers" && row["coordinates"].as_array().unwrap().len() == 96).unwrap();
    let coordinates = record["coordinates"].as_array().unwrap();
    let original = json!({"type":"LineString", "coordinates":coordinates});
    let expected = exclusion::check_geojson(&original).expect_err("the original excluded course is refused");
    let parts: Vec<_> = coordinates.windows(2).map(|pair| json!({"type":"GeometryCollection", "geometries":[{"type":"LineString", "coordinates":pair.iter().map(|point| json!([point[0].as_f64().unwrap()+360.0,point[1]])).collect::<Vec<_>>()}]})).collect();
    let evasion = json!({"type":"Feature", "properties":{"source":"Natural Earth", "license":"Public Domain"}, "geometry":{"type":"GeometryCollection","geometries":parts}});
    assert_eq!(exclusion::check_geojson(&evasion), Err(expected), "nested two-point features and wrapped longitudes preserve the complete excluded-course refusal despite permitted metadata");
    let permitted = json!({"type":"LineString", "coordinates":catalogue["permitted"][0]["coordinates"][0]});
    assert_eq!(exclusion::check_geojson(&permitted), Ok(()), "the pinned Natural Earth course remains admitted through the same door");
}
