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
    build, cycle_area, winding, FaceKind, Partition, PartitionConfig, WitnessPolyline,
    WitnessRegion,
};
use map_types::UnitVec;

/// Assemble the partition's witnesses from every source. Public so
/// the law suite builds exactly what the compiler builds.
pub fn gather_witnesses() -> Result<(Vec<WitnessRegion>, Vec<WitnessPolyline>), String> {
    let canaan = map_adapters::plate_canaan_ring();
    let (seas, _plate_lakes) = map_adapters::plate_water_witnesses();

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
    // lakes: Natural Earth's real geometry (the rivers are real, so
    // the lakes they flow into must be the real ones too)
    for (name, ring) in load_ne_lakes()? {
        regions.push(WitnessRegion { id: name, kind: FaceKind::Lake, rings: vec![ring] });
    }

    // rivers: OSM's connected network, clipped at the water witnesses
    let water_rings: Vec<Vec<UnitVec>> = regions
        .iter()
        .filter(|r| r.kind != FaceKind::LandClaim)
        .flat_map(|r| r.rings.iter().cloned())
        .collect();
    let mut polylines: Vec<WitnessPolyline> = Vec::new();
    let mut jordan_n = 0usize;
    let mut river_n = 0usize;
    for (name, pts) in load_osm_rivers(30.0)? {
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
    Ok((regions, polylines))
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
    for (fi, face) in part.faces.iter().enumerate() {
        if face.kind == FaceKind::Background {
            continue;
        }
        let rings_pts = part.face_rings(fi);
        let mut rings = BTreeSet::new();
        let mut holes = BTreeSet::new();
        for ring in &rings_pts {
            if ring.len() < 3 {
                continue;
            }
            let bid = store.insert_border(Border(ring.clone()));
            if cycle_area(ring) > 0.0 {
                rings.insert(bid);
            } else {
                holes.insert(bid);
            }
        }
        if rings.is_empty() {
            continue;
        }
        let who = face.claims.first().cloned().unwrap_or_else(|| format!("face-{fi}"));
        let entity = EntityId(format!("partition:{who}"));
        let name = match face.kind {
            FaceKind::LandClaim => format!("{who} (partition)"),
            FaceKind::Sea => "the Great Sea (partition)".to_string(),
            FaceKind::Lake => format!("{who} (partition)"),
            FaceKind::Background => unreachable!(),
        };
        let fid = store.insert_feature(Feature::Area(Area { entity, name, rings, holes }));
        store.set_provenance(
            fid,
            prov(format!(
                "sphere-partition face (claims: {:?}; conflicts: {:?}; area {:.3e} sr)",
                face.claims, face.conflicts, face.area
            )),
        );
        match face.kind {
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
    for chain in river_edge_chains(&part) {
        if chain.len() < 2 {
            continue;
        }
        let bid = store.insert_border(Border(chain));
        let fid = store.insert_feature(Feature::Line(PathLine {
            entity: EntityId("partition:jordan".into()),
            name: "river on the border (partition)".into(),
            border: bid,
        }));
        store.set_provenance(fid, prov("sphere-partition border river".into()));
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
fn load_osm_rivers(min_km: f64) -> Result<Vec<(String, Vec<UnitVec>)>, String> {
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
            out.push((name, pts));
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
                cur.push(pts[i - 1]); // reach back to touch the shore
            }
            cur.push(pts[i]);
        } else {
            if !cur.is_empty() {
                cur.push(pts[i]); // reach forward to touch the shore
                runs.push(std::mem::take(&mut cur));
            }
        }
    }
    if !cur.is_empty() {
        runs.push(cur);
    }
    runs
}

/// Maximal chains of river-attributed border edges, as point paths.
fn river_edge_chains(p: &Partition) -> Vec<Vec<UnitVec>> {
    use std::collections::BTreeMap;
    let mut adj: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    let river_edges: Vec<usize> = (0..p.edges.len()).filter(|&e| p.edges[e].river).collect();
    for &e in &river_edges {
        adj.entry(p.edges[e].a).or_default().push(e);
        adj.entry(p.edges[e].b).or_default().push(e);
    }
    let mut used = vec![false; p.edges.len()];
    let mut chains = Vec::new();
    let mut starts: Vec<usize> =
        adj.iter().filter(|(_, es)| es.len() == 1).map(|(&v, _)| v).collect();
    starts.extend(adj.keys().copied());
    for s in starts {
        let Some(es) = adj.get(&s) else { continue };
        for &e0 in es {
            if used[e0] {
                continue;
            }
            let mut chain = vec![s];
            let mut v = s;
            let mut e = e0;
            loop {
                used[e] = true;
                let w = if p.edges[e].a == v { p.edges[e].b } else { p.edges[e].a };
                chain.push(w);
                v = w;
                match adj.get(&v).and_then(|es| es.iter().find(|&&x| !used[x])) {
                    Some(&nxt) => e = nxt,
                    None => break,
                }
            }
            chains.push(chain.into_iter().map(|vi| p.vertices[vi]).collect());
        }
    }
    chains
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
