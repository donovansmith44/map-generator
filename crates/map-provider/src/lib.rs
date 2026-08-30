//! The reference MapProvider — phase 3 of docs/map-system-handoff.md.
//!
//! A snapshot at time t is a DERIVED, deterministic query result, never
//! a hand-maintained artifact: materialization = select ∘ simplify ∘
//! style, each stage a total pure function of the timeline (covenant
//! rules 6 and 10). "Tons of snapshots" = tons of queries, cached by
//! content hash — determinism (law 1) is what makes the cache and the
//! offline story sound.

use std::collections::{BTreeMap, BTreeSet};

use atlas_graph_types::covenant::TimePoint;
use atlas_graph_types::covenant::SourceId;

use map_types::scene::{LabelSubject, StyledMarker};
use map_types::style::{Paint, Rgba};
use map_types::{
    slerp, Bbox, Boundary, BoundaryId, BoundarySource, ChangeEvent, ChangeKind, GazetteerExport,
    Interval, Lod, MapAddressed, MapError, MapProvider, Orientation, PlacedLabel, RegionId,
    RegionPart, RenderQuery, RenderSubject, Ring, Snapshot, Style, StyleId, StyledBoundary,
    StyledRegion, SubjectListing, TimeSelector, TransitionScript, TransitionStep, UnitVec,
    WorldTimeline,
};
use map_types::{accumulate, sample_times, simplify_polyline, LayerSet, Monoid};

/// The LEGACY reference provider over the interval timeline. As of the
/// 2026-08-27 canon design (phase 6) it serves NOTHING — the workbench
/// and CLI run on `canon_provider::CanonProvider` only. It remains as
/// the reference implementation of morph transitions and range
/// accumulation until the canon provider gains morphs, then it goes.
pub struct TimelineProvider {
    pub timeline: WorldTimeline,
    pub styles: BTreeMap<StyleId, Style>,
    pub gazetteer: Option<GazetteerExport>,
}

// ------------------------------------------------------- small pure fns

fn mix_channel(a: u8, b: u8, t: f64) -> u8 {
    (f64::from(a) + (f64::from(b) - f64::from(a)) * t).round().clamp(0.0, 255.0) as u8
}

/// Linear paint mix: t = 0 gives `a`, t = 1 gives `b`.
fn mix_paint(a: Paint, b: Paint, t: f64) -> Paint {
    let (Rgba(ar, ag, ab, aa), Rgba(br, bg, bb, ba)) = (a.fill, b.fill);
    Paint {
        fill: Rgba(
            mix_channel(ar, br, t),
            mix_channel(ag, bg, t),
            mix_channel(ab, bb, t),
            mix_channel(aa, ba, t),
        ),
    }
}

/// Resample an open polyline to exactly `n` points, evenly spaced by
/// arc length, endpoints kept — the precondition of a lawful Morph
/// (equal counts; law 4).
pub fn resample(pts: &[UnitVec], n: usize) -> Vec<UnitVec> {
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
        let p = slerp(&pts[seg], &pts[seg + 1], t.clamp(0.0, 1.0)).unwrap_or(pts[seg]);
        out.push(p);
    }
    out
}

/// The source id Scripture-surveyed geometry carries into scenes: the
/// authority ladder (covenant rule 11) made visible in attribution and
/// selectable by consumers (a "Bible mode" is a semantic filter on it).
pub const SCRIPTURE_SOURCE: &str = "scripture";

fn boundary_sources(b: &Boundary) -> BTreeSet<SourceId> {
    match &b.source {
        BoundarySource::Imported { source } => BTreeSet::from([source.clone()]),
        BoundarySource::Survey(_) => BTreeSet::from([SourceId::new(SCRIPTURE_SOURCE)]),
        BoundarySource::Authored { .. } => BTreeSet::new(),
    }
}

fn in_viewport(viewport: &Option<Bbox>, pts: &[UnitVec]) -> bool {
    match viewport {
        None => true,
        Some(v) => pts.iter().any(|p| v.contains(p)),
    }
}

/// Linear year index that skips the missing year zero.
fn year_index(y: i32) -> i32 {
    if y > 0 {
        y - 1
    } else {
        y
    }
}

/// How much of a Way's span has elapsed by `t`, at year grain,
/// inclusive: the departure year walks the first 1/n of the road, the
/// arrival year completes it. An open interval is a completed walk.
fn way_fraction(interval: &Interval, t: &TimePoint) -> f64 {
    let from = year_index(interval.from.year.get());
    let Some(to) = interval.to else { return 1.0 };
    let arrival = year_index(to.year.get()) - 1; // half-open end
    let duration = f64::from((arrival - from + 1).max(1));
    let elapsed = f64::from((year_index(t.year.get()) - from + 1).max(0));
    (elapsed / duration).clamp(0.0, 1.0)
}

/// The road walked so far: the polyline truncated at the arc-length
/// fraction, ending mid-leg on an interpolated point — the walk is
/// between stations, not teleporting station to station.
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

/// A region's angular size — max angle from its centroid to any vertex.
fn angular_radius(r: &StyledRegion) -> f64 {
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

/// Paint order is clickability and visibility: the largest regions go
/// down first, so an empire never buries the vassals inside it.
fn sort_largest_first(regions: &mut [StyledRegion]) {
    regions.sort_by_cached_key(|r| {
        (std::cmp::Reverse((angular_radius(r) * 1e9) as u64), r.region.0 .0)
    });
}

// ------------------------------------------------------ the materializer

impl TimelineProvider {
    fn style(&self, id: StyleId) -> Result<&Style, MapError> {
        self.styles.get(&id).ok_or(MapError::UnknownStyle(id))
    }

    fn boundary_at(&self, id: BoundaryId, at: &TimePoint) -> Result<&Boundary, MapError> {
        let hist =
            self.timeline.boundaries.get(&id).ok_or(MapError::UnknownBoundary(id))?;
        hist.at(at).ok_or(MapError::NothingAtTime(*at))
    }

    /// The version alive at `at`, with its own interval — the walk's
    /// span is what a Way's partial rendering is computed against.
    fn version_at(&self, id: BoundaryId, at: &TimePoint) -> Option<(&Interval, &Boundary)> {
        self.timeline
            .boundaries
            .get(&id)?
            .versions
            .iter()
            .find(|(iv, _)| iv.contains(at))
            .map(|(iv, b)| (iv, b))
    }

    /// Resolve one part's cycle to a ring: oriented simplified arcs,
    /// each dropping its trailing junction. If simplification would
    /// collapse the ring below three points, the unsimplified geometry
    /// stands — lod never changes topology (law 7's second clause).
    /// An EMPTY cycle is the whole sphere (see RegionPart): it resolves
    /// to a sentinel ring spanning near-antipodal points, which
    /// projections render as everything in view.
    fn resolve_ring(
        &self,
        cycle: &[(BoundaryId, Orientation)],
        at: &TimePoint,
        lod: Lod,
    ) -> Result<(Ring, BTreeSet<SourceId>), MapError> {
        if cycle.is_empty() {
            let sphere = Ring::new(vec![
                UnitVec::from_lat_lon_deg(0.0, 0.0),
                UnitVec::from_lat_lon_deg(0.1, 179.95),
                UnitVec::from_lat_lon_deg(5.0, 90.0),
            ])
            .expect("sentinel ring is well-formed");
            return Ok((sphere, BTreeSet::new()));
        }
        let mut sources = BTreeSet::new();
        let build = |lod: Lod, sources: &mut BTreeSet<SourceId>| -> Result<Vec<UnitVec>, MapError> {
            let mut ring: Vec<UnitVec> = Vec::new();
            for (bid, orientation) in cycle {
                let b = self.boundary_at(*bid, at)?;
                sources.extend(boundary_sources(b));
                let pts = simplify_polyline(&b.pts, lod);
                match orientation {
                    Orientation::Forward => ring.extend_from_slice(&pts[..pts.len() - 1]),
                    Orientation::Reverse => ring.extend(pts[1..].iter().rev()),
                }
            }
            Ok(ring)
        };
        let simplified = build(lod, &mut sources)?;
        let pts = if simplified.len() >= 3 {
            simplified
        } else {
            // Collapse fallback: the feature is smaller than the
            // tolerance, so keep a MINIMAL ring — four evenly spaced
            // real vertices. Returning the exact geometry here would
            // invert law 7 (coarser lod growing the scene: with
            // fine-grained sources, thousands of collapsed islands
            // each re-inflated to full detail).
            let exact = build(Lod::exact(), &mut sources)?;
            let n = exact.len();
            if n <= 4 { exact } else { (0..4).map(|k| exact[k * n / 4]).collect() }
        };
        let ring = Ring::new(pts).map_err(|_| MapError::NothingAtTime(*at))?;
        Ok((ring, sources))
    }

    /// A Way's stations, made legible: one marker per waypoint the
    /// gazetteer can resolve, and (labels layer on) the station's
    /// canonical name, attached to the route so selection follows the
    /// journey. Sources ride each marker — a semantic selection keeps
    /// what the text grounds.
    fn push_way_stations(
        &self,
        scene: &mut Snapshot,
        id: BoundaryId,
        at: &TimePoint,
        q: &RenderQuery,
        style: &Style,
        seen: &mut BTreeSet<atlas_graph_types::covenant::PlaceId>,
    ) -> Result<(), MapError> {
        let Some((iv, b)) = self.version_at(id, at) else { return Ok(()) };
        let BoundarySource::Survey(survey) = &b.source else { return Ok(()) };
        let Some(gaz) = self.gazetteer.as_ref() else { return Ok(()) };
        let sources = boundary_sources(b);
        // A station appears when the walk REACHES it: its arc position
        // along the way against the elapsed fraction of the span.
        let f = way_fraction(iv, at);
        let mut cum = vec![0.0f64];
        for w in b.pts.windows(2) {
            cum.push(cum.last().unwrap() + w[0].angle_to(&w[1]));
        }
        let total = *cum.last().unwrap();
        for (idx, wp) in survey.waypoints.iter().enumerate() {
            let reached = match cum.get(idx) {
                Some(c) if total > 0.0 => c / total <= f + 1e-9,
                _ => true,
            };
            if !reached {
                break; // stations come in text order; the walk stops here
            }
            // A station shared by several journeys keeps ONE marker
            // and one name — the ways cross, the place is itself.
            if !seen.insert(wp.0.clone()) {
                continue;
            }
            let Some(entry) = gaz.places.get(&wp.0) else { continue };
            if !in_viewport(&q.viewport, std::slice::from_ref(&entry.position)) {
                continue;
            }
            scene.markers.push(StyledMarker {
                at: entry.position,
                style: style.marker_style(),
                sources: sources.clone(),
                place: Some(wp.clone()),
            });
            if q.layers.contains(LayerSet::LABELS) {
                let mut label = style.label_style();
                label.size *= 0.8; // a station is a footnote to the way
                scene.labels.push(PlacedLabel {
                    text: entry.canonical_name.clone(),
                    at: entry.position,
                    subject: LabelSubject::Place(wp.clone()),
                    style: label,
                    face: map_types::scene::LabelFace::Place,
                });
            }
        }
        Ok(())
    }

    fn styled_region(
        &self,
        id: RegionId,
        at: &TimePoint,
        lod: Lod,
        style: &Style,
        paint: Paint,
    ) -> Result<Option<(StyledRegion, Option<PlacedLabel>)>, MapError> {
        let Some(hist) = self.timeline.regions.get(&id) else {
            return Err(MapError::UnknownRegion(id));
        };
        let Some(geom) = hist.geom_at(at) else { return Ok(None) };
        let mut outer = Vec::new();
        let mut holes = Vec::new();
        let mut sources = BTreeSet::new();
        for RegionPart { cycle, holes: hole_cycles } in &geom.parts {
            let (ring, s) = self.resolve_ring(cycle, at, lod)?;
            outer.push(ring);
            sources.extend(s);
            for hc in hole_cycles {
                let (ring, s) = self.resolve_ring(hc, at, lod)?;
                holes.push(ring);
                sources.extend(s);
            }
        }
        // Deterministic label anchor: the normalized centroid of the
        // first outer ring.
        let label = hist.label_at(at).and_then(|text| {
            let pts = outer.first()?.points();
            let (mut x, mut y, mut z) = (0.0, 0.0, 0.0);
            for p in pts {
                x += p.x();
                y += p.y();
                z += p.z();
            }
            let at_pt = UnitVec::normalize(x, y, z).ok()?;
            Some(PlacedLabel {
                text: text.to_string(),
                at: at_pt,
                subject: LabelSubject::Region(id),
                style: style.label_style(),
                face: map_types::scene::LabelFace::Territory,
            })
        });
        Ok(Some((StyledRegion { region: id, outer, holes, paint, sources }, label)))
    }

    fn styled_boundary(
        &self,
        id: BoundaryId,
        at: &TimePoint,
        lod: Lod,
        style: &Style,
    ) -> Result<StyledBoundary, MapError> {
        let b = self.boundary_at(id, at)?;
        Ok(StyledBoundary {
            boundary: id,
            pts: simplify_polyline(&b.pts, lod),
            stroke: *style.stroke_for(&b.character),
            sources: boundary_sources(b),
        })
    }

    fn push_region(
        &self,
        scene: &mut Snapshot,
        q: &RenderQuery,
        id: RegionId,
        at: &TimePoint,
        style: &Style,
        paint: Paint,
        with_label: bool,
    ) -> Result<(), MapError> {
        // Water rides the TOPOGRAPHY layer in the water paint; relief
        // bands ride RELIEF in the hypsometric ramp; neither age-ramps.
        let paint = match self.timeline.regions.get(&id).map(|h| h.class).unwrap_or_default() {
            map_types::RegionClass::Land => paint,
            map_types::RegionClass::Water => {
                if !q.layers.contains(LayerSet::TOPOGRAPHY) {
                    return Ok(());
                }
                style.water_paint()
            }
            map_types::RegionClass::Terrain(band) => {
                if !q.layers.contains(LayerSet::RELIEF) {
                    return Ok(());
                }
                let ramp = style.topo_ramp();
                mix_paint(ramp.oldest, ramp.newest, f64::from(band) / 4.0)
            }
        };
        if let Some((region, label)) = self.styled_region(id, at, q.lod, style, paint)? {
            if in_viewport(&q.viewport, region.outer.first().map(Ring::points).unwrap_or(&[])) {
                scene.attribution.extend(region.sources.iter().cloned());
                scene.regions.push(region);
                if with_label && q.layers.contains(LayerSet::LABELS) {
                    if let Some(l) = label {
                        scene.labels.push(l);
                    }
                }
            }
        }
        Ok(())
    }

    /// Assign each land region alive at `at` a palette slot such that
    /// regions whose extents touch never share one — the atlas
    /// convention ("obvious what's what"), computed from the TIMELINE,
    /// not the scene, so a lone rendering and the world agree (law 10).
    /// Deterministic: id-ordered greedy, seeded by each region's hash.
    fn palette_assignment(
        &self,
        at: &TimePoint,
        style: &Style,
    ) -> Option<BTreeMap<RegionId, Paint>> {
        let palette = style.palette()?;
        type Bb = (f64, f64, f64, f64);
        let mut boxes: Vec<(RegionId, Bb)> = Vec::new();
        for (id, hist) in &self.timeline.regions {
            if hist.class != map_types::RegionClass::Land {
                continue;
            }
            let Some(geom) = hist.geom_at(at) else { continue };
            let mut b: Option<Bb> = None;
            for part in &geom.parts {
                for (bid, _) in part.cycle.iter().chain(part.holes.iter().flatten()) {
                    if let Some(bd) = self.timeline.boundaries.get(bid).and_then(|h| h.at(at)) {
                        for p in &bd.pts {
                            let lat = p.z().asin().to_degrees();
                            let lon = p.y().atan2(p.x()).to_degrees();
                            b = Some(match b {
                                None => (lon, lat, lon, lat),
                                Some((x0, y0, x1, y1)) => {
                                    (x0.min(lon), y0.min(lat), x1.max(lon), y1.max(lat))
                                }
                            });
                        }
                    }
                }
            }
            if let Some(b) = b {
                boxes.push((*id, b));
            }
        }
        let touches = |a: &Bb, b: &Bb| -> bool {
            const M: f64 = 0.2; // neighbors within a fifth of a degree
            a.0 - M < b.2 && b.0 - M < a.2 && a.1 - M < b.3 && b.1 - M < a.3
        };
        let mut chosen: BTreeMap<RegionId, usize> = BTreeMap::new();
        for i in 0..boxes.len() {
            let (id, bb) = &boxes[i];
            let mut used = [false; 8];
            for (jid, jb) in &boxes {
                if jid != id && touches(bb, jb) {
                    if let Some(&slot) = chosen.get(jid) {
                        used[slot] = true;
                    }
                }
            }
            let seed = (id.0 .0 % 8) as usize;
            let slot = (0..8).map(|k| (seed + k) % 8).find(|&s| !used[s]).unwrap_or(seed);
            chosen.insert(*id, slot);
        }
        Some(chosen.into_iter().map(|(id, s)| (id, palette[s])).collect())
    }

    /// One instant, one subject — the pure heart of the system.
    fn snapshot_at(&self, at: &TimePoint, q: &RenderQuery) -> Result<Snapshot, MapError> {
        let style = self.style(q.style)?;
        let mut scene = Snapshot::empty();
        if !q.layers.contains(LayerSet::GEOMETRY) {
            return Ok(scene);
        }
        let colors = self.palette_assignment(at, style);
        let paint_for = |id: &RegionId| -> Paint {
            colors
                .as_ref()
                .and_then(|m| m.get(id))
                .copied()
                .unwrap_or_else(|| style.region_paint())
        };
        match &q.subject {
            RenderSubject::World => {
                let region_ids: Vec<RegionId> = self.timeline.regions.keys().copied().collect();
                for id in region_ids {
                    self.push_region(&mut scene, q, id, at, style, paint_for(&id), true)?;
                }
                sort_largest_first(&mut scene.regions);
                // Relief renders as tint bands, never as border strokes:
                // arcs referenced by terrain regions are skipped here.
                let terrain_arcs: BTreeSet<BoundaryId> = self
                    .timeline
                    .regions
                    .values()
                    .filter(|h| matches!(h.class, map_types::RegionClass::Terrain(_)))
                    .filter_map(|h| h.geom_at(at))
                    .flat_map(|g| {
                        g.parts.iter().flat_map(|p| {
                            p.cycle.iter().chain(p.holes.iter().flatten()).map(|(b, _)| *b)
                        })
                    })
                    .collect();
                let boundary_ids: Vec<BoundaryId> =
                    self.timeline.boundaries.keys().copied().collect();
                let mut seen_stations = BTreeSet::new();
                for id in boundary_ids {
                    if terrain_arcs.contains(&id) {
                        continue;
                    }
                    if let Some((iv, b)) = self.version_at(id, at) {
                        // Journeys are their own layer: a way and its
                        // stations render only when asked for.
                        let is_way = b.character == map_types::EdgeCharacter::Way;
                        if is_way && !q.layers.contains(LayerSet::JOURNEYS) {
                            continue;
                        }
                        let mut sb = self.styled_boundary(id, at, q.lod, style)?;
                        if is_way {
                            // Mid-walk, the map shows the road so far.
                            sb.pts = walked_prefix(&sb.pts, way_fraction(iv, at));
                        }
                        if in_viewport(&q.viewport, &sb.pts) {
                            scene.attribution.extend(sb.sources.iter().cloned());
                            scene.boundaries.push(sb);
                            if is_way {
                                self.push_way_stations(
                                    &mut scene,
                                    id,
                                    at,
                                    q,
                                    style,
                                    &mut seen_stations,
                                )?;
                            }
                        }
                    }
                }
            }
            RenderSubject::Region(id) => {
                if self.timeline.regions.get(id).map(|h| h.geom_at(at)).flatten().is_none() {
                    self.timeline
                        .regions
                        .contains_key(id)
                        .then_some(())
                        .ok_or(MapError::UnknownRegion(*id))?;
                    return Err(MapError::NothingAtTime(*at));
                }
                self.push_region(&mut scene, q, *id, at, style, paint_for(id), true)?;
            }
            RenderSubject::Boundary(id) => {
                let sb = self.styled_boundary(*id, at, q.lod, style)?;
                if in_viewport(&q.viewport, &sb.pts) {
                    scene.attribution.extend(sb.sources.iter().cloned());
                    scene.boundaries.push(sb);
                }
            }
            RenderSubject::RawPoint(p) => {
                if in_viewport(&q.viewport, std::slice::from_ref(p)) {
                    scene.markers.push(StyledMarker {
                        at: *p,
                        style: style.marker_style(),
                        sources: Default::default(),
                        place: None,
                    });
                }
            }
            RenderSubject::Point(place) => {
                let gaz = self.gazetteer.as_ref().ok_or_else(|| {
                    MapError::UnknownPlace(place.0 .0.clone())
                })?;
                let entry = gaz
                    .places
                    .get(&place.0)
                    .ok_or_else(|| MapError::UnknownPlace(place.0 .0.clone()))?;
                if in_viewport(&q.viewport, std::slice::from_ref(&entry.position)) {
                    // A place a Survey walks through is scripture-
                    // grounded — the text is why it is on the map at
                    // all. Anything else stands on the gazetteer.
                    let surveyed = self.timeline.boundaries.values().any(|h| {
                        h.versions.iter().any(|(_, b)| match &b.source {
                            BoundarySource::Survey(s) => {
                                s.waypoints.iter().any(|w| w.0 == place.0)
                            }
                            _ => false,
                        })
                    });
                    let sources = BTreeSet::from([SourceId::new(if surveyed {
                        SCRIPTURE_SOURCE
                    } else {
                        "atlas-gazetteer"
                    })]);
                    scene.attribution.extend(sources.iter().cloned());
                    scene.markers.push(StyledMarker {
                        at: entry.position,
                        style: style.marker_style(),
                        sources,
                        place: Some(place.clone()),
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
            }
            RenderSubject::RegionTerrain(_) => return Err(MapError::TerrainUnavailable),
            RenderSubject::Change(id) => {
                let event = self
                    .timeline
                    .events
                    .iter()
                    .find(|e| e.id() == *id)
                    .ok_or(MapError::UnknownChange(*id))?;
                self.render_delta(&mut scene, q, event, style)?;
            }
        }
        // The JOURNEYS layer rides EVERY scene that asks for it:
        // focusing a subject never hides the ways walked through the
        // world around it. (The World arm draws them in its own
        // boundary walk above.)
        if q.layers.contains(LayerSet::JOURNEYS)
            && !matches!(q.subject, RenderSubject::World)
        {
            let mut seen = BTreeSet::new();
            let ids: Vec<BoundaryId> = self.timeline.boundaries.keys().copied().collect();
            for id in ids {
                if let Some((iv, b)) = self.version_at(id, at) {
                    if b.character != map_types::EdgeCharacter::Way {
                        continue;
                    }
                    let mut sb = self.styled_boundary(id, at, q.lod, style)?;
                    sb.pts = walked_prefix(&sb.pts, way_fraction(iv, at));
                    if in_viewport(&q.viewport, &sb.pts) {
                        scene.attribution.extend(sb.sources.iter().cloned());
                        scene.boundaries.push(sb);
                        self.push_way_stations(&mut scene, id, at, q, style, &mut seen)?;
                    }
                }
            }
        }
        Ok(scene)
    }

    /// A DELTA rendered: what stood before in the before-stroke, what
    /// stands after in the after-stroke, a split's seam in seam stroke.
    fn render_delta(
        &self,
        scene: &mut Snapshot,
        q: &RenderQuery,
        event: &ChangeEvent,
        style: &Style,
    ) -> Result<(), MapError> {
        let d = style.delta_emphasis();
        let at = event.at;
        // The instant just before the event: any version whose interval
        // ENDS at the event time.
        let before_version = |id: BoundaryId| -> Option<&Boundary> {
            self.timeline.boundaries.get(&id)?.versions.iter().find_map(|(iv, b)| {
                (iv.to == Some(at)).then_some(b)
            })
        };
        let mut stroke_region = |id: RegionId, when: Which, stroke| -> Result<(), MapError> {
            let hist = self.timeline.regions.get(&id).ok_or(MapError::UnknownRegion(id))?;
            let geom = match when {
                Which::After => hist.geom_at(&at),
                Which::Before => hist
                    .geom_history
                    .iter()
                    .find_map(|(iv, g)| (iv.to == Some(at)).then_some(g)),
            };
            let Some(geom) = geom else { return Ok(()) };
            for part in &geom.parts {
                for (bid, _) in &part.cycle {
                    // Draw each arc of the cycle in the emphasis stroke,
                    // resolved at the right instant.
                    let b = match when {
                        Which::After => self.boundary_at(*bid, &at).ok(),
                        Which::Before => before_version(*bid),
                    };
                    let Some(b) = b else { continue };
                    scene.boundaries.push(StyledBoundary {
                        boundary: *bid,
                        pts: simplify_polyline(&b.pts, q.lod),
                        stroke,
                        sources: boundary_sources(b),
                    });
                }
            }
            Ok(())
        };
        enum Which {
            Before,
            After,
        }
        match &event.kind {
            ChangeKind::Rise { region } | ChangeKind::Rename { region } => {
                stroke_region(*region, Which::After, d.after)?
            }
            ChangeKind::Fall { region } => stroke_region(*region, Which::Before, d.before)?,
            ChangeKind::Shift { boundary } => {
                if let Some(b) = before_version(*boundary) {
                    scene.boundaries.push(StyledBoundary {
                        boundary: *boundary,
                        pts: simplify_polyline(&b.pts, q.lod),
                        stroke: d.before,
                        sources: boundary_sources(b),
                    });
                }
                if let Ok(b) = self.boundary_at(*boundary, &at) {
                    scene.boundaries.push(StyledBoundary {
                        boundary: *boundary,
                        pts: simplify_polyline(&b.pts, q.lod),
                        stroke: d.after,
                        sources: boundary_sources(b),
                    });
                }
            }
            ChangeKind::Split { parent, children, seam } => {
                stroke_region(*parent, Which::Before, d.before)?;
                for c in children {
                    stroke_region(*c, Which::After, d.after)?;
                }
                if seam.len() >= 2 {
                    scene.boundaries.push(StyledBoundary {
                        boundary: BoundaryId(event.map_pid().hash),
                        pts: simplify_polyline(seam, q.lod),
                        stroke: d.seam,
                        sources: BTreeSet::new(),
                    });
                }
            }
            ChangeKind::Merge { parents, child } => {
                for p in parents {
                    stroke_region(*p, Which::Before, d.before)?;
                }
                stroke_region(*child, Which::After, d.after)?;
            }
            ChangeKind::Journey { boundary } => {
                // The walk so far, in the "after" emphasis.
                if let Ok(b) = self.boundary_at(*boundary, &at) {
                    scene.boundaries.push(StyledBoundary {
                        boundary: *boundary,
                        pts: simplify_polyline(&b.pts, q.lod),
                        stroke: d.after,
                        sources: boundary_sources(b),
                    });
                }
            }
        }
        for b in &scene.boundaries {
            scene.attribution.extend(b.sources.iter().cloned());
        }
        Ok(())
    }

    /// The arcs a subject wears at one instant — the accumulation's
    /// per-sample border census.
    fn subject_arcs_at(&self, subject: &RenderSubject, t: &TimePoint) -> Vec<BoundaryId> {
        match subject {
            RenderSubject::World => self
                .timeline
                .boundaries
                .iter()
                .filter(|(_, h)| h.at(t).is_some())
                .map(|(id, _)| *id)
                .collect(),
            RenderSubject::Region(rid) => self
                .timeline
                .regions
                .get(rid)
                .and_then(|h| h.geom_at(t))
                .map(|g| {
                    g.parts
                        .iter()
                        .flat_map(|p| {
                            p.cycle.iter().chain(p.holes.iter().flatten()).map(|(b, _)| *b)
                        })
                        .collect()
                })
                .unwrap_or_default(),
            RenderSubject::Boundary(bid) => {
                if self.timeline.boundaries.get(bid).and_then(|h| h.at(t)).is_some() {
                    vec![*bid]
                } else {
                    Vec::new()
                }
            }
            _ => Vec::new(),
        }
    }

    /// The long exposure: fold of overlay across the interval's
    /// distinct states — endpoints plus every change event (law 9).
    /// Temporal depth is STYLE: sample i of n wears the age ramp at its
    /// position, newest last and on top; a single-sample accumulation
    /// IS its snapshot (fold identity), so the ramp applies only when
    /// there are at least two states. Labels ride only the newest
    /// sample — a long exposure of borders, captioned once.
    ///
    /// THE DIFFS ARE LINES: every distinct border that existed anywhere
    /// in the range is drawn once, stroked in the ramp color of the
    /// last sample it was alive at — vanished borders faint, current
    /// borders strong, a moved border showing each of its positions.
    /// Fills convey tenure (alpha stacks where territory held), lines
    /// convey edges; both ride the same ramp.
    fn accumulation(&self, over: &Interval, q: &RenderQuery) -> Result<Snapshot, MapError> {
        let times = sample_times(over, &self.timeline.events);
        if times.len() <= 1 {
            let at = times.first().copied().unwrap_or(over.from);
            return self.snapshot_at(&at, q);
        }
        let style = self.style(q.style)?;
        let ramp = style.age_ramp();
        let n = times.len();
        let mut scenes = Vec::with_capacity(n);
        // Per arc: the last sample it was alive at (its recency tint).
        let mut last_alive: BTreeMap<BoundaryId, usize> = BTreeMap::new();
        for (i, t) in times.iter().enumerate() {
            let newest = i == n - 1;
            let toward_new = (i as f64) / ((n - 1) as f64);
            let paint = mix_paint(ramp.oldest, ramp.newest, toward_new);
            let mut sub = q.clone();
            if !newest {
                sub.layers = LayerSet::GEOMETRY; // labels only on newest
            }
            let mut scene = Snapshot::empty();
            match &q.subject {
                // A world range would saturate under stacked fills
                // (all land is always claimed by someone), so the
                // world wears only its NEWEST state as fills and lets
                // the tinted lines below carry every older border.
                RenderSubject::World if newest => {
                    let colors = self.palette_assignment(t, style);
                    let ids: Vec<RegionId> = self.timeline.regions.keys().copied().collect();
                    for id in ids {
                        let p = colors
                            .as_ref()
                            .and_then(|m| m.get(&id))
                            .copied()
                            .unwrap_or_else(|| style.region_paint());
                        self.push_region(&mut scene, &sub, id, t, style, p, true)?;
                    }
                    sort_largest_first(&mut scene.regions);
                }
                RenderSubject::World => {}
                // A focused subject's range keeps every state's fill,
                // age-ramped: its extent story, tenure as depth.
                RenderSubject::Region(id) => {
                    self.push_region(&mut scene, &sub, *id, t, style, paint, newest)?;
                }
                _ => {
                    scene = self.snapshot_at(t, &sub)?;
                }
            }
            for arc in self.subject_arcs_at(&q.subject, t) {
                last_alive.insert(arc, i);
            }
            scenes.push(scene);
        }
        let mut scene = accumulate(scenes);

        // Draw each border once, oldest recency first so the newest
        // lines land on top. Deterministic: sorted by (recency, id).
        let mut by_recency: Vec<(usize, BoundaryId)> =
            last_alive.iter().map(|(id, i)| (*i, *id)).collect();
        by_recency.sort();
        for (i, id) in by_recency {
            let t = &times[i];
            let b = self.boundary_at(id, t)?;
            let is_way = b.character == map_types::EdgeCharacter::Way;
            if is_way && !q.layers.contains(LayerSet::JOURNEYS) {
                continue;
            }
            let toward_new = (i as f64) / ((n - 1) as f64);
            let tint = mix_paint(ramp.oldest, ramp.newest, toward_new).fill;
            let base = style.stroke_for(&b.character);
            let stroke = map_types::style::Stroke {
                color: map_types::style::Rgba(
                    tint.0,
                    tint.1,
                    tint.2,
                    // Lines must stay readable even at ancient ages.
                    (120.0 + 135.0 * toward_new) as u8,
                ),
                width: base.width,
                pattern: base.pattern,
            };
            let mut pts = simplify_polyline(&b.pts, q.lod);
            if is_way {
                // In a range, a way shows what was walked BY its last
                // living sample — the whole road when the range holds
                // the whole walk, the road so far when it is cut short.
                if let Some((iv, _)) = self.version_at(id, t) {
                    pts = walked_prefix(&pts, way_fraction(iv, t));
                }
            }
            let sb = StyledBoundary { boundary: id, pts, stroke, sources: boundary_sources(b) };
            if in_viewport(&q.viewport, &sb.pts) {
                scene.attribution.extend(sb.sources.iter().cloned());
                scene.boundaries.push(sb);
            }
        }
        // The range's stations: each way's stops as reached by its
        // last living sample, deduped across journeys.
        if q.layers.contains(LayerSet::JOURNEYS) {
            let mut seen = BTreeSet::new();
            for (id, i) in &last_alive {
                let t = &times[*i];
                if self
                    .version_at(*id, t)
                    .is_some_and(|(_, b)| b.character == map_types::EdgeCharacter::Way)
                {
                    self.push_way_stations(&mut scene, *id, t, q, style, &mut seen)?;
                }
            }
        }
        Ok(scene)
    }
}

// ------------------------------------------------------- the contract

impl MapProvider for TimelineProvider {
    fn subjects(&self, at: TimePoint) -> Vec<SubjectListing> {
        let mut out = vec![SubjectListing {
            subject: RenderSubject::World,
            label: "the world".to_string(),
        }];
        for (id, hist) in &self.timeline.regions {
            if hist.geom_at(&at).is_some() {
                if let Some(label) = hist.label_at(&at) {
                    out.push(SubjectListing {
                        subject: RenderSubject::Region(*id),
                        label: label.to_string(),
                    });
                }
            }
        }
        // Stations of the ways active now are subjects too: the
        // places the text walks through, each listed once.
        if let Some(gaz) = self.gazetteer.as_ref() {
            let mut seen = BTreeSet::new();
            for hist in self.timeline.boundaries.values() {
                let Some(b) = hist.at(&at) else { continue };
                if b.character != map_types::EdgeCharacter::Way {
                    continue;
                }
                let BoundarySource::Survey(survey) = &b.source else { continue };
                for wp in &survey.waypoints {
                    if !seen.insert(wp.0.clone()) {
                        continue;
                    }
                    if let Some(entry) = gaz.places.get(&wp.0) {
                        out.push(SubjectListing {
                            subject: RenderSubject::Point(wp.clone()),
                            label: entry.canonical_name.clone(),
                        });
                    }
                }
            }
        }
        out
    }

    fn render(&self, q: &RenderQuery) -> Result<Snapshot, MapError> {
        match &q.time {
            TimeSelector::At(t) => self.snapshot_at(t, q),
            TimeSelector::Over(interval) => self.accumulation(interval, q),
        }
    }

    fn transition(
        &self,
        from: TimePoint,
        to: TimePoint,
        _viewport: Bbox,
        lod: Lod,
    ) -> Result<TransitionScript, MapError> {
        if from == to {
            return Ok(TransitionScript::empty()); // the identity (law 8)
        }
        if from > to {
            return Ok(invert(self.transition(to, from, _viewport, lod)?));
        }
        let mut script = TransitionScript::empty();
        for e in self.changes_between(from, to) {
            match &e.kind {
                ChangeKind::Rise { region } => {
                    script.steps.push(TransitionStep::FadeIn { region: *region })
                }
                ChangeKind::Fall { region } => {
                    script.steps.push(TransitionStep::FadeOut { region: *region })
                }
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
                ChangeKind::Shift { boundary } => {
                    // A same-arc shift MORPHS (slerp pairs, equal counts
                    // by resampling). The epoch source replaces arcs
                    // wholesale, so its shifts crossfade the owning
                    // regions instead — the honest animation for state
                    // data; morphs light up when sources carry
                    // correspondence.
                    let hist = self.timeline.boundaries.get(boundary);
                    let before = hist.and_then(|h| {
                        h.versions.iter().find_map(|(iv, b)| (iv.to == Some(e.at)).then_some(b))
                    });
                    let after = hist.and_then(|h| h.at(&e.at));
                    match (before, after) {
                        (Some(b0), Some(b1)) => {
                            let a = simplify_polyline(&b0.pts, lod);
                            let b = simplify_polyline(&b1.pts, lod);
                            let n = a.len().max(b.len()).max(2);
                            script.steps.push(TransitionStep::Morph {
                                boundary: *boundary,
                                from_pts: resample(&a, n),
                                to_pts: resample(&b, n),
                            });
                        }
                        _ => {
                            for (rid, rh) in &self.timeline.regions {
                                let touches = rh
                                    .geom_at(&e.at)
                                    .map(|g| {
                                        g.parts.iter().any(|p| {
                                            p.cycle
                                                .iter()
                                                .chain(p.holes.iter().flatten())
                                                .any(|(b, _)| b == boundary)
                                        })
                                    })
                                    .unwrap_or(false);
                                if touches {
                                    script.steps.push(TransitionStep::FadeOut { region: *rid });
                                    script.steps.push(TransitionStep::FadeIn { region: *rid });
                                }
                            }
                        }
                    }
                }
                // A journey's progress is time-parameterized rendering,
                // not a topology change: transitions carry no step for
                // it (the growing road is the At-time render's job).
                ChangeKind::Journey { .. } => {}
            }
        }
        Ok(script)
    }

    /// The scrubber's stops: events with from < at <= to, in time order.
    fn changes_between(&self, from: TimePoint, to: TimePoint) -> Vec<&ChangeEvent> {
        let (lo, hi) = if from <= to { (from, to) } else { (to, from) };
        let mut events: Vec<&ChangeEvent> =
            self.timeline.events.iter().filter(|e| e.at > lo && e.at <= hi).collect();
        events.sort_by_key(|e| e.at);
        events
    }
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

#[cfg(test)]
mod tests;
pub mod canon_provider;
