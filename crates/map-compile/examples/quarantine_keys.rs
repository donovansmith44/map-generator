use geo::{BooleanOps, BoundingRect, Destination, Haversine, LineString, MultiLineString};
use map_compile::exclusion::{
    geojson_lines, ExcludedGeometry, ExcludedSource, GeometricPolicy, NaturalEarthLicense,
};
use map_types::UnitVec;
use serde::Deserialize;

#[derive(Deserialize)]
struct Input {
    policy: GeometricPolicy,
    geometries: Vec<SourceGeometry>,
    permitted: Vec<PermittedSource>,
}

#[derive(Deserialize)]
struct SourceGeometry {
    source: ExcludedSource,
    geometry: String,
    origin: String,
    source_sha256: String,
    vertices: Vec<[f64; 2]>,
}

#[derive(Deserialize)]
struct PermittedSource {
    origin: String,
    source_sha256: String,
    license: NaturalEarthLicense,
    value: serde_json::Value,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let input: Input = serde_json::from_reader(std::io::stdin().lock())?;
    let bounds: LineString = input
        .geometries
        .iter()
        .flat_map(|row| row.vertices.iter().map(|[lat, lon]| (*lon, *lat)))
        .collect();
    let bounds = bounds
        .bounding_rect()
        .ok_or("quarantine contains no coordinates")?;
    let margin = input.policy.tolerance_meters;
    let southwest = Haversine.destination(bounds.min().into(), 225.0, margin * 2.0_f64.sqrt());
    let northeast = Haversine.destination(bounds.max().into(), 45.0, margin * 2.0_f64.sqrt());
    let clip = geo::Rect::new(southwest.0, northeast.0).to_polygon();
    let geometries: Vec<_> = input
        .geometries
        .into_iter()
        .map(|row| {
            let points: Vec<_> = row
                .vertices
                .into_iter()
                .map(|[lat, lon]| UnitVec::from_lat_lon_deg(lat, lon))
                .collect();
            ExcludedGeometry::from_points(
                row.source,
                row.geometry,
                row.origin,
                row.source_sha256,
                &points,
            )
        })
        .collect();
    let permitted: Vec<_> = input.permitted.into_iter().map(|row| {
        let lines = geojson_lines(&row.value);
        let lines = clip.clip(&MultiLineString(lines), false);
        let coordinates: Vec<Vec<_>> = lines.0.into_iter().map(|line| line.0.into_iter().map(|point| [point.x, point.y]).collect()).collect();
        serde_json::json!({"origin":row.origin,"source_sha256":row.source_sha256,"license":row.license,"coordinates":coordinates})
    }).collect();
    serde_json::to_writer(
        std::io::stdout().lock(),
        &serde_json::json!({"geometries":geometries,"permitted":permitted}),
    )?;
    Ok(())
}
