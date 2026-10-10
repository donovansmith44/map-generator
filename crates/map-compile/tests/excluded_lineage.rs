#[test]
fn excluded_lineage_is_removed_from_partition_provenance() {
    assert_eq!(
        map_canon::Witness::PARTITION_INPUTS.as_slice(),
        [
            map_canon::Witness::Atlas,
            map_canon::Witness::NaturalEarth,
            map_canon::Witness::Osm,
        ],
        "partition provenance lists exactly its retained source families"
    );
}

#[test]
fn excluded_lineage_never_enters_the_partition() {
    let (regions, polylines) =
        map_compile::partition_bridge::gather_witnesses(&[]).expect("partition sources load");
    assert_eq!(
        map_compile::exclusion::check_sequences(
            regions
                .iter()
                .flat_map(|region| &region.rings)
                .map(Vec::as_slice)
                .chain(polylines.iter().map(|line| line.pts.as_slice())),
        ),
        Ok(()),
        "every admitted partition geometry is free of quarantined content"
    );
}

#[test]
fn excluded_lineage_never_enters_the_scripture_timeline() {
    let timeline = map_adapters::scripture_timeline();
    assert_eq!(
        map_compile::exclusion::check_timeline(&timeline),
        Ok(()),
        "every Scripture boundary is free of quarantined content"
    );
    for history in timeline.boundaries.values() {
        for (_, boundary) in &history.versions {
            assert!(
                !boundary.provenance.contains("plate-trace"),
                "excluded plate tracing never enters a Scripture boundary"
            );
            if let map_types::BoundarySource::Survey(survey) = &boundary.source {
                for waypoint in &survey.waypoints {
                    assert!(
                        !waypoint.0 .0.contains("canaan-contour"),
                        "excluded contour markers never become survey places"
                    );
                }
            }
        }
    }
}
