//! Fail-loud validators for the design laws that judge DATA (§D).
//! Laws that judge FUNCTIONS (determinism, monoid algebra, morph
//! safety, lod monotonicity, encoder terminality) live as tests in
//! tests.rs; the validators here run over any WorldTimeline — at
//! ingestion, at load, in tests.
//!
//! Every violation is typed, reasoned, enumerable — never a silent skip.

use crate::boundary::{BoundarySource, Orientation};
use crate::contracts::{ChronologyExport, GazetteerExport};
use crate::geom::UnitVec;
use crate::ident::{BoundaryId, RegionId};
use crate::timeline::{ChangeKind, Interval, WorldTimeline};
use atlas_graph_types::covenant::TimePoint;
use atlas_graph_types::covenant::{EventId, PlaceId};

#[derive(Clone, Debug, PartialEq)]
pub enum Violation {
    // ---- law 0: the anchor ----
    /// A fact precedes the timeline's declared anchor — under a frame
    /// whose history starts at its first event, there is no "before".
    BeforeAnchor { what: String, at: TimePoint },

    // ---- law 5: history coherence ----
    /// A history interval is empty or overlaps its neighbor.
    IncoherentIntervals { what: String },
    /// Geometry changed at t with no ChangeEvent at t — a silent border
    /// move. Changes are NARRATED.
    UnnarratedChange { what: String, at: TimePoint },

    // ---- law 2 (structural half): partition sanity ----
    /// A region cycle references a boundary the timeline doesn't hold,
    /// or holds no version of during the region's interval.
    DanglingBoundary { region: RegionId, boundary: BoundaryId },
    /// Consecutive oriented arcs in a cycle fail to connect end-to-start.
    BrokenCycle { region: RegionId, position: usize },

    // ---- law 6: provenance totality ----
    EmptyProvenance { what: String },

    // ---- law 12: Bible preference ----
    /// (a) A driven event's date differs from the atlas placement.
    DriverDateMismatch { event: EventId, map_at: TimePoint, atlas_from: TimePoint },
    /// (a) A driven event names an atlas event the chronology export
    /// doesn't contain.
    DriverUnknownToAtlas { event: EventId },
    /// (b) An Imported boundary version overlaps a Scripture survey's
    /// validity on the same arc — scholarship silently overriding the
    /// text. Ending a survey's validity is a narrated ChangeEvent, not
    /// an import.
    ImportedOverridesSurvey { boundary: BoundaryId },
    /// (c) A survey waypoint doesn't resolve to a live atlas place.
    UnresolvedWaypoint { boundary: BoundaryId, place: PlaceId },

    // ---- referential integrity of the narrative ----
    UnknownRegionInEvent { region: RegionId },
    UnknownBoundaryInEvent { boundary: BoundaryId },
}

fn intervals_coherent(intervals: &[&Interval]) -> bool {
    for w in intervals.windows(2) {
        let (a, b) = (w[0], w[1]);
        if b.from < a.from {
            return false; // unordered
        }
        if a.intersects(b) {
            return false; // overlapping
        }
    }
    intervals.iter().all(|i| i.to.map_or(true, |end| end > i.from))
}

/// Law 0 (owner ruling, generalized): a timeline that declares its
/// anchor admits nothing before it — no event, no history interval.
/// An anchorless timeline passes vacuously; declaring the anchor is
/// what buys the guarantee.
pub fn validate_anchor(tl: &WorldTimeline) -> Vec<Violation> {
    let mut v = Vec::new();
    let Some(anchor) = &tl.anchor else { return v };
    if anchor.provenance.is_empty() {
        v.push(Violation::EmptyProvenance { what: "anchor".to_string() });
    }
    for (i, e) in tl.events.iter().enumerate() {
        if e.at < anchor.at {
            v.push(Violation::BeforeAnchor { what: format!("event #{}", i), at: e.at });
        }
    }
    for (id, hist) in &tl.boundaries {
        for (iv, _) in &hist.versions {
            if iv.from < anchor.at {
                v.push(Violation::BeforeAnchor {
                    what: format!("boundary {:?}", id),
                    at: iv.from,
                });
            }
        }
    }
    for (id, hist) in &tl.regions {
        for iv in hist
            .label_history
            .iter()
            .map(|(i, _)| i)
            .chain(hist.geom_history.iter().map(|(i, _)| i))
        {
            if iv.from < anchor.at {
                v.push(Violation::BeforeAnchor {
                    what: format!("region {:?}", id),
                    at: iv.from,
                });
            }
        }
    }
    v
}

/// Law 5: each history's intervals are disjoint, ordered, non-empty;
/// every geometry change at t has a ChangeEvent at t.
pub fn validate_history_coherence(tl: &WorldTimeline) -> Vec<Violation> {
    let mut v = Vec::new();
    let event_times: Vec<TimePoint> = tl.events.iter().map(|e| e.at).collect();
    let narrated = |t: &TimePoint| event_times.contains(t);

    for (id, hist) in &tl.boundaries {
        let ivs: Vec<&Interval> = hist.versions.iter().map(|(i, _)| i).collect();
        if !intervals_coherent(&ivs) {
            v.push(Violation::IncoherentIntervals { what: format!("boundary {:?}", id) });
        }
        // Every version start after the first is a geometry change.
        for (iv, _) in hist.versions.iter().skip(1) {
            if !narrated(&iv.from) {
                v.push(Violation::UnnarratedChange {
                    what: format!("boundary {:?}", id),
                    at: iv.from,
                });
            }
        }
    }
    for (id, hist) in &tl.regions {
        for (name, ivs) in [
            ("labels", hist.label_history.iter().map(|(i, _)| i).collect::<Vec<_>>()),
            ("geometry", hist.geom_history.iter().map(|(i, _)| i).collect::<Vec<_>>()),
        ] {
            if !intervals_coherent(&ivs) {
                v.push(Violation::IncoherentIntervals {
                    what: format!("region {:?} {}", id, name),
                });
            }
        }
        for (iv, _) in hist.geom_history.iter().skip(1) {
            if !narrated(&iv.from) {
                v.push(Violation::UnnarratedChange {
                    what: format!("region {:?} geometry", id),
                    at: iv.from,
                });
            }
        }
    }
    v
}

/// Law 6 (totality half): no boundary or event with empty provenance.
/// (The rendering half — Unknown renders distinctly — is enforced at
/// Style construction and proven in tests.)
pub fn validate_provenance_totality(tl: &WorldTimeline) -> Vec<Violation> {
    let mut v = Vec::new();
    for (id, hist) in &tl.boundaries {
        for (_, b) in &hist.versions {
            if b.provenance.is_empty() {
                v.push(Violation::EmptyProvenance { what: format!("boundary {:?}", id) });
            }
        }
    }
    for (i, e) in tl.events.iter().enumerate() {
        if e.provenance.is_empty() {
            v.push(Violation::EmptyProvenance { what: format!("event #{}", i) });
        }
    }
    v
}

fn oriented_endpoints<'a>(
    pts: &'a [UnitVec],
    o: Orientation,
) -> Option<(&'a UnitVec, &'a UnitVec)> {
    let (first, last) = (pts.first()?, pts.last()?);
    Some(match o {
        Orientation::Forward => (first, last),
        Orientation::Reverse => (last, first),
    })
}

fn validate_cycle(
    tl: &WorldTimeline,
    region: RegionId,
    interval: &Interval,
    cycle: &[(BoundaryId, Orientation)],
    v: &mut Vec<Violation>,
) {
    // Resolve each arc's geometry as valid at the interval's start —
    // the version any instant in the region interval sees first.
    let mut resolved: Vec<Option<&crate::boundary::Boundary>> = Vec::new();
    for (bid, _) in cycle {
        let b = tl.boundaries.get(bid).and_then(|h| h.at(&interval.from));
        if b.is_none() {
            v.push(Violation::DanglingBoundary { region, boundary: *bid });
        }
        resolved.push(b);
    }
    // Continuity: each arc's oriented end meets the next arc's start.
    // Shared arcs carry identical junction coordinates, so equality is
    // exact — no epsilon.
    for i in 0..cycle.len() {
        let j = (i + 1) % cycle.len();
        let (Some(a), Some(b)) = (resolved[i], resolved[j]) else { continue };
        let (Some((_, a_end)), Some((b_start, _))) = (
            oriented_endpoints(&a.pts, cycle[i].1),
            oriented_endpoints(&b.pts, cycle[j].1),
        ) else {
            continue;
        };
        if a_end != b_start {
            v.push(Violation::BrokenCycle { region, position: i });
        }
    }
}

/// Law 2, structural half: every cycle reference resolves, and shared
/// arcs chain end-to-start with exact junctions — the no-sliver payoff,
/// proven not assumed. (The geometric half — no undeclared overlap —
/// arrives with the materializer, phase 3.)
pub fn validate_partition_structure(tl: &WorldTimeline) -> Vec<Violation> {
    let mut v = Vec::new();
    for (rid, hist) in &tl.regions {
        for (iv, geom) in &hist.geom_history {
            for part in &geom.parts {
                validate_cycle(tl, *rid, iv, &part.cycle, &mut v);
                for hole in &part.holes {
                    validate_cycle(tl, *rid, iv, hole, &mut v);
                }
            }
        }
    }
    v
}

/// Law 12: the owner's authority order, testable.
pub fn validate_bible_preference(
    tl: &WorldTimeline,
    chronology: &ChronologyExport,
    gazetteer: &GazetteerExport,
) -> Vec<Violation> {
    let mut v = Vec::new();

    // (a) A driven event's `at` equals the atlas's resolved placement —
    // byte equality, no re-derivation.
    for e in &tl.events {
        let Some(driver) = &e.driver else { continue };
        match chronology.placements.get(&driver.event) {
            None => v.push(Violation::DriverUnknownToAtlas { event: driver.event.clone() }),
            Some(p) => {
                // A span-shaped placement (siege..fall) realizes at
                // either ENDPOINT — never an invented interior date.
                if e.at != p.date.from && e.at != p.date.to {
                    v.push(Violation::DriverDateMismatch {
                        event: driver.event.clone(),
                        map_at: e.at,
                        atlas_from: p.date.from,
                    });
                }
            }
        }
    }

    for (bid, hist) in &tl.boundaries {
        // (b) On one arc, an Imported version may not overlap a Survey
        // version's validity: Scripture is not silently overridden.
        let survey_ivs: Vec<&Interval> = hist
            .versions
            .iter()
            .filter(|(_, b)| matches!(b.source, BoundarySource::Survey(_)))
            .map(|(i, _)| i)
            .collect();
        for (iv, b) in &hist.versions {
            if matches!(b.source, BoundarySource::Imported { .. })
                && survey_ivs.iter().any(|s| s.intersects(iv))
            {
                v.push(Violation::ImportedOverridesSurvey { boundary: *bid });
            }
        }
        // (c) Every survey waypoint resolves to a live atlas place.
        for (_, b) in &hist.versions {
            if let BoundarySource::Survey(s) = &b.source {
                for w in &s.waypoints {
                    if !gazetteer.places.contains_key(&w.0) {
                        v.push(Violation::UnresolvedWaypoint {
                            boundary: *bid,
                            place: w.0.clone(),
                        });
                    }
                }
            }
        }
    }
    v
}

/// Every change event's referenced subjects exist in the timeline —
/// used by tests and adapters; not itself a numbered law, but laws 2
/// and 5 lean on it.
pub fn validate_event_refs(tl: &WorldTimeline) -> Vec<Violation> {
    let mut v = Vec::new();
    let check_region = |r: &RegionId, v: &mut Vec<Violation>| {
        if !tl.regions.contains_key(r) {
            v.push(Violation::UnknownRegionInEvent { region: *r });
        }
    };
    for e in &tl.events {
        match &e.kind {
            ChangeKind::Rise { region }
            | ChangeKind::Fall { region }
            | ChangeKind::Rename { region } => check_region(region, &mut v),
            ChangeKind::Shift { boundary } | ChangeKind::Journey { boundary } => {
                if !tl.boundaries.contains_key(boundary) {
                    v.push(Violation::UnknownBoundaryInEvent { boundary: *boundary });
                }
            }
            ChangeKind::Split { parent, children, .. } => {
                check_region(parent, &mut v);
                for c in children {
                    check_region(c, &mut v);
                }
            }
            ChangeKind::Merge { parents, child } => {
                for p in parents {
                    check_region(p, &mut v);
                }
                check_region(child, &mut v);
            }
        }
    }
    v
}

/// Run every data validator; the empty vec is the lawful state.
pub fn validate_all(
    tl: &WorldTimeline,
    chronology: &ChronologyExport,
    gazetteer: &GazetteerExport,
) -> Vec<Violation> {
    let mut v = validate_anchor(tl);
    v.extend(validate_history_coherence(tl));
    v.extend(validate_provenance_totality(tl));
    v.extend(validate_partition_structure(tl));
    v.extend(validate_bible_preference(tl, chronology, gazetteer));
    v.extend(validate_event_refs(tl));
    v
}
