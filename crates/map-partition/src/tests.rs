//! The ordered law suite of the sphere partition (spec §7). Later
//! laws assume earlier ones.

use std::f64::consts::PI;

use map_types::UnitVec;

use crate::build::{build, build_with, WitnessBorder, WitnessPolyline, WitnessRegion, WitnessSeed};
use crate::{cycle_area, tri_area, winding, FaceKind, Partition, PartitionConfig, RiverSystem, Watershed};

fn uv(lat: f64, lon: f64) -> UnitVec {
    UnitVec::from_lat_lon_deg(lat, lon)
}

fn square(lat0: f64, lon0: f64, lat1: f64, lon1: f64) -> Vec<UnitVec> {
    vec![uv(lat0, lon0), uv(lat0, lon1), uv(lat1, lon1), uv(lat1, lon0)]
}

fn region(id: &str, kind: FaceKind, rings: Vec<Vec<UnitVec>>) -> WitnessRegion {
    WitnessRegion { id: id.to_string(), kind, rings, parent: None }
}

fn cfg() -> PartitionConfig {
    PartitionConfig::default()
}

fn ok(p: &Partition) {
    let v = p.validate();
    assert!(v.is_empty(), "laws hold: {v:?}");
}

// ---------------------------------------------- primitive area laws

/// An octant of the sphere is exactly π/2 steradians.
#[test]
fn octant_area_is_pi_over_two() {
    let a = UnitVec::from_lat_lon_deg(0.0, 0.0);
    let b = UnitVec::from_lat_lon_deg(0.0, 90.0);
    let c = UnitVec::from_lat_lon_deg(90.0, 0.0);
    assert!((tri_area(&a, &b, &c) - PI / 2.0).abs() < 1e-12);
    // reversal flips the sign
    assert!((tri_area(&c, &b, &a) + PI / 2.0).abs() < 1e-12);
}

/// Fan area of a closed cycle agrees with orientation.
#[test]
fn cycle_area_signs_follow_orientation() {
    let sq = square(10.0, 10.0, 12.0, 12.0);
    let a = cycle_area(&sq);
    assert!(a > 0.0, "counterclockwise (interior-left) is positive");
    let mut rev = sq.clone();
    rev.reverse();
    assert!((cycle_area(&rev) + a).abs() < 1e-15);
}

/// Winding: +1 inside, 0 outside, -1 at the antipode.
#[test]
fn winding_classifies_in_out_antipode() {
    let sq = square(10.0, 10.0, 12.0, 12.0);
    assert_eq!(winding(&sq, &uv(11.0, 11.0)), 1);
    assert_eq!(winding(&sq, &uv(40.0, 40.0)), 0);
    assert_eq!(winding(&sq, &uv(-11.0, -169.0)), -1);
}

// ------------------------------------------- elementary subdivisions

/// L11: one polygon → exactly two faces (the claim + background),
/// closed shared boundary, 4π total.
#[test]
fn single_polygon_partitions_the_sphere() {
    let p = build(
        &[region("judah", FaceKind::LandClaim, vec![square(10.0, 10.0, 15.0, 15.0)])],
        &[],
        &cfg(),
    )
    .unwrap();
    ok(&p);
    assert_eq!(p.faces.len(), 2);
    let claim = p.faces.iter().find(|f| f.kind == FaceKind::LandClaim).expect("the claim");
    let back = p.faces.iter().find(|f| f.kind == FaceKind::Background).expect("the background");
    assert!(claim.area > 0.0 && back.area > claim.area);
    assert!(p.area_residual() < 1e-10, "Σ areas = 4π");
}

/// L12: two squares sharing one full edge — the shared border exists
/// ONCE (no coincident duplicates), three faces, 4π.
#[test]
fn adjacent_squares_share_one_border() {
    let p = build(
        &[
            region("west", FaceKind::LandClaim, vec![square(10.0, 10.0, 15.0, 15.0)]),
            region("east", FaceKind::LandClaim, vec![square(10.0, 15.0, 15.0, 20.0)]),
        ],
        &[],
        &cfg(),
    )
    .unwrap();
    ok(&p);
    assert_eq!(p.faces.len(), 3, "west, east, background");
    // no duplicate coincident edges
    let mut pairs: Vec<(usize, usize)> =
        p.edges.iter().map(|e| (e.a.min(e.b), e.a.max(e.b))).collect();
    let n = pairs.len();
    pairs.sort();
    pairs.dedup();
    assert_eq!(pairs.len(), n, "every canonical edge exists once");
    // some edge separates the two claims directly
    let shared = p.edges.iter().any(|e| {
        let f1 = &p.faces[p.halves[e.half_ab].face];
        let f2 = &p.faces[p.halves[e.half_ba].face];
        f1.kind == FaceKind::LandClaim && f2.kind == FaceKind::LandClaim
    });
    assert!(shared, "the shared border bounds both claims");
    assert!(p.area_residual() < 1e-10);
}

/// L13: a T-junction becomes an explicit vertex; no dangling edges.
/// The shared line lies on the equator — a true great circle — so the
/// junction is geometric, not an artifact of parallel-thinking.
#[test]
fn t_junctions_become_vertices() {
    // south square's top edge runs the equator lon 10..20; the north
    // square sits on only half of it: its corner lands mid-edge.
    let p = build(
        &[
            region("south", FaceKind::LandClaim, vec![square(-5.0, 10.0, 0.0, 20.0)]),
            region("north", FaceKind::LandClaim, vec![square(0.0, 10.0, 3.0, 15.0)]),
        ],
        &[],
        &cfg(),
    )
    .unwrap();
    ok(&p);
    assert_eq!(p.faces.len(), 3, "south, north, background — no lens, no dangle");
    // the junction vertex at (0,15) exists
    let junction = uv(0.0, 15.0);
    assert!(
        p.vertices.iter().any(|v| v.angle_to(&junction) < 1e-5),
        "the T-junction is an explicit vertex"
    );
    assert!(p.area_residual() < 1e-10);
}

/// L14: the same polygon with reversed winding is the same geometry.
#[test]
fn winding_is_normalized_away() {
    let sq = square(10.0, 10.0, 15.0, 15.0);
    let mut rev = sq.clone();
    rev.reverse();
    let a = build(&[region("w", FaceKind::LandClaim, vec![sq])], &[], &cfg()).unwrap();
    let b = build(&[region("w", FaceKind::LandClaim, vec![rev])], &[], &cfg()).unwrap();
    assert_eq!(a.content_hash(), b.content_hash());
}

/// L15: the same border drawn twice with sub-tolerance jitter merges
/// into ONE canonical geometry — no slivers, no duplicates.
#[test]
fn near_coincident_witnesses_merge() {
    let jitter: f64 = 2.0e-6; // ~13 m, well under tau_vertex
    let sq = square(10.0, 10.0, 15.0, 15.0);
    let sq2: Vec<UnitVec> = sq
        .iter()
        .map(|p| {
            let (lat, lon) = p.to_lat_lon_deg();
            uv(lat + jitter.to_degrees(), lon - jitter.to_degrees())
        })
        .collect();
    let p = build(
        &[
            region("a", FaceKind::LandClaim, vec![sq]),
            region("b", FaceKind::LandClaim, vec![sq2]),
        ],
        &[],
        &cfg(),
    )
    .unwrap();
    ok(&p);
    assert_eq!(p.faces.len(), 2, "one merged claim + background");
    assert_eq!(p.vertices.len(), 4, "clustered to the four corners");
    // both witnesses ride the same canonical edges
    assert!(p.edges.iter().all(|e| e.provenance.len() == 2));
}

/// L16: disagreement beyond tolerance stays distinct — never an
/// accidental merge.
#[test]
fn above_tolerance_disagreement_stays_distinct() {
    let off = 0.05; // ~5.5 km: a real disagreement
    let p = build(
        &[
            region("a", FaceKind::LandClaim, vec![square(10.0, 10.0, 15.0, 15.0)]),
            region("b", FaceKind::LandClaim, vec![square(10.0 + off, 10.0 + off, 15.0 + off, 15.0 + off)]),
        ],
        &[],
        &cfg(),
    )
    .unwrap();
    ok(&p);
    assert!(p.faces.len() > 3, "the lens of disagreement is its own cell");
    assert!(p.area_residual() < 1e-10);
}

/// A hole: a lake inside a land claim is the lake's face; the claim
/// keeps a hole cycle; everything still sums to 4π.
#[test]
fn lake_inside_claim_is_a_hole_face() {
    let p = build(
        &[
            region("land", FaceKind::LandClaim, vec![square(10.0, 10.0, 20.0, 20.0)]),
            region("lake", FaceKind::Lake, vec![square(13.0, 13.0, 15.0, 15.0)]),
        ],
        &[],
        &cfg(),
    )
    .unwrap();
    ok(&p);
    assert_eq!(p.faces.len(), 3);
    let land = p.faces.iter().find(|f| f.kind == FaceKind::LandClaim).unwrap();
    assert_eq!(land.cycles.len(), 2, "outer boundary + the lake hole");
    let lake = p.faces.iter().find(|f| f.kind == FaceKind::Lake).unwrap();
    assert!(lake.area < land.area);
    assert!(p.area_residual() < 1e-10);
}

// ----------------------------------------------------- determinism

/// L24–L27: witness order changes nothing.
#[test]
fn build_is_order_invariant() {
    let a = region("a", FaceKind::LandClaim, vec![square(10.0, 10.0, 15.0, 15.0)]);
    let b = region("b", FaceKind::LandClaim, vec![square(10.0, 15.0, 15.0, 20.0)]);
    let c = region("c", FaceKind::Lake, vec![square(11.0, 11.0, 12.0, 12.0)]);
    let p1 = build(
        &[
            region("a", FaceKind::LandClaim, vec![square(10.0, 10.0, 15.0, 15.0)]),
            region("b", FaceKind::LandClaim, vec![square(10.0, 15.0, 15.0, 20.0)]),
            region("c", FaceKind::Lake, vec![square(11.0, 11.0, 12.0, 12.0)]),
        ],
        &[],
        &cfg(),
    )
    .unwrap();
    let p2 = build(&[c, b, a], &[], &cfg()).unwrap();
    assert_eq!(p1.content_hash(), p2.content_hash());
    assert_eq!(p1.faces.len(), p2.faces.len());
}

// ---------------------------------------------------------- rivers

/// L28 (amended): an interior river is an overlay polyline — it
/// splits nothing, gaps nothing, and shares canonical vertices where
/// it touches the border.
#[test]
fn interior_river_is_an_overlay() {
    let p = build(
        &[region("land", FaceKind::LandClaim, vec![square(10.0, 10.0, 20.0, 20.0)])],
        &[WitnessPolyline {
            id: "brook".into(),
            pts: vec![uv(15.0, 10.0), uv(15.0, 13.0), uv(14.0, 16.0)],
        }],
        &cfg(),
    )
    .unwrap();
    ok(&p);
    assert_eq!(p.faces.len(), 2, "the river split no face");
    assert_eq!(p.rivers.len(), 1);
    assert!(p.area_residual() < 1e-10);
}

/// L29: a river along a border marks the ONE canonical border edge as
/// a river — geometry stored once, styled twice.
#[test]
fn border_river_is_one_edge() {
    let p = build(
        &[
            region("west", FaceKind::LandClaim, vec![square(10.0, 10.0, 15.0, 15.0)]),
            region("east", FaceKind::LandClaim, vec![square(10.0, 15.0, 15.0, 20.0)]),
        ],
        &[WitnessPolyline {
            id: "jordan".into(),
            pts: vec![uv(10.0, 15.0), uv(12.5, 15.0), uv(15.0, 15.0)],
        }],
        &cfg(),
    )
    .unwrap();
    ok(&p);
    let river_edges: Vec<_> = p.edges.iter().filter(|e| e.river).collect();
    assert!(!river_edges.is_empty(), "the border carries the river attribute");
    for e in river_edges {
        let f1 = p.halves[e.half_ab].face;
        let f2 = p.halves[e.half_ba].face;
        assert_ne!(f1, f2, "still exactly two faces on the river border");
    }
}

/// L30: rivers never alter the partition's geometric identity.
#[test]
fn rivers_leave_geometry_hash_unchanged() {
    let regions = || vec![region("land", FaceKind::LandClaim, vec![square(10.0, 10.0, 20.0, 20.0)])];
    let without = build(&regions(), &[], &cfg()).unwrap();
    let with = build(
        &regions(),
        &[WitnessPolyline { id: "r".into(), pts: vec![uv(12.0, 11.0), uv(13.0, 14.0)] }],
        &cfg(),
    )
    .unwrap();
    // rivers ride the overlay; the face/edge geometry hash ignores
    // them except for the river attribute on border edges (none here)
    assert_eq!(without.content_hash(), with.content_hash());
}

// -------------------------------------------------- the 4π law suite

/// L38–L40: positive areas, and global completeness at 1e-10.
#[test]
fn completeness_at_world_tolerance() {
    let p = build(
        &[
            region("a", FaceKind::LandClaim, vec![square(10.0, 10.0, 15.0, 15.0)]),
            region("b", FaceKind::LandClaim, vec![square(10.0, 15.0, 15.0, 20.0)]),
            region("c", FaceKind::LandClaim, vec![square(15.0, 10.0, 18.0, 15.0)]),
            region("lake", FaceKind::Lake, vec![square(11.0, 11.0, 12.5, 12.5)]),
            region("sea", FaceKind::Sea, vec![square(-20.0, -20.0, -10.0, -5.0)]),
        ],
        &[],
        &cfg(),
    )
    .unwrap();
    ok(&p);
    assert!(p.faces.iter().all(|f| f.area > 0.0));
    assert!(p.area_residual() < 1e-10, "residual {:e}", p.area_residual());
}

/// L41–L43 (diagnostic): random points land in exactly one face.
#[test]
fn sampled_points_have_one_home() {
    let p = build(
        &[
            region("a", FaceKind::LandClaim, vec![square(10.0, 10.0, 15.0, 15.0)]),
            region("b", FaceKind::LandClaim, vec![square(10.0, 15.0, 15.0, 20.0)]),
        ],
        &[],
        &cfg(),
    )
    .unwrap();
    // deterministic pseudo-random samples
    let mut seed = 0x9e3779b97f4a7c15u64;
    for _ in 0..200 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let lat = ((seed >> 16) as f64 / (1u64 << 48) as f64) * 170.0 - 85.0;
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let lon = ((seed >> 16) as f64 / (1u64 << 48) as f64) * 360.0 - 180.0;
        let q = uv(lat, lon);
        let mut homes = 0;
        for f in 0..p.faces.len() {
            let rings = p.face_rings(f);
            let signed: f64 = rings.iter().map(|r| crate::cycle_area(r)).sum();
            let w: i32 = rings.iter().map(|r| winding(r, &q)).sum();
            // the wrap face (the one that gained 4π) contains a point
            // when its bearing-winding is 0; every other face at +1.
            let target = if signed <= 1e-12 { 0 } else { 1 };
            if w == target {
                homes += 1;
            }
        }
        assert_eq!(homes, 1, "point ({lat:.2},{lon:.2}) has exactly one face");
    }
}

/// A seed claims exactly the cell its borders and the water enclose;
/// the cell's water-side boundary IS the water's edge — flush is
/// definitional, not approximate.
#[test]
fn seed_claims_its_cell() {
    // a lake ring, plus three open land borders that END ON the lake:
    // together they enclose a cell west of the lake
    let lake = region("lake", FaceKind::Lake, vec![square(10.0, 14.0, 14.0, 18.0)]);
    let borders = vec![
        WitnessBorder {
            id: "south".into(),
            pts: vec![uv(10.0, 8.0), uv(10.0, 14.0)], // ends on lake SW corner
        },
        WitnessBorder { id: "west".into(), pts: vec![uv(10.0, 8.0), uv(14.0, 8.0)] },
        WitnessBorder {
            id: "north".into(),
            pts: vec![uv(14.0, 8.0), uv(14.0, 14.0)], // ends on lake NW corner
        },
    ];
    let seeds = vec![WitnessSeed {
        id: "landia".into(),
        kind: FaceKind::LandClaim,
        seed: uv(12.0, 11.0),
    }];
    let p = build_with(&[lake], &borders, &seeds, &[], &cfg()).unwrap();
    ok(&p);
    let land = p
        .faces
        .iter()
        .find(|f| f.kind == FaceKind::LandClaim)
        .expect("the seed claimed a cell");
    assert!(land.claims.contains(&"landia".to_string()));
    // the land cell borders the lake through a SHARED edge
    let shared = p.edges.iter().any(|e| {
        let (f1, f2) = (p.halves[e.half_ab].face, p.halves[e.half_ba].face);
        let kinds = (p.faces[f1].kind.clone(), p.faces[f2].kind.clone());
        matches!(
            kinds,
            (FaceKind::LandClaim, FaceKind::Lake) | (FaceKind::Lake, FaceKind::LandClaim)
        )
    });
    assert!(shared, "land meets lake on the lake's own edge");
    assert!(p.area_residual() < 1e-10);
}

/// A dangling border cannot silently leak a claim into the world:
/// the seed floods the wrap face, the Background vanishes, and the
/// build REFUSES.
#[test]
fn dangling_border_refuses_to_build() {
    let lake = region("lake", FaceKind::Lake, vec![square(10.0, 14.0, 14.0, 18.0)]);
    let borders = vec![
        // south border stops 2 degrees short of the lake: a leak
        WitnessBorder { id: "south".into(), pts: vec![uv(10.0, 8.0), uv(10.0, 12.0)] },
        WitnessBorder { id: "west".into(), pts: vec![uv(10.0, 8.0), uv(14.0, 8.0)] },
        WitnessBorder { id: "north".into(), pts: vec![uv(14.0, 8.0), uv(14.0, 14.0)] },
    ];
    let seeds = vec![WitnessSeed {
        id: "landia".into(),
        kind: FaceKind::LandClaim,
        seed: uv(12.0, 11.0),
    }];
    assert!(
        build_with(&[lake], &borders, &seeds, &[], &cfg()).is_err(),
        "a leaked claim is a build failure, never a rendered gap"
    );
}

/// The dam-split class is unrepresentable: a RiverSystem whose pieces
/// do not touch refuses to construct, carrying the stray coordinates.
#[test]
fn river_system_must_connect() {
    use crate::RiverSystem;
    let tol = cfg().tau_vertex;
    // connected: two paths sharing an endpoint
    let ok = RiverSystem::new(
        "arnon".into(),
        vec![
            vec![uv(31.4, 36.0), uv(31.43, 35.85)],
            vec![uv(31.43, 35.85), uv(31.47, 35.6)],
        ],
        tol,
    );
    assert!(ok.is_ok(), "touching paths are one system");
    // disconnected: a 5 km dam gap between collinear pieces
    let bad = RiverSystem::new(
        "arnon-split".into(),
        vec![
            vec![uv(31.4, 36.0), uv(31.43, 35.87)],
            vec![uv(31.44, 35.82), uv(31.47, 35.6)],
        ],
        tol,
    );
    match bad {
        Err(crate::RiverSystemError::Disconnected { pieces, at, .. }) => {
            assert_eq!(pieces, 2);
            assert!((at.0 - 31.44).abs() < 0.01, "the stray piece is named by location");
        }
        other => panic!("a split river must refuse to construct, got {other:?}"),
    }
}

/// A subdivision claims only where its parent claims: a tribe's
/// overhang beyond the land (or into the water) is dropped, and the
/// most specific claim names the face.
#[test]
fn subdivision_claims_only_within_parent() {
    let land = region("land", FaceKind::LandClaim, vec![square(10.0, 10.0, 20.0, 20.0)]);
    let mut tribe = region("tribe", FaceKind::LandClaim, vec![square(12.0, 15.0, 18.0, 25.0)]);
    tribe.parent = Some("land".into());
    let lake = region("lake", FaceKind::Lake, vec![square(13.0, 16.0, 15.0, 18.0)]);
    let p = build(&[land, tribe, lake], &[], &cfg()).unwrap();
    ok(&p);
    // the tribe face exists and is named first (most specific)
    let tribe_face = p
        .faces
        .iter()
        .find(|f| f.claims.first().map(String::as_str) == Some("tribe"))
        .expect("tribe claims a face");
    assert_eq!(tribe_face.kind, FaceKind::LandClaim);
    assert!(tribe_face.claims.contains(&"land".to_string()), "parent rides along");
    // the tribe's overhang beyond the land (lon 20..25) is NOT tribe
    for (i, f) in p.faces.iter().enumerate() {
        if f.claims.first().map(String::as_str) == Some("tribe") {
            for ring in p.face_rings(i) {
                for pt in ring {
                    let (_, lon) = pt.to_lat_lon_deg();
                    assert!(lon <= 20.0 + 1e-6, "tribe never exceeds its parent");
                }
            }
        }
    }
    // the lake inside the tribe still wins
    let lake_face = p.faces.iter().find(|f| f.kind == FaceKind::Lake).expect("lake");
    assert_eq!(lake_face.claims.first().map(String::as_str), Some("lake"));
    assert!(p.area_residual() < 1e-10);
}

/// Overlapping same-kind witnesses without hierarchy: the SMALLER
/// witness names the face — specificity is measured, never curated.
/// A tribe overlapping the country that surrounds it wins its own
/// ground even where their rings disagree; the country keeps the rest.
#[test]
fn smaller_witness_names_the_shared_face() {
    let country = region("zz-country", FaceKind::LandClaim, vec![square(10.0, 10.0, 30.0, 30.0)]);
    // id sorts AFTER "zz-country" alphabetically? no — "province" < "zz-country",
    // so give the small one a LATER id to prove area (not id order) decides.
    let province = region("zz-z-province", FaceKind::LandClaim, vec![square(12.0, 12.0, 18.0, 18.0)]);
    let p = build(&[country, province], &[], &cfg()).unwrap();
    ok(&p);
    let f = p
        .faces
        .iter()
        .find(|f| f.claims.contains(&"zz-z-province".to_string()))
        .expect("the shared face exists");
    assert_eq!(
        f.claims.first().map(String::as_str),
        Some("zz-z-province"),
        "the smaller witness names the face it shares with its country"
    );
    assert!(p.area_residual() < 1e-10);
}

/// Where a river drains is a TYPE, not a filter: a network ending in
/// (or measurably at) the water classifies Draining; one draining
/// into sand classifies Endorheic, named with its distance — and only
/// Draining can become map geometry. Ring orientation is the data's
/// business; a mouth cut short of the shoreline (within the declared
/// allowance) still drains.
#[test]
fn watershed_classification_is_typed_and_measured() {
    let pt = |la: f64, lo: f64| UnitVec::from_lat_lon_deg(la, lo);
    let lake: Vec<UnitVec> =
        [(32.0, 35.0), (32.0, 35.2), (32.2, 35.2), (32.2, 35.0)].map(|(a, o)| pt(a, o)).into();
    let mouth_gap = 5.0 / 6371.0;
    let sys = |pts: Vec<UnitVec>| RiverSystem::new("r".into(), vec![pts], 2.4e-5).unwrap();

    // ends inside the lake: draining
    assert!(matches!(
        sys(vec![pt(31.5, 35.1), pt(31.8, 35.1), pt(32.1, 35.1)]).classify(&[lake.clone()], mouth_gap),
        Watershed::Draining(_)
    ));
    // the same lake wound the other way: still draining
    let lake_cw: Vec<UnitVec> = lake.iter().rev().cloned().collect();
    assert!(matches!(
        sys(vec![pt(31.5, 35.1), pt(32.1, 35.1)]).classify(&[lake_cw], mouth_gap),
        Watershed::Draining(_)
    ));
    // a mouth ~3 km shy of the shore: within the declared allowance
    assert!(matches!(
        sys(vec![pt(31.5, 35.1), pt(31.97, 35.1)]).classify(&[lake.clone()], mouth_gap),
        Watershed::Draining(_)
    ));
    // draining into sand far east: Endorheic, with the distance named
    match sys(vec![pt(31.5, 36.5), pt(31.8, 36.6), pt(32.1, 36.7)]).classify(&[lake], mouth_gap) {
        Watershed::Endorheic { id, nearest_water_km } => {
            assert_eq!(id, "r");
            assert!(
                nearest_water_km > 100.0,
                "the verdict carries its measurement: {nearest_water_km:.0} km"
            );
        }
        Watershed::Draining(_) => panic!("a desert network never drains here"),
    }
}

