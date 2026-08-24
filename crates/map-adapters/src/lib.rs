//! map-generator source adapters — phase 2 of docs/map-system-handoff.md.
//!
//! An adapter turns ONE source's bytes into a lawful `WorldTimeline`
//! behind the `TimelineSource` seam. The phase-2 law is FIDELITY: what
//! came out is exactly what went in (quantized by the disclosed
//! method), proven ring-for-ring by `fidelity_violations` and by the
//! tests, which also run every map-types data validator over real
//! ingested output.

pub mod arcs;
pub mod basemaps;
pub mod geojson;
pub mod hydro;
pub mod quantize;
pub mod surveys;

pub use basemaps::{
    epoch_year_from_label, fidelity_violations, ingest, EpochSource, Exemption, HistoricalBasemaps,
    Ingest, IngestConfig, IngestError, TimelineSource,
};
pub use hydro::{ingest_ocean, ingest_water, WaterSource};
pub use surveys::{
    merge_timelines, promised_land_timeline, scripture_timeline, stand_in_gazetteer, MergeError,
};

#[cfg(test)]
mod tests;
