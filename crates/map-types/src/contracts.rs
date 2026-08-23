//! The atlas-facing import contracts (C2, C3, C6). Direction of
//! authority: atlas -> map. The map system consumes these exports as
//! adapter sources with provenance "bible-atlas@<version-root>" and
//! never re-derives what they state.

use std::collections::BTreeMap;

use atlas_graph_types::chrono::ResolvedPlacement;
use atlas_graph_types::id::{ContentHash, EventId, PlaceId};

use crate::geom::UnitVec;

/// C2 — Chronology authority: the atlas's resolved placements
/// (traditional chronology, Ussher-anchored) are THE dates for every
/// Scripture-attested change. Law 12a checks every driven ChangeEvent
/// against this table.
#[derive(Clone, Debug, PartialEq)]
pub struct ChronologyExport {
    pub atlas_root: ContentHash,
    pub placements: BTreeMap<EventId, ResolvedPlacement>,
}

/// C3 — Gazetteer: atlas Place nodes are the coordinate authority.
/// Survey waypoints resolve here (law 12c); a place moving in the
/// atlas moves every border built through it.
#[derive(Clone, Debug, PartialEq)]
pub struct GazetteerEntry {
    pub canonical_name: String,
    pub position: UnitVec,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GazetteerExport {
    pub atlas_root: ContentHash,
    pub places: BTreeMap<PlaceId, GazetteerEntry>,
}
