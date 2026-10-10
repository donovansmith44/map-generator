use proptest::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[test]
fn every_waypoint_has_typed_relations_with_resolvable_targets() {
    let evidence = evidence();
    for survey in evidence["survey"].as_array().expect("surveys exist") {
        let names: Vec<_> = survey["sequence"]
            .as_array()
            .expect("sequences exist")
            .iter()
            .flat_map(|sequence| sequence["waypoints"].as_array().expect("waypoints exist"))
            .map(|waypoint| waypoint["name"].as_str().expect("reference name exists"))
            .collect();
        for sequence in survey["sequence"].as_array().expect("sequences exist") {
            for row in sequence["waypoints"].as_array().expect("waypoints exist") {
                let waypoint: Waypoint = serde_json::from_value(row.clone())
                    .expect("each waypoint has the closed relation vocabulary");
                assert!(
                    !waypoint.relations.is_empty(),
                    "every reference carries its cited relation"
                );
                for relation in &waypoint.relations {
                    for target in relation.references() {
                        assert!(
                            names.contains(&target),
                            "relation targets resolve to recorded references in the same survey"
                        );
                    }
                }
                assert_eq!(
                    serde_json::to_value(waypoint).expect("typed evidence serializes"),
                    *row,
                    "typed evidence retains the entire waypoint without discarded fields"
                );
            }
        }
    }
}

#[test]
fn scripture_preserves_subject_target_and_destination_as_distinct_facts() {
    let evidence = evidence();
    for expected in expectations() {
        assert_eq!(
            waypoint(&evidence, expected.lot, expected.side, expected.name),
            expected.row,
            "the complete cited waypoint retains Scripture's grammatical attachment"
        );
    }
}

#[test]
fn every_relative_and_neighbour_clause_matches_its_complete_scripture_outcome() {
    assert_eq!(relative_outcomes(&evidence()), expected_relative_outcomes(), "the entire relative-reference and neighbour inventory preserves the cited subjects, targets and directions");
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 128, failure_persistence: None, .. ProptestConfig::default() })]
    #[test]
    fn valid_reference_and_direction_mutations_cannot_pass_the_scripture_controls(
        index in 0usize..5, mutation in prop_oneof![Just(Mutation::Subject), Just(Mutation::Target), Just(Mutation::Direction)],
    ) {
        let evidence = evidence();
        let expectations = expectations();
        let expected = &expectations[index];
        let row = waypoint(&evidence, expected.lot, expected.side, expected.name);
        prop_assert_eq!(&row, &expected.row, "the unmodified cited waypoint matches the complete Scripture control");
        let mut changed = row.clone();
        match mutation {
            Mutation::Subject => {
                changed["relations"][1]["subject"] = if row["relations"][1]["subject"]["kind"] == "border" {
                    json!({"kind":"reference", "reference":expected.name, "part":"site"})
                } else { json!({"kind":"border"}) };
            },
            Mutation::Target => changed["relations"][1]["target"] = json!({"reference":expected.name, "part":"site"}),
            Mutation::Direction => changed["relations"][1]["predicate"] = json!("north_of"),
        }
        let typed: Waypoint = serde_json::from_value(changed).expect("mutated references use the same closed vocabulary");
        prop_assert_eq!(&typed.verse, row["verse"].as_str().expect("verse exists"), "attachment mutations retain the valid Scripture citation");
        prop_assert_eq!(serde_json::to_value(&typed.site).expect("site serializes"), row["site"].clone(), "attachment mutations retain the valid place identity");
        prop_assert_ne!(serde_json::to_value(typed).expect("waypoint serializes"), expected.row.clone(), "changed subject, target or direction fails the whole Scripture outcome");
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 128, failure_persistence: None, .. ProptestConfig::default() })]
    #[test]
    fn every_relative_family_detects_changed_attachment_with_a_valid_citation(
        expected in proptest::sample::select(expected_relative_outcomes()),
        mutation in prop_oneof![Just(Mutation::Subject), Just(Mutation::Target), Just(Mutation::Direction)],
    ) {
        let evidence = evidence();
        let actual = relative_outcomes(&evidence);
        prop_assert!(actual.contains(&expected), "the original complete relation is retained in the Scripture inventory");
        let mut changed = expected.clone();
        let relation = &mut changed["relation"];
        match mutation {
            Mutation::Subject => {
                if relation["kind"] == "neighbour" {
                    relation["subject"] = if relation["subject"] == "naphtali" { json!("manasseh_west") } else { json!("naphtali") };
                } else {
                    relation["subject"] = if relation["subject"]["kind"] == "border" {
                        json!({"kind":"reference", "reference":relation["target"]["reference"], "part":"site"})
                    } else { json!({"kind":"border"}) };
                }
            }
            Mutation::Target => {
                relation["target"]["reference"] = json!(alternate_reference(&evidence, expected["lot"].as_str().expect("lot exists"), relation["target"]["reference"].as_str().expect("target exists")));
            }
            Mutation::Direction => {
                if relation["kind"] == "neighbour" {
                    relation["side"] = if relation["side"] == "north" { json!("south") } else { json!("north") };
                } else {
                    relation["predicate"] = if relation["predicate"] == "north_of" { json!("south_of") } else { json!("north_of") };
                }
            }
        }
        let _: Relation = serde_json::from_value(changed["relation"].clone()).expect("mutated relation retains a valid typed shape");
        prop_assert_eq!(&changed["verse"], &expected["verse"], "relation mutations keep the original valid Scripture citation");
        prop_assert_ne!(changed, expected, "the complete relation outcome exposes changed subjects, valid targets and directions");
    }
}

fn relative_outcomes(evidence: &Value) -> Vec<Value> {
    let mut outcomes = Vec::new();
    for survey in evidence["survey"].as_array().expect("surveys exist") {
        for sequence in survey["sequence"].as_array().expect("sequences exist") {
            for waypoint in sequence["waypoints"].as_array().expect("references exist") {
                for relation in waypoint["relations"]
                    .as_array()
                    .expect("typed relations exist")
                {
                    if relation["kind"] == "position" || relation["kind"] == "neighbour" {
                        outcomes.push(json!({"lot":survey["lot"], "side":sequence["side"], "reference":waypoint["name"], "verse":waypoint["verse"], "relation":relation}));
                    }
                }
            }
        }
    }
    outcomes.sort_by_key(Value::to_string);
    outcomes
}

fn alternate_reference(evidence: &Value, lot: &str, target: &str) -> String {
    evidence["survey"]
        .as_array()
        .expect("surveys exist")
        .iter()
        .find(|survey| survey["lot"] == lot)
        .expect("survey exists")["sequence"]
        .as_array()
        .expect("sequences exist")
        .iter()
        .flat_map(|sequence| sequence["waypoints"].as_array().expect("references exist"))
        .map(|waypoint| waypoint["name"].as_str().expect("name exists"))
        .find(|name| *name != target)
        .expect("each survey has another valid named reference")
        .to_owned()
}

fn expected_relative_outcomes() -> Vec<Value> {
    let mut outcomes = json!([
        {"lot":"benjamin","side":"south","reference":"Arabah","verse":"JOS.18.18","relation":{"kind":"position","subject":{"kind":"border"},"predicate":"over_against","target":{"reference":"Arabah","part":"north_side"}}},
        {"lot":"benjamin","side":"north","reference":"Ataroth-adar","verse":"JOS.18.13","relation":{"kind":"position","subject":{"kind":"reference","reference":"Ataroth-adar","part":"site"},"predicate":"near","target":{"reference":"hill south of nether Beth-horon","part":"site"}}},
        {"lot":"benjamin","side":"south","reference":"Beth-hoglah","verse":"JOS.18.19","relation":{"kind":"position","subject":{"kind":"border"},"predicate":"north_of","target":{"reference":"Beth-hoglah","part":"site"}}},
        {"lot":"benjamin","side":"south","reference":"Geliloth","verse":"JOS.18.17","relation":{"kind":"position","subject":{"kind":"reference","reference":"Geliloth","part":"site"},"predicate":"over_against","target":{"reference":"Adummim","part":"ascent"}}},
        {"lot":"benjamin","side":"south","reference":"Jebusi Jerusalem","verse":"JOS.18.16","relation":{"kind":"position","subject":{"kind":"border"},"predicate":"south_of","target":{"reference":"Jebusi Jerusalem","part":"site"}}},
        {"lot":"benjamin","side":"north","reference":"Jericho","verse":"JOS.18.12","relation":{"kind":"position","subject":{"kind":"border"},"predicate":"north_of","target":{"reference":"Jericho","part":"site"}}},
        {"lot":"benjamin","side":"north","reference":"Luz which is Bethel","verse":"JOS.18.13","relation":{"kind":"position","subject":{"kind":"border"},"predicate":"south_of","target":{"reference":"Luz which is Bethel","part":"site"}}},
        {"lot":"benjamin","side":"south","reference":"end of mountain before Hinnom","verse":"JOS.18.16","relation":{"kind":"position","subject":{"kind":"reference","reference":"end of mountain before Hinnom","part":"site"},"predicate":"before","target":{"reference":"valley of Hinnom","part":"site"}}},
        {"lot":"benjamin","side":"south","reference":"end of mountain before Hinnom","verse":"JOS.18.16","relation":{"kind":"position","subject":{"kind":"reference","reference":"end of mountain before Hinnom","part":"site"},"predicate":"within","target":{"reference":"valley of giants","part":"north_side"}}},
        {"lot":"benjamin","side":"west","reference":"hill before Beth-horon","verse":"JOS.18.14","relation":{"kind":"position","subject":{"kind":"reference","reference":"hill before Beth-horon","part":"site"},"predicate":"before","target":{"reference":"nether Beth-horon","part":"site"}}},
        {"lot":"benjamin","side":"west","reference":"hill before Beth-horon","verse":"JOS.18.14","relation":{"kind":"position","subject":{"kind":"reference","reference":"hill before Beth-horon","part":"site"},"predicate":"south_of","target":{"reference":"nether Beth-horon","part":"site"}}},
        {"lot":"benjamin","side":"north","reference":"hill south of nether Beth-horon","verse":"JOS.18.13","relation":{"kind":"position","subject":{"kind":"reference","reference":"hill south of nether Beth-horon","part":"site"},"predicate":"south_of","target":{"reference":"nether Beth-horon","part":"site"}}},
        {"lot":"benjamin","side":"south","reference":"north bay of the Salt Sea","verse":"JOS.18.19","relation":{"kind":"position","subject":{"kind":"reference","reference":"north bay of the Salt Sea","part":"site"},"predicate":"at","target":{"reference":"south end of Jordan","part":"end"}}},
        {"lot":"dan","side":"west","reference":"Japho","verse":"JOS.19.46","relation":{"kind":"position","subject":{"kind":"border"},"predicate":"before","target":{"reference":"Japho","part":"site"}}},
        {"lot":"ephraim","side":"north","reference":"Janohah","verse":"JOS.16.6","relation":{"kind":"position","subject":{"kind":"border"},"predicate":"east_of","target":{"reference":"Taanath-shiloh","part":"site"}}},
        {"lot":"gad","side":"east","reference":"Aroer before Rabbah","verse":"JOS.13.25","relation":{"kind":"position","subject":{"kind":"reference","reference":"Aroer before Rabbah","part":"site"},"predicate":"before","target":{"reference":"Rabbah","part":"site"}}},
        {"lot":"gad","side":"west","reference":"sea of Chinnereth","verse":"JOS.13.27","relation":{"kind":"position","subject":{"kind":"border"},"predicate":"east_of","target":{"reference":"Jordan","part":"site"}}},
        {"lot":"judah","side":"north","reference":"Adummim","verse":"JOS.15.7","relation":{"kind":"position","subject":{"kind":"reference","reference":"Gilgal","part":"site"},"predicate":"before","target":{"reference":"Adummim","part":"ascent"}}},
        {"lot":"judah","side":"north","reference":"Adummim","verse":"JOS.15.7","relation":{"kind":"position","subject":{"kind":"reference","reference":"Adummim","part":"ascent"},"predicate":"south_of","target":{"reference":"river south of Adummim","part":"site"}}},
        {"lot":"judah","side":"north","reference":"Beth-arabah","verse":"JOS.15.6","relation":{"kind":"position","subject":{"kind":"border"},"predicate":"north_of","target":{"reference":"Beth-arabah","part":"site"}}},
        {"lot":"judah","side":"north","reference":"Chesalon","verse":"JOS.15.10","relation":{"kind":"position","subject":{"kind":"reference","reference":"Chesalon","part":"site"},"predicate":"same_as","target":{"reference":"mount Jearim","part":"site"}}},
        {"lot":"judah","side":"north","reference":"Ekron","verse":"JOS.15.11","relation":{"kind":"position","subject":{"kind":"border"},"predicate":"north_of","target":{"reference":"Ekron","part":"site"}}},
        {"lot":"judah","side":"north","reference":"Jebusite Jerusalem","verse":"JOS.15.8","relation":{"kind":"position","subject":{"kind":"border"},"predicate":"south_of","target":{"reference":"Jebusite Jerusalem","part":"site"}}},
        {"lot":"judah","side":"north","reference":"hilltop west of Hinnom","verse":"JOS.15.8","relation":{"kind":"position","subject":{"kind":"reference","reference":"hilltop west of Hinnom","part":"site"},"predicate":"at","target":{"reference":"valley of giants","part":"north_end"}}},
        {"lot":"judah","side":"north","reference":"hilltop west of Hinnom","verse":"JOS.15.8","relation":{"kind":"position","subject":{"kind":"reference","reference":"hilltop west of Hinnom","part":"site"},"predicate":"before","target":{"reference":"valley of Hinnom","part":"site"}}},
        {"lot":"judah","side":"north","reference":"hilltop west of Hinnom","verse":"JOS.15.8","relation":{"kind":"position","subject":{"kind":"reference","reference":"hilltop west of Hinnom","part":"site"},"predicate":"west_of","target":{"reference":"valley of Hinnom","part":"site"}}},
        {"lot":"manasseh_east","side":"interior","reference":"towns of Jair","verse":"JOS.13.30","relation":{"kind":"position","subject":{"kind":"reference","reference":"towns of Jair","part":"site"},"predicate":"within","target":{"reference":"Bashan","part":"site"}}},
        {"lot":"manasseh_west","side":"northeast","reference":"Asher","verse":"JOS.17.10","relation":{"kind":"neighbour","subject":"manasseh_west","side":"north","target":{"reference":"Asher","part":"site"}}},
        {"lot":"manasseh_west","side":"northeast","reference":"Issachar","verse":"JOS.17.10","relation":{"kind":"neighbour","subject":"manasseh_west","side":"east","target":{"reference":"Issachar","part":"site"}}},
        {"lot":"manasseh_west","side":"south","reference":"Michmethah","verse":"JOS.17.7","relation":{"kind":"position","subject":{"kind":"reference","reference":"Michmethah","part":"site"},"predicate":"before","target":{"reference":"Shechem","part":"site"}}},
        {"lot":"manasseh_west","side":"south","reference":"river Kanah","verse":"JOS.17.9","relation":{"kind":"position","subject":{"kind":"border"},"predicate":"north_of","target":{"reference":"river Kanah","part":"site"}}},
        {"lot":"naphtali","side":"surrounding","reference":"Asher","verse":"JOS.19.34","relation":{"kind":"neighbour","subject":"naphtali","side":"west","target":{"reference":"Asher","part":"site"}}},
        {"lot":"naphtali","side":"surrounding","reference":"Judah upon Jordan","verse":"JOS.19.34","relation":{"kind":"neighbour","subject":"naphtali","side":"east","target":{"reference":"Judah upon Jordan","part":"site"}}},
        {"lot":"naphtali","side":"surrounding","reference":"Zebulun","verse":"JOS.19.34","relation":{"kind":"neighbour","subject":"naphtali","side":"south","target":{"reference":"Zebulun","part":"site"}}},
        {"lot":"promised_land","side":"south","reference":"Kadesh-barnea","verse":"NUM.34.4","relation":{"kind":"position","subject":{"kind":"border"},"predicate":"south_of","target":{"reference":"Kadesh-barnea","part":"site"}}},
        {"lot":"promised_land","side":"east","reference":"Riblah","verse":"NUM.34.11","relation":{"kind":"position","subject":{"kind":"reference","reference":"Riblah","part":"site"},"predicate":"east_of","target":{"reference":"Ain","part":"site"}}},
        {"lot":"reuben","side":"south","reference":"Aroer by the Arnon","verse":"JOS.13.16","relation":{"kind":"position","subject":{"kind":"reference","reference":"Aroer by the Arnon","part":"site"},"predicate":"at","target":{"reference":"Arnon","part":"bank"}}},
        {"lot":"reuben","side":"south","reference":"city in the midst of the river","verse":"JOS.13.16","relation":{"kind":"position","subject":{"kind":"reference","reference":"city in the midst of the river","part":"site"},"predicate":"within","target":{"reference":"Arnon","part":"site"}}},
        {"lot":"zebulun","side":"west","reference":"river before Jokneam","verse":"JOS.19.11","relation":{"kind":"position","subject":{"kind":"reference","reference":"river before Jokneam","part":"site"},"predicate":"before","target":{"reference":"Jokneam","part":"site"}}}
    ]).as_array().expect("the full expected inventory is an array").clone();
    outcomes.sort_by_key(Value::to_string);
    outcomes
}

fn expectations() -> Vec<Expectation> {
    vec![
        Expectation {
            lot: "ephraim",
            side: "north",
            name: "Janohah",
            row: json!({
                "name":"Janohah", "verse":"JOS.16.6", "site":{"atlas":"janoah-1"}, "relations":[
                    {"kind":"travel", "verb":"unto", "direction":"unstated", "target":{"reference":"Janohah", "part":"site"}},
                    {"kind":"position", "subject":{"kind":"border"}, "predicate":"east_of", "target":{"reference":"Taanath-shiloh", "part":"site"}}
                ]
            }),
        },
        Expectation {
            lot: "judah",
            side: "north",
            name: "Adummim",
            row: json!({
                "name":"Adummim", "verse":"JOS.15.7", "site":{"atlas":"adummim"}, "relations":[
                    {"kind":"position", "subject":{"kind":"reference", "reference":"Adummim", "part":"ascent"}, "predicate":"south_of", "target":{"reference":"river south of Adummim", "part":"site"}},
                    {"kind":"position", "subject":{"kind":"reference", "reference":"Gilgal", "part":"site"}, "predicate":"before", "target":{"reference":"Adummim", "part":"ascent"}}
                ]
            }),
        },
        Expectation {
            lot: "benjamin",
            side: "south",
            name: "Geliloth",
            row: json!({
                "name":"Geliloth", "verse":"JOS.18.17", "site":{"atlas":"geliloth"}, "relations":[
                    {"kind":"travel", "verb":"toward", "direction":"unstated", "target":{"reference":"Geliloth", "part":"site"}},
                    {"kind":"position", "subject":{"kind":"reference", "reference":"Geliloth", "part":"site"}, "predicate":"over_against", "target":{"reference":"Adummim", "part":"ascent"}}
                ]
            }),
        },
        Expectation {
            lot: "judah",
            side: "north",
            name: "hilltop west of Hinnom",
            row: json!({
                "name":"hilltop west of Hinnom", "verse":"JOS.15.8", "site":{"unlocated":"hilltop west of Hinnom"}, "relations":[
                    {"kind":"travel", "verb":"unto", "direction":"unstated", "target":{"reference":"hilltop west of Hinnom", "part":"top"}},
                    {"kind":"position", "subject":{"kind":"reference", "reference":"hilltop west of Hinnom", "part":"site"}, "predicate":"before", "target":{"reference":"valley of Hinnom", "part":"site"}},
                    {"kind":"position", "subject":{"kind":"reference", "reference":"hilltop west of Hinnom", "part":"site"}, "predicate":"west_of", "target":{"reference":"valley of Hinnom", "part":"site"}},
                    {"kind":"position", "subject":{"kind":"reference", "reference":"hilltop west of Hinnom", "part":"site"}, "predicate":"at", "target":{"reference":"valley of giants", "part":"north_end"}}
                ]
            }),
        },
        Expectation {
            lot: "benjamin",
            side: "south",
            name: "end of mountain before Hinnom",
            row: json!({
                "name":"end of mountain before Hinnom", "verse":"JOS.18.16", "site":{"unlocated":"end of mountain before Hinnom"}, "relations":[
                    {"kind":"travel", "verb":"unto", "direction":"unstated", "target":{"reference":"end of mountain before Hinnom", "part":"end"}},
                    {"kind":"position", "subject":{"kind":"reference", "reference":"end of mountain before Hinnom", "part":"site"}, "predicate":"before", "target":{"reference":"valley of Hinnom", "part":"site"}},
                    {"kind":"position", "subject":{"kind":"reference", "reference":"end of mountain before Hinnom", "part":"site"}, "predicate":"within", "target":{"reference":"valley of giants", "part":"north_side"}}
                ]
            }),
        },
    ]
}

fn evidence() -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/authored/surveys/allotments.toml");
    let output = std::process::Command::new("python3")
        .args([
            "-c",
            "import json,sys,tomli; json.dump(tomli.loads(open(sys.argv[1]).read()),sys.stdout)",
        ])
        .arg(path)
        .output()
        .expect("the existing test-only TOMLI bridge runs");
    assert!(
        output.status.success(),
        "the Scripture evidence parses with the maintained TOML library"
    );
    serde_json::from_slice(&output.stdout).expect("parsed TOML is JSON serializable")
}

fn waypoint(evidence: &Value, lot: &str, side: &str, name: &str) -> Value {
    evidence["survey"]
        .as_array()
        .expect("surveys exist")
        .iter()
        .find(|row| row["lot"] == lot)
        .expect("survey exists")["sequence"]
        .as_array()
        .expect("sequences exist")
        .iter()
        .find(|row| row["side"] == side)
        .expect("sequence exists")["waypoints"]
        .as_array()
        .expect("references exist")
        .iter()
        .find(|row| row["name"] == name)
        .expect("reference exists")
        .clone()
}

struct Expectation {
    lot: &'static str,
    side: &'static str,
    name: &'static str,
    row: Value,
}

#[derive(Clone, Debug)]
enum Mutation {
    Subject,
    Target,
    Direction,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Waypoint {
    name: String,
    verse: String,
    site: Site,
    relations: Vec<Relation>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum Site {
    Atlas(String),
    Unlocated(String),
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Relation {
    Travel {
        verb: Travel,
        direction: Direction,
        target: Target,
    },
    Position {
        subject: Subject,
        predicate: Position,
        target: Target,
    },
    Neighbour {
        subject: Lot,
        side: Side,
        target: Target,
    },
    Extent {
        verb: Extent,
        target: Target,
    },
    Listed {
        target: Target,
    },
    Boundary {
        target: Target,
    },
    Reference {
        target: Target,
    },
}

impl Relation {
    fn references(&self) -> Vec<&str> {
        match self {
            Self::Position {
                subject: Subject::Reference { reference, .. },
                target,
                ..
            } => vec![reference, &target.reference],
            Self::Travel { target, .. }
            | Self::Position { target, .. }
            | Self::Neighbour { target, .. }
            | Self::Extent { target, .. }
            | Self::Listed { target }
            | Self::Boundary { target }
            | Self::Reference { target } => vec![&target.reference],
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Target {
    reference: String,
    part: Part,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Subject {
    Border,
    Reference { reference: String, part: Part },
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum Part {
    Site,
    Border,
    Ascent,
    Bank,
    Edge,
    End,
    Top,
    Cities,
    NorthSide,
    SouthSide,
    EastSide,
    NorthEnd,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum Travel {
    From,
    Unto,
    Along,
    Toward,
    Outgoing,
    At,
    Near,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum Position {
    EastOf,
    WestOf,
    NorthOf,
    SouthOf,
    Before,
    OverAgainst,
    Within,
    At,
    Near,
    SameAs,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum Direction {
    Unstated,
    North,
    South,
    East,
    West,
    SeaWard,
    RightHand,
    LeftHand,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum Side {
    North,
    South,
    East,
    West,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum Extent {
    From,
    Includes,
    Half,
    Unto,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum Lot {
    ManassehWest,
    Naphtali,
}
