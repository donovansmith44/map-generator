fn main() {
    let (regions, _) = map_compile::partition_bridge::gather_witnesses(&[]).expect("partition inputs load");
    let restored = regions.iter().find(|region| region.id == "great-sea-1").expect("restored excluded ring was admitted under a water id");
    assert_eq!(restored.rings[0].len(), 156, "the restored ring retains every excluded vertex");
    println!("admitted id={} vertices={}", restored.id, restored.rings[0].len());
}
