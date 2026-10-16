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
pub mod exports;
pub mod geojson;
pub mod hydro;
pub mod plate_water;
pub mod quantize;
pub mod surveys;
pub mod terrain;

pub use basemaps::{
    epoch_year_from_label, fidelity_violations, ingest, EpochSource, Exemption, HistoricalBasemaps,
    Ingest, IngestConfig, IngestError, TimelineSource,
};
pub use exports::{load_exports, AtlasExports, ExportError};
pub use hydro::{ingest_ocean, ingest_water, WaterSource};
pub use plate_water::{plate_chart, plate_river_paths, plate_water_timeline, plate_water_witnesses};
pub use surveys::plate_canaan_ring;
pub use surveys::{authored_routes, AuthoredRoute};
pub use terrain::{ingest_terrain, ElevationGrid};
pub use surveys::{
    binding_report, merge_timelines, merged_gazetteer, promised_land_timeline,
    scripture_timeline, scripture_timeline_with, stand_in_gazetteer, BindingRow, MergeError,
};

#[cfg(test)]
mod tests;

fn scripture(range: atlas_graph_types::text::BibleLocusRange) -> atlas_graph_types::edge::Ground {
    use atlas_graph_types::passage::{Reading, Written};
    assert!(range.from.span.is_none() && range.to.span.is_none(), "map Scripture evidence must cite whole source verses");
    let first = range.from.unit;
    let last = range.to.unit;
    let written = Written::new(first.clone(), last.clone()).expect("an admitted Scripture range is ordered");
    let units = if first == last { vec![first] } else { vec![first, last] };
    let reading = Reading::of(units).expect("an admitted Scripture range has ordered endpoints");
    atlas_graph_types::edge::Ground::Scripture(reading.place(&written).expect("the cited endpoints are present in their admitted reading"))
}

#[cfg(test)]
mod scripture_admission_tests {
    #[test]
    fn whole_scripture_admission_retains_generated_cited_bounds() {
        use atlas_graph_types::text::{BibleLocus, LocusRange, VerseRef};
        // Arrange
        for book in 1..=66 {
            let first = VerseRef { book: atlas_graph_types::text::BookId::new(book).expect("the original book index is admitted"), chapter: 1, verse: 1 };
            let last = VerseRef { book: atlas_graph_types::text::BookId::new(book).expect("the original book index is admitted"), chapter: 1, verse: 2 };
            let range = LocusRange::new(BibleLocus::whole(first.clone()), BibleLocus::whole(last.clone())).unwrap();
            // Act
            let atlas_graph_types::edge::Ground::Scripture(span) = super::scripture(range) else { panic!("Scripture admission must remain Scripture"); };
            // Assert
            assert_eq!(span.first(), &first, "the original first cited verse survives source admission");
            assert_eq!(span.last(), &last, "the original last cited verse survives source admission");
        }
    }
}
