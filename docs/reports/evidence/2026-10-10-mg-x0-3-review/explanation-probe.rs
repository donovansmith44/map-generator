use map_compile::exclusion;
use map_types::UnitVec;
use serde_json::{json, Value};

#[test]
fn permitted_decoys_cannot_explain_resampled_excluded_fragments() {
    let catalogue: Value = serde_json::from_str(include_str!(
        "../../../data/authored/excluded-geometry-fingerprints.json"
    ))
    .unwrap();
    let record = catalogue["geometries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["source"] == "OsmRivers" && row["coordinates"].as_array().unwrap().len() == 96)
        .unwrap();
    let points = record["coordinates"].as_array().unwrap();
    let original = json!({"type":"LineString", "coordinates":points});
    let expected = exclusion::check_geojson(&original)
        .expect_err("the original excluded course supplies a complete refusal control");
    let mut features: Vec<Value> = catalogue["permitted"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| json!({"type":"Feature","properties":{"source":"Natural Earth"},"geometry":{"type":"MultiLineString","coordinates":row["coordinates"]}}))
        .collect();
    let decoys = json!({"type":"FeatureCollection","features":features});
    assert_eq!(exclusion::check_geojson(&decoys), Ok(()), "all genuine Natural Earth decoys remain admitted");
    for pair in points.windows(2).rev() {
        let point = |value: &Value| UnitVec::from_lat_lon_deg(value[1].as_f64().unwrap(), value[0].as_f64().unwrap());
        let a = point(&pair[0]);
        let b = point(&pair[1]);
        let middle = map_types::slerp(&a, &b, 0.5)
            .expect("adjacent source vertices have a nonzero midpoint");
        let (lat, lon) = middle.to_lat_lon_deg();
        features.push(json!({"type":"Feature","properties":{"source":"Natural Earth","license":"Public Domain"},"geometry":{"type":"LineString","coordinates":[[pair[1][0],pair[1][1],0],[lon,lat,0],[pair[0][0],pair[0][1],0]]}}));
    }
    let candidate = json!({"type":"FeatureCollection","features":features});
    assert_eq!(exclusion::check_geojson(&candidate), Err(expected), "permitted decoys cannot explain reversed split midpoint descendants with renamed metadata and altitude coordinates");
}
