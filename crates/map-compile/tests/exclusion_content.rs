use std::process::Command;
use std::sync::OnceLock;

use map_compile::exclusion::{self, ExclusionError};
use map_compile::vendor::PolityRow;
use map_types::UnitVec;
use proptest::prelude::*;

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

fn expected(index: usize) -> ExclusionError {
    let catalogue: serde_json::Value = serde_json::from_str(include_str!(
        "../../../data/authored/excluded-geometry-fingerprints.json"
    ))
    .expect("the checked quarantine catalogue decodes");
    let geometry = serde_json::from_value(catalogue["geometries"][index].clone())
        .expect("the independently catalogued complete geometry decodes");
    ExclusionError::Excluded(vec![geometry])
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

#[test]
fn restored_judah_is_refused_through_another_polity_input() {
    let original = &originals()[0];
    assert_eq!(
        original.points.len(),
        156,
        "the reviewer seed restores every Judah vertex"
    );
    let expected = expected(0);
    let result = map_compile::partition_bridge::gather_witnesses(&[polity(
        &original.points,
        "renamed-control".into(),
    )]);
    assert_eq!(
        result.err(),
        Some(format!("excluded input: {expected:?}")),
        "a renamed Judah input is refused with its complete provenance"
    );
}

#[test]
fn every_excluded_original_is_bound_to_its_content() {
    assert_eq!(
        originals().len(),
        107,
        "every quarantined geometry has an executable content control"
    );
    for (index, original) in originals().iter().enumerate() {
        let expected = expected(index);
        assert_eq!(
            exclusion::check_points(&original.points),
            Err(expected.clone()),
            "{} reports its complete independently catalogued origin",
            original.name
        );
        let admitted = map_compile::partition_bridge::gather_witnesses(&[polity(
            &original.points,
            "renamed".into(),
        )]);
        assert_eq!(
            admitted.err(),
            Some(format!("excluded input: {expected:?}")),
            "{} is refused before compiler derivation",
            original.name
        );
    }
}

#[test]
fn restored_med_input_is_refused() {
    let root = std::env::temp_dir().join(format!("maps-x0-restoration-{}", std::process::id()));
    let directory = root.join("data/natural-earth");
    std::fs::create_dir_all(&directory).expect("temporary probe directory");
    let source: serde_json::Value =
        serde_json::from_str(&historical("data/wikimedia/tribes12.geojson"))
            .expect("the original Judah source parses");
    let data = serde_json::json!({"type":"FeatureCollection","features":[{"type":"Feature","geometry":source["features"][0]["geometry"],"properties":{"name":"permitted-looking-water"}}]});
    std::fs::write(
        directory.join("med_clip.geojson"),
        serde_json::to_vec(&data).expect("probe GeoJSON serializes"),
    )
    .expect("temporary probe input");
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
    std::fs::remove_file(directory.join("med_clip.geojson")).expect("remove own temporary probe");
    std::fs::remove_dir_all(&root).expect("remove own empty probe directory");
    assert!(
        output.status.success(),
        "the reviewer's real med-clip restoration is refused: {}",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
#[ignore]
fn restoration_probe_child() {
    let expected = expected(0);
    let result = map_compile::partition_bridge::gather_witnesses(&[]);
    assert_eq!(
        result.err(),
        Some(format!("excluded input: {expected:?}")),
        "the real med loader rejects Judah before filtering or partitioning"
    );
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]
    #[test]
    fn renamed_split_and_indirect_descendants_keep_the_excluded_content(
        index in 0usize..107, rotations in proptest::collection::vec(any::<usize>(), 1..8),
        reverse in any::<bool>(), split in 1usize..9, name in "[a-z]{1,24}",
    ) {
        let original = &originals()[index];
        let expected = expected(index);
        let mut points = original.points.clone();
        for rotation in rotations {
            let length = points.len();
            points.rotate_left(rotation % length);
            if reverse { points.reverse(); }
            points.extend(permitted());
        }
        let mut store = map_canon::CanonStore::default();
        for part in points.chunks(split) { store.insert_border(map_canon::Border(part.to_vec())); }
        prop_assert_eq!(exclusion::check_compiled(&store), Err(expected.clone()), "compiled descendants retain the complete excluded-source refusal");
        let positions: Vec<_> = points.iter().map(|point| { let (lat, lon) = point.to_lat_lon_deg(); serde_json::json!([lon, lat]) }).collect();
        let features: Vec<_> = positions.chunks(split).map(|part| serde_json::json!({"geometry":{"coordinates":part},"properties":{"name":name}})).collect();
        let json = serde_json::json!({"derived":{"renamed":{"features":features}}});
        prop_assert_eq!(exclusion::check_geojson(&json), Err(expected.clone()), "nested renamed raw inputs retain the complete excluded-source refusal");
        let mut timeline = map_adapters::promised_land_timeline();
        timeline.boundaries.values_mut().next().expect("survey boundary").versions[0].1.pts = points.clone();
        let mut compiled = map_canon::CanonStore::default();
        let result = map_compile::timeline_bridge::bridge_timeline_regions(
            &mut compiled, &timeline, map_canon::LayerKind::ScriptureClaims,
            map_canon::Witness::Authored, &name,
        );
        prop_assert_eq!(result, Err(format!("excluded input: {expected:?}")), "the timeline compiler rejects indirect excluded ancestors before mutation");
        prop_assert_eq!(compiled, map_canon::CanonStore::default(), "refused timeline input leaves the complete compiler output unchanged");
        let result = map_compile::compile::append_ways(&mut store, &[]);
        prop_assert_eq!(result, Err(format!("excluded output: {expected:?}")), "the compiler checks every stored output border even without new ways");

        let result = map_compile::partition_bridge::gather_witnesses(&[polity(&points, name)]);
        prop_assert_eq!(result.err(), Some(format!("excluded input: {expected:?}")), "nonempty polity families reject content regardless of names and derivation order");
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

#[test]
fn excluded_content_split_across_input_files_is_refused() {
    let root = std::env::temp_dir().join(format!("maps-x0-multiple-inputs-{}", std::process::id()));
    std::fs::create_dir_all(&root).expect("temporary input directory");
    let original = &originals()[0];
    let expected = expected(0);
    for (index, part) in original.points.chunks(50).enumerate() {
        let coordinates: Vec<_> = part
            .iter()
            .map(|point| {
                let (lat, lon) = point.to_lat_lon_deg();
                serde_json::json!([lon, lat])
            })
            .collect();
        let value = serde_json::json!({"geometry":{"type":"LineString","coordinates":coordinates}});
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
        Err(expected),
        "multiple renamed inputs collectively retain the complete excluded geometry"
    );
}
