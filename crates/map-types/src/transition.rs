//! Transitions: slerp morphs + typed topology changes. Continuous
//! drift MORPHS; topology change gets its own honest verb — an empire
//! does not "morph" into its successors, it SPLITS.
//!
//! Scripts compose (law 3, law 8): identity is the empty script (t->t),
//! sequencing is associative, and composed scripts end where the direct
//! script ends. The semantic script is format-free; concrete animation
//! encodings are terminal encoder work (law 11).

use crate::algebra::Monoid;
use crate::geom::UnitVec;
use crate::ident::{BoundaryId, RegionId};

#[derive(Clone, Debug, PartialEq)]
pub enum TransitionStep {
    /// Equal point counts; pairs interpolate by slerp (law 4).
    Morph { boundary: BoundaryId, from_pts: Vec<UnitVec>, to_pts: Vec<UnitVec> },
    FadeIn { region: RegionId },
    FadeOut { region: RegionId },
    SplitAlong { parent: RegionId, seam: Vec<UnitVec>, children: Vec<RegionId> },
    MergeAcross { parents: Vec<RegionId>, child: RegionId },
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct TransitionScript {
    pub steps: Vec<TransitionStep>,
}

/// Sequencing: run `self`, then `other`. The identity is the empty
/// script — exactly what transition(t, t) must return.
impl Monoid for TransitionScript {
    fn empty() -> Self {
        TransitionScript::default()
    }
    fn combine(mut self, mut other: Self) -> Self {
        self.steps.append(&mut other.steps);
        self
    }
}
