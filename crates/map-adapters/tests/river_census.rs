use std::path::Path;
use std::process::Command;

use map_adapters::hydro::{read_rivers, RiverNumber};
use serde::Deserialize;

#[test]
fn the_published_census_is_generated_from_the_golden_requirements() {
    let output = Command::new("python3")
        .args(["tools/river_census.py", "--check"])
        .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(0),
        "the census covers the entire golden-authority inventory through its sole generator"
    );
}

#[test]
fn every_selected_natural_earth_identity_resolves_without_a_name_substitution() {
    let census: Census = serde_json::from_str(include_str!(
        "../../../data/authored/river-requirements.json"
    ))
    .unwrap();
    let rivers = read_rivers(include_str!(
        "../../../data/natural-earth/ne_10m_rivers_lake_centerlines.geojson"
    ))
    .unwrap();
    for requirement in census.requirements {
        let expected: Vec<_> = requirement
            .sources
            .iter()
            .map(|source| (RiverNumber(source.number), source.name.clone()))
            .collect();
        let actual: Vec<_> = requirement
            .sources
            .iter()
            .flat_map(|source| {
                rivers
                    .iter()
                    .filter(move |river| {
                        river.number == RiverNumber(source.number)
                            && river.name.as_deref() == Some(source.name.as_str())
                    })
                    .map(|river| (river.number, river.name.clone().unwrap()))
            })
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        assert_eq!(
            actual, expected,
            "every selected source identity resolves exactly for {}",
            requirement.name
        );
        assert_eq!(requirement.outcome, if requirement.sources.is_empty() { Outcome::Missing } else { Outcome::Course }, "an unlocated requirement stays Missing and every Course declares its permitted source: {}", requirement.name);
    }
}

#[derive(Deserialize)]
struct Census {
    requirements: Vec<Requirement>,
}

#[derive(Deserialize)]
struct Requirement {
    name: String,
    outcome: Outcome,
    sources: Vec<Source>,
}

#[derive(Debug, Deserialize, PartialEq)]
enum Outcome {
    Course,
    Missing,
}

#[derive(Deserialize)]
struct Source {
    number: i64,
    name: String,
}
