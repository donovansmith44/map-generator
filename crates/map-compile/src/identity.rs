use std::collections::BTreeMap;

use map_canon::{
    EntityId, EntityKind, LayerKind, Registry, RegistryViolation, Unification, Witness, WitnessRef,
};
use serde_json::Value;

pub struct Identity {
    registry: Registry,
    declared: BTreeMap<EntityId, EntityKind>,
}

impl Identity {
    pub fn resolve<'a>(&'a self, minted: &'a EntityId) -> &'a EntityId {
        self.registry.resolve(minted)
    }

    pub fn registry(&self) -> &Registry {
        &self.registry
    }

    pub fn witness(
        &mut self,
        minted: &EntityId,
        name: &str,
        layer: LayerKind,
        witness: Witness,
        kind: &'static str,
    ) {
        let home = self.registry.resolve(minted).clone();
        let entity_kind = self
            .declared
            .get(minted)
            .or_else(|| self.declared.get(&home))
            .copied()
            .unwrap_or_else(|| implied_kind(layer, kind));
        self.registry.observe(
            minted.clone(),
            name,
            entity_kind,
            WitnessRef {
                minted_as: minted.clone(),
                witness,
                layer,
                kind,
            },
        );
    }

    pub fn check(&self) -> Result<(), String> {
        let found: Vec<String> = self
            .registry
            .validate()
            .into_iter()
            .map(|v| match v {
                RegistryViolation::DanglingCanonical(id) => {
                    format!("canonical id '{}' was never minted by any witness", id.0)
                }
                RegistryViolation::ChainedUnification { minted, via } => {
                    format!(
                        "'{}' resolves through '{}': unification must be one hop",
                        minted.0, via.0
                    )
                }
                RegistryViolation::KindConflict { entity, a, b } => {
                    format!("'{}' is witnessed as both {a:?} and {b:?}", entity.0)
                }
            })
            .collect();
        if found.is_empty() {
            Ok(())
        } else {
            Err(format!("registry: {}", found.join("; ")))
        }
    }
}

pub fn load_registry(json: &str, source: &str) -> Result<Identity, String> {
    let v: Value = serde_json::from_str(json).map_err(|e| format!("registry: bad json: {e}"))?;
    let document = v
        .as_object()
        .ok_or_else(|| "registry: document must be an object".to_string())?;
    let rows = match document.get("unifications") {
        None if document.is_empty() => &[][..],
        None => return Err("registry: document lacks 'unifications'".into()),
        Some(rows) => rows
            .as_array()
            .map(Vec::as_slice)
            .ok_or_else(|| "registry: 'unifications' must be an array".to_string())?,
    };
    let mut registry = Registry::default();
    let mut declared = BTreeMap::new();
    for row in rows {
        let field = |k: &str| -> Result<String, String> {
            row.get(k)
                .and_then(Value::as_str)
                .map(str::to_string)
                .ok_or_else(|| format!("registry: a unification lacks '{k}'"))
        };
        let (canonical, minted) = (EntityId(field("canonical")?), EntityId(field("minted")?));
        let kind = kind_of(&field("kind")?)?;
        let reason = field("reason")?;
        registry
            .declare(
                canonical.clone(),
                minted.clone(),
                Unification::Declared {
                    reason,
                    source: source.to_string(),
                },
            )
            .map_err(|e| format!("registry: {e}"))?;
        for id in [canonical, minted] {
            match declared.entry(id) {
                std::collections::btree_map::Entry::Vacant(entry) => {
                    entry.insert(kind);
                }
                std::collections::btree_map::Entry::Occupied(entry) if *entry.get() == kind => {}
                std::collections::btree_map::Entry::Occupied(entry) => {
                    return Err(format!(
                        "registry: '{}' is declared as both {:?} and {kind:?}",
                        entry.key().0,
                        entry.get()
                    ));
                }
            }
        }
    }
    let chains: Vec<String> = registry
        .validate()
        .into_iter()
        .filter_map(|v| match v {
            RegistryViolation::ChainedUnification { minted, via } => {
                Some(format!("{} resolves through {}", minted.0, via.0))
            }
            _ => None,
        })
        .collect();
    if !chains.is_empty() {
        return Err(format!(
            "registry: unification chains: {}",
            chains.join("; ")
        ));
    }
    Ok(Identity { registry, declared })
}

fn kind_of(s: &str) -> Result<EntityKind, String> {
    Ok(match s {
        "Polity" => EntityKind::Polity,
        "District" => EntityKind::District,
        "People" => EntityKind::People,
        "Allotment" => EntityKind::Allotment,
        "Promise" => EntityKind::Promise,
        "Vision" => EntityKind::Vision,
        "Waterbody" => EntityKind::Waterbody,
        "Terrain" => EntityKind::Terrain,
        "Place" => EntityKind::Place,
        "Route" => EntityKind::Route,
        other => return Err(format!("registry: unknown entity kind '{other}'")),
    })
}

fn implied_kind(layer: LayerKind, kind: &str) -> EntityKind {
    match (layer, kind) {
        (_, "way") => EntityKind::Route,
        (_, "point") | (_, "memory") => EntityKind::Place,
        (_, "line") => EntityKind::Waterbody,
        (LayerKind::Water, _) => EntityKind::Waterbody,
        (LayerKind::Relief, _) => EntityKind::Terrain,
        (LayerKind::ScriptureClaims, _) => EntityKind::Allotment,
        (LayerKind::Territory, _) | (LayerKind::Background, _) | (LayerKind::Journeys, _) => {
            EntityKind::Polity
        }
    }
}

#[cfg(test)]
mod laws {
    use super::*;
    use serde_json::json;

    const GENERATED_IDS: usize = 32;

    #[test]
    fn non_object_documents_are_refused() {
        for text in ["null", "false", "[]", "42", "\"registry\""] {
            let actual = load_registry(text, "test source").err();
            assert_eq!(
                actual,
                Some("registry: document must be an object".into()),
                "a scalar or array cannot become an empty registry"
            );
        }
    }

    #[test]
    fn a_nonempty_document_cannot_silently_omit_its_declaration_collection() {
        for text in [
            r#"{"unification":[]}"#,
            r#"{"_comment":"forgot the collection"}"#,
        ] {
            let actual = load_registry(text, "test source").err();
            assert_eq!(
                actual,
                Some("registry: document lacks 'unifications'".into()),
                "a misspelled or omitted collection cannot hide declarations"
            );
        }
    }

    #[test]
    fn malformed_declaration_collections_are_refused() {
        for rows in [json!(null), json!(false), json!("alias"), json!({})] {
            let text = json!({"unifications": rows}).to_string();
            let actual = load_registry(&text, "test source").err();
            assert_eq!(
                actual,
                Some("registry: 'unifications' must be an array".into()),
                "malformed rows cannot become an empty registry"
            );
        }
    }

    #[test]
    fn duplicate_generated_aliases_are_refused_before_observation() {
        for index in 0..GENERATED_IDS {
            let minted = format!("alias:{index}");
            let row = json!({"canonical":"home", "minted":minted, "kind":"Place", "reason":"written decision"});
            let text = json!({"unifications":[row.clone(),row]}).to_string();
            let actual = load_registry(&text, "test source").err();
            assert_eq!(
                actual,
                Some(format!(
                    "registry: {minted:?} is already declared; refusing a duplicate unification"
                )),
                "a duplicate id cannot overwrite its declaration"
            );
        }
    }

    #[test]
    fn shared_homes_cannot_overwrite_a_conflicting_declared_kind() {
        for (first, second) in [
            ("Polity", "Place"),
            ("District", "People"),
            ("Allotment", "Promise"),
        ] {
            let text = json!({"unifications":[
                {"canonical":"home", "minted":"a", "kind":first, "reason":"first decision"},
                {"canonical":"home", "minted":"b", "kind":second, "reason":"second decision"}
            ]})
            .to_string();
            let actual = load_registry(&text, "test source").err();
            assert_eq!(
                actual,
                Some(format!(
                    "registry: 'home' is declared as both {first} and {second}"
                )),
                "every declaration for one home must agree about its kind"
            );
        }
    }

    #[test]
    fn generated_aliases_can_share_one_home_when_their_kinds_agree() {
        for kind in [
            "Polity",
            "District",
            "People",
            "Allotment",
            "Promise",
            "Vision",
            "Waterbody",
            "Terrain",
            "Place",
            "Route",
        ] {
            let rows: Vec<_> = (0..GENERATED_IDS).map(|index| json!({
                "canonical":"home", "minted":format!("alias:{index}"), "kind":kind, "reason":"written decision"
            })).collect();
            let text = json!({"unifications":rows}).to_string();
            let identity = load_registry(&text, "test source").expect("consistent declarations");
            let actual: Vec<_> = (0..GENERATED_IDS)
                .map(|index| {
                    identity
                        .resolve(&EntityId(format!("alias:{index}")))
                        .clone()
                })
                .collect();
            assert_eq!(
                actual,
                vec![EntityId("home".into()); GENERATED_IDS],
                "agreeing declarations retain the one explicit home"
            );
        }
    }

    #[test]
    fn the_pinned_atlas_identity_inventory_has_no_registry_moves() {
        let identity = load_registry(
            include_str!("../../../data/authored/registry.json"),
            "data/authored/registry.json",
        )
        .expect("authored registry");
        let polities =
            crate::vendor::parse_polities(include_str!("../../../data/atlas-vendor/polities.json"))
                .expect("pinned polity export");
        let narratives = crate::vendor::parse_narratives(include_str!(
            "../../../data/atlas-vendor/narratives.json"
        ))
        .expect("pinned narrative export");
        let atlas = map_adapters::load_exports(
            include_str!("../../../data/atlas-exports/gazetteer.json"),
            include_str!("../../../data/atlas-exports/chronology.json"),
        )
        .expect("pinned atlas exports");
        let ids: std::collections::BTreeSet<_> = polities
            .into_iter()
            .map(|row| EntityId(row.id))
            .chain(narratives.into_iter().map(|row| EntityId(row.id)))
            .chain(
                atlas
                    .gazetteer
                    .places
                    .keys()
                    .map(|id| EntityId(format!("place:{}", id.0))),
            )
            .collect();
        let actual: Vec<_> = ids
            .iter()
            .filter_map(|minted| {
                let home = identity.resolve(minted);
                (home != minted).then(|| (minted.clone(), home.clone()))
            })
            .collect();
        assert_eq!(
            actual,
            vec![],
            "the complete pinned atlas polity, narrative and place ids keep their homes"
        );
    }

    #[test]
    fn the_authored_registry_keeps_every_inherited_alias_and_reason() {
        let text = include_str!("../../../data/authored/registry.json");
        let mut identity =
            load_registry(text, "data/authored/registry.json").expect("authored registry");
        let rows: Value = serde_json::from_str(text).expect("authored JSON");
        for row in rows["unifications"].as_array().expect("declarations") {
            let minted = EntityId(row["minted"].as_str().expect("minted id").into());
            let home = EntityId(row["canonical"].as_str().expect("home id").into());
            identity.witness(
                &home,
                "canonical witness",
                LayerKind::Territory,
                Witness::Atlas,
                "area",
            );
            identity.witness(
                &minted,
                "alias witness",
                LayerKind::Background,
                Witness::Basemap,
                "area",
            );
            assert_eq!(
                identity.resolve(&minted),
                &home,
                "each authored alias keeps its written home"
            );
            assert_eq!(
                identity.registry().why(&minted),
                Some(&Unification::Declared {
                    reason: row["reason"].as_str().expect("written reason").into(),
                    source: "data/authored/registry.json".into(),
                }),
                "every identity decision retains its original justification"
            );
        }
        assert_eq!(
            identity.registry().validate(),
            vec![],
            "all authored homes validate when witnessed"
        );
        assert_eq!(
            identity.check(),
            Ok(()),
            "the compiler accepts a sound authored registry"
        );
    }
}
