use map_compile::exclusion::{
    geojson_lines, ExcludedGeometry, ExcludedSource, NaturalEarthLicense,
};
use map_types::UnitVec;
use serde::Deserialize;

#[derive(Deserialize)]
struct Input {
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
        let coordinates: Vec<Vec<_>> = lines.into_iter().map(|line| line.0.into_iter().map(|point| [point.x, point.y]).collect()).collect();
        serde_json::json!({"origin":row.origin,"source_sha256":row.source_sha256,"license":row.license,"coordinates":coordinates})
    }).collect();
    serde_json::to_writer(
        std::io::stdout().lock(),
        &serde_json::json!({"geometries":geometries,"permitted":permitted}),
    )?;
    Ok(())
}
