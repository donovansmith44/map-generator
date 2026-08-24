//! THE FRONT DOOR: RenderQuery. The renderable universe (spec §B) has
//! no privileged "draw the world" path — EVERY node of the map graph is
//! a query subject, every output is a Scene, so every output overlays
//! with every other. Snapshot and accumulation queries are the same
//! type: the time selector decides which.
//!
//! Queries are content-addressed: query hash = cache key = artifact
//! filename. That is the whole offline story (law 1).

use atlas_graph_types::covenant::TimePoint;

use crate::boundary::AtlasPlaceRef;
use crate::geom::{Bbox, Lod, UnitVec};
use crate::ident::{BoundaryId, Canon, ChangeEventId, MapAddressed, MapKind, RegionId, StyleId};
use crate::style::LayerSet;
use crate::timeline::{canon_time_point, Interval};

/// Anything the system can be asked to draw — from a point in space, to
/// one border, to a delta, to the whole world.
#[derive(Clone, Debug, PartialEq)]
pub enum RenderSubject {
    /// A gazetteer place: styled marker in period dress.
    Point(AtlasPlaceRef),
    /// An arbitrary point in space.
    RawPoint(UnitVec),
    /// ONE border, alone (JOS 15 as a drawn line).
    Boundary(BoundaryId),
    /// One region's border, fill optional.
    Region(RegionId),
    /// The region clipped over relief.
    RegionTerrain(RegionId),
    /// A DELTA, rendered: before-stroke, after-stroke, the seam.
    Change(ChangeEventId),
    /// Everything in the viewport.
    World,
}

/// Snapshot vs accumulation, UNIFIED: At renders an instant; Over
/// renders the long exposure of an interval.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TimeSelector {
    At(TimePoint),
    Over(Interval),
}

#[derive(Clone, Debug, PartialEq)]
pub struct RenderQuery {
    pub subject: RenderSubject,
    pub time: TimeSelector,
    /// None = auto-frame to the subject's own extent.
    pub viewport: Option<Bbox>,
    pub lod: Lod,
    pub layers: LayerSet,
    pub style: StyleId,
}

impl MapAddressed for RenderQuery {
    fn canonical_bytes(&self) -> Vec<u8> {
        let mut c = Canon::new();
        c.tag("render-query");
        match &self.subject {
            RenderSubject::Point(p) => {
                c.u8_(0).str_(&p.0 .0);
            }
            RenderSubject::RawPoint(v) => {
                c.u8_(1);
                v.canon(&mut c);
            }
            RenderSubject::Boundary(b) => {
                c.u8_(2).u64_(b.0 .0);
            }
            RenderSubject::Region(r) => {
                c.u8_(3).u64_(r.0 .0);
            }
            RenderSubject::RegionTerrain(r) => {
                c.u8_(4).u64_(r.0 .0);
            }
            RenderSubject::Change(e) => {
                c.u8_(5).u64_(e.0 .0);
            }
            RenderSubject::World => {
                c.u8_(6);
            }
        }
        match &self.time {
            TimeSelector::At(t) => {
                c.u8_(0);
                canon_time_point(&mut c, t);
            }
            TimeSelector::Over(i) => {
                c.u8_(1);
                canon_time_point(&mut c, &i.from);
                c.opt(&i.to, |c, t| canon_time_point(c, t));
            }
        }
        c.opt(&self.viewport, |c, v| v.canon(c));
        self.lod.canon(&mut c);
        c.u8_(self.layers.bits());
        c.u64_(self.style.0 .0);
        c.done()
    }
    fn map_kind(&self) -> MapKind {
        MapKind::Query
    }
}
