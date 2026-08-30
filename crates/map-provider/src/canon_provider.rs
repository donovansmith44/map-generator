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

use atlas_graph_types::covenant::{ContentHash, SourceId, TimePoint};
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
    events: Vec<ChangeEvent>,
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
        for f in store.features().values() {
            entity_by_rid.insert(rid_of(f.entity()), f.entity().clone());
        }
        let events = derive_events(&store);
        CanonProvider { store, styles, gazetteer, entity_by_rid, events }
    }

    pub fn from_canon_file(
        path: &std::path::Path,
        styles: BTreeMap<StyleId, Style>,
        gazetteer: Option<GazetteerExport>,
    ) -> Result<Self, String> {
        let bytes = std::fs::read(path).map_err(|e| format!("canon: {e}"))?;
        let store = map_canon::persist::from_bytes(&bytes)?;
        Ok(Self::new(store, styles, gazetteer))
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

    fn ring_points(&self, id: map_canon::BorderId) -> Option<Ring> {
        let b = self.store.borders().get(&id)?;
        Ring::new(b.0.clone()).ok()
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
                // Bands mix along the topo ramp by a stable per-entity
                // position (band identity survives the bridge as the
                // entity slug).
                let ramp = style.topo_ramp();
                let t = (hash64(&entity.0) % 1000) as f64 / 1000.0;
                mix(ramp.oldest, ramp.newest, t)
            }
            _ => match style.palette() {
                Some(slots) => slots[(hash64(&entity.0) % 8) as usize],
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
            if let Some(ring) = self.ring_points(*r) {
                outer.push(ring);
            }
        }
        for h in &a.holes {
            if let Some(ring) = self.ring_points(*h) {
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
            if let Some(at) = pole_of_inaccessibility(&outer, &holes).or_else(|| centroid(&outer)) {
                let is_water = layer == LayerKind::Water;
                let mut label = style.label_style();
                // the label grows with the ground it names: sqrt of
                // the largest ring's area, clamped to stay readable
                let sr = outer.iter().map(|r| ring_area_sr(r).abs()).fold(0.0, f64::max);
                label.size *= (sr.sqrt() / 0.006).clamp(0.85, 2.1);
                if is_water {
                    // water speaks in its own deep color
                    let map_types::style::Rgba(r, g, b, _) =
                        self.area_paint(layer, &a.entity, style).fill;
                    let dim = |v: u8| (f64::from(v) * 0.45) as u8;
                    label.color = map_types::style::Rgba(dim(r), dim(g), dim(b), 255);
                    label.size *= 0.8;
                }
                scene.labels.push(PlacedLabel {
                    text: a.name.clone(),
                    at,
                    subject: LabelSubject::Region(rid_of(&a.entity)),
                    style: label,
                    face: if is_water {
                        map_types::scene::LabelFace::Water
                    } else {
                        map_types::scene::LabelFace::Territory
                    },
                });
            }
        }
        scene.attribution.extend(sources.iter().cloned());
        scene.regions.push(StyledRegion {
            region: rid_of(&a.entity),
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
                LayerKind::Territory => 2,
                LayerKind::ScriptureClaims => 3,
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
                        // path and may be as short as two points.
                        if let Some(pts) =
                            self.store.borders().get(&l.border).map(|b| b.0.clone())
                        {
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
                    Feature::Point(p) => {
                        let sources = self.sources_of(fid);
                        scene.attribution.extend(sources.iter().cloned());
                        scene.markers.push(StyledMarker {
                            at: p.at,
                            style: style.marker_style(),
                            sources,
                            place: None,
                        });
                    }
                }
            }
        }
        // Neighbor-aware palette: greedy slot assignment so touching
        // areas never share a color when a free slot exists (stable:
        // deterministic order, hash-seeded start).
        recolor_adjacent(&mut scene, self.style(q.style)?);
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
                                    if let Some(ring) = self.ring_points(*r) {
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
        _viewport: Bbox,
        _lod: Lod,
    ) -> Result<TransitionScript, MapError> {
        if from == to {
            return Ok(TransitionScript::empty());
        }
        let q = |t: TimePoint| RenderQuery {
            subject: RenderSubject::World,
            time: TimeSelector::At(t),
            viewport: None,
            lod: Lod::exact(),
            layers: LayerSet::GEOMETRY,
            style: *self.styles.keys().next().expect("a style"),
        };
        let a = self.render(&q(from))?;
        let b = self.render(&q(to))?;
        let ra: BTreeSet<RegionId> = a.regions.iter().map(|r| r.region).collect();
        let rb: BTreeSet<RegionId> = b.regions.iter().map(|r| r.region).collect();
        let mut script = TransitionScript::empty();
        for gone in ra.difference(&rb) {
            script.steps.push(TransitionStep::FadeOut { region: *gone });
        }
        for come in rb.difference(&ra) {
            script.steps.push(TransitionStep::FadeIn { region: *come });
        }
        Ok(script)
    }

    fn changes_between(&self, from: TimePoint, to: TimePoint) -> Vec<&ChangeEvent> {
        let (lo, hi) = if from <= to { (from, to) } else { (to, from) };
        self.events.iter().filter(|e| e.at > lo && e.at <= hi).collect()
    }
}

// ------------------------------------------------------ pure helpers

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
    let big = outer.iter().max_by(|a, b| {
        ring_area_sr(a)
            .abs()
            .partial_cmp(&ring_area_sr(b).abs())
            .unwrap_or(std::cmp::Ordering::Equal)
    })?;
    let ring: Vec<(f64, f64)> = big.points().iter().map(|p| p.to_lat_lon_deg()).collect();
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
            h.points()
                .iter()
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

/// Greedy adjacent-color avoidance over the scene's palette-painted
/// areas: keep each area's hash-seeded slot unless a bbox-overlapping,
/// already-colored area wears it — then take the next free slot.
fn recolor_adjacent(scene: &mut Snapshot, style: &Style) {
    let Some(slots) = style.palette() else { return };
    let slot_of = |p: &Paint| slots.iter().position(|s| s == p);
    let bbox = |r: &StyledRegion| -> (f64, f64, f64, f64) {
        let (mut a, mut b, mut c, mut d) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
        for ring in &r.outer {
            for p in ring.points() {
                let lat = p.z().asin();
                let lon = p.y().atan2(p.x());
                a = a.min(lat);
                b = b.min(lon);
                c = c.max(lat);
                d = d.max(lon);
            }
        }
        (a, b, c, d)
    };
    let overlaps = |x: &(f64, f64, f64, f64), y: &(f64, f64, f64, f64)| {
        x.0 <= y.2 && y.0 <= x.2 && x.1 <= y.3 && y.1 <= x.3
    };
    let mut placed: Vec<((f64, f64, f64, f64), usize)> = Vec::new();
    for r in scene.regions.iter_mut() {
        let Some(seed) = slot_of(&r.paint) else { continue };
        let b = bbox(r);
        let used: BTreeSet<usize> = placed
            .iter()
            .filter(|(pb, _)| overlaps(pb, &b))
            .map(|(_, s)| *s)
            .collect();
        let mut pick = seed;
        for step in 0..slots.len() {
            let cand = (seed + step) % slots.len();
            if !used.contains(&cand) {
                pick = cand;
                break;
            }
        }
        r.paint = slots[pick];
        placed.push((b, pick));
    }
}
