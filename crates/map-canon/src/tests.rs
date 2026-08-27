//! Phase-1 laws of the Canon (2026-08-27 design), written before the
//! implementation: content addressing with one home per fact, worlds
//! as timestamp-keyed SETS at covenant granularity, closed references,
//! and the hard no-overlap law inside Territory.

use std::collections::BTreeSet;

use atlas_graph_types::covenant::{PlaceId, TimePoint, Year};
use map_types::UnitVec;

use crate::*;

fn uv(lat: f64, lon: f64) -> UnitVec {
    UnitVec::from_lat_lon_deg(lat, lon)
}

fn ts(year: i32) -> Timestamp {
    TimePoint::year_only(Year::new(year).unwrap())
}

fn ts_day(year: i32, month: u8, day: u8) -> Timestamp {
    TimePoint::new(Year::new(year).unwrap(), Some(month), Some(day)).unwrap()
}

fn square(lat0: f64, lon0: f64, d: f64) -> Border {
    Border(vec![
        uv(lat0, lon0),
        uv(lat0, lon0 + d),
        uv(lat0 + d, lon0 + d),
        uv(lat0 + d, lon0),
    ])
}

fn entity(s: &str) -> EntityId {
    EntityId(s.to_string())
}

/// Area over one ring, no holes.
fn area(store: &mut CanonStore, ent: &str, ring: Border) -> FeatureId {
    let b = store.insert_border(ring);
    store.insert_feature(Feature::Area(Area {
        entity: entity(ent),
        name: ent.to_string(),
        rings: BTreeSet::from([b]),
        holes: BTreeSet::new(),
    }))
}

// ------------------------------------------- one home per fact

/// The same border inserted twice IS one border: same id, one stored
/// object. A different border gets a different id.
#[test]
fn borders_are_content_addressed_and_deduped() {
    let mut store = CanonStore::default();
    let a1 = store.insert_border(square(30.0, 30.0, 2.0));
    let a2 = store.insert_border(square(30.0, 30.0, 2.0));
    let b = store.insert_border(square(40.0, 40.0, 2.0));
    assert_eq!(a1, a2, "identical content, identical id");
    assert_ne!(a1, b, "distinct content, distinct id");
    assert_eq!(store.borders().len(), 2, "one home per border");
}

/// Features and snapshots follow the same discipline, and a snapshot's
/// identity does not depend on insertion order (its features are a SET).
#[test]
fn snapshots_are_sets_with_stable_identity() {
    let mut s1 = CanonStore::default();
    let f1 = area(&mut s1, "egypt", square(25.0, 26.0, 8.0));
    let f2 = area(&mut s1, "edom", square(30.0, 35.0, 1.5));
    let snap_a = s1.insert_snapshot(Snapshot { features: BTreeSet::from([f1, f2]) });
    let snap_b = s1.insert_snapshot(Snapshot { features: BTreeSet::from([f2, f1]) });
    assert_eq!(snap_a, snap_b, "feature order cannot matter");
    assert_eq!(s1.snapshots().len(), 1);

    // And the SAME content built in a second store gets the SAME ids.
    let mut s2 = CanonStore::default();
    let g1 = area(&mut s2, "egypt", square(25.0, 26.0, 8.0));
    let g2 = area(&mut s2, "edom", square(30.0, 35.0, 1.5));
    assert_eq!((f1, f2), (g1, g2), "ids are content, not session state");
}

// ------------------------------------------- worlds are keyed sets

/// A world is a set of (timestamp, snapshot) pairs keyed by timestamp:
/// one world state per instant. Re-asserting the same pair is
/// idempotent; a DIFFERENT snapshot at the same instant is a
/// contradiction and is refused.
#[test]
fn one_world_state_per_instant() {
    let mut store = CanonStore::default();
    let f = area(&mut store, "egypt", square(25.0, 26.0, 8.0));
    let g = area(&mut store, "edom", square(30.0, 35.0, 1.5));
    let s1 = store.insert_snapshot(Snapshot { features: BTreeSet::from([f]) });
    let s2 = store.insert_snapshot(Snapshot { features: BTreeSet::from([f, g]) });

    let mut world = World::default();
    world.insert(ts(-1450), s1).unwrap();
    world.insert(ts(-1450), s1).expect("idempotent re-assertion is fine");
    let err = world.insert(ts(-1450), s2);
    assert!(matches!(err, Err(WorldError::ContradictionAt(t)) if t == ts(-1450)));
    assert_eq!(world.moments().len(), 1);
}

/// Timestamps are covenant TimePoints — arbitrarily granular. A world
/// can change twice inside one month, and the moments order by the
/// covenant's total order, not by insertion.
#[test]
fn moments_are_timestamp_granular_and_derive_order() {
    let mut store = CanonStore::default();
    let f = area(&mut store, "egypt", square(25.0, 26.0, 8.0));
    let g = area(&mut store, "edom", square(30.0, 35.0, 1.5));
    let s1 = store.insert_snapshot(Snapshot { features: BTreeSet::from([f]) });
    let s2 = store.insert_snapshot(Snapshot { features: BTreeSet::from([f, g]) });

    let mut world = World::default();
    // Inserted out of order, on purpose.
    world.insert(ts_day(33, 4, 3), s2).unwrap();
    world.insert(ts(-1450), s1).unwrap();
    world.insert(ts_day(33, 4, 1), s1).unwrap();
    let order: Vec<Timestamp> = world.moments().keys().copied().collect();
    assert_eq!(order, vec![ts(-1450), ts_day(33, 4, 1), ts_day(33, 4, 3)]);

    /// The state AT a time is the latest moment at or before it.
    fn at(world: &World, t: Timestamp) -> Option<SnapshotId> {
        world.state_at(&t)
    }
    assert_eq!(at(&world, ts_day(33, 4, 2)), Some(s1), "between moments: the earlier holds");
    assert_eq!(at(&world, ts_day(33, 4, 3)), Some(s2));
    assert_eq!(at(&world, ts(-2000)), None, "before the first moment there is no state");
}

// ------------------------------------------- closed references

/// Every id in the canon resolves, or validation says exactly what is
/// dangling. A well-formed canon validates to an empty list.
#[test]
fn references_are_closed_or_named() {
    let mut store = CanonStore::default();
    let f = area(&mut store, "egypt", square(25.0, 26.0, 8.0));
    let snap = store.insert_snapshot(Snapshot { features: BTreeSet::from([f]) });
    let mut world = World::default();
    world.insert(ts(-1450), snap).unwrap();
    store.set_layer(LayerKind::Territory, world);
    assert_eq!(store.validate(), vec![], "a whole canon is quiet");

    // A snapshot referencing a feature nobody stored:
    let ghost = FeatureId(atlas_graph_types::covenant::ContentHash(0xdead));
    let bad_snap = store.insert_snapshot(Snapshot { features: BTreeSet::from([ghost]) });
    let mut w2 = World::default();
    w2.insert(ts(100), bad_snap).unwrap();
    store.set_layer(LayerKind::Journeys, w2);
    assert!(store
        .validate()
        .iter()
        .any(|v| matches!(v, CanonViolation::UnresolvedFeature { feature, .. } if *feature == ghost)));
}

/// A feature referencing a border nobody stored is named too.
#[test]
fn dangling_borders_are_named() {
    let mut store = CanonStore::default();
    let ghost = BorderId(atlas_graph_types::covenant::ContentHash(0xbeef));
    let f = store.insert_feature(Feature::Area(Area {
        entity: entity("nowhere"),
        name: "Nowhere".to_string(),
        rings: BTreeSet::from([ghost]),
        holes: BTreeSet::new(),
    }));
    let snap = store.insert_snapshot(Snapshot { features: BTreeSet::from([f]) });
    let mut world = World::default();
    world.insert(ts(0 - 1), snap).unwrap();
    store.set_layer(LayerKind::Territory, world);
    assert!(store
        .validate()
        .iter()
        .any(|v| matches!(v, CanonViolation::UnresolvedBorder { border, .. } if *border == ghost)));
}

// ---------------------------------- the hard law: Territory no-overlap

fn territory_with(store: &mut CanonStore, at: Timestamp, features: &[FeatureId]) {
    let snap = store.insert_snapshot(Snapshot { features: features.iter().copied().collect() });
    let mut world = World::default();
    world.insert(at, snap).unwrap();
    store.set_layer(LayerKind::Territory, world);
}

/// Two territorial claims on the same ground at the same moment is a
/// contradiction: validation names the moment and both entities.
#[test]
fn overlapping_territories_fail_loud() {
    let mut store = CanonStore::default();
    let a = area(&mut store, "egypt", square(25.0, 26.0, 8.0));
    let b = area(&mut store, "usurper", square(28.0, 29.0, 8.0)); // overlaps egypt
    territory_with(&mut store, ts(-1450), &[a, b]);
    let violations = store.validate();
    assert!(
        violations.iter().any(|v| matches!(
            v,
            CanonViolation::TerritorialOverlap { at, .. } if *at == ts(-1450)
        )),
        "expected a territorial overlap, got {violations:?}"
    );
}

/// Neighbors that share an edge do NOT overlap; a small square wholly
/// inside a big one DOES (containment without edge crossings).
#[test]
fn touching_is_not_overlap_but_containment_is() {
    let mut store = CanonStore::default();
    let west = area(&mut store, "westia", square(10.0, 10.0, 5.0));
    let east = area(&mut store, "estia", square(10.0, 15.0, 5.0)); // shares the lon=15 edge
    territory_with(&mut store, ts(-2000), &[west, east]);
    assert_eq!(store.validate(), vec![], "shared edges are peace, not war");

    let mut store2 = CanonStore::default();
    let big = area(&mut store2, "empire", square(10.0, 10.0, 10.0));
    let tiny = area(&mut store2, "enclave", square(14.0, 14.0, 1.0)); // wholly inside
    territory_with(&mut store2, ts(-2000), &[big, tiny]);
    assert!(
        store2
            .validate()
            .iter()
            .any(|v| matches!(v, CanonViolation::TerritorialOverlap { .. })),
        "containment is overlap even though no edges cross"
    );
}

/// Overlap ACROSS layers is meaning, not contradiction: the promise
/// over a kingdom is fine. Overlap in the SAME layer at DIFFERENT
/// moments is fine too — that is just history moving.
#[test]
fn overlap_is_only_a_crime_within_territory_at_one_moment() {
    let mut store = CanonStore::default();
    let kingdom = area(&mut store, "kingdom", square(30.0, 34.0, 3.0));
    let promise = area(&mut store, "authored:promise", square(29.0, 33.0, 6.0));
    territory_with(&mut store, ts(-1000), &[kingdom]);
    let snap = store.insert_snapshot(Snapshot { features: BTreeSet::from([promise]) });
    let mut claims = World::default();
    claims.insert(ts(-1000), snap).unwrap();
    store.set_layer(LayerKind::ScriptureClaims, claims);
    assert_eq!(store.validate(), vec![], "cross-layer overlap is meaning");

    // Same layer, different moments: Egypt's border moved over itself.
    let mut store2 = CanonStore::default();
    let egypt_early = area(&mut store2, "egypt", square(25.0, 26.0, 8.0));
    let egypt_late = area(&mut store2, "egypt", square(24.0, 25.0, 8.0));
    let s1 = store2.insert_snapshot(Snapshot { features: BTreeSet::from([egypt_early]) });
    let s2 = store2.insert_snapshot(Snapshot { features: BTreeSet::from([egypt_late]) });
    let mut world = World::default();
    world.insert(ts(-1500), s1).unwrap();
    world.insert(ts(-1400), s2).unwrap();
    store2.set_layer(LayerKind::Territory, world);
    assert_eq!(store2.validate(), vec![], "history moving is not a contradiction");
}

// ------------------------------------------- routes carry time in legs

/// A route's legs carry timestamp spans (day-granular welcome), and a
/// leg whose span runs backward is named by validation.
#[test]
fn route_legs_carry_ordered_spans() {
    let mut store = CanonStore::default();
    let road = store.insert_border(Border(vec![uv(31.78, 35.22), uv(33.51, 36.29)]));
    let ok = store.insert_feature(Feature::Way(Route {
        entity: entity("damascus-road"),
        name: "the Damascus road".to_string(),
        legs: vec![Leg {
            from: PlaceId::new("jerusalem".to_string()),
            to: PlaceId::new("damascus".to_string()),
            border: road,
            span: (ts_day(35, 3, 1), ts_day(35, 3, 6)),
        }],
    }));
    let snap = store.insert_snapshot(Snapshot { features: BTreeSet::from([ok]) });
    let mut world = World::default();
    world.insert(ts_day(35, 3, 1), snap).unwrap();
    store.set_layer(LayerKind::Journeys, world);
    assert_eq!(store.validate(), vec![]);

    let backward = store.insert_feature(Feature::Way(Route {
        entity: entity("backward"),
        name: "impossible walk".to_string(),
        legs: vec![Leg {
            from: PlaceId::new("a".to_string()),
            to: PlaceId::new("b".to_string()),
            border: road,
            span: (ts_day(35, 3, 6), ts_day(35, 3, 1)),
        }],
    }));
    let snap2 = store.insert_snapshot(Snapshot { features: BTreeSet::from([backward]) });
    let mut w2 = World::default();
    w2.insert(ts_day(35, 3, 6), snap2).unwrap();
    store.set_layer(LayerKind::Journeys, w2);
    assert!(store
        .validate()
        .iter()
        .any(|v| matches!(v, CanonViolation::BackwardLeg { feature, .. } if *feature == backward)));
}
