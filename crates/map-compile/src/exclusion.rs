use std::collections::BTreeSet;
use std::sync::OnceLock;

use map_canon::CanonStore;
use map_types::{UnitVec, WorldTimeline};
use serde::Deserialize;
use sha2::{Digest, Sha256};

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
pub enum ExcludedSource {
    KnowingTheBible,
    Tribes12,
    SplicedRegions,
    HistoricalBasemaps,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
struct Fingerprint(String);

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct ExcludedGeometry {
    pub source: ExcludedSource,
    pub geometry: String,
    pub origin: String,
    pub source_sha256: String,
    vertices: BTreeSet<Fingerprint>,
}

#[derive(Deserialize)]
struct Catalogue {
    base: String,
    decision: String,
    geometries: Vec<ExcludedGeometry>,
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
    Excluded(Vec<ExcludedGeometry>),
}

pub fn check_points<'a>(
    points: impl IntoIterator<Item = &'a UnitVec>,
) -> Result<(), ExclusionError> {
    let observed: BTreeSet<_> = points.into_iter().map(fingerprint).collect();
    let excluded: Vec<_> = catalogue()?
        .iter()
        .filter(|geometry| geometry.vertices.is_subset(&observed))
        .cloned()
        .collect();
    if excluded.is_empty() {
        Ok(())
    } else {
        Err(ExclusionError::Excluded(excluded))
    }
}

pub fn check_timeline(timeline: &WorldTimeline) -> Result<(), ExclusionError> {
    check_points(timeline.boundaries.values().flat_map(|history| {
        history
            .versions
            .iter()
            .flat_map(|(_, boundary)| &boundary.pts)
    }))
}

pub fn check_compiled(store: &CanonStore) -> Result<(), ExclusionError> {
    check_points(store.borders().values().flat_map(|border| &border.0))
}

pub fn check_geojson(value: &serde_json::Value) -> Result<(), ExclusionError> {
    let mut points = Vec::new();
    geojson_points(value, &mut points);
    check_points(&points)
}

pub fn check_build_inputs(root: &std::path::Path) -> Result<(), ExclusionError> {
    let mut points = Vec::new();
    input_points(root, &mut points)?;
    check_points(&points)
}

fn input_points(root: &std::path::Path, points: &mut Vec<UnitVec>) -> Result<(), ExclusionError> {
    let entries = std::fs::read_dir(root).map_err(|error| ExclusionError::InputRead {
        path: root.to_path_buf(),
        message: error.to_string(),
    })?;
    for entry in entries {
        let entry = entry.map_err(|error| ExclusionError::InputRead {
            path: root.to_path_buf(),
            message: error.to_string(),
        })?;
        let path = entry.path();
        let kind = entry
            .file_type()
            .map_err(|error| ExclusionError::InputRead {
                path: path.clone(),
                message: error.to_string(),
            })?;
        if kind.is_dir() {
            input_points(&path, points)?;
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
            geojson_points(&value, points);
        }
    }
    Ok(())
}

fn catalogue() -> Result<&'static [ExcludedGeometry], ExclusionError> {
    static CATALOGUE: OnceLock<Result<Catalogue, ExclusionError>> = OnceLock::new();
    let catalogue = CATALOGUE.get_or_init(|| {
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
        Ok(catalogue)
    });
    catalogue
        .as_ref()
        .map(|catalogue| catalogue.geometries.as_slice())
        .map_err(Clone::clone)
}

fn fingerprint(point: &UnitVec) -> Fingerprint {
    let mut hash = Sha256::new();
    for value in [point.x(), point.y(), point.z()] {
        hash.update(((value * 1e9).round() as i64).to_be_bytes());
    }
    Fingerprint(format!("{:x}", hash.finalize()))
}

fn geojson_points(value: &serde_json::Value, points: &mut Vec<UnitVec>) {
    match value {
        serde_json::Value::Object(fields) => {
            for (name, value) in fields {
                if name == "coordinates" {
                    coordinate_points(value, points);
                } else {
                    geojson_points(value, points);
                }
            }
        }
        serde_json::Value::Array(values) => {
            for value in values {
                geojson_points(value, points);
            }
        }
        _ => {}
    }
}

fn coordinate_points(value: &serde_json::Value, points: &mut Vec<UnitVec>) {
    let Some(values) = value.as_array() else {
        return;
    };
    if let [lon, lat, ..] = values.as_slice() {
        if let (Some(lon), Some(lat)) = (lon.as_f64(), lat.as_f64()) {
            points.push(UnitVec::from_lat_lon_deg(lat, lon));
            return;
        }
    }
    for value in values {
        coordinate_points(value, points);
    }
}
