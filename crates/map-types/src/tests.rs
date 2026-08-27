//! The design laws (spec §D) as tests. One test module per law,
//! numbered as in the spec. Laws that need the phase-3 materializer to
//! test in full (1's byte-identical scenes, 2's geometric overlap, 3's
//! endpoint rule against a real provider, 10's provider equation) are
//! tested here in their type-level form and re-proven against the real
//! materializer when it lands — each notes what remains.

use std::collections::BTreeMap;

use atlas_graph_types::covenant::{
    PlacementBasis, ResolvedDate, ResolvedPlacement, SeqKey, TimePoint, Year,
};
use atlas_graph_types::covenant::Justification;
use atlas_graph_types::covenant::{ContentHash, EventId, PlaceId, SourceId};
use atlas_graph_types::covenant::{BibleLocus, LocusRange, VerseRef};

use crate::algebra::Monoid;
use crate::boundary::*;
use crate::contracts::*;
use crate::geom::*;
use crate::ident::*;
use crate::laws::*;
use crate::query::*;
use crate::scene::*;
use crate::style::*;
use crate::timeline::*;
use crate::transition::*;

// ---------------------------------------------------------------- fixtures

fn y(i: i32) -> Year {
    Year::new(i).unwrap()
}
fn tp(i: i32) -> TimePoint {
    TimePoint::year_only(y(i))
}
fn uv(lat: f64, lon: f64) -> UnitVec {
    UnitVec::from_lat_lon_deg(lat, lon)
}
fn prov() -> String {
    "test:fixture".to_string()
}
fn imported_source() -> BoundarySource {
    BoundarySource::Imported { source: SourceId::new("historical-source") }
}
fn arc(pts: Vec<UnitVec>) -> Boundary {
    Boundary {
        pts,
        character: EdgeCharacter::Line,
        source: imported_source(),
        justification: Justification::default(),
        provenance: prov(),
    }
}
fn open_hist(from: i32, b: Boundary) -> BoundaryHistory {
    BoundaryHistory { versions: vec![(Interval::open_from(tp(from)), b)] }
}
fn shift_event(bid: BoundaryId, at: i32) -> ChangeEvent {
    ChangeEvent {
        at: tp(at),
        kind: ChangeKind::Shift { boundary: bid },
        driver: None,
        justification: Justification::default(),
        provenance: prov(),
    }
}

const W: BoundaryId = BoundaryId(ContentHash(1));
const S: BoundaryId = BoundaryId(ContentHash(2));
const E: BoundaryId = BoundaryId(ContentHash(3));
const A: RegionId = RegionId(ContentHash(10));
const B: RegionId = RegionId(ContentHash(11));

/// Two regions sharing ONE arc (S): the arc-sharing shape from §B.
/// Junctions n and s are the SAME values in every arc touching them.
fn two_region_world() -> WorldTimeline {
    let n = uv(10.0, 0.0);
    let s = uv(-10.0, 0.0);
    let west = arc(vec![n, uv(0.0, -10.0), s]); // n -> s the long way west
    let shared = arc(vec![s, uv(0.0, 0.0), n]); // s -> n up the middle
    let east = arc(vec![s, uv(0.0, 10.0), n]); // s -> n the long way east

    let mut boundaries = BTreeMap::new();
    boundaries.insert(W, open_hist(-2000, west));
    boundaries.insert(S, open_hist(-2000, shared));
    boundaries.insert(E, open_hist(-2000, east));

    let region = |name: &str, cycle: Vec<(BoundaryId, Orientation)>| RegionHistory {
        class: RegionClass::default(),
        label_history: vec![(Interval::open_from(tp(-2000)), name.to_string())],
        geom_history: vec![(
            Interval::open_from(tp(-2000)),
            RegionGeom { parts: vec![RegionPart { cycle, holes: vec![] }] },
        )],
    };
    let mut regions = BTreeMap::new();
    regions.insert(A, region("Westland", vec![(W, Orientation::Forward), (S, Orientation::Forward)]));
    regions.insert(B, region("Eastland", vec![(S, Orientation::Reverse), (E, Orientation::Forward)]));

    WorldTimeline { anchor: None, boundaries, regions, events: Vec::new(), atlas_pin: None }
}

fn biblical_anchor(at: i32) -> Anchor {
    Anchor {
        frame: "biblical (Ussher tradition)".to_string(),
        at: tp(at),
        justification: Justification::default(),
        provenance: prov(),
    }
}

fn honest_style() -> Style {
    let stroke = |r, pattern| Stroke { color: Rgba(r, 0, 0, 255), width: 1.0, pattern };
    Style::new(
        BoundaryStrokes {
            line: stroke(0, StrokePattern::Solid),
            frontier: stroke(60, StrokePattern::Zonal),
            disputed: stroke(120, StrokePattern::Hatched),
            unknown: stroke(180, StrokePattern::Dashed),
            way: stroke(240, StrokePattern::Dashed),
        },
        Paint { fill: Rgba(200, 200, 180, 255) },
        Paint { fill: Rgba(120, 160, 200, 235) },
        AgeRamp {
            newest: Paint { fill: Rgba(150, 110, 80, 200) },
            oldest: Paint { fill: Rgba(225, 214, 180, 200) },
        },
        None,
        AgeRamp {
            newest: Paint { fill: Rgba(255, 0, 0, 255) },
            oldest: Paint { fill: Rgba(255, 0, 0, 40) },
        },
        LabelStyle { color: Rgba(20, 20, 20, 255), halo: Rgba(245, 240, 225, 220), size: 12.0 },
        MarkerStyle { color: Rgba(0, 0, 0, 255), size: 4.0 },
        DeltaEmphasis {
            before: stroke(90, StrokePattern::Dashed),
            after: stroke(30, StrokePattern::Solid),
            seam: stroke(250, StrokePattern::Solid),
        },
    )
    .unwrap()
}

fn marker_scene(tag: u8) -> Snapshot {
    let mut sc = Snapshot::empty();
    sc.markers.push(StyledMarker {
        at: uv(f64::from(tag), f64::from(tag)),
        style: MarkerStyle { color: Rgba(tag, tag, tag, 255), size: 3.0 },
        sources: Default::default(),
        place: None,
    });
    sc.attribution.insert(SourceId::new(format!("src-{tag}")));
    sc
}

fn check_monoid<M: Monoid + Clone + PartialEq + std::fmt::Debug>(samples: &[M]) {
    for a in samples {
        assert_eq!(M::empty().combine(a.clone()), a.clone(), "left identity");
        assert_eq!(a.clone().combine(M::empty()), a.clone(), "right identity");
    }
    for a in samples {
        for b in samples {
            for c in samples {
                assert_eq!(
                    a.clone().combine(b.clone()).combine(c.clone()),
                    a.clone().combine(b.clone().combine(c.clone())),
                    "associativity"
                );
            }
        }
    }
}

// --------------------------------------------------- law 0: the anchor

/// Owner rulings: history starts at the frame's first event (for the
/// biblical frame, creation) — and the anchor is a PARAMETER, so rival
/// frames each declare their own and comparisons never corner us.
#[test]
fn law00_anchor() {
    // An anchored timeline whose facts all follow the anchor is lawful.
    let mut world = two_region_world();
    world.anchor = Some(biblical_anchor(-4004));
    assert_eq!(validate_anchor(&world), vec![]);

    // A fact BEFORE the anchor is a violation, not a mystery.
    let mut deep_time = two_region_world();
    deep_time.anchor = Some(biblical_anchor(-1000));
    let violations = validate_anchor(&deep_time);
    assert!(
        violations.iter().any(|v| matches!(v, Violation::BeforeAnchor { .. })),
        "pre-anchor history must be flagged: {violations:?}"
    );

    // A different frame anchors the SAME data differently — the anchor
    // is a parameter, and each timeline stands under its own.
    let mut other_frame = two_region_world();
    other_frame.anchor = Some(Anchor {
        frame: "conventional archaeological".to_string(),
        at: tp(-10000),
        justification: Justification::default(),
        provenance: prov(),
    });
    assert_eq!(validate_anchor(&other_frame), vec![]);

    // Anchorless timelines pass vacuously; an anchor with empty
    // provenance is as unlawful as any other unattributed claim.
    assert_eq!(validate_anchor(&two_region_world()), vec![]);
    let mut anonymous = two_region_world();
    anonymous.anchor = Some(Anchor { provenance: String::new(), ..biblical_anchor(-4004) });
    assert!(validate_anchor(&anonymous)
        .iter()
        .any(|v| matches!(v, Violation::EmptyProvenance { .. })));
}

// ------------------------------------------------- law 1: determinism

/// Identical queries hash identically (query hash = cache key = artifact
/// name — the offline story); distinct queries hash apart. Byte-identical
/// scene output is re-proven against the materializer in phase 3.
#[test]
fn law01_query_determinism() {
    let q = RenderQuery {
        subject: RenderSubject::Region(A),
        time: TimeSelector::At(tp(-586)),
        viewport: None,
        lod: Lod(0.001),
        layers: LayerSet::GEOMETRY.with(LayerSet::LABELS),
        style: StyleId(ContentHash(7)),
    };
    assert_eq!(q.map_pid(), q.clone().map_pid());

    let mut coarser = q.clone();
    coarser.lod = Lod(0.01);
    assert_ne!(q.map_pid(), coarser.map_pid());

    let mut other_subject = q.clone();
    other_subject.subject = RenderSubject::World;
    assert_ne!(q.map_pid(), other_subject.map_pid());

    let over = RenderQuery {
        time: TimeSelector::Over(Interval::new(tp(-586), Some(tp(-500))).unwrap()),
        ..q.clone()
    };
    assert_ne!(q.map_pid(), over.map_pid());
}

// ------------------------------------- law 2: partition sanity (structural)

/// Shared arcs chain end-to-start with exact junctions; every cycle
/// reference resolves. The geometric no-overlap half arrives with the
/// materializer.
#[test]
fn law02_partition_structure() {
    let world = two_region_world();
    assert_eq!(validate_partition_structure(&world), vec![]);

    // Break a junction: Eastland's east arc no longer starts at s.
    let mut broken = world.clone();
    let hist = broken.boundaries.get_mut(&E).unwrap();
    hist.versions[0].1.pts[0] = uv(-10.0, 0.5);
    let violations = validate_partition_structure(&broken);
    assert!(
        violations.iter().any(|v| matches!(v, Violation::BrokenCycle { region, .. } if *region == B)),
        "moved junction must break Eastland's cycle: {violations:?}"
    );

    // Reference an arc the timeline doesn't hold.
    let mut dangling = world.clone();
    let ghost = BoundaryId(ContentHash(99));
    dangling.regions.get_mut(&A).unwrap().geom_history[0].1.parts[0].cycle[0].0 = ghost;
    let violations = validate_partition_structure(&dangling);
    assert!(violations
        .iter()
        .any(|v| matches!(v, Violation::DanglingBoundary { boundary, .. } if *boundary == ghost)));
}

// --------------------------------------- law 3: transition composition

/// Toy end-state model: applying a composed script equals applying its
/// parts in sequence, and the empty script (t -> t) changes nothing.
/// The endpoint rule against a real provider is a phase-3 obligation.
#[derive(Clone, Debug, PartialEq)]
struct ToyState {
    present: BTreeMap<RegionId, bool>,
    geoms: BTreeMap<BoundaryId, Vec<UnitVec>>,
}

fn apply(mut st: ToyState, script: &TransitionScript) -> ToyState {
    for step in &script.steps {
        match step {
            TransitionStep::Morph { boundary, to_pts, .. } => {
                st.geoms.insert(*boundary, to_pts.clone());
            }
            TransitionStep::FadeIn { region } => {
                st.present.insert(*region, true);
            }
            TransitionStep::FadeOut { region } => {
                st.present.insert(*region, false);
            }
            TransitionStep::SplitAlong { parent, children, .. } => {
                st.present.insert(*parent, false);
                for c in children {
                    st.present.insert(*c, true);
                }
            }
            TransitionStep::MergeAcross { parents, child } => {
                for p in parents {
                    st.present.insert(*p, false);
                }
                st.present.insert(*child, true);
            }
        }
    }
    st
}

#[test]
fn law03_transition_composition() {
    let c1 = RegionId(ContentHash(21));
    let c2 = RegionId(ContentHash(22));
    let start = ToyState {
        present: BTreeMap::from([(A, true), (B, true)]),
        geoms: BTreeMap::from([(S, vec![uv(0.0, 0.0)])]),
    };
    let first = TransitionScript {
        steps: vec![TransitionStep::Morph {
            boundary: S,
            from_pts: vec![uv(0.0, 0.0)],
            to_pts: vec![uv(1.0, 1.0)],
        }],
    };
    let second = TransitionScript {
        steps: vec![
            TransitionStep::SplitAlong { parent: A, seam: vec![uv(0.0, 0.0)], children: vec![c1, c2] },
            TransitionStep::FadeOut { region: B },
        ],
    };

    // Sequencing agrees with the composed script.
    let sequential = apply(apply(start.clone(), &first), &second);
    let composed = apply(start.clone(), &first.clone().combine(second.clone()));
    assert_eq!(sequential, composed);

    // transition(t, t) is the empty script and changes nothing.
    assert_eq!(apply(start.clone(), &TransitionScript::empty()), start);
}

// --------------------------------------------------- law 4: morph safety

#[test]
fn law04_morph_safety() {
    let tri_a = Ring::new(vec![uv(0.0, 0.0), uv(0.0, 10.0), uv(10.0, 5.0)]).unwrap();
    let tri_b = Ring::new(vec![uv(5.0, 20.0), uv(5.0, 30.0), uv(15.0, 25.0)]).unwrap();
    let start_winding = tri_a.winding();
    assert_eq!(start_winding, tri_b.winding(), "fixture rings must agree");

    for t in [0.0, 0.25, 0.5, 0.75, 1.0] {
        let mid = morph_rings(&tri_a, &tri_b, t).unwrap();
        assert_eq!(mid.len(), tri_a.len(), "closure: point count preserved at t={t}");
        assert_eq!(mid.winding(), start_winding, "winding preserved at t={t}");
    }

    // Endpoints reproduce the inputs (to floating-point identity of path).
    let at_zero = morph_rings(&tri_a, &tri_b, 0.0).unwrap();
    for (p, q) in at_zero.points().iter().zip(tri_a.points()) {
        assert!(p.angle_to(q) < 1e-9);
    }
    let at_one = morph_rings(&tri_a, &tri_b, 1.0).unwrap();
    for (p, q) in at_one.points().iter().zip(tri_b.points()) {
        assert!(p.angle_to(q) < 1e-9);
    }

    // Mismatched counts refuse to morph — resampling is upstream work.
    let square =
        Ring::new(vec![uv(0.0, 0.0), uv(0.0, 10.0), uv(10.0, 10.0), uv(10.0, 0.0)]).unwrap();
    assert_eq!(morph_rings(&tri_a, &square, 0.5), Err(GeomError::PointCountMismatch));

    // Slerp between antipodes fails loud, never guesses a path.
    assert_eq!(slerp(&uv(0.0, 0.0), &uv(0.0, 180.0), 0.5), Err(GeomError::Antipodal));
}

// ---------------------------------------------- law 5: history coherence

#[test]
fn law05_history_coherence() {
    // A narrated shift: two versions, and the event that tells the story.
    let mut world = two_region_world();
    let hist = world.boundaries.get_mut(&S).unwrap();
    let old = hist.versions[0].1.clone();
    let mut moved = old.clone();
    moved.pts[1] = uv(0.0, 1.0);
    hist.versions = vec![
        (Interval::new(tp(-2000), Some(tp(-1500))).unwrap(), old),
        (Interval::open_from(tp(-1500)), moved),
    ];
    world.events.push(shift_event(S, -1500));
    assert_eq!(validate_history_coherence(&world), vec![]);

    // The narrated world is lawful under EVERY data validator at once.
    let (chronology, gazetteer) = exports("fall-of-samaria", -721, "hazar-enan");
    assert_eq!(validate_all(&world, &chronology, &gazetteer), vec![]);

    // Silence the narrative: the same data with no event is a violation.
    let mut silent = world.clone();
    silent.events.clear();
    assert!(silent.events.is_empty());
    let violations = validate_history_coherence(&silent);
    assert!(
        violations.iter().any(|v| matches!(v, Violation::UnnarratedChange { at, .. } if *at == tp(-1500))),
        "silent border move must be flagged: {violations:?}"
    );

    // Overlapping intervals are incoherent.
    let mut overlapping = two_region_world();
    let hist = overlapping.boundaries.get_mut(&S).unwrap();
    let b = hist.versions[0].1.clone();
    hist.versions = vec![
        (Interval::new(tp(-2000), Some(tp(-1400))).unwrap(), b.clone()),
        (Interval::open_from(tp(-1500)), b),
    ];
    overlapping.events.push(shift_event(S, -1500));
    let violations = validate_history_coherence(&overlapping);
    assert!(violations
        .iter()
        .any(|v| matches!(v, Violation::IncoherentIntervals { .. })));
}

// ------------------------------- law 6: provenance totality + honesty

#[test]
fn law06_provenance_totality_and_honesty() {
    let world = two_region_world();
    assert_eq!(validate_provenance_totality(&world), vec![]);

    let mut anonymous = world.clone();
    anonymous.boundaries.get_mut(&S).unwrap().versions[0].1.provenance = String::new();
    let violations = validate_provenance_totality(&anonymous);
    assert!(violations.iter().any(|v| matches!(v, Violation::EmptyProvenance { .. })));

    // Honesty is unrepresentable to violate: a style drawing Unknown
    // like Line, or a frontier as a crisp stroke, cannot be built.
    let honest = honest_style();
    assert_ne!(
        honest.stroke_for(&EdgeCharacter::Unknown),
        honest.stroke_for(&EdgeCharacter::Line)
    );

    let line = Stroke { color: Rgba(0, 0, 0, 255), width: 1.0, pattern: StrokePattern::Solid };
    let zonal = Stroke { color: Rgba(0, 0, 0, 255), width: 1.0, pattern: StrokePattern::Zonal };
    let strokes = |unknown, frontier| BoundaryStrokes { line, frontier, disputed: line, unknown, way: zonal };
    let rest = (
        Paint { fill: Rgba(0, 0, 0, 0) },
        Paint { fill: Rgba(0, 0, 60, 200) },
        AgeRamp { newest: Paint { fill: Rgba(9, 9, 9, 9) }, oldest: Paint { fill: Rgba(3, 3, 3, 3) } },
        None,
        AgeRamp { newest: Paint { fill: Rgba(0, 0, 0, 0) }, oldest: Paint { fill: Rgba(0, 0, 0, 0) } },
        LabelStyle { color: Rgba(0, 0, 0, 255), halo: Rgba(255, 255, 255, 200), size: 10.0 },
        MarkerStyle { color: Rgba(0, 0, 0, 255), size: 3.0 },
        DeltaEmphasis { before: line, after: line, seam: line },
    );
    assert_eq!(
        Style::new(strokes(line, zonal), rest.0, rest.1, rest.2, rest.3, rest.4, rest.5, rest.6, rest.7).unwrap_err(),
        StyleError::UnknownIndistinctFromLine
    );
    assert_eq!(
        Style::new(strokes(zonal, line), rest.0, rest.1, rest.2, rest.3, rest.4, rest.5, rest.6, rest.7).unwrap_err(),
        StyleError::FrontierNotZonal
    );
}

// ------------------------------------------- law 7: lod monotonicity

#[test]
fn law07_lod_monotonicity() {
    // A wiggly arc: 51 points, latitude oscillating along a meridian run.
    let wiggly: Vec<UnitVec> = (0..=50)
        .map(|i| uv((f64::from(i) * 0.7).sin() * 2.0, f64::from(i)))
        .collect();

    let tolerances = [0.0, 1e-4, 1e-3, 5e-3, 2e-2, 1e-1];
    let mut previous: Option<Vec<UnitVec>> = None;
    for tol in tolerances {
        let simplified = simplify_polyline(&wiggly, Lod(tol));
        // Endpoints always survive.
        assert_eq!(simplified.first(), wiggly.first());
        assert_eq!(simplified.last(), wiggly.last());
        if let Some(prev) = &previous {
            // Coarser never ADDS points…
            assert!(
                simplified.len() <= prev.len(),
                "tol {tol}: {} points after {} at finer tol",
                simplified.len(),
                prev.len()
            );
            // …and keeps a SUBSET of the finer selection, so zooming out
            // never invents geometry.
            assert!(simplified.iter().all(|p| prev.contains(p)));
        }
        previous = Some(simplified);
    }

    // Exact lod is the identity.
    assert_eq!(simplify_polyline(&wiggly, Lod::exact()), wiggly);
}

// ---------------------------------------- law 8: composition algebra

#[test]
fn law08_overlay_and_script_monoids() {
    let scenes =
        vec![Snapshot::empty(), marker_scene(1), marker_scene(2), marker_scene(3)];
    check_monoid(&scenes);

    let scripts = vec![
        TransitionScript::empty(),
        TransitionScript { steps: vec![TransitionStep::FadeIn { region: A }] },
        TransitionScript { steps: vec![TransitionStep::FadeOut { region: B }] },
    ];
    check_monoid(&scripts);
}

// -------------------------------------- law 9: accumulation exactness

/// A toy piecewise-constant scene function: its value changes ONLY at
/// change events. Sampling at events + endpoints is exact; any extra
/// sample between events is inert because identical scenes hash
/// identically and the fold touches each distinct scene once.
#[test]
fn law09_accumulation_exactness() {
    let events = vec![shift_event(S, -1500), shift_event(S, -1200)];
    let scene_at = |t: TimePoint| -> Snapshot {
        let mut sc = marker_scene(0);
        for (i, e) in events.iter().enumerate() {
            if e.at <= t {
                sc = sc.combine(marker_scene(i as u8 + 1));
            }
        }
        sc
    };

    let over = Interval::new(tp(-2000), Some(tp(-1000))).unwrap();
    let exact_times = sample_times(&over, &events);
    assert_eq!(exact_times, vec![tp(-2000), tp(-1500), tp(-1200), tp(-1000)]);

    let exact = accumulate(exact_times.iter().map(|t| scene_at(*t)));

    // Oversampling between events changes NOTHING.
    let mut oversampled_times = exact_times.clone();
    oversampled_times.extend([tp(-1700), tp(-1300), tp(-1100)]);
    oversampled_times.sort();
    let oversampled = accumulate(oversampled_times.iter().map(|t| scene_at(*t)));
    assert_eq!(exact, oversampled);

    // Fold identity: a single-instant accumulation IS its snapshot.
    let instant = Interval::new(tp(-1500), Some(tp(-1500))).unwrap();
    assert_eq!(sample_times(&instant, &events), vec![tp(-1500)]);
    assert_eq!(accumulate([scene_at(tp(-1500))]), scene_at(tp(-1500)));

    // The monoid is closed: an accumulation is a scene and overlays on.
    let composed = exact.combine(marker_scene(9));
    assert!(composed.markers.len() > 0);
}

// ------------------------------------- law 10: selection coherence

/// The selection side of P4: selecting a subject out of a composed
/// scene agrees with composing the selections. The full provider
/// equation — render(subject) == select(render(World)) — is proven
/// against the materializer in phase 3.
#[test]
fn law10_selection_coherence() {
    let style = honest_style();
    let region_scene = |id: RegionId, name: &str, lat: f64| -> Snapshot {
        let mut sc = Snapshot::empty();
        sc.regions.push(StyledRegion {
            region: id,
            outer: vec![Ring::new(vec![uv(lat, 0.0), uv(lat, 5.0), uv(lat + 5.0, 2.5)]).unwrap()],
            holes: vec![],
            paint: style.region_paint(),
            sources: [SourceId::new("historical-source")].into(),
        });
        sc.labels.push(PlacedLabel {
            text: name.to_string(),
            at: uv(lat + 2.0, 2.5),
            subject: LabelSubject::Region(id),
            style: style.label_style(),
        });
        sc.attribution.insert(SourceId::new("historical-source"));
        sc
    };
    let a = region_scene(A, "Westland", 0.0);
    let b = region_scene(B, "Eastland", 20.0);
    let world = a.clone().combine(b.clone());

    // Selecting A out of the world is exactly A's own scene.
    assert_eq!(world.select_region(A), a);
    // Selection distributes over overlay.
    assert_eq!(
        world.select_region(B),
        a.select_region(B).combine(b.select_region(B))
    );
    // Selecting an absent subject yields (attributed) emptiness.
    let ghost = world.select_region(RegionId(ContentHash(404)));
    assert!(ghost.regions.is_empty() && ghost.labels.is_empty());
}

// ------------------------------------- law 11: encoder terminality

/// Grep-enforced: no source file upstream of the encoder boundary names
/// a concrete output format. The determinism half binds each encoder
/// impl as it is written (none exist yet — deliberately).
#[test]
fn law11_encoder_terminality() {
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let format_names =
        ["svg", "geojson", "png", "jpeg", "webgl", "pdf", "raster", "vector tile", "waapi"];
    for entry in std::fs::read_dir(&src).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        if name == "encode.rs" || name == "tests.rs" {
            continue; // the terminal boundary itself, and this test
        }
        let text = std::fs::read_to_string(&path).unwrap().to_lowercase();
        for f in format_names {
            assert!(
                !text.contains(f),
                "{name} names the concrete format {f:?} upstream of the encoder boundary"
            );
        }
    }
}

// --------------------------------------- law 12: Bible preference

fn survey_boundary(place: &str) -> Boundary {
    let verse = |v| BibleLocus::whole(VerseRef { book: 4, chapter: 34, verse: v });
    Boundary {
        pts: vec![uv(31.0, 34.0), uv(33.0, 36.0)],
        character: EdgeCharacter::Line,
        source: BoundarySource::Survey(BorderSurvey {
            verses: LocusRange::new(verse(1), verse(12)).unwrap(),
            waypoints: vec![AtlasPlaceRef(PlaceId::new(place))],
            interpolation: InterpolationMethod::Geodesic,
            provenance: prov(),
        }),
        justification: Justification::default(),
        provenance: prov(),
    }
}

fn exports(known_event: &str, at: i32, known_place: &str) -> (ChronologyExport, GazetteerExport) {
    let placement = ResolvedPlacement {
        date: ResolvedDate { from: tp(at), to: tp(at) },
        seq: SeqKey(0),
        basis: PlacementBasis::Traditional,
    };
    let chronology = ChronologyExport {
        atlas_root: ContentHash(555),
        placements: BTreeMap::from([(EventId::new(known_event), placement)]),
        spans: Vec::new(),
    };
    let gazetteer = GazetteerExport {
        atlas_root: ContentHash(555),
        places: BTreeMap::from([(
            PlaceId::new(known_place),
            GazetteerEntry { canonical_name: known_place.to_string(), position: uv(33.0, 36.0), aliases: Vec::new(), provenance: None, attestations: Vec::new() },
        )]),
    };
    (chronology, gazetteer)
}

#[test]
fn law12_bible_preference() {
    let (chronology, gazetteer) = exports("fall-of-samaria", -721, "hazar-enan");
    let driven = |at: i32, event: &str| ChangeEvent {
        at: tp(at),
        kind: ChangeKind::Fall { region: A },
        driver: Some(AtlasEventRef { event: EventId::new(event), atlas_root: ContentHash(555) }),
        justification: Justification::default(),
        provenance: prov(),
    };

    // (a) A driven event carrying the atlas's own date is lawful…
    let mut world = two_region_world();
    world.events.push(driven(-721, "fall-of-samaria"));
    assert_eq!(validate_bible_preference(&world, &chronology, &gazetteer), vec![]);

    // …re-dating it is not.
    let mut redated = two_region_world();
    redated.events.push(driven(-722, "fall-of-samaria"));
    let violations = validate_bible_preference(&redated, &chronology, &gazetteer);
    assert!(violations.iter().any(|v| matches!(v, Violation::DriverDateMismatch { .. })));

    // …and naming an event the atlas doesn't know is not.
    let mut unknown = two_region_world();
    unknown.events.push(driven(-721, "no-such-event"));
    let violations = validate_bible_preference(&unknown, &chronology, &gazetteer);
    assert!(violations.iter().any(|v| matches!(v, Violation::DriverUnknownToAtlas { .. })));

    // (b) An Imported version overlapping a Survey's validity on the
    // same arc silently overrides Scripture: violation.
    let mut overridden = two_region_world();
    let hist = overridden.boundaries.get_mut(&S).unwrap();
    let imported = hist.versions[0].1.clone();
    hist.versions = vec![
        (Interval::open_from(tp(-1400)), survey_boundary("hazar-enan")),
        (Interval::open_from(tp(-1300)), imported.clone()),
    ];
    let violations = validate_bible_preference(&overridden, &chronology, &gazetteer);
    assert!(violations.iter().any(|v| matches!(v, Violation::ImportedOverridesSurvey { boundary } if *boundary == S)));

    // A closed survey followed by an import (a narrated succession) is
    // lawful — Scripture speaks for its interval, scholarship after.
    let mut succeeded = two_region_world();
    let hist = succeeded.boundaries.get_mut(&S).unwrap();
    hist.versions = vec![
        (Interval::new(tp(-1400), Some(tp(-1300))).unwrap(), survey_boundary("hazar-enan")),
        (Interval::open_from(tp(-1300)), imported),
    ];
    succeeded.events.push(shift_event(S, -1300));
    assert_eq!(validate_bible_preference(&succeeded, &chronology, &gazetteer), vec![]);

    // (c) A waypoint the gazetteer doesn't hold fails referential
    // integrity across the repos.
    let mut unmoored = two_region_world();
    unmoored.boundaries.get_mut(&S).unwrap().versions[0].1 = survey_boundary("atlantis");
    let violations = validate_bible_preference(&unmoored, &chronology, &gazetteer);
    assert!(violations.iter().any(|v| matches!(v, Violation::UnresolvedWaypoint { .. })));
}

// ---------------------------------------- contract C6: version drift

#[test]
fn contract_c6_stale_pin_fails_loud() {
    let pin = AtlasPin { version_root: ContentHash(555) };
    assert!(!pin.is_stale(ContentHash(555)));
    assert!(pin.is_stale(ContentHash(556)));
}
