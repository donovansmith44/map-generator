//! map-generator — the domain types (phase 1 of docs/map-system-handoff.md).
//!
//! The compiled crate is the authority; where prose and crate disagree,
//! the crate wins. Every type either makes an illegal state
//! unrepresentable or names the law a fail-loud validator enforces; the
//! §D design laws run as the tests at the bottom — if this crate
//! compiles and its tests pass, the design composes.
//!
//! Vocabulary (covenant rule 10): sum and product types for data;
//! traits with laws for behavior; pure functions for derivation;
//! algebras for the operations users feel (scenes are a monoid under
//! overlay; scripts compose; accumulation is a fold).
//!
//! Deliberate deviations from the §B sketch, recorded for owner review:
//! - `MapAddressed`/`MapKind` mirror (not reuse) the atlas identity
//!   trait — the atlas kind enums are closed and this repo does not
//!   edit the atlas (see ident.rs).
//! - `Snapshot` carries styled boundaries and markers alongside regions
//!   and labels, because EVERY RenderSubject renders to a scene (see
//!   scene.rs).
//! - SnapshotQuery/AccumulationQuery are collapsed into `RenderQuery`,
//!   exactly as §B instructs.
//! - Accumulation dedups samples by content address before folding —
//!   that is what makes redundant samples inert (law 9).

#![allow(dead_code)]

pub mod algebra;
pub mod boundary;
pub mod contracts;
pub mod encode;
pub mod geom;
pub mod ident;
pub mod laws;
pub mod provider;
pub mod query;
pub mod scene;
pub mod style;
pub mod timeline;
pub mod transition;

/// The atlas covenant surface (contract C1), re-exported so consumers
/// of map-types never import the atlas crate directly.
pub mod atlas {
    pub use atlas_graph_types::covenant::{ResolvedPlacement, TimePoint, Year};
    pub use atlas_graph_types::covenant::{Ground, Justification};
    pub use atlas_graph_types::covenant::{ContentAddressed, ContentHash, EventId, Pid, PlaceId, SourceId};
    pub use atlas_graph_types::covenant::{Confidence, Provenance, ProvenanceId};
    pub use atlas_graph_types::covenant::{BibleLocus, BibleLocusRange, VerseRef};
}

pub use algebra::{mconcat, Monoid};
pub use boundary::{
    AtlasPlaceRef, BorderSurvey, Boundary, BoundarySource, EdgeCharacter, InterpolationMethod,
    Orientation, RegionGeom, RegionPart,
};
pub use contracts::{ChronoSpan, ChronologyExport, GazetteerEntry, GazetteerExport};
pub use encode::{EncodeError, SceneEncoder, TransitionEncoder};
pub use geom::{covers_sphere, inside_ring, morph_rings, simplify_polyline, slerp, Bbox, GeomError, Lod, Ring, UnitVec, Winding};
pub use ident::{BoundaryId, ChangeEventId, MapAddressed, MapKind, MapPid, RegionId, StyleId};
pub use laws::{validate_all, Violation};
pub use provider::{MapError, MapProvider, SubjectListing};
pub use query::{RenderQuery, RenderSubject, TimeSelector};
pub use scene::{accumulate, sample_times, PlacedLabel, Snapshot, StyledBoundary, StyledRegion};
pub use style::{LayerSet, Style, StyleError};
pub use timeline::{
    Anchor, AtlasEventRef, AtlasPin, BoundaryHistory, ChangeEvent, ChangeKind, Interval,
    RegionClass, RegionHistory, WorldTimeline,
};
pub use transition::{TransitionScript, TransitionStep};

pub mod chart;
#[cfg(test)]
mod tests;
