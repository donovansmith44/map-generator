//! The sphere partition enters the canon: plate witnesses build ONE
//! closed arrangement (map-partition), and its faces and river paths
//! become canon features. Flushness is no longer an emission trick —
//! adjacent faces share their border arcs by construction, and the
//! completeness law (Σ areas = 4π) is checked at compile time.

use std::collections::BTreeSet;

use map_canon::{
    Area, Border, CanonStore, EntityId, Feature, LayerKind, PathLine, Provenance, Snapshot,
    Timestamp, Witness, World,
};
use map_partition::{
    build, cycle_area, FaceKind, Partition, PartitionConfig, WitnessPolyline, WitnessRegion,
};
use map_types::UnitVec;

/// Build the partition from the plate witnesses and bridge it into
/// the store. Returns a human-readable summary line.
pub fn bridge_partition(store: &mut CanonStore, t0: Timestamp) -> Result<String, String> {
    // ---- witnesses
    let canaan = map_adapters::plate_canaan_ring();
    let (seas, lakes) = map_adapters::plate_water_witnesses();
    let (rivers, jordan) = map_adapters::plate_river_paths();

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
    let mut polylines: Vec<WitnessPolyline> = Vec::new();
    for (i, pts) in jordan.into_iter().enumerate() {
        polylines.push(WitnessPolyline { id: format!("jordan-{i}"), pts });
    }
    for (i, pts) in rivers.into_iter().enumerate() {
        polylines.push(WitnessPolyline { id: format!("river-{i}"), pts });
    }

    let part = build(&regions, &polylines, &PartitionConfig::default())
        .map_err(|e| format!("partition build: {e:?}"))?;

    let residual = part.area_residual();
    let n_faces = part.faces.len();
    let n_rivers = part.rivers.len();

    // ---- faces -> canon Areas
    let prov = |note: String| Provenance {
        witness: Witness::Authored,
        verses: Vec::new(),
        note,
    };
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

    // ---- rivers -> canon Lines (overlay paths + river border chains)
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
            name: format!("{} (partition river)", r.id),
            border: bid,
        }));
        store.set_provenance(fid, prov("sphere-partition river overlay".into()));
        water_fids.insert(fid);
    }
    // border edges that ARE rivers: assemble maximal chains
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

/// Maximal chains of river-attributed border edges, as point paths.
fn river_edge_chains(p: &Partition) -> Vec<Vec<UnitVec>> {
    use std::collections::BTreeMap;
    let mut adj: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    let river_edges: Vec<usize> =
        (0..p.edges.len()).filter(|&e| p.edges[e].river).collect();
    for &e in &river_edges {
        adj.entry(p.edges[e].a).or_default().push(e);
        adj.entry(p.edges[e].b).or_default().push(e);
    }
    let mut used = vec![false; p.edges.len()];
    let mut chains = Vec::new();
    // start at chain ends (degree 1) first, then cycles
    let mut starts: Vec<usize> = adj
        .iter()
        .filter(|(_, es)| es.len() == 1)
        .map(|(&v, _)| v)
        .collect();
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
