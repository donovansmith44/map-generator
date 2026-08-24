//! Boundaries and regions. Boundaries are first-class and SHARED: two
//! neighboring polities reference ONE arc, so sliver gaps are
//! unrepresentable, scholarship edits one place, and a morphing arc
//! moves both neighbors together (spec §B).

use atlas_graph_types::covenant::Justification;
use atlas_graph_types::covenant::{PlaceId, SourceId};
use atlas_graph_types::covenant::ProvenanceId;
use atlas_graph_types::covenant::BibleLocusRange;

use crate::geom::UnitVec;
use crate::ident::{BoundaryId, RegionId};

/// Honesty as a type: what KIND of edge is this? Ancient borders were
/// mostly frontier gradients, not lines — the renderer must be told,
/// never left to invent crispness (covenant rule 5).
#[derive(Clone, Debug, PartialEq)]
pub enum EdgeCharacter {
    /// Genuinely attested precise line (a river, a wall).
    Line,
    /// Gradient of control — renders as a zone, never a stroke.
    Frontier { width_km: f64 },
    /// Multiple simultaneous claims, all named.
    Disputed { claimants: Vec<RegionId> },
    /// Scholarship is silent — renders distinctly, never invented.
    Unknown,
}

/// A reference into the atlas gazetteer (contract C3): the atlas owns
/// coordinates; a place moving there moves every border built through it.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AtlasPlaceRef(pub PlaceId);

/// How authored geometry between survey waypoints was drawn — the only
/// authored geometry in a survey boundary, and its method is disclosed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InterpolationMethod {
    Geodesic,
    TerrainValley,
    Coast,
}

/// Scripture contains literal border surveys (NUM 34:1-12, JOS 15:1-12,
/// JOS 16-19, 2KI 14:25). A survey-derived boundary is CONSTRUCTED from
/// the text: waypoints are atlas PlaceIds in text order; the verses are
/// the justification grounds (contract C3+C4).
#[derive(Clone, Debug, PartialEq)]
pub struct BorderSurvey {
    pub verses: BibleLocusRange,
    pub waypoints: Vec<AtlasPlaceRef>,
    pub interpolation: InterpolationMethod,
    pub provenance: ProvenanceId,
}

/// Where a boundary's geometry came from — the authority ladder of
/// covenant rule 11: Scripture surveys first; imported scholarship
/// fills silence, always labeled; authored geometry always justified.
#[derive(Clone, Debug, PartialEq)]
pub enum BoundarySource {
    /// Bible-driven: the text IS the border.
    Survey(BorderSurvey),
    /// Scholarship fills what Scripture is silent on — labeled.
    Imported { source: SourceId },
    Authored { justification: Justification },
}

/// A shared border arc: an OPEN polyline. A border claim cites its
/// grounds exactly like a date claim (covenant rule 4).
#[derive(Clone, Debug, PartialEq)]
pub struct Boundary {
    pub pts: Vec<UnitVec>,
    pub character: EdgeCharacter,
    pub source: BoundarySource,
    pub justification: Justification,
    pub provenance: ProvenanceId,
}

/// Which way a region walks a shared arc.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Orientation {
    Forward,
    Reverse,
}

/// One connected piece of a region: an outer cycle of oriented boundary
/// references, plus hole cycles. Consecutive arcs must connect
/// end-to-start — the cycle-continuity validator proves it (law 2,
/// structural half).
///
/// CONVENTION: an EMPTY outer cycle means THE WHOLE SPHERE — the part
/// is everything, minus its holes. This is how the world ocean exists:
/// the sphere minus the land, with no fictitious envelope boundary.
#[derive(Clone, Debug, PartialEq)]
pub struct RegionPart {
    pub cycle: Vec<(BoundaryId, Orientation)>,
    pub holes: Vec<Vec<(BoundaryId, Orientation)>>,
}

/// A region's geometry. AMENDMENT to the §B sketch (phase-2 finding,
/// recorded for owner review): real polities are routinely several
/// separate pieces of land — a mainland and its islands — so a region
/// is a LIST of parts, not one cycle. The sketch's single-cycle shape
/// is the one-part case.
#[derive(Clone, Debug, PartialEq)]
pub struct RegionGeom {
    pub parts: Vec<RegionPart>,
}
