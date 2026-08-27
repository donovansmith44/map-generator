//! The bridge from the old interval-timeline model into the canon:
//! regions become layer Areas, intervals become moments (the old `to`
//! was already exclusive), labels become names, and every entity
//! carries its witness prefix. The bridge exists so no witness's data
//! is lost while the old model is retired (phase 6 deletes it).

use std::collections::{BTreeMap, BTreeSet};

use map_canon::{
    Area, Border, CanonStore, EntityId, Feature, LayerKind, Provenance, Snapshot, Timestamp,
    Witness, World,
};
use map_types::{Orientation, RegionClass, WorldTimeline};

fn slug(label: &str) -> String {
    let mut out = String::with_capacity(label.len());
    let mut dash = false;
    for ch in label.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            dash = false;
        } else if !dash && !out.is_empty() {
            out.push('-');
            dash = true;
        }
    }
    out.trim_end_matches('-').to_string()
}

/// Bridge every region of `class_filter` (None = all classes) into one
/// canon layer. Moments are the union of interval edges; at each edge
/// the snapshot holds the regions whose interval contains it.
pub fn bridge_timeline_regions(
    store: &mut CanonStore,
    tl: &WorldTimeline,
    layer: LayerKind,
    witness: Witness,
    prefix: &str,
) -> Result<(), String> {
    bridge_filtered(store, tl, layer, witness, prefix, None, &BTreeSet::new())
}

/// The full-control bridge: an optional class filter and a drop-list
/// of entity slugs (reconciliation's superseded entries).
pub fn bridge_filtered(
    store: &mut CanonStore,
    tl: &WorldTimeline,
    layer: LayerKind,
    witness: Witness,
    prefix: &str,
    class_filter: Option<RegionClass>,
    drop_slugs: &BTreeSet<String>,
) -> Result<(), String> {
    // (interval, feature) rows, then edge sweep.
    let mut rows: Vec<(map_types::Interval, map_canon::FeatureId)> = Vec::new();

    for (rid, hist) in &tl.regions {
        if let Some(filter) = class_filter {
            let matches = match (filter, hist.class) {
                (RegionClass::Land, RegionClass::Land) => true,
                (RegionClass::Water, RegionClass::Water) => true,
                (RegionClass::Terrain(_), RegionClass::Terrain(_)) => true,
                _ => false,
            };
            if !matches {
                continue;
            }
        }
        for (iv, geom) in &hist.geom_history {
            let label = hist
                .label_history
                .iter()
                .find(|(li, _)| li.intersects(iv))
                .map(|(_, l)| l.clone())
                .unwrap_or_else(|| format!("region-{:016x}", rid.0 .0));
            let entity_slug = slug(&label);
            if drop_slugs.contains(&entity_slug) {
                continue;
            }
            let mut rings = BTreeSet::new();
            let mut holes = BTreeSet::new();
            for part in &geom.parts {
                if let Some(b) = resolve_cycle(store, tl, &part.cycle, iv)? {
                    rings.insert(b);
                }
                for hc in &part.holes {
                    if let Some(b) = resolve_cycle(store, tl, hc, iv)? {
                        holes.insert(b);
                    }
                }
            }
            if rings.is_empty() {
                continue;
            }
            let fid = store.insert_feature(Feature::Area(Area {
                entity: EntityId(format!("{prefix}:{entity_slug}")),
                name: label,
                rings,
                holes,
            }));
            store.set_provenance(
                fid,
                Provenance {
                    witness,
                    verses: Vec::new(),
                    note: format!("bridged from the interval model ({prefix})"),
                },
            );
            rows.push((*iv, fid));
        }
    }

    // Edge sweep: every from and every (exclusive) to is a moment.
    let mut edges: BTreeSet<Timestamp> = BTreeSet::new();
    for (iv, _) in &rows {
        edges.insert(iv.from);
        if let Some(to) = iv.to {
            edges.insert(to);
        }
    }
    let mut world = store
        .layers()
        .get(&layer)
        .cloned()
        .unwrap_or_default();
    // Merge with any moments the layer already carries: re-sweep the
    // union of edges so combined layers stay one-state-per-instant.
    let mut all_edges = edges;
    for t in world.moments().keys() {
        all_edges.insert(*t);
    }
    let mut existing_rows: BTreeMap<Timestamp, BTreeSet<map_canon::FeatureId>> = BTreeMap::new();
    for (t, sid) in world.moments() {
        existing_rows.insert(*t, store.snapshots()[sid].features.clone());
    }
    let mut merged = World::default();
    for edge in all_edges {
        let mut active: BTreeSet<map_canon::FeatureId> = existing_rows
            .range(..=edge)
            .next_back()
            .map(|(_, f)| f.clone())
            .unwrap_or_default();
        for (iv, fid) in &rows {
            if iv.contains(&edge) {
                active.insert(*fid);
            }
        }
        let sid = store.insert_snapshot(Snapshot { features: active });
        merged
            .insert(edge, sid)
            .map_err(|_| format!("{layer:?}: contradiction while merging moments"))?;
    }
    world = merged;
    store.set_layer(layer, world);
    Ok(())
}

/// Resolve one cycle's boundary references into a single ring border
/// at the version alive during `iv`. Returns None for degenerate
/// (sub-3-point) cycles.
fn resolve_cycle(
    store: &mut CanonStore,
    tl: &WorldTimeline,
    cycle: &[(map_types::BoundaryId, Orientation)],
    iv: &map_types::Interval,
) -> Result<Option<map_canon::BorderId>, String> {
    let mut ring: Vec<map_types::UnitVec> = Vec::new();
    for (bid, orientation) in cycle {
        let hist = tl
            .boundaries
            .get(bid)
            .ok_or_else(|| format!("bridge: boundary {:016x} missing", bid.0 .0))?;
        let b = hist
            .versions
            .iter()
            .find(|(vi, _)| vi.intersects(iv))
            .map(|(_, b)| b)
            .ok_or_else(|| format!("bridge: boundary {:016x} has no version in span", bid.0 .0))?;
        let pts = &b.pts;
        if pts.is_empty() {
            continue;
        }
        match orientation {
            Orientation::Forward => ring.extend_from_slice(&pts[..pts.len() - 1]),
            Orientation::Reverse => ring.extend(pts[1..].iter().rev()),
        }
    }
    if ring.len() < 3 {
        return Ok(None);
    }
    Ok(Some(store.insert_border(Border(ring))))
}
