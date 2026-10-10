use std::process::Command;

fn source_at(path: &str) -> serde_json::Value {
    let result = Command::new("git")
        .args(["show", &format!("6ac32bfbf67e26b9cfe94560806fda293db05801:{path}")])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("the recorded source is readable");
    assert!(result.status.success(), "the recorded source exists in history");
    serde_json::from_slice(&result.stdout).expect("the source is GeoJSON")
}

fn renamed_osm() -> serde_json::Value {
    let source = source_at("data/osm/rivers.geojson");
    let feature = source["features"].as_array().expect("source features")
        .iter()
        .filter(|feature| feature["properties"]["name"] == "River Jordan")
        .filter(|feature| feature["geometry"]["type"] == "LineString")
        .max_by_key(|feature| feature["geometry"]["coordinates"].as_array().expect("source course").len())
        .expect("the recorded Jordan has a course");
    serde_json::json!({"type":"FeatureCollection","features":[{
        "type":"Feature",
        "properties":{"name":"renamed-permitted-looking-course","rivernum":229,"featurecla":"River"},
        "geometry":feature["geometry"]
    }]})
}

#[test]
fn restored_osm_content_is_refused_even_with_natural_earth_properties() {
    let input = renamed_osm();
    let result = map_compile::exclusion::check_geojson(&input);
    eprintln!("renamed historical OSM content guard: {result:?}");
    assert!(result.is_err(), "excluded OSM course content is refused independently of its path and labels");
}

#[test]
fn restored_osm_content_cannot_enter_the_real_partition_loader() {
    let output = run_loader(renamed_osm(), "excluded");
    eprintln!("{}", String::from_utf8_lossy(&output.stdout));
    eprintln!("{}", String::from_utf8_lossy(&output.stderr));
    assert!(output.status.success(), "the real partition loader refuses a renamed historical OSM course");
}

#[test]
fn permitted_natural_earth_courses_still_enter_the_real_partition_loader() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let source: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(root.join("data/natural-earth/ne_10m_rivers_lake_centerlines.geojson"))
            .expect("the retained Natural Earth source is readable"),
    ).expect("the retained source is GeoJSON");
    let jordan: Vec<_> = source["features"].as_array().expect("source features")
        .iter().filter(|feature| feature["properties"]["rivernum"] == 229).cloned().collect();
    let output = run_loader(serde_json::json!({"type":"FeatureCollection","features":jordan}), "permitted");
    eprintln!("{}", String::from_utf8_lossy(&output.stdout));
    eprintln!("{}", String::from_utf8_lossy(&output.stderr));
    assert!(output.status.success(), "the real partition loader retains the permitted Jordan drainage");
}

fn run_loader(input: serde_json::Value, disposition: &str) -> std::process::Output {
    let root = std::env::temp_dir().join(format!("maps-x1-review-{}-{disposition}", std::process::id()));
    let directory = root.join("data/natural-earth");
    std::fs::create_dir_all(&directory).expect("the temporary input directory is created");
    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/natural-earth");
    for name in ["med_clip.geojson", "ne_10m_lakes.geojson"] {
        std::fs::copy(source.join(name), directory.join(name)).expect("permitted water inputs are copied");
    }
    std::fs::write(directory.join("ne_10m_rivers_lake_centerlines.geojson"),
        serde_json::to_vec(&input).expect("the input serializes"))
        .expect("only temporary river input is written");
    let output = Command::new(std::env::current_exe().expect("the probe executable exists"))
        .args(["--exact", "real_loader_child", "--ignored", "--nocapture"])
        .env("MAPS_X1_REVIEW_DISPOSITION", disposition)
        .current_dir(&root).output().expect("the real loader subprocess runs");
    std::fs::remove_dir_all(&root).expect("only the probe's own temporary inputs are removed");
    output
}

#[test]
#[ignore]
fn real_loader_child() {
    let result = map_compile::partition_bridge::gather_witnesses(&[]);
    eprintln!("real loader: {:?}", result.as_ref().map(|(regions, lines)|
        (regions.len(), lines.iter().map(|line| (&line.id, line.pts.len())).collect::<Vec<_>>())));
    if std::env::var("MAPS_X1_REVIEW_DISPOSITION").expect("the expected disposition is supplied") == "excluded" {
        assert!(result.is_err(), "the real loader refuses historical OSM geometry before partition derivation");
    } else {
        let (_, lines) = result.expect("permitted Natural Earth inputs are admitted");
        assert!(!lines.is_empty(), "the permitted control produces real river witnesses");
    }
}
