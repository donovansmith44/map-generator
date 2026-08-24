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

// The remaining allotments, Joshua 13-19. Where the text WALKS a
// border, the circuit follows it and renders as an attested Line;
// where the text LISTS CITIES, the circuit is a disclosed hull through
// the named places and renders as Unknown — the cities are Scripture's,
// the line between them is not. Levi has no territory (JOS 13:33).

const JOS_18_BENJAMIN: &[Waypoint] = &[
    Waypoint { name: "Jordan over against Jericho", lat: 31.85, lon: 35.53 },
    Waypoint { name: "Jericho north side", lat: 31.90, lon: 35.44 },
    Waypoint { name: "Bethel", lat: 31.93, lon: 35.22 },
    Waypoint { name: "lower Beth-horon", lat: 31.87, lon: 35.07 },
    Waypoint { name: "Kiriath-jearim (Baalah)", lat: 31.81, lon: 35.09 },
    Waypoint { name: "waters of Nephtoah", lat: 31.80, lon: 35.13 },
    Waypoint { name: "valley of Hinnom south of Jebus", lat: 31.77, lon: 35.22 },
    Waypoint { name: "En-rogel", lat: 31.77, lon: 35.24 },
    Waypoint { name: "En-shemesh", lat: 31.78, lon: 35.26 },
    Waypoint { name: "ascent of Adummim", lat: 31.82, lon: 35.36 },
    Waypoint { name: "stone of Bohan by the valley of Achor", lat: 31.82, lon: 35.40 },
    Waypoint { name: "Beth-hoglah", lat: 31.80, lon: 35.50 },
];

const JOS_16_EPHRAIM: &[Waypoint] = &[
    Waypoint { name: "the sea at the Kanah brook's mouth", lat: 32.10, lon: 34.78 },
    Waypoint { name: "Gezer", lat: 31.86, lon: 34.92 },
    Waypoint { name: "lower Beth-horon", lat: 31.87, lon: 35.07 },
    Waypoint { name: "Bethel (Luz)", lat: 31.93, lon: 35.22 },
    Waypoint { name: "Naarah toward Jericho", lat: 31.90, lon: 35.44 },
    Waypoint { name: "the Jordan by Jericho", lat: 31.93, lon: 35.52 },
    Waypoint { name: "Taanath-shiloh", lat: 32.11, lon: 35.38 },
    Waypoint { name: "Janoah", lat: 32.15, lon: 35.30 },
    Waypoint { name: "Michmethath before Shechem", lat: 32.17, lon: 35.20 },
    Waypoint { name: "brook of Kanah", lat: 32.16, lon: 34.98 },
];

const JOS_17_MANASSEH_WEST: &[Waypoint] = &[
    Waypoint { name: "the sea north of Kanah", lat: 32.28, lon: 34.87 },
    Waypoint { name: "brook of Kanah", lat: 32.16, lon: 34.98 },
    Waypoint { name: "Michmethath before Shechem", lat: 32.17, lon: 35.20 },
    Waypoint { name: "En-tappuah", lat: 32.12, lon: 35.27 },
    Waypoint { name: "the Jordan at Asher-ward descent", lat: 32.20, lon: 35.55 },
    Waypoint { name: "Beth-shean", lat: 32.50, lon: 35.50 },
    Waypoint { name: "Jezreel valley edge", lat: 32.55, lon: 35.33 },
    Waypoint { name: "Megiddo", lat: 32.58, lon: 35.18 },
    Waypoint { name: "Dor and her towns", lat: 32.62, lon: 34.92 },
];

const JOS_19_SIMEON: &[Waypoint] = &[
    Waypoint { name: "Sharuhen", lat: 31.28, lon: 34.48 },
    Waypoint { name: "Beersheba", lat: 31.24, lon: 34.79 },
    Waypoint { name: "Moladah", lat: 31.22, lon: 34.90 },
    Waypoint { name: "Hormah", lat: 31.30, lon: 35.03 },
    Waypoint { name: "En-rimmon", lat: 31.38, lon: 34.87 },
    Waypoint { name: "Ziklag", lat: 31.38, lon: 34.68 },
];

const JOS_19_ZEBULUN: &[Waypoint] = &[
    Waypoint { name: "Sarid", lat: 32.63, lon: 35.23 },
    Waypoint { name: "Chisloth-tabor", lat: 32.68, lon: 35.39 },
    Waypoint { name: "Gath-hepher", lat: 32.74, lon: 35.32 },
    Waypoint { name: "Rimmon", lat: 32.78, lon: 35.30 },
    Waypoint { name: "Hannathon", lat: 32.78, lon: 35.24 },
    Waypoint { name: "valley of Jiphthah-el", lat: 32.76, lon: 35.13 },
    Waypoint { name: "Jokneam westward", lat: 32.66, lon: 35.11 },
];

const JOS_19_ISSACHAR: &[Waypoint] = &[
    Waypoint { name: "Jezreel", lat: 32.56, lon: 35.33 },
    Waypoint { name: "Shunem", lat: 32.60, lon: 35.33 },
    Waypoint { name: "Chisloth-tabor southside", lat: 32.66, lon: 35.39 },
    Waypoint { name: "Tabor toward the Jordan", lat: 32.63, lon: 35.52 },
    Waypoint { name: "the Jordan above Beth-shean", lat: 32.52, lon: 35.55 },
    Waypoint { name: "En-gannim", lat: 32.46, lon: 35.30 },
];

const JOS_19_ASHER: &[Waypoint] = &[
    Waypoint { name: "Carmel westward", lat: 32.75, lon: 35.00 },
    Waypoint { name: "Achzib on the coast", lat: 33.05, lon: 35.10 },
    Waypoint { name: "great Zidon-ward at Tyre", lat: 33.27, lon: 35.19 },
    Waypoint { name: "Kanah of the north", lat: 33.21, lon: 35.30 },
    Waypoint { name: "Cabul", lat: 32.87, lon: 35.21 },
    Waypoint { name: "Beth-emek", lat: 32.92, lon: 35.15 },
    Waypoint { name: "valley of Jiphthah-el northside", lat: 32.80, lon: 35.14 },
];

const JOS_19_NAPHTALI: &[Waypoint] = &[
    Waypoint { name: "Heleph at the oak of Zaanannim", lat: 32.70, lon: 35.45 },
    Waypoint { name: "sea of Chinnereth west shore", lat: 32.85, lon: 35.52 },
    Waypoint { name: "Hazor", lat: 33.02, lon: 35.57 },
    Waypoint { name: "Kedesh", lat: 33.11, lon: 35.53 },
    Waypoint { name: "Beth-shemesh of the north", lat: 33.20, lon: 35.50 },
    Waypoint { name: "upper Jordan", lat: 33.22, lon: 35.62 },
    Waypoint { name: "Jordan at Chinnereth's outflow", lat: 32.72, lon: 35.57 },
];

const JOS_19_DAN: &[Waypoint] = &[
    Waypoint { name: "Zorah", lat: 31.77, lon: 34.98 },
    Waypoint { name: "Aijalon", lat: 31.84, lon: 35.02 },
    Waypoint { name: "Bene-berak", lat: 32.08, lon: 34.83 },
    Waypoint { name: "Me-jarkon before Japho", lat: 32.05, lon: 34.75 },
    Waypoint { name: "Ekron border", lat: 31.78, lon: 34.85 },
];

const JOS_13_REUBEN: &[Waypoint] = &[
    Waypoint { name: "Salt Sea shore at the Arnon's mouth", lat: 31.47, lon: 35.56 },
    Waypoint { name: "Aroer on the Arnon's brink", lat: 31.47, lon: 35.80 },
    Waypoint { name: "the wilderness edge beyond Dibon", lat: 31.60, lon: 36.10 },
    Waypoint { name: "Heshbon", lat: 31.80, lon: 35.81 },
    Waypoint { name: "Beth-jeshimoth", lat: 31.78, lon: 35.60 },
];

const JOS_13_GAD: &[Waypoint] = &[
    Waypoint { name: "Beth-jeshimoth northward", lat: 31.80, lon: 35.60 },
    Waypoint { name: "Heshbon", lat: 31.80, lon: 35.81 },
    Waypoint { name: "toward Rabbah of Ammon", lat: 31.95, lon: 35.93 },
    Waypoint { name: "the Jabbok", lat: 32.18, lon: 35.75 },
    Waypoint { name: "Jordan valley at the Jabbok", lat: 32.18, lon: 35.58 },
    Waypoint { name: "down the Jordan", lat: 31.85, lon: 35.55 },
];

const JOS_13_MANASSEH_EAST: &[Waypoint] = &[
    Waypoint { name: "the Jabbok at Mahanaim", lat: 32.18, lon: 35.75 },
    Waypoint { name: "Jordan below Chinnereth", lat: 32.20, lon: 35.60 },
    Waypoint { name: "sea of Chinnereth east shore", lat: 32.75, lon: 35.65 },
    Waypoint { name: "Golan in Bashan", lat: 33.00, lon: 35.85 },
    Waypoint { name: "Salecah eastward", lat: 32.85, lon: 36.60 },
    Waypoint { name: "Edrei", lat: 32.60, lon: 36.10 },
];

// ------------------------------------------------- the kingdom arc
//
// Eras: regions whose borders CHANGE at Scripture-attested moments.
// Each phase's circuit is an authored hull through named places
// (rendered Unknown — extents, not walked borders); the DATES and the
// fact-of-change are the text's, under the traditional (Ussher)
// chronology, disclosed as approximate where the text gives a reign,
// not a year.

const UNITED_CORE: &[Waypoint] = &[
    Waypoint { name: "Dan", lat: 33.25, lon: 35.65 },
    Waypoint { name: "coast at Acco", lat: 32.92, lon: 35.07 },
    Waypoint { name: "coast before Joppa", lat: 32.05, lon: 34.75 },
    Waypoint { name: "Gaza border", lat: 31.50, lon: 34.45 },
    Waypoint { name: "Beersheba", lat: 31.24, lon: 34.79 },
    Waypoint { name: "south end of the Salt Sea", lat: 31.05, lon: 35.44 },
    Waypoint { name: "north end of the Salt Sea", lat: 31.76, lon: 35.55 },
    Waypoint { name: "Gilead", lat: 32.50, lon: 35.90 },
    Waypoint { name: "Bashan toward Dan", lat: 32.90, lon: 36.00 },
];

const UNITED_SOLOMONIC: &[Waypoint] = &[
    Waypoint { name: "Tiphsah on the Euphrates", lat: 35.86, lon: 38.55 },
    Waypoint { name: "toward Hamath the great", lat: 35.10, lon: 36.75 },
    Waypoint { name: "entrance of Hamath", lat: 34.42, lon: 36.37 },
    Waypoint { name: "great Zidon", lat: 33.56, lon: 35.37 },
    Waypoint { name: "coast before Joppa", lat: 32.05, lon: 34.75 },
    Waypoint { name: "Gaza border", lat: 31.50, lon: 34.45 },
    Waypoint { name: "brook of Egypt at the Great Sea", lat: 31.16, lon: 33.80 },
    Waypoint { name: "Kadesh-barnea", lat: 30.69, lon: 34.49 },
    Waypoint { name: "Ezion-geber on the Red sea", lat: 29.55, lon: 34.95 },
    Waypoint { name: "Edom eastward", lat: 30.40, lon: 35.80 },
    Waypoint { name: "desert east of Ammon", lat: 31.90, lon: 36.50 },
    Waypoint { name: "Damascus", lat: 33.51, lon: 36.31 },
    Waypoint { name: "desert toward Tadmor", lat: 34.80, lon: 37.80 },
];

const JUDAH_KINGDOM: &[Waypoint] = &[
    Waypoint { name: "Mizpah of Benjamin", lat: 31.90, lon: 35.20 },
    Waypoint { name: "Bethel border", lat: 31.93, lon: 35.24 },
    Waypoint { name: "Jericho southward", lat: 31.85, lon: 35.46 },
    Waypoint { name: "north end of the Salt Sea", lat: 31.76, lon: 35.55 },
    Waypoint { name: "south end of the Salt Sea", lat: 31.05, lon: 35.44 },
    Waypoint { name: "Beersheba", lat: 31.24, lon: 34.79 },
    Waypoint { name: "toward Gerar", lat: 31.38, lon: 34.62 },
    Waypoint { name: "the shephelah edge", lat: 31.70, lon: 34.88 },
    Waypoint { name: "Aijalon", lat: 31.84, lon: 35.02 },
];

const ISRAEL_DIVIDED: &[Waypoint] = &[
    Waypoint { name: "Bethel northward", lat: 31.95, lon: 35.22 },
    Waypoint { name: "Aijalon northward", lat: 31.87, lon: 35.00 },
    Waypoint { name: "coast before Joppa", lat: 32.08, lon: 34.78 },
    Waypoint { name: "coast at Acco", lat: 32.92, lon: 35.07 },
    Waypoint { name: "Dan", lat: 33.25, lon: 35.65 },
    Waypoint { name: "Bashan eastward", lat: 32.90, lon: 36.10 },
    Waypoint { name: "Gilead", lat: 32.30, lon: 35.90 },
    Waypoint { name: "Jordan by Jericho", lat: 31.87, lon: 35.50 },
];

const ISRAEL_RESTORED: &[Waypoint] = &[
    Waypoint { name: "Bethel northward", lat: 31.95, lon: 35.22 },
    Waypoint { name: "Aijalon northward", lat: 31.87, lon: 35.00 },
    Waypoint { name: "coast before Joppa", lat: 32.08, lon: 34.78 },
    Waypoint { name: "Great Sea off Tyre", lat: 33.27, lon: 35.18 },
    Waypoint { name: "entrance of Hamath", lat: 34.42, lon: 36.37 },
    Waypoint { name: "Damascus recovered", lat: 33.60, lon: 36.40 },
    Waypoint { name: "Bashan eastward", lat: 32.90, lon: 36.20 },
    Waypoint { name: "sea of the plain", lat: 31.80, lon: 35.60 },
    Waypoint { name: "Jordan by Jericho", lat: 31.87, lon: 35.50 },
];

const YEHUD: &[Waypoint] = &[
    Waypoint { name: "Bethel", lat: 31.93, lon: 35.22 },
    Waypoint { name: "Jericho", lat: 31.87, lon: 35.44 },
    Waypoint { name: "north end of the Salt Sea", lat: 31.76, lon: 35.55 },
    Waypoint { name: "En-gedi", lat: 31.45, lon: 35.38 },
    Waypoint { name: "Beth-zur", lat: 31.60, lon: 35.10 },
    Waypoint { name: "Keilah westward", lat: 31.61, lon: 34.97 },
    Waypoint { name: "Emmaus", lat: 31.84, lon: 34.99 },
];

struct PhaseSpec {
    /// Traditional (Ussher) year the phase begins.
    year: i32,
    book: u8,
    chapter: u16,
    verse_from: u16,
    verse_to: u16,
    note: &'static str,
    circuit: &'static [Waypoint],
}

struct EraSpec {
    tag: &'static str,
    label: &'static str,
    phases: &'static [PhaseSpec],
    /// The end, when Scripture narrates one: (year, verses, note).
    fall: Option<(i32, u8, u16, u16, u16, &'static str)>,
}

const ERA_NOTE: &str = "An extent authored from the text's named places (rendered \
    Unknown — the places are Scripture's, the hull is not); coordinates are \
    approximate traditional identifications (stand-in, see provenance); dates \
    follow the traditional (Ussher) chronology.";

const KINGDOMS: &[EraSpec] = &[
    EraSpec {
        tag: "ISRAEL-UNITED",
        label: "the kingdom of Israel, united",
        phases: &[
            PhaseSpec {
                year: -1095, // Saul made king at Gilgal (Ussher)
                book: 9, chapter: 11, verse_from: 14, verse_to: 15,
                note: ERA_NOTE,
                circuit: UNITED_CORE,
            },
            PhaseSpec {
                year: -1015, // Solomon reigns over all kingdoms (Ussher)
                book: 11, chapter: 4, verse_from: 21, verse_to: 25,
                note: "\"From Tiphsah even to Azzah\", \"from Dan even to Beersheba\" — \
                       the dominion realized. An extent authored from the text's named \
                       places; approximate stand-in coordinates; Ussher dates.",
                circuit: UNITED_SOLOMONIC,
            },
        ],
        fall: Some((-975, 11, 12, 16, 20, "The kingdom rent from Rehoboam at Shechem.")),
    },
    EraSpec {
        tag: "JUDAH-KINGDOM",
        label: "the kingdom of Judah",
        phases: &[PhaseSpec {
            year: -975,
            book: 11, chapter: 12, verse_from: 20, verse_to: 24,
            note: ERA_NOTE,
            circuit: JUDAH_KINGDOM,
        }],
        fall: Some((-588, 12, 25, 8, 11, "Jerusalem burned, Judah carried away (Ussher year).")),
    },
    EraSpec {
        tag: "ISRAEL-NORTH",
        label: "the kingdom of Israel",
        phases: &[
            PhaseSpec {
                year: -975,
                book: 11, chapter: 12, verse_from: 16, verse_to: 20,
                note: ERA_NOTE,
                circuit: ISRAEL_DIVIDED,
            },
            PhaseSpec {
                year: -810, // within Jeroboam II's reign (Ussher 825-784), approximate
                book: 12, chapter: 14, verse_from: 25, verse_to: 27,
                note: "\"He restored the coast of Israel from the entering of Hamath \
                       unto the sea of the plain\" — Scripture's own attested border \
                       CHANGE. Year approximate within Jeroboam II's reign (Ussher); \
                       extent authored from the text's named places, stand-in \
                       coordinates.",
                circuit: ISRAEL_RESTORED,
            },
        ],
        fall: Some((-721, 12, 17, 5, 6, "Samaria taken, Israel carried into Assyria (Ussher year).")),
    },
    EraSpec {
        tag: "YEHUD",
        label: "Judah returned (Yehud)",
        phases: &[PhaseSpec {
            year: -536, // the decree of Cyrus (Ussher)
            book: 15, chapter: 1, verse_from: 1, verse_to: 3,
            note: ERA_NOTE,
            circuit: YEHUD,
        }],
        fall: None,
    },
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

/// How Scripture gives the shape — the honesty grade of the circuit.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Grade {
    /// The text walks the border line itself: renders as Line.
    BorderText,
    /// The text names the cities; the hull between them is disclosed
    /// interpolation: renders as Unknown, distinctly, always.
    CityDerived,
}

/// One Scripture survey, ready to build: the verses, their traditional
/// date, and the circuit the text gives.
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
    grade: Grade,
    circuit: &'static [Waypoint],
}

const CITY_NOTE: &str = "The text lists cities, not a border line; this circuit is a \
    disclosed hull through the named places (rendered as Unknown). Coordinates are \
    approximate traditional identifications (stand-in, see provenance).";
const BORDER_NOTE: &str = "The text walks this border; waypoint coordinates are \
    approximate traditional identifications (stand-in, see provenance).";

/// The division of the land, Ussher's traditional year.
const ALLOTMENT_YEAR: i32 = -1444;

const SURVEYS: &[SurveySpec] = &[
    SurveySpec {
        tag: "NUM34",
        label: "the land promised (NUM 34)",
        note: "The border circuit God specified to Moses, NUM 34:1-12; waypoint \
               coordinates are approximate traditional identifications (stand-in, \
               see provenance), several northern and eastern ones uncertain.",
        book: 4, chapter: 34, verse_from: 1, verse_to: 12,
        year: -1452,
        grade: Grade::BorderText,
        circuit: NUM_34_CIRCUIT,
    },
    SurveySpec {
        tag: "JOS15",
        label: "Judah (JOS 15)",
        note: BORDER_NOTE,
        book: 6, chapter: 15, verse_from: 1, verse_to: 12,
        year: ALLOTMENT_YEAR,
        grade: Grade::BorderText,
        circuit: JOS_15_CIRCUIT,
    },
    SurveySpec {
        tag: "JOS18",
        label: "Benjamin (JOS 18)",
        note: BORDER_NOTE,
        book: 6, chapter: 18, verse_from: 11, verse_to: 20,
        year: ALLOTMENT_YEAR,
        grade: Grade::BorderText,
        circuit: JOS_18_BENJAMIN,
    },
    SurveySpec {
        tag: "JOS16",
        label: "Ephraim (JOS 16)",
        note: BORDER_NOTE,
        book: 6, chapter: 16, verse_from: 1, verse_to: 9,
        year: ALLOTMENT_YEAR,
        grade: Grade::BorderText,
        circuit: JOS_16_EPHRAIM,
    },
    SurveySpec {
        tag: "JOS17",
        label: "Manasseh west (JOS 17)",
        note: CITY_NOTE,
        book: 6, chapter: 17, verse_from: 7, verse_to: 11,
        year: ALLOTMENT_YEAR,
        grade: Grade::CityDerived,
        circuit: JOS_17_MANASSEH_WEST,
    },
    SurveySpec {
        tag: "JOS19SIM",
        label: "Simeon, within Judah (JOS 19)",
        note: CITY_NOTE,
        book: 6, chapter: 19, verse_from: 1, verse_to: 9,
        year: ALLOTMENT_YEAR,
        grade: Grade::CityDerived,
        circuit: JOS_19_SIMEON,
    },
    SurveySpec {
        tag: "JOS19ZEB",
        label: "Zebulun (JOS 19)",
        note: BORDER_NOTE,
        book: 6, chapter: 19, verse_from: 10, verse_to: 16,
        year: ALLOTMENT_YEAR,
        grade: Grade::BorderText,
        circuit: JOS_19_ZEBULUN,
    },
    SurveySpec {
        tag: "JOS19ISS",
        label: "Issachar (JOS 19)",
        note: CITY_NOTE,
        book: 6, chapter: 19, verse_from: 17, verse_to: 23,
        year: ALLOTMENT_YEAR,
        grade: Grade::CityDerived,
        circuit: JOS_19_ISSACHAR,
    },
    SurveySpec {
        tag: "JOS19ASH",
        label: "Asher (JOS 19)",
        note: CITY_NOTE,
        book: 6, chapter: 19, verse_from: 24, verse_to: 31,
        year: ALLOTMENT_YEAR,
        grade: Grade::CityDerived,
        circuit: JOS_19_ASHER,
    },
    SurveySpec {
        tag: "JOS19NAP",
        label: "Naphtali (JOS 19)",
        note: CITY_NOTE,
        book: 6, chapter: 19, verse_from: 32, verse_to: 39,
        year: ALLOTMENT_YEAR,
        grade: Grade::CityDerived,
        circuit: JOS_19_NAPHTALI,
    },
    SurveySpec {
        tag: "JOS19DAN",
        label: "Dan (JOS 19)",
        note: CITY_NOTE,
        book: 6, chapter: 19, verse_from: 40, verse_to: 46,
        year: ALLOTMENT_YEAR,
        grade: Grade::CityDerived,
        circuit: JOS_19_DAN,
    },
    SurveySpec {
        tag: "JOS13REU",
        label: "Reuben, beyond Jordan (JOS 13)",
        note: CITY_NOTE,
        book: 6, chapter: 13, verse_from: 15, verse_to: 23,
        year: ALLOTMENT_YEAR,
        grade: Grade::CityDerived,
        circuit: JOS_13_REUBEN,
    },
    SurveySpec {
        tag: "JOS13GAD",
        label: "Gad, beyond Jordan (JOS 13)",
        note: CITY_NOTE,
        book: 6, chapter: 13, verse_from: 24, verse_to: 28,
        year: ALLOTMENT_YEAR,
        grade: Grade::CityDerived,
        circuit: JOS_13_GAD,
    },
    SurveySpec {
        tag: "JOS13MAN",
        label: "Manasseh east, Bashan (JOS 13)",
        note: CITY_NOTE,
        book: 6, chapter: 13, verse_from: 29, verse_to: 31,
        year: ALLOTMENT_YEAR,
        grade: Grade::CityDerived,
        circuit: JOS_13_MANASSEH_EAST,
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
    let mut add = |w: &Waypoint| {
        places.insert(
            place_id(w.name),
            GazetteerEntry {
                canonical_name: w.name.to_string(),
                position: UnitVec::from_lat_lon_deg(w.lat, w.lon),
            },
        );
    };
    for s in SURVEYS {
        for w in s.circuit {
            add(w);
        }
    }
    for e in KINGDOMS {
        for p in e.phases {
            for w in p.circuit {
                add(w);
            }
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
        // Honesty renders: a walked border is a Line; a city-derived
        // hull is Unknown and the styles draw it distinctly (law 6).
        character: match s.grade {
            Grade::BorderText => EdgeCharacter::Line,
            Grade::CityDerived => EdgeCharacter::Unknown,
        },
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
            class: Default::default(),
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

/// Add one kingdom era: a region whose geometry CHANGES at
/// Scripture-attested moments. Phase 1 rises; each later phase is a
/// Shift narrated by its own verses (2KI 14:25 made machinery); the
/// fall, when the text gives one, closes the era.
fn add_era(tl: &mut WorldTimeline, e: &EraSpec) {
    let region_id = RegionId(hash_id(&format!("scripture-era/{}", e.tag)));
    let end = e.fall.map(|(y, ..)| tp(y));
    let mut geom_history = Vec::new();

    for (i, ph) in e.phases.iter().enumerate() {
        let verses = {
            let v = |verse| BibleLocus::whole(VerseRef { book: ph.book, chapter: ph.chapter, verse });
            LocusRange::new(v(ph.verse_from), v(ph.verse_to)).expect("era verses are ordered")
        };
        let justification = Justification {
            text: Some(ph.note.to_string()),
            grounds: [Ground::Scripture(verses.clone())].into(),
        };
        let mut pts: Vec<UnitVec> =
            ph.circuit.iter().map(|w| UnitVec::from_lat_lon_deg(w.lat, w.lon)).collect();
        pts.push(pts[0]);
        let bid = BoundaryId(hash_id(&format!("scripture-era/{}/phase{}", e.tag, i)));
        let until = e.phases.get(i + 1).map(|n| tp(n.year)).or(end);
        let interval = Interval { from: tp(ph.year), to: until };
        tl.boundaries.insert(
            bid,
            BoundaryHistory {
                versions: vec![(
                    interval,
                    Boundary {
                        pts,
                        character: EdgeCharacter::Unknown, // extents, not walked lines
                        source: BoundarySource::Survey(BorderSurvey {
                            verses,
                            waypoints: ph
                                .circuit
                                .iter()
                                .map(|w| AtlasPlaceRef(place_id(w.name)))
                                .collect(),
                            interpolation: InterpolationMethod::Geodesic,
                            provenance: STAND_IN_PROVENANCE.to_string(),
                        }),
                        justification: justification.clone(),
                        provenance: STAND_IN_PROVENANCE.to_string(),
                    },
                )],
            },
        );
        geom_history.push((
            interval,
            RegionGeom {
                parts: vec![RegionPart {
                    cycle: vec![(bid, Orientation::Forward)],
                    holes: vec![],
                }],
            },
        ));
        tl.events.push(ChangeEvent {
            at: tp(ph.year),
            kind: if i == 0 {
                ChangeKind::Rise { region: region_id }
            } else {
                ChangeKind::Shift { boundary: bid }
            },
            driver: None, // atlas C2 export will drive these (law 12a)
            justification,
            provenance: USSHER_PROVENANCE.to_string(),
        });
    }

    if let Some((year, book, chapter, v0, v1, note)) = e.fall {
        let v = |verse| BibleLocus::whole(VerseRef { book, chapter, verse });
        let verses = LocusRange::new(v(v0), v(v1)).expect("fall verses are ordered");
        tl.events.push(ChangeEvent {
            at: tp(year),
            kind: ChangeKind::Fall { region: region_id },
            driver: None,
            justification: Justification {
                text: Some(note.to_string()),
                grounds: [Ground::Scripture(verses)].into(),
            },
            provenance: USSHER_PROVENANCE.to_string(),
        });
    }

    let whole = Interval { from: tp(e.phases[0].year), to: end };
    tl.regions.insert(
        region_id,
        RegionHistory {
            class: Default::default(),
            label_history: vec![(whole, e.label.to_string())],
            geom_history,
        },
    );
}

/// Every ingested Scripture survey and era as one timeline.
pub fn scripture_timeline() -> WorldTimeline {
    let mut tl = WorldTimeline::default();
    for s in SURVEYS {
        add_survey(&mut tl, s);
    }
    for e in KINGDOMS {
        add_era(&mut tl, e);
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
