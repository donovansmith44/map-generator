use atlas_graph_types::edge::HoldingState;
use atlas_graph_types::id::{AnyNodeId, ContentHash};
use atlas_graph_types::map_slice::{FeatureCollection, ShapeId, StoredShape};
use atlas_graph_types::map_timing::RecordedMapCause;
use atlas_graph_types::place_versions::Basis;

use crate::boundary::RegionGeom;
use crate::transition::{TransitionScript, TransitionStep};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ArcRef {
    pub store: ContentHash,
    pub key: ContentHash,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "kind", content = "boundary", rename_all = "kebab-case", deny_unknown_fields))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum TransitionBoundary {
    Arc(ArcRef),
    Ring(RegionGeom<ArcRef>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ArcChange {
    pub arc: ArcRef,
    pub before_shape: Option<ShapeId>,
    pub after_shape: Option<ShapeId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct FaceHolding<N = AnyNodeId> {
    pub territory: N,
    pub holding: Option<HoldingState>,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct FaceTransfer<N = AnyNodeId> {
    pub subset: ShapeId,
    pub boundary: RegionGeom<ArcRef>,
    pub from: Option<FaceHolding<N>>,
    pub to: Option<FaceHolding<N>>,
    pub basis: Basis,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct RecordedMapChange<N = AnyNodeId> {
    pub year: i32,
    pub before_year: i32,
    pub causes: Vec<RecordedMapCause>,
    pub arcs: Vec<ArcChange>,
    pub transfers: Vec<FaceTransfer<N>>,
    pub script: TransitionScript<N, TransitionBoundary>,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct MapKeyframe<N = AnyNodeId> {
    pub change: RecordedMapChange<N>,
    pub before: FeatureCollection,
    pub after: FeatureCollection,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct MapTransition<N = AnyNodeId> {
    pub from_year: i32,
    pub to_year: i32,
    pub origin: FeatureCollection,
    pub destination: FeatureCollection,
    pub shapes: Vec<StoredShape>,
    pub keyframes: Vec<MapKeyframe<N>>,
    pub after_year: Option<i32>,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(tag = "reason", rename_all = "kebab-case", deny_unknown_fields))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum TransitionRefusal {
    MissingGrounds { year: i32 },
    MissingCorrespondence {
        year: i32,
        #[cfg_attr(feature = "serde", serde(skip_serializing))]
        #[cfg_attr(feature = "openapi", schema(ignore))]
        shape: String,
    },
    UnclassifiedSubset {
        year: i32,
        #[cfg_attr(feature = "serde", serde(skip_serializing))]
        #[cfg_attr(feature = "openapi", schema(ignore))]
        shape: String,
    },
    UnresolvedDating { causes: Vec<RecordedMapCause> },
    EndpointMismatch,
    InvalidWindow,
    NotRecorded,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(tag = "status", rename_all = "kebab-case", deny_unknown_fields))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum TransitionRead<N = AnyNodeId> {
    Ready { transition: MapTransition<N> },
    Refused { reason: TransitionRefusal, destination: FeatureCollection },
}


pub fn admit_script<R, B>(script: TransitionScript<R, B>) -> Result<TransitionScript<R, B>, TransitionRefusal> {
    if script.steps.iter().all(|draw| match &draw.step {
        TransitionStep::Morph { from_pts, to_pts, .. } => paired(from_pts, to_pts),
        TransitionStep::SegmentTrack { from, to, .. } => paired(from, to),
        TransitionStep::SegmentFade { course, .. } => course.len() >= 2,
        TransitionStep::FadeIn { .. } | TransitionStep::FadeOut { .. }
        | TransitionStep::SplitAlong { .. } | TransitionStep::MergeAcross { .. }
        | TransitionStep::PointTrack { .. } | TransitionStep::PointFade { .. }
        | TransitionStep::LabelTrack { .. } | TransitionStep::LabelFade { .. } => true,
    }) { Ok(script) } else { Err(TransitionRefusal::EndpointMismatch) }
}

fn paired(from: &[crate::geom::UnitVec], to: &[crate::geom::UnitVec]) -> bool {
    from.len() >= 2 && from.len() == to.len()
}

#[cfg(feature = "serde")]
#[path = "atlas_transition_codec.rs"]
mod codec;
