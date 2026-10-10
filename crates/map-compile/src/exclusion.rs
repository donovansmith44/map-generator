use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use geo::algorithm::bool_ops::unary_union;
use geo::{
    Bearing, BooleanOps, BoundingRect, Buffer, Coord, Densify, Destination, Distance, Euclidean,
    Haversine, Length, LineString, MultiLineString, MultiPolygon, Point,
};
use map_canon::CanonStore;
use map_types::{UnitVec, WorldTimeline};
use rstar::{RTree, AABB};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum ExcludedSource {
    KnowingTheBible,
    Tribes12,
    SplicedRegions,
    HistoricalBasemaps,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct ExcludedGeometry {
    pub source: ExcludedSource,
    pub geometry: String,
    pub origin: String,
    pub source_sha256: String,
    coordinates: Vec<[f64; 2]>,
}

impl ExcludedGeometry {
    pub fn from_points(
        source: ExcludedSource,
        geometry: String,
        origin: String,
        source_sha256: String,
        points: &[UnitVec],
    ) -> Self {
        Self {
            source,
            geometry,
            origin,
            source_sha256,
            coordinates: points
                .iter()
                .map(|point| {
                    let (lat, lon) = point.to_lat_lon_deg();
                    [lon, lat]
                })
                .collect(),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
pub struct GeometricPolicy {
    pub tolerance_meters: f64,
    pub maximum_unexplained_meters: f64,
    pub short_line_fraction: f64,
}

#[derive(Deserialize)]
struct Catalogue {
    base: String,
    decision: String,
    policy: GeometricPolicy,
    geometries: Vec<ExcludedGeometry>,
    permitted: Vec<PermittedGeometry>,
}

#[derive(Deserialize)]
struct PermittedGeometry {
    origin: String,
    source_sha256: String,
    license: NaturalEarthLicense,
    coordinates: Vec<Vec<[f64; 2]>>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub enum NaturalEarthLicense {
    #[serde(rename = "Public Domain")]
    PublicDomain,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExcludedStretch {
    pub geometry: ExcludedGeometry,
    pub policy: GeometricPolicy,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ExclusionError {
    CatalogueDecode(String),
    EmptyCatalogue,
    InputRead { path: PathBuf, message: String },
    InputDecode { path: PathBuf, message: String },
    Excluded(ExcludedStretch),
}

pub struct LineageIndex {
    geometries: Vec<BufferedExclusion>,
    policy: GeometricPolicy,
}

struct BufferedExclusion {
    geometry: ExcludedGeometry,
    frame: MeterFrame,
    unexplained: MultiPolygon,
    envelope: AABB<Point>,
    threshold: f64,
}

impl LineageIndex {
    pub fn new(
        geometries: Vec<ExcludedGeometry>,
        policy: GeometricPolicy,
        permitted: &[Vec<UnitVec>],
    ) -> Self {
        let permitted = segment_index(
            &permitted
                .iter()
                .map(|points| unit_line(points.iter()))
                .collect::<Vec<_>>(),
        );
        let geometries = geometries
            .into_iter()
            .filter_map(|geometry| {
                let coordinates = geographic_line(&geometry.coordinates);
                let origin = coordinates.points().next()?;
                let frame = MeterFrame(origin);
                let line = frame.line(&coordinates);
                let threshold = policy
                    .maximum_unexplained_meters
                    .min(Haversine.length(&coordinates) * policy.short_line_fraction);
                let mut unexplained = line.buffer(policy.tolerance_meters);
                let geographic_bounds = Haversine
                    .densify(&coordinates, MAXIMUM_GEODESIC_SEGMENT_METERS)
                    .bounding_rect()?;
                let southwest = Haversine.destination(
                    geographic_bounds.min().into(),
                    225.0,
                    policy.tolerance_meters * 2.0_f64.sqrt(),
                );
                let northeast = Haversine.destination(
                    geographic_bounds.max().into(),
                    45.0,
                    policy.tolerance_meters * 2.0_f64.sqrt(),
                );
                let envelope = AABB::from_corners(southwest, northeast);
                let explained = permitted
                    .locate_in_envelope_intersecting(&envelope)
                    .map(|segment| frame.line(&LineString::from(vec![segment.start, segment.end])))
                    .map(|line| line.buffer(policy.tolerance_meters))
                    .collect::<Vec<_>>();
                unexplained = unexplained.difference(&unary_union(&explained));
                Some(BufferedExclusion {
                    geometry,
                    frame,
                    unexplained,
                    envelope,
                    threshold,
                })
            })
            .collect();
        Self { geometries, policy }
    }

    pub fn check_points<'a>(
        &self,
        points: impl IntoIterator<Item = &'a UnitVec>,
    ) -> Result<(), ExclusionError> {
        self.check_lines(&[unit_line(points)])
    }

    pub fn check_sequences<'a>(
        &self,
        sequences: impl IntoIterator<Item = &'a [UnitVec]>,
    ) -> Result<(), ExclusionError> {
        self.check_lines(
            &sequences
                .into_iter()
                .map(|points| unit_line(points.iter()))
                .collect::<Vec<_>>(),
        )
    }

    fn check_lines(&self, lines: &[LineString]) -> Result<(), ExclusionError> {
        let segments = segment_index(lines);
        for excluded in &self.geometries {
            let mut length = 0.0;
            for segment in segments.locate_in_envelope_intersecting(&excluded.envelope) {
                let line = excluded
                    .frame
                    .line(&LineString::from(vec![segment.start, segment.end]));
                let clipped = excluded
                    .unexplained
                    .clip(&MultiLineString(vec![line]), false);
                length += Euclidean.length(&clipped);
                if length > excluded.threshold {
                    return Err(ExclusionError::Excluded(ExcludedStretch {
                        geometry: excluded.geometry.clone(),
                        policy: self.policy,
                    }));
                }
            }
        }
        Ok(())
    }
}

pub fn check_points<'a>(
    points: impl IntoIterator<Item = &'a UnitVec>,
) -> Result<(), ExclusionError> {
    catalogue()?.check_points(points)
}

pub fn check_sequences<'a>(
    sequences: impl IntoIterator<Item = &'a [UnitVec]>,
) -> Result<(), ExclusionError> {
    catalogue()?.check_sequences(sequences)
}

pub fn check_timeline(timeline: &WorldTimeline) -> Result<(), ExclusionError> {
    check_sequences(timeline.boundaries.values().flat_map(|history| {
        history
            .versions
            .iter()
            .map(|(_, boundary)| boundary.pts.as_slice())
    }))
}

pub fn check_timeline_and_compiled(
    timeline: &WorldTimeline,
    store: &CanonStore,
) -> Result<(), ExclusionError> {
    check_inputs_and_sequences(
        &build_data_root(),
        timeline
            .boundaries
            .values()
            .flat_map(|history| {
                history
                    .versions
                    .iter()
                    .map(|(_, boundary)| boundary.pts.as_slice())
            })
            .chain(store.borders().values().map(|border| border.0.as_slice())),
    )
}

pub fn check_compiled(store: &CanonStore) -> Result<(), ExclusionError> {
    check_inputs_and_sequences(
        &build_data_root(),
        store.borders().values().map(|border| border.0.as_slice()),
    )
}

pub fn check_geojson(value: &serde_json::Value) -> Result<(), ExclusionError> {
    catalogue()?.check_lines(&geojson_lines(value))
}

pub fn check_build_inputs(root: &Path) -> Result<(), ExclusionError> {
    check_inputs_and_sequences(root, std::iter::empty())
}

pub fn check_inputs_and_sequences<'a>(
    root: &Path,
    sequences: impl IntoIterator<Item = &'a [UnitVec]>,
) -> Result<(), ExclusionError> {
    let mut lines = sequences
        .into_iter()
        .map(|points| unit_line(points.iter()))
        .collect::<Vec<_>>();
    catalogue()?.check_lines(&lines)?;
    collect_inputs(root, &mut lines)?;
    catalogue()?.check_lines(&lines)
}

fn build_data_root() -> PathBuf {
    let direct = PathBuf::from("data");
    if direct.is_dir() {
        direct
    } else {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data")
    }
}

fn collect_inputs(root: &Path, lines: &mut Vec<LineString>) -> Result<(), ExclusionError> {
    let entries = std::fs::read_dir(root).map_err(|error| ExclusionError::InputRead {
        path: root.to_path_buf(),
        message: error.to_string(),
    })?;
    let mut paths = Vec::new();
    for entry in entries {
        paths.push(
            entry
                .map_err(|error| ExclusionError::InputRead {
                    path: root.to_path_buf(),
                    message: error.to_string(),
                })?
                .path(),
        );
    }
    paths.sort();
    for path in paths {
        if path.is_dir() {
            collect_inputs(&path, lines)?;
        } else if path.file_name().is_some_and(|name| name == "polities.json") {
            let text =
                std::fs::read_to_string(&path).map_err(|error| ExclusionError::InputRead {
                    path: path.clone(),
                    message: error.to_string(),
                })?;
            let polities = crate::vendor::parse_polities(&text).map_err(|message| {
                ExclusionError::InputDecode {
                    path: path.clone(),
                    message,
                }
            })?;
            lines.extend(
                polities
                    .iter()
                    .flat_map(|polity| &polity.rings)
                    .map(|ring| {
                        ring.iter()
                            .map(|(lat, lon)| Coord { x: *lon, y: *lat })
                            .collect::<LineString>()
                    }),
            );
        } else if path
            .extension()
            .is_some_and(|extension| extension == "geojson")
        {
            let bytes = std::fs::read(&path).map_err(|error| ExclusionError::InputRead {
                path: path.clone(),
                message: error.to_string(),
            })?;
            let value =
                serde_json::from_slice(&bytes).map_err(|error| ExclusionError::InputDecode {
                    path: path.clone(),
                    message: error.to_string(),
                })?;
            collect_geojson(&value, lines);
        }
    }
    Ok(())
}

fn catalogue() -> Result<&'static LineageIndex, ExclusionError> {
    static CATALOGUE: OnceLock<Result<LineageIndex, ExclusionError>> = OnceLock::new();
    CATALOGUE
        .get_or_init(|| {
            let catalogue: Catalogue = serde_json::from_str(include_str!(
                "../../../data/authored/excluded-geometry-fingerprints.json"
            ))
            .map_err(|error| ExclusionError::CatalogueDecode(error.to_string()))?;
            if catalogue.base.is_empty()
                || catalogue.decision.is_empty()
                || catalogue.geometries.is_empty()
                || catalogue
                    .geometries
                    .iter()
                    .any(|geometry| geometry.coordinates.len() < 2)
                || catalogue
                    .permitted
                    .iter()
                    .any(|geometry| geometry.origin.is_empty() || geometry.source_sha256.is_empty())
            {
                return Err(ExclusionError::EmptyCatalogue);
            }
            let permitted = catalogue
                .permitted
                .into_iter()
                .flat_map(|geometry| {
                    let coordinates = match geometry.license {
                        NaturalEarthLicense::PublicDomain => geometry.coordinates,
                    };
                    coordinates.into_iter().map(|line| {
                        line.into_iter()
                            .map(|[lon, lat]| UnitVec::from_lat_lon_deg(lat, lon))
                            .collect()
                    })
                })
                .collect::<Vec<_>>();
            Ok(LineageIndex::new(
                catalogue.geometries,
                catalogue.policy,
                &permitted,
            ))
        })
        .as_ref()
        .map_err(Clone::clone)
}

struct MeterFrame(Point);

impl MeterFrame {
    fn line(&self, line: &LineString) -> LineString {
        Haversine
            .densify(line, MAXIMUM_GEODESIC_SEGMENT_METERS)
            .points()
            .map(|point| {
                let distance = Haversine.distance(self.0, point);
                let bearing = Haversine.bearing(self.0, point).to_radians();
                let (east, north) = bearing.sin_cos();
                Coord {
                    x: distance * east,
                    y: distance * north,
                }
            })
            .collect()
    }
}

const MAXIMUM_GEODESIC_SEGMENT_METERS: f64 = 10_000.0;

fn segment_index(lines: &[LineString]) -> RTree<geo::Line> {
    RTree::bulk_load(
        lines
            .iter()
            .flat_map(|line| {
                Haversine
                    .densify(line, MAXIMUM_GEODESIC_SEGMENT_METERS)
                    .lines()
                    .collect::<Vec<_>>()
            })
            .collect(),
    )
}

fn unit_line<'a>(points: impl IntoIterator<Item = &'a UnitVec>) -> LineString {
    points
        .into_iter()
        .map(|point| {
            let (lat, lon) = point.to_lat_lon_deg();
            Coord { x: lon, y: lat }
        })
        .collect()
}

fn geographic_line(coordinates: &[[f64; 2]]) -> LineString {
    coordinates
        .iter()
        .map(|[lon, lat]| Coord { x: *lon, y: *lat })
        .collect()
}

pub fn geojson_lines(value: &serde_json::Value) -> Vec<LineString> {
    let mut lines = Vec::new();
    collect_geojson(value, &mut lines);
    lines
}

fn collect_geojson(value: &serde_json::Value, lines: &mut Vec<LineString>) {
    match value {
        serde_json::Value::Object(fields) => {
            for (name, value) in fields {
                if name == "coordinates" {
                    collect_coordinates(value, lines);
                } else {
                    collect_geojson(value, lines);
                }
            }
        }
        serde_json::Value::Array(values) => {
            for value in values {
                collect_geojson(value, lines);
            }
        }
        _ => {}
    }
}

fn collect_coordinates(value: &serde_json::Value, lines: &mut Vec<LineString>) {
    let Some(values) = value.as_array() else {
        return;
    };
    let point = |position: &serde_json::Value| {
        let values = position.as_array()?;
        let [lon, lat, ..] = values.as_slice() else {
            return None;
        };
        Some(Coord {
            x: lon.as_f64()?,
            y: lat.as_f64()?,
        })
    };
    if point(value).is_some() {
        return;
    }
    if values.first().and_then(point).is_some() {
        lines.push(values.iter().filter_map(point).collect());
    } else {
        for value in values {
            collect_coordinates(value, lines);
        }
    }
}
