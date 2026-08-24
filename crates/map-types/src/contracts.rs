//! The atlas-facing import contracts (C2, C3, C6). Direction of
//! authority: atlas -> map. The map system consumes these exports as
//! adapter sources with provenance "bible-atlas@<version-root>" and
//! never re-derives what they state.

use std::collections::BTreeMap;

use atlas_graph_types::covenant::ResolvedPlacement;
use atlas_graph_types::covenant::{ContentHash, EventId, PlaceId};

use crate::geom::UnitVec;

/// An era/reign span from the atlas chronology: dates the text gives
/// only at span width can live here honestly instead of as fake
/// point-years.
#[derive(Clone, Debug, PartialEq)]
pub struct ChronoSpan {
    pub label: String,
    pub from_year: i32,
    pub to_year: i32,
}

/// C2 — Chronology authority: the atlas's resolved placements
/// (traditional chronology, creation-anchored) are THE dates for every
/// Scripture-attested change. Law 12a checks every driven ChangeEvent
/// against this table.
#[derive(Clone, Debug, PartialEq)]
pub struct ChronologyExport {
    pub atlas_root: ContentHash,
    pub placements: BTreeMap<EventId, ResolvedPlacement>,
    pub spans: Vec<ChronoSpan>,
}

/// C3 — Gazetteer: atlas Place nodes are the coordinate authority.
/// Survey waypoints resolve here (law 12c); a place moving in the
/// atlas moves every border built through it. Resolution metadata
/// rides along so honesty renders per-identification: aliases (the
/// correction layer over canonical names), provenance, attestation
/// loci as the atlas serializes them. Absent metadata is absent —
/// never a fabricated default.
#[derive(Clone, Debug, PartialEq)]
pub struct GazetteerEntry {
    pub canonical_name: String,
    pub position: UnitVec,
    pub aliases: Vec<String>,
    pub provenance: Option<String>,
    pub attestations: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GazetteerExport {
    pub atlas_root: ContentHash,
    pub places: BTreeMap<PlaceId, GazetteerEntry>,
}
