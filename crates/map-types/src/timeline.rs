//! The temporal model: DELTAS, NOT STATES. Borders change at events;
//! a snapshot at time t is a derived, deterministic query result —
//! never a hand-maintained artifact (covenant rule 6).
//!
//! Time vocabulary is the atlas's (contract C1): Year (non-zero i32,
//! negative = BC) and TimePoint — the two systems never argue about time.

use std::collections::BTreeMap;

use atlas_graph_types::chrono::TimePoint;
use atlas_graph_types::edge::Justification;
use atlas_graph_types::id::{ContentHash, EventId};
use atlas_graph_types::ingest::ProvenanceId;

use crate::boundary::{Boundary, RegionGeom};
use crate::geom::UnitVec;
use crate::ident::{BoundaryId, RegionId};

/// Validity of a fact. `to: None` = open — the current edge of
/// knowledge. Containment is half-open ([from, to)): a version ends the
/// instant its successor begins, so piecewise histories tile time with
/// no double-cover.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Interval {
    pub from: TimePoint,
    pub to: Option<TimePoint>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntervalError {
    Inverted,
}

impl Interval {
    /// `from == to` is lawful for QUERY intervals (a single-instant
    /// accumulation, law 9); as a history interval it is empty and the
    /// coherence validator rejects it.
    pub fn new(from: TimePoint, to: Option<TimePoint>) -> Result<Self, IntervalError> {
        match to {
            Some(t) if t < from => Err(IntervalError::Inverted),
            _ => Ok(Interval { from, to }),
        }
    }
    pub fn open_from(from: TimePoint) -> Self {
        Interval { from, to: None }
    }
    pub fn contains(&self, t: &TimePoint) -> bool {
        *t >= self.from && self.to.map_or(true, |end| *t < end)
    }
    pub fn intersects(&self, o: &Interval) -> bool {
        let self_ends_before = self.to.map_or(false, |e| e <= o.from);
        let other_ends_before = o.to.map_or(false, |e| e <= self.from);
        !(self_ends_before || other_ends_before)
    }
}

/// Piecewise boundary geometry; intervals disjoint and ordered (law 5).
#[derive(Clone, Debug, PartialEq)]
pub struct BoundaryHistory {
    pub versions: Vec<(Interval, Boundary)>,
}

impl BoundaryHistory {
    pub fn at(&self, t: &TimePoint) -> Option<&Boundary> {
        self.versions.iter().find(|(i, _)| i.contains(t)).map(|(_, b)| b)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RegionHistory {
    pub label_history: Vec<(Interval, String)>,
    pub geom_history: Vec<(Interval, RegionGeom)>,
}

impl RegionHistory {
    pub fn geom_at(&self, t: &TimePoint) -> Option<&RegionGeom> {
        self.geom_history.iter().find(|(i, _)| i.contains(t)).map(|(_, g)| g)
    }
    pub fn label_at(&self, t: &TimePoint) -> Option<&str> {
        self.label_history
            .iter()
            .find(|(i, _)| i.contains(t))
            .map(|(_, l)| l.as_str())
    }
}

/// A reference to the atlas Event that DROVE a border change (contract
/// C4), pinned to the atlas version root it was read from (contract C6).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AtlasEventRef {
    pub event: EventId,
    pub atlas_root: ContentHash,
}

/// Border change IS an event — conquest, treaty, exile. Continuous
/// drift is a Shift; topology change gets its own honest verb:
/// Alexander's empire does not drift into the Diadochi, it SPLITS.
#[derive(Clone, Debug, PartialEq)]
pub enum ChangeKind {
    Rise { region: RegionId },
    Fall { region: RegionId },
    Shift { boundary: BoundaryId },
    Split { parent: RegionId, children: Vec<RegionId>, seam: Vec<UnitVec> },
    Merge { parents: Vec<RegionId>, child: RegionId },
    Rename { region: RegionId },
}

/// BIBLE-DRIVEN (C4): when the change corresponds to a Scripture-
/// attested event, `driver` carries the atlas's EventId and `at` MUST
/// equal the atlas's resolved placement (law 12a) — the map never
/// re-dates what the Word, via the atlas's traditional chronology,
/// already dates. Extra-biblical changes leave driver = None and carry
/// Source grounds, disclosed.
#[derive(Clone, Debug, PartialEq)]
pub struct ChangeEvent {
    pub at: TimePoint,
    pub kind: ChangeKind,
    pub driver: Option<AtlasEventRef>,
    pub justification: Justification,
    pub provenance: ProvenanceId,
}

/// THE ANCHOR (owner rulings, 2026-08-23): "the world starts with God's
/// creation - that is the first event" — and, generalized: "biblical is
/// a parameter... the point is that we can define an anchor."
///
/// A timeline declares the anchor it is built under: the frame's first
/// event, before which it admits nothing (law 0). The type is
/// frame-GENERIC so rival chronologies can each carry their own anchor
/// and be compared — the biblical timeline (anchored at creation, its
/// date from the atlas's traditional chronology via C2, never a
/// map-side literal) against false ones. Which frame is TRUE is
/// doctrine, not type structure; it shows in the justification and in
/// what the owner's tools choose to trust. The seven days and the
/// unknown span between creation and the fall are atlas chronology
/// facts (its placement vocabulary carries unknown gaps); the map
/// never invents durations.
#[derive(Clone, Debug, PartialEq)]
pub struct Anchor {
    /// Human-readable frame name, e.g. "biblical (Ussher tradition)".
    pub frame: String,
    pub at: TimePoint,
    pub justification: Justification,
    pub provenance: ProvenanceId,
}

/// The atlas version root a timeline compiled against (contract C6):
/// an atlas chronology/gazetteer change flips a fail-loud stale flag
/// instead of silently serving outdated borders.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AtlasPin {
    pub version_root: ContentHash,
}

impl AtlasPin {
    pub fn is_stale(&self, current: ContentHash) -> bool {
        self.version_root != current
    }
}

/// The whole model: boundary histories, region histories, and the
/// narrative of the map — its change events.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct WorldTimeline {
    pub anchor: Option<Anchor>,
    pub boundaries: BTreeMap<BoundaryId, BoundaryHistory>,
    pub regions: BTreeMap<RegionId, RegionHistory>,
    pub events: Vec<ChangeEvent>,
    pub atlas_pin: Option<AtlasPin>,
}
