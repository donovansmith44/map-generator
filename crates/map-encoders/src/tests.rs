//! Law 11's determinism half, per backend: same scene + same encoder
//! config -> same bytes. (The terminality half — no format names
//! upstream — is enforced inside map-types by its grep test.)

use std::collections::BTreeSet;

use atlas_graph_types::covenant::SourceId;
use map_types::scene::{LabelSubject, StyledMarker};
use map_types::style::*;
use map_types::{
    Monoid, PlacedLabel, Ring, SceneEncoder, Snapshot, StyledBoundary, StyledRegion, UnitVec,
};

use crate::{GeoJsonEncoder, Projection, SvgEncoder};

fn uv(lat: f64, lon: f64) -> UnitVec {
    UnitVec::from_lat_lon_deg(lat, lon)
}

fn sample_scene() -> Snapshot {
    let mut s = Snapshot::empty();
    let sources: BTreeSet<SourceId> = [SourceId::new("historical-basemaps")].into();
    s.regions.push(StyledRegion {
        region: map_types::RegionId(atlas_graph_types::covenant::ContentHash(1)),
        outer: vec![Ring::new(vec![uv(0.0, 0.0), uv(0.0, 10.0), uv(8.0, 5.0)]).unwrap()],
        holes: vec![],
        paint: Paint { fill: Rgba(210, 190, 150, 255) },
        sources: sources.clone(),
    });
    s.boundaries.push(StyledBoundary {
        boundary: map_types::BoundaryId(atlas_graph_types::covenant::ContentHash(2)),
        pts: vec![uv(0.0, 0.0), uv(0.0, 10.0)],
        stroke: Stroke { color: Rgba(90, 60, 40, 255), width: 1.5, pattern: StrokePattern::Dashed },
        sources: sources.clone(),
    });
    s.markers.push(StyledMarker {
        at: uv(4.0, 5.0),
        style: MarkerStyle { color: Rgba(20, 20, 20, 255), size: 3.0 },
    });
    s.labels.push(PlacedLabel {
        text: "Judah & <friends>".to_string(),
        at: uv(3.0, 5.0),
        subject: LabelSubject::Free,
        style: LabelStyle { color: Rgba(10, 10, 10, 255), halo: Rgba(245, 240, 225, 220), size: 12.0 },
    });
    s.attribution = sources;
    s
}

#[test]
fn svg_is_deterministic_and_wellformed() {
    let scene = sample_scene();
    let a = SvgEncoder::default().encode(&scene).unwrap();
    let b = SvgEncoder::default().encode(&scene).unwrap();
    assert_eq!(a, b);
    assert!(a.starts_with("<svg") && a.ends_with("</svg>"));
    // Markup-significant characters in labels are escaped.
    assert!(a.contains("Judah &amp; &lt;friends&gt;"));
    // Licensing rides the artifact.
    assert!(a.contains("historical-basemaps"));
    // Config participates in the bytes.
    let wider = SvgEncoder { width: 2000.0, ..SvgEncoder::default() }.encode(&scene).unwrap();
    assert_ne!(a, wider);
}

#[test]
fn globe_clips_the_far_hemisphere() {
    // Content on both sides of the planet, viewed from over the near
    // side: the far label must not appear; the near one must.
    let mut scene = sample_scene();
    scene.labels.push(PlacedLabel {
        text: "ANTIPODEAN".to_string(),
        at: uv(-3.0, -175.0),
        subject: LabelSubject::Free,
        style: LabelStyle { color: Rgba(10, 10, 10, 255), halo: Rgba(245, 240, 225, 220), size: 12.0 },
    });
    let enc = SvgEncoder {
        projection: Projection::Globe { center: Some((3.0, 5.0)), zoom: None },
        ..SvgEncoder::default()
    };
    let out = enc.encode(&scene).unwrap();
    assert!(out.contains("Judah"));
    assert!(!out.contains("ANTIPODEAN"), "far-side content must be clipped");
    // With content wrapping the limb, the full disc (limb circle) shows.
    assert!(out.contains("<circle"));
    // The resolved view rides the artifact for interactive consumers.
    assert!(out.contains("data-clat=\"3.000\"") && out.contains("data-zoom=\"90.000\""));
}

#[test]
fn explicit_view_overrides_autofit_deterministically() {
    let scene = sample_scene();
    let enc = SvgEncoder {
        projection: Projection::Globe { center: Some((4.0, 5.0)), zoom: Some(20.0) },
        ..SvgEncoder::default()
    };
    let a = enc.encode(&scene).unwrap();
    assert_eq!(a, enc.encode(&scene).unwrap());
    assert!(a.contains("data-zoom=\"20.000\""));
    // A different view is a different artifact.
    let closer = SvgEncoder {
        projection: Projection::Globe { center: Some((4.0, 5.0)), zoom: Some(10.0) },
        ..SvgEncoder::default()
    };
    assert_ne!(a, closer.encode(&scene).unwrap());
    // Regions carry their id for click-to-select consumers.
    assert!(a.contains("data-region=\"0000000000000001\""));
}

#[test]
fn globe_zooms_to_a_regional_slice() {
    // Compact content: the view zooms in, so the limb circle is
    // offscreen (no full-disc), but the graticule still curves through.
    let scene = sample_scene();
    let out = SvgEncoder::default().encode(&scene).unwrap();
    let disc = out.matches("<circle").count();
    // The only circle is the marker — no limb disc at slice zoom.
    assert_eq!(disc, 1);
    assert!(out.contains("stroke-opacity=\"0.22\""), "graticule present");
}

#[test]
fn flat_projection_still_available() {
    let scene = sample_scene();
    let flat = SvgEncoder { projection: Projection::Flat, ..SvgEncoder::default() };
    let a = flat.encode(&scene).unwrap();
    assert_eq!(a, flat.encode(&scene).unwrap());
    assert!(a.starts_with("<svg"));
    assert_ne!(a, SvgEncoder::default().encode(&scene).unwrap());
}

#[test]
fn geojson_is_deterministic_and_parses() {
    let scene = sample_scene();
    let a = GeoJsonEncoder.encode(&scene).unwrap();
    let b = GeoJsonEncoder.encode(&scene).unwrap();
    assert_eq!(a, b);
    let v: serde_json::Value = serde_json::from_str(&a).unwrap();
    assert_eq!(v["type"], "FeatureCollection");
    assert_eq!(v["features"].as_array().unwrap().len(), 4);
    assert_eq!(v["attribution"][0], "historical-basemaps");
}

#[test]
fn geodesics_curve_not_chord() {
    // A 60-degree border segment must render as many small steps, not
    // one straight chord across the globe.
    let mut scene = Snapshot::empty();
    scene.boundaries.push(StyledBoundary {
        boundary: map_types::BoundaryId(atlas_graph_types::covenant::ContentHash(9)),
        pts: vec![uv(10.0, 0.0), uv(10.0, 60.0)],
        stroke: Stroke { color: Rgba(0, 0, 0, 255), width: 1.0, pattern: StrokePattern::Solid },
        sources: BTreeSet::new(),
    });
    let out = SvgEncoder {
        projection: Projection::Globe { center: Some((10.0, 30.0)), zoom: Some(45.0) },
        smooth: false, // isolate the densification from the spline
        ..SvgEncoder::default()
    }
    .encode(&scene)
    .unwrap();
    // The boundary path (not the graticule) carries the subdivision:
    // find the path drawn with the boundary's stroke.
    let seg = out.split("stroke=\"rgb(0,0,0)\"").next().unwrap();
    let boundary_path = seg.rsplit("<path d=\"").next().unwrap();
    let steps = boundary_path.matches('L').count();
    assert!(steps >= 8, "60 degrees as {steps} steps is a chord, not a curve");
}

#[test]
fn labels_fit_their_territory_and_never_collide() {
    // A label longer than its tiny territory is DROPPED, not smeared
    // across the map.
    let mut scene = sample_scene();
    let tiny = map_types::RegionId(atlas_graph_types::covenant::ContentHash(7));
    scene.regions.push(StyledRegion {
        region: tiny,
        outer: vec![Ring::new(vec![uv(20.0, 20.0), uv(20.0, 20.1), uv(20.1, 20.05)]).unwrap()],
        holes: vec![],
        paint: Paint { fill: Rgba(210, 190, 150, 255) },
        sources: BTreeSet::new(),
    });
    scene.labels.push(PlacedLabel {
        text: "AN IMPOSSIBLY LONG NAME FOR A TINY PLACE".to_string(),
        at: uv(20.03, 20.05),
        subject: LabelSubject::Region(tiny),
        style: LabelStyle { color: Rgba(0, 0, 0, 255), halo: Rgba(255, 255, 255, 200), size: 12.0 },
    });
    let out = SvgEncoder::default().encode(&scene).unwrap();
    assert!(!out.contains("IMPOSSIBLY"), "a label that cannot fit is dropped");

    // Two labels at the same anchor never overlap: the second nudges
    // or yields. Same text twice -> if both survive, different y.
    let mut scene = sample_scene();
    scene.labels.push(scene.labels[0].clone());
    let out = SvgEncoder::default().encode(&scene).unwrap();
    let ys: Vec<&str> = out
        .match_indices("<text x=")
        .map(|(i, _)| {
            let rest = &out[i..];
            let y0 = rest.find("y=\"").unwrap() + 3;
            &rest[y0..y0 + rest[y0..].find('"').unwrap()]
        })
        .collect();
    if ys.len() == 2 {
        assert_ne!(ys[0], ys[1], "colliding labels must separate");
    }
    // Halos ride every label.
    assert!(out.contains("paint-order=\"stroke\""));
}

#[test]
fn empty_scene_encodes() {
    let empty = Snapshot::empty();
    assert!(SvgEncoder::default().encode(&empty).is_ok());
    assert!(GeoJsonEncoder.encode(&empty).is_ok());
}

// -------------------------------------------------- transition encoder

/// The transition backend under law 11: identity encodes to an empty
/// step list, every semantic verb passes through untranslated, and the
/// bytes are deterministic.
#[test]
fn transition_json_is_faithful_and_deterministic() {
    use crate::JsonTransitionEncoder;
    use atlas_graph_types::covenant::ContentHash;
    use map_types::{BoundaryId, RegionId, TransitionEncoder, TransitionScript, TransitionStep};

    let empty = JsonTransitionEncoder.encode_transition(&TransitionScript::empty()).unwrap();
    assert_eq!(empty, r#"{"steps":[]}"#);

    let rid = |n| RegionId(ContentHash(n));
    let script = TransitionScript {
        steps: vec![
            TransitionStep::Morph {
                boundary: BoundaryId(ContentHash(7)),
                from_pts: vec![uv(31.0, 35.0), uv(32.0, 35.0)],
                to_pts: vec![uv(31.0, 35.5), uv(32.0, 35.5)],
            },
            TransitionStep::FadeIn { region: rid(1) },
            TransitionStep::FadeOut { region: rid(2) },
            TransitionStep::SplitAlong {
                parent: rid(3),
                seam: vec![uv(31.0, 35.0), uv(32.0, 35.0)],
                children: vec![rid(4), rid(5)],
            },
            TransitionStep::MergeAcross { parents: vec![rid(4), rid(5)], child: rid(3) },
        ],
    };
    let a = JsonTransitionEncoder.encode_transition(&script).unwrap();
    let b = JsonTransitionEncoder.encode_transition(&script).unwrap();
    assert_eq!(a, b, "law 11: same script, same bytes");

    let doc: serde_json::Value = serde_json::from_str(&a).unwrap();
    let kinds: Vec<&str> = doc["steps"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["kind"].as_str().unwrap())
        .collect();
    assert_eq!(kinds, ["morph", "fade_in", "fade_out", "split", "merge"]);
    assert_eq!(doc["steps"][0]["boundary"], "0000000000000007");
    assert_eq!(doc["steps"][3]["children"].as_array().unwrap().len(), 2);
    assert_eq!(doc["steps"][4]["child"], "0000000000000003");
}

// ------------------------------------------------------ viewport culling

/// Performance honesty (the laggy-drag fix): paths the page cannot
/// show are not emitted. Off-page geometry is culled, a ring that
/// swallows the page survives (its FILL is the page), and on-page
/// geometry is untouched.
#[test]
fn globe_culls_offscreen_but_keeps_swallowing_fills() {
    use atlas_graph_types::covenant::ContentHash;
    use map_types::RegionId;

    let region = |n: u64, ring: Vec<UnitVec>| StyledRegion {
        region: RegionId(ContentHash(n)),
        outer: vec![Ring::new(ring).unwrap()],
        holes: vec![],
        paint: Paint { fill: Rgba(10 + n as u8, 20, 30, 200) },
        sources: [SourceId::new("test")].into(),
    };
    let square = |lat0: f64, lon0: f64, d: f64| {
        vec![
            uv(lat0, lon0),
            uv(lat0, lon0 + d),
            uv(lat0 + d, lon0 + d),
            uv(lat0 + d, lon0),
        ]
    };
    let scene = Snapshot {
        regions: vec![
            region(1, square(31.0, 35.0, 2.0)),  // on page (view center)
            region(2, square(31.0, 75.0, 2.0)),  // same hemisphere, far off page
            region(3, square(11.0, 15.0, 42.0)), // swallows the 5-degree view
        ],
        boundaries: vec![],
        markers: vec![],
        labels: vec![],
        attribution: [SourceId::new("test")].into(),
    };
    let enc = SvgEncoder {
        width: 800.0,
        padding: 16.0,
        projection: Projection::Globe { center: Some((32.0, 36.0)), zoom: Some(5.0) },
        smooth: false,
    };
    let svg = enc.encode(&scene).unwrap();
    assert!(svg.contains("data-region=\"0000000000000001\""), "on-page region emitted");
    assert!(
        !svg.contains("data-region=\"0000000000000002\""),
        "off-page region culled from the bytes"
    );
    assert!(
        svg.contains("data-region=\"0000000000000003\""),
        "a ring swallowing the page still fills it"
    );
    // Whole-hemisphere view: nothing in this scene may be culled.
    let wide = SvgEncoder {
        projection: Projection::Globe { center: Some((32.0, 36.0)), zoom: Some(90.0) },
        ..enc
    };
    let svg = wide.encode(&scene).unwrap();
    for n in 1..=3 {
        assert!(svg.contains(&format!("data-region=\"{n:016x}\"")), "region {n} at zoom 90");
    }
}

/// A swallowing ring keeps the fill but must not ship its full
/// off-page detail: only the winding matters out there. And a stroke
/// gains nothing from the swallow rule — an entirely off-page
/// boundary paints no visible pixel, whatever its bbox spans.
#[test]
fn swallowing_geometry_ships_thin() {
    use atlas_graph_types::covenant::ContentHash;
    use map_types::RegionId;

    // A dense ring circling the view center 40 degrees out: 1440
    // points, every one of them far off a 5-degree page.
    let circle: Vec<UnitVec> = (0..1440)
        .map(|i| {
            let a = i as f64 / 1440.0 * std::f64::consts::TAU;
            uv(32.0 + 40.0 * a.cos(), 36.0 + 40.0 * a.sin())
        })
        .collect();
    let scene = Snapshot {
        regions: vec![StyledRegion {
            region: RegionId(ContentHash(1)),
            outer: vec![Ring::new(circle.clone()).unwrap()],
            holes: vec![],
            paint: Paint { fill: Rgba(10, 20, 30, 200) },
            sources: [SourceId::new("test")].into(),
        }],
        boundaries: vec![StyledBoundary {
            boundary: map_types::BoundaryId(ContentHash(2)),
            pts: circle,
            stroke: Stroke { color: Rgba(0, 0, 0, 255), width: 1.0, pattern: StrokePattern::Solid },
            sources: [SourceId::new("test")].into(),
        }],
        markers: vec![],
        labels: vec![],
        attribution: [SourceId::new("test")].into(),
    };
    let enc = SvgEncoder {
        width: 800.0,
        padding: 16.0,
        projection: Projection::Globe { center: Some((32.0, 36.0)), zoom: Some(5.0) },
        smooth: false,
    };
    let svg = enc.encode(&scene).unwrap();
    let start = svg.find("data-region=\"0000000000000001\"").expect("swallowing fill survives");
    let path = &svg[start..svg[start..].find("/>").map(|e| start + e).unwrap()];
    let pairs = path.matches('L').count();
    assert!(pairs < 400, "off-page ring detail must be thinned, got ~{pairs} segments");
    assert!(!svg.contains("stroke=\"rgb(0,0,0)\""), "an all-off-page stroke paints nothing");
}
