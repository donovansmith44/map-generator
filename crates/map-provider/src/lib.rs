//! The reference MapProvider — phase 3 of docs/map-system-handoff.md.
//!
//! A snapshot at time t is a DERIVED, deterministic query result, never
//! a hand-maintained artifact: materialization = select ∘ simplify ∘
//! style, each stage a total pure function of the timeline (covenant
//! rules 6 and 10). "Tons of snapshots" = tons of queries, cached by
//! content hash — determinism (law 1) is what makes the cache and the
//! offline story sound.

use std::collections::{BTreeMap, BTreeSet};

use atlas_graph_types::chrono::TimePoint;
use atlas_graph_types::id::SourceId;

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

/// The reference provider: a timeline, the styles it may be asked to
/// wear, and (when the atlas handshake lands) the gazetteer for place
/// subjects.
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

    /// Resolve one part's cycle to a ring: oriented simplified arcs,
    /// each dropping its trailing junction. If simplification would
    /// collapse the ring below three points, the unsimplified geometry
    /// stands — lod never changes topology (law 7's second clause).
    fn resolve_ring(
        &self,
        cycle: &[(BoundaryId, Orientation)],
        at: &TimePoint,
        lod: Lod,
    ) -> Result<(Ring, BTreeSet<SourceId>), MapError> {
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
        let pts = if simplified.len() >= 3 { simplified } else { build(Lod::exact(), &mut sources)? };
        let ring = Ring::new(pts).map_err(|_| MapError::NothingAtTime(*at))?;
        Ok((ring, sources))
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

    /// One instant, one subject — the pure heart of the system.
    fn snapshot_at(&self, at: &TimePoint, q: &RenderQuery) -> Result<Snapshot, MapError> {
        let style = self.style(q.style)?;
        let mut scene = Snapshot::empty();
        if !q.layers.contains(LayerSet::GEOMETRY) {
            return Ok(scene);
        }
        match &q.subject {
            RenderSubject::World => {
                let region_ids: Vec<RegionId> = self.timeline.regions.keys().copied().collect();
                for id in region_ids {
                    self.push_region(&mut scene, q, id, at, style, style.region_paint(), true)?;
                }
                sort_largest_first(&mut scene.regions);
                let boundary_ids: Vec<BoundaryId> =
                    self.timeline.boundaries.keys().copied().collect();
                for id in boundary_ids {
                    if self.timeline.boundaries[&id].at(at).is_some() {
                        let sb = self.styled_boundary(id, at, q.lod, style)?;
                        if in_viewport(&q.viewport, &sb.pts) {
                            scene.attribution.extend(sb.sources.iter().cloned());
                            scene.boundaries.push(sb);
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
                self.push_region(&mut scene, q, *id, at, style, style.region_paint(), true)?;
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
                    scene.markers.push(StyledMarker { at: *p, style: style.marker_style() });
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
                    scene
                        .markers
                        .push(StyledMarker { at: entry.position, style: style.marker_style() });
                    if q.layers.contains(LayerSet::LABELS) {
                        scene.labels.push(PlacedLabel {
                            text: entry.canonical_name.clone(),
                            at: entry.position,
                            subject: LabelSubject::Free,
                            style: style.label_style(),
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
                    let ids: Vec<RegionId> = self.timeline.regions.keys().copied().collect();
                    for id in ids {
                        self.push_region(&mut scene, &sub, id, t, style, style.region_paint(), true)?;
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
            let sb = StyledBoundary {
                boundary: id,
                pts: simplify_polyline(&b.pts, q.lod),
                stroke,
                sources: boundary_sources(b),
            };
            if in_viewport(&q.viewport, &sb.pts) {
                scene.attribution.extend(sb.sources.iter().cloned());
                scene.boundaries.push(sb);
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
