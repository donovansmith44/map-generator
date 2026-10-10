use std::collections::{BTreeMap, VecDeque};
use std::sync::OnceLock;

use map_canon::CanonStore;
use map_partition::PointKey;
use map_types::{UnitVec, WorldTimeline};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum ExcludedSource {
    KnowingTheBible,
    Tribes12,
    SplicedRegions,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
struct Fingerprint(String);

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ExcludedGeometry {
    pub source: ExcludedSource,
    pub geometry: String,
    pub origin: String,
    pub source_sha256: String,
    vertices: Vec<[Fingerprint; PointKey::LINEAGE_NEIGHBOR_COUNT]>,
}

impl ExcludedGeometry {
    pub fn from_points(
        source: ExcludedSource,
        geometry: String,
        origin: String,
        source_sha256: String,
        points: &[UnitVec],
    ) -> Self {
        let mut vertices: Vec<_> = points
            .iter()
            .map(|point| PointKey::lineage_neighborhood(point).map(fingerprint))
            .collect();
        if vertices.len() > 1 && vertices.first() == vertices.last() {
            vertices.pop();
        }
        Self {
            source,
            geometry,
            origin,
            source_sha256,
            vertices,
        }
    }
}

#[derive(Deserialize)]
struct Catalogue {
    base: String,
    decision: String,
    geometries: Vec<ExcludedGeometry>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunDirection {
    Forward,
    Reverse,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExcludedRun {
    pub geometry: ExcludedGeometry,
    pub input_start: usize,
    pub ring_start: usize,
    pub vertex_count: usize,
    pub direction: RunDirection,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExclusionError {
    CatalogueDecode(String),
    EmptyCatalogue,
    InputRead {
        path: std::path::PathBuf,
        message: String,
    },
    InputDecode {
        path: std::path::PathBuf,
        message: String,
    },
    Excluded(ExcludedRun),
}

#[derive(Clone, Copy)]
struct VertexLocation {
    geometry: usize,
    vertex: usize,
}

pub struct LineageIndex {
    geometries: Vec<ExcludedGeometry>,
    vertices: BTreeMap<Fingerprint, Vec<VertexLocation>>,
}

impl LineageIndex {
    pub const REFUSED_RUN_LENGTH: usize = 3;

    pub fn new(geometries: Vec<ExcludedGeometry>) -> Self {
        let mut vertices: BTreeMap<Fingerprint, Vec<VertexLocation>> = BTreeMap::new();
        for (geometry, ring) in geometries.iter().enumerate() {
            for (vertex, neighborhood) in ring.vertices.iter().enumerate() {
                for fingerprint in neighborhood {
                    vertices
                        .entry(fingerprint.clone())
                        .or_default()
                        .push(VertexLocation { geometry, vertex });
                }
            }
        }
        Self {
            geometries,
            vertices,
        }
    }

    pub fn check_points<'a>(
        &self,
        points: impl IntoIterator<Item = &'a UnitVec>,
    ) -> Result<(), ExclusionError> {
        let mut window = VecDeque::new();
        for (position, point) in points.into_iter().enumerate() {
            window.push_back(fingerprint(PointKey::lineage(point)));
            if window.len() < Self::REFUSED_RUN_LENGTH {
                continue;
            }
            let mut candidates = self.vertices.get(&window[0]).cloned().unwrap_or_default();
            candidates.sort_by_key(|candidate| (candidate.geometry, candidate.vertex));
            for candidate in candidates {
                let ring = &self.geometries[candidate.geometry];
                if ring.vertices.len() < Self::REFUSED_RUN_LENGTH {
                    continue;
                }
                for direction in [RunDirection::Forward, RunDirection::Reverse] {
                    let matches = window.iter().enumerate().all(|(offset, keys)| {
                        let vertex = match direction {
                            RunDirection::Forward => {
                                (candidate.vertex + offset) % ring.vertices.len()
                            }
                            RunDirection::Reverse => {
                                (candidate.vertex + ring.vertices.len() - offset)
                                    % ring.vertices.len()
                            }
                        };
                        ring.vertices[vertex].contains(keys)
                    });
                    if matches {
                        return Err(ExclusionError::Excluded(ExcludedRun {
                            geometry: ring.clone(),
                            input_start: position + 1 - Self::REFUSED_RUN_LENGTH,
                            ring_start: candidate.vertex,
                            vertex_count: Self::REFUSED_RUN_LENGTH,
                            direction,
                        }));
                    }
                }
            }
            window.pop_front();
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
    for points in sequences {
        check_points(points)?;
    }
    Ok(())
}

pub fn check_timeline(timeline: &WorldTimeline) -> Result<(), ExclusionError> {
    check_sequences(timeline.boundaries.values().flat_map(|history| {
        history
            .versions
            .iter()
            .map(|(_, boundary)| boundary.pts.as_slice())
    }))
}

pub fn check_compiled(store: &CanonStore) -> Result<(), ExclusionError> {
    check_sequences(store.borders().values().map(|border| border.0.as_slice()))
}

pub fn check_geojson(value: &serde_json::Value) -> Result<(), ExclusionError> {
    match value {
        serde_json::Value::Object(fields) => {
            for (name, value) in fields {
                if name == "coordinates" {
                    check_coordinates(value)?;
                } else {
                    check_geojson(value)?;
                }
            }
        }
        serde_json::Value::Array(values) => {
            for value in values {
                check_geojson(value)?;
            }
        }
        _ => {}
    }
    Ok(())
}

pub fn check_build_inputs(root: &std::path::Path) -> Result<(), ExclusionError> {
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
            check_build_inputs(&path)?;
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
            check_geojson(&value)?;
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
                    .any(|geometry| geometry.vertices.is_empty())
            {
                return Err(ExclusionError::EmptyCatalogue);
            }
            Ok(LineageIndex::new(catalogue.geometries))
        })
        .as_ref()
        .map_err(Clone::clone)
}

fn fingerprint(key: PointKey) -> Fingerprint {
    Fingerprint(format!("{:x}", Sha256::digest(key.bytes())))
}

fn check_coordinates(value: &serde_json::Value) -> Result<(), ExclusionError> {
    let Some(values) = value.as_array() else {
        return Ok(());
    };
    let point = |position: &serde_json::Value| {
        let values = position.as_array()?;
        let [lon, lat, ..] = values.as_slice() else {
            return None;
        };
        Some(UnitVec::from_lat_lon_deg(lat.as_f64()?, lon.as_f64()?))
    };
    if point(value).is_some() {
        return Ok(());
    }
    if values.first().and_then(point).is_some() {
        let points: Vec<_> = values.iter().filter_map(point).collect();
        check_points(&points)
    } else {
        for value in values {
            check_coordinates(value)?;
        }
        Ok(())
    }
}
