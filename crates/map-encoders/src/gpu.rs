//! The retained-scene encoder (rendering spec §18): the terminal
//! backend that turns a styled scene into IMMUTABLE RESOURCES instead
//! of a picture. The SVG encoder answers "what does this camera see";
//! this one answers "what does a retained renderer need resident" —
//! a manifest of semantic references plus content-addressed binary
//! geometry payloads, so a camera change downstream is a uniform
//! update, never a re-encode (§R1/§R2).
//!
//! Laws carried here:
//! - the manifest holds NO projected vertices, no rasterized frame,
//!   no SVG (§8) — projection belongs to the consumer's GPU (§29);
//! - geometry identity is content identity (§I5): equal point runs
//!   hash to one resource however many features reference them, and
//!   whatever year or style asked (§10, §31);
//! - representations are content-addressed separately from canonical
//!   geometry (§7): the resource id covers kind + payload, the
//!   geometry id covers the point content alone;
//! - determinism (law 11): same scene, same bytes.

use std::collections::BTreeMap;
use std::collections::hash_map::DefaultHasher;
use std::fmt::Write as _;
use std::hash::{Hash, Hasher};

use map_types::ident::{Canon, ContentHash};
use map_types::style::{MarkerStyle, Rgba, StrokePattern};
use map_types::{EncodeError, MapAddressed, SceneEncoder, Snapshot, UnitVec};

/// A rendering representation's content address (§7): kind + payload.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ResourceId(pub ContentHash);

/// A geometry's content address (§6): the point run alone — no time,
/// no projection, no style, no view (§R8).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GeometryId(pub ContentHash);

/// What a resource's vertices mean to a renderer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourceKind {
    /// An open polyline: consecutive vertices are stroke segments.
    LineStrip,
    /// A closed ring: last vertex connects to first (fills/outlines).
    RingLoop,
    /// Independent points (markers).
    Points,
}

impl ResourceKind {
    fn wire(self) -> u32 {
        match self {
            ResourceKind::LineStrip => 0,
            ResourceKind::RingLoop => 1,
            ResourceKind::Points => 2,
        }
    }
    fn name(self) -> &'static str {
        match self {
            ResourceKind::LineStrip => "line",
            ResourceKind::RingLoop => "ring",
            ResourceKind::Points => "points",
        }
    }
}

/// A spherical cap around a resource's content — the bounds visibility
/// and residency reason about (§33), projection-free by construction.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SphericalBounds {
    pub center: (f64, f64, f64),
    /// Angular radius in radians.
    pub radius: f64,
}

/// Immutable metadata sufficient for caching and residency (§19).
/// LOD parent/children stay empty until the refinement stages land —
/// absent, not faked.
#[derive(Clone, Debug, PartialEq)]
pub struct ResourceDescriptor {
    pub id: ResourceId,
    pub geometry: GeometryId,
    pub kind: ResourceKind,
    pub byte_length: u64,
    pub vertex_count: u32,
    pub bounds: SphericalBounds,
}

/// One resource: descriptor + the packed binary packet (§63).
#[derive(Clone, Debug, PartialEq)]
pub struct GeometryResource {
    pub descriptor: ResourceDescriptor,
    pub payload: Vec<u8>,
}

/// One semantic feature's reference into the resource set (§8). Order
/// in the manifest is paint order — overlay order is meaning.
#[derive(Clone, Debug, PartialEq)]
pub struct FeatureInstance {
    /// "region:HEX" | "boundary:HEX" | "markers" — the same keys the
    /// rest of the wire speaks.
    pub feature: String,
    pub geometry: GeometryId,
    pub resource: ResourceId,
    pub style: StyleKey,
}

/// A compiled paint's content address: the manifest's style table key.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StyleKey(pub u64);

/// The paint a GPU layer may apply — style stays data (§61): the
/// values come from the scene's own resolved styles, never invented.
#[derive(Clone, Debug, PartialEq)]
pub enum GpuStyle {
    Stroke { color: Rgba, width: f64, pattern: StrokePattern },
    Marker { color: Rgba, size: f64 },
}

/// The semantic scene manifest (§8): references, not pictures.
#[derive(Clone, Debug, PartialEq)]
pub struct SceneManifest {
    pub scene_revision: ContentHash,
    pub features: Vec<FeatureInstance>,
    pub styles: BTreeMap<StyleKey, GpuStyle>,
}

/// The encoder's whole answer (§18).
#[derive(Clone, Debug, PartialEq)]
pub struct EncodedScene {
    pub manifest: SceneManifest,
    pub resources: Vec<GeometryResource>,
}

/// Retained-scene backend alongside `SvgEncoder` (§18). Stateless:
/// everything it emits derives from the scene alone.
pub struct GpuSceneEncoder;

// ------------------------------------------------------------ hashing

fn hash64(bytes: &[u8]) -> u64 {
    // The project's content-address skeleton (map-types::ident): a
    // deterministic 64-bit std hash standing in for a real multihash.
    let mut h = DefaultHasher::new();
    bytes.hash(&mut h);
    h.finish()
}

fn geometry_id(pts: &[UnitVec]) -> GeometryId {
    let mut c = Canon::new();
    c.tag("geometry");
    c.seq(pts, |c, p| p.canon(c));
    GeometryId(ContentHash(hash64(&c.done())))
}

fn style_key(style: &GpuStyle) -> StyleKey {
    let mut c = Canon::new();
    match style {
        GpuStyle::Stroke { color, width, pattern } => {
            c.tag("stroke");
            c.u8_(color.0).u8_(color.1).u8_(color.2).u8_(color.3);
            c.f64_(*width);
            c.u8_(match pattern {
                StrokePattern::Solid => 0,
                StrokePattern::Dashed => 1,
                StrokePattern::Hatched => 2,
                StrokePattern::Zonal => 3,
            });
        }
        GpuStyle::Marker { color, size } => {
            c.tag("marker");
            c.u8_(color.0).u8_(color.1).u8_(color.2).u8_(color.3);
            c.f64_(*size);
        }
    }
    StyleKey(hash64(&c.done()))
}

// ----------------------------------------------------- binary packing

/// Packet magic: "MGR1" — map geometry resource, format 1.
pub const RESOURCE_MAGIC: [u8; 4] = *b"MGR1";

fn bounds_of(pts: &[UnitVec]) -> SphericalBounds {
    let (mut x, mut y, mut z) = (0.0f64, 0.0f64, 0.0f64);
    for p in pts {
        x += p.x();
        y += p.y();
        z += p.z();
    }
    let center = match UnitVec::normalize(x, y, z) {
        Ok(c) => c,
        // A degenerate (balanced) point set: any center with the whole
        // sphere as radius stays sound.
        Err(_) => {
            return SphericalBounds { center: (0.0, 0.0, 1.0), radius: std::f64::consts::PI }
        }
    };
    let radius = pts.iter().map(|p| center.angle_to(p)).fold(0.0f64, f64::max);
    SphericalBounds { center: (center.x(), center.y(), center.z()), radius }
}

/// Pack one geometry into the §63 wire packet:
///
/// ```text
/// 0   magic "MGR1"
/// 4   u32 kind
/// 8   u64 resource id
/// 16  u64 geometry id
/// 24  f32 bounds center x, y, z
/// 36  f32 bounds radius (radians)
/// 40  u32 vertex count
/// 44  u32 index count (0 — no index payload in format 1)
/// 48  vertex payload: count × (f32 x, f32 y, f32 z)  (§17)
/// ```
///
/// All integers and floats little-endian.
fn pack(kind: ResourceKind, pts: &[UnitVec]) -> GeometryResource {
    let geometry = geometry_id(pts);
    let mut verts = Vec::with_capacity(pts.len() * 12);
    for p in pts {
        verts.extend_from_slice(&(p.x() as f32).to_le_bytes());
        verts.extend_from_slice(&(p.y() as f32).to_le_bytes());
        verts.extend_from_slice(&(p.z() as f32).to_le_bytes());
    }
    // The resource id covers what the payload IS: kind + vertices.
    // Bounds and counts derive from those, so hashing them too would
    // add nothing but the chance of drift.
    let id = {
        let mut c = Canon::new();
        c.tag("resource");
        c.u8_(kind.wire() as u8);
        c.0.extend_from_slice(&verts);
        ResourceId(ContentHash(hash64(&c.done())))
    };
    let bounds = bounds_of(pts);
    let mut payload = Vec::with_capacity(48 + verts.len());
    payload.extend_from_slice(&RESOURCE_MAGIC);
    payload.extend_from_slice(&kind.wire().to_le_bytes());
    payload.extend_from_slice(&id.0 .0.to_le_bytes());
    payload.extend_from_slice(&geometry.0 .0.to_le_bytes());
    payload.extend_from_slice(&(bounds.center.0 as f32).to_le_bytes());
    payload.extend_from_slice(&(bounds.center.1 as f32).to_le_bytes());
    payload.extend_from_slice(&(bounds.center.2 as f32).to_le_bytes());
    payload.extend_from_slice(&(bounds.radius as f32).to_le_bytes());
    payload.extend_from_slice(&(pts.len() as u32).to_le_bytes());
    payload.extend_from_slice(&0u32.to_le_bytes()); // no indices in v1
    payload.extend_from_slice(&verts);
    GeometryResource {
        descriptor: ResourceDescriptor {
            id,
            geometry,
            kind,
            byte_length: payload.len() as u64,
            vertex_count: pts.len() as u32,
            bounds,
        },
        payload,
    }
}

// ------------------------------------------------------------ encoder

impl GpuSceneEncoder {
    fn build(&self, scene: &Snapshot) -> EncodedScene {
        let mut resources: Vec<GeometryResource> = Vec::new();
        let mut seen: BTreeMap<ResourceId, usize> = BTreeMap::new();
        let mut features: Vec<FeatureInstance> = Vec::new();
        let mut styles: BTreeMap<StyleKey, GpuStyle> = BTreeMap::new();

        let mut add = |resources: &mut Vec<GeometryResource>,
                       seen: &mut BTreeMap<ResourceId, usize>,
                       kind: ResourceKind,
                       pts: &[UnitVec]|
         -> (ResourceId, GeometryId) {
            let r = pack(kind, pts);
            let (id, geom) = (r.descriptor.id, r.descriptor.geometry);
            // Content dedup (§I5): equal content, one resident copy,
            // however many features reference it.
            seen.entry(id).or_insert_with(|| {
                resources.push(r);
                resources.len() - 1
            });
            (id, geom)
        };
        let mut style_of = |styles: &mut BTreeMap<StyleKey, GpuStyle>, s: GpuStyle| -> StyleKey {
            let key = style_key(&s);
            styles.entry(key).or_insert(s);
            key
        };

        // Region rings ship as ring loops: retained geometry for the
        // fill/outline stages (residency is not visibility — §I13 —
        // a consumer may keep them resident and draw nothing yet).
        for r in &scene.regions {
            let sk = style_of(
                &mut styles,
                GpuStyle::Stroke {
                    color: r.paint.fill,
                    width: 0.0, // a fill paint carries no stroke width
                    pattern: StrokePattern::Solid,
                },
            );
            for ring in r.outer.iter().chain(&r.holes) {
                let (id, geom) =
                    add(&mut resources, &mut seen, ResourceKind::RingLoop, ring.points());
                features.push(FeatureInstance {
                    feature: format!("region:{:016x}", r.region.0 .0),
                    geometry: geom,
                    resource: id,
                    style: sk,
                });
            }
        }
        // Boundaries: the representative Stage-4 layer — real strokes
        // from the style system, drawn by the GPU consumer.
        for b in &scene.boundaries {
            let sk = style_of(
                &mut styles,
                GpuStyle::Stroke {
                    color: b.stroke.color,
                    width: b.stroke.width,
                    pattern: b.stroke.pattern,
                },
            );
            let (id, geom) = add(&mut resources, &mut seen, ResourceKind::LineStrip, &b.pts);
            features.push(FeatureInstance {
                feature: format!("boundary:{:016x}", b.boundary.0 .0),
                geometry: geom,
                resource: id,
                style: sk,
            });
        }
        // Markers batch by style: one Points resource per marker dress.
        let mut by_style: BTreeMap<StyleKey, (MarkerStyle, Vec<UnitVec>)> = BTreeMap::new();
        for m in &scene.markers {
            let sk = style_of(
                &mut styles,
                GpuStyle::Marker { color: m.style.color, size: m.style.size },
            );
            by_style.entry(sk).or_insert_with(|| (m.style, Vec::new())).1.push(m.at);
        }
        for (sk, (_, pts)) in by_style {
            let (id, geom) = add(&mut resources, &mut seen, ResourceKind::Points, &pts);
            features.push(FeatureInstance {
                feature: "markers".to_string(),
                geometry: geom,
                resource: id,
                style: sk,
            });
        }

        EncodedScene {
            manifest: SceneManifest {
                scene_revision: scene.map_pid().hash,
                features,
                styles,
            },
            resources,
        }
    }
}

impl SceneEncoder for GpuSceneEncoder {
    type Output = EncodedScene;
    fn encode(&self, scene: &Snapshot) -> Result<EncodedScene, EncodeError> {
        Ok(self.build(scene))
    }
}

// ---------------------------------------------------------- wire JSON

impl EncodedScene {
    /// The manifest on the wire. Deterministic: features in paint
    /// order, styles and resources in key order. Carries NO vertices —
    /// payloads travel separately by resource id (§63).
    pub fn manifest_json(&self) -> String {
        let m = &self.manifest;
        let mut s = String::new();
        let _ = write!(s, "{{\"scene\":\"{:016x}\",\"features\":[", m.scene_revision.0);
        for (i, f) in m.features.iter().enumerate() {
            let _ = write!(
                s,
                "{}{{\"feature\":\"{}\",\"geometry\":\"{:016x}\",\"resource\":\"{:016x}\",\"style\":\"{:016x}\"}}",
                if i > 0 { "," } else { "" },
                f.feature,
                f.geometry.0 .0,
                f.resource.0 .0,
                f.style.0
            );
        }
        s.push_str("],\"styles\":{");
        for (i, (k, v)) in m.styles.iter().enumerate() {
            if i > 0 {
                s.push(',');
            }
            match v {
                GpuStyle::Stroke { color, width, pattern } => {
                    let _ = write!(
                        s,
                        "\"{:016x}\":{{\"kind\":\"stroke\",\"color\":[{},{},{},{}],\"width\":{},\"pattern\":\"{}\"}}",
                        k.0,
                        color.0,
                        color.1,
                        color.2,
                        color.3,
                        width,
                        match pattern {
                            StrokePattern::Solid => "solid",
                            StrokePattern::Dashed => "dashed",
                            StrokePattern::Hatched => "hatched",
                            StrokePattern::Zonal => "zonal",
                        }
                    );
                }
                GpuStyle::Marker { color, size } => {
                    let _ = write!(
                        s,
                        "\"{:016x}\":{{\"kind\":\"marker\",\"color\":[{},{},{},{}],\"size\":{}}}",
                        k.0, color.0, color.1, color.2, color.3, size
                    );
                }
            }
        }
        s.push_str("},\"resources\":[");
        for (i, r) in self.resources.iter().enumerate() {
            let d = &r.descriptor;
            let _ = write!(
                s,
                "{}{{\"id\":\"{:016x}\",\"kind\":\"{}\",\"bytes\":{},\"vertices\":{},\"bounds\":{{\"center\":[{:.6},{:.6},{:.6}],\"radius\":{:.6}}}}}",
                if i > 0 { "," } else { "" },
                d.id.0 .0,
                d.kind.name(),
                d.byte_length,
                d.vertex_count,
                d.bounds.center.0,
                d.bounds.center.1,
                d.bounds.center.2,
                d.bounds.radius
            );
        }
        s.push_str("]}");
        s
    }
}
