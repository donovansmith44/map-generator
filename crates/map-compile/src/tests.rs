//! Phase-2 laws, written first: the atlas API's payloads parse into
//! typed rows (shape drift fails loud, never vendors garbage), and the
//! vendor writer is deterministic — same payloads, same bytes, same pin.

use crate::vendor::*;

fn fx(name: &str) -> String {
    std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures").join(name),
    )
    .expect("fixture exists")
}

// ------------------------------------------------------- typed parses

#[test]
fn polities_parse_typed_and_sane() {
    let rows = parse_polities(&fx("polities.json")).expect("live-captured fixture parses");
    assert!(rows.len() >= 10, "a real polity book, got {}", rows.len());
    let assyria = rows.iter().find(|p| p.id == "assyria").expect("assyria present");
    assert_eq!(assyria.name, "Assyria");
    assert_eq!((assyria.from_year, assyria.to_year), (-1900, -912));
    assert!(!assyria.rings.is_empty() && assyria.rings[0].len() >= 4);
    for p in &rows {
        assert!(p.from_year <= p.to_year, "{}: era runs forward", p.id);
        assert!(p.rings.iter().all(|r| r.len() >= 3), "{}: rings are areas", p.id);
    }
}

#[test]
fn narratives_parse_with_ordered_legs() {
    let rows = parse_narratives(&fx("narratives.json")).expect("parses");
    let abraham = rows.iter().find(|n| n.id == "abraham-migration").expect("present");
    assert_eq!(abraham.name, "Abraham's Migration");
    assert_eq!(abraham.color, "#D97706");
    assert_eq!(abraham.legs.len(), 6);
    assert_eq!(abraham.legs[1], "ab_haran");
    assert!(rows.len() >= 5, "the Bible walks in many narratives");
}

#[test]
fn events_parse_with_time_places_verses() {
    let e = parse_event(&fx("event-ab_haran.json")).expect("parses");
    assert_eq!(e.id, "ab_haran");
    assert_eq!(e.when, Some((-2092, -2091)));
    assert_eq!(e.places, vec!["haran".to_string()]);
    assert!(e.verses.iter().any(|v| v == "GEN.12.1"), "verses flatten: {:?}", e.verses);
}

#[test]
fn eras_landmarks_landmask_parse() {
    let eras = parse_eras(&fx("eras.json")).expect("parses");
    assert!(eras.iter().any(|e| e.id == "patriarchs" && e.from_year == -2166));
    let lm = parse_landmarks(&fx("landmarks.json")).expect("parses");
    assert!(lm.iter().any(|l| l.name == "Sea of Galilee" && l.kind == "water"));
    let mask = parse_land_mask(&fx("land-mask.json")).expect("parses");
    assert!(!mask.rings.is_empty() && mask.rings[0].len() >= 10);
}

/// Shape drift fails loud: a payload missing required fields is an
/// error naming the field, never a silently-empty vendor file.
#[test]
fn shape_drift_is_a_named_error() {
    let err = parse_polities(r#"{"polities":[{"id":"x"}]}"#).unwrap_err();
    assert!(err.contains("name"), "the missing field is named: {err}");
    assert!(parse_event(r#"{"nonsense":true}"#).is_err());
}

// ------------------------------------------- deterministic vendoring

#[test]
fn vendor_writes_are_deterministic_and_pinned() {
    let payloads = vec![
        ("polities.json".to_string(), fx("polities.json").into_bytes()),
        ("narratives.json".to_string(), fx("narratives.json").into_bytes()),
    ];
    let dir1 = std::env::temp_dir().join("canon-vendor-test-1");
    let dir2 = std::env::temp_dir().join("canon-vendor-test-2");
    let _ = std::fs::remove_dir_all(&dir1);
    let _ = std::fs::remove_dir_all(&dir2);
    let pin1 = write_vendor(&dir1, &payloads).expect("writes");
    let pin2 = write_vendor(&dir2, &payloads).expect("writes");
    assert_eq!(pin1, pin2, "same payloads, same pin");
    let m1 = std::fs::read(dir1.join("manifest.json")).unwrap();
    let m2 = std::fs::read(dir2.join("manifest.json")).unwrap();
    assert_eq!(m1, m2, "same payloads, byte-identical manifest");

    // A changed payload moves the pin — staleness is visible (C6 spirit).
    let mut changed = payloads.clone();
    changed[0].1.push(b' ');
    let dir3 = std::env::temp_dir().join("canon-vendor-test-3");
    let _ = std::fs::remove_dir_all(&dir3);
    let pin3 = write_vendor(&dir3, &changed).expect("writes");
    assert_ne!(pin1, pin3, "a changed world changes the pin");
}
