#[test]
#[ignore]
fn reviewer_single_vertex_deletion_is_refused() {
    let source: serde_json::Value = serde_json::from_str(&historical("data/wikimedia/tribes12.geojson")).expect("historical source decodes");
    let ring = source["features"][0]["geometry"]["coordinates"][0].as_array().expect("outer ring exists");
    let removed = ring.iter().enumerate().find(|(_, point)| ring.iter().filter(|candidate| *candidate == *point).count() == 1).expect("unique vertex exists").0;
    let points: Vec<_> = ring.iter().enumerate().filter(|(index, _)| *index != removed).map(|(_, point)| UnitVec::from_lat_lon_deg(point[1].as_f64().expect("latitude"), point[0].as_f64().expect("longitude"))).collect();
    let expected = expected_historical_run(&points);
    let result = map_compile::partition_bridge::gather_witnesses(&[polity(&points, "review-deletion".into())]);
    println!("removed_index={removed} input_vertices={} refused={}", points.len(), result.is_err());
    assert_eq!(result.err(), Some(format!("excluded input: {expected:?}")), "deleting one unique Judah vertex still reports the complete excluded ring and span");
}

#[test]
#[ignore]
fn reviewer_densification_is_refused() {
    let original = &originals()[0].points;
    let mut points = Vec::new();
    for pair in original.windows(2) {
        points.push(pair[0]);
        points.push(UnitVec::normalize(pair[0].x() + pair[1].x(), pair[0].y() + pair[1].y(), pair[0].z() + pair[1].z()).expect("spherical midpoint exists"));
    }
    points.push(*original.last().expect("closed endpoint"));
    let row = polity(&points, "review-densified".into());
    let expected: Vec<_> = row.rings[0].iter().map(|(lat, lon)| UnitVec::from_lat_lon_deg(*lat, *lon)).collect();
    let result = map_compile::partition_bridge::gather_witnesses(&[row]);
    if let Ok((regions, _)) = &result {
        let admitted = regions.iter().find(|region| region.id == "review-densified@-1000").expect("renamed descendant exists");
        assert_eq!(admitted.rings, vec![expected], "the actual compiler admits every original Judah vertex and inserted spherical midpoint unchanged");
        println!("original_vertices={} admitted_vertices={} content_guard={:?}", original.len(), admitted.rings[0].len(), exclusion::check_points(&points));
    }
    assert!(result.is_err(), "inserting exact edge midpoints must not erase excluded Judah ancestry");
}

#[test]
#[ignore]
fn reviewer_split_features_are_refused() {
    let original = &originals()[0].points;
    let (x, y, z) = original.iter().fold((0.0, 0.0, 0.0), |(x, y, z), point| (x + point.x(), y + point.y(), z + point.z()));
    let center = UnitVec::normalize(x, y, z).expect("fan center exists");
    let rows: Vec<_> = original.windows(2).enumerate().map(|(index, pair)| polity(&[pair[0], pair[1], center, pair[0]], format!("review-split-{index}"))).collect();
    let value = serde_json::json!({"type":"FeatureCollection", "features":rows.iter().map(|row| serde_json::json!({"type":"Feature", "properties":{"name":row.name}, "geometry":{"type":"Polygon", "coordinates":[row.rings[0].iter().map(|(lat,lon)| [*lon,*lat]).collect::<Vec<_>>()]}})).collect::<Vec<_>>()});
    println!("split_features={} geojson_guard={:?}", rows.len(), exclusion::check_geojson(&value));
    let result = map_compile::partition_bridge::gather_witnesses(&rows);
    if let Ok((regions, _)) = &result {
        let actual: Vec<_> = regions.iter().filter(|region| region.id.starts_with("review-split-")).map(|region| region.rings.clone()).collect();
        let expected: Vec<_> = rows.iter().map(|row| vec![row.rings[0].iter().map(|(lat,lon)| UnitVec::from_lat_lon_deg(*lat,*lon)).collect::<Vec<_>>()]).collect();
        assert_eq!(actual, expected, "the real compiler admits all split polygons and every original Judah edge unchanged");
        println!("admitted_features={} original_edges={}", actual.len(), original.len()-1);
    }
    assert!(result.is_err(), "splitting Judah into polygon features must not erase excluded ancestry");
}

#[test]
#[ignore]
fn reviewer_split_long_runs_are_refused() {
    let original = &originals()[0].points;
    let rows: Vec<_> = original.chunks(50).enumerate().filter(|(_, points)| points.len() >= 3).map(|(index, points)| polity(points, format!("review-long-{index}"))).collect();
    let expected = expected_historical_run(&rows[0].rings[0].iter().map(|(lat,lon)| UnitVec::from_lat_lon_deg(*lat,*lon)).collect::<Vec<_>>());
    assert_eq!(map_compile::partition_bridge::gather_witnesses(&rows).err(), Some(format!("excluded input: {expected:?}")), "splitting a ring across features that retain three-vertex runs preserves the complete refusal");
}

#[test]
#[ignore]
fn reviewer_permitted_geometry_survives() {
    let points = permitted();
    let row = polity(&points, "judah".into());
    let expected: Vec<_> = row.rings[0].iter().map(|(lat,lon)| UnitVec::from_lat_lon_deg(*lat,*lon)).collect();
    let (regions, _) = map_compile::partition_bridge::gather_witnesses(&[row]).expect("permitted geometry enters the real compiler");
    assert_eq!(regions.iter().find(|region| region.id == "judah@-1000").expect("Judah replacement exists").rings, vec![expected], "permitted independently authored Judah geometry remains unchanged");
}
