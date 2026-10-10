use map_compile::exclusion::{ExcludedGeometry, ExcludedSource};
use map_types::UnitVec;
use serde::Deserialize;

#[derive(Deserialize)]
struct SourceGeometry {
    source: ExcludedSource,
    geometry: String,
    origin: String,
    source_sha256: String,
    vertices: Vec<[f64; 2]>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rows: Vec<SourceGeometry> = serde_json::from_reader(std::io::stdin().lock())?;
    let geometries: Vec<_> = rows
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
    serde_json::to_writer(std::io::stdout().lock(), &geometries)?;
    Ok(())
}
