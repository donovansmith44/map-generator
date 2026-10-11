use std::path::Path;

#[test]
fn excluded_lineage_has_no_compiled_plate_or_vendored_descendants() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for relative in [
        "crates/map-adapters/src/plate_water.rs",
        "tools/plate_trace",
        "data/wikimedia",
        "data/openbible/regions.geojson",
    ] {
        assert!(
            !root.join(relative).exists(),
            "excluded descendant {relative} is removed"
        );
    }
}
