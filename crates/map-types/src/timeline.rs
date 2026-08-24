//! The temporal model: DELTAS, NOT STATES. Borders change at events;
//! a snapshot at time t is a derived, deterministic query result —
//! never a hand-maintained artifact (covenant rule 6).
//!
//! Time vocabulary is the atlas's (contract C1): Year (non-zero i32,
//! negative = BC) and TimePoint — the two systems never argue about time.

use std::collections::BTreeMap;

use atlas_graph_types::chrono::TimePoint;
use atlas_graph_types::edge::{Ground, Justification};
use atlas_graph_types::id::{ContentHash, EventId};
use atlas_graph_types::ingest::ProvenanceId;

use crate::boundary::{Boundary, RegionGeom};
use crate::geom::UnitVec;
use crate::ident::{BoundaryId, Canon, ChangeEventId, MapAddressed, MapKind, RegionId};

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

/// What kind of area a region is. The WHOLE world is the map: seas,
/// lakes, and deserts are as explorable as polities — class picks the
/// dress (water paint vs region paint), nothing else differs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum RegionClass {
    #[default]
    Land,
    Water,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RegionHistory {
    pub class: RegionClass,
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

pub fn canon_time_point(c: &mut Canon, t: &TimePoint) {
    c.i32_(t.year.get());
    c.opt(&t.month, |c, m| {
        c.u8_(*m);
    });
    c.opt(&t.day, |c, d| {
        c.u8_(*d);
    });
}

fn canon_justification(c: &mut Canon, j: &Justification) {
    c.opt(&j.text, |c, t| {
        c.str_(t);
    });
    let grounds: Vec<_> = j.grounds.iter().collect();
    c.seq(&grounds, |c, g| match g {
        Ground::Scripture(range) => {
            c.u8_(0);
            for locus in [&range.from, &range.to] {
                c.u8_(locus.unit.book)
                    .u64_(u64::from(locus.unit.chapter))
                    .u64_(u64::from(locus.unit.verse));
                c.opt(&locus.span, |c, s| {
                    c.str_(&s.layer.0).u64_(u64::from(s.start)).u64_(u64::from(s.end));
                });
            }
        }
        Ground::Anchor(a) => {
            c.u8_(1).str_(&a.0);
        }
        Ground::Source(s) => {
            c.u8_(2).str_(&s.0);
        }
    });
}

/// Change events are content-addressed like everything else, so a
/// DELTA is a render subject with a stable id (RenderSubject::Change).
impl MapAddressed for ChangeEvent {
    fn canonical_bytes(&self) -> Vec<u8> {
        let mut c = Canon::new();
        c.tag("change-event");
        canon_time_point(&mut c, &self.at);
        match &self.kind {
            ChangeKind::Rise { region } => {
                c.u8_(0).u64_(region.0 .0);
            }
            ChangeKind::Fall { region } => {
                c.u8_(1).u64_(region.0 .0);
            }
            ChangeKind::Shift { boundary } => {
                c.u8_(2).u64_(boundary.0 .0);
            }
            ChangeKind::Split { parent, children, seam } => {
                c.u8_(3).u64_(parent.0 .0);
                c.seq(children, |c, r| {
                    c.u64_(r.0 .0);
                });
                c.seq(seam, |c, p| p.canon(c));
            }
            ChangeKind::Merge { parents, child } => {
                c.u8_(4);
                c.seq(parents, |c, r| {
                    c.u64_(r.0 .0);
                });
                c.u64_(child.0 .0);
            }
            ChangeKind::Rename { region } => {
                c.u8_(5).u64_(region.0 .0);
            }
        }
        c.opt(&self.driver, |c, d| {
            c.str_(&d.event.0).u64_(d.atlas_root.0);
        });
        canon_justification(&mut c, &self.justification);
        c.str_(&self.provenance);
        c.done()
    }
    fn map_kind(&self) -> MapKind {
        MapKind::ChangeEvent
    }
}

impl ChangeEvent {
    pub fn id(&self) -> ChangeEventId {
        ChangeEventId(self.map_pid().hash)
    }
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
