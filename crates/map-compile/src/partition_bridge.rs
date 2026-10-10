use std::collections::BTreeSet;

use map_canon::{
    Area, Border, CanonStore, EntityId, Feature, LayerKind, PathLine, PresenceBook, Provenance,
    Snapshot, Timestamp, Witness, World,
};
use map_partition::{
    build, cycle_area, winding, FaceKind, PartitionConfig, WitnessPolyline,
    WitnessRegion,
};
use map_types::UnitVec;

use crate::vendor::PolityRow;

fn ts_year(y: i32) -> Result<Timestamp, String> {
    use atlas_graph_types::covenant::{TimePoint, Year};
    Year::new(y).map(TimePoint::year_only).map_err(|_| format!("no such year {y}"))
}

/// The year after y, minding the missing year zero.
fn year_after(y: i32) -> i32 {
    if y == -1 { 1 } else { y + 1 }
}

/// Assemble the partition's witnesses from every source. Public so
/// the law suite builds exactly what the compiler builds.
pub fn gather_witnesses(
    polities: &[PolityRow],
) -> Result<(Vec<WitnessRegion>, Vec<WitnessPolyline>), String> {
    let polity_points: Vec<_> = polities.iter().flat_map(|row| &row.rings)
        .flatten().map(|(lat, lon)| UnitVec::from_lat_lon_deg(*lat, *lon)).collect();
    crate::exclusion::check_points(&polity_points)
        .map_err(|error| format!("excluded input: {error:?}"))?;
    crate::exclusion::check_build_inputs(&data_path("data"))
        .map_err(|error| format!("excluded input: {error:?}"))?;
    let seas = load_ne_med()?; // real coast, same family as the lakes
    let lakes = load_ne_lakes()?;

    let mut water_rings = seas.clone();
    water_rings.extend(lakes.iter().map(|(_, ring)| ring.clone()));
    let mut polylines = Vec::new();
    for river in load_ne_rivers()? {
        let system = map_partition::RiverSystem::new(
            river.name.clone(),
            river.paths,
            PartitionConfig::default().tau_edge,
        )
        .map_err(|error| format!("Natural Earth river integrity: {error:?}"))?;
        let mut run_number = 0;
        for path in system.paths {
            for run in clip_outside_water(&path, &water_rings) {
                if run.len() < 2 {
                    continue;
                }
                run_number += 1;
                polylines.push(WitnessPolyline {
                    id: format!("{}-{run_number}", river.name),
                    pts: run,
                });
            }
        }
    }

    let mut regions: Vec<WitnessRegion> = Vec::new();
    for (i, ring) in seas.into_iter().enumerate() {
        regions.push(WitnessRegion {
            id: if i == 0 { "great-sea".into() } else { format!("great-sea-{i}") },
            kind: FaceKind::Sea,
            rings: vec![ring],
            parent: None,
        });
    }
    for (name, ring) in lakes {
        regions.push(WitnessRegion { id: name, kind: FaceKind::Lake, rings: vec![ring], parent: None });
    }
    // THE BORDERING WORLD: every atlas polity era enters as a witness
    // of its own — one per (polity, era) so geometry may change at a
    // cut — and joins the ONE arrangement. Water-over-land trims each
    // ring flush at the seas and lakes by law; smaller and deeper
    // witnesses (tribes, neighbors) outrank empire-sized claims by
    // specificity; WHO STANDS WHEN is declared by the rows' own years.
    for row in polities {
        let rings: Vec<Vec<UnitVec>> = row
            .rings
            .iter()
            .filter(|r| r.len() >= 3)
            .map(|r| r.iter().map(|(lat, lon)| UnitVec::from_lat_lon_deg(*lat, *lon)).collect())
            .collect();
        if rings.is_empty() {
            continue;
        }
        regions.push(WitnessRegion {
            id: format!("{}@{}", row.id, row.from_year),
            kind: FaceKind::LandClaim,
            rings,
            parent: None,
        });
    }

    crate::exclusion::check_points(
        regions.iter().flat_map(|region| &region.rings).flatten()
            .chain(polylines.iter().flat_map(|line| &line.pts)),
    ).map_err(|error| format!("excluded input: {error:?}"))?;
    Ok((regions, polylines))
}

/// ONE Area per entity: all of an entity's faces fill as a single
/// path, so same-paint interior seams cannot render. The entity's
/// kind is its largest face's kind. `absent` names entities not yet
/// standing in the era being bundled — a face falls to its first
/// claimant that IS present.
struct Bundle {
    kind: FaceKind,
    rings: BTreeSet<map_canon::BorderId>,
    holes: BTreeSet<map_canon::BorderId>,
    note: String,
}

fn bundle_faces(
    store: &mut CanonStore,
    part: &map_partition::Partition,
    absent: &BTreeSet<String>,
) -> std::collections::BTreeMap<String, Bundle> {
    // Pass 1: gather each entity's face SET. Pass 2: DISSOLVE the set
    // into its boundary cycles (map_partition::dissolve_rings). The
    // whole-frame arrangement dices every territory with every other
    // era's borders — the sea alone once arrived as ~6,800 face rings
    // — but those interior seams are artifacts of the arrangement,
    // not geometry of the era's territory. The union's geometry IS
    // its boundary; the renderer draws cycles, not dice.
    struct Gather {
        kind: FaceKind,
        biggest: f64,
        faces: BTreeSet<usize>,
        note: String,
    }
    let mut gathers: std::collections::BTreeMap<String, Gather> =
        std::collections::BTreeMap::new();
    for (fi, face) in part.faces.iter().enumerate() {
        if face.kind == FaceKind::Background {
            continue;
        }
        let Some(who) = face.claims.iter().find(|c| !absent.contains(*c)).cloned() else {
            continue; // every claimant is yet to come: unnamed ground
        };
        let entry = gathers.entry(who).or_insert_with(|| Gather {
            kind: face.kind.clone(),
            biggest: face.area,
            faces: BTreeSet::new(),
            note: String::new(),
        });
        if face.area > entry.biggest {
            entry.biggest = face.area;
            entry.kind = face.kind.clone();
        }
        entry.faces.insert(fi);
        entry.note.push_str(&format!(
            "face {fi}: claims {:?} conflicts {:?} area {:.3e} sr; ",
            face.claims, face.conflicts, face.area
        ));
    }
    let mut bundles: std::collections::BTreeMap<String, Bundle> =
        std::collections::BTreeMap::new();
    for (who, g) in gathers {
        let mut bundle = Bundle {
            kind: g.kind,
            rings: BTreeSet::new(),
            holes: BTreeSet::new(),
            note: g.note,
        };
        for ring in part.dissolve_rings(&g.faces) {
            if ring.len() < 3 {
                continue;
            }
            let bid = store.insert_border(Border(ring.clone()));
            if cycle_area(&ring) > 0.0 {
                bundle.rings.insert(bid);
            } else {
                bundle.holes.insert(bid);
            }
        }
        bundles.insert(who, bundle);
    }
    bundles
}

/// Display names are DATA (data/atlas-vendor/names.json): exact ids,
/// then prefix matches for suffixed witnesses, then the generic
/// title-cased slug. No entity is named in code.
struct NameBook {
    exact: std::collections::BTreeMap<String, String>,
    prefix: Vec<(String, String)>,
}

fn load_names() -> Result<NameBook, String> {
    let text = std::fs::read_to_string(data_path("data/atlas-vendor/names.json"))
        .map_err(|e| format!("names: {e}"))?;
    let v: serde_json::Value = serde_json::from_str(&text).map_err(|e| format!("names: {e}"))?;
    let entries = |key: &str| -> Vec<(String, String)> {
        v[key]
            .as_object()
            .into_iter()
            .flatten()
            .filter_map(|(k, val)| Some((k.clone(), val.as_str()?.to_string())))
            .collect()
    };
    Ok(NameBook { exact: entries("exact").into_iter().collect(), prefix: entries("prefix") })
}

impl NameBook {
    fn name_of(&self, who: &str) -> String {
        if let Some(n) = self.exact.get(who) {
            return n.clone();
        }
        if let Some((_, n)) = self.prefix.iter().find(|(p, _)| who.starts_with(p.as_str())) {
            return n.clone();
        }
        who.split('-')
            .map(|part| {
                let mut cs = part.chars();
                match cs.next() {
                    Some(f) => f.to_uppercase().collect::<String>() + cs.as_str(),
                    None => String::new(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// The vendored settlement roster: (place id, display name, lat, lon).
pub(crate) fn load_settlements() -> Result<Vec<(String, String, f64, f64)>, String> {
    let text = std::fs::read_to_string(data_path("data/openbible/settlements.geojson"))
        .map_err(|e| format!("settlements: {e}"))?;
    let v: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("settlements: {e}"))?;
    let mut out = Vec::new();
    for f in v["features"].as_array().into_iter().flatten() {
        let (Some(place), Some(name)) =
            (f["properties"]["place"].as_str(), f["properties"]["name"].as_str())
        else {
            continue;
        };
        let c = &f["geometry"]["coordinates"];
        let (Some(lon), Some(lat)) = (c[0].as_f64(), c[1].as_f64()) else { continue };
        out.push((place.to_string(), name.to_string(), lat, lon));
    }
    if out.len() < 12 {
        return Err(format!("settlements: expected a plate's worth, found {}", out.len()));
    }
    Ok(out)
}


/// One entity's ground in one era, as the bridge is about to store it.
#[derive(Clone, Debug, PartialEq)]
pub struct EraArea {
    /// every minted id that resolved to this entity in this era
    pub witnessed: Vec<EntityId>,
    pub entity: EntityId,
    pub name: String,
    pub layer: LayerKind,
    pub witness: Witness,
    pub verses: Vec<String>,
    pub note: String,
    pub rings: BTreeSet<map_canon::BorderId>,
    pub holes: BTreeSet<map_canon::BorderId>,
}

/// ONE ENTITY PER ERA. Every bundle that resolves to the same entity
/// is one area: rings and holes unioned, the held witness's layer and
/// witness kept (a polity's Territory over a cohort's claim), every
/// witness's note carried. Order is the entities' first appearance.
pub fn unify_era_areas(rows: Vec<EraArea>) -> Vec<EraArea> {
    let mut out: Vec<EraArea> = Vec::new();
    for row in rows {
        match out.iter_mut().find(|a| a.entity == row.entity) {
            None => out.push(row),
            Some(a) => {
                if row.layer == LayerKind::Territory && a.layer != LayerKind::Territory {
                    a.layer = row.layer;
                    a.witness = row.witness;
                    a.verses = row.verses.clone();
                }
                a.rings.extend(row.rings);
                a.holes.extend(row.holes);
                a.witnessed.extend(row.witnessed);
                a.note = format!("{}; {}", a.note, row.note);
            }
        }
    }
    out
}

pub fn bridge_partition(
    store: &mut CanonStore,
    t0: Timestamp,
    polities: &[PolityRow],
    identity: &mut crate::identity::Identity,
) -> Result<String, String> {
    let (regions, polylines) = gather_witnesses(polities)?;
    let part = build(&regions, &polylines, &PartitionConfig::default())
        .map_err(|e| format!("partition build: {e:?}"))?;

    let residual = part.area_residual();
    let n_faces = part.faces.len();
    let n_rivers = part.rivers.len();

    let mut claim_fids: BTreeSet<map_canon::FeatureId> = BTreeSet::new();
    let mut water_fids: BTreeSet<map_canon::FeatureId> = BTreeSet::new();
    // WHO STANDS WHEN comes from the data: each cohort ring declares
    // its standing as era ids, the presence algebra derives the eras,
    // and history compiles as a fold over them. The code below knows
    // no tribe, no conquest, and no kingdom — only the laws:
    //   - a face is named by its first claimant PRESENT in the era
    //     (an absent claimant's ground falls to the next in the
    //     face's own specificity chain);
    //   - one Area per entity per era, so an entity whose ground
    //     changes at a cut morphs through the transition machinery;
    //   - content addressing dedups every era that repeats a state
    //     (a claimant leaving restores the SAME features it displaced).
    let names = load_names()?;
    // THE COHORT BOOK: how each witness's bundles enter the canon —
    // its entity (era-variants of one polity collapse to ONE entity,
    // so the transition machinery morphs the empire growing), its
    // display name, its layer, its witness kind, and its verses.
    #[derive(Clone)]
    struct CohortSpec {
        minted: EntityId,
        entity: EntityId,
        name: String,
        layer: LayerKind,
        witness: Witness,
        verses: Vec<String>,
        note_prefix: String,
    }
    let mut specs: std::collections::BTreeMap<String, CohortSpec> =
        std::collections::BTreeMap::new();
    let mut presence = PresenceBook::default();
    // Polity standings come from the rows' own years; a second book
    // keyed by ENTITY catches era-variants that would stand twice at
    // once — the same entity must not wear two witnesses at a moment.
    let mut entity_disjoint = PresenceBook::default();
    for row in polities {
        let wid = format!("{}@{}", row.id, row.from_year);
        let from = ts_year(row.from_year)?;
        let until = ts_year(year_after(row.to_year))?;
        presence
            .declare(&wid, from, Some(until))
            .map_err(|e| format!("presence for {wid}: {e:?}"))?;
        entity_disjoint
            .declare(&row.id, from, Some(until))
            .map_err(|e| format!("polity '{}': eras overlap in time ({e:?})", row.id))?;
        let mut verses = row.transition_verses.clone();
        verses.extend(row.fall_verses.iter().cloned());
        specs.insert(
            wid,
            CohortSpec {
                minted: EntityId(row.id.clone()),
                entity: identity.resolve(&EntityId(row.id.clone())).clone(),
                name: row.name.clone(),
                layer: LayerKind::Territory,
                witness: Witness::Atlas,
                verses,
                note_prefix: format!("atlas polity era {}..{}", row.from_year, row.to_year),
            },
        );
    }
    let mut era_overlays: Vec<(
        LayerKind,
        BTreeSet<map_canon::FeatureId>,
        Timestamp,
        Option<Timestamp>,
    )> = Vec::new();
    for era in presence.eras(t0) {
        let mut per_layer: std::collections::BTreeMap<LayerKind, BTreeSet<map_canon::FeatureId>> =
            std::collections::BTreeMap::new();
        let mut rows: Vec<EraArea> = Vec::new();
        for (who, bundle) in bundle_faces(store, &part, &era.absent) {
            if bundle.rings.is_empty() {
                continue;
            }
            let spec = specs.get(&who).cloned().unwrap_or_else(|| CohortSpec {
                minted: EntityId(format!("partition:{who}")),
                entity: identity.resolve(&EntityId(format!("partition:{who}"))).clone(),
                name: names.name_of(&who),
                layer: LayerKind::ScriptureClaims,
                witness: Witness::Authored,
                verses: Vec::new(),
                note_prefix: "sphere-partition entity".to_string(),
            });
            let layer = match bundle.kind {
                FaceKind::LandClaim => spec.layer,
                _ => LayerKind::Water,
            };
            rows.push(EraArea {
                witnessed: vec![spec.minted],
                entity: spec.entity,
                name: spec.name,
                layer,
                witness: spec.witness,
                verses: spec.verses,
                note: format!("{} ({})", spec.note_prefix, bundle.note),
                rings: bundle.rings,
                holes: bundle.holes,
            });
        }
        for area in unify_era_areas(rows) {
            for w in &area.witnessed {
                identity.witness(w, &area.name, area.layer, area.witness, "area");
            }
            let fid = store.insert_feature(Feature::Area(Area {
                entity: area.entity,
                name: area.name,
                rings: area.rings,
                holes: area.holes,
                tenure: map_canon::Tenure::Held,
            }));
            store.set_provenance(
                fid,
                Provenance { witness: Witness::Partition, verses: area.verses, note: area.note },
            );
            per_layer.entry(area.layer).or_default().insert(fid);
        }
        for (layer, fids) in per_layer {
            if layer == LayerKind::Water {
                // water stands outside time: overlaid once, below
                water_fids.extend(fids);
            } else {
                era_overlays.push((layer, fids, era.from, era.until));
            }
        }
    }

    for r in &part.rivers {
        if r.pts.len() < 2 {
            continue;
        }
        let bid = store.insert_border(Border(r.pts.clone()));
        let minted = EntityId(if r.id.starts_with("Jordan-") {
            "partition:jordan".into()
        } else {
            "partition:rivers".into()
        });
        identity.witness(&minted, &format!("{} (river)", r.id), LayerKind::Water, Witness::NaturalEarth, "line");
        let entity = identity.resolve(&minted).clone();
        let fid = store.insert_feature(Feature::Line(PathLine {
            entity,
            name: format!("{} (river)", r.id),
            border: bid,
        }));
        store.set_provenance(
            fid,
            Provenance {
                witness: Witness::NaturalEarth,
                verses: Vec::new(),
                note: "Natural Earth 1:10m rivers and lake centerlines (public domain; modern generalized course), noded into the partition; missing courses recorded in docs/errata/rivers.md".into(),
            },
        );
        water_fids.insert(fid);
    }

    // THE SETTLEMENTS: city dots from the vendored join of the atlas
    // gazetteer (coordinates, verse attestations) with OpenBible's
    // place typing — only dominant-sense settlements, thresholds
    // A CITY STANDS ON LAND: a settlement whose traditional site lies
    // beneath the map's waters (Sodom under the Dead Sea's south
    // basin) cannot stand on this map — refused here, by name,
    // against the same water rings the partition itself is built on.
    let water_rings_all: Vec<&Vec<UnitVec>> = regions
        .iter()
        .filter(|r| matches!(r.kind, FaceKind::Sea | FaceKind::Lake))
        .flat_map(|r| r.rings.iter())
        .collect();
    let mut n_cities = 0usize;
    let mut drowned: Vec<String> = Vec::new();
    for (place, name, lat, lon) in load_settlements()? {
        let at = UnitVec::from_lat_lon_deg(lat, lon);
        if water_rings_all.iter().any(|ring| winding(ring, &at) != 0) {
            // beneath the waters: the site becomes a MEMORY — its own
            // canon kind, rendered as an inscription, never a dot
            let minted = EntityId(format!("place:{place}"));
            identity.witness(&minted, &name, LayerKind::ScriptureClaims, Witness::Atlas, "memory");
            let fid = store.insert_feature(Feature::Memory(map_canon::Memory {
                entity: identity.resolve(&minted).clone(),
                name: name.clone(),
                at,
            }));
            store.set_provenance(
                fid,
                Provenance {
                    witness: Witness::Atlas,
                    verses: Vec::new(),
                    note: "traditional site beneath the waters (GEN 14:3).                            ENDURES to the frame's edge: a memory is kept, not governed by eras."
                        .into(),
                },
            );
            claim_fids.insert(fid);
            drowned.push(name);
            continue;
        }
        let minted = EntityId(format!("place:{place}"));
        identity.witness(&minted, &name, LayerKind::ScriptureClaims, Witness::Atlas, "point");
        let fid = store.insert_feature(Feature::Point(map_canon::Landmark {
            entity: identity.resolve(&minted).clone(),
            name,
            at,
        }));
        store.set_provenance(
            fid,
            Provenance {
                witness: Witness::Atlas,
                verses: Vec::new(),
                note: "gazetteer settlement (OpenBible-typed, see data/openbible).                        ENDURES to the frame's edge: a place-name outlives every polity."
                    .into(),
            },
        );
        claim_fids.insert(fid);
        n_cities += 1;
    }

    overlay_features(store, LayerKind::ScriptureClaims, &claim_fids, t0, None)?;
    // The era windows apply in chronological order after the claims,
    // each into its cohort's own layer (tribal cohorts into
    // ScriptureClaims, polity cohorts into Territory).
    for (layer, fids, from, until) in &era_overlays {
        overlay_features(store, *layer, fids, *from, *until)?;
    }
    overlay_features(store, LayerKind::Water, &water_fids, t0, None)?;

    crate::exclusion::check_compiled(store)
        .map_err(|error| format!("excluded output: {error:?}"))?;
    Ok(format!(
        "partition: {n_faces} faces, {n_rivers} river paths, 4π residual {residual:.2e} sr;          {n_cities} cities stand, {} remembered beneath the waters ({})",
        drowned.len(),
        drowned.join(", ")
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

/// The real Mediterranean (vendored NE land complement).
fn load_ne_med() -> Result<Vec<Vec<UnitVec>>, String> {
    let text = std::fs::read_to_string(data_path("data/natural-earth/med_clip.geojson"))
        .map_err(|e| format!("med clip: {e}"))?;
    let v: serde_json::Value = serde_json::from_str(&text).map_err(|e| format!("med clip: {e}"))?;
    crate::exclusion::check_geojson(&v)
        .map_err(|error| format!("excluded input: {error:?}"))?;
    let mut out = Vec::new();
    for f in v["features"].as_array().into_iter().flatten() {
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
    if out.is_empty() {
        return Err("med clip: empty".into());
    }
    Ok(out)
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
                    if physical_frame_contains(lat, lon) {
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

struct RiverSystemSource {
    name: String,
    paths: Vec<Vec<UnitVec>>,
}

fn physical_frame_contains(latitude: f64, longitude: f64) -> bool {
    (29.0..=34.6).contains(&latitude) && (33.5..=37.8).contains(&longitude)
}

fn load_ne_rivers() -> Result<Vec<RiverSystemSource>, String> {
    let text = std::fs::read_to_string(data_path(
        "data/natural-earth/ne_10m_rivers_lake_centerlines.geojson",
    ))
    .map_err(|error| format!("Natural Earth rivers: {error}"))?;
    let rivers = map_adapters::hydro::read_rivers(&text)
        .map_err(|error| format!("Natural Earth rivers: {error:?}"))?;
    let mut systems: std::collections::BTreeMap<
        map_adapters::hydro::RiverNumber,
        RiverSystemSource,
    > = std::collections::BTreeMap::new();
    for river in rivers {
        let paths = match river.shape {
            map_adapters::hydro::RiverShape::Course(paths) => paths,
            map_adapters::hydro::RiverShape::Unlocated => continue,
        };
        let system = systems
            .entry(river.number)
            .or_insert_with(|| RiverSystemSource {
                name: river
                    .name
                    .clone()
                    .unwrap_or_else(|| format!("unnamed-natural-earth-{}", river.number.0)),
                paths: Vec::new(),
            });
        system.paths.extend(paths);
    }
    Ok(systems
        .into_values()
        .filter(|system| {
            system.paths.iter().flatten().any(|point| {
                let (latitude, longitude) = point.to_lat_lon_deg();
                physical_frame_contains(latitude, longitude)
            })
        })
        .collect())
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
/// timeless within their span); the features join the world FROM
/// `from` onward: a moment is created at `from` if none stands there
/// (carrying forward whatever was already in effect), every moment at
/// or after `from` gains the features, and every earlier moment is
/// left exactly as it was — this is how a cohort ENTERS TIME.
fn overlay_features(
    store: &mut CanonStore,
    layer: LayerKind,
    fids: &BTreeSet<map_canon::FeatureId>,
    from: Timestamp,
    until: Option<Timestamp>,
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
    let mut edges = vec![from];
    edges.extend(until);
    for edge in edges {
        if !moments.iter().any(|(t, _)| *t == edge) {
            // a turning moment: the state already in effect there
            let inherited = world
                .state_at(&edge)
                .map(|sid| store.snapshots()[&sid].features.clone())
                .unwrap_or_default();
            moments.push((edge, inherited));
        }
    }
    let mut merged = World::default();
    for (t, mut feats) in moments {
        if t >= from && until.map_or(true, |u| t < u) {
            feats.extend(fids.iter().copied());
        }
        let sid = store.insert_snapshot(Snapshot { features: feats });
        merged.insert(t, sid).map_err(|_| format!("{layer:?}: moment contradiction"))?;
    }
    store.set_layer(layer, merged);
    Ok(())
}

#[cfg(test)]
pub(crate) fn load_settlements_for_law() -> Result<Vec<(String, String, f64, f64)>, String> {
    load_settlements()
}

#[cfg(test)]
pub(crate) fn overlay_features_for_law(
    store: &mut CanonStore,
    layer: LayerKind,
    fids: &BTreeSet<map_canon::FeatureId>,
    from: Timestamp,
) -> Result<(), String> {
    overlay_features(store, layer, fids, from, None)
}

#[cfg(test)]
pub(crate) fn overlay_features_for_law_span(
    store: &mut CanonStore,
    layer: LayerKind,
    fids: &BTreeSet<map_canon::FeatureId>,
    from: Timestamp,
    until: Option<Timestamp>,
) -> Result<(), String> {
    overlay_features(store, layer, fids, from, until)
}
