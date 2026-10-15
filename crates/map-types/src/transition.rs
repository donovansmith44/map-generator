use crate::algebra::Monoid;
use crate::geom::UnitVec;
use crate::ident::{BoundaryId, RegionId};

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "step", rename_all = "kebab-case", deny_unknown_fields))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum TransitionStep<R = RegionId, B = BoundaryId> {
    Morph { entity: R, boundary: B, from_pts: Vec<UnitVec>, to_pts: Vec<UnitVec> },
    FadeIn { region: R },
    FadeOut { region: R },
    SplitAlong { parent: R, seam: Vec<UnitVec>, children: Vec<R> },
    MergeAcross { parents: Vec<R>, child: R },
    PointTrack { entity: R, from: UnitVec, to: UnitVec, extent: (PlaceExtentKind, PlaceExtentKind), radius_m: (u32, u32) },
    PointFade { entity: R, at: UnitVec, radius_m: u32, appears: bool },
    SegmentTrack { entity: R, segment: B, from: Vec<UnitVec>, to: Vec<UnitVec> },
    SegmentFade { entity: R, segment: B, course: Vec<UnitVec>, appears: bool },
    LabelTrack { entity: R, text: String, from: UnitVec, to: UnitVec },
    LabelFade { entity: R, from: Option<PlacedLabel>, to: Option<PlacedLabel> },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum PlaceExtentKind {
    Point,
    Vicinity,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct PlacedLabel {
    pub text: String,
    pub anchor: UnitVec,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct TransitionDraw<R = RegionId, B = BoundaryId> {
    pub step: TransitionStep<R, B>,
    pub clip: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct TransitionScript<R = RegionId, B = BoundaryId> {
    pub steps: Vec<TransitionDraw<R, B>>,
}

impl<R, B> From<TransitionStep<R, B>> for TransitionDraw<R, B> {
    fn from(step: TransitionStep<R, B>) -> Self { Self { step, clip: None } }
}

impl<R, B> Default for TransitionScript<R, B> {
    fn default() -> Self { Self { steps: Vec::new() } }
}

impl<R, B> Monoid for TransitionScript<R, B> {
    fn empty() -> Self { Self::default() }
    fn combine(mut self, mut other: Self) -> Self {
        self.steps.append(&mut other.steps);
        self
    }
}

pub fn invert<R, B>(script: TransitionScript<R, B>) -> TransitionScript<R, B> {
    let steps = script.steps.into_iter().rev().map(|draw| {
        let step = match draw.step {
            TransitionStep::Morph { entity, boundary, from_pts, to_pts } => TransitionStep::Morph { entity, boundary, from_pts: to_pts, to_pts: from_pts },
            TransitionStep::FadeIn { region } => TransitionStep::FadeOut { region },
            TransitionStep::FadeOut { region } => TransitionStep::FadeIn { region },
            TransitionStep::SplitAlong { parent, children, .. } => TransitionStep::MergeAcross { parents: children, child: parent },
            TransitionStep::MergeAcross { parents, child } => TransitionStep::SplitAlong { parent: child, seam: Vec::new(), children: parents },
            TransitionStep::PointTrack { entity, from, to, extent, radius_m } => TransitionStep::PointTrack { entity, from: to, to: from, extent: (extent.1,extent.0), radius_m: (radius_m.1,radius_m.0) },
            TransitionStep::PointFade { entity, at, radius_m, appears } => TransitionStep::PointFade { entity, at, radius_m, appears: !appears },
            TransitionStep::SegmentTrack { entity, segment, from, to } => TransitionStep::SegmentTrack { entity, segment, from: to, to: from },
            TransitionStep::SegmentFade { entity, segment, course, appears } => TransitionStep::SegmentFade { entity, segment, course, appears: !appears },
            TransitionStep::LabelTrack { entity, text, from, to } => TransitionStep::LabelTrack { entity, text, from: to, to: from },
            TransitionStep::LabelFade { entity, from, to } => TransitionStep::LabelFade { entity, from: to, to: from },
        };
        TransitionDraw { step, clip: draw.clip }
    }).collect();
    TransitionScript { steps }
}
