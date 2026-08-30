//! The sphere partition enters the canon: witnesses build ONE closed
//! arrangement (map-partition), and its faces and river paths become
//! canon features. Flushness is structural — adjacent faces share
//! their border arcs — and the completeness law (Σ areas = 4π) is
//! checked at compile time.
//!
//! Witness sourcing: the PLATE witnesses the historical region shapes
//! (Canaan) and the Great Sea's frame; the LAKES are Natural Earth's
//! real geometry; the RIVERS are OpenStreetMap's real connected
//! drainage (ways share exact nodes, so junctions meet in the data,
//! not by stitching). Rivers are clipped at the water witnesses so a
//! mouth ends exactly on the shoreline it flows into.

use std::collections::BTreeSet;

use map_canon::{
    Area, Border, CanonStore, EntityId, Feature, LayerKind, PathLine, Provenance, Snapshot,
    Timestamp, Witness, World,
};
use map_partition::{
    build, cycle_area, winding, FaceKind, PartitionConfig, WitnessPolyline,
    WitnessRegion,
};
use map_types::UnitVec;

/// Assemble the partition's witnesses from every source. Public so
/// the law suite builds exactly what the compiler builds.
pub fn gather_witnesses() -> Result<(Vec<WitnessRegion>, Vec<WitnessPolyline>), String> {
    let canaan = map_adapters::plate_canaan_ring();
    let (seas, _plate_lakes) = map_adapters::plate_water_witnesses();
    let lakes = load_ne_lakes()?;

    // rivers: OSM's connected network, clipped at the water witnesses
    let mut water_rings: Vec<Vec<UnitVec>> = seas.clone();
    water_rings.extend(lakes.iter().map(|(_, r)| r.clone()));
    // THE TYPE GATE: every network passes through RiverSystem, whose
    // constructor refuses a disconnected system — the dam-split class
    // (one river arriving as separate collinear pieces) cannot reach
    // the canon; it dies here with coordinates. Clipping at water may
    // legitimately split a system (a river through a lake), so the
    // gate runs on the UNCLIPPED network and clipping happens after.
    let mut by_net: std::collections::BTreeMap<String, (String, Vec<Vec<UnitVec>>)> =
        std::collections::BTreeMap::new();
    for (name, net, pts) in load_osm_rivers(30.0)? {
        let e = by_net.entry(net).or_insert_with(|| (name.clone(), Vec::new()));
        if e.0.is_empty() {
            e.0 = name;
        }
        e.1.push(pts);
    }
    let mut polylines: Vec<WitnessPolyline> = Vec::new();
    let mut jordan_n = 0usize;
    let mut river_n = 0usize;
    for (net, (name, paths)) in by_net {
        let system = map_partition::RiverSystem::new(
            if name.is_empty() { format!("network-{net}") } else { name.clone() },
            paths,
            PartitionConfig::default().tau_edge,
        )
        .map_err(|e| format!("river system integrity: {e:?}"))?;
        for pts in system.paths {
            for run in clip_outside_water(&pts, &water_rings) {
                if run.len() < 2 {
                    continue;
                }
                let id = if name.contains("Jordan") {
                    jordan_n += 1;
                    format!("jordan-{jordan_n}")
                } else {
                    river_n += 1;
                    format!("river-{river_n}")
                };
                polylines.push(WitnessPolyline { id, pts: run });
            }
        }
    }

    // ALONG WATER THE REGION HAS NO BORDER OF ITS OWN — and it does
    // not copy one either. The border river (the Jordan) becomes a
    // thin CORRIDOR FACE (a ~400 m buffer of its centerline), so it
    // can win overlaps exactly like a lake. The canaan ring then just
    // pushes generously INTO every adjacent water body, and the
    // classification clips it back to the water's own boundary:
    // one mechanism for sea, lakes, and river — no snapping, no
    // splices, no transition chords at the lake tips.
    let jordan_paths: Vec<Vec<UnitVec>> = polylines
        .iter()
        .filter(|p| p.id.starts_with("jordan"))
        .map(|p| p.pts.clone())
        .collect();
    // the corridor is raster-buffered at vendor time (a meandering
    // centerline buffered on the sphere self-intersects; a raster
    // union cannot)
    let corridors: Vec<Vec<UnitVec>> = load_osm_corridors()?;

    // (region-to-water overlap is prepared in raster space at
    // vendor time — see tools/plate_trace/trace_green.py)

    let mut regions: Vec<WitnessRegion> = Vec::new();
    regions.push(WitnessRegion {
        id: "canaan".into(),
        kind: FaceKind::LandClaim,
        rings: vec![canaan],
    });
    for (i, ring) in seas.into_iter().enumerate() {
        regions.push(WitnessRegion {
            id: if i == 0 { "great-sea".into() } else { format!("great-sea-{i}") },
            kind: FaceKind::Sea,
            rings: vec![ring],
        });
    }
    for (name, ring) in lakes {
        regions.push(WitnessRegion { id: name, kind: FaceKind::Lake, rings: vec![ring] });
    }
    for ring in corridors {
        if ring.len() >= 3 {
            regions.push(WitnessRegion {
                id: "jordan".into(),
                kind: FaceKind::Lake,
                rings: vec![ring],
            });
        }
    }
    Ok((regions, polylines))
}

/// Push ring points INTO adjacent water: through a lake/sea boundary
/// to `depth` beyond it (points already inside stay), and just past a
/// river centerline so the corridor face owns the crossing. The water
/// then clips the region back to its own boundary — flush without
/// copying anyone's line.
#[allow(clippy::too_many_arguments)]
fn push_into_water(
    ring: &[UnitVec],
    lakes: &[Vec<UnitVec>],
    seas: &[Vec<UnitVec>],
    rivers: &[Vec<UnitVec>],
    shore_reach: f64,
    shore_depth: f64,
    river_reach: f64,
    river_depth: f64,
) -> Vec<UnitVec> {
    let past = |p: &UnitVec, q: &UnitVec, extra: f64| -> Option<UnitVec> {
        let d = p.angle_to(q);
        if d < 1e-12 {
            return None;
        }
        // walk from p through q, distance d + extra
        let t = (d + extra) / d;
        let (sa, sb) = (((1.0 - t) * d).sin() / d.sin(), (t * d).sin() / d.sin());
        UnitVec::normalize(
            sa * p.x() + sb * q.x(),
            sa * p.y() + sb * q.y(),
            sa * p.z() + sb * q.z(),
        )
        .ok()
    };
    let nearest_on_lines = |p: &UnitVec, lines: &[Vec<UnitVec>], closed: bool| -> Option<(UnitVec, f64)> {
        let mut best: Option<(UnitVec, f64)> = None;
        for line in lines {
            let t = SnapTarget { pts: line.clone(), closed, budget: 0.0 };
            if let Some((_, _, q, d)) = nearest_on_target(p, &t) {
                if best.as_ref().map_or(true, |(_, bd)| d < *bd) {
                    best = Some((q, d));
                }
            }
        }
        best
    };
    ring.iter()
        .map(|p| {
            // already inside a shoreline body: leave it (overlap done)
            let in_shore = lakes.iter().chain(seas.iter()).any(|r| winding(r, p) == 1);
            if in_shore {
                return *p;
            }
            let shore = nearest_on_lines(p, lakes, true)
                .into_iter()
                .chain(nearest_on_lines(p, seas, true))
                .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
            let river = nearest_on_lines(p, rivers, false);
            match (shore, river) {
                (Some((q, ds)), r)
                    if ds <= shore_reach
                        && r.as_ref().map_or(true, |(_, dr)| ds <= *dr || *dr > river_reach) =>
                {
                    past(p, &q, shore_depth).unwrap_or(*p)
                }
                (_, Some((q, dr))) if dr <= river_reach => {
                    past(p, &q, river_depth).unwrap_or(*p)
                }
                _ => *p,
            }
        })
        .collect()
}

struct SnapTarget {
    pts: Vec<UnitVec>,
    closed: bool,
    /// how far this target reaches for ring points (radians)
    budget: f64,
}

/// Subdivide ring segments so no chord exceeds `max_step` radians —
/// a dense ring snaps continuously instead of chording past meanders.
fn densify_ring(ring: &[UnitVec], max_step: f64) -> Vec<UnitVec> {
    let mut out = Vec::with_capacity(ring.len() * 2);
    let n = ring.len();
    for i in 0..n {
        let a = ring[i];
        let b = ring[(i + 1) % n];
        out.push(a);
        let d = a.angle_to(&b);
        let steps = (d / max_step).ceil() as usize;
        for k in 1..steps {
            let f = k as f64 / steps as f64;
            if let Ok(m) = UnitVec::normalize(
                a.x() * (1.0 - f) + b.x() * f,
                a.y() * (1.0 - f) + b.y() * f,
                a.z() * (1.0 - f) + b.z() * f,
            ) {
                out.push(m);
            }
        }
    }
    out
}

/// Nearest point on a target polyline to `p`: (segment index, param
/// in [0,1], the point, angular distance).
fn nearest_on_target(p: &UnitVec, t: &SnapTarget) -> Option<(usize, f64, UnitVec, f64)> {
    let n = t.pts.len();
    let segs = if t.closed { n } else { n - 1 };
    let mut best: Option<(usize, f64, UnitVec, f64)> = None;
    for s in 0..segs {
        let a = t.pts[s];
        let b = t.pts[(s + 1) % n];
        let (nx, ny, nz) = a.cross_raw(&b);
        let nn = (nx * nx + ny * ny + nz * nz).sqrt();
        if nn < 1e-12 {
            continue;
        }
        let d = (p.x() * nx + p.y() * ny + p.z() * nz) / nn;
        let proj =
            UnitVec::normalize(p.x() - d * nx / nn, p.y() - d * ny / nn, p.z() - d * nz / nn);
        let full = a.angle_to(&b);
        let (q, tt) = match proj {
            Ok(pr) if (pr.angle_to(&a) + pr.angle_to(&b) - full).abs() < full * 0.02 + 1e-9 => {
                (pr, if full > 0.0 { pr.angle_to(&a) / full } else { 0.0 })
            }
            _ => {
                if p.angle_to(&a) <= p.angle_to(&b) {
                    (a, 0.0)
                } else {
                    (b, 1.0)
                }
            }
        };
        let dist = p.angle_to(&q);
        if best.as_ref().map_or(true, |(_, _, _, bd)| dist < *bd) {
            best = Some((s, tt, q, dist));
        }
    }
    best
}

/// Build the partition and bridge it into the store. Returns a
/// human-readable summary line.
pub fn bridge_partition(store: &mut CanonStore, t0: Timestamp) -> Result<String, String> {
    let (regions, polylines) = gather_witnesses()?;
    let part = build(&regions, &polylines, &PartitionConfig::default())
        .map_err(|e| format!("partition build: {e:?}"))?;

    let residual = part.area_residual();
    let n_faces = part.faces.len();
    let n_rivers = part.rivers.len();

    let prov = |note: String| Provenance { witness: Witness::Authored, verses: Vec::new(), note };
    let mut claim_fids: BTreeSet<map_canon::FeatureId> = BTreeSet::new();
    let mut water_fids: BTreeSet<map_canon::FeatureId> = BTreeSet::new();
    // ONE Area per entity: all of an entity's faces fill as a single
    // path, so same-paint interior seams cannot render. The entity's
    // kind is its largest face's kind.
    struct Bundle {
        kind: FaceKind,
        biggest: f64,
        rings: BTreeSet<map_canon::BorderId>,
        holes: BTreeSet<map_canon::BorderId>,
        note: String,
    }
    let mut bundles: std::collections::BTreeMap<String, Bundle> =
        std::collections::BTreeMap::new();
    for (fi, face) in part.faces.iter().enumerate() {
        if face.kind == FaceKind::Background {
            continue;
        }
        let who = face.claims.first().cloned().unwrap_or_else(|| format!("face-{fi}"));
        let rings_pts = part.face_rings(fi);
        let entry = bundles.entry(who).or_insert_with(|| Bundle {
            kind: face.kind.clone(),
            biggest: face.area,
            rings: BTreeSet::new(),
            holes: BTreeSet::new(),
            note: String::new(),
        });
        if face.area > entry.biggest {
            entry.biggest = face.area;
            entry.kind = face.kind.clone();
        }
        for ring in &rings_pts {
            if ring.len() < 3 {
                continue;
            }
            let bid = store.insert_border(Border(ring.clone()));
            if cycle_area(ring) > 0.0 {
                entry.rings.insert(bid);
            } else {
                entry.holes.insert(bid);
            }
        }
        entry.note.push_str(&format!(
            "face {fi}: claims {:?} conflicts {:?} area {:.3e} sr; ",
            face.claims, face.conflicts, face.area
        ));
    }
    for (who, bundle) in bundles {
        if bundle.rings.is_empty() {
            continue;
        }
        let entity = EntityId(format!("partition:{who}"));
        let name = match who.as_str() {
            "canaan" => "Canaan".to_string(),
            "jordan" => "the Jordan".to_string(),
            w if w.starts_with("sea-of-galilee") => "the Sea of Galilee".to_string(),
            w if w.starts_with("dead-sea") => "the Dead Sea".to_string(),
            w if w.starts_with("great-sea") => "the Great Sea".to_string(),
            w => w.to_string(),
        };
        let fid = store.insert_feature(Feature::Area(Area {
            entity,
            name,
            rings: bundle.rings,
            holes: bundle.holes,
        }));
        store.set_provenance(fid, prov(format!("sphere-partition entity ({})", bundle.note)));
        match bundle.kind {
            FaceKind::LandClaim => {
                claim_fids.insert(fid);
            }
            _ => {
                water_fids.insert(fid);
            }
        }
    }

    for r in &part.rivers {
        if r.pts.len() < 2 {
            continue;
        }
        let bid = store.insert_border(Border(r.pts.clone()));
        let entity = if r.id.starts_with("jordan") {
            EntityId("partition:jordan".into())
        } else {
            EntityId("partition:rivers".into())
        };
        let fid = store.insert_feature(Feature::Line(PathLine {
            entity,
            name: format!("{} (river)", r.id),
            border: bid,
        }));
        store.set_provenance(
            fid,
            prov("river from OpenStreetMap (ODbL), noded into the partition".into()),
        );
        water_fids.insert(fid);
    }

    overlay_features(store, LayerKind::ScriptureClaims, &claim_fids, t0)?;
    overlay_features(store, LayerKind::Water, &water_fids, t0)?;

    Ok(format!(
        "partition: {n_faces} faces, {n_rivers} river paths, 4π residual {residual:.2e} sr"
    ))
}

/// Repo-root-relative data path that works from both the compiled
/// binary (run at the root) and the test harness (run in the crate).
fn data_path(rel: &str) -> std::path::PathBuf {
    let direct = std::path::PathBuf::from(rel);
    if direct.exists() {
        return direct;
    }
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}

/// Natural Earth lakes inside the frame, by name.
fn load_ne_lakes() -> Result<Vec<(String, Vec<UnitVec>)>, String> {
    let text = std::fs::read_to_string(data_path("data/natural-earth/ne_10m_lakes.geojson"))
        .map_err(|e| format!("ne lakes: {e}"))?;
    let v: serde_json::Value = serde_json::from_str(&text).map_err(|e| format!("ne lakes: {e}"))?;
    let mut out = Vec::new();
    let wanted = ["Sea of Galilee", "Dead Sea"];
    for f in v["features"].as_array().into_iter().flatten() {
        let Some(name) = f["properties"]["name"].as_str() else { continue };
        if !wanted.contains(&name) {
            continue;
        }
        let g = &f["geometry"];
        let polys: Vec<&serde_json::Value> = match g["type"].as_str() {
            Some("Polygon") => vec![&g["coordinates"]],
            Some("MultiPolygon") => g["coordinates"].as_array().into_iter().flatten().collect(),
            _ => continue,
        };
        for (i, poly) in polys.into_iter().enumerate() {
            let Some(outer) = poly.as_array().and_then(|r| r.first()) else { continue };
            let ring: Vec<UnitVec> = outer
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|c| {
                    let lon = c[0].as_f64()?;
                    let lat = c[1].as_f64()?;
                    // frame guard
                    if (29.0..=34.6).contains(&lat) && (33.5..=37.8).contains(&lon) {
                        Some(UnitVec::from_lat_lon_deg(lat, lon))
                    } else {
                        None
                    }
                })
                .collect();
            if ring.len() >= 3 {
                let slug = name.to_lowercase().replace(' ', "-");
                out.push((format!("{slug}-{i}"), ring));
            }
        }
    }
    if out.is_empty() {
        return Err("ne lakes: none found in frame".into());
    }
    Ok(out)
}

/// OSM rivers from the vendored geojson: only networks whose total
/// length reaches `min_km` (the plate draws rivers, not ditches).
fn load_osm_rivers(min_km: f64) -> Result<Vec<(String, String, Vec<UnitVec>)>, String> {
    let text = std::fs::read_to_string(data_path("data/osm/rivers.geojson"))
        .map_err(|e| format!("osm rivers: {e}"))?;
    let v: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("osm rivers: {e}"))?;
    let feats: Vec<&serde_json::Value> =
        v["features"].as_array().into_iter().flatten().collect();
    // total length per network
    use std::collections::BTreeMap;
    let mut net_len: BTreeMap<String, f64> = BTreeMap::new();
    let line_of = |f: &serde_json::Value| -> Vec<UnitVec> {
        f["geometry"]["coordinates"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|c| Some(UnitVec::from_lat_lon_deg(c[1].as_f64()?, c[0].as_f64()?)))
            .collect()
    };
    let len_km = |pts: &[UnitVec]| -> f64 {
        pts.windows(2).map(|w| w[0].angle_to(&w[1]) * 6371.0).sum()
    };
    for f in &feats {
        let net = f["properties"]["network"].as_str().unwrap_or("").to_string();
        let pts = line_of(f);
        *net_len.entry(net).or_insert(0.0) += len_km(&pts);
    }
    let mut out = Vec::new();
    for f in &feats {
        let net = f["properties"]["network"].as_str().unwrap_or("");
        if net_len.get(net).copied().unwrap_or(0.0) < min_km {
            continue;
        }
        let pts = line_of(f);
        if pts.len() >= 2 {
            let name = f["properties"]["name"].as_str().unwrap_or("").to_string();
            out.push((name, net.to_string(), pts));
        }
    }
    Ok(out)
}

/// The vendored Jordan corridor polygons (raster-buffered).
fn load_osm_corridors() -> Result<Vec<Vec<UnitVec>>, String> {
    let text = std::fs::read_to_string(data_path("data/osm/rivers.geojson"))
        .map_err(|e| format!("osm rivers: {e}"))?;
    let v: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("osm rivers: {e}"))?;
    let mut out = Vec::new();
    for f in v["features"].as_array().into_iter().flatten() {
        if !f["properties"]["corridor"].as_bool().unwrap_or(false) {
            continue;
        }
        let Some(outer) = f["geometry"]["coordinates"].as_array().and_then(|r| r.first()) else {
            continue;
        };
        let ring: Vec<UnitVec> = outer
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|c| Some(UnitVec::from_lat_lon_deg(c[1].as_f64()?, c[0].as_f64()?)))
            .collect();
        if ring.len() >= 3 {
            out.push(ring);
        }
    }
    Ok(out)
}

/// Split a polyline into the runs OUTSIDE every water ring, keeping
/// one crossing point on each side so a mouth touches the shoreline.
fn clip_outside_water(pts: &[UnitVec], water: &[Vec<UnitVec>]) -> Vec<Vec<UnitVec>> {
    let inside = |p: &UnitVec| water.iter().any(|r| winding(r, p) == 1);
    let flags: Vec<bool> = pts.iter().map(inside).collect();
    let mut runs = Vec::new();
    let mut cur: Vec<UnitVec> = Vec::new();
    for i in 0..pts.len() {
        if !flags[i] {
            if cur.is_empty() && i > 0 && flags[i - 1] {
                // the mouth touches the TRUE shoreline crossing — an
                // interior lake point would cut the corner over land
                if let Some(x) = shoreline_crossing(&pts[i], &pts[i - 1], water) {
                    cur.push(x);
                }
            }
            cur.push(pts[i]);
        } else {
            if !cur.is_empty() {
                if let Some(x) = shoreline_crossing(&pts[i - 1], &pts[i], water) {
                    cur.push(x);
                }
                runs.push(std::mem::take(&mut cur));
            }
        }
    }
    if !cur.is_empty() {
        runs.push(cur);
    }
    runs
}

/// Where the arc from `out` (on land) to `inw` (in water) crosses a
/// water ring: the crossing nearest to `out`.
fn shoreline_crossing(out: &UnitVec, inw: &UnitVec, water: &[Vec<UnitVec>]) -> Option<UnitVec> {
    let (n1x, n1y, n1z) = out.cross_raw(inw);
    let full = out.angle_to(inw);
    let mut best: Option<(f64, UnitVec)> = None;
    for ring in water {
        let n = ring.len();
        for s in 0..n {
            let a = ring[s];
            let b = ring[(s + 1) % n];
            let (n2x, n2y, n2z) = a.cross_raw(&b);
            let px = n1y * n2z - n1z * n2y;
            let py = n1z * n2x - n1x * n2z;
            let pz = n1x * n2y - n1y * n2x;
            if (px * px + py * py + pz * pz).sqrt() < 1e-14 {
                continue;
            }
            for sign in [1.0f64, -1.0] {
                let Ok(c) = UnitVec::normalize(sign * px, sign * py, sign * pz) else { continue };
                let on_seg = |p: &UnitVec, u: &UnitVec, v: &UnitVec| {
                    let f = u.angle_to(v);
                    (p.angle_to(u) + p.angle_to(v) - f).abs() < 1e-9 + f * 1e-6
                };
                if on_seg(&c, out, inw) && on_seg(&c, &a, &b) {
                    let d = c.angle_to(out);
                    if d <= full + 1e-12 && best.as_ref().map_or(true, |(bd, _)| d < *bd) {
                        best = Some((d, c));
                    }
                }
            }
        }
    }
    best.map(|(_, c)| c)
}

/// Add features to EVERY moment of a layer (partition features are
/// timeless); create the layer's first moment at t0 if it is empty.
fn overlay_features(
    store: &mut CanonStore,
    layer: LayerKind,
    fids: &BTreeSet<map_canon::FeatureId>,
    t0: Timestamp,
) -> Result<(), String> {
    if fids.is_empty() {
        return Ok(());
    }
    let world = store.layers().get(&layer).cloned().unwrap_or_default();
    let mut moments: Vec<(Timestamp, BTreeSet<map_canon::FeatureId>)> = world
        .moments()
        .iter()
        .map(|(t, sid)| (*t, store.snapshots()[sid].features.clone()))
        .collect();
    if moments.is_empty() {
        moments.push((t0, BTreeSet::new()));
    }
    let mut merged = World::default();
    for (t, mut feats) in moments {
        feats.extend(fids.iter().copied());
        let sid = store.insert_snapshot(Snapshot { features: feats });
        merged.insert(t, sid).map_err(|_| format!("{layer:?}: moment contradiction"))?;
    }
    store.set_layer(layer, merged);
    Ok(())
}
