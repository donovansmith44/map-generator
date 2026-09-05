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

// ================================================== retained-scene GPU

use crate::{EncodedScene, GpuSceneEncoder, ResourceKind, RESOURCE_MAGIC};

fn gpu_encode(scene: &Snapshot) -> EncodedScene {
    GpuSceneEncoder.encode(scene).unwrap()
}

/// Law 11 for the retained backend: same scene, same manifest, same
/// payload bytes — content-addressed caching survives the encoding.
#[test]
fn gpu_encoder_is_deterministic() {
    let a = gpu_encode(&sample_scene());
    let b = gpu_encode(&sample_scene());
    assert_eq!(a.manifest_json(), b.manifest_json());
    assert_eq!(a.resources.len(), b.resources.len());
    for (ra, rb) in a.resources.iter().zip(&b.resources) {
        assert_eq!(ra.payload, rb.payload);
    }
}

/// §8: the manifest is semantic references only — no projected
/// vertices, no rasterized frame, no SVG.
#[test]
fn manifest_carries_no_geometry() {
    let enc = gpu_encode(&sample_scene());
    let json = enc.manifest_json();
    assert!(!json.contains("<svg"), "no SVG in a manifest");
    assert!(!json.contains("\"x\":") && !json.contains("\"points\":["), "no vertex payloads");
    // Every feature references resources by id; the ids resolve.
    let ids: std::collections::BTreeSet<_> =
        enc.resources.iter().map(|r| r.descriptor.id).collect();
    for f in &enc.manifest.features {
        assert!(ids.contains(&f.resource), "feature references a shipped resource");
    }
}

/// §I5 / Test G: equal geometry content yields ONE resource however
/// many features reference it — the historical-reuse property at the
/// resource layer.
#[test]
fn equal_content_shares_one_resource() {
    let mut scene = Snapshot::empty();
    let pts = vec![uv(0.0, 0.0), uv(0.0, 10.0), uv(5.0, 12.0)];
    for n in [7u64, 8u64] {
        scene.boundaries.push(StyledBoundary {
            boundary: map_types::BoundaryId(atlas_graph_types::covenant::ContentHash(n)),
            pts: pts.clone(),
            stroke: Stroke {
                color: Rgba(90, 60, 40, 255),
                width: 1.5,
                pattern: StrokePattern::Solid,
            },
            sources: Default::default(),
        });
    }
    let enc = gpu_encode(&scene);
    let lines: Vec<_> = enc
        .resources
        .iter()
        .filter(|r| r.descriptor.kind == ResourceKind::LineStrip)
        .collect();
    assert_eq!(lines.len(), 1, "one payload for one content");
    let features: Vec<_> =
        enc.manifest.features.iter().filter(|f| f.feature.starts_with("boundary:")).collect();
    assert_eq!(features.len(), 2, "both semantic features remain");
    assert_eq!(features[0].resource, features[1].resource);
    assert_eq!(features[0].geometry, features[1].geometry);
}

/// §R8: a geometry's identity is independent of its style — restyling
/// the same points must not mint a new geometry or resource.
#[test]
fn identity_is_independent_of_style() {
    let pts = vec![uv(0.0, 0.0), uv(0.0, 10.0)];
    let scene_with = |color: Rgba| {
        let mut s = Snapshot::empty();
        s.boundaries.push(StyledBoundary {
            boundary: map_types::BoundaryId(atlas_graph_types::covenant::ContentHash(3)),
            pts: pts.clone(),
            stroke: Stroke { color, width: 2.0, pattern: StrokePattern::Solid },
            sources: Default::default(),
        });
        s
    };
    let a = gpu_encode(&scene_with(Rgba(255, 0, 0, 255)));
    let b = gpu_encode(&scene_with(Rgba(0, 0, 255, 255)));
    assert_eq!(a.resources[0].descriptor.id, b.resources[0].descriptor.id);
    assert_eq!(a.resources[0].payload, b.resources[0].payload);
    assert_ne!(a.manifest.features[0].style, b.manifest.features[0].style);
}

/// §63/§17: the binary packet decodes back to the geometry — header
/// fields honest, f32 unit-sphere vertices, bounds containing every
/// vertex.
#[test]
fn binary_packet_roundtrips() {
    let enc = gpu_encode(&sample_scene());
    for r in &enc.resources {
        let p = &r.payload;
        assert_eq!(&p[0..4], &RESOURCE_MAGIC);
        let u32_at = |o: usize| u32::from_le_bytes(p[o..o + 4].try_into().unwrap());
        let u64_at = |o: usize| u64::from_le_bytes(p[o..o + 8].try_into().unwrap());
        let f32_at = |o: usize| f32::from_le_bytes(p[o..o + 4].try_into().unwrap());
        assert_eq!(u64_at(8), r.descriptor.id.0 .0);
        assert_eq!(u64_at(16), r.descriptor.geometry.0 .0);
        let count = u32_at(40) as usize;
        assert_eq!(count, r.descriptor.vertex_count as usize);
        assert_eq!(u32_at(44), 0, "format 1 ships no indices");
        assert_eq!(p.len(), 48 + count * 12);
        assert_eq!(p.len() as u64, r.descriptor.byte_length);
        let (bx, by, bz, br) =
            (f32_at(24) as f64, f32_at(28) as f64, f32_at(32) as f64, f32_at(36) as f64);
        for i in 0..count {
            let o = 48 + i * 12;
            let (x, y, z) = (f32_at(o) as f64, f32_at(o + 4) as f64, f32_at(o + 8) as f64);
            let n = (x * x + y * y + z * z).sqrt();
            assert!((n - 1.0).abs() < 1e-3, "vertex stays on the unit sphere");
            let dot = (x * bx + y * by + z * bz) / n;
            let angle = dot.clamp(-1.0, 1.0).acos();
            assert!(angle <= br + 1e-3, "bounds cap contains every vertex");
        }
    }
}

/// Every scene element lands as a resource of its kind: region rings
/// as loops, boundaries as strips, markers as points.
#[test]
fn scene_elements_map_to_kinds() {
    let enc = gpu_encode(&sample_scene());
    let kind_count = |k: ResourceKind| {
        enc.resources.iter().filter(|r| r.descriptor.kind == k).count()
    };
    assert_eq!(kind_count(ResourceKind::RingLoop), 1);
    assert_eq!(kind_count(ResourceKind::LineStrip), 1);
    assert_eq!(kind_count(ResourceKind::Points), 1);
    let json = enc.manifest_json();
    assert!(json.contains("\"kind\":\"stroke\""));
    assert!(json.contains("\"kind\":\"marker\""));
}

/// Stage 5: a region's rings wear its FILL style in the manifest —
/// one feature key across all its rings, so a consumer realizes the
/// interior under one even-odd pass, exactly like the SVG's
/// fill-rule="evenodd".
#[test]
fn region_rings_share_one_fill_feature() {
    let mut scene = Snapshot::empty();
    scene.regions.push(StyledRegion {
        region: map_types::RegionId(atlas_graph_types::covenant::ContentHash(21)),
        entity: None,
        outer: vec![Ring::new(vec![uv(0.0, 0.0), uv(0.0, 10.0), uv(8.0, 5.0)]).unwrap()],
        holes: vec![Ring::new(vec![uv(2.0, 4.0), uv(2.0, 6.0), uv(4.0, 5.0)]).unwrap()],
        paint: Paint { fill: Rgba(210, 190, 150, 200) },
        sources: Default::default(),
    });
    let enc = gpu_encode(&scene);
    let fills: Vec<_> = enc
        .manifest
        .features
        .iter()
        .filter(|f| matches!(enc.manifest.styles[&f.style], crate::GpuStyle::Fill { .. }))
        .collect();
    assert_eq!(fills.len(), 2, "outer + hole, both fill-styled");
    assert_eq!(fills[0].feature, fills[1].feature, "one region, one feature key");
    assert_eq!(fills[0].style, fills[1].style);
    assert!(enc.manifest_json().contains("\"kind\":\"fill\""));
}

/// The whole-sphere sentinel is a first-class law (covers_sphere):
/// the manifest marks it so no consumer re-guesses which ring is the
/// world's envelope.
#[test]
fn sentinel_ring_is_marked_whole() {
    let sentinel = Ring::new(vec![
        uv(0.0, 0.0),
        uv(0.0, 120.0),
        uv(0.0, -120.0),
        uv(5.0, 179.0), // near-antipodal to the first point
    ])
    .unwrap();
    assert!(map_types::covers_sphere(sentinel.points()), "test ring must trip the sentinel law");
    let mut scene = Snapshot::empty();
    scene.regions.push(StyledRegion {
        region: map_types::RegionId(atlas_graph_types::covenant::ContentHash(22)),
        entity: None,
        outer: vec![sentinel],
        holes: vec![],
        paint: Paint { fill: Rgba(1, 2, 200, 255) },
        sources: Default::default(),
    });
    let enc = gpu_encode(&scene);
    assert!(enc.resources[0].descriptor.whole);
    assert!(enc.manifest_json().contains("\"whole\":true"));
    // An ordinary territory never wears the mark.
    let plain = gpu_encode(&sample_scene());
    assert!(plain.resources.iter().all(|r| !r.descriptor.whole));
    assert!(!plain.manifest_json().contains("\"whole\""));
}

/// THE PARITY LAW behind the whole retained path: whatever the SVG
/// backend draws, the manifest describes — same scene value, same
/// paints, same features. The system that makes the maps the owner
/// likes IS the system the GPU consumes; this test is the tripwire
/// against the two ever describing different worlds.
#[test]
fn manifest_and_svg_describe_the_same_scene() {
    use map_types::MapAddressed as _;
    let scene = sample_scene();
    let svg = SvgEncoder {
        projection: Projection::Globe { center: Some((4.0, 5.0)), zoom: Some(12.0) },
        width: 1200.0,
        smooth: false,
        ..SvgEncoder::default()
    }
    .encode(&scene)
    .unwrap();
    let enc = gpu_encode(&scene);
    // Every region the scene styles: its exact fill paint appears in
    // both encodings.
    for r in &scene.regions {
        let Rgba(cr, cg, cb, _) = r.paint.fill;
        assert!(svg.contains(&format!("fill=\"rgb({cr},{cg},{cb})\"")), "SVG paints the region");
        let key = format!("region:{:016x}", r.region.0 .0);
        let f = enc.manifest.features.iter().find(|f| f.feature == key).expect("manifest has it");
        match &enc.manifest.styles[&f.style] {
            crate::GpuStyle::Fill { color } => assert_eq!(*color, r.paint.fill),
            other => panic!("region wears a fill, not {other:?}"),
        }
    }
    // Every boundary: same stroke color and width both sides.
    for b in &scene.boundaries {
        let Rgba(cr, cg, cb, _) = b.stroke.color;
        assert!(svg.contains(&format!("stroke=\"rgb({cr},{cg},{cb})\"")), "SVG strokes it");
        let key = format!("boundary:{:016x}", b.boundary.0 .0);
        let f = enc.manifest.features.iter().find(|f| f.feature == key).expect("manifest has it");
        match &enc.manifest.styles[&f.style] {
            crate::GpuStyle::Stroke { color, width, .. } => {
                assert_eq!(*color, b.stroke.color);
                assert_eq!(*width, b.stroke.width);
            }
            other => panic!("boundary wears a stroke, not {other:?}"),
        }
    }
    // And the manifest's revision is the scene's own content address —
    // the same identity law the rest of the system caches by.
    assert_eq!(enc.manifest.scene_revision, scene.map_pid().hash);
}

/// The antimeridian split: pieces never cross the seam, cut edges
/// cancel under even-odd parity, and geometry clear of the seam ships
/// whole with untouched ids.
#[test]
fn antimeridian_split_is_seam_safe() {
    // A ring straddling lon 180 (the Bering shape of the problem).
    let ring = Ring::new(vec![
        uv(60.0, 170.0),
        uv(60.0, -170.0),
        uv(70.0, -170.0),
        uv(70.0, 170.0),
    ])
    .unwrap();
    let mut scene = Snapshot::empty();
    scene.regions.push(StyledRegion {
        region: map_types::RegionId(atlas_graph_types::covenant::ContentHash(31)),
        entity: None,
        outer: vec![ring],
        holes: vec![],
        paint: Paint { fill: Rgba(1, 2, 3, 255) },
        sources: Default::default(),
    });
    scene.boundaries.push(StyledBoundary {
        boundary: map_types::BoundaryId(atlas_graph_types::covenant::ContentHash(32)),
        pts: vec![uv(60.0, 170.0), uv(60.0, -170.0), uv(55.0, -160.0)],
        stroke: Stroke { color: Rgba(0, 0, 0, 255), width: 1.0, pattern: StrokePattern::Solid },
        sources: Default::default(),
    });
    let enc = gpu_encode(&scene);
    // Every emitted piece lives in ONE longitude half: no edge of any
    // piece can cross lon ±180.
    for r in &enc.resources {
        let p = &r.payload;
        let count = u32::from_le_bytes(p[40..44].try_into().unwrap()) as usize;
        let mut pos = false;
        let mut neg = false;
        for i in 0..count {
            let o = 48 + i * 12;
            let y = f32::from_le_bytes(p[o + 4..o + 8].try_into().unwrap());
            if y > 0.0 {
                pos = true;
            }
            if y < 0.0 {
                neg = true;
            }
        }
        assert!(!(pos && neg), "a piece stays within one longitude half");
    }
    // The seam-crossing ring became two ring pieces under one feature.
    let ring_feats: Vec<_> =
        enc.manifest.features.iter().filter(|f| f.feature.starts_with("region:")).collect();
    assert_eq!(ring_feats.len(), 2, "two parity pieces, one region");
    assert_eq!(ring_feats[0].feature, ring_feats[1].feature);
    // The crossing boundary split into two strips under one feature.
    let line_feats: Vec<_> =
        enc.manifest.features.iter().filter(|f| f.feature.starts_with("boundary:")).collect();
    assert_eq!(line_feats.len(), 2, "the strip splits at the seam");
    // Seam-free geometry ships whole and keeps its identity.
    let calm = gpu_encode(&sample_scene());
    let calm2 = gpu_encode(&sample_scene());
    assert_eq!(calm.resources.len(), calm2.resources.len());
    assert_eq!(
        calm.manifest.features.iter().filter(|f| f.feature.starts_with("boundary:")).count(),
        1,
        "a near-east boundary stays one strip"
    );
}

// ---------------------------------------- the limb clip, law-checked
// clip_ring_front derives everything from inside_ring: limb arcs
// between crossings are kept exactly when their midpoints lie inside
// the original ring. These tests pin the geometry the old
// hidden-azimuth sweep got wrong near the camera antipode.

/// A circle of angular radius `r` (radians) around `axis`, `m` points.
fn circle_ring(axis: &UnitVec, r: f64, m: usize) -> Vec<UnitVec> {
    let e = UnitVec::normalize(-axis.y(), axis.x(), 0.0)
        .unwrap_or_else(|_| UnitVec::from_lat_lon_deg(0.0, 90.0));
    let (nx, ny, nz) = axis.cross_raw(&e);
    let n = UnitVec::normalize(nx, ny, nz).unwrap();
    (0..m)
        .map(|k| {
            let t = std::f64::consts::TAU * k as f64 / m as f64;
            let (c, s) = (r.cos(), r.sin());
            UnitVec::normalize(
                axis.x() * c + (e.x() * t.cos() + n.x() * t.sin()) * s,
                axis.y() * c + (e.y() * t.cos() + n.y() * t.sin()) * s,
                axis.z() * c + (e.z() * t.cos() + n.z() * t.sin()) * s,
            )
            .unwrap()
        })
        .collect()
}

/// The view-plane frame at camera centre `c` (the projectors' own
/// east/north construction).
fn view_frame(c: &UnitVec) -> (UnitVec, UnitVec) {
    let e = UnitVec::normalize(-c.y(), c.x(), 0.0)
        .unwrap_or_else(|_| UnitVec::from_lat_lon_deg(0.0, 90.0));
    let (nx, ny, nz) = c.cross_raw(&e);
    (e, UnitVec::normalize(nx, ny, nz).unwrap())
}

/// Even-odd over every emitted loop, projected to the view plane.
fn clipped_contains(loops: &[Vec<UnitVec>], c: &UnitVec, probe: &UnitVec) -> bool {
    let (e, nv) = view_frame(c);
    let (x, y) = (probe.dot(&e), probe.dot(&nv));
    let mut inside = false;
    for run in loops {
        let pts: Vec<(f64, f64)> = run.iter().map(|p| (p.dot(&e), p.dot(&nv))).collect();
        let n = pts.len();
        for j in 0..n {
            let (jx, jy) = pts[j];
            let (kx, ky) = pts[(j + 1) % n];
            if ((jy > y) != (ky > y)) && x < jx + (y - jy) / (ky - jy) * (kx - jx) {
                inside = !inside;
            }
        }
    }
    inside
}

#[test]
fn limb_clip_keeps_every_point_on_the_front() {
    let c = uv(20.0, -150.0);
    let deg = std::f64::consts::PI / 180.0;
    for (axis_off, radius) in [(60.0, 40.0), (100.0, 79.0), (85.0, 30.0), (30.0, 45.0)] {
        let axis = circle_ring(&c, axis_off * deg, 8)[0];
        let ring = circle_ring(&axis, radius * deg, 180);
        for run in crate::clip_ring_front(&ring, &c) {
            assert!(run.len() >= 3, "a clipped run is a closed ring");
            for p in &run {
                assert!(p.dot(&c) >= -1e-9, "every clipped point faces the camera");
            }
        }
    }
}

#[test]
fn limb_clip_does_not_invert_around_the_antipode() {
    // The Pacific bug, distilled: a big ring whose hidden stretch
    // passes within a degree of the camera antipode, where the old
    // sweep accumulated atan2 of two near-zeros. The view centre is
    // NOT inside this ring, and the clip must agree.
    let c = uv(20.0, -150.0);
    let deg = std::f64::consts::PI / 180.0;
    let axis = circle_ring(&c, 100.0 * deg, 12)[3];
    let ring = circle_ring(&axis, 79.0 * deg, 720);
    assert!(
        !map_types::inside_ring(&c, &ring),
        "fixture sanity: the camera centre is outside the ring"
    );
    let loops = crate::clip_ring_front(&ring, &c);
    assert!(!loops.is_empty(), "the ring straddles the limb, something is visible");
    assert!(
        !clipped_contains(&loops, &c, &c),
        "the clipped fill must not swallow the view centre"
    );
    // And a point inside the ring on the front side must stay covered.
    let probe = circle_ring(&axis, 60.0 * deg, 720)
        .into_iter()
        .find(|p| p.dot(&c) > 0.35)
        .expect("part of the interior faces the camera");
    assert!(map_types::inside_ring(&probe, &ring), "fixture sanity: probe is inside");
    assert!(clipped_contains(&loops, &c, &probe), "the clipped fill keeps the visible interior");
}

#[test]
fn limb_clip_totality_at_the_extremes() {
    let c = uv(20.0, -150.0);
    let deg = std::f64::consts::PI / 180.0;
    // Wholly front: the ring comes back as itself, one run.
    let front = circle_ring(&c, 30.0 * deg, 90);
    let loops = crate::clip_ring_front(&front, &c);
    assert_eq!(loops.len(), 1);
    assert_eq!(loops[0], front);
    // Wholly behind, small: dropped.
    let behind = circle_ring(&c.antipode(), 30.0 * deg, 90);
    assert!(crate::clip_ring_front(&behind, &c).is_empty());
    // Wholly behind and large: STILL dropped. Under the smaller-
    // component law a hidden ring's interior can never reach the
    // front — the ring-free front hemisphere (area 2 pi) always sits
    // in the larger component. (A ring surrounding the whole front,
    // like a 95-degree circle about the antipode, has front-facing
    // vertices and never lands in this branch; the whole-sphere
    // sentinel is dressed upstream.)
    let wide = circle_ring(&c.antipode(), 80.0 * deg, 360);
    assert!(wide.iter().all(|p| p.dot(&c) < 0.0), "fixture sanity: all vertices hidden");
    assert!(crate::clip_ring_front(&wide, &c).is_empty());
    // And the near-limb front-facing giant: comes back whole, one run.
    let surround = circle_ring(&c.antipode(), 95.0 * deg, 360);
    assert!(surround.iter().all(|p| p.dot(&c) > 0.0), "fixture sanity: all vertices front");
    assert_eq!(crate::clip_ring_front(&surround, &c).len(), 1);
}

#[test]
fn limb_clip_emits_multiple_lobes_when_the_ring_returns() {
    // An hourglass band: up the front at lon -65, across the far side
    // at lat 40, down the front at lon 65, and back across the far
    // side at lat -40. Its interior (the smaller component) is the
    // band outside [-65, 65] longitude between the two parallels; the
    // visible part of that is TWO strips, one at each side of the
    // disc, so the clip must emit at least two closed loops.
    let c = uv(0.0, 0.0);
    let mut ring: Vec<UnitVec> = Vec::new();
    let mut lat = -40.0;
    while lat <= 40.0 {
        ring.push(uv(lat, -65.0));
        lat += 5.0;
    }
    let mut lon = -75.0;
    while lon >= -175.0 {
        ring.push(uv(40.0, lon));
        lon -= 10.0;
    }
    let mut lon = 175.0;
    while lon >= 75.0 {
        ring.push(uv(40.0, lon));
        lon -= 10.0;
    }
    let mut lat = 40.0;
    while lat >= -40.0 {
        ring.push(uv(lat, 65.0));
        lat -= 5.0;
    }
    let mut lon = 75.0;
    while lon <= 175.0 {
        ring.push(uv(-40.0, lon));
        lon += 10.0;
    }
    let mut lon = -175.0;
    while lon <= -75.0 {
        ring.push(uv(-40.0, lon));
        lon += 10.0;
    }
    let loops = crate::clip_ring_front(&ring, &c);
    assert!(loops.len() >= 2, "two visible strips need two loops (got {})", loops.len());
    for (lat, lon, want) in
        [(0.0, -75.0, true), (0.0, 75.0, true), (0.0, 0.0, false), (60.0, -75.0, false)]
    {
        assert_eq!(
            clipped_contains(&loops, &c, &uv(lat, lon)),
            want,
            "probe at lat {lat} lon {lon}"
        );
    }
}
