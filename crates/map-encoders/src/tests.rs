//! Law 11's determinism half, per backend: same scene + same encoder
//! config -> same bytes. (The terminality half — no format names
//! upstream — is enforced inside map-types by its grep test.)

use std::collections::BTreeSet;

use atlas_graph_types::covenant::SourceId;
use map_types::scene::{LabelSubject, StyledMarker};

fn test_voice() -> map_types::style::TypeVoice {
    map_types::style::TypeVoice {
        family: "'Segoe UI', ui-sans-serif, system-ui, sans-serif",
        weight: 600,
        italic: false,
        uppercase: false,
        tracking_em: 0.0,
        advance_em: 0.62,
    }
}
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
        entity: None,
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
        sources: Default::default(),
        place: None,
    });
    s.labels.push(PlacedLabel {
        text: "Judah & <friends>".to_string(),
        at: uv(3.0, 5.0),
        subject: LabelSubject::Free,
        style: LabelStyle { color: Rgba(10, 10, 10, 255), halo: Rgba(245, 240, 225, 220), size: 12.0 },
        face: map_types::scene::LabelFace::Place,
        voice: test_voice(),
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
        face: map_types::scene::LabelFace::Place,
        voice: test_voice(),
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
fn nothing_unseen_speaks() {
    // A name appears only once the reader can resolve the thing it
    // names: a pond spanning less than the legibility floor stays
    // silent at a wide camera and speaks when the camera closes in.
    fn lake(id: u64, lat: f64, lon: f64, r: f64) -> StyledRegion {
        let ring = Ring::new(vec![
            uv(lat - r, lon - r),
            uv(lat - r, lon + r),
            uv(lat + r, lon + r),
            uv(lat + r, lon - r),
        ])
        .unwrap();
        StyledRegion {
            region: map_types::RegionId(atlas_graph_types::covenant::ContentHash(id)),
            entity: None,
            outer: vec![ring],
            holes: vec![],
            paint: Paint { fill: Rgba(120, 150, 200, 255) },
            sources: Default::default(),
        }
    }
    fn name(id: u64, text: &str, lat: f64, lon: f64) -> PlacedLabel {
        PlacedLabel {
            text: text.to_string(),
            at: uv(lat, lon),
            subject: LabelSubject::Region(map_types::RegionId(
                atlas_graph_types::covenant::ContentHash(id),
            )),
            style: LabelStyle {
                color: Rgba(10, 10, 10, 255),
                halo: Rgba(245, 240, 225, 220),
                size: 12.0,
            },
            face: map_types::scene::LabelFace::Water,
            voice: test_voice(),
        }
    }
    let mut scene = Snapshot::empty();
    scene.regions.push(lake(11, 0.0, 0.0, 5.0)); // spans ~10 degrees
    scene.regions.push(lake(12, 8.0, 8.0, 0.02)); // a speck
    scene.labels.push(name(11, "GREATWATER", 0.0, 0.0));
    scene.labels.push(name(12, "SPECKPOND", 8.0, 8.0));
    let wide = SvgEncoder {
        projection: Projection::Globe { center: Some((4.0, 4.0)), zoom: Some(45.0) },
        ..SvgEncoder::default()
    }
    .encode(&scene)
    .unwrap();
    assert!(wide.contains("GREATWATER"), "a resolvable subject speaks");
    assert!(!wide.contains("SPECKPOND"), "a sub-pixel subject stays silent");
    let close = SvgEncoder {
        projection: Projection::Globe { center: Some((8.0, 8.0)), zoom: Some(0.5) },
        ..SvgEncoder::default()
    }
    .encode(&scene)
    .unwrap();
    assert!(close.contains("SPECKPOND"), "the name returns as its subject resolves");
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
    let flat = SvgEncoder {
        projection: Projection::Flat { center: None, zoom: None },
        ..SvgEncoder::default()
    };
    let a = flat.encode(&scene).unwrap();
    assert_eq!(a, flat.encode(&scene).unwrap());
    assert!(a.starts_with("<svg"));
    assert_ne!(a, SvgEncoder::default().encode(&scene).unwrap());
}

/// Flat gets a camera too: center+zoom crop the map to a window, cull
/// what the window cannot show, and stamp the resolved view on the
/// artifact so the workbench can pan and zoom it like the globe.
#[test]
fn flat_zooms_to_a_window() {
    use atlas_graph_types::covenant::ContentHash;
    use map_types::RegionId;
    let region = |n: u64, lat: f64, lon: f64| StyledRegion {
        region: RegionId(ContentHash(n)),
        entity: None,
        outer: vec![Ring::new(vec![
            uv(lat, lon),
            uv(lat, lon + 2.0),
            uv(lat + 2.0, lon + 1.0),
        ])
        .unwrap()],
        holes: vec![],
        paint: Paint { fill: Rgba(210, 190, 150, 255) },
        sources: [SourceId::new("test")].into(),
    };
    let scene = Snapshot {
        regions: vec![region(1, 31.0, 35.0), region(2, 31.0, 155.0)],
        boundaries: vec![],
        markers: vec![],
        labels: vec![],
        attribution: [SourceId::new("test")].into(),
    };
    let windowed = SvgEncoder {
        projection: Projection::Flat { center: Some((32.0, 36.0)), zoom: Some(5.0) },
        ..SvgEncoder::default()
    };
    let svg = windowed.encode(&scene).unwrap();
    assert!(svg.contains("data-region=\"0000000000000001\""), "in-window region kept");
    assert!(!svg.contains("data-region=\"0000000000000002\""), "far region culled");
    assert!(svg.contains("data-clat=\"32.000\"") && svg.contains("data-zoom=\"5.000\""));
    // The whole-world flat still carries a view for the navigator.
    let world = SvgEncoder {
        projection: Projection::Flat { center: None, zoom: None },
        ..SvgEncoder::default()
    }
    .encode(&scene)
    .unwrap();
    assert!(world.contains("data-zoom="), "unwindowed flat still reports its view");
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
        entity: None,
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
        face: map_types::scene::LabelFace::Place,
        voice: test_voice(),
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
        entity: None,
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
            entity: None,
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

/// A station marker is CLICKABLE: it carries its place id on the
/// element and a generous transparent hit target around the dot.
#[test]
fn markers_with_places_are_clickable() {
    use map_types::scene::StyledMarker;
    let mut scene = Snapshot::empty();
    scene.markers.push(StyledMarker {
        at: uv(37.94, 27.34),
        style: MarkerStyle { color: Rgba(20, 20, 20, 255), size: 3.0 },
        sources: Default::default(),
        place: Some(map_types::AtlasPlaceRef(
            atlas_graph_types::covenant::PlaceId::new("place:Ephesus".to_string()),
        )),
    });
    scene.attribution.insert(SourceId::new("test"));
    let svg = SvgEncoder {
        projection: Projection::Globe { center: Some((37.94, 27.34)), zoom: Some(5.0) },
        ..SvgEncoder::default()
    }
    .encode(&scene)
    .unwrap();
    assert!(svg.contains("data-place=\"place:Ephesus\""), "place id rides the marker");
    assert!(svg.contains("fill=\"transparent\""), "a finger-sized hit target wraps the dot");
}

/// The encoder renders EXACTLY the voice the label carries — family,
/// weight, posture, case, tracking all come from style data, so
/// restyling type never touches rendering code.
#[test]
fn label_wears_its_declared_voice() {
    use map_types::PlacedLabel;
    use map_types::style::{LabelStyle, Rgba};
    let mut scene = Snapshot::empty();
    scene.labels.push(PlacedLabel {
        text: "philistia".to_string(),
        at: UnitVec::from_lat_lon_deg(31.5, 34.6),
        subject: LabelSubject::Free,
        style: LabelStyle { color: Rgba(10, 10, 10, 255), halo: Rgba(240, 240, 240, 220), size: 16.0 },
        face: map_types::scene::LabelFace::Territory,
        voice: map_types::style::TypeVoice {
            family: "TestFace, serif",
            weight: 512,
            italic: true,
            uppercase: true,
            tracking_em: 0.25,
            advance_em: 0.9,
        },
    });
    let svg = SvgEncoder::default().encode(&scene).unwrap();
    assert!(svg.contains("PHILISTIA"), "uppercase comes from the voice");
    assert!(!svg.contains(">philistia<"), "the lowercase text never leaks");
    assert!(svg.contains("TestFace, serif"), "family comes from the voice");
    assert!(svg.contains("font-weight=\"512\""), "weight comes from the voice");
    assert!(svg.contains("font-style=\"italic\""), "posture comes from the voice");
    assert!(svg.contains("letter-spacing=\"4.00\""), "tracking = size x tracking_em");
}

/// THE COMPOSABILITY LAW: for a fixed (camera, width), every artifact
/// — scaffold, piece, whole scene — shares the same projection frame:
/// identical viewBox and identical data-clat/clon/zoom stamps, so
/// stacked piece-SVGs align geometrically by construction. And every
/// region keeps its addressability: the entity string the API speaks
/// rides the path as data-entity.
#[test]
fn fixed_camera_frames_identically_and_pieces_stay_addressable() {
    let camera = Projection::Globe { center: Some((32.0, 35.3)), zoom: Some(1.5) };
    let enc = || SvgEncoder { projection: camera, width: 900.0, ..SvgEncoder::default() };
    let region = |name: &str, lat: f64| StyledRegion {
        region: map_types::RegionId(atlas_graph_types::covenant::ContentHash(lat as u64 + 7)),
        entity: Some(name.to_string()),
        outer: vec![Ring::new(vec![
            UnitVec::from_lat_lon_deg(lat, 35.0),
            UnitVec::from_lat_lon_deg(lat, 35.6),
            UnitVec::from_lat_lon_deg(lat + 0.4, 35.6),
            UnitVec::from_lat_lon_deg(lat + 0.4, 35.0),
        ])
        .unwrap()],
        holes: Vec::new(),
        paint: map_types::style::Paint { fill: map_types::style::Rgba(10, 120, 30, 255) },
        sources: Default::default(),
    };
    let mut whole = Snapshot::empty();
    whole.regions.push(region("partition:judah", 31.6));
    whole.regions.push(region("partition:ephraim", 32.1));
    let mut piece = Snapshot::empty();
    piece.regions.push(region("partition:judah", 31.6));
    let empty = Snapshot::empty();

    let frame = |svg: &str| {
        let grab = |attr: &str| {
            let i = svg.find(attr).unwrap_or_else(|| panic!("{attr} stamped"));
            svg[i..].split('"').nth(1).unwrap().to_string()
        };
        (grab("viewBox="), grab("data-clat="), grab("data-clon="), grab("data-zoom="))
    };
    let a = enc().encode(&whole).unwrap();
    let b = enc().encode(&piece).unwrap();
    let c = enc().encode(&empty).unwrap();
    assert_eq!(frame(&a), frame(&b), "a piece frames exactly like the whole");
    assert_eq!(frame(&a), frame(&c), "even an empty scaffold frames identically");
    assert!(b.contains("data-entity=\"partition:judah\""), "the piece stays addressable");
    assert!(
        a.contains("data-entity=\"partition:ephraim\""),
        "every region carries the name the API speaks"
    );
}

/// THE FLAT PLATE'S TOPOLOGY GUARD: a water body larger than the
/// window, with a land hole inside the view, must keep its hole —
/// culling ring chunks broke even-odd topology and flooded the page
/// (the world ocean, observed); clipping keeps every ring closed.
#[test]
fn flat_window_keeps_holes_of_oversized_rings() {
    let ring = |pts: Vec<(f64, f64)>| {
        Ring::new(pts.into_iter().map(|(la, lo)| UnitVec::from_lat_lon_deg(la, lo)).collect())
            .unwrap()
    };
    // an ocean far larger than the window…
    let ocean = ring(vec![(0.0, 0.0), (0.0, 60.0), (50.0, 60.0), (50.0, 0.0)]);
    // …with a small island hole inside the camera window
    let island = ring(vec![(31.0, 34.0), (31.0, 36.0), (33.0, 36.0), (33.0, 34.0)]);
    let mut scene = Snapshot::empty();
    scene.regions.push(StyledRegion {
        region: map_types::RegionId(atlas_graph_types::covenant::ContentHash(9)),
        entity: Some("sea".into()),
        outer: vec![ocean],
        holes: vec![island],
        paint: map_types::style::Paint { fill: map_types::style::Rgba(10, 20, 200, 255) },
        sources: Default::default(),
    });
    let svg = SvgEncoder {
        projection: Projection::Flat { center: Some((32.0, 35.0)), zoom: Some(2.0) },
        width: 800.0,
        smooth: false,
        ..SvgEncoder::default()
    }
    .encode(&scene)
    .unwrap();
    // the region's path must contain TWO subpaths (outer + hole): two
    // moveto commands — a flooded page would have one
    let d = svg.split("d=\"").nth(1).unwrap().split('"').next().unwrap();
    let movetos = d.matches('M').count();
    assert!(movetos >= 2, "the island hole survives the window: {movetos} subpaths");
}

/// THE STANDARD PARALLEL: at the camera's latitude, one degree of
/// longitude renders cos(lat) as wide as a degree of latitude — a
/// square league is square, not 18% fat.
#[test]
fn flat_projection_holds_its_standard_parallel() {
    let mut scene = Snapshot::empty();
    scene.markers.push(map_types::scene::StyledMarker {
        at: UnitVec::from_lat_lon_deg(32.0, 35.0),
        style: map_types::style::MarkerStyle {
            color: map_types::style::Rgba(0, 0, 0, 255),
            size: 2.0,
        },
        sources: Default::default(),
        place: None,
    });
    let enc = SvgEncoder {
        projection: Projection::Flat { center: Some((32.0, 35.0)), zoom: Some(2.0) },
        width: 800.0,
        smooth: false,
        ..SvgEncoder::default()
    };
    let svg = enc.encode(&scene).unwrap();
    // read the resolved view: span check via data attributes
    let grab = |attr: &str| -> f64 {
        svg.split(attr).nth(1).unwrap().split('"').nth(1).unwrap().parse().unwrap()
    };
    assert!((grab("data-clat=") - 32.0).abs() < 1e-6);
    assert!((grab("data-clon=") - 35.0).abs() < 1e-6);
    // place two probes 1 deg apart in lat and in cos-corrected lon:
    // their pixel distances must match (the projection is measured
    // through the public artifact, not internals)
    let mut probe = Snapshot::empty();
    for (la, lo) in [(32.0, 35.0), (33.0, 35.0), (32.0, 35.0 + 1.0 / 32f64.to_radians().cos())] {
        probe.markers.push(map_types::scene::StyledMarker {
            at: UnitVec::from_lat_lon_deg(la, lo),
            style: map_types::style::MarkerStyle {
                color: map_types::style::Rgba(0, 0, 0, 255),
                size: 2.0,
            },
            sources: Default::default(),
            place: None,
        });
    }
    let svg = enc.encode(&probe).unwrap();
    let mut pts: Vec<(f64, f64)> = svg
        .match_indices("<circle")
        .map(|(i, _)| {
            let seg = &svg[i..];
            let g = |a: &str| -> f64 {
                seg.split(a).nth(1).unwrap().split('"').nth(1).unwrap().parse().unwrap()
            };
            (g("cx="), g("cy="))
        })
        .collect();
    pts.truncate(3);
    let d_lat = ((pts[0].0 - pts[1].0).powi(2) + (pts[0].1 - pts[1].1).powi(2)).sqrt();
    let d_lon = ((pts[0].0 - pts[2].0).powi(2) + (pts[0].1 - pts[2].1).powi(2)).sqrt();
    assert!(
        (d_lat - d_lon).abs() / d_lat < 0.02,
        "a square league is square: lat step {d_lat:.1}px vs lon step {d_lon:.1}px"
    );
}

/// THE CORRESPONDENCE LAW: the same camera shows the same ground on
/// either chart. Both projections stamp the same resolved view, and a
/// step of one ground-degree eastward lands the same pixel distance
/// on the flat plate as on the globe (to first order at the center).
#[test]
fn flat_and_globe_correspond_under_one_camera()  {
    let camera = ((32.0, 35.0), 2.0);
    let probe = |projection: Projection| -> (String, f64) {
        let mut scene = Snapshot::empty();
        let kx = 32f64.to_radians().cos();
        for (la, lo) in [(32.0, 35.0), (32.0, 35.0 + 1.0 / kx)] {
            scene.markers.push(map_types::scene::StyledMarker {
                at: UnitVec::from_lat_lon_deg(la, lo),
                style: map_types::style::MarkerStyle {
                    color: map_types::style::Rgba(0, 0, 0, 255),
                    size: 2.0,
                },
                sources: Default::default(),
                place: None,
            });
        }
        let svg = SvgEncoder { projection, width: 800.0, smooth: false, ..SvgEncoder::default() }
            .encode(&scene)
            .unwrap();
        let stamps: Vec<String> = ["data-clat=", "data-clon=", "data-zoom="]
            .iter()
            .map(|a| svg.split(a).nth(1).unwrap().split('"').nth(1).unwrap().to_string())
            .collect();
        let pts: Vec<(f64, f64)> = svg
            .match_indices("<circle")
            .take(2)
            .map(|(i, _)| {
                let seg = &svg[i..];
                let g = |a: &str| -> f64 {
                    seg.split(a).nth(1).unwrap().split('"').nth(1).unwrap().parse().unwrap()
                };
                (g("cx="), g("cy="))
            })
            .collect();
        let d = ((pts[0].0 - pts[1].0).powi(2) + (pts[0].1 - pts[1].1).powi(2)).sqrt();
        (stamps.join("|"), d)
    };
    let (gs, gd) = probe(Projection::Globe { center: Some(camera.0), zoom: Some(camera.1) });
    let (fs, fd) = probe(Projection::Flat { center: Some(camera.0), zoom: Some(camera.1) });
    assert_eq!(gs, fs, "both charts stamp the same resolved view");
    assert!(
        (gd - fd).abs() / gd < 0.02,
        "one ground-degree east: globe {gd:.1}px vs flat {fd:.1}px"
    );
}

/// The sentinel's dress is a shared convention: on EITHER chart, a
/// whole-sphere region keeps its land hole — the world ocean can
/// never flood a page again, in any projection.
#[test]
fn the_sentinel_keeps_its_holes_on_both_charts() {
    let sentinel = Ring::new(vec![
        UnitVec::from_lat_lon_deg(0.0, 0.0),
        UnitVec::from_lat_lon_deg(0.0, 179.5),
        UnitVec::from_lat_lon_deg(1.0, -90.0),
    ])
    .unwrap();
    let island = Ring::new(vec![
        UnitVec::from_lat_lon_deg(31.0, 34.0),
        UnitVec::from_lat_lon_deg(31.0, 36.0),
        UnitVec::from_lat_lon_deg(33.0, 36.0),
        UnitVec::from_lat_lon_deg(33.0, 34.0),
    ])
    .unwrap();
    for projection in [
        Projection::Globe { center: Some((32.0, 35.0)), zoom: Some(3.0) },
        Projection::Flat { center: Some((32.0, 35.0)), zoom: Some(3.0) },
    ] {
        let mut scene = Snapshot::empty();
        scene.regions.push(StyledRegion {
            region: map_types::RegionId(atlas_graph_types::covenant::ContentHash(11)),
            entity: Some("sea".into()),
            outer: vec![sentinel.clone()],
            holes: vec![island.clone()],
            paint: map_types::style::Paint { fill: map_types::style::Rgba(1, 2, 200, 255) },
            sources: Default::default(),
        });
        let svg = SvgEncoder { projection, width: 800.0, smooth: false, ..SvgEncoder::default() }
            .encode(&scene)
            .unwrap();
        let d = svg.split("d=\"").nth(1).unwrap().split('"').next().unwrap();
        assert!(
            d.matches('M').count() >= 2,
            "{projection:?}: the hole survives ({} subpaths)",
            d.matches('M').count()
        );
    }
}
