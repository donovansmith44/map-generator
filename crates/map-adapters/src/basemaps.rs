//! The historical-basemaps adapter (spec first source; phase 2).
//!
//! The source gives world STATES at coarse epochs. The adapter is
//! honest about that: epochs become piecewise intervals; epoch-to-epoch
//! differences are narrated as Rise/Fall/Shift events with Source
//! grounds (no fabricated narrative); every arc's kind of edge is
//! `EdgeCharacter::Unknown` because the source does not say — finer
//! granularity arrives by adding sources and events, not by
//! interpolating fiction.
//!
//! Everything excluded is a typed, counted exemption — never a silent
//! skip. Pre-anchor epochs are excluded per the configured anchor
//! (owner ruling: the anchor is a parameter; under the biblical frame,
//! nothing precedes creation).

use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, BTreeSet};
use std::hash::{Hash, Hasher};

use atlas_graph_types::chrono::{TimePoint, Year};
use atlas_graph_types::edge::{Ground, Justification};
use atlas_graph_types::id::{ContentHash, SourceId};

use map_types::{
    Anchor, Boundary, BoundaryHistory, BoundaryId, BoundarySource, ChangeEvent, ChangeKind,
    EdgeCharacter, Interval, Orientation, RegionGeom, RegionHistory, RegionId, RegionPart,
    UnitVec, WorldTimeline,
};

use crate::arcs::{extract, ArcDir, Extraction};
use crate::geojson::{parse_features, ParseError, SourceFeature};
use crate::quantize::{clean_ring, QPoint};

/// One epoch file: a year (negative = BC, no zero) and its text.
#[derive(Clone, Debug)]
pub struct EpochSource {
    pub year: i32,
    pub label: String,
    pub text: String,
}

/// "world_bc2000" -> -2000, "world_100" -> 100.
pub fn epoch_year_from_label(label: &str) -> Option<i32> {
    let rest = label.strip_prefix("world_")?;
    match rest.strip_prefix("bc") {
        Some(n) => n.parse::<i32>().ok().map(|y| -y),
        None => rest.parse::<i32>().ok(),
    }
}

#[derive(Clone, Debug)]
pub struct IngestConfig {
    pub source: SourceId,
    pub anchor: Option<Anchor>,
}

/// Typed, counted, enumerable — the atlas's exemption discipline.
#[derive(Clone, Debug, PartialEq)]
pub enum Exemption {
    /// The epoch predates the configured anchor's first event.
    PreAnchorEpoch { label: String, year: i32 },
    /// Features the source leaves anonymous (unattributed land).
    UnnamedFeatures { year: i32, count: usize },
    /// Rings degenerate after quantization (under three points).
    DegenerateRings { year: i32, count: usize },
}

#[derive(Clone, Debug, PartialEq)]
pub enum IngestError {
    Parse(String, ParseError),
    /// Year zero does not exist in the shared chrono vocabulary.
    BadYear(String, i32),
}

#[derive(Clone, Debug)]
pub struct Ingest {
    pub timeline: WorldTimeline,
    pub exemptions: Vec<Exemption>,
}

/// A source adapter turns ONE source's bytes into a lawful timeline.
/// The interface is the reversibility seam (covenant rule 7): the next
/// source implements this trait and nothing downstream notices.
pub trait TimelineSource {
    fn source(&self) -> SourceId;
    fn ingest(&self) -> Result<Ingest, IngestError>;
}

pub struct HistoricalBasemaps {
    pub config: IngestConfig,
    pub epochs: Vec<EpochSource>,
}

impl TimelineSource for HistoricalBasemaps {
    fn source(&self) -> SourceId {
        self.config.source.clone()
    }
    fn ingest(&self) -> Result<Ingest, IngestError> {
        ingest(&self.config, &self.epochs)
    }
}

fn tp(year: i32, label: &str) -> Result<TimePoint, IngestError> {
    Year::new(year)
        .map(TimePoint::year_only)
        .map_err(|_| IngestError::BadYear(label.to_string(), year))
}

fn arc_boundary_id(pts: &[QPoint]) -> BoundaryId {
    let mut h = DefaultHasher::new();
    "historical-basemaps/arc".hash(&mut h);
    for p in pts {
        p.lon.hash(&mut h);
        p.lat.hash(&mut h);
    }
    BoundaryId(ContentHash(h.finish()))
}

fn region_id(name: &str) -> RegionId {
    let mut h = DefaultHasher::new();
    "historical-basemaps/region".hash(&mut h);
    name.hash(&mut h);
    RegionId(ContentHash(h.finish()))
}

fn orient(dir: ArcDir) -> Orientation {
    match dir {
        ArcDir::Forward => Orientation::Forward,
        ArcDir::Reverse => Orientation::Reverse,
    }
}

/// One epoch, resolved: regions as arc-referencing geometry, plus the
/// arc table itself.
struct EpochWorld {
    year: i32,
    label: String,
    regions: BTreeMap<String, RegionGeom>,
    arcs: BTreeMap<BoundaryId, Vec<QPoint>>,
}

fn build_epoch(
    epoch: &EpochSource,
    features: &[SourceFeature],
    exemptions: &mut Vec<Exemption>,
) -> EpochWorld {
    // Collect every ring (outer and hole) of every named feature into
    // one global list, remembering who owns what.
    struct Owner {
        name: String,
        part: usize,
        hole: Option<usize>,
    }
    let mut rings: Vec<Vec<QPoint>> = Vec::new();
    let mut owners: Vec<Owner> = Vec::new();
    let mut parts_per_name: BTreeMap<String, usize> = BTreeMap::new();
    let (mut unnamed, mut degenerate) = (0usize, 0usize);

    for f in features {
        let Some(name) = &f.name else {
            unnamed += 1;
            continue;
        };
        for poly in &f.polygons {
            let Some(outer) = clean_ring(&poly.outer) else {
                degenerate += 1;
                continue;
            };
            let part = *parts_per_name
                .entry(name.clone())
                .and_modify(|p| *p += 1)
                .or_insert(0);
            rings.push(outer);
            owners.push(Owner { name: name.clone(), part, hole: None });
            let mut holes_kept = 0usize;
            for hole in &poly.holes {
                let Some(cleaned) = clean_ring(hole) else {
                    degenerate += 1;
                    continue;
                };
                rings.push(cleaned);
                owners.push(Owner { name: name.clone(), part, hole: Some(holes_kept) });
                holes_kept += 1;
            }
        }
    }
    if unnamed > 0 {
        exemptions.push(Exemption::UnnamedFeatures { year: epoch.year, count: unnamed });
    }
    if degenerate > 0 {
        exemptions.push(Exemption::DegenerateRings { year: epoch.year, count: degenerate });
    }

    let ext: Extraction = extract(&rings);
    let mut arcs = BTreeMap::new();
    let arc_ids: Vec<BoundaryId> = ext
        .arcs
        .iter()
        .map(|pts| {
            let id = arc_boundary_id(pts);
            arcs.insert(id, pts.clone());
            id
        })
        .collect();

    // Assemble per-region geometry, parts in first-seen order.
    let mut regions: BTreeMap<String, RegionGeom> = BTreeMap::new();
    for (ring_idx, owner) in owners.iter().enumerate() {
        let cycle: Vec<(BoundaryId, Orientation)> = ext.cycles[ring_idx]
            .iter()
            .map(|&(arc, dir)| (arc_ids[arc], orient(dir)))
            .collect();
        let geom = regions.entry(owner.name.clone()).or_insert(RegionGeom { parts: Vec::new() });
        while geom.parts.len() <= owner.part {
            geom.parts.push(RegionPart { cycle: Vec::new(), holes: Vec::new() });
        }
        match owner.hole {
            None => geom.parts[owner.part].cycle = cycle,
            Some(_) => geom.parts[owner.part].holes.push(cycle),
        }
    }

    EpochWorld { year: epoch.year, label: epoch.label.clone(), regions, arcs }
}

fn source_justification(source: &SourceId) -> Justification {
    Justification {
        text: None,
        grounds: BTreeSet::from([Ground::Source(source.clone())]),
    }
}

pub fn ingest(config: &IngestConfig, epochs: &[EpochSource]) -> Result<Ingest, IngestError> {
    let mut exemptions = Vec::new();

    // Order epochs; exclude pre-anchor ones per the configured frame.
    let mut kept: Vec<&EpochSource> = Vec::new();
    let mut sorted: Vec<&EpochSource> = epochs.iter().collect();
    sorted.sort_by_key(|e| e.year);
    for e in &sorted {
        let at = tp(e.year, &e.label)?;
        if let Some(anchor) = &config.anchor {
            if at < anchor.at {
                exemptions
                    .push(Exemption::PreAnchorEpoch { label: e.label.clone(), year: e.year });
                continue;
            }
        }
        kept.push(e);
    }

    // Resolve each kept epoch.
    let mut worlds: Vec<EpochWorld> = Vec::new();
    for e in &kept {
        let features = parse_features(&e.text)
            .map_err(|err| IngestError::Parse(e.label.clone(), err))?;
        worlds.push(build_epoch(e, &features, &mut exemptions));
    }

    // Interval end for epoch k: the next epoch's start; the last stays
    // open — the current edge of THIS source's knowledge.
    let starts: Vec<TimePoint> =
        worlds.iter().map(|w| tp(w.year, &w.label)).collect::<Result<_, _>>()?;
    // interval(from, last): starts at epoch `from`, ends where the
    // epoch after `last` begins — or stays open past the final epoch.
    let interval = |from: usize, last: usize| -> Interval {
        Interval { from: starts[from], to: starts.get(last + 1).copied() }
    };

    let justification = source_justification(&config.source);
    let provenance = config.source.0.clone();

    // ---- boundaries: same arc in consecutive epochs = one version ----
    let mut arc_epochs: BTreeMap<BoundaryId, Vec<usize>> = BTreeMap::new();
    for (k, w) in worlds.iter().enumerate() {
        for id in w.arcs.keys() {
            arc_epochs.entry(*id).or_default().push(k);
        }
    }
    let mut boundaries: BTreeMap<BoundaryId, BoundaryHistory> = BTreeMap::new();
    for (id, ks) in &arc_epochs {
        let pts = &worlds[ks[0]].arcs[id];
        let boundary = Boundary {
            pts: pts.iter().map(|q| q.to_unit_vec()).collect::<Vec<UnitVec>>(),
            character: EdgeCharacter::Unknown,
            source: BoundarySource::Imported { source: config.source.clone() },
            justification: justification.clone(),
            provenance: provenance.clone(),
        };
        let mut versions = Vec::new();
        let mut run_start = ks[0];
        let mut prev = ks[0];
        for &k in &ks[1..] {
            if k != prev + 1 {
                versions.push((interval(run_start, prev), boundary.clone()));
                run_start = k;
            }
            prev = k;
        }
        versions.push((interval(run_start, prev), boundary.clone()));
        boundaries.insert(*id, BoundaryHistory { versions });
    }

    // ---- regions: presence runs label; equal-geometry runs version ----
    let mut region_epochs: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (k, w) in worlds.iter().enumerate() {
        for name in w.regions.keys() {
            region_epochs.entry(name.clone()).or_default().push(k);
        }
    }
    // The arc multiset of a geometry — the adapter's change detector.
    // Two epochs listing the same arcs in a different part order are
    // the same geometry, not a border change.
    fn arc_set(geom: &RegionGeom) -> Vec<(BoundaryId, Orientation)> {
        let mut v: Vec<(BoundaryId, Orientation)> = geom
            .parts
            .iter()
            .flat_map(|p| p.cycle.iter().chain(p.holes.iter().flatten()).copied())
            .collect();
        v.sort();
        v
    }
    let same_geometry = |a: &RegionGeom, b: &RegionGeom| a == b || arc_set(a) == arc_set(b);

    let mut regions: BTreeMap<RegionId, RegionHistory> = BTreeMap::new();
    for (name, ks) in &region_epochs {
        let mut label_history = Vec::new();
        let mut geom_history: Vec<(Interval, RegionGeom)> = Vec::new();
        let mut run_start = ks[0];
        let mut geom_start = ks[0];
        let mut prev = ks[0];
        let mut prev_geom = &worlds[ks[0]].regions[name];
        for &k in &ks[1..] {
            let geom = &worlds[k].regions[name];
            if k != prev + 1 {
                // Presence run ended: close label and geometry.
                label_history.push((interval(run_start, prev), name.clone()));
                geom_history.push((interval(geom_start, prev), prev_geom.clone()));
                run_start = k;
                geom_start = k;
            } else if !same_geometry(geom, prev_geom) {
                geom_history.push((interval(geom_start, prev), prev_geom.clone()));
                geom_start = k;
            }
            prev = k;
            prev_geom = geom;
        }
        label_history.push((interval(run_start, prev), name.clone()));
        geom_history.push((interval(geom_start, prev), prev_geom.clone()));
        regions.insert(region_id(name), RegionHistory { label_history, geom_history });
    }

    // ---- events: the narrative of epoch-to-epoch difference ----
    let mut events: Vec<ChangeEvent> = Vec::new();
    for k in 1..worlds.len() {
        let (before, after) = (&worlds[k - 1], &worlds[k]);
        let at = starts[k];
        let file_provenance = format!("{}@{}", config.source.0, after.label);
        let mut push = |kind: ChangeKind| {
            events.push(ChangeEvent {
                at,
                kind,
                driver: None,
                justification: justification.clone(),
                provenance: file_provenance.clone(),
            });
        };
        let mut shifted: BTreeSet<BoundaryId> = BTreeSet::new();
        for (name, geom) in &after.regions {
            match before.regions.get(name) {
                None => push(ChangeKind::Rise { region: region_id(name) }),
                Some(old) if !same_geometry(old, geom) => {
                    // Narrate the symmetric difference: arcs gained AND
                    // arcs lost are both border changes at this epoch.
                    let old_arcs: BTreeSet<BoundaryId> =
                        arc_set(old).into_iter().map(|(b, _)| b).collect();
                    let new_arcs: BTreeSet<BoundaryId> =
                        arc_set(geom).into_iter().map(|(b, _)| b).collect();
                    for b in new_arcs.symmetric_difference(&old_arcs) {
                        if shifted.insert(*b) {
                            push(ChangeKind::Shift { boundary: *b });
                        }
                    }
                }
                Some(_) => {}
            }
        }
        for name in before.regions.keys() {
            if !after.regions.contains_key(name) {
                push(ChangeKind::Fall { region: region_id(name) });
            }
        }
    }

    Ok(Ingest {
        timeline: WorldTimeline {
            anchor: config.anchor.clone(),
            boundaries,
            regions,
            events,
            atlas_pin: None,
        },
        exemptions,
    })
}

/// THE FIDELITY LAW (phase 2): what came out is what went in. For the
/// given epoch, every named source feature's rings — quantized by the
/// disclosed method — are exactly recoverable from the stored timeline,
/// ring for ring, point for point (compared as canonical rotations).
pub fn fidelity_violations(
    ingest: &Ingest,
    epoch: &EpochSource,
) -> Result<Vec<String>, IngestError> {
    let features = parse_features(&epoch.text)
        .map_err(|err| IngestError::Parse(epoch.label.clone(), err))?;
    let at = tp(epoch.year, &epoch.label)?;
    let tl = &ingest.timeline;
    let mut violations = Vec::new();

    // Both sides canonicalize the SAME way: convert to bit-canonical
    // unit-vector triples (the deterministic pipeline the adapter
    // stores through), then rotate to the least phase in bit order.
    fn rotate_min(ring: Vec<(u64, u64, u64)>) -> Vec<(u64, u64, u64)> {
        let start = (0..ring.len())
            .min_by(|&a, &b| {
                let ra = ring[a..].iter().chain(&ring[..a]);
                let rb = ring[b..].iter().chain(&ring[..b]);
                ra.cmp(rb)
            })
            .unwrap_or(0);
        let mut rotated = Vec::with_capacity(ring.len());
        rotated.extend_from_slice(&ring[start..]);
        rotated.extend_from_slice(&ring[..start]);
        rotated
    }
    let canon = |pts: &[QPoint]| -> Vec<(u64, u64, u64)> {
        rotate_min(
            pts.iter()
                .map(|q| {
                    let v = q.to_unit_vec();
                    (v.x().to_bits(), v.y().to_bits(), v.z().to_bits())
                })
                .collect(),
        )
    };
    let mut expected: BTreeMap<String, BTreeSet<Vec<(u64, u64, u64)>>> = BTreeMap::new();
    for f in &features {
        let Some(name) = &f.name else { continue };
        let entry = expected.entry(name.clone()).or_default();
        for poly in &f.polygons {
            for ring in std::iter::once(&poly.outer).chain(&poly.holes) {
                if let Some(cleaned) = clean_ring(ring) {
                    entry.insert(canon(&cleaned));
                }
            }
        }
    }

    // Actual: reconstruct every ring of every region from the stored
    // arc geometry at this epoch.
    for (name, want) in &expected {
        let rid = region_id(name);
        let Some(hist) = tl.regions.get(&rid) else {
            violations.push(format!("{name}: region missing from timeline"));
            continue;
        };
        let Some(geom) = hist.geom_at(&at) else {
            violations.push(format!("{name}: no geometry at {}", epoch.year));
            continue;
        };
        let mut got: BTreeSet<Vec<(u64, u64, u64)>> = BTreeSet::new();
        for part in &geom.parts {
            for cycle in std::iter::once(&part.cycle).chain(&part.holes) {
                let mut ring: Vec<(u64, u64, u64)> = Vec::new();
                for (bid, orientation) in cycle {
                    let Some(b) = tl.boundaries.get(bid).and_then(|h| h.at(&at)) else {
                        violations.push(format!("{name}: arc {bid:?} unresolved at {}", epoch.year));
                        continue;
                    };
                    let pts: Vec<(u64, u64, u64)> = b
                        .pts
                        .iter()
                        .map(|v| (v.x().to_bits(), v.y().to_bits(), v.z().to_bits()))
                        .collect();
                    match orientation {
                        Orientation::Forward => ring.extend_from_slice(&pts[..pts.len() - 1]),
                        Orientation::Reverse => ring.extend(pts[1..].iter().rev()),
                    }
                }
                got.insert(rotate_min(ring));
            }
        }
        if &got != want {
            violations.push(format!(
                "{name}: rings differ at {} ({} stored vs {} in source)",
                epoch.year,
                got.len(),
                want.len()
            ));
        }
    }
    Ok(violations)
}
