use proptest::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;

mod survey_relations {
    pub(super) mod scripture;
}

#[test]
fn every_waypoint_has_typed_relations_with_resolvable_targets() {
    let evidence = evidence();
    for survey in evidence["survey"].as_array().expect("surveys exist") {
        let rows: Vec<Waypoint> = survey["sequence"]
            .as_array()
            .expect("sequences exist")
            .iter()
            .flat_map(|sequence| sequence["waypoints"].as_array().expect("waypoints exist"))
            .map(|row| {
                serde_json::from_value(row.clone())
                    .expect("each waypoint has the closed relation vocabulary")
            })
            .collect();
        for waypoint in &rows {
            assert!(
                !waypoint.relations.is_empty(),
                "every reference carries its cited relation"
            );
            for relation in &waypoint.relations {
                for target in relation.references() {
                    assert!(
                        rows.iter().any(|row| row.name == target),
                        "relation targets resolve to recorded references in the same survey"
                    );
                }
            }
            if let Site::Unlocated(identity) = &waypoint.site {
                assert_eq!(
                    identity, &waypoint.name,
                    "an unlocated identity matches its recorded reference"
                );
            }
        }
        let serialized: Vec<Value> = rows
            .into_iter()
            .map(|row| serde_json::to_value(row).expect("typed evidence serializes"))
            .collect();
        let original: Vec<Value> = survey["sequence"]
            .as_array()
            .expect("sequences exist")
            .iter()
            .flat_map(|sequence| {
                sequence["waypoints"]
                    .as_array()
                    .expect("waypoints exist")
                    .iter()
                    .cloned()
            })
            .collect();
        assert_eq!(
            serialized, original,
            "typed evidence retains every complete waypoint without discarded fields"
        );
    }
}

#[test]
fn every_survey_relation_matches_the_hand_checked_scripture_table() {
    assert_eq!(outcomes(&evidence()), survey_relations::scripture::outcomes(), "all survey relations preserve the independently read subjects, targets, feature parts and cited verses");
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 128, failure_persistence: None, .. ProptestConfig::default() })]
    #[test]
    fn every_relative_family_detects_changed_attachment_with_a_valid_citation(
        expected in proptest::sample::select(relative_expectations()),
        mutation in prop_oneof![Just(Mutation::Subject), Just(Mutation::Target), Just(Mutation::Direction)],
    ) {
        let mut actual = outcomes(&evidence());
        let index = actual.iter().position(|row| row == &expected).expect("the independent cited outcome occurs in the survey");
        prop_assert_eq!(&actual, &survey_relations::scripture::outcomes(), "the complete unmodified survey agrees with the Scripture table");
        let replacement = actual.iter().find(|row| row.lot == expected.lot && row.relation.target() != expected.relation.target()).expect("each lot records another target").relation.target().clone();
        let changed = &mut actual[index];
        match (&mut changed.relation, mutation) {
            (Relation::Position { subject, .. }, Mutation::Subject) => *subject = match subject {
                Subject::Border => Subject::Reference { reference: replacement.reference.clone(), part: replacement.part.clone() },
                Subject::Reference { .. } => Subject::Border,
            },
            (Relation::Neighbour { subject, .. }, Mutation::Subject) => *subject = if *subject == Lot::Naphtali { Lot::ManassehWest } else { Lot::Naphtali },
            (Relation::Position { target, .. } | Relation::Neighbour { target, .. }, Mutation::Target) => *target = replacement,
            (Relation::Position { predicate, .. }, Mutation::Direction) => *predicate = if *predicate == Position::NorthOf { Position::SouthOf } else { Position::NorthOf },
            (Relation::Neighbour { side, .. }, Mutation::Direction) => *side = if *side == Side::North { Side::South } else { Side::North },
            _ => unreachable!("the generated family contains only positional and neighbour facts"),
        }
        prop_assert_eq!(&changed.verse, &expected.verse, "attachment changes preserve the valid cited verse");
        actual.sort();
        prop_assert_ne!(actual, survey_relations::scripture::outcomes(), "the whole independent Scripture outcome refuses changed subjects, recorded targets and directions");
    }
}

fn outcomes(evidence: &Value) -> Vec<CitedRelation> {
    let mut outcomes = Vec::new();
    for survey in evidence["survey"].as_array().expect("surveys exist") {
        let lot: Lot = serde_json::from_value(survey["lot"].clone())
            .expect("the lot is in the closed vocabulary");
        for sequence in survey["sequence"].as_array().expect("sequences exist") {
            for row in sequence["waypoints"].as_array().expect("references exist") {
                let waypoint: Waypoint =
                    serde_json::from_value(row.clone()).expect("the complete waypoint is typed");
                for relation in waypoint.relations {
                    outcomes.push(CitedRelation {
                        lot: lot.clone(),
                        verse: waypoint.verse.clone(),
                        relation,
                    });
                }
            }
        }
    }
    outcomes.sort();
    outcomes
}

fn relative_expectations() -> Vec<CitedRelation> {
    survey_relations::scripture::outcomes()
        .into_iter()
        .filter(|row| {
            matches!(
                row.relation,
                Relation::Position { .. } | Relation::Neighbour { .. }
            )
        })
        .collect()
}

#[derive(Clone, Debug)]
enum Mutation {
    Subject,
    Target,
    Direction,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct CitedRelation {
    lot: Lot,
    verse: String,
    relation: Relation,
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

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Waypoint {
    name: String,
    verse: String,
    site: Site,
    relations: Vec<Relation>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum Site {
    Atlas(String),
    Unlocated(String),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
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
    fn target(&self) -> &Target {
        match self {
            Self::Travel { target, .. }
            | Self::Position { target, .. }
            | Self::Neighbour { target, .. }
            | Self::Extent { target, .. }
            | Self::Listed { target }
            | Self::Boundary { target }
            | Self::Reference { target } => target,
        }
    }

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

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Target {
    reference: String,
    part: Part,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Subject {
    Border,
    Reference { reference: String, part: Part },
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
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

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
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

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
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

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
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

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum Side {
    North,
    South,
    East,
    West,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum Extent {
    From,
    Includes,
    Half,
    Unto,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum Lot {
    PromisedLand,
    Reuben,
    Gad,
    ManassehEast,
    Judah,
    Ephraim,
    ManassehWest,
    Benjamin,
    Simeon,
    Zebulun,
    Issachar,
    Asher,
    Naphtali,
    Dan,
}
