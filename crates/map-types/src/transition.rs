use crate::algebra::Monoid;
use crate::geom::UnitVec;
use crate::ident::{BoundaryId, RegionId};

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "step", rename_all = "kebab-case", deny_unknown_fields))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum TransitionStep<R = RegionId, B = BoundaryId> {
    Morph { boundary: B, from_pts: Vec<UnitVec>, to_pts: Vec<UnitVec> },
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
