//! The C2/C3 import adapter: the atlas's chronology and gazetteer
//! export artifacts become the map system's dating and coordinate
//! AUTHORITY (covenant rule 11 across repos). Direction of authority:
//! atlas -> map; nothing here re-derives or re-dates.
//!
//! Artifacts are text files carrying format_version and
//! atlas_version_root; the root feeds the C6 stale-pin. Binding rules,
//! agreed with the atlas session: places bind CANONICAL-first with
//! aliases as a correction layer; events bind by attestation-verse
//! intersection with our justification ranges, ambiguity left unbound
//! rather than guessed.

use std::collections::BTreeMap;

use atlas_graph_types::covenant::{
    ContentHash, EventId, PlaceId, PlacementBasis, ResolvedDate, ResolvedPlacement, SeqKey,
    TimePoint, Year,
};

use map_types::{ChronoSpan, ChronologyExport, GazetteerEntry, GazetteerExport, UnitVec};

use serde_json::Value;

/// The format this adapter reads. A bump atlas-side without one here
/// is a loud error, never a silent misparse.
pub const SUPPORTED_FORMAT_VERSION: i64 = 1;

#[derive(Clone, Debug, PartialEq)]
pub enum ExportError {
    BadSyntax(String),
    BadShape(&'static str),
    /// The artifact's format_version is one this adapter doesn't speak.
    UnsupportedFormat(i64),
    /// The two artifacts disagree about the atlas root — they must be
    /// from one compile (the atlas asserts this by law; we re-check).
    RootMismatch,
}

/// KJV book abbreviations as the atlas serializes attestation loci,
/// in canon order — index+1 is the book number our VerseRef speaks.
const BOOKS: [&str; 66] = [
    "GEN", "EXO", "LEV", "NUM", "DEU", "JOS", "JDG", "RUT", "1SA", "2SA", "1KI", "2KI", "1CH",
    "2CH", "EZR", "NEH", "EST", "JOB", "PSA", "PRO", "ECC", "SNG", "ISA", "JER", "LAM", "EZK",
    "DAN", "HOS", "JOL", "AMO", "OBA", "JON", "MIC", "NAM", "HAB", "ZEP", "HAG", "ZEC", "MAL",
    "MAT", "MRK", "LUK", "JHN", "ACT", "ROM", "1CO", "2CO", "GAL", "EPH", "PHP", "COL", "1TH",
    "2TH", "1TI", "2TI", "TIT", "PHM", "HEB", "JAS", "1PE", "2PE", "1JN", "2JN", "3JN", "JUD",
    "REV",
];

/// "2KI.5.12" -> (12, 5, 12): (book number, chapter, verse).
pub fn parse_locus(s: &str) -> Option<(u8, u16, u16)> {
    let mut it = s.split('.');
    let book = it.next()?;
    let chapter: u16 = it.next()?.parse().ok()?;
    let verse: u16 = it.next()?.parse().ok()?;
    let n = BOOKS.iter().position(|b| b.eq_ignore_ascii_case(book))? as u8 + 1;
    Some((n, chapter, verse))
}

fn root_of(v: &Value) -> Result<(ContentHash, i64), ExportError> {
    let fv = v
        .get("format_version")
        .and_then(Value::as_i64)
        .ok_or(ExportError::BadShape("no format_version"))?;
    if fv != SUPPORTED_FORMAT_VERSION {
        return Err(ExportError::UnsupportedFormat(fv));
    }
    let root = v
        .get("atlas_version_root")
        .and_then(Value::as_str)
        .ok_or(ExportError::BadShape("no atlas_version_root"))?;
    let hash =
        u64::from_str_radix(root, 16).map_err(|_| ExportError::BadShape("bad root hex"))?;
    Ok((ContentHash(hash), fv))
}

/// One atlas event row, kept in binding-friendly form.
#[derive(Clone, Debug)]
pub struct AtlasEvent {
    pub id: String,
    pub label: String,
    pub attestations: Vec<(u8, u16, u16)>,
    pub from_year: i32,
    pub to_year: i32,
}

/// The loaded authority: everything the builders need to bind.
pub struct AtlasExports {
    pub root: ContentHash,
    /// (id, year, citation) anchor rows — creation lives here.
    pub anchors: Vec<(String, i32, String)>,
    pub gazetteer: GazetteerExport,
    pub chronology: ChronologyExport,
    pub events: Vec<AtlasEvent>,
    /// lowercase canonical/alias name -> place row index.
    name_index: BTreeMap<String, usize>,
    places: Vec<(String, f64, f64)>, // (atlas place id, lat, lon)
}

impl AtlasExports {
    /// Canonical-first, alias-as-correction-layer place resolution.
    /// Parentheticals in OUR waypoint names ("Kiriath-jearim (Baalah)")
    /// are stripped before the lookup.
    pub fn resolve_place(&self, name: &str) -> Option<(PlaceId, f64, f64)> {
        let bare = match name.split(" (").next() {
            Some(b) => b,
            None => name,
        };
        let idx = self
            .name_index
            .get(&name.to_lowercase())
            .or_else(|| self.name_index.get(&bare.to_lowercase()))?;
        let (id, lat, lon) = &self.places[*idx];
        Some((PlaceId::new(id.clone()), *lat, *lon))
    }

    /// Bind by attestation: the atlas event whose attested verses fall
    /// inside our justification range [book ch:v .. ch:v]. One clear
    /// winner (most verses inside; unique) binds; ambiguity stays
    /// unbound — honesty over guesswork.
    pub fn resolve_event(
        &self,
        book: u8,
        from: (u16, u16),
        to: (u16, u16),
    ) -> Option<(EventId, i32, i32)> {
        let lo = (book, from.0, from.1);
        let hi = (book, to.0, to.1);
        let mut best: Option<(usize, usize)> = None; // (index, hits)
        let mut tied = false;
        for (i, e) in self.events.iter().enumerate() {
            let hits =
                e.attestations.iter().filter(|v| **v >= lo && **v <= hi).count();
            if hits == 0 {
                continue;
            }
            match best {
                Some((_, b)) if hits < b => {}
                Some((_, b)) if hits == b => tied = true,
                _ => {
                    best = Some((i, hits));
                    tied = false;
                }
            }
        }
        let (i, _) = best?;
        if tied {
            return None;
        }
        let e = &self.events[i];
        Some((EventId::new(e.id.clone()), e.from_year, e.to_year))
    }

    /// The atlas's creation anchor row, when present: (year, citation).
    pub fn creation_anchor(&self) -> Option<(i32, String)> {
        self.anchors
            .iter()
            .find(|(id, ..)| id == "creation")
            .map(|(_, year, citation)| (*year, citation.clone()))
    }
}

pub fn load_exports(gazetteer_text: &str, chronology_text: &str) -> Result<AtlasExports, ExportError> {
    let g: Value =
        serde_json::from_str(gazetteer_text).map_err(|e| ExportError::BadSyntax(e.to_string()))?;
    let c: Value =
        serde_json::from_str(chronology_text).map_err(|e| ExportError::BadSyntax(e.to_string()))?;
    let (g_root, _) = root_of(&g)?;
    let (c_root, _) = root_of(&c)?;
    if g_root != c_root {
        return Err(ExportError::RootMismatch);
    }

    // ---- gazetteer ----
    let mut places = Vec::new();
    let mut name_index = BTreeMap::new();
    let mut entries = BTreeMap::new();
    for p in g.get("places").and_then(Value::as_array).ok_or(ExportError::BadShape("no places"))?
    {
        let id = p.get("id").and_then(Value::as_str).ok_or(ExportError::BadShape("place id"))?;
        let canonical = p
            .get("canonical")
            .and_then(Value::as_str)
            .ok_or(ExportError::BadShape("place canonical"))?;
        let (Some(lat), Some(lon)) =
            (p.get("lat").and_then(Value::as_f64), p.get("lon").and_then(Value::as_f64))
        else {
            return Err(ExportError::BadShape("place lat/lon"));
        };
        let aliases: Vec<String> = p
            .get("aliases")
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(Value::as_str).map(str::to_string).collect())
            .unwrap_or_default();
        let attestations: Vec<String> = p
            .get("attestations")
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(Value::as_str).map(str::to_string).collect())
            .unwrap_or_default();
        let idx = places.len();
        name_index.entry(canonical.to_lowercase()).or_insert(idx);
        for a in &aliases {
            name_index.entry(a.to_lowercase()).or_insert(idx);
        }
        places.push((id.to_string(), lat, lon));
        entries.insert(
            PlaceId::new(id.to_string()),
            GazetteerEntry {
                canonical_name: canonical.to_string(),
                position: UnitVec::from_lat_lon_deg(lat, lon),
                aliases,
                provenance: p
                    .get("provenance")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                attestations,
            },
        );
    }

    // ---- chronology ----
    let year = |v: i64| -> Result<TimePoint, ExportError> {
        Year::new(v as i32)
            .map(TimePoint::year_only)
            .map_err(|_| ExportError::BadShape("year zero in chronology"))
    };
    let mut events = Vec::new();
    let mut placements = BTreeMap::new();
    for (seq, e) in c
        .get("events")
        .and_then(Value::as_array)
        .ok_or(ExportError::BadShape("no events"))?
        .iter()
        .enumerate()
    {
        let id = e.get("id").and_then(Value::as_str).ok_or(ExportError::BadShape("event id"))?;
        let label = e.get("label").and_then(Value::as_str).unwrap_or("").to_string();
        let pl = e.get("placement").ok_or(ExportError::BadShape("event placement"))?;
        let (Some(fy), Some(ty)) = (
            pl.get("from_year").and_then(Value::as_i64),
            pl.get("to_year").and_then(Value::as_i64),
        ) else {
            return Err(ExportError::BadShape("placement years"));
        };
        let basis = match pl.get("basis").and_then(Value::as_str) {
            Some("Textual") => PlacementBasis::Textual,
            _ => PlacementBasis::Traditional,
        };
        let attestations: Vec<(u8, u16, u16)> = e
            .get("attestations")
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(Value::as_str).filter_map(parse_locus).collect())
            .unwrap_or_default();
        placements.insert(
            EventId::new(id.to_string()),
            ResolvedPlacement {
                date: ResolvedDate { from: year(fy)?, to: year(ty.max(fy))? },
                seq: SeqKey(seq as u32),
                basis,
            },
        );
        events.push(AtlasEvent {
            id: id.to_string(),
            label,
            attestations,
            from_year: fy as i32,
            to_year: ty as i32,
        });
    }
    let mut spans = Vec::new();
    for s in c.get("spans").and_then(Value::as_array).unwrap_or(&Vec::new()) {
        let (Some(label), Some(f), Some(t)) = (
            s.get("label").and_then(Value::as_str),
            s.get("from").and_then(Value::as_i64),
            s.get("to").and_then(Value::as_i64),
        ) else {
            continue;
        };
        spans.push(ChronoSpan { label: label.to_string(), from_year: f as i32, to_year: t as i32 });
    }

    let mut anchors = Vec::new();
    for a in c.get("anchors").and_then(Value::as_array).unwrap_or(&Vec::new()) {
        let (Some(id), Some(y)) = (
            a.get("id").and_then(Value::as_str),
            a.get("at").and_then(|t| t.get("year")).and_then(Value::as_i64),
        ) else {
            continue;
        };
        let citation = a.get("citation").and_then(Value::as_str).unwrap_or("").to_string();
        anchors.push((id.to_string(), y as i32, citation));
    }

    Ok(AtlasExports {
        root: g_root,
        anchors,
        gazetteer: GazetteerExport { atlas_root: g_root, places: entries },
        chronology: ChronologyExport { atlas_root: c_root, placements, spans },
        events,
        name_index,
        places,
    })
}
