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

fn place_id(name: &str) -> PlaceId {
    PlaceId::new(format!("standin:{}", name.replace(' ', "-")))
}

fn hash_id(tag: &str) -> ContentHash {
    let mut h = DefaultHasher::new();
    tag.hash(&mut h);
    ContentHash(h.finish())
}

fn num34_range() -> LocusRange<atlas_graph_types::text::BibleTag> {
    let v = |verse| BibleLocus::whole(VerseRef { book: 4, chapter: 34, verse });
    LocusRange::new(v(1), v(12)).expect("NUM 34:1-12 is ordered")
}

fn tp(year: i32) -> TimePoint {
    TimePoint::year_only(Year::new(year).expect("no year zero in survey data"))
}

/// The stand-in gazetteer for the survey waypoints — the shape of the
/// atlas C3 export, filled with traditional identifications until the
/// real one arrives. Law 12c validates survey waypoints against this.
pub fn stand_in_gazetteer() -> GazetteerExport {
    let mut places = BTreeMap::new();
    for w in NUM_34_CIRCUIT {
        places.insert(
            place_id(w.name),
            GazetteerEntry {
                canonical_name: w.name.to_string(),
                position: UnitVec::from_lat_lon_deg(w.lat, w.lon),
            },
        );
    }
    GazetteerExport { atlas_root: ContentHash(0), places }
}

/// Build the survey timeline: one Scripture-surveyed boundary (the NUM
/// 34 circuit) and the region it bounds, valid from the survey's
/// traditional date (Ussher: the plains of Moab, 1452 BC) to the open
/// edge of knowledge, with a narrated Rise grounded in the verses.
pub fn promised_land_timeline() -> WorldTimeline {
    let justification = Justification {
        text: Some(
            "The border circuit God specified to Moses, NUM 34:1-12; waypoint \
             coordinates are approximate traditional identifications (stand-in, \
             see provenance), several northern and eastern ones uncertain."
                .to_string(),
        ),
        grounds: [Ground::Scripture(num34_range())].into(),
    };

    // The circuit closes: repeat the first point, our closed-arc form.
    let mut pts: Vec<UnitVec> =
        NUM_34_CIRCUIT.iter().map(|w| UnitVec::from_lat_lon_deg(w.lat, w.lon)).collect();
    pts.push(pts[0]);

    let survey = BorderSurvey {
        verses: num34_range(),
        waypoints: NUM_34_CIRCUIT.iter().map(|w| AtlasPlaceRef(place_id(w.name))).collect(),
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

    let boundary_id = BoundaryId(hash_id("scripture-survey:NUM34"));
    let region_id = RegionId(hash_id("scripture-region:promised-land"));
    let valid = Interval::open_from(tp(-1452));

    let mut boundaries = BTreeMap::new();
    boundaries.insert(boundary_id, BoundaryHistory { versions: vec![(valid, boundary)] });

    let mut regions = BTreeMap::new();
    regions.insert(
        region_id,
        RegionHistory {
            label_history: vec![(valid, "the land promised (NUM 34)".to_string())],
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

    let events = vec![ChangeEvent {
        at: tp(-1452),
        kind: ChangeKind::Rise { region: region_id },
        // driver stays None until the atlas C2 export names this event;
        // law 12a will then hold the date to the atlas's placement.
        driver: None,
        justification,
        provenance: USSHER_PROVENANCE.to_string(),
    }];

    WorldTimeline { anchor: None, boundaries, regions, events, atlas_pin: None }
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
