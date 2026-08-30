//! The map provider crate: `canon_provider::CanonProvider` serves the
//! full MapProvider contract from the compiled canon — the only truth
//! store (phase 6: data in, maps out). The phase-3 interval-timeline
//! reference provider lived here until the canon provider absorbed its
//! last capability (morph transitions); it is gone, as promised.

/// The source id Scripture-surveyed geometry carries into scenes: the
/// authority ladder (covenant rule 11) made visible in attribution and
/// selectable by consumers (a "Bible mode" is a semantic filter on it).
pub const SCRIPTURE_SOURCE: &str = "scripture";

#[cfg(test)]
mod tests;
pub mod canon_provider;
