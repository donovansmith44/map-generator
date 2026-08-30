//! The canon-backed MapProvider (phase 4 of the 2026-08-27 design):
//! the SAME contract the encoders and workbench already speak, served
//! from the compiled CanonStore instead of the interval timeline. The
//! provider is dumb on purpose — witnesses, reconciliation, and layer
//! discipline all happened at compile time.
//!
//! Identity plumbing: canon entities are strings; the contract's
//! subject ids are hashes. `rid_of(entity)` is the stable mapping, and
//! the reverse map is built at construction — clicking, focusing, and
//! subject listings keep working unchanged.

use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, BTreeSet};
use std::hash::{Hash, Hasher};

use atlas_graph_types::covenant::{ContentHash, PlaceId, SourceId, TimePoint};
use map_canon::{
    Area, CanonStore, EntityId, Feature, FeatureId, LayerKind, Route, Timestamp, Witness,
};
use map_types::scene::{LabelSubject, StyledMarker};
use map_types::style::Paint;
use map_types::Monoid;
use map_types::{
    slerp, Bbox, BoundaryId, ChangeEvent, ChangeKind, GazetteerExport, LayerSet, Lod, MapError,
    MapProvider, PlacedLabel, RegionId, RenderQuery, RenderSubject, Ring, Snapshot, Style, StyleId,
    StyledBoundary, StyledRegion, SubjectListing, TimeSelector, TransitionScript, TransitionStep,
    UnitVec,
};

/// The scripture tag consumers filter on (bible mode): atlas and
/// authored truth carries it; scholarship base data does not.
const SCRIPTURE_SOURCE: &str = "scripture";

pub struct CanonProvider {
    store: CanonStore,
    styles: BTreeMap<StyleId, Style>,
    gazetteer: Option<GazetteerExport>,
    entity_by_rid: BTreeMap<RegionId, EntityId>,
    entity_by_bid: BTreeMap<BoundaryId, EntityId>,
    events: Vec<ChangeEvent>,
    /// palette slot per area entity, assigned ONCE at load by graph
    /// coloring over the shared-border graph — the style's promise
    /// that touching territories never match is made true here, and
    /// an entity keeps its slot no matter which pieces are rendered.
    palette_slot: BTreeMap<EntityId, usize>,
    /// relief band position 0..1 by measured area order (largest =
    /// lowest = 0), computed once at load.
    relief_pos: BTreeMap<EntityId, f64>,
    /// label anchor per Area feature, computed ONCE at load: the pole
    /// of inaccessibility is a pure function of content-addressed
    /// geometry, so recomputing it per render was pure waste (it was
    /// 96% of a world render's time, measured).
    label_anchor: BTreeMap<FeatureId, UnitVec>,
    /// each border's spherical cap (centroid direction, angular
    /// radius), computed once — culling a render to its viewport is
    /// one angle test per border instead of a walk of the world.
    border_cap: BTreeMap<map_canon::BorderId, (UnitVec, f64)>,
}

fn hash64(s: &str) -> u64 {
    let mut h = DefaultHasher::new();
    s.hash(&mut h);
    h.finish()
}

pub fn rid_of(entity: &EntityId) -> RegionId {
    RegionId(ContentHash(hash64(&format!("entity:{}", entity.0))))
}

fn bid_of(entity: &EntityId) -> BoundaryId {
    BoundaryId(ContentHash(hash64(&format!("entity:{}", entity.0))))
}

fn year_index(y: i32) -> i32 {
    if y > 0 {
        y - 1
    } else {
        y
    }
}

/// Which canon layers a LayerSet bit vocabulary asks for.
fn layers_wanted(bits: LayerSet) -> Vec<LayerKind> {
    let mut out = Vec::new();
    if bits.contains(LayerSet::GEOMETRY) {
        out.extend([LayerKind::Background, LayerKind::Territory, LayerKind::ScriptureClaims]);
    }
    if bits.contains(LayerSet::TOPOGRAPHY) {
        out.push(LayerKind::Water);
    }
    if bits.contains(LayerSet::RELIEF) {
        out.push(LayerKind::Relief);
    }
    if bits.contains(LayerSet::JOURNEYS) {
        out.push(LayerKind::Journeys);
    }
    out
}

fn witness_source(w: Witness) -> SourceId {
    SourceId::new(match w {
        Witness::Atlas => "witness:atlas",
        Witness::Authored => "witness:authored",
        Witness::Basemap => "witness:basemap",
        Witness::NaturalEarth => "witness:natural-earth",
    })
}

impl CanonProvider {
    pub fn new(
        store: CanonStore,
        styles: BTreeMap<StyleId, Style>,
        gazetteer: Option<GazetteerExport>,
    ) -> Self {
        let mut entity_by_rid = BTreeMap::new();
        let mut entity_by_bid = BTreeMap::new();
        for f in store.features().values() {
            entity_by_rid.insert(rid_of(f.entity()), f.entity().clone());
            entity_by_bid.insert(bid_of(f.entity()), f.entity().clone());
        }
        let t0 = std::time::Instant::now();
        let events = derive_events(&store);
        eprintln!("  events {:.1}s", t0.elapsed().as_secs_f64());
        let t0 = std::time::Instant::now();
        let palette_slot = palette_slots(&store);
        eprintln!("  palette {:.1}s", t0.elapsed().as_secs_f64());
        let t0 = std::time::Instant::now();
        let relief_pos = relief_positions(&store);
        eprintln!("  relief rank {:.1}s", t0.elapsed().as_secs_f64());
        let t0 = std::time::Instant::now();
        let label_anchor = label_anchors(&store);
        eprintln!("  label anchors {:.1}s", t0.elapsed().as_secs_f64());
        let border_cap = border_caps(&store);
        CanonProvider {
            store,
            styles,
            gazetteer,
            entity_by_rid,
            entity_by_bid,
            events,
            palette_slot,
            relief_pos,
            label_anchor,
            border_cap,
        }
    }

    pub fn from_canon_file(
        path: &std::path::Path,
        styles: BTreeMap<StyleId, Style>,
        gazetteer: Option<GazetteerExport>,
    ) -> Result<Self, String> {
        let t0 = std::time::Instant::now();
        let bytes = std::fs::read(path).map_err(|e| format!("canon: {e}"))?;
        let store = map_canon::persist::from_bytes(&bytes)?;
        eprintln!("  canon parse {:.1}s", t0.elapsed().as_secs_f64());
        Ok(Self::new(store, styles, gazetteer))
    }

    /// The polyline of a feature, for morphing: an area's longest
    /// ring, a line's border. Ways morph nothing.
    fn morph_pts(&self, f: &Feature) -> Option<Vec<UnitVec>> {
        match f {
            Feature::Area(a) => a
                .rings
                .iter()
                .filter_map(|bid| self.store.borders().get(bid).map(|b| b.0.clone()))
                .max_by_key(Vec::len),
            Feature::Line(l) => self.store.borders().get(&l.border).map(|b| b.0.clone()),
            _ => None,
        }
    }

    /// A Shift event's before/after geometry: the entity's feature in
    /// the snapshot AT the event moment and in the moment before it,
    /// searched across layers (the event's boundary id names the
    /// entity, and an entity lives in one layer).
    fn shift_geometries(
        &self,
        boundary: &BoundaryId,
        at: &Timestamp,
    ) -> Option<(Vec<UnitVec>, Vec<UnitVec>)> {
        let ent = self.entity_by_bid.get(boundary)?;
        for world in self.store.layers().values() {
            let Some(sid_after) = world.moments().get(at) else { continue };
            let sid_before = world
                .moments()
                .range(..at.clone())
                .next_back()
                .map(|(_, s)| *s)?;
            let find = |sid: &map_canon::SnapshotId| -> Option<Vec<UnitVec>> {
                let snap = self.store.snapshots().get(sid)?;
                snap.features
                    .iter()
                    .filter_map(|fid| self.store.features().get(fid))
                    .find(|f| f.entity() == ent)
                    .and_then(|f| self.morph_pts(f))
            };
            if let (Some(before), Some(after)) = (find(&sid_before), find(sid_after)) {
                return Some((before, after));
            }
        }
        None
    }

    fn style(&self, id: StyleId) -> Result<&Style, MapError> {
        self.styles.get(&id).ok_or(MapError::UnknownStyle(id))
    }

    fn sources_of(&self, fid: FeatureId) -> BTreeSet<SourceId> {
        let mut out = BTreeSet::new();
        if let Some(p) = self.store.provenance().get(&fid) {
            out.insert(witness_source(p.witness));
            if matches!(p.witness, Witness::Atlas | Witness::Authored) {
                out.insert(SourceId::new(SCRIPTURE_SOURCE));
            }
        }
        out
    }

    /// select ∘ SIMPLIFY ∘ style — the stage the canon provider had
    /// dropped: geometry leaves here at the query's level of detail,
    /// and a border whose cap misses the viewport never leaves at all.
    fn ring_points(&self, id: map_canon::BorderId, q: &RenderQuery) -> Option<Ring> {
        if self.border_culled(id, q) {
            return None;
        }
        let b = self.store.borders().get(&id)?;
        // fidelity may thin a ring, never erase it: a ring the
        // tolerance would collapse ships unsimplified instead, so a
        // small territory is present at every level of detail
        match Ring::new(map_types::simplify_polyline(&b.0, q.lod)) {
            Ok(r) => Some(r),
            Err(_) => Ring::new(b.0.clone()).ok(),
        }
    }

    fn border_culled(&self, id: map_canon::BorderId, q: &RenderQuery) -> bool {
        let Some(view) = &q.viewport else { return false };
        let Some((center, radius)) = self.border_cap.get(&id) else { return false };
        center.angle_to(&view.center) > view.radius + radius
    }

    fn line_points(&self, id: map_canon::BorderId, q: &RenderQuery) -> Option<Vec<UnitVec>> {
        if self.border_culled(id, q) {
            return None;
        }
        let b = self.store.borders().get(&id)?;
        Some(map_types::simplify_polyline(&b.0, q.lod))
    }

    /// The features active at `t` in one layer, in deterministic order.
    fn active(&self, layer: LayerKind, t: &Timestamp) -> Vec<(FeatureId, &Feature)> {
        let Some(world) = self.store.layers().get(&layer) else { return Vec::new() };
        let Some(sid) = world.state_at(t) else { return Vec::new() };
        let Some(snap) = self.store.snapshots().get(&sid) else { return Vec::new() };
        snap.features
            .iter()
            .filter_map(|fid| self.store.features().get(fid).map(|f| (*fid, f)))
            .collect()
    }

    fn area_paint(&self, layer: LayerKind, entity: &EntityId, style: &Style) -> Paint {
        match layer {
            LayerKind::Water => style.water_paint(),
            LayerKind::Relief => {
                // Bands tint along the topo ramp by MEASURED order: a
                // lower band always encloses more area than the band
                // above it, so area rank IS elevation rank — no name
                // parsing, no hashing.
                let ramp = style.topo_ramp();
                let t = self.relief_pos.get(entity).copied().unwrap_or(0.0);
                mix(ramp.oldest, ramp.newest, t)
            }
            _ => match style.palette() {
                Some(slots) => {
                    let i = self
                        .palette_slot
                        .get(entity)
                        .copied()
                        .unwrap_or_else(|| (hash64(&entity.0) % 8) as usize);
                    slots[i]
                }
                None => style.region_paint(),
            },
        }
    }

    fn push_area(
        &self,
        scene: &mut Snapshot,
        layer: LayerKind,
        fid: FeatureId,
        a: &Area,
        t: &Timestamp,
        q: &RenderQuery,
        style: &Style,
    ) {
        let _ = t;
        let sources = self.sources_of(fid);
        let mut outer = Vec::new();
        let mut holes = Vec::new();
        for r in &a.rings {
            if let Some(ring) = self.ring_points(*r, q) {
                outer.push(ring);
            }
        }
        for h in &a.holes {
            if let Some(ring) = self.ring_points(*h, q) {
                holes.push(ring);
            }
        }
        if outer.is_empty() {
            return;
        }
        // The area's outline rides as a stroke too — atlas/authored
        // territory in the Line dress, background scholarship dashed.
        let character = match layer {
            LayerKind::Background => map_types::EdgeCharacter::Unknown,
            _ => map_types::EdgeCharacter::Line,
        };
        if layer != LayerKind::Relief && layer != LayerKind::Water {
            for ring in &outer {
                scene.boundaries.push(StyledBoundary {
                    boundary: bid_of(&a.entity),
                    pts: ring.points().to_vec(),
                    stroke: *style.stroke_for(&character),
                    sources: sources.clone(),
                });
            }
        }
        if q.layers.contains(LayerSet::LABELS) && layer != LayerKind::Relief {
            if let Some(at) =
                self.label_anchor.get(&fid).copied().or_else(|| centroid(&outer)) {
                let labeling = style.labeling();
                let face = if layer == LayerKind::Water {
                    map_types::scene::LabelFace::Water
                } else {
                    map_types::scene::LabelFace::Territory
                };
                let mut label = labeling.base;
                // the label grows with the ground it names, by the
                // style's own declared scaling law
                let scale = labeling.scale;
                let sr = outer.iter().map(|r| ring_area_sr(r).abs()).fold(0.0, f64::max);
                label.size *= (sr / scale.unit_area_sr).sqrt().clamp(scale.min, scale.max);
                if face == map_types::scene::LabelFace::Water {
                    // water speaks in its own deep color
                    let map_types::style::Rgba(r, g, b, _) =
                        self.area_paint(layer, &a.entity, style).fill;
                    let dim = |v: u8| (f64::from(v) * scale.water_ink) as u8;
                    label.color = map_types::style::Rgba(dim(r), dim(g), dim(b), 255);
                    label.size *= scale.water_shrink;
                }
                scene.labels.push(PlacedLabel {
                    text: a.name.clone(),
                    at,
                    subject: LabelSubject::Region(rid_of(&a.entity)),
                    style: label,
                    face,
                    voice: labeling.voice(face),
                });
            }
        }
        scene.attribution.extend(sources.iter().cloned());
        scene.regions.push(StyledRegion {
            region: rid_of(&a.entity),
            entity: Some(a.entity.0.clone()),
            outer,
            holes,
            paint: self.area_paint(layer, &a.entity, style),
            sources,
        });
    }

    /// The road walked by `t`: passed legs in full, the in-progress leg
    /// clipped at its own granularity, future legs absent. Returns the
    /// polyline plus how many stations have been reached.
    fn walked(&self, route: &Route, t: &Timestamp) -> (Vec<UnitVec>, usize) {
        let mut pts: Vec<UnitVec> = Vec::new();
        let mut stations = 0usize;
        for (i, leg) in route.legs.iter().enumerate() {
            let Some(border) = self.store.borders().get(&leg.border) else { continue };
            if *t < leg.span.0 {
                break; // not yet departed on this leg
            }
            if i == 0 {
                stations = 1; // standing at the first station
            }
            let s0 = year_index(leg.span.0.year.get());
            let s1 = year_index(leg.span.1.year.get());
            let done = *t >= leg.span.1;
            let f = if done {
                1.0
            } else {
                let dur = f64::from((s1 - s0 + 1).max(1));
                let gone = f64::from((year_index(t.year.get()) - s0 + 1).max(0));
                (gone / dur).clamp(0.0, 1.0)
            };
            let seg = walked_prefix(&border.0, f);
            if !seg.is_empty() && pts.last() == seg.first() {
                pts.extend(seg.into_iter().skip(1));
            } else {
                pts.extend(seg);
            }
            if done {
                stations = i + 2; // arrived at this leg's destination
            }
            if !done {
                break;
            }
        }
        (pts, stations)
    }

    fn push_way(
        &self,
        scene: &mut Snapshot,
        fid: FeatureId,
        route: &Route,
        t: &Timestamp,
        q: &RenderQuery,
        style: &Style,
    ) {
        let (pts, reached) = self.walked(route, t);
        if pts.len() < 2 {
            return;
        }
        let sources = self.sources_of(fid);
        scene.attribution.extend(sources.iter().cloned());
        scene.boundaries.push(StyledBoundary {
            boundary: bid_of(&route.entity),
            pts,
            stroke: *style.stroke_for(&map_types::EdgeCharacter::Way),
            sources: sources.clone(),
        });
        // Stations, as reached, named from the gazetteer.
        let mut station_places: Vec<&atlas_graph_types::covenant::PlaceId> = Vec::new();
        if let Some(first) = route.legs.first() {
            station_places.push(&first.from);
        }
        for leg in &route.legs {
            station_places.push(&leg.to);
        }
        for pid in station_places.into_iter().take(reached) {
            let Some(gaz) = self.gazetteer.as_ref() else { continue };
            let Some(entry) = gaz.places.get(pid) else { continue };
            scene.markers.push(StyledMarker {
                at: entry.position,
                style: style.marker_style(),
                sources: sources.clone(),
                place: Some(map_types::AtlasPlaceRef(pid.clone())),
            });
            if q.layers.contains(LayerSet::LABELS) {
                let mut label = style.label_style();
                label.size *= 0.8;
                scene.labels.push(PlacedLabel {
                    text: entry.canonical_name.clone(),
                    at: entry.position,
                    subject: LabelSubject::Place(map_types::AtlasPlaceRef(pid.clone())),
                    style: label,
                    face: map_types::scene::LabelFace::Place,
                    voice: style.labeling().place,
                });
            }
        }
    }

    fn scene_at(
        &self,
        t: &Timestamp,
        q: &RenderQuery,
        pieces: Option<&BTreeSet<EntityId>>,
    ) -> Result<Snapshot, MapError> {
        let style = self.style(q.style)?;
        let mut scene = Snapshot::empty();
        let subject_only: Option<BTreeSet<EntityId>> = match &q.subject {
            RenderSubject::Region(rid) => Some(BTreeSet::from([self
                .entity_by_rid
                .get(rid)
                .ok_or(MapError::UnknownRegion(*rid))?
                .clone()])),
            _ => None,
        };
        let only: Option<&BTreeSet<EntityId>> = pieces.or(subject_only.as_ref());
        // Paint rank per region: the stage under everything, claims
        // over it, water above every claim (a lake is never buried),
        // recorded at push time because the scene type carries no
        // layer.
        let mut paint_rank: BTreeMap<map_types::RegionId, u8> = BTreeMap::new();
        for layer in layers_wanted(q.layers) {
            let rank = match layer {
                LayerKind::Background => 0u8,
                LayerKind::Relief => 1,
                // the scripture-frame (Canaan, the allotments, the
                // nations) lies beneath the POLITICAL layer: eras
                // hand off between them, and when both speak at once
                // the kingdom paints over the frame it rose from
                LayerKind::ScriptureClaims => 2,
                LayerKind::Territory => 3,
                LayerKind::Water => 4,
                LayerKind::Journeys => 5,
            };
            for (fid, f) in self.active(layer, t) {
                if let Some(set) = only {
                    if !set.contains(f.entity()) {
                        continue;
                    }
                }
                match f {
                    Feature::Area(a) => {
                        paint_rank.insert(rid_of(&a.entity), rank);
                        self.push_area(&mut scene, layer, fid, a, t, q, style)
                    }
                    Feature::Way(r) => self.push_way(&mut scene, fid, r, t, q, style),
                    // A Line (a river): the border geometry stroked in
                    // the water color — never a filled area, so it can
                    // neither gap nor balloon.
                    Feature::Line(l) => {
                        // read the border directly: a line is an OPEN
                        // path and may be as short as two points —
                        // simplified and viewport-culled like any ring.
                        if let Some(pts) = self.line_points(l.border, q) {
                            let sources = self.sources_of(fid);
                            scene.attribution.extend(sources.iter().cloned());
                            scene.boundaries.push(map_types::StyledBoundary {
                                boundary: bid_of(&l.entity),
                                pts,
                                stroke: map_types::style::Stroke {
                                    color: style.water_paint().fill,
                                    // RIVER DISPLAY WIDTH ~600 m: paired
                                    // with the 8 px corridor raster in
                                    // tools/plate_trace — one quantity,
                                    // two representations; the stroke on
                                    // the centerline also covers the
                                    // fill-abutment antialiasing seam.
                                    width: 1.9,
                                    pattern: map_types::style::StrokePattern::Solid,
                                },
                                sources,
                            });
                        }
                    }
                    Feature::Memory(m) => {
                        if let Some(view) = &q.viewport {
                            if m.at.angle_to(&view.center) > view.radius {
                                continue;
                            }
                        }
                        // an inscription at the traditional site: the
                        // memory voice, no marker — nothing "stands"
                        let sources = self.sources_of(fid);
                        scene.attribution.extend(sources.iter().cloned());
                        if q.layers.contains(LayerSet::LABELS) {
                            let mut label = style.label_style();
                            label.size *= 0.85;
                            scene.labels.push(PlacedLabel {
                                text: m.name.clone(),
                                at: m.at,
                                subject: LabelSubject::Place(map_types::AtlasPlaceRef(
                                    PlaceId::new(m.entity.0.clone()),
                                )),
                                style: label,
                                face: map_types::scene::LabelFace::Memory,
                                voice: style.labeling().memory,
                            });
                        }
                    }
                    Feature::Point(p) => {
                        if let Some(view) = &q.viewport {
                            if p.at.angle_to(&view.center) > view.radius {
                                continue;
                            }
                        }
                        let sources = self.sources_of(fid);
                        scene.attribution.extend(sources.iter().cloned());
                        scene.markers.push(StyledMarker {
                            at: p.at,
                            style: style.marker_style(),
                            sources,
                            place: Some(map_types::AtlasPlaceRef(PlaceId::new(p.entity.0.clone()))),
                        });
                        if q.layers.contains(LayerSet::LABELS) {
                            let mut label = style.label_style();
                            label.size *= 0.85; // a city is a note, not a shout
                            scene.labels.push(PlacedLabel {
                                text: p.name.clone(),
                                at: p.at,
                                subject: LabelSubject::Place(map_types::AtlasPlaceRef(
                                    PlaceId::new(p.entity.0.clone()),
                                )),
                                style: label,
                                face: map_types::scene::LabelFace::Place,
                                voice: style.labeling().place,
                            });
                        }
                    }
                }
            }
        }
        // Layer rank first (water above every claim — a lake is never
        // buried), then largest areas first within a rank: an empire
        // never buries its vassals.
        scene.regions.sort_by_cached_key(|r| {
            let pts: usize = r.outer.iter().map(|ring| ring.len()).sum();
            let radius = area_radius(r);
            let rank = paint_rank.get(&r.region).copied().unwrap_or(2);
            (rank, std::cmp::Reverse((radius * 1e9) as u64), pts, r.region.0 .0)
        });
        // Point subjects (gazetteer places) ride on top of the world.
        if let RenderSubject::Point(place) = &q.subject {
            let gaz = self
                .gazetteer
                .as_ref()
                .ok_or_else(|| MapError::UnknownPlace(place.0 .0.clone()))?;
            let entry = gaz
                .places
                .get(&place.0)
                .ok_or_else(|| MapError::UnknownPlace(place.0 .0.clone()))?;
            let sources = BTreeSet::from([SourceId::new(SCRIPTURE_SOURCE)]);
            scene.attribution.extend(sources.iter().cloned());
            scene.markers.push(StyledMarker {
                at: entry.position,
                style: style.marker_style(),
                sources,
                place: Some(map_types::AtlasPlaceRef(place.0.clone())),
            });
            if q.layers.contains(LayerSet::LABELS) {
                scene.labels.push(PlacedLabel {
                    text: entry.canonical_name.clone(),
                    at: entry.position,
                    subject: LabelSubject::Place(place.clone()),
                    style: style.label_style(),
                    face: map_types::scene::LabelFace::Place,
                    voice: style.labeling().place,
                });
            }
        }
        Ok(scene)
    }
}

impl CanonProvider {
    /// A COMPOSABLE PIECE: the same scene machinery filtered to the
    /// named entities — callers stack the resulting layers however
    /// they choose (the alignment law lives at the camera).
    pub fn render_pieces(
        &self,
        q: &RenderQuery,
        entities: &BTreeSet<EntityId>,
    ) -> Result<Snapshot, MapError> {
        let t = match &q.time {
            TimeSelector::At(t) => *t,
            TimeSelector::Over(i) => i.to.unwrap_or(i.from),
        };
        self.scene_at(&t, q, Some(entities))
    }

    /// The entities alive at `t`, with kind and witness — the listing
    /// callers pick pieces from.
    pub fn entities_at(&self, t: &Timestamp) -> Vec<(EntityId, String, &'static str, &'static str)> {
        let mut out = Vec::new();
        let mut seen = BTreeSet::new();
        for (layer, kind_name) in [
            (LayerKind::Territory, "territory"),
            (LayerKind::ScriptureClaims, "scripture-claim"),
            (LayerKind::Journeys, "journey"),
            (LayerKind::Water, "water"),
            (LayerKind::Relief, "relief"),
            (LayerKind::Background, "background"),
        ] {
            for (fid, f) in self.active(layer, t) {
                if !seen.insert(f.entity().clone()) {
                    continue;
                }
                let witness = self
                    .store
                    .provenance()
                    .get(&fid)
                    .map(|p| match p.witness {
                        Witness::Atlas => "atlas",
                        Witness::Authored => "authored",
                        Witness::Basemap => "basemap",
                        Witness::NaturalEarth => "natural-earth",
                    })
                    .unwrap_or("unknown");
                out.push((f.entity().clone(), f.name().to_string(), kind_name, witness));
            }
        }
        out
    }
}

impl MapProvider for CanonProvider {
    fn subjects(&self, at: TimePoint) -> Vec<SubjectListing> {
        let mut out = vec![SubjectListing {
            subject: RenderSubject::World,
            label: "the world".to_string(),
        }];
        let mut seen: BTreeSet<EntityId> = BTreeSet::new();
        for layer in [
            LayerKind::Territory,
            LayerKind::ScriptureClaims,
            LayerKind::Background,
            LayerKind::Water,
        ] {
            for (_, f) in self.active(layer, &at) {
                if seen.insert(f.entity().clone()) {
                    out.push(SubjectListing {
                        subject: RenderSubject::Region(rid_of(f.entity())),
                        label: f.name().to_string(),
                    });
                }
            }
        }
        // Stations of the ways alive now.
        let mut places: BTreeSet<atlas_graph_types::covenant::PlaceId> = BTreeSet::new();
        for (_, f) in self.active(LayerKind::Journeys, &at) {
            if let Feature::Way(r) = f {
                if let Some(first) = r.legs.first() {
                    places.insert(first.from.clone());
                }
                for leg in &r.legs {
                    places.insert(leg.to.clone());
                }
            }
        }
        if let Some(gaz) = self.gazetteer.as_ref() {
            for pid in places {
                if let Some(entry) = gaz.places.get(&pid) {
                    out.push(SubjectListing {
                        subject: RenderSubject::Point(map_types::AtlasPlaceRef(pid.clone())),
                        label: entry.canonical_name.clone(),
                    });
                }
            }
        }
        out
    }

    fn render(&self, q: &RenderQuery) -> Result<Snapshot, MapError> {
        match &q.time {
            TimeSelector::At(t) => self.scene_at(t, q, None),
            TimeSelector::Over(interval) => {
                // A range wears its END state in full, with every older
                // distinct area outline stroked in the age ramp beneath
                // (the range story as tinted lines, like the legacy
                // accumulation, simplified).
                let end = interval.to.unwrap_or(interval.from);
                let mut scene = self.scene_at(&end, q, None)?;
                let style = self.style(q.style)?;
                let ramp = style.age_ramp();
                // Distinct feature versions alive anywhere in the range.
                let mut seen: BTreeSet<FeatureId> = BTreeSet::new();
                for layer in layers_wanted(q.layers) {
                    let Some(world) = self.store.layers().get(&layer) else { continue };
                    let moments: Vec<Timestamp> = world
                        .moments()
                        .keys()
                        .filter(|t| **t >= interval.from && Some(**t) != interval.to)
                        .copied()
                        .chain([interval.from])
                        .collect();
                    let n = moments.len().max(1);
                    for (i, t) in moments.iter().enumerate() {
                        if interval.to.is_some_and(|to| *t > to) {
                            continue;
                        }
                        let toward = i as f64 / n as f64;
                        for (fid, f) in self.active(layer, t) {
                            if !seen.insert(fid) {
                                continue;
                            }
                            if let Feature::Area(a) = f {
                                let tint = mix(ramp.oldest, ramp.newest, toward);
                                for r in &a.rings {
                                    if let Some(ring) = self.ring_points(*r, q) {
                                        scene.boundaries.push(StyledBoundary {
                                            boundary: bid_of(&a.entity),
                                            pts: ring.points().to_vec(),
                                            stroke: map_types::style::Stroke {
                                                color: tint.fill,
                                                width: style
                                                    .stroke_for(&map_types::EdgeCharacter::Line)
                                                    .width,
                                                pattern: map_types::style::StrokePattern::Solid,
                                            },
                                            sources: self.sources_of(fid),
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
                Ok(scene)
            }
        }
    }

    fn transition(
        &self,
        from: TimePoint,
        to: TimePoint,
        viewport: Bbox,
        lod: Lod,
    ) -> Result<TransitionScript, MapError> {
        if from == to {
            return Ok(TransitionScript::empty()); // the identity (law 8)
        }
        if from > to {
            return Ok(invert(self.transition(to, from, viewport, lod)?));
        }
        let mut events: Vec<&ChangeEvent> = self.changes_between(from, to);
        events.sort_by_key(|e| e.at);
        let mut script = TransitionScript::empty();
        for e in events {
            match &e.kind {
                ChangeKind::Rise { region } => {
                    script.steps.push(TransitionStep::FadeIn { region: *region })
                }
                ChangeKind::Fall { region } => {
                    script.steps.push(TransitionStep::FadeOut { region: *region })
                }
                ChangeKind::Shift { boundary } => {
                    // a same-entity reshape MORPHS: slerp pairs with
                    // equal counts by resampling; if either side's
                    // geometry cannot be found, crossfade honestly
                    match self.shift_geometries(boundary, &e.at) {
                        Some((before, after)) => {
                            let a = map_types::simplify_polyline(&before, lod);
                            let b = map_types::simplify_polyline(&after, lod);
                            let n = a.len().max(b.len()).max(2);
                            script.steps.push(TransitionStep::Morph {
                                boundary: *boundary,
                                from_pts: resample(&a, n),
                                to_pts: resample(&b, n),
                            });
                        }
                        None => {
                            if let Some(ent) = self.entity_by_bid.get(boundary) {
                                let rid = rid_of(ent);
                                script.steps.push(TransitionStep::FadeOut { region: rid });
                                script.steps.push(TransitionStep::FadeIn { region: rid });
                            }
                        }
                    }
                }
                // a journey's progress is time-parameterized rendering,
                // not a topology change
                ChangeKind::Journey { .. } => {}
                ChangeKind::Rename { .. } => {}
                ChangeKind::Split { parent, children, seam } => {
                    script.steps.push(TransitionStep::SplitAlong {
                        parent: *parent,
                        seam: seam.clone(),
                        children: children.clone(),
                    })
                }
                ChangeKind::Merge { parents, child } => {
                    script.steps.push(TransitionStep::MergeAcross {
                        parents: parents.clone(),
                        child: *child,
                    })
                }
            }
        }
        Ok(script)
    }

    fn changes_between(&self, from: TimePoint, to: TimePoint) -> Vec<&ChangeEvent> {
        let (lo, hi) = if from <= to { (from, to) } else { (to, from) };
        self.events.iter().filter(|e| e.at > lo && e.at <= hi).collect()
    }
}

// ------------------------------------------------------ pure helpers

/// Equal-count resampling along a polyline (slerp between attested
/// points) so a morph pairs vertices one-to-one.
fn resample(pts: &[UnitVec], n: usize) -> Vec<UnitVec> {
    if pts.len() < 2 || n < 2 {
        return pts.to_vec();
    }
    let mut cumulative = vec![0.0];
    for w in pts.windows(2) {
        cumulative.push(cumulative.last().unwrap() + w[0].angle_to(&w[1]));
    }
    let total = *cumulative.last().unwrap();
    if total <= 0.0 {
        return vec![pts[0]; n];
    }
    let mut out = Vec::with_capacity(n);
    let mut seg = 0usize;
    for i in 0..n {
        let target = total * (i as f64) / ((n - 1) as f64);
        while seg + 2 < cumulative.len() && cumulative[seg + 1] < target {
            seg += 1;
        }
        let span = cumulative[seg + 1] - cumulative[seg];
        let t = if span > 0.0 { (target - cumulative[seg]) / span } else { 0.0 };
        let p = map_types::slerp(&pts[seg], &pts[seg + 1], t.clamp(0.0, 1.0)).unwrap_or(pts[seg]);
        out.push(p);
    }
    out
}

/// Play a script backwards: fades swap, splits merge, morphs reverse.
fn invert(script: TransitionScript) -> TransitionScript {
    let steps = script
        .steps
        .into_iter()
        .rev()
        .map(|s| match s {
            TransitionStep::Morph { boundary, from_pts, to_pts } => {
                TransitionStep::Morph { boundary, from_pts: to_pts, to_pts: from_pts }
            }
            TransitionStep::FadeIn { region } => TransitionStep::FadeOut { region },
            TransitionStep::FadeOut { region } => TransitionStep::FadeIn { region },
            TransitionStep::SplitAlong { parent, children, .. } => {
                TransitionStep::MergeAcross { parents: children, child: parent }
            }
            TransitionStep::MergeAcross { parents, child } => TransitionStep::SplitAlong {
                parent: child,
                seam: Vec::new(),
                children: parents,
            },
        })
        .collect();
    TransitionScript { steps }
}


/// Palette slots by GRAPH COLORING over the shared-border graph,
/// computed once per canon: two areas that share a border stretch
/// (two or more identical ring vertices — spliced borders carry the
/// same points on both sides) never wear the same slot. Welsh-Powell
/// order (highest degree first, then id) keeps the greedy pass
/// honest; if all eight slots are worn nearby, the least-worn wins
/// deterministically. Water and relief never enter — they wear their
/// own paint.
pub(crate) fn palette_slots(store: &CanonStore) -> BTreeMap<EntityId, usize> {
    use map_canon::{Feature, LayerKind};
    // features living in palette-wearing layers
    let mut wearers: BTreeSet<map_canon::FeatureId> = BTreeSet::new();
    for (layer, world) in store.layers() {
        if matches!(layer, LayerKind::Water | LayerKind::Relief) {
            continue;
        }
        for sid in world.moments().values() {
            if let Some(snap) = store.snapshots().get(sid) {
                wearers.extend(snap.features.iter().copied());
            }
        }
    }
    // entity -> quantized ring-vertex keys (quantization only guards
    // serialization round-trips; shared borders are identical points)
    let key = |p: &UnitVec| -> (i64, i64, i64) {
        (
            (p.x() * 1e7).round() as i64,
            (p.y() * 1e7).round() as i64,
            (p.z() * 1e7).round() as i64,
        )
    };
    let mut keys_of: BTreeMap<EntityId, BTreeSet<(i64, i64, i64)>> = BTreeMap::new();
    for fid in &wearers {
        let Some(Feature::Area(a)) = store.features().get(fid) else { continue };
        let e = keys_of.entry(a.entity.clone()).or_default();
        for bid in a.rings.iter().chain(a.holes.iter()) {
            if let Some(b) = store.borders().get(bid) {
                for p in &b.0 {
                    e.insert(key(p));
                }
            }
        }
    }
    // adjacency: entities sharing >= 2 vertex keys
    let ids: Vec<EntityId> = keys_of.keys().cloned().collect();
    let mut at_vertex: BTreeMap<(i64, i64, i64), Vec<usize>> = BTreeMap::new();
    for (i, id) in ids.iter().enumerate() {
        for k in &keys_of[id] {
            at_vertex.entry(*k).or_default().push(i);
        }
    }
    let mut shared: BTreeMap<(usize, usize), usize> = BTreeMap::new();
    for owners in at_vertex.values() {
        for x in 0..owners.len() {
            for y in x + 1..owners.len() {
                *shared.entry((owners[x], owners[y])).or_insert(0) += 1;
            }
        }
    }
    let mut adj: BTreeMap<EntityId, BTreeSet<EntityId>> = BTreeMap::new();
    for (&(x, y), &n) in &shared {
        if n >= 2 {
            adj.entry(ids[x].clone()).or_default().insert(ids[y].clone());
            adj.entry(ids[y].clone()).or_default().insert(ids[x].clone());
        }
    }
    color_shared_border_graph(&ids, &adj)
}

/// Label anchors, computed once per canon: every Area feature's pole
/// of inaccessibility. Geometry is content-addressed, so the anchor
/// can never go stale — a changed ring is a new feature id.
fn label_anchors(store: &CanonStore) -> BTreeMap<FeatureId, UnitVec> {
    let mut out = BTreeMap::new();
    for (fid, f) in store.features() {
        let Feature::Area(a) = f else { continue };
        // a whole-sphere region (the sentinel convention) has no
        // meaningful pole — the world ocean's would be Point Nemo —
        // and searching the planet for it is pure startup cost; it
        // falls back to the centroid like any anchorless area
        let is_sentinel = a
            .rings
            .iter()
            .any(|bid| store.borders().get(bid).is_some_and(|b| map_types::covers_sphere(&b.0)));
        if is_sentinel {
            continue;
        }
        let resolve = |ids: &BTreeSet<map_canon::BorderId>| -> Vec<Ring> {
            ids.iter()
                .filter_map(|bid| store.borders().get(bid))
                .filter_map(|b| Ring::new(b.0.clone()).ok())
                .collect()
        };
        let outer = resolve(&a.rings);
        let holes = resolve(&a.holes);
        let t0 = std::time::Instant::now();
        if let Some(at) = pole_of_inaccessibility(&outer, &holes) {
            out.insert(*fid, at);
        }
        let dt = t0.elapsed().as_secs_f64();
        if dt > 0.3 {
            eprintln!(
                "    SLOW anchor {:.1}s: {} rings={} holes={} pts={}",
                dt,
                a.entity.0,
                outer.len(),
                holes.len(),
                outer.iter().chain(holes.iter()).map(Ring::len).sum::<usize>()
            );
        }
    }
    out
}

/// Each border's spherical cap, once per canon: centroid direction
/// and the farthest point's angle from it.
fn border_caps(store: &CanonStore) -> BTreeMap<map_canon::BorderId, (UnitVec, f64)> {
    let mut out = BTreeMap::new();
    for (bid, b) in store.borders() {
        if map_types::covers_sphere(&b.0) {
            // the sentinel's interior is everything: no viewport may
            // cull it, whatever its stored points say
            out.insert(*bid, (b.0[0], std::f64::consts::PI));
            continue;
        }
        let (mut x, mut y, mut z) = (0.0, 0.0, 0.0);
        for p in &b.0 {
            x += p.x();
            y += p.y();
            z += p.z();
        }
        let Ok(c) = UnitVec::normalize(x, y, z) else {
            // a degenerate centroid (ring straddling the sphere):
            // never cull it
            out.insert(*bid, (b.0[0], std::f64::consts::PI));
            continue;
        };
        let r = b.0.iter().map(|p| c.angle_to(p)).fold(0.0, f64::max);
        out.insert(*bid, (c, r));
    }
    out
}

/// Relief band order by MEASUREMENT: each Relief-layer entity's total
/// ring area (planar shoelace suffices — bands nest), largest first.
/// Position 0 = the lowest, widest band; 1 = the highest peak band.
fn relief_positions(store: &CanonStore) -> BTreeMap<EntityId, f64> {
    use map_canon::{Feature, LayerKind};
    let mut area_of: BTreeMap<EntityId, f64> = BTreeMap::new();
    let Some(world) = store.layers().get(&LayerKind::Relief) else { return BTreeMap::new() };
    for sid in world.moments().values() {
        let Some(snap) = store.snapshots().get(sid) else { continue };
        for fid in &snap.features {
            let Some(Feature::Area(a)) = store.features().get(fid) else { continue };
            let e = area_of.entry(a.entity.clone()).or_insert(0.0);
            *e = 0.0; // recompute per latest snapshot mention
            for bid in &a.rings {
                if let Some(b) = store.borders().get(bid) {
                    let ll: Vec<(f64, f64)> = b.0.iter().map(|p| p.to_lat_lon_deg()).collect();
                    let mut s = 0.0;
                    for i in 0..ll.len() {
                        let (la1, lo1) = ll[i];
                        let (la2, lo2) = ll[(i + 1) % ll.len()];
                        s += lo1 * la2 - lo2 * la1;
                    }
                    *e += (s / 2.0).abs();
                }
            }
        }
    }
    let mut order: Vec<(EntityId, f64)> = area_of.into_iter().collect();
    order.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    let n = order.len().max(2) as f64 - 1.0;
    order
        .into_iter()
        .enumerate()
        .map(|(i, (e, _))| (e, i as f64 / n))
        .collect()
}

/// The greedy pass, separated so the law can test it directly.
pub(crate) fn color_shared_border_graph(
    ids: &[EntityId],
    adj: &BTreeMap<EntityId, BTreeSet<EntityId>>,
) -> BTreeMap<EntityId, usize> {
    let mut order: Vec<&EntityId> = ids.iter().collect();
    order.sort_by_key(|id| {
        (std::cmp::Reverse(adj.get(*id).map_or(0, BTreeSet::len)), (*id).clone())
    });
    let mut slot: BTreeMap<EntityId, usize> = BTreeMap::new();
    let mut used = [0usize; 8]; // global wear, so all eight slots serve
    for id in order {
        let worn: Vec<usize> = adj
            .get(id)
            .into_iter()
            .flatten()
            .filter_map(|n| slot.get(n).copied())
            .collect();
        // among slots no neighbor wears, the least worn globally —
        // the palette spreads instead of leaning on its first colors;
        // if neighbors wear all eight, the least worn among them
        let free = (0..8usize).filter(|s| !worn.contains(s));
        let pick = free.min_by_key(|&s| (used[s], s)).unwrap_or_else(|| {
            let mut counts = [0usize; 8];
            for &u in &worn {
                counts[u] += 1;
            }
            (0..8).min_by_key(|&s| (counts[s], s)).expect("eight slots exist")
        });
        used[pick] += 1;
        slot.insert(id.clone(), pick);
    }
    slot
}

fn mix(a: Paint, b: Paint, t: f64) -> Paint {
    let c = |x: u8, y: u8| -> u8 {
        (f64::from(x) + (f64::from(y) - f64::from(x)) * t).round().clamp(0.0, 255.0) as u8
    };
    let (map_types::style::Rgba(ar, ag, ab, aa), map_types::style::Rgba(br, bg, bb, ba)) =
        (a.fill, b.fill);
    Paint { fill: map_types::style::Rgba(c(ar, br), c(ag, bg), c(ab, bb), c(aa, ba)) }
}

/// Planar shoelace area of a ring in steradians (lon/lat tangent
/// approximation -- regions here are small; used only to scale labels).
fn ring_area_sr(ring: &Ring) -> f64 {
    let pts = ring.points();
    if pts.len() < 3 {
        return 0.0;
    }
    let ll: Vec<(f64, f64)> = pts.iter().map(|p| p.to_lat_lon_deg()).collect();
    let latc = ll.iter().map(|(la, _)| la).sum::<f64>() / ll.len() as f64;
    let k = latc.to_radians().cos();
    let mut a = 0.0;
    for i in 0..ll.len() {
        let (la1, lo1) = ll[i];
        let (la2, lo2) = ll[(i + 1) % ll.len()];
        a += (lo1 * k) * la2 - (lo2 * k) * la1;
    }
    (a / 2.0) * (std::f64::consts::PI / 180.0).powi(2)
}

/// The POLE OF INACCESSIBILITY of the largest outer ring: the interior
/// point farthest from any border (holes included) -- a label anchored
/// here is inside its region by construction, never straddling an
/// edge. Deterministic grid search with local refinement, in a local
/// lon/lat tangent frame.
fn pole_of_inaccessibility(outer: &[Ring], holes: &[Ring]) -> Option<UnitVec> {
    // areas computed ONCE per ring: max_by re-evaluates its key per
    // comparison, which re-measured a 150k-point continent thousands
    // of times (20 s of it, measured)
    let big = outer
        .iter()
        .map(|r| (ring_area_sr(r).abs(), r))
        .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(_, r)| r)?;
    // a label anchor needs the ring's shape, not its every vertex:
    // stride-decimate huge rings so the search stays cheap (the
    // worldwide coastline has ~1e5 points; 512 keep its pose)
    let pts_all = big.points();
    let stride = (pts_all.len() / 512).max(1);
    let ring: Vec<(f64, f64)> =
        pts_all.iter().step_by(stride).map(|p| p.to_lat_lon_deg()).collect();
    if ring.len() < 3 {
        return None;
    }
    let latc = ring.iter().map(|(la, _)| la).sum::<f64>() / ring.len() as f64;
    let k = latc.to_radians().cos().max(0.05);
    // x = lon * k, y = lat
    let poly: Vec<(f64, f64)> = ring.iter().map(|&(la, lo)| (lo * k, la)).collect();
    let hole_polys: Vec<Vec<(f64, f64)>> = holes
        .iter()
        .map(|h| {
            let pts = h.points();
            let stride = (pts.len() / 256).max(1);
            pts.iter()
                .step_by(stride)
                .map(|p| {
                    let (la, lo) = p.to_lat_lon_deg();
                    (lo * k, la)
                })
                .collect()
        })
        .collect();
    fn inside(p: (f64, f64), poly: &[(f64, f64)]) -> bool {
        let (x, y) = p;
        let mut in_ = false;
        let n = poly.len();
        let mut j = n - 1;
        for i in 0..n {
            let (xi, yi) = poly[i];
            let (xj, yj) = poly[j];
            if (yi > y) != (yj > y) && x < (xj - xi) * (y - yi) / (yj - yi) + xi {
                in_ = !in_;
            }
            j = i;
        }
        in_
    }
    fn seg_dist(p: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
        let (px, py) = p;
        let (ax, ay) = a;
        let (bx, by) = b;
        let (dx, dy) = (bx - ax, by - ay);
        let l2 = dx * dx + dy * dy;
        let t = if l2 <= 0.0 { 0.0 } else { ((px - ax) * dx + (py - ay) * dy) / l2 };
        let t = t.clamp(0.0, 1.0);
        let (qx, qy) = (ax + t * dx, ay + t * dy);
        ((px - qx).powi(2) + (py - qy).powi(2)).sqrt()
    }
    let clearance = |p: (f64, f64)| -> f64 {
        if !inside(p, &poly) || hole_polys.iter().any(|h| inside(p, h)) {
            return f64::NEG_INFINITY;
        }
        let mut d = f64::INFINITY;
        for ring in std::iter::once(&poly).chain(hole_polys.iter()) {
            let n = ring.len();
            for i in 0..n {
                d = d.min(seg_dist(p, ring[i], ring[(i + 1) % n]));
            }
        }
        d
    };
    let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for &(x, y) in &poly {
        x0 = x0.min(x);
        y0 = y0.min(y);
        x1 = x1.max(x);
        y1 = y1.max(y);
    }
    const N: usize = 24;
    let mut best = (f64::NEG_INFINITY, (0.0, 0.0));
    for i in 0..N {
        for j in 0..N {
            let p = (
                x0 + (x1 - x0) * (i as f64 + 0.5) / N as f64,
                y0 + (y1 - y0) * (j as f64 + 0.5) / N as f64,
            );
            let c = clearance(p);
            if c > best.0 {
                best = (c, p);
            }
        }
    }
    if !best.0.is_finite() {
        return None;
    }
    let mut step = ((x1 - x0) / N as f64).max((y1 - y0) / N as f64);
    for _ in 0..5 {
        step /= 2.0;
        let center = best.1;
        for di in -2i32..=2 {
            for dj in -2i32..=2 {
                let p = (center.0 + f64::from(di) * step, center.1 + f64::from(dj) * step);
                let c = clearance(p);
                if c > best.0 {
                    best = (c, p);
                }
            }
        }
    }
    let (x, y) = best.1;
    Some(UnitVec::from_lat_lon_deg(y, x / k))
}

fn centroid(rings: &[Ring]) -> Option<UnitVec> {
    let pts = rings.first()?.points();
    let (mut x, mut y, mut z) = (0.0, 0.0, 0.0);
    for p in pts {
        x += p.x();
        y += p.y();
        z += p.z();
    }
    UnitVec::normalize(x, y, z).ok()
}

fn area_radius(r: &StyledRegion) -> f64 {
    let pts: Vec<&UnitVec> = r.outer.iter().flat_map(|ring| ring.points()).collect();
    let (mut x, mut y, mut z) = (0.0, 0.0, 0.0);
    for p in &pts {
        x += p.x();
        y += p.y();
        z += p.z();
    }
    let Ok(c) = UnitVec::normalize(x, y, z) else { return 0.0 };
    pts.iter().map(|p| c.angle_to(p)).fold(0.0, f64::max)
}

fn walked_prefix(pts: &[UnitVec], f: f64) -> Vec<UnitVec> {
    if f >= 1.0 || pts.len() < 2 {
        return pts.to_vec();
    }
    let mut cum = vec![0.0f64];
    for w in pts.windows(2) {
        cum.push(cum.last().unwrap() + w[0].angle_to(&w[1]));
    }
    let total = *cum.last().unwrap();
    if total <= 0.0 {
        return pts.to_vec();
    }
    let cut = total * f;
    let mut out = vec![pts[0]];
    for i in 1..pts.len() {
        if cum[i] <= cut {
            out.push(pts[i]);
            continue;
        }
        let seg = cum[i] - cum[i - 1];
        let t = if seg > 0.0 { (cut - cum[i - 1]) / seg } else { 0.0 };
        if t > 1e-9 {
            if let Ok(mid) = slerp(&pts[i - 1], &pts[i], t) {
                out.push(mid);
            }
        }
        break;
    }
    out
}

/// Derive the scrubber's events from moment diffs, per layer: an
/// entity appearing is a Rise (a way, a Journey), a vanishing one a
/// Fall, a changed body a Shift.
fn derive_events(store: &CanonStore) -> Vec<ChangeEvent> {
    use atlas_graph_types::covenant::Justification;
    let mut out = Vec::new();
    for (layer, world) in store.layers() {
        let mut prev: BTreeMap<EntityId, FeatureId> = BTreeMap::new();
        for (t, sid) in world.moments() {
            let mut now: BTreeMap<EntityId, FeatureId> = BTreeMap::new();
            if let Some(snap) = store.snapshots().get(sid) {
                for fid in &snap.features {
                    if let Some(f) = store.features().get(fid) {
                        now.insert(f.entity().clone(), *fid);
                    }
                }
            }
            for (ent, fid) in &now {
                let is_way = matches!(store.features().get(fid), Some(Feature::Way(_)));
                match prev.get(ent) {
                    None => out.push(ChangeEvent {
                        at: *t,
                        kind: if is_way {
                            ChangeKind::Journey { boundary: bid_of(ent) }
                        } else {
                            ChangeKind::Rise { region: rid_of(ent) }
                        },
                        driver: None,
                        justification: Justification::default(),
                        provenance: format!("canon:{layer:?}"),
                    }),
                    Some(old) if old != fid => out.push(ChangeEvent {
                        at: *t,
                        kind: ChangeKind::Shift { boundary: bid_of(ent) },
                        driver: None,
                        justification: Justification::default(),
                        provenance: format!("canon:{layer:?}"),
                    }),
                    _ => {}
                }
            }
            for ent in prev.keys() {
                if !now.contains_key(ent) {
                    out.push(ChangeEvent {
                        at: *t,
                        kind: ChangeKind::Fall { region: rid_of(ent) },
                        driver: None,
                        justification: Justification::default(),
                        provenance: format!("canon:{layer:?}"),
                    });
                }
            }
            prev = now;
        }
    }
    out.sort_by_key(|e| e.at);
    out
}

