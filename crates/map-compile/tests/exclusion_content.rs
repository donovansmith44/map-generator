use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::OnceLock;

use geo::{Destination, Haversine, InterpolatePoint, Point};
use map_compile::exclusion::{
    self, ExcludedGeometry, ExcludedSource, ExcludedStretch, ExclusionError, GeometricPolicy,
    LineageIndex,
};
use map_compile::vendor::PolityRow;
use map_partition::PointKey;
use map_types::UnitVec;
use proptest::prelude::*;

const HISTORICAL_GEOMETRY_COUNT: usize = 107;
const POLICY: GeometricPolicy = GeometricPolicy {
    tolerance_meters: 100.0,
    maximum_unexplained_meters: 2000.0,
    short_line_fraction: 0.1,
};

#[test]
fn reviewer_densification_is_refused() {
    if std::env::var_os("MAPS_X0_ISOLATED_INPUT").is_none() {
        return isolated_input_door("reviewer_densification_is_refused");
    }
    let original = &originals()[0].points;
    let mut points = Vec::new();
    for pair in original.windows(2) {
        points.push(pair[0]);
        points.push(
            UnitVec::normalize(
                pair[0].x() + pair[1].x(),
                pair[0].y() + pair[1].y(),
                pair[0].z() + pair[1].z(),
            )
            .expect("the reviewer's spherical midpoint exists"),
        );
    }
    points.push(*original.last().expect("closed endpoint exists"));
    assert_eq!(
        map_compile::partition_bridge::gather_witnesses(&[polity(
            &points,
            "review-densified".into()
        )])
        .err(),
        Some(format!("excluded input: {:?}", historical_refusal())),
        "inserting the reviewer's spherical midpoints preserves the complete Judah exclusion"
    );
}

#[test]
fn reviewer_split_features_are_refused() {
    if std::env::var_os("MAPS_X0_ISOLATED_INPUT").is_none() {
        return isolated_input_door("reviewer_split_features_are_refused");
    }
    let original = &originals()[0].points;
    let (x, y, z) = original.iter().fold((0.0, 0.0, 0.0), |(x, y, z), point| {
        (x + point.x(), y + point.y(), z + point.z())
    });
    let center = UnitVec::normalize(x, y, z).expect("fan center exists");
    let rows: Vec<_> = original
        .windows(2)
        .enumerate()
        .map(|(index, pair)| {
            polity(
                &[pair[0], pair[1], center, pair[0]],
                format!("review-split-{index}"),
            )
        })
        .collect();
    assert_eq!(
        map_compile::partition_bridge::gather_witnesses(&rows).err(),
        Some(format!("excluded input: {:?}", historical_refusal())),
        "all reviewer's fan features share the complete excluded Judah outcome"
    );
}

#[test]
fn judah_with_one_unique_vertex_removed_is_refused_as_a_polity() {
    if std::env::var_os("MAPS_X0_ISOLATED_INPUT").is_none() {
        return isolated_input_door("judah_with_one_unique_vertex_removed_is_refused_as_a_polity");
    }
    let mut points = originals()[0].points.clone();
    let removed = points
        .iter()
        .position(|point| {
            points
                .iter()
                .filter(|candidate| *candidate == point)
                .count()
                == 1
        })
        .expect("unique vertex exists");
    points.remove(removed);
    assert_eq!(
        map_compile::partition_bridge::gather_witnesses(&[polity(
            &points,
            "review-deletion".into()
        )])
        .err(),
        Some(format!("excluded input: {:?}", historical_refusal())),
        "removing a unique Judah vertex preserves the complete excluded geometry outcome"
    );
}

#[test]
fn all_historical_geometries_are_recorded_in_the_negative_catalogue() {
    assert_eq!(
        recorded().len(),
        HISTORICAL_GEOMETRY_COUNT,
        "every historical excluded line remains recorded"
    );
    for (geometry, original) in recorded().into_iter().zip(originals()) {
        let index = LineageIndex::new(vec![geometry.clone()], POLICY, &[]);
        assert_eq!(
            index.check_points(&original.points),
            Err(refusal(geometry)),
            "even a two-vertex historical path is refused by its geometric length"
        );
    }
}

#[test]
fn split_geojson_features_are_summed_without_connecting_them() {
    let features: Vec<_> = originals()[0].points.windows(2).map(|pair| serde_json::json!({"type":"Feature","properties":{"name":"renamed","source":"Natural Earth"},"geometry":{"type":"LineString","coordinates":positions(pair)}})).collect();
    assert_eq!(
        exclusion::check_geojson(
            &serde_json::json!({"type":"FeatureCollection","features":features})
        ),
        Err(historical_refusal()),
        "renaming every feature and claiming permitted metadata cannot erase excluded geometry"
    );
}

#[test]
fn compiled_edges_are_summed_across_all_borders() {
    let mut store = map_canon::CanonStore::default();
    for pair in originals()[0].points.windows(2) {
        store.insert_border(map_canon::Border(pair.to_vec()));
    }
    assert_eq!(
        exclusion::check_compiled(&store),
        Err(historical_refusal()),
        "separate compiled borders retain their complete excluded source decision"
    );
    assert_eq!(
        map_compile::compile::append_ways(&mut store, &[]),
        Err(format!("excluded output: {:?}", historical_refusal())),
        "the real compiler output door refuses split excluded borders"
    );
}

#[test]
fn timeline_refusal_preserves_the_complete_output() {
    let mut timeline = map_adapters::scripture_timeline();
    timeline
        .boundaries
        .values_mut()
        .next()
        .expect("survey boundary")
        .versions[0]
        .1
        .pts = resample(&originals()[0].points, 3);
    let mut store = map_canon::CanonStore::default();
    let result = map_compile::timeline_bridge::bridge_timeline_regions(
        &mut store,
        &timeline,
        map_canon::LayerKind::ScriptureClaims,
        map_canon::Witness::Authored,
        "renamed",
    );
    assert_eq!(
        result,
        Err(format!("excluded input: {:?}", historical_refusal())),
        "the timeline door refuses resampled ancestry before its transforms"
    );
    assert_eq!(
        store,
        map_canon::CanonStore::default(),
        "refused timeline geometry leaves the entire output unchanged"
    );
}

#[test]
fn every_input_file_contributes_to_the_build_total() {
    let root = std::env::temp_dir().join(format!("maps-x0-geometric-files-{}", std::process::id()));
    std::fs::create_dir_all(root.join("nested")).expect("own input directory exists");
    for (index, pair) in originals()[0].points.windows(2).enumerate() {
        let value = serde_json::json!({"type":"LineString","coordinates":positions(pair)});
        std::fs::write(
            root.join("nested")
                .join(format!("fragment-{index}.geojson")),
            serde_json::to_vec(&value).expect("input encodes"),
        )
        .expect("own file writes");
    }
    let result = exclusion::check_build_inputs(&root);
    std::fs::remove_dir_all(&root).expect("own files removed");
    assert_eq!(
        result,
        Err(historical_refusal()),
        "splitting every excluded edge into a separate nested file preserves exclusion"
    );
}

#[test]
fn restored_med_input_is_refused() {
    let root = std::env::temp_dir().join(format!(
        "maps-x0-geometric-restoration-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(root.join("data/natural-earth")).expect("own input directory exists");
    let value = serde_json::json!({"type":"FeatureCollection","features":[{"type":"Feature","geometry":{"type":"Polygon","coordinates":[positions(&originals()[0].points)]},"properties":{"name":"permitted-looking-water"}}]});
    std::fs::write(
        root.join("data/natural-earth/med_clip.geojson"),
        serde_json::to_vec(&value).expect("input encodes"),
    )
    .expect("own file writes");
    let output = Command::new(std::env::current_exe().expect("test executable"))
        .args([
            "--exact",
            "restoration_probe_child",
            "--ignored",
            "--nocapture",
        ])
        .current_dir(&root)
        .output()
        .expect("restoration subprocess finishes");
    std::fs::remove_dir_all(&root).expect("own files removed");
    assert!(
        output.status.success(),
        "the real med input refuses excluded geometry: {}",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
#[ignore]
fn restoration_probe_child() {
    assert_eq!(
        map_compile::partition_bridge::gather_witnesses(&[]).err(),
        Some(format!("excluded input: {:?}", historical_refusal())),
        "the med loader cannot admit the excluded line under a permitted path"
    );
}

#[test]
fn permitted_replacements_survive_the_real_input_door() {
    if std::env::var_os("MAPS_X0_ISOLATED_INPUT").is_none() {
        return isolated_input_door("permitted_replacements_survive_the_real_input_door");
    }
    let points = permitted();
    let rows: Vec<_> = ["judah", "canaan", "moab"]
        .into_iter()
        .map(|name| polity(&points, name.into()))
        .collect();
    let (regions, _) = map_compile::partition_bridge::gather_witnesses(&rows)
        .expect("independent replacements gather");
    for row in rows {
        let expected: Vec<_> = row
            .rings
            .iter()
            .map(|ring| {
                ring.iter()
                    .map(|(lat, lon)| UnitVec::from_lat_lon_deg(*lat, *lon))
                    .collect::<Vec<_>>()
            })
            .collect();
        assert_eq!(
            regions
                .iter()
                .find(|region| region.id == format!("{}@-1000", row.id))
                .expect("replacement survives")
                .rings,
            expected,
            "the biblical identity retains its entire independent geometry"
        );
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    #[test]
    fn generated_resampling_is_refused(points in random_ring(), divisions in 2usize..8) {
        let geometry = generated_geometry(&points);
        prop_assert_eq!(LineageIndex::new(vec![geometry.clone()], POLICY, &[]).check_points(&resample(&points, divisions)), Err(refusal(geometry)), "any generated edge resampling retains the complete exclusion");
    }

    #[test]
    fn generated_splitting_is_refused(points in random_ring(), divisions in 2usize..8) {
        let geometry = generated_geometry(&points);
        let points = resample(&points, divisions);
        let segments: Vec<_> = points.windows(2).collect();
        prop_assert_eq!(LineageIndex::new(vec![geometry.clone()], POLICY, &[]).check_sequences(segments), Err(refusal(geometry)), "all generated split feature lengths contribute to the same excluded line");
    }

    #[test]
    fn generated_reversal_is_refused(mut points in random_ring()) {
        let geometry = generated_geometry(&points);
        points.reverse();
        prop_assert_eq!(LineageIndex::new(vec![geometry.clone()], POLICY, &[]).check_points(&points), Err(refusal(geometry)), "reversal preserves the complete geometric refusal");
    }

    #[test]
    fn generated_single_vertex_deletion_is_refused(mut points in random_ring(), removed in 0usize..5) {
        let geometry = generated_geometry(&points);
        points.remove(removed);
        prop_assert_eq!(LineageIndex::new(vec![geometry.clone()], POLICY, &[]).check_points(&points), Err(refusal(geometry)), "single-vertex deletion retains sufficient excluded segment length");
    }

    #[test]
    fn generated_sub_tolerance_perturbation_is_refused(points in random_ring(), meters in -99.999f64..99.999, bearing in 0.0f64..360.0) {
        let geometry = generated_geometry(&points);
        let moved: Vec<_> = points.iter().map(|point| {
            let (lat, lon) = point.to_lat_lon_deg();
            let point = Haversine.destination(Point::new(lon, lat), bearing, meters);
            UnitVec::from_lat_lon_deg(point.y(), point.x())
        }).collect();
        prop_assert_eq!(LineageIndex::new(vec![geometry.clone()], POLICY, &[]).check_points(&moved), Err(refusal(geometry)), "generated perturbations below the recorded meter tolerance preserve exclusion");
    }

    #[test]
    fn independently_authored_geometry_is_admitted(points in random_ring()) {
        let geometry = generated_geometry(&points);
        let independent: Vec<_> = points.iter().map(|point| { let (lat, lon) = point.to_lat_lon_deg(); UnitVec::from_lat_lon_deg(lat + 10.0, lon) }).collect();
        prop_assert_eq!(LineageIndex::new(vec![geometry], POLICY, &[]).check_points(&independent), Ok(()), "independent geometry in a disjoint region is admitted");
    }

    #[test]
    fn permitted_lineage_explains_generated_shared_geometry(points in random_ring(), divisions in 2usize..8) {
        let geometry = generated_geometry(&points);
        prop_assert_eq!(LineageIndex::new(vec![geometry], POLICY, &[points.clone()]).check_points(&resample(&points, divisions)), Ok(()), "a permitted source explains resampled shared stretches");
    }

    #[test]
    fn sub_tolerance_permitted_geometry_is_admitted(points in random_ring(), meters in -99.999f64..99.999, bearing in 0.0f64..360.0) {
        let geometry = generated_geometry(&points);
        let moved: Vec<_> = points.iter().map(|point| {
            let (lat, lon) = point.to_lat_lon_deg();
            let point = Haversine.destination(Point::new(lon, lat), bearing, meters);
            UnitVec::from_lat_lon_deg(point.y(), point.x())
        }).collect();
        prop_assert_eq!(LineageIndex::new(vec![geometry], POLICY, &[points]).check_points(&moved), Ok(()), "sub-tolerance proximity to a permitted source explains the whole shared stretch");
    }

    #[test]
    fn generated_partition_keys_preserve_the_existing_quantization(latitude in -90.0f64..90.0, longitude in -180.0f64..180.0) {
        let point = UnitVec::from_lat_lon_deg(latitude, longitude);
        let expected: Vec<_> = [point.x(), point.y(), point.z()].into_iter().flat_map(|value| ((value * 1_000_000_000.0).round() as i64).to_be_bytes()).collect();
        prop_assert_eq!(PointKey::partition(&point).bytes().to_vec(), expected, "the single point-key owner preserves all established Cartesian bytes");
    }

    #[test]
    fn catalogue_producer_and_compiler_preserve_complete_generated_geometry(points in random_ring()) {
        let vertices: Vec<_> = points.iter().map(|point| { let (lat, lon) = point.to_lat_lon_deg(); [lat, lon] }).collect();
        let decoded: Vec<_> = vertices.iter().map(|[lat, lon]| UnitVec::from_lat_lon_deg(*lat, *lon)).collect();
        let expected = vec![generated_geometry(&decoded)];
        let input = serde_json::json!({"policy":POLICY,"geometries":[{"source":"Tribes12","geometry":"generated","origin":"law","source_sha256":"generated-source","vertices":vertices}],"permitted":[]});
        let executable = std::env::var_os("QUARANTINE_KEYS_EXECUTABLE").map(std::path::PathBuf::from).unwrap_or_else(|| std::path::PathBuf::from(std::env::var_os("CARGO_TARGET_DIR").expect("shared target configured")).join("debug/examples/quarantine_keys"));
        let mut child = Command::new(executable).stdin(Stdio::piped()).stdout(Stdio::piped()).spawn().expect("owned producer starts");
        child.stdin.take().expect("producer stdin").write_all(&serde_json::to_vec(&input).expect("input encodes")).expect("input writes");
        let output = child.wait_with_output().expect("producer finishes");
        prop_assert!(output.status.success(), "the producer succeeds for generated coordinates");
        let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("output decodes");
        let actual: Vec<ExcludedGeometry> = serde_json::from_value(value["geometries"].clone()).expect("geometry decodes");
        prop_assert_eq!(actual, expected, "producer and compiler preserve the entire generated source geometry");
    }
}

#[test]
fn disconnected_vertices_do_not_invent_segments() {
    let points = &originals()[0].points;
    assert_eq!(
        exclusion::check_sequences(points.chunks(1)),
        Ok(()),
        "separate one-vertex features contribute no invented connecting segments"
    );
}

#[test]
fn a_short_crossing_is_admitted() {
    let excluded = vec![
        UnitVec::from_lat_lon_deg(0.0, -0.1),
        UnitVec::from_lat_lon_deg(0.0, 0.1),
    ];
    let crossing = vec![
        UnitVec::from_lat_lon_deg(-0.1, 0.0),
        UnitVec::from_lat_lon_deg(0.1, 0.0),
    ];
    assert_eq!(
        LineageIndex::new(vec![generated_geometry(&excluded)], POLICY, &[]).check_points(&crossing),
        Ok(()),
        "only the short length inside the buffer counts at a perpendicular crossing"
    );
}

#[test]
fn shorter_excluded_lines_use_the_fractional_threshold() {
    let start = Point::new(0.0, 0.0);
    let points = meter_line(start, 300.0);
    let geometry = generated_geometry(&points);
    let index = LineageIndex::new(vec![geometry.clone()], POLICY, &[]);
    assert_eq!(
        index.check_points(&meter_line(start, 20.0)),
        Ok(()),
        "twenty meters is below ten percent of the three-hundred-meter excluded line"
    );
    assert_eq!(
        index.check_points(&meter_line(start, 40.0)),
        Err(refusal(geometry)),
        "forty meters exceeds the shorter line's thirty-meter threshold"
    );
}

#[test]
fn repeated_features_are_summed_instead_of_dissolved() {
    let points = meter_line(Point::new(0.0, 0.0), 30000.0);
    let geometry = generated_geometry(&points);
    let fragment = meter_line(Point::new(0.0, 0.0), 800.0);
    let index = LineageIndex::new(vec![geometry.clone()], POLICY, &[]);
    assert_eq!(
        index.check_points(&fragment),
        Ok(()),
        "one eight-hundred-meter fragment is below the two-kilometer threshold"
    );
    assert_eq!(
        index.check_sequences([
            fragment.as_slice(),
            fragment.as_slice(),
            fragment.as_slice()
        ]),
        Err(refusal(geometry)),
        "three separate feature occurrences contribute twenty-four hundred meters"
    );
}

#[test]
fn an_unexplained_tail_is_refused_even_when_the_rest_has_permitted_lineage() {
    let start = Point::new(0.0, 0.0);
    let points = meter_line(start, 10000.0);
    let geometry = generated_geometry(&points);
    let permitted = meter_line(start, 1000.0);
    assert_eq!(
        LineageIndex::new(vec![geometry.clone()], POLICY, &[permitted]).check_points(&points),
        Err(refusal(geometry)),
        "permitted explanation removes only its own stretch and leaves the excluded tail refused"
    );
}

#[test]
fn input_and_output_fragments_contribute_to_one_build_total() {
    const INLAND_JUDAH_SEGMENT_START: usize = 6;
    const FRAGMENT_METERS: f64 = 1500.0;
    let original = &originals()[0].points;
    let (lat, lon) = original[INLAND_JUDAH_SEGMENT_START].to_lat_lon_deg();
    let (end_lat, end_lon) = original[INLAND_JUDAH_SEGMENT_START + 1].to_lat_lon_deg();
    let end = Haversine.point_at_distance_between(
        Point::new(lon, lat),
        Point::new(end_lon, end_lat),
        FRAGMENT_METERS,
    );
    let fragment = vec![
        original[INLAND_JUDAH_SEGMENT_START],
        UnitVec::from_lat_lon_deg(end.y(), end.x()),
    ];
    let root = std::env::temp_dir().join(format!("maps-x0-geometric-total-{}", std::process::id()));
    std::fs::create_dir_all(&root).expect("own input directory exists");
    std::fs::write(
        root.join("input.geojson"),
        serde_json::to_vec(
            &serde_json::json!({"type":"LineString","coordinates":positions(&fragment)}),
        )
        .expect("input encodes"),
    )
    .expect("own file writes");
    let input = exclusion::check_build_inputs(&root);
    let output = exclusion::check_sequences([fragment.as_slice()]);
    let combined = exclusion::check_inputs_and_sequences(&root, [fragment.as_slice()]);
    std::fs::remove_dir_all(&root).expect("own input directory removed");
    assert_eq!(
        input,
        Ok(()),
        "the fifteen-hundred-meter input alone is below the build threshold"
    );
    assert_eq!(
        output,
        Ok(()),
        "the fifteen-hundred-meter output alone is below the build threshold"
    );
    assert_eq!(
        combined,
        Err(historical_refusal()),
        "input and output jointly exceed the threshold for the same excluded Judah line"
    );
}

#[test]
fn all_recorded_natural_earth_lines_are_admitted() {
    let value: serde_json::Value = serde_json::from_str(include_str!(
        "../../../data/authored/excluded-geometry-fingerprints.json"
    ))
    .expect("catalogue decodes");
    let coordinates: Vec<_> = value["permitted"]
        .as_array()
        .expect("permitted sources exist")
        .iter()
        .flat_map(|source| {
            source["coordinates"]
                .as_array()
                .expect("source lines exist")
                .iter()
                .cloned()
        })
        .collect();
    assert_eq!(exclusion::check_geojson(&serde_json::json!({"type":"MultiLineString","coordinates":coordinates})), Ok(()), "every pinned Natural Earth coast river lake and retained clip has permitted geometric lineage");
}

#[test]
fn every_natural_earth_input_has_permitted_lineage() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/natural-earth");
    let mut paths: Vec<_> = std::fs::read_dir(root)
        .expect("native sources exist")
        .map(|entry| entry.expect("source entry exists").path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "geojson")
        })
        .collect();
    paths.sort();
    for path in paths {
        let value: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).expect("native source reads"))
                .expect("native geometry decodes");
        assert_eq!(
            exclusion::check_geojson(&value),
            Ok(()),
            "all geometry from the permitted native Natural Earth input {} is admitted",
            path.display()
        );
    }
}

fn isolated_input_door(probe: &str) {
    let root =
        std::env::temp_dir().join(format!("maps-x0-isolated-{}-{probe}", std::process::id()));
    let native = root.join("data/natural-earth");
    std::fs::create_dir_all(&native).expect("own source directory exists");
    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/natural-earth");
    for entry in std::fs::read_dir(source).expect("permitted native inputs exist") {
        let path = entry.expect("native input entry exists").path();
        if path.is_file() {
            std::fs::copy(
                &path,
                native.join(path.file_name().expect("native filename exists")),
            )
            .expect("permitted input copies without links");
        }
    }
    std::fs::create_dir_all(root.join("data/osm")).expect("own empty-source directory exists");
    std::fs::write(
        root.join("data/osm/rivers.geojson"),
        br#"{"type":"FeatureCollection","features":[]}"#,
    )
    .expect("explicit empty test input writes");
    let output = Command::new(std::env::current_exe().expect("test executable exists"))
        .args(["--exact", probe, "--nocapture"])
        .env("MAPS_X0_ISOLATED_INPUT", "1")
        .current_dir(&root)
        .output()
        .expect("isolated actual input door finishes");
    std::fs::remove_dir_all(root).expect("own copied input directory removed");
    assert!(output.status.success(), "the actual input-door probe {probe} succeeds in a build containing only its supplied geometry and permitted native inputs: {} {}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
}

fn random_ring() -> impl Strategy<Value = Vec<UnitVec>> {
    (10.0f64..40.0, 10.0f64..40.0, 0.05f64..0.3, 0.05f64..0.3).prop_map(
        |(lat, lon, height, width)| {
            vec![
                UnitVec::from_lat_lon_deg(lat, lon),
                UnitVec::from_lat_lon_deg(lat, lon + width),
                UnitVec::from_lat_lon_deg(lat + height, lon + width),
                UnitVec::from_lat_lon_deg(lat + height, lon),
                UnitVec::from_lat_lon_deg(lat, lon),
            ]
        },
    )
}

fn resample(points: &[UnitVec], divisions: usize) -> Vec<UnitVec> {
    let mut sampled = Vec::new();
    for pair in points.windows(2) {
        let (lat, lon) = pair[0].to_lat_lon_deg();
        let (end_lat, end_lon) = pair[1].to_lat_lon_deg();
        for step in 0..divisions {
            let point = Haversine.point_at_ratio_between(
                Point::new(lon, lat),
                Point::new(end_lon, end_lat),
                step as f64 / divisions as f64,
            );
            sampled.push(UnitVec::from_lat_lon_deg(point.y(), point.x()));
        }
    }
    sampled.push(*points.last().expect("sampled line is nonempty"));
    sampled
}

fn meter_line(start: Point, length: f64) -> Vec<UnitVec> {
    let end = Haversine.destination(start, 90.0, length);
    vec![
        UnitVec::from_lat_lon_deg(start.y(), start.x()),
        UnitVec::from_lat_lon_deg(end.y(), end.x()),
    ]
}

fn positions(points: &[UnitVec]) -> Vec<[f64; 2]> {
    points
        .iter()
        .map(|point| {
            let (lat, lon) = point.to_lat_lon_deg();
            [lon, lat]
        })
        .collect()
}

fn generated_geometry(points: &[UnitVec]) -> ExcludedGeometry {
    ExcludedGeometry::from_points(
        ExcludedSource::Tribes12,
        "generated".into(),
        "law".into(),
        "generated-source".into(),
        points,
    )
}

fn refusal(geometry: ExcludedGeometry) -> ExclusionError {
    ExclusionError::Excluded(ExcludedStretch {
        geometry,
        policy: POLICY,
    })
}

fn historical_refusal() -> ExclusionError {
    refusal(recorded().remove(0))
}

fn recorded() -> Vec<ExcludedGeometry> {
    #[derive(serde::Deserialize)]
    struct Catalogue {
        geometries: Vec<ExcludedGeometry>,
    }
    static RECORDED: std::sync::OnceLock<Vec<ExcludedGeometry>> = std::sync::OnceLock::new();
    RECORDED
        .get_or_init(|| {
            let catalogue: Catalogue = serde_json::from_str(include_str!(
                "../../../data/authored/excluded-geometry-fingerprints.json"
            ))
            .expect("catalogue decodes");
            catalogue
                .geometries
                .into_iter()
                .filter(|geometry| {
                    matches!(
                        geometry.source,
                        ExcludedSource::KnowingTheBible
                            | ExcludedSource::Tribes12
                            | ExcludedSource::SplicedRegions
                    )
                })
                .collect()
        })
        .clone()
}
#[derive(Clone, Debug)]
struct Original {
    points: Vec<UnitVec>,
}

fn historical(path: &str) -> String {
    let output = Command::new("git")
        .args([
            "show",
            &format!("6ac32bfbf67e26b9cfe94560806fda293db05801:{path}"),
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("recorded quarantine base is readable");
    assert!(
        output.status.success(),
        "the recorded quarantine source exists in history"
    );
    String::from_utf8(output.stdout).expect("quarantine source is UTF-8")
}

fn originals() -> &'static [Original] {
    static ORIGINALS: OnceLock<Vec<Original>> = OnceLock::new();
    ORIGINALS.get_or_init(|| {
        let mut originals = Vec::new();
        for path in ["data/wikimedia/tribes12.geojson", "data/openbible/regions.geojson"] {
            let value: serde_json::Value = serde_json::from_str(&historical(path)).expect("historical GeoJSON parses");
            for feature in value["features"].as_array().expect("features exist").iter() {
                let geometry = &feature["geometry"];
                let polygons = if geometry["type"] == "MultiPolygon" {
                    geometry["coordinates"].as_array().expect("polygons exist").clone()
                } else {
                    vec![geometry["coordinates"].clone()]
                };
                for polygon in polygons {
                    for ring in polygon.as_array().expect("rings exist") {
                        originals.push(Original {
                            points: ring.as_array().expect("positions exist").iter().map(|position| {
                                UnitVec::from_lat_lon_deg(position[1].as_f64().expect("latitude"), position[0].as_f64().expect("longitude"))
                            }).collect(),
                        });
                    }
                }
            }
        }
        for (path, chart) in [
            ("crates/map-adapters/src/surveys.rs", None),
            ("crates/map-adapters/src/plate_water.rs", Some(map_types::chart::Chart::new(
                [[7.423165730975e-04, -1.392245320025e-05], [-1.314773386506e-05, -6.519117998042e-04]],
                [3.354108963464e+01, 3.407261055596e+01],
                (0.0, 0.0, 4500.0, 6000.0),
            ).expect("the historical plate chart is invertible"))),
        ] {
            let file = syn::parse_file(&historical(path)).expect("historical Rust parses");
            for item in file.items {
                let syn::Item::Const(item) = item else { continue };
                if !item.ident.to_string().starts_with("PLATE_") { continue }
                let syn::Expr::Reference(reference) = *item.expr else { continue };
                let syn::Expr::Array(array) = *reference.expr else { continue };
                let mut points = Vec::new();
                for entry in array.elems {
                    match entry {
                        syn::Expr::Tuple(tuple) => {
                            let values: Vec<_> = tuple.elems.iter().map(number).collect();
                            points.push(chart.expect("pixel geometry has its chart").to_sphere(values[0], values[1]).expect("historical vertex is on the chart"));
                        }
                        syn::Expr::Struct(waypoint) => {
                            let field = |name: &str| waypoint.fields.iter().find(|field| matches!(&field.member, syn::Member::Named(id) if id == name)).expect("waypoint field exists");
                            points.push(UnitVec::from_lat_lon_deg(number(&field("lat").expr), number(&field("lon").expr)));
                        }
                        _ => panic!("historical geometry has a tuple or waypoint"),
                    }
                }
                if !points.is_empty() {
                    originals.push(Original { points });
                }
            }
        }
        originals
    })
}

fn number(expression: &syn::Expr) -> f64 {
    match expression {
        syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Float(value),
            ..
        }) => value.base10_parse().expect("historical float"),
        syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Int(value),
            ..
        }) => value.base10_parse().expect("historical integer"),
        syn::Expr::Unary(value) if matches!(value.op, syn::UnOp::Neg(_)) => -number(&value.expr),
        _ => panic!("historical coordinate is numeric"),
    }
}

fn polity(points: &[UnitVec], name: String) -> PolityRow {
    PolityRow {
        id: name.clone(),
        name,
        from_year: -1000,
        to_year: -900,
        rings: vec![points.iter().map(UnitVec::to_lat_lon_deg).collect()],
        color_key: None,
        transition_verses: vec![],
        fall_verses: vec![],
    }
}

fn permitted() -> Vec<UnitVec> {
    [(0.0, 0.0), (0.0, 1.0), (1.0, 1.0), (1.0, 0.0)]
        .into_iter()
        .map(|(lat, lon)| UnitVec::from_lat_lon_deg(lat, lon))
        .collect()
}
