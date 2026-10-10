use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::OnceLock;

use map_compile::exclusion::{
    self, ExcludedGeometry, ExcludedRun, ExcludedSource, ExclusionError, LineageIndex, RunDirection,
};
use map_compile::vendor::PolityRow;
use map_partition::PointKey;
use map_types::UnitVec;
use proptest::prelude::*;

const HISTORICAL_GEOMETRY_COUNT: usize = 107;
const JUDAH_VERTEX_COUNT: usize = 156;

#[test]
fn judah_with_one_unique_vertex_removed_is_refused_as_a_polity() {
    let source: serde_json::Value =
        serde_json::from_str(&historical("data/wikimedia/tribes12.geojson"))
            .expect("historical Judah decodes");
    let original = source["features"][0]["geometry"]["coordinates"][0]
        .as_array()
        .expect("Judah outer ring exists");
    assert_eq!(
        original.len(),
        JUDAH_VERTEX_COUNT,
        "the historical source contains every original Judah vertex"
    );
    let removed = original
        .iter()
        .enumerate()
        .find(|(_, candidate)| original.iter().filter(|point| *point == *candidate).count() == 1)
        .expect("a unique Judah vertex exists")
        .0;
    let points: Vec<_> = original
        .iter()
        .enumerate()
        .filter(|(index, _)| *index != removed)
        .map(|(_, point)| {
            UnitVec::from_lat_lon_deg(
                point[1].as_f64().expect("latitude"),
                point[0].as_f64().expect("longitude"),
            )
        })
        .collect();
    let expected = expected_historical_run(&points);
    let result = map_compile::partition_bridge::gather_witnesses(&[polity(
        &points,
        "review-renamed".into(),
    )]);
    assert_eq!(
        result.err(),
        Some(format!("excluded input: {expected:?}")),
        "removing one unique vertex reports the excluded Judah ring and matched span"
    );
}

#[test]
fn every_historical_sequence_is_bound_to_its_ordered_content() {
    assert_eq!(
        originals().len(),
        HISTORICAL_GEOMETRY_COUNT,
        "all recorded excluded geometries have an executable content control"
    );
    for original in originals() {
        let expected = independent_historical_match(&original.points);
        assert_eq!(
            exclusion::check_points(&original.points),
            expected,
            "{} reports its complete run refusal or the explicit shorter-than-three control",
            original.name
        );
    }
}

#[test]
fn restored_judah_is_refused_through_another_polity_input() {
    let points = &originals()[0].points;
    let expected = expected_historical_run(points);
    let result = map_compile::partition_bridge::gather_witnesses(&[polity(
        points,
        "renamed-control".into(),
    )]);
    assert_eq!(
        result.err(),
        Some(format!("excluded input: {expected:?}")),
        "a renamed full Judah input is refused with its ring and span"
    );
}

#[test]
fn restored_med_input_is_refused() {
    let root = std::env::temp_dir().join(format!("maps-x0-restoration-{}", std::process::id()));
    let directory = root.join("data/natural-earth");
    std::fs::create_dir_all(&directory).expect("temporary probe directory");
    let source: serde_json::Value =
        serde_json::from_str(&historical("data/wikimedia/tribes12.geojson"))
            .expect("historical Judah parses");
    let data = serde_json::json!({"type":"FeatureCollection","features":[{"type":"Feature","geometry":source["features"][0]["geometry"],"properties":{"name":"permitted-looking-water"}}]});
    std::fs::write(
        directory.join("med_clip.geojson"),
        serde_json::to_vec(&data).expect("probe serializes"),
    )
    .expect("probe input writes");
    let output = Command::new(std::env::current_exe().expect("test executable"))
        .args([
            "--exact",
            "restoration_probe_child",
            "--ignored",
            "--nocapture",
        ])
        .current_dir(&root)
        .output()
        .expect("restoration subprocess");
    std::fs::remove_dir_all(&root).expect("own probe removed");
    assert!(
        output.status.success(),
        "the real med-clip restoration is refused: {}",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
#[ignore]
fn restoration_probe_child() {
    let expected = expected_historical_run(&originals()[0].points);
    let result = map_compile::partition_bridge::gather_witnesses(&[]);
    assert_eq!(
        result.err(),
        Some(format!("excluded input: {expected:?}")),
        "the real med loader reports the excluded ring and span before transformation"
    );
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    #[test]
    fn historical_clips_rotations_reversals_and_deletions_are_refused(
        index in 0usize..HISTORICAL_GEOMETRY_COUNT, start in any::<usize>(), reverse in any::<bool>(), name in "[a-z]{1,24}",
    ) {
        let original = &originals()[index];
        prop_assume!(original.points.len() >= 6);
        let mut points = original.points.clone();
        if PointKey::lineage(points.first().expect("first")) == PointKey::lineage(points.last().expect("last")) { points.pop(); }
        let length = points.len();
        points.rotate_left(start % length);
        if reverse { points.reverse(); }
        points.remove(0);
        let expected = expected_historical_run(&points);
        prop_assert_eq!(exclusion::check_points(&points), Err(expected.clone()), "historical single-vertex deletion retains its complete refusal");
        let clipped = &points[..LineageIndex::REFUSED_RUN_LENGTH];
        prop_assert_eq!(exclusion::check_points(clipped), Err(expected.clone()), "a three-vertex clip alone retains the same ring and span");
        let positions: Vec<_> = clipped.iter().map(|point| { let (lat, lon) = point.to_lat_lon_deg(); serde_json::json!([lon, lat]) }).collect();
        let value = serde_json::json!({"derived":{"renamed":{"geometry":{"coordinates":positions},"name":name}}});
        prop_assert_eq!(exclusion::check_geojson(&value), Err(expected.clone()), "nested renamed GeoJSON refuses the complete partial-descendant outcome");
        let mut store = map_canon::CanonStore::default();
        store.insert_border(map_canon::Border(clipped.to_vec()));
        prop_assert_eq!(exclusion::check_compiled(&store), Err(expected.clone()), "compiled partial output reports its excluded ring and span");
        let result = map_compile::compile::append_ways(&mut store, &[]);
        prop_assert_eq!(result, Err(format!("excluded output: {expected:?}")), "compiler output checking refuses a clipped descendant even without new ways");
        let mut timeline = map_adapters::scripture_timeline();
        prop_assert!(!timeline.boundaries.is_empty(), "the retained Scripture fixture supplies a nonempty timeline control");
        timeline.boundaries.values_mut().next().expect("survey boundary").versions[0].1.pts = clipped.to_vec();
        let mut compiled = map_canon::CanonStore::default();
        let result = map_compile::timeline_bridge::bridge_timeline_regions(&mut compiled, &timeline, map_canon::LayerKind::ScriptureClaims, map_canon::Witness::Authored, &name);
        prop_assert_eq!(result, Err(format!("excluded input: {expected:?}")), "timeline admission refuses the clipped ancestor with its complete provenance");
        prop_assert_eq!(compiled, map_canon::CanonStore::default(), "refused timeline input leaves the entire output unchanged");
        let result = map_compile::partition_bridge::gather_witnesses(&[polity(clipped, name)]);
        prop_assert_eq!(result.err(), Some(format!("excluded input: {expected:?}")), "nonempty renamed polity inputs refuse clipped ancestry before derivation");
    }

    #[test]
    fn generated_random_ring_subsets_are_refused(coordinates in random_ring(), start in any::<usize>(), length in 3usize..30) {
        let points = sphere_points(&coordinates);
        let geometry = generated_geometry(&points);
        let length = length.min(points.len());
        let start = start % points.len();
        let subset: Vec<_> = (0..length).map(|offset| points[(start + offset) % points.len()]).collect();
        let expected = generated_refusal(&geometry, 0, start, RunDirection::Forward);
        prop_assert_eq!(LineageIndex::new(vec![geometry]).check_points(&subset), Err(expected), "every generated contiguous subset of at least three vertices reports its source span");
    }

    #[test]
    fn generated_random_ring_rotations_are_refused(coordinates in random_ring(), start in any::<usize>()) {
        let mut points = sphere_points(&coordinates);
        let geometry = generated_geometry(&points);
        let start = start % points.len();
        points.rotate_left(start);
        let expected = generated_refusal(&geometry, 0, start, RunDirection::Forward);
        prop_assert_eq!(LineageIndex::new(vec![geometry]).check_points(&points), Err(expected), "a generated ring rotation retains the complete original span");
    }

    #[test]
    fn generated_random_ring_reversals_are_refused(coordinates in random_ring(), start in any::<usize>()) {
        let points = sphere_points(&coordinates);
        let geometry = generated_geometry(&points);
        let start = start % points.len();
        let reverse: Vec<_> = (0..points.len()).map(|offset| points[(start + points.len() - offset) % points.len()]).collect();
        let expected = generated_refusal(&geometry, 0, start, RunDirection::Reverse);
        prop_assert_eq!(LineageIndex::new(vec![geometry]).check_points(&reverse), Err(expected), "reversed generated rings report reverse traversal from any starting vertex");
    }

    #[test]
    fn generated_random_ring_single_vertex_deletions_are_refused(coordinates in random_ring(), removed in any::<usize>()) {
        let mut points = sphere_points(&coordinates);
        let geometry = generated_geometry(&points);
        let removed = removed % points.len();
        points.remove(removed);
        let (input_start, ring_start) = if removed < 3 { (removed, removed + 1) } else { (0, 0) };
        let expected = generated_refusal(&geometry, input_start, ring_start, RunDirection::Forward);
        prop_assert_eq!(LineageIndex::new(vec![geometry]).check_points(&points), Err(expected), "deleting any single vertex from a generated ring preserves another forbidden run");
    }

    #[test]
    fn generated_random_ring_sub_tolerance_perturbations_are_refused(
        coordinates in random_ring(), latitude_delta in -0.999f64..0.999, longitude_delta in -0.999f64..0.999,
    ) {
        let points = sphere_points(&coordinates);
        let geometry = generated_geometry(&points);
        let perturbed: Vec<_> = coordinates.iter().map(|(lat, lon)| UnitVec::from_lat_lon_deg(lat + latitude_delta * PointKey::LINEAGE_TOLERANCE_DEGREES, lon + longitude_delta * PointKey::LINEAGE_TOLERANCE_DEGREES)).collect();
        let expected = generated_refusal(&geometry, 0, 0, RunDirection::Forward);
        prop_assert_eq!(LineageIndex::new(vec![geometry]).check_points(&perturbed), Err(expected), "perturbations below one microdegree cannot evade the complete run refusal");
    }

    #[test]
    fn independently_authored_random_geometry_is_admitted(coordinates in random_ring(), authored in random_ring()) {
        let excluded = sphere_points(&coordinates);
        let permitted = sphere_points(&authored.iter().map(|(lat, lon)| (lat + 100.0, *lon)).collect::<Vec<_>>());
        prop_assert_eq!(LineageIndex::new(vec![generated_geometry(&excluded)]).check_points(&permitted), Ok(()), "independently authored geometry outside the excluded latitude band is admitted");
    }

    #[test]
    fn generated_partition_keys_preserve_the_existing_quantization(latitude in -90.0f64..90.0, longitude in -180.0f64..180.0) {
        let point = UnitVec::from_lat_lon_deg(latitude, longitude);
        let expected: Vec<_> = [point.x(), point.y(), point.z()].into_iter().flat_map(|value| ((value * 1_000_000_000.0).round() as i64).to_be_bytes()).collect();
        prop_assert_eq!(PointKey::partition(&point).bytes().to_vec(), expected, "the owning key preserves every generated signed Cartesian byte identity");
    }

    #[test]
    fn catalogue_producer_and_compiler_share_generated_rounding(coordinates in random_ring()) {
        let points = sphere_points(&coordinates);
        let expected = vec![generated_geometry(&points)];
        let input = serde_json::json!([{"source":"Tribes12","geometry":"generated","origin":"law","source_sha256":"generated-source","vertices":coordinates.iter().map(|(lat, lon)| [lat, lon]).collect::<Vec<_>>()}]);
        let executable = std::path::PathBuf::from(std::env::var_os("CARGO_TARGET_DIR").expect("shared target configured")).join("debug/examples/quarantine_keys");
        let mut child = Command::new(executable).stdin(Stdio::piped()).stdout(Stdio::piped()).spawn().expect("owned catalogue producer starts");
        child.stdin.take().expect("producer stdin").write_all(&serde_json::to_vec(&input).expect("input serializes")).expect("producer input writes");
        let output = child.wait_with_output().expect("producer completes");
        prop_assert!(output.status.success(), "the producer completes for generated boundary inputs");
        let actual: Vec<ExcludedGeometry> = serde_json::from_slice(&output.stdout).expect("producer output decodes");
        prop_assert_eq!(actual, expected, "producer and compiler emit identical complete ordered fingerprints through the one point-key owner");
    }

    #[test]
    fn permitted_controls_survive_with_biblical_names(name in prop_oneof![Just("judah"), Just("canaan"), Just("moab")], repeats in 1usize..9) {
        let points: Vec<_> = permitted().into_iter().cycle().take(4 * repeats).collect();
        prop_assert_eq!(exclusion::check_points(&points), Ok(()), "permitted geometry is accepted independently of biblical identities");
        let mut store = map_canon::CanonStore::default();
        store.insert_border(map_canon::Border(points));
        prop_assert_eq!(exclusion::check_compiled(&store), Ok(()), "permitted descendants remain admissible under the name {}", name);
    }
}

#[test]
fn unrelated_sequences_do_not_make_an_excluded_run() {
    let points = &originals()[0].points[..LineageIndex::REFUSED_RUN_LENGTH];
    let coordinates: Vec<_> = points
        .iter()
        .map(|point| {
            let (lat, lon) = point.to_lat_lon_deg();
            serde_json::json!([lon, lat])
        })
        .collect();
    let value = serde_json::json!({"features":coordinates.into_iter().map(|point| serde_json::json!({"geometry":{"coordinates":[point]}})).collect::<Vec<_>>()});
    assert_eq!(
        exclusion::check_geojson(&value),
        Ok(()),
        "three separate one-vertex inputs do not invent a consecutive source run"
    );
    assert_eq!(
        exclusion::check_sequences(points.chunks(1)),
        Ok(()),
        "separate compiled sequences do not invent source continuity"
    );
}

#[test]
fn nonconsecutive_excluded_vertices_are_admitted_under_the_run_ruling() {
    let coordinates = vec![
        (10.0, 10.0),
        (11.0, 11.0),
        (12.0, 12.0),
        (13.0, 13.0),
        (14.0, 14.0),
        (15.0, 15.0),
    ];
    let points = sphere_points(&coordinates);
    let selected = vec![points[0], points[2], points[4]];
    assert_eq!(
        LineageIndex::new(vec![generated_geometry(&points)]).check_points(&selected),
        Ok(()),
        "three nonconsecutive source vertices do not satisfy the ordered-run exclusion ruling"
    );
}

#[test]
fn the_partition_point_key_preserves_the_existing_byte_identity() {
    let point = UnitVec::from_lat_lon_deg(0.0, 0.0);
    let mut expected = [0; 24];
    expected[..8].copy_from_slice(&1_000_000_000i64.to_be_bytes());
    assert_eq!(
        PointKey::partition(&point).bytes(),
        expected,
        "the owned partition key preserves the complete established Cartesian byte encoding"
    );
}

#[test]
fn lineage_rounding_boundaries_and_longitude_seams_preserve_nearby_keys() {
    for latitude in [-30.0000005, 30.0000005] {
        let original = UnitVec::from_lat_lon_deg(latitude, 179.9999999);
        let perturbed = UnitVec::from_lat_lon_deg(latitude + 0.0000009, -179.9999999);
        assert!(
            PointKey::lineage_neighborhood(&perturbed).contains(&PointKey::lineage(&original)),
            "signed half-cell rounding and the longitude seam retain the nearby source key"
        );
    }
}

#[test]
fn excluded_content_in_multiple_input_files_is_refused() {
    let root = std::env::temp_dir().join(format!("maps-x0-multiple-inputs-{}", std::process::id()));
    std::fs::create_dir_all(&root).expect("temporary input directory");
    let points = &originals()[0].points;
    for (index, part) in points.chunks(50).enumerate() {
        let positions: Vec<_> = part
            .iter()
            .map(|point| {
                let (lat, lon) = point.to_lat_lon_deg();
                serde_json::json!([lon, lat])
            })
            .collect();
        let value = serde_json::json!({"geometry":{"type":"LineString","coordinates":positions}});
        std::fs::write(
            root.join(format!("renamed-{index}.geojson")),
            serde_json::to_vec(&value).expect("input serializes"),
        )
        .expect("temporary input file");
    }
    let result = exclusion::check_build_inputs(&root);
    std::fs::remove_dir_all(&root).expect("remove own temporary inputs");
    assert_eq!(
        result,
        Err(expected_historical_run(&points[..50])),
        "a renamed input file containing a partial run reports the complete source and span"
    );
}

#[test]
fn permitted_replacements_survive_the_real_input_door() {
    let points = permitted();
    let rows: Vec<_> = ["judah", "canaan", "moab"]
        .into_iter()
        .map(|name| polity(&points, name.into()))
        .collect();
    let (regions, _) = map_compile::partition_bridge::gather_witnesses(&rows)
        .expect("permitted replacements gather");
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
        let admitted = regions
            .iter()
            .find(|region| region.id == format!("{}@-1000", row.id))
            .expect("permitted replacement survives");
        assert_eq!(
            admitted.rings, expected,
            "a biblical identity retains its complete permitted replacement geometry"
        );
    }
}

fn random_ring() -> impl Strategy<Value = Vec<(f64, f64)>> {
    (
        proptest::collection::btree_set(
            (-60_000_000i32..-10_000_000, -160_000_000i32..160_000_000),
            6..30,
        ),
        -0.5f64..0.5,
    )
        .prop_map(|(values, phase)| {
            values
                .into_iter()
                .map(|(lat, lon)| {
                    (
                        (lat as f64 + phase) * PointKey::LINEAGE_TOLERANCE_DEGREES,
                        (lon as f64 + phase) * PointKey::LINEAGE_TOLERANCE_DEGREES,
                    )
                })
                .collect()
        })
}

fn sphere_points(coordinates: &[(f64, f64)]) -> Vec<UnitVec> {
    coordinates
        .iter()
        .map(|(lat, lon)| UnitVec::from_lat_lon_deg(*lat, *lon))
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

fn generated_refusal(
    geometry: &ExcludedGeometry,
    input_start: usize,
    ring_start: usize,
    direction: RunDirection,
) -> ExclusionError {
    ExclusionError::Excluded(ExcludedRun {
        geometry: geometry.clone(),
        input_start,
        ring_start,
        vertex_count: LineageIndex::REFUSED_RUN_LENGTH,
        direction,
    })
}

fn expected_historical_run(points: &[UnitVec]) -> ExclusionError {
    independent_historical_match(points)
        .expect_err("the independent historical oracle finds a forbidden run")
}

fn independent_historical_match(points: &[UnitVec]) -> Result<(), ExclusionError> {
    let catalogue: serde_json::Value = serde_json::from_str(include_str!(
        "../../../data/authored/excluded-geometry-fingerprints.json"
    ))
    .expect("recorded catalogue decodes");
    for (input_start, window) in points.windows(LineageIndex::REFUSED_RUN_LENGTH).enumerate() {
        for (index, original) in originals().iter().enumerate() {
            let mut keys: Vec<_> = original.points.iter().map(PointKey::lineage).collect();
            if keys.len() > 1 && keys.first() == keys.last() {
                keys.pop();
            }
            if keys.len() < LineageIndex::REFUSED_RUN_LENGTH {
                continue;
            }
            for ring_start in 0..keys.len() {
                for direction in [RunDirection::Forward, RunDirection::Reverse] {
                    if window.iter().enumerate().all(|(offset, point)| {
                        let index = match direction {
                            RunDirection::Forward => (ring_start + offset) % keys.len(),
                            RunDirection::Reverse => {
                                (ring_start + keys.len() - offset) % keys.len()
                            }
                        };
                        PointKey::lineage_neighborhood(point).contains(&keys[index])
                    }) {
                        let geometry =
                            serde_json::from_value(catalogue["geometries"][index].clone())
                                .expect("recorded complete geometry decodes");
                        return Err(generated_refusal(
                            &geometry,
                            input_start,
                            ring_start,
                            direction,
                        ));
                    }
                }
            }
        }
    }
    Ok(())
}
#[derive(Clone, Debug)]
struct Original {
    name: String,
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
            for (index, feature) in value["features"].as_array().expect("features exist").iter().enumerate() {
                let geometry = &feature["geometry"];
                let polygons = if geometry["type"] == "MultiPolygon" {
                    geometry["coordinates"].as_array().expect("polygons exist").clone()
                } else {
                    vec![geometry["coordinates"].clone()]
                };
                for polygon in polygons {
                    for ring in polygon.as_array().expect("rings exist") {
                        originals.push(Original {
                            name: format!("{path}/{index}"),
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
                    originals.push(Original { name: item.ident.to_string(), points });
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
