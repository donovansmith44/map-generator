//! The semantic scene: styled geometry + labels + attribution with NO
//! commitment to any encoding. ALL composition — the overlay monoid,
//! accumulation folds — happens HERE, at the semantic level, never on
//! encoded bytes (law 11).
//!
//! DELIBERATE AMENDMENT to the §B sketch (recorded for owner review):
//! the sketch's Snapshot held only regions + labels, but the renderable
//! universe (§B RenderSubject) says every subject — a lone boundary, a
//! point, a delta — renders to a Scene. So the scene carries styled
//! boundaries and markers alongside regions; a subject that renders
//! nothing of a kind leaves that list empty.

use std::collections::BTreeSet;

use atlas_graph_types::covenant::TimePoint;
use atlas_graph_types::covenant::SourceId;

use crate::algebra::{mconcat, Monoid};
use crate::geom::{Ring, UnitVec};
use crate::ident::{BoundaryId, Canon, MapAddressed, MapKind, RegionId};
use crate::style::{LabelStyle, MarkerStyle, Paint, Stroke};
use crate::timeline::{ChangeEvent, Interval};

/// A region with its geometry resolved to rings and its paint resolved
/// from the style. Simplification (lod) already applied.
///
/// AMENDMENT (phase 3, recorded for review): every styled element
/// carries the sources it came from, so scene attribution is always
/// derivable from content — which is what lets selection agree with
/// lone rendering down to the attribution set (law 10).
#[derive(Clone, Debug, PartialEq)]
pub struct StyledRegion {
    pub region: RegionId,
    /// The entity string the composable API speaks (`pieces=…`) —
    /// stamped as `data-entity` so a composed artifact stays
    /// addressable by the same names the caller requested.
    pub entity: Option<String>,
    pub outer: Vec<Ring>,
    pub holes: Vec<Ring>,
    pub paint: Paint,
    pub sources: BTreeSet<SourceId>,
    /// WHICH PIECE this face belongs to — Ground, Water, Fills or
    /// Claims. The scene type used to carry no such notion, so the
    /// provider recorded a paint RANK at push time (`scene_at`'s
    /// `paint_rank`, "recorded at push time because the scene type
    /// carries no layer") and threw the rest away: a fill and its
    /// border could not be asked for apart, and a claim could not be
    /// told from a territory at all (diagnosis §3.2).
    pub piece: crate::piece::Piece,
}

/// One border arc, styled — JOS 15 as a drawn line, alone if asked.
#[derive(Clone, Debug, PartialEq)]
pub struct StyledBoundary {
    pub boundary: BoundaryId,
    pub pts: Vec<UnitVec>,
    pub stroke: Stroke,
    pub sources: BTreeSet<SourceId>,
    /// WHICH PIECE this edge belongs to — Borders, Claims, Water (a
    /// river), Journeys (a road) or Ground (a range render's age-tinted
    /// relief outline). The scene type used to carry no such notion, so
    /// an outline was inseparable from the face it enclosed: turning
    /// fills off took every border with it (diagnosis §3.2).
    pub piece: crate::piece::Piece,
}

/// A styled point — a place in period dress, or a raw point. Carries
/// its sources like regions and boundaries do (honesty at element
/// grain — a semantic selection can keep or drop it by provenance).
#[derive(Clone, Debug, PartialEq)]
pub struct StyledMarker {
    pub at: UnitVec,
    pub style: MarkerStyle,
    pub sources: BTreeSet<SourceId>,
    /// The gazetteer place this marker stands on, when it stands on
    /// one — selection follows markers by their place (law 10's
    /// spirit), never by guessing from position.
    pub place: Option<crate::boundary::AtlasPlaceRef>,
    /// WHICH PIECE this standing point belongs to — Markers for a
    /// gazetteer landmark, Journeys for a station on a walked road.
    /// THE DIAGNOSIS'S OWN CASE: with no piece here the encoder could
    /// only group markers by PAINT, so one points buffer held every
    /// origin's markers, and omitting journeys changed the bytes of a
    /// buffer other pieces were using — geometry appeared when a piece
    /// was turned OFF (diagnosis §3.2).
    pub piece: crate::piece::Piece,
}

/// What a label is attached to — selection (law 10) follows labels by
/// their subject, not by guessing from text.
#[derive(Clone, Debug, PartialEq)]
pub enum LabelSubject {
    Region(RegionId),
    Boundary(BoundaryId),
    /// A gazetteer place — a journey station, a landmark by name.
    Place(crate::boundary::AtlasPlaceRef),
    Free,
}

pub use crate::style::LabelFace;

#[derive(Clone, Debug, PartialEq)]
pub struct PlacedLabel {
    pub text: String,
    pub at: UnitVec,
    pub subject: LabelSubject,
    pub style: LabelStyle,
    /// what KIND of thing this names (drives layout semantics, e.g.
    /// water may overflow its shores)
    pub face: LabelFace,
    /// the fully resolved typographic dress, straight from the style
    pub voice: crate::style::TypeVoice,
    /// WHICH PIECE this name belongs to. Every label is the Labels
    /// piece — a name is its own piece, not a property of the thing
    /// named — which is what lets a caller ask for a silent map, or
    /// for names alone over someone else's ground. The scene type
    /// used to carry no such notion (diagnosis §3.2).
    pub piece: crate::piece::Piece,
}

/// The scene. Later entries paint over earlier ones — overlay order is
/// meaning, not accident. Attribution rides every response: licensing
/// is part of the data, not a footnote.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Snapshot {
    pub regions: Vec<StyledRegion>,
    pub boundaries: Vec<StyledBoundary>,
    pub markers: Vec<StyledMarker>,
    pub labels: Vec<PlacedLabel>,
    pub attribution: BTreeSet<SourceId>,
}

/// "Overlay maps in a clean fashion" IS this monoid (law 8): identity
/// is the empty scene, combine draws `other` over `self`, attribution
/// unions.
impl Monoid for Snapshot {
    fn empty() -> Self {
        Snapshot::default()
    }
    fn combine(mut self, mut other: Self) -> Self {
        self.regions.append(&mut other.regions);
        self.boundaries.append(&mut other.boundaries);
        self.markers.append(&mut other.markers);
        self.labels.append(&mut other.labels);
        self.attribution.extend(other.attribution);
        self
    }
}

impl MapAddressed for Snapshot {
    fn canonical_bytes(&self) -> Vec<u8> {
        let mut c = Canon::new();
        c.tag("scene");
        c.seq(&self.regions, |c, r| {
            c.u64_(r.region.0 .0);
            c.str_(r.entity.as_deref().unwrap_or(""));
            c.seq(&r.outer, |c, ring| ring.canon(c));
            c.seq(&r.holes, |c, ring| ring.canon(c));
            r.paint.canon(c);
            let srcs: Vec<_> = r.sources.iter().collect();
            c.seq(&srcs, |c, s| {
                c.str_(&s.0);
            });
            // Two scenes differing only in attribution are genuinely
            // different answers, so the pid must see the piece.
            c.str_(r.piece.name());
        });
        c.seq(&self.boundaries, |c, b| {
            c.u64_(b.boundary.0 .0);
            c.seq(&b.pts, |c, p| p.canon(c));
            b.stroke.canon(c);
            let srcs: Vec<_> = b.sources.iter().collect();
            c.seq(&srcs, |c, s| {
                c.str_(&s.0);
            });
            c.str_(b.piece.name());
        });
        c.seq(&self.markers, |c, m| {
            m.at.canon(c);
            let crate::style::Rgba(r, g, bl, a) = m.style.color;
            c.u8_(r).u8_(g).u8_(bl).u8_(a).f64_(m.style.size);
            let srcs: Vec<_> = m.sources.iter().collect();
            c.seq(&srcs, |c, s| {
                c.str_(&s.0);
            });
            match &m.place {
                None => c.str_(""),
                Some(p) => c.str_(&p.0 .0),
            };
            c.str_(m.piece.name());
        });
        c.seq(&self.labels, |c, l| {
            c.str_(&l.text);
            l.at.canon(c);
            match &l.subject {
                LabelSubject::Region(r) => c.u8_(0).u64_(r.0 .0),
                LabelSubject::Boundary(b) => c.u8_(1).u64_(b.0 .0),
                LabelSubject::Free => c.u8_(2),
                LabelSubject::Place(p) => c.u8_(3).str_(&p.0 .0),
            };
            let crate::style::Rgba(r, g, bl, a) = l.style.color;
            c.u8_(r).u8_(g).u8_(bl).u8_(a);
            let crate::style::Rgba(r, g, bl, a) = l.style.halo;
            c.u8_(r).u8_(g).u8_(bl).u8_(a).f64_(l.style.size);
            c.str_(l.piece.name());
        });
        let sources: Vec<_> = self.attribution.iter().collect();
        c.seq(&sources, |c, s| {
            c.str_(&s.0);
        });
        c.done()
    }
    fn map_kind(&self) -> MapKind {
        MapKind::Scene
    }
}

impl Snapshot {
    /// Spec §3 law 1: `scene(q \ P)` equals `scene(q)` minus exactly P's
    /// contribution. With every element attributed, that is a filter —
    /// the law becomes a definition rather than a hope.
    pub fn restrict(&self, keep: crate::piece::PieceSet) -> Snapshot {
        Snapshot {
            regions: self.regions.iter().filter(|r| keep.contains(r.piece)).cloned().collect(),
            boundaries: self.boundaries.iter().filter(|b| keep.contains(b.piece)).cloned().collect(),
            markers: self.markers.iter().filter(|m| keep.contains(m.piece)).cloned().collect(),
            labels: self.labels.iter().filter(|l| keep.contains(l.piece)).cloned().collect(),
            attribution: self.attribution.clone(),
        }
    }

    /// Select one subject's contribution out of a scene. Law 10
    /// (selection coherence): a provider must make rendering a subject
    /// alone agree with selecting it out of the world — this is the
    /// selection side of that equation.
    pub fn select_region(&self, id: RegionId) -> Snapshot {
        let regions: Vec<StyledRegion> =
            self.regions.iter().filter(|r| r.region == id).cloned().collect();
        let attribution = regions.iter().flat_map(|r| r.sources.iter().cloned()).collect();
        Snapshot {
            regions,
            boundaries: Vec::new(),
            markers: Vec::new(),
            labels: self
                .labels
                .iter()
                .filter(|l| matches!(&l.subject, LabelSubject::Region(r) if *r == id))
                .cloned()
                .collect(),
            attribution,
        }
    }

    pub fn select_boundary(&self, id: BoundaryId) -> Snapshot {
        let boundaries: Vec<StyledBoundary> =
            self.boundaries.iter().filter(|b| b.boundary == id).cloned().collect();
        let attribution = boundaries.iter().flat_map(|b| b.sources.iter().cloned()).collect();
        Snapshot {
            regions: Vec::new(),
            boundaries,
            markers: Vec::new(),
            labels: self
                .labels
                .iter()
                .filter(|l| matches!(&l.subject, LabelSubject::Boundary(b) if *b == id))
                .cloned()
                .collect(),
            attribution,
        }
    }
}

/// The exact sample points for an accumulation over an interval: the
/// endpoints plus every change event inside (law 9). The timeline is
/// piecewise-constant, so these are the ONLY distinct snapshots —
/// uniform time-ticks would alias (miss a short-lived kingdom, or
/// re-render an unchanged century).
pub fn sample_times(over: &Interval, events: &[ChangeEvent]) -> Vec<TimePoint> {
    let mut ts = vec![over.from];
    for e in events {
        if e.at > over.from && over.contains(&e.at) {
            ts.push(e.at);
        }
        // A closed query interval also samples an event AT its end.
        if let Some(end) = over.to {
            if e.at == end {
                ts.push(e.at);
            }
        }
    }
    if let Some(end) = over.to {
        if end != over.from {
            ts.push(end);
        }
    }
    ts.sort();
    ts.dedup();
    ts
}

/// An accumulation is the STILL form of a transition: the fold of
/// overlay across the distinct snapshots of an interval — a
/// long-exposure photograph of the change events. Content addressing
/// dedups: identical snapshots hash identically and the fold touches
/// each distinct scene once, which is why adding a redundant sample
/// between change events changes nothing (law 9).
pub fn accumulate(snapshots: impl IntoIterator<Item = Snapshot>) -> Snapshot {
    let mut seen = BTreeSet::new();
    let distinct: Vec<Snapshot> = snapshots
        .into_iter()
        .filter(|s| seen.insert(s.map_pid()))
        .collect();
    mconcat(distinct)
}
