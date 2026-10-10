#[derive(Clone, Copy, Debug)]
enum ExcludedSource {
    KnowingTheBible,
    Tribes12,
    SplicedRegions,
}

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

impl ExcludedSource {
    fn witness_ids(self) -> &'static [&'static str] {
        match self {
            Self::KnowingTheBible => &["canaan"],
            Self::Tribes12 => &[
                "judah",
                "simeon",
                "benjamin",
                "ephraim",
                "manasseh-west",
                "dan",
                "issachar",
                "zebulun",
                "asher",
                "naphtali",
                "reuben",
                "gad",
                "manasseh-east",
            ],
            Self::SplicedRegions => &["philistia", "phoenicia", "geshur", "ammon", "moab", "edom"],
        }
    }
}

#[test]
fn excluded_lineage_never_enters_the_partition() {
    let (regions, _) =
        map_compile::partition_bridge::gather_witnesses(&[]).expect("partition sources load");
    for source in [
        ExcludedSource::KnowingTheBible,
        ExcludedSource::Tribes12,
        ExcludedSource::SplicedRegions,
    ] {
        for id in source.witness_ids() {
            assert!(
                regions.iter().all(|region| region.id != *id),
                "excluded {source:?} witness {id} never enters the partition"
            );
        }
    }
}

#[test]
fn excluded_lineage_never_enters_the_scripture_timeline() {
    let timeline = map_adapters::scripture_timeline();
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
