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

use atlas_graph_types::covenant::{TimePoint, Year};
use atlas_graph_types::covenant::{Ground, Justification};
use atlas_graph_types::covenant::{ContentHash, PlaceId};
use atlas_graph_types::covenant::{BibleLocus, LocusRange, VerseRef};

use crate::exports::AtlasExports;
use map_types::{
    AtlasEventRef, AtlasPlaceRef, BorderSurvey, Boundary, BoundaryHistory, BoundaryId, BoundarySource,
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

// --------------------------------- the table of nations (GEN 10)
//
// The ancestral homelands after the scattering (GEN 11:8-9), placed by
// traditional identifications — broad hulls, rendered Unknown. Rise at
// the division of the earth in Peleg's days (Ussher 2247 BC).

macro_rules! circuit {
    ($name:ident : $(($la:expr, $lo:expr, $n:expr)),+ $(,)?) => {
        const $name: &[Waypoint] = &[
            $(Waypoint { name: $n, lat: $la, lon: $lo }),+
        ];
    };
}

circuit!(N_GOMER: (41.5,33.0,"Gomer northward"),(41.5,37.0,"Gomer eastward"),(40.0,38.5,"Togarmah"),(39.5,35.0,"Gomer southward"),(40.2,32.5,"Ashkenaz westward"));
circuit!(N_MAGOG: (45.0,38.0,"Magog westward"),(45.0,44.0,"Magog northward"),(43.0,46.0,"Magog eastward"),(42.0,41.0,"Magog southward"),(43.3,38.5,"Magog by the sea"));
circuit!(N_MADAI: (37.0,45.0,"Madai northwest"),(37.0,50.0,"Madai northeast"),(34.0,50.5,"Madai southeast"),(33.5,47.0,"Madai southwest"));
circuit!(N_JAVAN: (39.5,20.5,"Javan westward"),(39.5,26.5,"Javan eastward"),(37.0,27.5,"the isles of Elishah"),(36.5,22.5,"Javan southward"),(38.0,20.0,"Javan by the sea"));
circuit!(N_TUBAL: (40.5,34.0,"Tubal westward"),(40.5,37.5,"Tubal eastward"),(38.8,37.8,"Tubal southeast"),(38.5,34.5,"Tubal southwest"));
circuit!(N_MESHECH: (41.5,38.0,"Meshech westward"),(41.5,42.0,"Meshech eastward"),(39.8,42.0,"Meshech southeast"),(39.8,38.5,"Meshech southwest"));
circuit!(N_TIRAS: (42.5,23.0,"Tiras westward"),(42.5,28.0,"Tiras eastward"),(40.8,28.5,"Tiras by the strait"),(40.8,24.0,"Tiras southward"));
circuit!(N_CUSH: (18.0,30.0,"Cush westward"),(18.0,37.0,"Cush eastward"),(12.0,37.5,"Cush southeast"),(11.5,32.0,"Cush southwest"),(15.0,30.0,"Cush by the river"));
circuit!(N_MIZRAIM: (31.2,29.8,"Mizraim by the sea"),(31.2,32.3,"Mizraim eastward"),(29.5,32.5,"Mizraim by the Red sea"),(24.0,33.0,"Pathros southward"),(24.0,31.5,"Pathros westward"),(29.8,29.5,"Mizraim of the west"));
circuit!(N_PHUT: (32.8,20.0,"Phut westward"),(32.8,25.0,"Phut eastward"),(30.0,25.5,"Phut southeast"),(29.5,20.5,"Phut southwest"));
circuit!(N_CANAAN: (33.56,35.37,"Sidon"),(33.20,36.00,"Canaan eastward"),(31.76,35.72,"toward Sodom and Gomorrah"),(31.00,35.40,"unto Lasha-ward"),(31.24,34.79,"toward Gerar"),(31.38,34.62,"Gerar"),(31.50,34.42,"unto Gaza"),(32.90,35.05,"Canaan by the sea"));
circuit!(N_SHINAR: (33.5,43.5,"Shinar northward"),(33.5,46.0,"Shinar eastward"),(30.8,47.5,"Erech southward"),(30.5,44.5,"Shinar southwest"),(32.5,43.0,"Accad westward"));
circuit!(N_ELAM: (33.0,46.0,"Elam northwest"),(33.0,50.0,"Elam northeast"),(30.0,50.5,"Elam southeast"),(29.8,47.0,"Elam southwest"));
circuit!(N_ASSHUR: (37.0,42.0,"Asshur northwest"),(37.0,44.5,"Nineveh-ward"),(34.5,44.5,"Calah southward"),(34.8,42.0,"Asshur southwest"));
circuit!(N_ARAM: (36.5,37.5,"Aram northward"),(36.5,40.5,"Aram eastward"),(34.0,38.5,"Aram southeast"),(33.30,36.20,"Aram of Damascus"),(34.5,36.8,"Aram westward"));
circuit!(N_LUD: (39.3,26.8,"Lud westward"),(39.3,29.5,"Lud eastward"),(38.0,29.7,"Lud southeast"),(37.8,27.0,"Lud southwest"));
circuit!(N_JOKTAN: (20.0,44.0,"from Mesha"),(20.0,50.0,"Joktan eastward"),(15.5,50.5,"toward Sephar"),(14.5,44.5,"Joktan southwest"));

// --------------------------------------- the land in vision (EZK 47-48)

const EZK_47_OUTER: &[Waypoint] = &[
    Waypoint { name: "the great sea toward Hethlon", lat: 34.55, lon: 35.90 },
    Waypoint { name: "entrance of Hamath", lat: 34.42, lon: 36.37 },
    Waypoint { name: "Zedad", lat: 34.31, lon: 36.60 },
    Waypoint { name: "Hazar-enan by Hauran", lat: 34.23, lon: 37.24 },
    Waypoint { name: "Hauran eastward", lat: 32.80, lon: 36.30 },
    Waypoint { name: "east of the sea of Chinnereth", lat: 32.75, lon: 35.65 },
    Waypoint { name: "the east sea's north bay", lat: 31.76, lon: 35.55 },
    Waypoint { name: "Tamar", lat: 31.00, lon: 35.40 },
    Waypoint { name: "waters of strife in Kadesh", lat: 30.69, lon: 34.49 },
    Waypoint { name: "the river toward the great sea", lat: 31.16, lon: 33.80 },
    Waypoint { name: "Great Sea coast off Gaza", lat: 31.50, lon: 34.42 },
    Waypoint { name: "coast before Joppa", lat: 32.05, lon: 34.75 },
    Waypoint { name: "Great Sea off Tyre", lat: 33.27, lon: 35.18 },
];

const EZK_48_OBLATION: &[Waypoint] = &[
    Waypoint { name: "oblation southwest corner", lat: 31.40, lon: 34.90 },
    Waypoint { name: "oblation southeast corner", lat: 31.40, lon: 35.70 },
    Waypoint { name: "oblation northeast corner", lat: 32.10, lon: 35.70 },
    Waypoint { name: "oblation northwest corner", lat: 32.10, lon: 34.90 },
];

// ---------------------------------- the tetrarchies of LUK 3:1 (AD 26)

const NT_JUDAEA: &[Waypoint] = &[
    Waypoint { name: "coast at Caesarea", lat: 32.50, lon: 34.89 },
    Waypoint { name: "Samaria border north", lat: 32.30, lon: 35.55 },
    Waypoint { name: "Jordan at Jericho", lat: 31.85, lon: 35.50 },
    Waypoint { name: "north end of the Salt Sea", lat: 31.76, lon: 35.55 },
    Waypoint { name: "Masada southward", lat: 31.00, lon: 35.35 },
    Waypoint { name: "Idumea south", lat: 31.10, lon: 34.80 },
    Waypoint { name: "Gaza border", lat: 31.50, lon: 34.42 },
    Waypoint { name: "coast at Joppa", lat: 32.05, lon: 34.75 },
];
const NT_GALILEE: &[Waypoint] = &[
    Waypoint { name: "Ptolemais border", lat: 33.00, lon: 35.10 },
    Waypoint { name: "north of Capernaum", lat: 33.05, lon: 35.60 },
    Waypoint { name: "sea of Galilee east shore", lat: 32.75, lon: 35.62 },
    Waypoint { name: "Nazareth southward", lat: 32.62, lon: 35.30 },
    Waypoint { name: "Jezreel edge", lat: 32.60, lon: 35.10 },
];
const NT_PEREA: &[Waypoint] = &[
    Waypoint { name: "Pella northward", lat: 32.40, lon: 35.60 },
    Waypoint { name: "Gerasa border", lat: 32.28, lon: 35.90 },
    Waypoint { name: "Machaerus south", lat: 31.55, lon: 35.65 },
    Waypoint { name: "Jordan by the Salt Sea", lat: 31.76, lon: 35.56 },
    Waypoint { name: "Jordan at Pella", lat: 32.38, lon: 35.56 },
];
const NT_ITUREA: &[Waypoint] = &[
    Waypoint { name: "Caesarea Philippi", lat: 33.25, lon: 35.69 },
    Waypoint { name: "Damascus-ward north", lat: 33.50, lon: 36.30 },
    Waypoint { name: "Trachonitis east", lat: 32.90, lon: 36.70 },
    Waypoint { name: "Gaulanitis south", lat: 32.78, lon: 35.75 },
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

/// Resolve a circuit against the atlas gazetteer: bound waypoints take
/// the ATLAS's coordinates and PlaceIds (C3: one fact, one home);
/// unbound ones keep their disclosed stand-ins. Returns (points,
/// place refs, bound count).
fn resolve_circuit(
    circuit: &[Waypoint],
    atlas: Option<&AtlasExports>,
) -> (Vec<UnitVec>, Vec<AtlasPlaceRef>, usize) {
    let mut pts = Vec::with_capacity(circuit.len());
    let mut refs = Vec::with_capacity(circuit.len());
    let mut bound = 0usize;
    for w in circuit {
        match atlas.and_then(|a| a.resolve_place(w.name)) {
            Some((pid, lat, lon)) => {
                pts.push(UnitVec::from_lat_lon_deg(lat, lon));
                refs.push(AtlasPlaceRef(pid));
                bound += 1;
            }
            None => {
                pts.push(UnitVec::from_lat_lon_deg(w.lat, w.lon));
                refs.push(AtlasPlaceRef(place_id(w.name)));
            }
        }
    }
    (pts, refs, bound)
}

/// The provenance a resolved circuit carries: full-atlas, mixed, or
/// stand-in — disclosed either way.
fn circuit_provenance(atlas: Option<&AtlasExports>, bound: usize, total: usize) -> String {
    match (atlas, bound) {
        (Some(a), b) if b == total => format!("bible-atlas@{:016x}", a.root.0),
        (Some(a), b) if b > 0 => format!(
            "mixed: bible-atlas@{:016x} ({b}/{total} waypoints) + {STAND_IN_PROVENANCE}",
            a.root.0
        ),
        _ => STAND_IN_PROVENANCE.to_string(),
    }
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

const NATIONS_NOTE: &str = "An ancestral homeland of the table of nations, placed by \
    traditional identifications as a broad hull (rendered Unknown); rise at the \
    division of the earth in Peleg's days (GEN 10:25, Ussher 2247 BC).";
const NATIONS_YEAR: i32 = -2247;

/// GEN 10 + the vision + the tetrarchies: more SurveySpec rows, same
/// machinery, same stand-in review flag.
const SURVEYS_MORE: &[SurveySpec] = &[
    // ---- Japheth (GEN 10:2-5) ----
    SurveySpec { tag: "N-GOMER", label: "Gomer (GEN 10)", note: NATIONS_NOTE, book: 1, chapter: 10, verse_from: 2, verse_to: 3, year: NATIONS_YEAR, grade: Grade::CityDerived, circuit: N_GOMER },
    SurveySpec { tag: "N-MAGOG", label: "Magog (GEN 10)", note: NATIONS_NOTE, book: 1, chapter: 10, verse_from: 2, verse_to: 2, year: NATIONS_YEAR, grade: Grade::CityDerived, circuit: N_MAGOG },
    SurveySpec { tag: "N-MADAI", label: "Madai (GEN 10)", note: NATIONS_NOTE, book: 1, chapter: 10, verse_from: 2, verse_to: 2, year: NATIONS_YEAR, grade: Grade::CityDerived, circuit: N_MADAI },
    SurveySpec { tag: "N-JAVAN", label: "Javan (GEN 10)", note: NATIONS_NOTE, book: 1, chapter: 10, verse_from: 2, verse_to: 5, year: NATIONS_YEAR, grade: Grade::CityDerived, circuit: N_JAVAN },
    SurveySpec { tag: "N-TUBAL", label: "Tubal (GEN 10)", note: NATIONS_NOTE, book: 1, chapter: 10, verse_from: 2, verse_to: 2, year: NATIONS_YEAR, grade: Grade::CityDerived, circuit: N_TUBAL },
    SurveySpec { tag: "N-MESHECH", label: "Meshech (GEN 10)", note: NATIONS_NOTE, book: 1, chapter: 10, verse_from: 2, verse_to: 2, year: NATIONS_YEAR, grade: Grade::CityDerived, circuit: N_MESHECH },
    SurveySpec { tag: "N-TIRAS", label: "Tiras (GEN 10)", note: NATIONS_NOTE, book: 1, chapter: 10, verse_from: 2, verse_to: 2, year: NATIONS_YEAR, grade: Grade::CityDerived, circuit: N_TIRAS },
    // ---- Ham (GEN 10:6-20) ----
    SurveySpec { tag: "N-CUSH", label: "Cush (GEN 10)", note: NATIONS_NOTE, book: 1, chapter: 10, verse_from: 6, verse_to: 7, year: NATIONS_YEAR, grade: Grade::CityDerived, circuit: N_CUSH },
    SurveySpec { tag: "N-MIZRAIM", label: "Mizraim (GEN 10)", note: NATIONS_NOTE, book: 1, chapter: 10, verse_from: 6, verse_to: 6, year: NATIONS_YEAR, grade: Grade::CityDerived, circuit: N_MIZRAIM },
    SurveySpec { tag: "N-PHUT", label: "Phut (GEN 10)", note: NATIONS_NOTE, book: 1, chapter: 10, verse_from: 6, verse_to: 6, year: NATIONS_YEAR, grade: Grade::CityDerived, circuit: N_PHUT },
    SurveySpec {
        tag: "N-CANAAN",
        label: "Canaan (GEN 10)",
        note: "\"The border of the Canaanites was from Sidon, as thou comest to Gerar, \
               unto Gaza; as thou goest, unto Sodom\" — the text walks it (GEN 10:19). \
               Stand-in coordinates; Ussher dates.",
        book: 1, chapter: 10, verse_from: 15, verse_to: 19,
        year: NATIONS_YEAR,
        grade: Grade::BorderText,
        circuit: N_CANAAN,
    },
    SurveySpec { tag: "N-SHINAR", label: "the land of Shinar, Nimrod's (GEN 10)", note: NATIONS_NOTE, book: 1, chapter: 10, verse_from: 8, verse_to: 10, year: NATIONS_YEAR, grade: Grade::CityDerived, circuit: N_SHINAR },
    // ---- Shem (GEN 10:21-31) ----
    SurveySpec { tag: "N-ELAM", label: "Elam (GEN 10)", note: NATIONS_NOTE, book: 1, chapter: 10, verse_from: 22, verse_to: 22, year: NATIONS_YEAR, grade: Grade::CityDerived, circuit: N_ELAM },
    SurveySpec { tag: "N-ASSHUR", label: "Asshur (GEN 10)", note: NATIONS_NOTE, book: 1, chapter: 10, verse_from: 11, verse_to: 12, year: NATIONS_YEAR, grade: Grade::CityDerived, circuit: N_ASSHUR },
    SurveySpec { tag: "N-ARAM", label: "Aram (GEN 10)", note: NATIONS_NOTE, book: 1, chapter: 10, verse_from: 22, verse_to: 23, year: NATIONS_YEAR, grade: Grade::CityDerived, circuit: N_ARAM },
    SurveySpec { tag: "N-LUD", label: "Lud (GEN 10)", note: NATIONS_NOTE, book: 1, chapter: 10, verse_from: 22, verse_to: 22, year: NATIONS_YEAR, grade: Grade::CityDerived, circuit: N_LUD },
    SurveySpec { tag: "N-JOKTAN", label: "the sons of Joktan (GEN 10)", note: NATIONS_NOTE, book: 1, chapter: 10, verse_from: 25, verse_to: 30, year: NATIONS_YEAR, grade: Grade::CityDerived, circuit: N_JOKTAN },
    // ---- the land in vision (EZK 47-48, Ussher 574 BC) ----
    SurveySpec {
        tag: "EZK47",
        label: "the land in vision (EZK 47)",
        note: "The border of Ezekiel's vision, walked by the text (EZK 47:13-20) — \
               a VISION, never a realized historical border; stand-in coordinates; \
               dated to the vision's own year (Ussher 574 BC).",
        book: 26, chapter: 47, verse_from: 13, verse_to: 20,
        year: -574,
        grade: Grade::BorderText,
        circuit: EZK_47_OUTER,
    },
    SurveySpec {
        tag: "EZK48",
        label: "the holy oblation in vision (EZK 48)",
        note: "The oblation of the vision, five and twenty thousand reeds square \
               (EZK 48:8-20) — schematic, a VISION; stand-in placement.",
        book: 26, chapter: 48, verse_from: 8, verse_to: 20,
        year: -574,
        grade: Grade::BorderText,
        circuit: EZK_48_OBLATION,
    },
    // ---- the tetrarchies of LUK 3:1 (the 15th year of Tiberius) ----
    SurveySpec { tag: "NT-JUDAEA", label: "Judaea under Pilate (LUK 3)", note: CITY_NOTE, book: 42, chapter: 3, verse_from: 1, verse_to: 1, year: 26, grade: Grade::CityDerived, circuit: NT_JUDAEA },
    SurveySpec { tag: "NT-GALILEE", label: "Galilee of Herod the tetrarch (LUK 3)", note: CITY_NOTE, book: 42, chapter: 3, verse_from: 1, verse_to: 1, year: 26, grade: Grade::CityDerived, circuit: NT_GALILEE },
    SurveySpec { tag: "NT-PEREA", label: "Perea of Herod the tetrarch (LUK 3)", note: CITY_NOTE, book: 42, chapter: 3, verse_from: 1, verse_to: 1, year: 26, grade: Grade::CityDerived, circuit: NT_PEREA },
    SurveySpec { tag: "NT-ITUREA", label: "Iturea and Trachonitis of Philip (LUK 3)", note: CITY_NOTE, book: 42, chapter: 3, verse_from: 1, verse_to: 1, year: 26, grade: Grade::CityDerived, circuit: NT_ITUREA },
];

// ------------------------------------------ journeys (open routes)
//
// Scripture's own itineraries: OPEN polylines through named stations,
// rendered as dashed Unknown ways (the stations are the text's, the
// road between them is not). NUM 33 is the Word's own station list.

circuit!(R_EXODUS:
    (30.80,31.83,"Rameses"),(30.55,32.10,"Succoth"),(30.35,32.25,"Etham"),
    (30.05,32.45,"Pi-hahiroth"),(29.65,32.65,"Marah"),(29.35,32.90,"Elim"),
    (29.15,32.95,"encamp by the Red sea"),(29.00,33.20,"wilderness of Sin"),
    (28.90,33.45,"Dophkah"),(28.70,33.75,"Rephidim"),
    (28.54,33.97,"the wilderness of Sinai"),(28.75,34.20,"Kibroth-hattaavah"),
    (28.95,34.40,"Hazeroth"),(30.10,34.50,"Rithmah toward Kadesh"),
    (30.69,34.49,"Kadesh-barnea"),(29.55,34.95,"Ezion-geber on the Red sea"),
    (30.32,35.07,"mount Hor by Edom's border"),(30.45,35.35,"Zalmonah"),
    (30.65,35.45,"Punon"),(30.85,35.55,"Oboth"),(31.05,35.70,"Ije-abarim"),
    (31.50,35.78,"Dibon-gad"),(31.62,35.75,"Almon-diblathaim"),
    (31.76,35.72,"mountains of Abarim before Nebo"),
    (31.85,35.62,"plains of Moab by Jordan"));

circuit!(R_PAUL1:
    (36.20,36.16,"Antioch of Syria"),(36.12,35.93,"Seleucia"),(35.18,33.90,"Salamis"),
    (34.77,32.42,"Paphos"),(36.96,30.85,"Perga in Pamphylia"),
    (38.31,31.19,"Antioch in Pisidia"),(37.87,32.49,"Iconium"),(37.58,32.45,"Lystra"),
    (37.35,33.25,"Derbe"),(36.88,30.70,"Attalia"));

circuit!(R_PAUL2:
    (36.20,36.16,"Antioch of Syria"),(36.92,34.90,"Tarsus"),(37.35,33.25,"Derbe"),
    (37.58,32.45,"Lystra"),(37.87,32.49,"Iconium"),(39.75,26.15,"Troas"),
    (40.94,24.41,"Neapolis"),(41.01,24.28,"Philippi"),(40.64,22.94,"Thessalonica"),
    (40.52,22.20,"Berea"),(37.98,23.73,"Athens"),(37.91,22.88,"Corinth"),
    (37.94,27.34,"Ephesus"),(32.50,34.89,"Caesarea"),(31.78,35.22,"Jerusalem"));

circuit!(R_PAUL3:
    (36.20,36.16,"Antioch of Syria"),(36.92,34.90,"Tarsus"),(37.87,32.49,"Iconium"),
    (37.94,27.34,"Ephesus"),(39.75,26.15,"Troas"),(41.01,24.28,"Philippi"),
    (40.64,22.94,"Thessalonica"),(37.91,22.88,"Corinth"),(37.53,27.28,"Miletus"),
    (36.26,29.31,"Patara"),(33.27,35.19,"Tyre"),(32.50,34.89,"Caesarea"),
    (31.78,35.22,"Jerusalem"));

circuit!(R_ROME:
    (32.50,34.89,"Caesarea"),(33.56,35.37,"Sidon"),(36.26,29.98,"Myra of Lycia"),
    (34.92,24.73,"the Fair Havens of Crete"),(35.90,14.45,"Melita"),
    (37.06,15.29,"Syracuse"),(38.11,15.65,"Rhegium"),(40.83,14.12,"Puteoli"),
    (41.89,12.49,"Rome"));

struct RouteSpec {
    tag: &'static str,
    note: &'static str,
    book: u8,
    chapter_from: u16,
    verse_from: u16,
    chapter_to: u16,
    verse_to: u16,
    from_year: i32,
    to_year: Option<i32>,
    stations: &'static [Waypoint],
}

const ROUTE_NOTE: &str = "A journey: the stations are the text's, the way between \
    them is interpolation (rendered Unknown, dashed); station coordinates are \
    approximate traditional identifications (stand-in); Ussher dates.";

const ROUTES: &[RouteSpec] = &[
    RouteSpec {
        tag: "R-EXODUS",
        note: "\"These are the journeys of the children of Israel\" — the Word's own \
               itinerary, NUM 33; forty years from Rameses to the plains of Moab. \
               Station identifications approximate (stand-in); Ussher dates.",
        book: 4, chapter_from: 33, verse_from: 5, chapter_to: 33, verse_to: 49,
        from_year: -1491, to_year: Some(-1451),
        stations: R_EXODUS,
    },
    RouteSpec {
        tag: "R-PAUL1", note: ROUTE_NOTE,
        book: 44, chapter_from: 13, verse_from: 1, chapter_to: 14, verse_to: 28,
        from_year: 45, to_year: None,
        stations: R_PAUL1,
    },
    RouteSpec {
        tag: "R-PAUL2", note: ROUTE_NOTE,
        book: 44, chapter_from: 15, verse_from: 36, chapter_to: 18, verse_to: 22,
        from_year: 49, to_year: None,
        stations: R_PAUL2,
    },
    RouteSpec {
        tag: "R-PAUL3", note: ROUTE_NOTE,
        book: 44, chapter_from: 18, verse_from: 23, chapter_to: 21, verse_to: 17,
        from_year: 53, to_year: None,
        stations: R_PAUL3,
    },
    RouteSpec {
        tag: "R-ROME", note: ROUTE_NOTE,
        book: 44, chapter_from: 27, verse_from: 1, chapter_to: 28, verse_to: 16,
        from_year: 60, to_year: None,
        stations: R_ROME,
    },
];

/// Add one journey: an OPEN Survey boundary through the stations —
/// no region, no closure, a way through the land.
fn add_route(tl: &mut WorldTimeline, r: &RouteSpec, atlas: Option<&AtlasExports>) {
    let verses = LocusRange::new(
        BibleLocus::whole(VerseRef { book: r.book, chapter: r.chapter_from, verse: r.verse_from }),
        BibleLocus::whole(VerseRef { book: r.book, chapter: r.chapter_to, verse: r.verse_to }),
    )
    .expect("route verses are ordered");
    let justification = Justification {
        text: Some(r.note.to_string()),
        grounds: [Ground::Scripture(verses.clone())].into(),
    };
    let resolved = atlas.and_then(|a| {
        a.resolve_event(r.book, (r.chapter_from, r.verse_from), (r.chapter_to, r.verse_to))
    });
    let (from_year, to_year) = match &resolved {
        Some((_, fy, ty)) => (*fy, if ty > fy { Some(*ty) } else { r.to_year }),
        None => (r.from_year, r.to_year),
    };
    let (pts, waypoints, bound) = resolve_circuit(r.stations, atlas);
    let provenance = circuit_provenance(atlas, bound, r.stations.len());
    let bid = BoundaryId(hash_id(&format!("scripture-route/{}", r.tag)));
    tl.boundaries.insert(
        bid,
        BoundaryHistory {
            versions: vec![(
                Interval { from: tp(from_year), to: to_year.map(tp) },
                Boundary {
                    pts,
                    character: EdgeCharacter::Unknown,
                    source: BoundarySource::Survey(BorderSurvey {
                        verses,
                        waypoints,
                        interpolation: InterpolationMethod::Geodesic,
                        provenance: provenance.clone(),
                    }),
                    justification,
                    provenance,
                },
            )],
        },
    );
}

fn verses_of(s: &SurveySpec) -> LocusRange<atlas_graph_types::covenant::BibleTag> {
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
                    aliases: Vec::new(),
                    provenance: None,
                    attestations: Vec::new(),
                },
        );
    };
    for s in SURVEYS.iter().chain(SURVEYS_MORE) {
        for w in s.circuit {
            add(w);
        }
    }
    for r in ROUTES {
        for w in r.stations {
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
fn add_survey(tl: &mut WorldTimeline, s: &SurveySpec, atlas: Option<&AtlasExports>) {
    let justification = Justification {
        text: Some(s.note.to_string()),
        grounds: [Ground::Scripture(verses_of(s))].into(),
    };

    // C2/C3 binding: the atlas dates and locates what it can; the
    // rest stays disclosed stand-in.
    let resolved =
        atlas.and_then(|a| a.resolve_event(s.book, (s.chapter, s.verse_from), (s.chapter, s.verse_to)));
    let year = resolved.as_ref().map(|(_, y, _)| *y).unwrap_or(s.year);
    let driver = resolved.as_ref().zip(atlas).map(|((eid, ..), a)| AtlasEventRef {
        event: eid.clone(),
        atlas_root: a.root,
    });
    let (mut pts, waypoints, bound) = resolve_circuit(s.circuit, atlas);
    pts.push(pts[0]); // the circuit closes: our closed-arc form
    let provenance = circuit_provenance(atlas, bound, s.circuit.len());

    let survey = BorderSurvey {
        verses: verses_of(s),
        waypoints,
        // Geodesic between waypoints — the disclosed (and only)
        // authored geometry. Coast-following is future authoring work.
        interpolation: InterpolationMethod::Geodesic,
        provenance: provenance.clone(),
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
        provenance: provenance.clone(),
    };

    let boundary_id = BoundaryId(hash_id(&format!("scripture-survey:{}", s.tag)));
    let region_id = RegionId(hash_id(&format!("scripture-region:{}", s.tag)));
    let valid = Interval::open_from(tp(year));

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
        at: tp(year),
        kind: ChangeKind::Rise { region: region_id },
        // Bound events carry the atlas driver; law 12a holds the date
        // to the atlas's placement byte-for-byte.
        driver,
        justification,
        provenance: match atlas {
            Some(a) if resolved.is_some() => format!("bible-atlas@{:016x}", a.root.0),
            _ => USSHER_PROVENANCE.to_string(),
        },
    });
}

/// Add one kingdom era: a region whose geometry CHANGES at
/// Scripture-attested moments. Phase 1 rises; each later phase is a
/// Shift narrated by its own verses (2KI 14:25 made machinery); the
/// fall, when the text gives one, closes the era.
fn add_era(tl: &mut WorldTimeline, e: &EraSpec, atlas: Option<&AtlasExports>) {
    let region_id = RegionId(hash_id(&format!("scripture-era/{}", e.tag)));

    // Resolve every phase and the fall FIRST; a binding that would
    // break the era's ordering is dropped (year and driver together —
    // law 12a forbids keeping one without the other).
    let mut years: Vec<(i32, Option<AtlasEventRef>)> = e
        .phases
        .iter()
        .map(|ph| {
            match atlas.and_then(|a| {
                a.resolve_event(ph.book, (ph.chapter, ph.verse_from), (ph.chapter, ph.verse_to))
            }) {
                Some((eid, y, _)) => (
                    y,
                    atlas.map(|a| AtlasEventRef { event: eid, atlas_root: a.root }),
                ),
                None => (ph.year, None),
            }
        })
        .collect();
    let mut fall_res: Option<(i32, Option<AtlasEventRef>)> = e.fall.map(|(y, book, ch, v0, v1, _)| {
        match atlas.and_then(|a| a.resolve_event(book, (ch, v0), (ch, v1))) {
            Some((eid, ry, _)) => {
                (ry, atlas.map(|a| AtlasEventRef { event: eid, atlas_root: a.root }))
            }
            None => (y, None),
        }
    });
    for i in 1..years.len() {
        if years[i].0 <= years[i - 1].0 {
            years[i] = (e.phases[i].year, None); // unbind rather than invert
        }
    }
    if let (Some((fy, _)), Some(last)) = (fall_res.as_ref(), years.last()) {
        if *fy <= last.0 {
            if let (Some((orig, ..)), Some(f)) = (e.fall, fall_res.as_mut()) {
                *f = (orig, None) // keep spec year, drop driver
            }
        }
    }

    let end = fall_res.as_ref().map(|(y, _)| tp(*y));
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
        let until = years.get(i + 1).map(|(y, _)| tp(*y)).or(end);
        let interval = Interval { from: tp(years[i].0), to: until };
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
            at: tp(years[i].0),
            kind: if i == 0 {
                ChangeKind::Rise { region: region_id }
            } else {
                ChangeKind::Shift { boundary: bid }
            },
            driver: years[i].1.clone(),
            justification,
            provenance: match (&years[i].1, atlas) {
                (Some(_), Some(a)) => format!("bible-atlas@{:016x}", a.root.0),
                _ => USSHER_PROVENANCE.to_string(),
            },
        });
    }

    if let (Some((_, book, chapter, v0, v1, note)), Some((fy, fdriver))) = (e.fall, fall_res) {
        let v = |verse| BibleLocus::whole(VerseRef { book, chapter, verse });
        let verses = LocusRange::new(v(v0), v(v1)).expect("fall verses are ordered");
        let prov = match (&fdriver, atlas) {
            (Some(_), Some(a)) => format!("bible-atlas@{:016x}", a.root.0),
            _ => USSHER_PROVENANCE.to_string(),
        };
        tl.events.push(ChangeEvent {
            at: tp(fy),
            kind: ChangeKind::Fall { region: region_id },
            driver: fdriver,
            justification: Justification {
                text: Some(note.to_string()),
                grounds: [Ground::Scripture(verses)].into(),
            },
            provenance: prov,
        });
    }

    let whole = Interval { from: tp(years[0].0), to: end };
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
    scripture_timeline_with(None)
}

/// The validation gazetteer for a bound timeline: the atlas's places
/// plus the residual stand-ins (waypoints the atlas could not name) —
/// law 12c then holds over the whole mixed set.
pub fn merged_gazetteer(atlas: &AtlasExports) -> GazetteerExport {
    let mut g = atlas.gazetteer.clone();
    for (id, entry) in stand_in_gazetteer().places {
        g.places.entry(id).or_insert(entry);
    }
    g
}

/// One row of the cross-repo chronology audit: where an independent
/// stand-in year and the atlas's placement disagree.
#[derive(Clone, Debug)]
pub struct BindingRow {
    /// Our spec tag (e.g. "ISRAEL-NORTH/phase1", "JOS15", "R-EXODUS").
    pub ours: String,
    /// The atlas event we bound to: (id, label).
    pub atlas_event: (String, String),
    pub our_year: i32,
    pub their_year: i32,
}

/// The covenant dividend: every consumer with independent stand-ins is
/// a free auditor of the authority data. This reports every binding
/// where the two derivations DISAGREE — input for the atlas's
/// chronology audits, recomputed free at every re-vendor.
pub fn binding_report(atlas: &AtlasExports) -> Vec<BindingRow> {
    let mut rows = Vec::new();
    let mut push = |ours: String, res: Option<(atlas_graph_types::covenant::EventId, i32, i32)>, our_year: i32| {
        if let Some((eid, fy, _)) = res {
            if fy != our_year {
                let label = atlas
                    .events
                    .iter()
                    .find(|e| e.id == eid.0)
                    .map(|e| e.label.clone())
                    .unwrap_or_default();
                rows.push(BindingRow {
                    ours,
                    atlas_event: (eid.0.clone(), label),
                    our_year,
                    their_year: fy,
                });
            }
        }
    };
    for s in SURVEYS.iter().chain(SURVEYS_MORE) {
        push(
            s.tag.to_string(),
            atlas.resolve_event(s.book, (s.chapter, s.verse_from), (s.chapter, s.verse_to)),
            s.year,
        );
    }
    for e in KINGDOMS {
        for (i, ph) in e.phases.iter().enumerate() {
            push(
                format!("{}/phase{}", e.tag, i),
                atlas.resolve_event(ph.book, (ph.chapter, ph.verse_from), (ph.chapter, ph.verse_to)),
                ph.year,
            );
        }
        if let Some((y, book, ch, v0, v1, _)) = e.fall {
            push(format!("{}/fall", e.tag), atlas.resolve_event(book, (ch, v0), (ch, v1)), y);
        }
    }
    for r in ROUTES {
        push(
            r.tag.to_string(),
            atlas.resolve_event(r.book, (r.chapter_from, r.verse_from), (r.chapter_to, r.verse_to)),
            r.from_year,
        );
    }
    rows
}

/// The Scripture set bound against the atlas authority (C2/C3): the
/// atlas dates and locates everything it can; the rest keeps its
/// disclosed stand-ins. None = fully stand-in (fixtures, tests).
pub fn scripture_timeline_with(atlas: Option<&AtlasExports>) -> WorldTimeline {
    let mut tl = WorldTimeline::default();
    for s in SURVEYS.iter().chain(SURVEYS_MORE) {
        add_survey(&mut tl, s, atlas);
    }
    for e in KINGDOMS {
        add_era(&mut tl, e, atlas);
    }
    for r in ROUTES {
        add_route(&mut tl, r, atlas);
    }
    tl.events.sort_by_key(|e| e.at);
    tl
}

/// The NUM 34 survey alone (the founding fixture; tests lean on it).
pub fn promised_land_timeline() -> WorldTimeline {
    let mut tl = WorldTimeline::default();
    add_survey(&mut tl, &SURVEYS[0], None);
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
