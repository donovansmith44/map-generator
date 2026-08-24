//! Bible-driven borders (covenant rule 11, spec §B BorderSurvey):
//! Scripture contains literal border SURVEYS, and a survey-derived
//! boundary is CONSTRUCTED from the text — its waypoints are place
//! references, its interpolation between waypoints is the only
//! authored geometry (method disclosed), and its justification grounds
//! are the survey verses themselves.
//!
//! STAND-IN DISCLOSURE (recorded for owner review, rides every
//! provenance string): the atlas gazetteer (contract C3) is the
//! coordinate authority and its export does not exist yet. Until it
//! does, waypoint coordinates here are TRADITIONAL IDENTIFICATIONS,
//! approximate, authored as a stand-in; several northern and eastern
//! identifications are scholarly guesses. When the atlas C3 export
//! lands, these coordinates are REPLACED by atlas PlaceId resolution —
//! one fact, one home — and law 12c already checks that resolution.

use std::collections::hash_map::DefaultHasher;
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};

use atlas_graph_types::chrono::{TimePoint, Year};
use atlas_graph_types::edge::{Ground, Justification};
use atlas_graph_types::id::{ContentHash, PlaceId};
use atlas_graph_types::text::{BibleLocus, LocusRange, VerseRef};

use map_types::{
    AtlasPlaceRef, BorderSurvey, Boundary, BoundaryHistory, BoundaryId, BoundarySource,
    ChangeEvent, ChangeKind, EdgeCharacter, GazetteerEntry, GazetteerExport, Interval,
    InterpolationMethod, Orientation, RegionGeom, RegionHistory, RegionId, RegionPart, UnitVec,
    WorldTimeline,
};

pub const STAND_IN_PROVENANCE: &str =
    "authored:traditional-identifications (approximate stand-in pending atlas C3 gazetteer)";
const USSHER_PROVENANCE: &str = "owner-config:ussher-tradition (pending atlas C2 export)";

/// One waypoint of a survey: the name the text uses and its stand-in
/// identification.
struct Waypoint {
    name: &'static str,
    lat: f64,
    lon: f64,
}

/// The promised land's borders, specified by God to Moses — NUM
/// 34:1-12, walked in text order: south side west along Edom, up the
/// Great Sea, the north border to Hazar-enan, then down the east side
/// to the Salt Sea. A closed circuit.
const NUM_34_CIRCUIT: &[Waypoint] = &[
    Waypoint { name: "south end of the Salt Sea", lat: 31.05, lon: 35.44 },
    Waypoint { name: "ascent of Akrabbim", lat: 30.95, lon: 35.20 },
    Waypoint { name: "wilderness of Zin", lat: 30.80, lon: 34.80 },
    Waypoint { name: "Kadesh-barnea", lat: 30.69, lon: 34.49 },
    Waypoint { name: "Hazar-addar", lat: 30.75, lon: 34.30 },
    Waypoint { name: "Azmon", lat: 30.85, lon: 34.20 },
    Waypoint { name: "brook of Egypt at the Great Sea", lat: 31.16, lon: 33.80 },
    Waypoint { name: "Great Sea off Joppa", lat: 32.05, lon: 34.70 },
    Waypoint { name: "Great Sea off Tyre", lat: 33.27, lon: 35.18 },
    Waypoint { name: "mount Hor (northern)", lat: 34.30, lon: 35.90 },
    Waypoint { name: "entrance of Hamath", lat: 34.42, lon: 36.37 },
    Waypoint { name: "Zedad", lat: 34.31, lon: 36.60 },
    Waypoint { name: "Ziphron", lat: 34.35, lon: 36.85 },
    Waypoint { name: "Hazar-enan", lat: 34.23, lon: 37.24 },
    Waypoint { name: "Shepham", lat: 33.80, lon: 36.40 },
    Waypoint { name: "Riblah east of Ain", lat: 33.40, lon: 35.95 },
    Waypoint { name: "east slope of the sea of Chinnereth", lat: 32.83, lon: 35.65 },
    Waypoint { name: "the Jordan at Bethabara", lat: 32.00, lon: 35.55 },
    Waypoint { name: "north end of the Salt Sea", lat: 31.76, lon: 35.55 },
];

/// JOS 15:1-12 — Judah's allotment, drawn lot by lot and border by
/// border: north from the Jordan's mouth past En-rogel and the valley
/// of Hinnom to the sea at Jabneel, the Great Sea west, the NUM 34
/// south line, the Salt Sea east. Waypoints in circuit order.
const JOS_15_CIRCUIT: &[Waypoint] = &[
    Waypoint { name: "bay of the Salt Sea at the Jordan's mouth", lat: 31.76, lon: 35.55 },
    Waypoint { name: "Beth-hoglah", lat: 31.80, lon: 35.50 },
    Waypoint { name: "stone of Bohan by the valley of Achor", lat: 31.82, lon: 35.40 },
    Waypoint { name: "ascent of Adummim", lat: 31.82, lon: 35.36 },
    Waypoint { name: "En-shemesh", lat: 31.78, lon: 35.26 },
    Waypoint { name: "En-rogel", lat: 31.77, lon: 35.24 },
    Waypoint { name: "valley of Hinnom south of Jebus", lat: 31.77, lon: 35.22 },
    Waypoint { name: "mountain west of Hinnom", lat: 31.78, lon: 35.19 },
    Waypoint { name: "waters of Nephtoah", lat: 31.80, lon: 35.13 },
    Waypoint { name: "Kiriath-jearim (Baalah)", lat: 31.81, lon: 35.09 },
    Waypoint { name: "mount Seir toward Chesalon", lat: 31.79, lon: 35.05 },
    Waypoint { name: "Beth-shemesh", lat: 31.75, lon: 34.98 },
    Waypoint { name: "Timnah", lat: 31.78, lon: 34.91 },
    Waypoint { name: "north side of Ekron", lat: 31.78, lon: 34.85 },
    Waypoint { name: "Jabneel toward the sea", lat: 31.87, lon: 34.73 },
    Waypoint { name: "the Great Sea at Jabneel", lat: 31.88, lon: 34.68 },
    Waypoint { name: "Great Sea coast off Gaza", lat: 31.50, lon: 34.42 },
    Waypoint { name: "brook of Egypt at the Great Sea", lat: 31.16, lon: 33.80 },
    Waypoint { name: "Azmon", lat: 30.85, lon: 34.20 },
    Waypoint { name: "Hazar-addar", lat: 30.75, lon: 34.30 },
    Waypoint { name: "Kadesh-barnea", lat: 30.69, lon: 34.49 },
    Waypoint { name: "wilderness of Zin", lat: 30.80, lon: 34.80 },
    Waypoint { name: "ascent of Akrabbim", lat: 30.95, lon: 35.20 },
    Waypoint { name: "south end of the Salt Sea", lat: 31.05, lon: 35.44 },
    Waypoint { name: "west shore of the Salt Sea", lat: 31.40, lon: 35.42 },
];

fn place_id(name: &str) -> PlaceId {
    PlaceId::new(format!("standin:{}", name.replace(' ', "-")))
}

fn hash_id(tag: &str) -> ContentHash {
    let mut h = DefaultHasher::new();
    tag.hash(&mut h);
    ContentHash(h.finish())
}

fn tp(year: i32) -> TimePoint {
    TimePoint::year_only(Year::new(year).expect("no year zero in survey data"))
}

/// One Scripture survey, ready to build: the verses, their traditional
/// date, and the circuit the text walks.
struct SurveySpec {
    tag: &'static str,
    label: &'static str,
    note: &'static str,
    book: u8,
    chapter: u16,
    verse_from: u16,
    verse_to: u16,
    /// Traditional (Ussher) year the survey takes effect.
    year: i32,
    circuit: &'static [Waypoint],
}

const SURVEYS: &[SurveySpec] = &[
    SurveySpec {
        tag: "NUM34",
        label: "the land promised (NUM 34)",
        note: "The border circuit God specified to Moses, NUM 34:1-12; waypoint \
               coordinates are approximate traditional identifications (stand-in, \
               see provenance), several northern and eastern ones uncertain.",
        book: 4,
        chapter: 34,
        verse_from: 1,
        verse_to: 12,
        year: -1452,
        circuit: NUM_34_CIRCUIT,
    },
    SurveySpec {
        tag: "JOS15",
        label: "Judah's allotment (JOS 15)",
        note: "Judah's lot as drawn at the division of the land, JOS 15:1-12; \
               waypoint coordinates are approximate traditional identifications \
               (stand-in, see provenance).",
        book: 6,
        chapter: 15,
        verse_from: 1,
        verse_to: 12,
        year: -1444,
        circuit: JOS_15_CIRCUIT,
    },
];

fn verses_of(s: &SurveySpec) -> LocusRange<atlas_graph_types::text::BibleTag> {
    let v = |verse| BibleLocus::whole(VerseRef { book: s.book, chapter: s.chapter, verse });
    LocusRange::new(v(s.verse_from), v(s.verse_to)).expect("survey verses are ordered")
}

/// The stand-in gazetteer for every survey waypoint — the shape of the
/// atlas C3 export, filled with traditional identifications until the
/// real one arrives. Law 12c validates survey waypoints against this.
pub fn stand_in_gazetteer() -> GazetteerExport {
    let mut places = BTreeMap::new();
    for s in SURVEYS {
        for w in s.circuit {
            places.insert(
                place_id(w.name),
                GazetteerEntry {
                    canonical_name: w.name.to_string(),
                    position: UnitVec::from_lat_lon_deg(w.lat, w.lon),
                },
            );
        }
    }
    GazetteerExport { atlas_root: ContentHash(0), places }
}

/// Add one survey to a timeline: the Scripture-surveyed boundary (a
/// closed circuit), the region it bounds, valid from the survey's
/// traditional date to the open edge of knowledge, and a narrated Rise
/// grounded in the verses.
fn add_survey(tl: &mut WorldTimeline, s: &SurveySpec) {
    let justification = Justification {
        text: Some(s.note.to_string()),
        grounds: [Ground::Scripture(verses_of(s))].into(),
    };

    // The circuit closes: repeat the first point, our closed-arc form.
    let mut pts: Vec<UnitVec> =
        s.circuit.iter().map(|w| UnitVec::from_lat_lon_deg(w.lat, w.lon)).collect();
    pts.push(pts[0]);

    let survey = BorderSurvey {
        verses: verses_of(s),
        waypoints: s.circuit.iter().map(|w| AtlasPlaceRef(place_id(w.name))).collect(),
        // Geodesic between waypoints — the disclosed (and only)
        // authored geometry. Coast-following is future authoring work.
        interpolation: InterpolationMethod::Geodesic,
        provenance: STAND_IN_PROVENANCE.to_string(),
    };
    let boundary = Boundary {
        pts,
        character: EdgeCharacter::Line,
        source: BoundarySource::Survey(survey),
        justification: justification.clone(),
        provenance: STAND_IN_PROVENANCE.to_string(),
    };

    let boundary_id = BoundaryId(hash_id(&format!("scripture-survey:{}", s.tag)));
    let region_id = RegionId(hash_id(&format!("scripture-region:{}", s.tag)));
    let valid = Interval::open_from(tp(s.year));

    tl.boundaries.insert(boundary_id, BoundaryHistory { versions: vec![(valid, boundary)] });
    tl.regions.insert(
        region_id,
        RegionHistory {
            label_history: vec![(valid, s.label.to_string())],
            geom_history: vec![(
                valid,
                RegionGeom {
                    parts: vec![RegionPart {
                        cycle: vec![(boundary_id, Orientation::Forward)],
                        holes: vec![],
                    }],
                },
            )],
        },
    );
    tl.events.push(ChangeEvent {
        at: tp(s.year),
        kind: ChangeKind::Rise { region: region_id },
        // driver stays None until the atlas C2 export names this event;
        // law 12a will then hold the date to the atlas's placement.
        driver: None,
        justification,
        provenance: USSHER_PROVENANCE.to_string(),
    });
}

/// Every ingested Scripture survey as one timeline.
pub fn scripture_timeline() -> WorldTimeline {
    let mut tl = WorldTimeline::default();
    for s in SURVEYS {
        add_survey(&mut tl, s);
    }
    tl.events.sort_by_key(|e| e.at);
    tl
}

/// The NUM 34 survey alone (the founding fixture; tests lean on it).
pub fn promised_land_timeline() -> WorldTimeline {
    let mut tl = WorldTimeline::default();
    add_survey(&mut tl, &SURVEYS[0]);
    tl
}

/// Merge two timelines from different sources into one world. Ids are
/// content-derived, so collisions mean the same fact arrived twice —
/// fail loud rather than silently prefer either.
#[derive(Clone, Debug, PartialEq)]
pub enum MergeError {
    DuplicateBoundary(BoundaryId),
    DuplicateRegion(RegionId),
}

pub fn merge_timelines(
    mut base: WorldTimeline,
    other: WorldTimeline,
) -> Result<WorldTimeline, MergeError> {
    for (id, hist) in other.boundaries {
        if base.boundaries.insert(id, hist).is_some() {
            return Err(MergeError::DuplicateBoundary(id));
        }
    }
    for (id, hist) in other.regions {
        if base.regions.insert(id, hist).is_some() {
            return Err(MergeError::DuplicateRegion(id));
        }
    }
    base.events.extend(other.events);
    base.events.sort_by_key(|e| e.at);
    Ok(base)
}
