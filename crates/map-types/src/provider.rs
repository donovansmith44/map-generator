//! THE SEAM (contract C5, co-owned with the atlas): the query surface a
//! consumer lives on. Semantic scenes, attribution riding every
//! response, content-addressed caching. Frozen with the owner before
//! serving code exists on either side (spec §E).

use atlas_graph_types::chrono::TimePoint;
use atlas_graph_types::id::ContentHash;

use crate::geom::{Bbox, Lod};
use crate::ident::{BoundaryId, ChangeEventId, RegionId, StyleId};
use crate::query::RenderQuery;
use crate::scene::Snapshot;
use crate::timeline::ChangeEvent;
use crate::transition::TransitionScript;

#[derive(Clone, Debug, PartialEq)]
pub enum MapError {
    UnknownRegion(RegionId),
    UnknownBoundary(BoundaryId),
    UnknownChange(ChangeEventId),
    UnknownStyle(StyleId),
    UnknownPlace(String),
    /// The subject exists but has no fact at the queried time.
    NothingAtTime(TimePoint),
    /// Contract C6: the timeline was compiled against a different atlas
    /// version root — fail loud, never silently serve stale borders.
    StaleAgainstAtlas { pinned: ContentHash, current: ContentHash },
}

pub trait MapProvider {
    /// One front door for every granularity: snapshot, accumulation,
    /// lone boundary, delta, world — all RenderQuery, all Scene out.
    fn render(&self, q: &RenderQuery) -> Result<Snapshot, MapError>;

    /// A semantic animation between two instants. Laws: transition(t,t)
    /// is empty; composed transitions agree with the direct one (law 3).
    fn transition(
        &self,
        from: TimePoint,
        to: TimePoint,
        viewport: Bbox,
        lod: Lod,
    ) -> Result<TransitionScript, MapError>;

    /// The narrative between two instants — what scrubber UIs stop at:
    /// the piecewise-constant timeline made visible.
    fn changes_between(&self, from: TimePoint, to: TimePoint) -> Vec<&ChangeEvent>;
}
