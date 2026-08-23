//! Law 11's determinism half, per backend: same scene + same encoder
//! config -> same bytes. (The terminality half — no format names
//! upstream — is enforced inside map-types by its grep test.)

use std::collections::BTreeSet;

use atlas_graph_types::id::SourceId;
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
        region: map_types::RegionId(atlas_graph_types::id::ContentHash(1)),
        outer: vec![Ring::new(vec![uv(0.0, 0.0), uv(0.0, 10.0), uv(8.0, 5.0)]).unwrap()],
        holes: vec![],
        paint: Paint { fill: Rgba(210, 190, 150, 255) },
        sources: sources.clone(),
    });
    s.boundaries.push(StyledBoundary {
        boundary: map_types::BoundaryId(atlas_graph_types::id::ContentHash(2)),
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
        style: LabelStyle { color: Rgba(10, 10, 10, 255), size: 12.0 },
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
        style: LabelStyle { color: Rgba(10, 10, 10, 255), size: 12.0 },
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
fn empty_scene_encodes() {
    let empty = Snapshot::empty();
    assert!(SvgEncoder::default().encode(&empty).is_ok());
    assert!(GeoJsonEncoder.encode(&empty).is_ok());
}
