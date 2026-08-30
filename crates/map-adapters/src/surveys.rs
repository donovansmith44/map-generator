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
    Waypoint { name: "Brook of Egypt", lat: 31.16, lon: 33.80 },
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

// THE ALLOTMENT LATTICE (JOS 13-19), organic authoring. Every border
// two tribes share is authored ONCE as a densified, gently wiggling
// polyline and BOTH circuits walk the identical literals, so neighbors
// tile with no gap and no overlap. Coastal circuits overhang into the
// sea and the lakes; the Water layer paints after the claims, so the
// visible edge is the real natural-earth shoreline. West-bank circuits
// stop at lon 35.555, east-bank at 35.57: the hairline between is the
// Jordan (river geometry itself is a standing atlas ask). Intermediate
// "reach" waypoints are disclosed interpolation markers, not places.

const JOS_15_CIRCUIT: &[Waypoint] = &[
    Waypoint { name: "the sea before Judah", lat: 31.7200, lon: 34.4000 },
    Waypoint { name: "the coast toward Jabneel", lat: 31.7400, lon: 34.6200 },
    Waypoint { name: "the Sorek valley", lat: 31.7800, lon: 34.8500 },
    Waypoint { name: "the going down of Beth-shemesh", lat: 31.7200, lon: 35.0000 },
    Waypoint { name: "the saddle by Kiriath-jearim", lat: 31.7600, lon: 35.1200 },
    Waypoint { name: "south of Jebus, Hinnom", lat: 31.7400, lon: 35.2100 },
    Waypoint { name: "the wilderness toward Jericho", lat: 31.7800, lon: 35.3000 },
    Waypoint { name: "the descent to the Salt Sea", lat: 31.7200, lon: 35.4200 },
    Waypoint { name: "the Salt Sea's north bay", lat: 31.7400, lon: 35.5200 },
    Waypoint { name: "judah deadsea reach 0.1", lat: 31.5954, lon: 35.4970 },
    Waypoint { name: "the Salt Sea under En-gedi", lat: 31.4500, lon: 35.4800 },
    Waypoint { name: "judah deadsea reach 1.1", lat: 31.3005, lon: 35.4980 },
    Waypoint { name: "the Salt Sea toward Zoar", lat: 31.1500, lon: 35.5000 },
    Waypoint { name: "the south end of the sea", lat: 31.0200, lon: 35.5000 },
    Waypoint { name: "judah south reach 0.1", lat: 30.9573, lon: 35.3763 },
    Waypoint { name: "the ascent of Akrabbim southward", lat: 30.9000, lon: 35.2500 },
    Waypoint { name: "judah south reach 1.1", lat: 30.8423, lon: 35.1196 },
    Waypoint { name: "judah south reach 1.2", lat: 30.8035, lon: 34.9820 },
    Waypoint { name: "the wilderness of Zin southward", lat: 30.7500, lon: 34.8500 },
    Waypoint { name: "judah south reach 2.1", lat: 30.7109, lon: 34.7350 },
    Waypoint { name: "judah south reach 2.2", lat: 30.6891, lon: 34.6150 },
    Waypoint { name: "toward Kadesh-barnea", lat: 30.6500, lon: 34.5000 },
    Waypoint { name: "judah south reach 3.1", lat: 30.6879, lon: 34.3758 },
    Waypoint { name: "the wilderness toward Shur", lat: 30.7200, lon: 34.2500 },
    Waypoint { name: "the sea toward the river of Egypt", lat: 30.8500, lon: 34.0500 },
    Waypoint { name: "the shore below Gerar", lat: 30.9200, lon: 34.2800 },
    Waypoint { name: "judah simeon reach 6.2", lat: 30.9360, lon: 34.4196 },
    Waypoint { name: "judah simeon reach 6.1", lat: 30.9340, lon: 34.5604 },
    Waypoint { name: "the pastures toward Gerar", lat: 30.9500, lon: 34.7000 },
    Waypoint { name: "judah simeon reach 5.1", lat: 31.0000, lon: 34.8250 },
    Waypoint { name: "the wells at Beersheba eastward", lat: 31.0500, lon: 34.9500 },
    Waypoint { name: "toward Moladah", lat: 31.2500, lon: 35.0500 },
    Waypoint { name: "the border above Beersheba", lat: 31.4200, lon: 35.0000 },
    Waypoint { name: "toward Ziklag northward", lat: 31.4500, lon: 34.8500 },
    Waypoint { name: "judah simeon reach 1.1", lat: 31.4927, lon: 34.7373 },
    Waypoint { name: "the fields of Gerar northward", lat: 31.5200, lon: 34.6200 },
    Waypoint { name: "judah simeon reach 0.1", lat: 31.5380, lon: 34.4703 },
    Waypoint { name: "the shore above Gerar", lat: 31.5500, lon: 34.3200 },
];

const JOS_18_BENJAMIN: &[Waypoint] = &[
    Waypoint { name: "the going down of Beth-shemesh", lat: 31.7200, lon: 35.0000 },
    Waypoint { name: "the border under Aijalon", lat: 31.8300, lon: 35.0500 },
    Waypoint { name: "the going up to Beth-horon", lat: 31.9500, lon: 35.0000 },
    Waypoint { name: "toward Bethel southward", lat: 31.9400, lon: 35.2000 },
    Waypoint { name: "the wilderness of Beth-aven", lat: 31.9000, lon: 35.3500 },
    Waypoint { name: "north of Jericho", lat: 31.9300, lon: 35.4700 },
    Waypoint { name: "the Jordan at Jericho", lat: 31.9000, lon: 35.5550 },
    Waypoint { name: "the Salt Sea's north bay", lat: 31.7400, lon: 35.5200 },
    Waypoint { name: "the descent to the Salt Sea", lat: 31.7200, lon: 35.4200 },
    Waypoint { name: "the wilderness toward Jericho", lat: 31.7800, lon: 35.3000 },
    Waypoint { name: "south of Jebus, Hinnom", lat: 31.7400, lon: 35.2100 },
    Waypoint { name: "the saddle by Kiriath-jearim", lat: 31.7600, lon: 35.1200 },
];

const JOS_16_EPHRAIM: &[Waypoint] = &[
    Waypoint { name: "the sea by Me-jarkon", lat: 32.0200, lon: 34.6800 },
    Waypoint { name: "the plain toward Japho", lat: 32.0000, lon: 34.8500 },
    Waypoint { name: "the going up to Beth-horon", lat: 31.9500, lon: 35.0000 },
    Waypoint { name: "toward Bethel southward", lat: 31.9400, lon: 35.2000 },
    Waypoint { name: "the wilderness of Beth-aven", lat: 31.9000, lon: 35.3500 },
    Waypoint { name: "north of Jericho", lat: 31.9300, lon: 35.4700 },
    Waypoint { name: "the Jordan at Jericho", lat: 31.9000, lon: 35.5550 },
    Waypoint { name: "eph jordan reach 0.1", lat: 32.0750, lon: 35.5550 },
    Waypoint { name: "the Jordan by Adam", lat: 32.2500, lon: 35.5550 },
    Waypoint { name: "the descent toward the Jordan", lat: 32.2800, lon: 35.4500 },
    Waypoint { name: "before Shechem southward", lat: 32.1800, lon: 35.3500 },
    Waypoint { name: "the spring of Tappuah", lat: 32.2500, lon: 35.2000 },
    Waypoint { name: "the Kanah brook upward", lat: 32.2200, lon: 35.0500 },
    Waypoint { name: "the mouth of Kanah", lat: 32.2800, lon: 34.9000 },
    Waypoint { name: "the sea at the Kanah brook", lat: 32.2800, lon: 34.7200 },
];

const JOS_17_MANASSEH_WEST: &[Waypoint] = &[
    Waypoint { name: "the sea at the Kanah brook", lat: 32.2800, lon: 34.7200 },
    Waypoint { name: "the mouth of Kanah", lat: 32.2800, lon: 34.9000 },
    Waypoint { name: "the Kanah brook upward", lat: 32.2200, lon: 35.0500 },
    Waypoint { name: "the spring of Tappuah", lat: 32.2500, lon: 35.2000 },
    Waypoint { name: "before Shechem southward", lat: 32.1800, lon: 35.3500 },
    Waypoint { name: "the descent toward the Jordan", lat: 32.2800, lon: 35.4500 },
    Waypoint { name: "the Jordan by Adam", lat: 32.2500, lon: 35.5550 },
    Waypoint { name: "the Jordan under Beth-shean", lat: 32.3800, lon: 35.5550 },
    Waypoint { name: "above Beth-shean", lat: 32.4000, lon: 35.4800 },
    Waypoint { name: "the spring of Harod", lat: 32.4800, lon: 35.3800 },
    Waypoint { name: "the valley toward Jezreel", lat: 32.5800, lon: 35.2800 },
    Waypoint { name: "the valley before Megiddo", lat: 32.6000, lon: 35.2200 },
    Waypoint { name: "the edge of the great valley", lat: 32.6300, lon: 35.1200 },
    Waypoint { name: "under Jokneam", lat: 32.6800, lon: 35.0200 },
    Waypoint { name: "the shoulder of Carmel", lat: 32.7000, lon: 34.9500 },
    Waypoint { name: "the sea under Carmel", lat: 32.7200, lon: 34.8500 },
];

const JOS_19_SIMEON: &[Waypoint] = &[
    Waypoint { name: "the shore above Gerar", lat: 31.5500, lon: 34.3200 },
    Waypoint { name: "judah simeon reach 0.1", lat: 31.5380, lon: 34.4703 },
    Waypoint { name: "the fields of Gerar northward", lat: 31.5200, lon: 34.6200 },
    Waypoint { name: "judah simeon reach 1.1", lat: 31.4927, lon: 34.7373 },
    Waypoint { name: "toward Ziklag northward", lat: 31.4500, lon: 34.8500 },
    Waypoint { name: "the border above Beersheba", lat: 31.4200, lon: 35.0000 },
    Waypoint { name: "toward Moladah", lat: 31.2500, lon: 35.0500 },
    Waypoint { name: "the wells at Beersheba eastward", lat: 31.0500, lon: 34.9500 },
    Waypoint { name: "judah simeon reach 5.1", lat: 31.0000, lon: 34.8250 },
    Waypoint { name: "the pastures toward Gerar", lat: 30.9500, lon: 34.7000 },
    Waypoint { name: "judah simeon reach 6.1", lat: 30.9340, lon: 34.5604 },
    Waypoint { name: "judah simeon reach 6.2", lat: 30.9360, lon: 34.4196 },
    Waypoint { name: "the shore below Gerar", lat: 30.9200, lon: 34.2800 },
];

const JOS_19_ZEBULUN: &[Waypoint] = &[
    Waypoint { name: "under Jokneam", lat: 32.6800, lon: 35.0200 },
    Waypoint { name: "the edge of the great valley", lat: 32.6300, lon: 35.1200 },
    Waypoint { name: "the valley before Megiddo", lat: 32.6000, lon: 35.2200 },
    Waypoint { name: "the valley toward Jezreel", lat: 32.5800, lon: 35.2800 },
    Waypoint { name: "the oak in Zaanannim", lat: 32.6400, lon: 35.3500 },
    Waypoint { name: "the slopes under Rimmon", lat: 32.7800, lon: 35.3300 },
    Waypoint { name: "the border above Hannathon", lat: 32.8800, lon: 35.3000 },
    Waypoint { name: "the valley of Jiphthah-el northward", lat: 32.8800, lon: 35.2200 },
    Waypoint { name: "the hill before Jiphthah-el", lat: 32.7800, lon: 35.0800 },
];

const JOS_19_ISSACHAR: &[Waypoint] = &[
    Waypoint { name: "the valley toward Jezreel", lat: 32.5800, lon: 35.2800 },
    Waypoint { name: "the spring of Harod", lat: 32.4800, lon: 35.3800 },
    Waypoint { name: "above Beth-shean", lat: 32.4000, lon: 35.4800 },
    Waypoint { name: "the Jordan under Beth-shean", lat: 32.3800, lon: 35.5550 },
    Waypoint { name: "iss jordan reach 0.1", lat: 32.5400, lon: 35.5550 },
    Waypoint { name: "the Jordan at Chinnereth's outflowing", lat: 32.7000, lon: 35.5550 },
    Waypoint { name: "under the slopes of Tabor", lat: 32.7000, lon: 35.4700 },
    Waypoint { name: "the oak in Zaanannim", lat: 32.6400, lon: 35.3500 },
];

const JOS_19_ASHER: &[Waypoint] = &[
    Waypoint { name: "under Jokneam", lat: 32.6800, lon: 35.0200 },
    Waypoint { name: "the hill before Jiphthah-el", lat: 32.7800, lon: 35.0800 },
    Waypoint { name: "the valley of Jiphthah-el northward", lat: 32.8800, lon: 35.2200 },
    Waypoint { name: "the border above Hannathon", lat: 32.8800, lon: 35.3000 },
    Waypoint { name: "the height of Ramah northward", lat: 33.0500, lon: 35.2800 },
    Waypoint { name: "the hills above Kedesh", lat: 33.2000, lon: 35.3200 },
    Waypoint { name: "the north border under Lebanon", lat: 33.3200, lon: 35.3500 },
    Waypoint { name: "asher north reach 0.1", lat: 33.3120, lon: 35.2000 },
    Waypoint { name: "the sea toward great Zidon", lat: 33.3200, lon: 35.0500 },
    Waypoint { name: "asher west reach 0.1", lat: 33.1600, lon: 35.0000 },
    Waypoint { name: "the coast under Achzib", lat: 33.0000, lon: 34.9500 },
    Waypoint { name: "asher west reach 1.1", lat: 32.8563, lon: 34.9104 },
    Waypoint { name: "the sea under Carmel", lat: 32.7200, lon: 34.8500 },
    Waypoint { name: "under Jokneam", lat: 32.6800, lon: 35.0200 },
    Waypoint { name: "the shoulder of Carmel", lat: 32.7000, lon: 34.9500 },
    Waypoint { name: "the sea under Carmel", lat: 32.7200, lon: 34.8500 },
];

const JOS_19_NAPHTALI: &[Waypoint] = &[
    Waypoint { name: "the oak in Zaanannim", lat: 32.6400, lon: 35.3500 },
    Waypoint { name: "under the slopes of Tabor", lat: 32.7000, lon: 35.4700 },
    Waypoint { name: "the Jordan at Chinnereth's outflowing", lat: 32.7000, lon: 35.5550 },
    Waypoint { name: "the shore of Chinnereth westward", lat: 32.7800, lon: 35.5800 },
    Waypoint { name: "Chinnereth under Capernaum", lat: 32.9500, lon: 35.6000 },
    Waypoint { name: "the waters of Merom eastward", lat: 33.1000, lon: 35.6000 },
    Waypoint { name: "the upper Jordan northward", lat: 33.2500, lon: 35.6100 },
    Waypoint { name: "toward Ijon", lat: 33.3200, lon: 35.5800 },
    Waypoint { name: "the north border under Lebanon", lat: 33.3200, lon: 35.3500 },
    Waypoint { name: "the hills above Kedesh", lat: 33.2000, lon: 35.3200 },
    Waypoint { name: "the height of Ramah northward", lat: 33.0500, lon: 35.2800 },
    Waypoint { name: "the border above Hannathon", lat: 32.8800, lon: 35.3000 },
    Waypoint { name: "the slopes under Rimmon", lat: 32.7800, lon: 35.3300 },
];

const JOS_19_DAN: &[Waypoint] = &[
    Waypoint { name: "the sea by Me-jarkon", lat: 32.0200, lon: 34.6800 },
    Waypoint { name: "the plain toward Japho", lat: 32.0000, lon: 34.8500 },
    Waypoint { name: "the going up to Beth-horon", lat: 31.9500, lon: 35.0000 },
    Waypoint { name: "the border under Aijalon", lat: 31.8300, lon: 35.0500 },
    Waypoint { name: "the going down of Beth-shemesh", lat: 31.7200, lon: 35.0000 },
    Waypoint { name: "the Sorek valley", lat: 31.7800, lon: 34.8500 },
    Waypoint { name: "the coast toward Jabneel", lat: 31.7400, lon: 34.6200 },
    Waypoint { name: "the sea before Judah", lat: 31.7200, lon: 34.4000 },
];

const JOS_13_REUBEN: &[Waypoint] = &[
    Waypoint { name: "the Salt Sea's eastern shore", lat: 31.7600, lon: 35.5000 },
    Waypoint { name: "reuben west reach 0.1", lat: 31.6057, lon: 35.5210 },
    Waypoint { name: "the mouth of the Arnon", lat: 31.4500, lon: 35.5200 },
    Waypoint { name: "the Arnon gorge eastward", lat: 31.4800, lon: 35.7000 },
    Waypoint { name: "the brink of the Arnon at Aroer", lat: 31.4400, lon: 35.8500 },
    Waypoint { name: "the high plain by Dibon", lat: 31.5000, lon: 36.0000 },
    Waypoint { name: "the wilderness beyond Mephaath", lat: 31.4700, lon: 36.1500 },
    Waypoint { name: "the desert rim of Moab", lat: 31.7000, lon: 36.1800 },
    Waypoint { name: "the border toward Ammon", lat: 31.8800, lon: 36.1200 },
    Waypoint { name: "the plain above Heshbon", lat: 31.9200, lon: 35.9000 },
    Waypoint { name: "the fields of Abel-shittim", lat: 31.8800, lon: 35.7200 },
    Waypoint { name: "the Jordan by Beth-jeshimoth", lat: 31.9000, lon: 35.5700 },
];

const JOS_13_GAD: &[Waypoint] = &[
    Waypoint { name: "the Jordan by Beth-jeshimoth", lat: 31.9000, lon: 35.5700 },
    Waypoint { name: "the Jordan toward Succoth", lat: 32.1000, lon: 35.5700 },
    Waypoint { name: "the mouth of the Jabbok", lat: 32.2000, lon: 35.5700 },
    Waypoint { name: "the Jordan under Zaphon", lat: 32.3500, lon: 35.5700 },
    Waypoint { name: "the Jabbok toward Gerasa", lat: 32.3000, lon: 35.7200 },
    Waypoint { name: "the upper Jabbok at Mahanaim", lat: 32.2200, lon: 35.8500 },
    Waypoint { name: "the hills toward Ramoth", lat: 32.3500, lon: 36.0000 },
    Waypoint { name: "toward Ramoth in Gilead", lat: 32.3500, lon: 36.1000 },
    Waypoint { name: "gad east reach 0.1", lat: 32.2284, lon: 36.1505 },
    Waypoint { name: "the desert rim of Gilead", lat: 32.1000, lon: 36.1800 },
    Waypoint { name: "the border toward Ammon", lat: 31.8800, lon: 36.1200 },
    Waypoint { name: "the plain above Heshbon", lat: 31.9200, lon: 35.9000 },
    Waypoint { name: "the fields of Abel-shittim", lat: 31.8800, lon: 35.7200 },
];

const JOS_13_MANASSEH_EAST: &[Waypoint] = &[
    Waypoint { name: "the Jordan under Zaphon", lat: 32.3500, lon: 35.5700 },
    Waypoint { name: "the Jordan toward Chinnereth eastward", lat: 32.5500, lon: 35.5800 },
    Waypoint { name: "Chinnereth's east shore southward", lat: 32.7200, lon: 35.6000 },
    Waypoint { name: "the east shore of Chinnereth", lat: 32.8000, lon: 35.6000 },
    Waypoint { name: "above Chinnereth eastward", lat: 32.9000, lon: 35.6000 },
    Waypoint { name: "the upper Jordan eastward", lat: 33.1000, lon: 35.6300 },
    Waypoint { name: "toward Dan of the north", lat: 33.2500, lon: 35.6500 },
    Waypoint { name: "under mount Hermon", lat: 33.3000, lon: 35.7200 },
    Waypoint { name: "me northeast reach 0.1", lat: 33.2930, lon: 35.8602 },
    Waypoint { name: "the slopes of Hermon eastward", lat: 33.2800, lon: 36.0000 },
    Waypoint { name: "me northeast reach 1.1", lat: 33.2150, lon: 36.1500 },
    Waypoint { name: "the border of Maakah", lat: 33.1500, lon: 36.3000 },
    Waypoint { name: "me northeast reach 2.1", lat: 33.0250, lon: 36.4000 },
    Waypoint { name: "the coasts of Argob", lat: 32.9000, lon: 36.5000 },
    Waypoint { name: "toward Salecah", lat: 32.7500, lon: 36.5500 },
    Waypoint { name: "me northeast reach 4.1", lat: 32.6500, lon: 36.4250 },
    Waypoint { name: "the rim of Bashan", lat: 32.5500, lon: 36.3000 },
    Waypoint { name: "by Edrei", lat: 32.5500, lon: 36.1000 },
    Waypoint { name: "toward Ramoth in Gilead", lat: 32.3500, lon: 36.1000 },
    Waypoint { name: "the hills toward Ramoth", lat: 32.3500, lon: 36.0000 },
    Waypoint { name: "the upper Jabbok at Mahanaim", lat: 32.2200, lon: 35.8500 },
    Waypoint { name: "the Jabbok toward Gerasa", lat: 32.3000, lon: 35.7200 },
];

// ------------------------- the traced plate contour (calibration proof)
//
// One region of the owner's reference plate, georeferenced and traced:
// the pixel->position function is an affine fit over 12 detected city
// dots (mean residual 1.6 km, max 2.8 km), the border is the region's
// color mask contour (~75 m/px), Douglas-Peucker simplified at ~300 m.
// Every waypoint is an interpolation marker of that tracing, not a
// place. This is the precision reference the tribal circuits converge
// to; the method spreads region by region.

const PLATE_CANAAN_CONTOUR: &[Waypoint] = &[
    Waypoint { name: "canaan contour 000", lat: 33.45371, lon: 35.66209 },
    Waypoint { name: "canaan contour 001", lat: 33.42445, lon: 35.62288 },
    Waypoint { name: "canaan contour 002", lat: 33.39677, lon: 35.60376 },
    Waypoint { name: "canaan contour 003", lat: 33.39530, lon: 35.57774 },
    Waypoint { name: "canaan contour 004", lat: 33.37539, lon: 35.56100 },
    Waypoint { name: "canaan contour 005", lat: 33.32080, lon: 35.55099 },
    Waypoint { name: "canaan contour 006", lat: 33.31118, lon: 35.54263 },
    Waypoint { name: "canaan contour 007", lat: 33.28334, lon: 35.53167 },
    Waypoint { name: "canaan contour 008", lat: 33.19385, lon: 35.50462 },
    Waypoint { name: "canaan contour 009", lat: 32.99502, lon: 35.43080 },
    Waypoint { name: "canaan contour 010", lat: 32.96732, lon: 35.41242 },
    Waypoint { name: "canaan contour 011", lat: 32.89036, lon: 35.37819 },
    Waypoint { name: "canaan contour 012", lat: 32.80002, lon: 35.32662 },
    Waypoint { name: "canaan contour 013", lat: 32.68974, lon: 35.26053 },
    Waypoint { name: "canaan contour 014", lat: 32.65823, lon: 35.23688 },
    Waypoint { name: "canaan contour 015", lat: 32.64307, lon: 35.21058 },
    Waypoint { name: "canaan contour 016", lat: 32.64341, lon: 35.19276 },
    Waypoint { name: "canaan contour 017", lat: 32.65443, lon: 35.16105 },
    Waypoint { name: "canaan contour 018", lat: 32.69447, lon: 35.11285 },
    Waypoint { name: "canaan contour 019", lat: 32.74095, lon: 35.10339 },
    Waypoint { name: "canaan contour 020", lat: 32.76893, lon: 35.07202 },
    Waypoint { name: "canaan contour 021", lat: 32.79179, lon: 35.07026 },
    Waypoint { name: "canaan contour 022", lat: 32.81459, lon: 35.03656 },
    Waypoint { name: "canaan contour 023", lat: 32.82506, lon: 35.03454 },
    Waypoint { name: "canaan contour 024", lat: 32.82522, lon: 35.02638 },
    Waypoint { name: "canaan contour 025", lat: 32.84727, lon: 34.99786 },
    Waypoint { name: "canaan contour 026", lat: 32.84876, lon: 34.98823 },
    Waypoint { name: "canaan contour 027", lat: 32.83866, lon: 34.97021 },
    Waypoint { name: "canaan contour 028", lat: 32.79772, lon: 34.96270 },
    Waypoint { name: "canaan contour 029", lat: 32.76902, lon: 34.96286 },
    Waypoint { name: "canaan contour 030", lat: 32.72420, lon: 34.95305 },
    Waypoint { name: "canaan contour 031", lat: 32.70626, lon: 34.93635 },
    Waypoint { name: "canaan contour 032", lat: 32.66910, lon: 34.93560 },
    Waypoint { name: "canaan contour 033", lat: 32.65033, lon: 34.92780 },
    Waypoint { name: "canaan contour 034", lat: 32.60154, lon: 34.92161 },
    Waypoint { name: "canaan contour 035", lat: 32.56266, lon: 34.90821 },
    Waypoint { name: "canaan contour 036", lat: 32.48468, lon: 34.89327 },
    Waypoint { name: "canaan contour 037", lat: 32.47507, lon: 34.88416 },
    Waypoint { name: "canaan contour 038", lat: 32.44315, lon: 34.88203 },
    Waypoint { name: "canaan contour 039", lat: 32.40497, lon: 34.86641 },
    Waypoint { name: "canaan contour 040", lat: 32.30886, lon: 34.84442 },
    Waypoint { name: "canaan contour 041", lat: 32.17080, lon: 34.79931 },
    Waypoint { name: "canaan contour 042", lat: 32.15142, lon: 34.78926 },
    Waypoint { name: "canaan contour 043", lat: 32.13773, lon: 34.78899 },
    Waypoint { name: "canaan contour 044", lat: 32.10275, lon: 34.77640 },
    Waypoint { name: "canaan contour 045", lat: 32.09831, lon: 34.76963 },
    Waypoint { name: "canaan contour 046", lat: 32.06652, lon: 34.76082 },
    Waypoint { name: "canaan contour 047", lat: 32.04531, lon: 34.74405 },
    Waypoint { name: "canaan contour 048", lat: 31.94295, lon: 34.70783 },
    Waypoint { name: "canaan contour 049", lat: 31.92499, lon: 34.69187 },
    Waypoint { name: "canaan contour 050", lat: 31.85063, lon: 34.65770 },
    Waypoint { name: "canaan contour 051", lat: 31.84491, lon: 34.65016 },
    Waypoint { name: "canaan contour 052", lat: 31.83382, lon: 34.64993 },
    Waypoint { name: "canaan contour 053", lat: 31.83079, lon: 34.63799 },
    Waypoint { name: "canaan contour 054", lat: 31.81843, lon: 34.63626 },
    Waypoint { name: "canaan contour 055", lat: 31.76417, lon: 34.60843 },
    Waypoint { name: "canaan contour 056", lat: 31.67849, lon: 34.55175 },
    Waypoint { name: "canaan contour 057", lat: 31.52941, lon: 34.43364 },
    Waypoint { name: "canaan contour 058", lat: 31.47946, lon: 34.38436 },
    Waypoint { name: "canaan contour 059", lat: 31.47489, lon: 34.38501 },
    Waypoint { name: "canaan contour 060", lat: 31.45746, lon: 34.40991 },
    Waypoint { name: "canaan contour 061", lat: 31.43515, lon: 34.41763 },
    Waypoint { name: "canaan contour 062", lat: 31.42582, lon: 34.42784 },
    Waypoint { name: "canaan contour 063", lat: 31.40360, lon: 34.43036 },
    Waypoint { name: "canaan contour 064", lat: 31.37901, lon: 34.45511 },
    Waypoint { name: "canaan contour 065", lat: 31.35064, lon: 34.47236 },
    Waypoint { name: "canaan contour 066", lat: 31.32634, lon: 34.48153 },
    Waypoint { name: "canaan contour 067", lat: 31.29444, lon: 34.47865 },
    Waypoint { name: "canaan contour 068", lat: 31.25252, lon: 34.48821 },
    Waypoint { name: "canaan contour 069", lat: 31.22812, lon: 34.50257 },
    Waypoint { name: "canaan contour 070", lat: 31.22126, lon: 34.52099 },
    Waypoint { name: "canaan contour 071", lat: 31.20006, lon: 34.53839 },
    Waypoint { name: "canaan contour 072", lat: 31.19502, lon: 34.56353 },
    Waypoint { name: "canaan contour 073", lat: 31.18556, lon: 34.58117 },
    Waypoint { name: "canaan contour 074", lat: 31.15983, lon: 34.59698 },
    Waypoint { name: "canaan contour 075", lat: 31.13176, lon: 34.63355 },
    Waypoint { name: "canaan contour 076", lat: 31.11404, lon: 34.67403 },
    Waypoint { name: "canaan contour 077", lat: 31.09315, lon: 34.74490 },
    Waypoint { name: "canaan contour 078", lat: 31.05886, lon: 34.90387 },
    Waypoint { name: "canaan contour 079", lat: 31.04070, lon: 34.96811 },
    Waypoint { name: "canaan contour 080", lat: 31.01336, lon: 35.03513 },
    Waypoint { name: "canaan contour 081", lat: 30.95568, lon: 35.22407 },
    Waypoint { name: "canaan contour 082", lat: 30.92763, lon: 35.29405 },
    Waypoint { name: "canaan contour 083", lat: 30.91014, lon: 35.32266 },
    Waypoint { name: "canaan contour 084", lat: 30.91478, lon: 35.35320 },
    Waypoint { name: "canaan contour 085", lat: 30.93010, lon: 35.37133 },
    Waypoint { name: "canaan contour 086", lat: 30.94825, lon: 35.37690 },
    Waypoint { name: "canaan contour 087", lat: 31.00819, lon: 35.38034 },
    Waypoint { name: "canaan contour 088", lat: 31.05016, lon: 35.40272 },
    Waypoint { name: "canaan contour 089", lat: 31.05472, lon: 35.40281 },
    Waypoint { name: "canaan contour 090", lat: 31.06051, lon: 35.37248 },
    Waypoint { name: "canaan contour 091", lat: 31.06650, lon: 35.36592 },
    Waypoint { name: "canaan contour 092", lat: 31.11625, lon: 35.35578 },
    Waypoint { name: "canaan contour 093", lat: 31.12943, lon: 35.34862 },
    Waypoint { name: "canaan contour 094", lat: 31.14011, lon: 35.33547 },
    Waypoint { name: "canaan contour 095", lat: 31.19633, lon: 35.32844 },
    Waypoint { name: "canaan contour 096", lat: 31.21457, lon: 35.32955 },
    Waypoint { name: "canaan contour 097", lat: 31.26868, lon: 35.36554 },
    Waypoint { name: "canaan contour 098", lat: 31.31223, lon: 35.37310 },
    Waypoint { name: "canaan contour 099", lat: 31.32567, lon: 35.38674 },
    Waypoint { name: "canaan contour 100", lat: 31.33948, lon: 35.38033 },
    Waypoint { name: "canaan contour 101", lat: 31.36297, lon: 35.38006 },
    Waypoint { name: "canaan contour 102", lat: 31.38078, lon: 35.36929 },
    Waypoint { name: "canaan contour 103", lat: 31.39186, lon: 35.36951 },
    Waypoint { name: "canaan contour 104", lat: 31.40307, lon: 35.36305 },
    Waypoint { name: "canaan contour 105", lat: 31.44985, lon: 35.37216 },
    Waypoint { name: "canaan contour 106", lat: 31.51314, lon: 35.37047 },
    Waypoint { name: "canaan contour 107", lat: 31.53649, lon: 35.37762 },
    Waypoint { name: "canaan contour 108", lat: 31.55188, lon: 35.39130 },
    Waypoint { name: "canaan contour 109", lat: 31.56828, lon: 35.38643 },
    Waypoint { name: "canaan contour 110", lat: 31.59370, lon: 35.38695 },
    Waypoint { name: "canaan contour 111", lat: 31.60531, lon: 35.39386 },
    Waypoint { name: "canaan contour 112", lat: 31.61355, lon: 35.40665 },
    Waypoint { name: "canaan contour 113", lat: 31.62528, lon: 35.40689 },
    Waypoint { name: "canaan contour 114", lat: 31.64978, lon: 35.42224 },
    Waypoint { name: "canaan contour 115", lat: 31.67519, lon: 35.42349 },
    Waypoint { name: "canaan contour 116", lat: 31.68682, lon: 35.42892 },
    Waypoint { name: "canaan contour 117", lat: 31.69773, lon: 35.43880 },
    Waypoint { name: "canaan contour 118", lat: 31.69940, lon: 35.45368 },
    Waypoint { name: "canaan contour 119", lat: 31.74911, lon: 35.48068 },
    Waypoint { name: "canaan contour 120", lat: 31.74819, lon: 35.52967 },
    Waypoint { name: "canaan contour 121", lat: 31.76263, lon: 35.52476 },
    Waypoint { name: "canaan contour 122", lat: 31.78350, lon: 35.52518 },
    Waypoint { name: "canaan contour 123", lat: 31.79340, lon: 35.51870 },
    Waypoint { name: "canaan contour 124", lat: 31.86114, lon: 35.52304 },
    Waypoint { name: "canaan contour 125", lat: 31.87037, lon: 35.51802 },
    Waypoint { name: "canaan contour 126", lat: 31.87178, lon: 35.51211 },
    Waypoint { name: "canaan contour 127", lat: 31.89145, lon: 35.50657 },
    Waypoint { name: "canaan contour 128", lat: 31.90969, lon: 35.50768 },
    Waypoint { name: "canaan contour 129", lat: 31.91926, lon: 35.51901 },
    Waypoint { name: "canaan contour 130", lat: 31.96030, lon: 35.52132 },
    Waypoint { name: "canaan contour 131", lat: 31.99970, lon: 35.50727 },
    Waypoint { name: "canaan contour 132", lat: 32.04218, lon: 35.50218 },
    Waypoint { name: "canaan contour 133", lat: 32.04593, lon: 35.51117 },
    Waypoint { name: "canaan contour 134", lat: 32.07503, lon: 35.52438 },
    Waypoint { name: "canaan contour 135", lat: 32.10376, lon: 35.52199 },
    Waypoint { name: "canaan contour 136", lat: 32.11272, lon: 35.53108 },
    Waypoint { name: "canaan contour 137", lat: 32.13556, lon: 35.53080 },
    Waypoint { name: "canaan contour 138", lat: 32.14121, lon: 35.54205 },
    Waypoint { name: "canaan contour 139", lat: 32.17707, lon: 35.54278 },
    Waypoint { name: "canaan contour 140", lat: 32.18601, lon: 35.55261 },
    Waypoint { name: "canaan contour 141", lat: 32.24274, lon: 35.55301 },
    Waypoint { name: "canaan contour 142", lat: 32.25263, lon: 35.54727 },
    Waypoint { name: "canaan contour 143", lat: 32.27929, lon: 35.55152 },
    Waypoint { name: "canaan contour 144", lat: 32.28592, lon: 35.54571 },
    Waypoint { name: "canaan contour 145", lat: 32.30808, lon: 35.54690 },
    Waypoint { name: "canaan contour 146", lat: 32.31337, lon: 35.54255 },
    Waypoint { name: "canaan contour 147", lat: 32.36225, lon: 35.54428 },
    Waypoint { name: "canaan contour 148", lat: 32.36345, lon: 35.55025 },
    Waypoint { name: "canaan contour 149", lat: 32.36865, lon: 35.55109 },
    Waypoint { name: "canaan contour 150", lat: 32.38445, lon: 35.54324 },
    Waypoint { name: "canaan contour 151", lat: 32.40009, lon: 35.54356 },
    Waypoint { name: "canaan contour 152", lat: 32.40131, lon: 35.54804 },
    Waypoint { name: "canaan contour 153", lat: 32.43978, lon: 35.54882 },
    Waypoint { name: "canaan contour 154", lat: 32.44027, lon: 35.55699 },
    Waypoint { name: "canaan contour 155", lat: 32.45197, lon: 35.55946 },
    Waypoint { name: "canaan contour 156", lat: 32.47483, lon: 35.55769 },
    Waypoint { name: "canaan contour 157", lat: 32.48509, lon: 35.56681 },
    Waypoint { name: "canaan contour 158", lat: 32.48965, lon: 35.56690 },
    Waypoint { name: "canaan contour 159", lat: 32.49906, lon: 35.55224 },
    Waypoint { name: "canaan contour 160", lat: 32.54145, lon: 35.55235 },
    Waypoint { name: "canaan contour 161", lat: 32.55031, lon: 35.56664 },
    Waypoint { name: "canaan contour 162", lat: 32.58938, lon: 35.56966 },
    Waypoint { name: "canaan contour 163", lat: 32.59471, lon: 35.56382 },
    Waypoint { name: "canaan contour 164", lat: 32.61361, lon: 35.56420 },
    Waypoint { name: "canaan contour 165", lat: 32.61893, lon: 35.55911 },
    Waypoint { name: "canaan contour 166", lat: 32.65735, lon: 35.56212 },
    Waypoint { name: "canaan contour 167", lat: 32.66860, lon: 35.55343 },
    Waypoint { name: "canaan contour 168", lat: 32.68555, lon: 35.55377 },
    Waypoint { name: "canaan contour 169", lat: 32.69193, lon: 35.56133 },
    Waypoint { name: "canaan contour 170", lat: 32.71083, lon: 35.56171 },
    Waypoint { name: "canaan contour 171", lat: 32.71465, lon: 35.56698 },
    Waypoint { name: "canaan contour 172", lat: 32.73943, lon: 35.56674 },
    Waypoint { name: "canaan contour 173", lat: 32.75387, lon: 35.56183 },
    Waypoint { name: "canaan contour 174", lat: 32.77372, lon: 35.54664 },
    Waypoint { name: "canaan contour 175", lat: 32.79730, lon: 35.54117 },
    Waypoint { name: "canaan contour 176", lat: 32.82183, lon: 35.52013 },
    Waypoint { name: "canaan contour 177", lat: 32.83943, lon: 35.52049 },
    Waypoint { name: "canaan contour 178", lat: 32.86208, lon: 35.52986 },
    Waypoint { name: "canaan contour 179", lat: 32.87362, lon: 35.54049 },
    Waypoint { name: "canaan contour 180", lat: 32.88036, lon: 35.56364 },
    Waypoint { name: "canaan contour 181", lat: 32.89804, lon: 35.59445 },
    Waypoint { name: "canaan contour 182", lat: 32.89773, lon: 35.61078 },
    Waypoint { name: "canaan contour 183", lat: 32.91779, lon: 35.61935 },
    Waypoint { name: "canaan contour 184", lat: 33.04806, lon: 35.62792 },
    Waypoint { name: "canaan contour 185", lat: 33.05801, lon: 35.61921 },
    Waypoint { name: "canaan contour 186", lat: 33.07081, lon: 35.59719 },
    Waypoint { name: "canaan contour 187", lat: 33.08711, lon: 35.59752 },
    Waypoint { name: "canaan contour 188", lat: 33.09164, lon: 35.59909 },
    Waypoint { name: "canaan contour 189", lat: 33.09840, lon: 35.62151 },
    Waypoint { name: "canaan contour 190", lat: 33.13739, lon: 35.62898 },
    Waypoint { name: "canaan contour 191", lat: 33.15697, lon: 35.62789 },
    Waypoint { name: "canaan contour 192", lat: 33.17352, lon: 35.61485 },
    Waypoint { name: "canaan contour 193", lat: 33.20082, lon: 35.61986 },
    Waypoint { name: "canaan contour 194", lat: 33.21399, lon: 35.61270 },
    Waypoint { name: "canaan contour 195", lat: 33.23420, lon: 35.61311 },
    Waypoint { name: "canaan contour 196", lat: 33.25412, lon: 35.62910 },
    Waypoint { name: "canaan contour 197", lat: 33.29595, lon: 35.62401 },
    Waypoint { name: "canaan contour 198", lat: 33.32364, lon: 35.64313 },
    Waypoint { name: "canaan contour 199", lat: 33.36611, lon: 35.63879 },
    Waypoint { name: "canaan contour 200", lat: 33.41712, lon: 35.66581 },
    Waypoint { name: "canaan contour 201", lat: 33.42736, lon: 35.67641 },
    Waypoint { name: "canaan contour 202", lat: 33.43485, lon: 35.69438 },
    Waypoint { name: "canaan contour 203", lat: 33.44345, lon: 35.68787 },
    Waypoint { name: "canaan contour 204", lat: 33.45370, lon: 35.66283 },
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
    Waypoint { name: "Brook of Egypt", lat: 31.16, lon: 33.80 },
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
    Waypoint { name: "Brook of Egypt", lat: 31.16, lon: 33.80 },
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
        tag: "PLATE-CANAAN",
        label: "Canaan (traced contour)",
        note: "Georeferenced tracing of the reference plate's Canaan region \
               (affine calibration over 12 city dots, mean residual 1.6 km); \
               waypoints are tracing markers, not places.",
        book: 4, chapter: 34, verse_from: 1, verse_to: 12,
        year: -2200,
        grade: Grade::CityDerived,
        circuit: PLATE_CANAAN_CONTOUR,
    },
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

circuit!(R_ABRAHAM:
    (30.96,46.10,"Ur of the Chaldees"),(36.87,39.03,"Haran"),(32.21,35.28,"Sichem"),
    (31.93,35.22,"Bethel with Hai eastward"),(30.50,31.20,"Egypt in the famine"),
    (31.25,34.79,"the south country"),(31.93,35.22,"Bethel again, to the altar"),
    (31.54,35.09,"the plain of Mamre in Hebron"));

circuit!(R_JACOB:
    (31.24,34.84,"Beer-sheba"),(31.93,35.22,"Bethel, the ladder"),(36.87,39.03,"Haran, to Laban"),
    (32.35,35.85,"Mahanaim"),(32.19,35.61,"Peniel"),(32.20,35.65,"Succoth of Jacob"),
    (32.21,35.28,"Shalem, a city of Shechem"),(31.93,35.22,"Bethel, El-beth-el"),
    (31.70,35.20,"the way to Ephrath, which is Bethlehem"),(31.54,35.09,"Mamre, unto Isaac"));

circuit!(R_JOSEPH:
    (31.54,35.09,"the vale of Hebron"),(32.21,35.28,"Shechem, seeking his brethren"),
    (32.40,35.35,"Dothan, the pit"),(30.55,32.10,"the way of the Ishmeelites"),
    (30.05,31.25,"Egypt, Potiphar's house"));

circuit!(R_SPIES:
    (30.69,34.49,"the wilderness of Zin"),(31.25,34.79,"the south country of the spies"),
    (31.54,35.09,"Hebron, where Anak's children were"),(31.68,35.10,"the brook of Eshcol"),
    (33.25,35.85,"Rehob, as men come to Hamath"),(30.69,34.49,"Kadesh, to bring word"));

circuit!(R_ARK:
    (31.90,34.90,"Eben-ezer, where Israel pitched"),(31.75,34.65,"Ashdod, the house of Dagon"),
    (31.70,34.85,"Gath of the Philistines"),(31.78,34.99,"Ekron"),
    (31.75,34.97,"Beth-shemesh, the great stone"),(31.80,35.10,"Kirjath-jearim, twenty years"));

circuit!(R_ELIJAH:
    (32.56,35.33,"Jezreel, before Ahab's chariot"),(31.24,34.84,"Beer-sheba of Judah"),
    (30.85,34.60,"a day's journey into the wilderness"),(28.54,33.97,"Horeb the mount of God"));

circuit!(R_JONAH:
    (32.74,35.34,"Gath-hepher"),(32.05,34.75,"Joppa, the ship to Tarshish"),
    (33.50,32.50,"the sea, the great fish"),(36.36,43.15,"Nineveh, that great city"));

circuit!(R_EXILE:
    (31.78,35.22,"Jerusalem, the city broken up"),(34.45,36.52,"Riblah in the land of Hamath"),
    (35.10,40.42,"the way of the plain"),(32.54,44.42,"Babylon, by the rivers"));

circuit!(R_RETURN:
    (32.54,44.42,"Babylon, when the LORD turned the captivity"),
    (35.10,40.42,"the river of Ahava"),(34.45,36.52,"the crossing of the west"),
    (31.78,35.22,"Jerusalem, the house of the LORD"));

circuit!(R_NATIVITY:
    (32.70,35.30,"Nazareth of Galilee"),(31.70,35.20,"Bethlehem of Judaea"),
    (30.50,31.20,"Egypt, out of which the Son was called"),(32.70,35.30,"Nazareth, that it might be fulfilled"));

circuit!(R_MINISTRY:
    (32.88,35.58,"Capernaum, his own city"),(32.21,35.28,"Sychar's country of Samaria"),
    (31.87,35.44,"Jericho, where Zacchaeus climbed"),(31.77,35.26,"Bethany, at the mount of Olives"),
    (31.78,35.22,"Jerusalem, the city of the great King"));

circuit!(R_PHILIP:
    (31.78,35.22,"Jerusalem"),(31.53,34.60,"the way that goeth down to Gaza"),
    (31.75,34.65,"Azotus, where Philip was found"),(32.50,34.89,"Caesarea, preaching in all the cities"));

circuit!(R_DAMASCUS:
    (31.78,35.22,"Jerusalem, breathing threatenings"),(32.60,35.50,"the road north"),
    (33.51,36.29,"Damascus, the street called Straight"));

circuit!(R_PETER:
    (31.95,34.89,"Lydda, where Aeneas lay"),(32.05,34.75,"Joppa, the house of Simon a tanner"),
    (32.50,34.89,"Caesarea, the house of Cornelius"));

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
        tag: "R-ABRAHAM", note: ROUTE_NOTE,
        book: 1, chapter_from: 11, verse_from: 31, chapter_to: 13, verse_to: 18,
        from_year: -1921, to_year: Some(-1917),
        stations: R_ABRAHAM,
    },
    RouteSpec {
        tag: "R-JACOB", note: ROUTE_NOTE,
        book: 1, chapter_from: 28, verse_from: 10, chapter_to: 35, verse_to: 27,
        from_year: -1760, to_year: Some(-1739),
        stations: R_JACOB,
    },
    RouteSpec {
        tag: "R-JOSEPH", note: ROUTE_NOTE,
        book: 1, chapter_from: 37, verse_from: 12, chapter_to: 37, verse_to: 36,
        from_year: -1729, to_year: None,
        stations: R_JOSEPH,
    },
    RouteSpec {
        tag: "R-SPIES", note: ROUTE_NOTE,
        book: 4, chapter_from: 13, verse_from: 17, chapter_to: 13, verse_to: 26,
        from_year: -1490, to_year: None,
        stations: R_SPIES,
    },
    RouteSpec {
        tag: "R-ARK", note: ROUTE_NOTE,
        book: 9, chapter_from: 4, verse_from: 1, chapter_to: 7, verse_to: 2,
        from_year: -1141, to_year: None,
        stations: R_ARK,
    },
    RouteSpec {
        tag: "R-ELIJAH", note: ROUTE_NOTE,
        book: 11, chapter_from: 19, verse_from: 1, chapter_to: 19, verse_to: 8,
        from_year: -906, to_year: None,
        stations: R_ELIJAH,
    },
    RouteSpec {
        tag: "R-JONAH", note: ROUTE_NOTE,
        book: 32, chapter_from: 1, verse_from: 3, chapter_to: 3, verse_to: 3,
        from_year: -787, to_year: None,
        stations: R_JONAH,
    },
    RouteSpec {
        tag: "R-EXILE", note: ROUTE_NOTE,
        book: 12, chapter_from: 25, verse_from: 1, chapter_to: 25, verse_to: 21,
        from_year: -586, to_year: None,
        stations: R_EXILE,
    },
    RouteSpec {
        tag: "R-RETURN", note: ROUTE_NOTE,
        book: 15, chapter_from: 1, verse_from: 1, chapter_to: 2, verse_to: 70,
        from_year: -536, to_year: None,
        stations: R_RETURN,
    },
    RouteSpec {
        tag: "R-NATIVITY", note: ROUTE_NOTE,
        book: 40, chapter_from: 2, verse_from: 1, chapter_to: 2, verse_to: 23,
        from_year: -4, to_year: Some(-2),
        stations: R_NATIVITY,
    },
    RouteSpec {
        tag: "R-MINISTRY", note: ROUTE_NOTE,
        book: 42, chapter_from: 9, verse_from: 51, chapter_to: 19, verse_to: 28,
        from_year: 33, to_year: None,
        stations: R_MINISTRY,
    },
    RouteSpec {
        tag: "R-PHILIP", note: ROUTE_NOTE,
        book: 44, chapter_from: 8, verse_from: 26, chapter_to: 8, verse_to: 40,
        from_year: 34, to_year: None,
        stations: R_PHILIP,
    },
    RouteSpec {
        tag: "R-DAMASCUS", note: ROUTE_NOTE,
        book: 44, chapter_from: 9, verse_from: 1, chapter_to: 9, verse_to: 19,
        from_year: 35, to_year: None,
        stations: R_DAMASCUS,
    },
    RouteSpec {
        tag: "R-PETER", note: ROUTE_NOTE,
        book: 44, chapter_from: 9, verse_from: 32, chapter_to: 10, verse_to: 48,
        from_year: 37, to_year: None,
        stations: R_PETER,
    },
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
        from_year: 45, to_year: Some(47),
        stations: R_PAUL1,
    },
    RouteSpec {
        tag: "R-PAUL2", note: ROUTE_NOTE,
        book: 44, chapter_from: 15, verse_from: 36, chapter_to: 18, verse_to: 22,
        from_year: 49, to_year: Some(52),
        stations: R_PAUL2,
    },
    RouteSpec {
        tag: "R-PAUL3", note: ROUTE_NOTE,
        book: 44, chapter_from: 18, verse_from: 23, chapter_to: 21, verse_to: 17,
        from_year: 53, to_year: Some(57),
        stations: R_PAUL3,
    },
    RouteSpec {
        tag: "R-ROME", note: ROUTE_NOTE,
        book: 44, chapter_from: 27, verse_from: 1, chapter_to: 28, verse_to: 16,
        from_year: 60, to_year: Some(62),
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
    // A journey happens IN TIME: the interval runs departure through
    // arrival (half-open, so `to` is the year after arrival — minding
    // the missing year zero). A snapshot outside the span shows none
    // of it; a snapshot mid-walk shows the road so far.
    let (from_year, to_year) = match &resolved {
        Some((_, fy, ty)) => (*fy, if ty > fy { Some(*ty) } else { r.to_year }),
        None => (r.from_year, r.to_year),
    };
    let arrival = to_year.unwrap_or(from_year).max(from_year);
    let year_after = if arrival == -1 { 1 } else { arrival + 1 };
    let (pts, waypoints, bound) = resolve_circuit(r.stations, atlas);
    let provenance = circuit_provenance(atlas, bound, r.stations.len());
    let bid = BoundaryId(hash_id(&format!("scripture-route/{}", r.tag)));
    tl.boundaries.insert(
        bid,
        BoundaryHistory {
            versions: vec![(
                Interval { from: tp(from_year), to: Some(tp(year_after)) },
                Boundary {
                    pts,
                    // A journey is a WAY, never a border — its own
                    // character, its own dress in every style.
                    character: EdgeCharacter::Way,
                    source: BoundarySource::Survey(BorderSurvey {
                        verses,
                        waypoints,
                        interpolation: InterpolationMethod::Geodesic,
                        provenance: provenance.clone(),
                    }),
                    justification: justification.clone(),
                    provenance: provenance.clone(),
                },
            )],
        },
    );
    // Every journey is a scrub stop: the departure always, and the
    // arrival too when the walk crosses years — so the scrubber can
    // land mid-walk (the road so far) and at journey's end (the whole
    // way), and the range picker can bracket it.
    let mut push_stop = |year: i32| {
        tl.events.push(ChangeEvent {
            at: tp(year),
            kind: ChangeKind::Journey { boundary: bid },
            driver: None,
            justification: justification.clone(),
            provenance: provenance.clone(),
        });
    };
    push_stop(from_year);
    if arrival > from_year {
        push_stop(arrival);
    }
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
            // A fall realizes at the span's END: the atlas may place
            // siege-through-fall as an interval; the fall is its
            // consummation (their curation note agrees).
            Some((eid, _, ty)) => {
                (ty, atlas.map(|a| AtlasEventRef { event: eid, atlas_root: a.root }))
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
            // Falls adopt the span END — audit against the same.
            let res = atlas
                .resolve_event(book, (ch, v0), (ch, v1))
                .map(|(eid, _, ty)| (eid, ty, ty));
            push(format!("{}/fall", e.tag), res, y);
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

/// The authored route book, exposed for the canon compiler's
/// reconciliation against atlas narratives (2026-08-27 design). Each
/// row is the spec's own data: tag, verse span, Ussher-stand-in years,
/// and the stations with their stand-in coordinates.
pub struct AuthoredRoute {
    pub tag: &'static str,
    pub display: &'static str,
    pub book: u8,
    pub chapter_from: u16,
    pub chapter_to: u16,
    pub from_year: i32,
    pub to_year: Option<i32>,
    pub stations: Vec<(&'static str, f64, f64)>,
}

pub fn authored_routes() -> Vec<AuthoredRoute> {
    let display = |tag: &str| -> &'static str {
        match tag {
            "R-ABRAHAM" => "Abraham's call (GEN 11-13)",
            "R-JACOB" => "Jacob to Haran and back (GEN 28-35)",
            "R-JOSEPH" => "Joseph to Egypt (GEN 37)",
            "R-SPIES" => "the spies' circuit (NUM 13)",
            "R-ARK" => "the ark among the Philistines (1SA 4-7)",
            "R-ELIJAH" => "Elijah to Horeb (1KI 19)",
            "R-JONAH" => "Jonah to Nineveh (JON 1-3)",
            "R-EXILE" => "the road into exile (2KI 25)",
            "R-RETURN" => "the return (EZR 1-2)",
            "R-NATIVITY" => "the nativity flight (MAT 2)",
            "R-MINISTRY" => "the last journey to Jerusalem (LUK 9-19)",
            "R-PHILIP" => "Philip on the Gaza road (ACT 8)",
            "R-DAMASCUS" => "the Damascus road (ACT 9)",
            "R-PETER" => "Peter to Caesarea (ACT 9-10)",
            "R-EXODUS" => "the Exodus (NUM 33)",
            "R-PAUL1" => "Paul's first journey (ACT 13-14)",
            "R-PAUL2" => "Paul's second journey (ACT 15-18)",
            "R-PAUL3" => "Paul's third journey (ACT 18-21)",
            "R-ROME" => "the voyage to Rome (ACT 27-28)",
            _ => "a journey",
        }
    };
    ROUTES
        .iter()
        .map(|r| AuthoredRoute {
            tag: r.tag,
            display: display(r.tag),
            book: r.book,
            chapter_from: r.chapter_from,
            chapter_to: r.chapter_to,
            from_year: r.from_year,
            to_year: r.to_year,
            stations: r.stations.iter().map(|w| (w.name, w.lat, w.lon)).collect(),
        })
        .collect()
}
