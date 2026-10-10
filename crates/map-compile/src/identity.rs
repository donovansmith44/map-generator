use std::collections::BTreeMap;
use std::fmt;
use std::marker::PhantomData;

use map_canon::{
    DeclarationRefusal, EntityId, EntityKind, GeometryKind, LayerKind, Registry, RegistryViolation,
    Unification, Witness, WitnessRef,
};
use serde::{Deserialize, Deserializer};

pub struct Identity {
    registry: Registry,
    declared: BTreeMap<EntityId, EntityKind>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RegistryLoadRefusal {
    InvalidDocument,
    InvalidSyntax,
    IncompleteDocument,
    MissingDeclarations,
    Declaration(DeclarationRefusal),
    ConflictingKind {
        entity: EntityId,
        a: EntityKind,
        b: EntityKind,
    },
    Chains(Vec<RegistryViolation>),
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
        kind: GeometryKind,
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

    pub fn check(&self) -> Result<(), Vec<RegistryViolation>> {
        let violations = self.registry.validate();
        if violations.is_empty() {
            Ok(())
        } else {
            Err(violations)
        }
    }
}

pub fn load_registry(json: &str, source: &str) -> Result<Identity, RegistryLoadRefusal> {
    let Authored(document): Authored<DeclarationDocument> =
        serde_json::from_str(json).map_err(decode_refusal)?;
    let rows = match document.unifications {
        Some(rows) => rows,
        None if document.comment.is_none() => Vec::new(),
        None => return Err(RegistryLoadRefusal::MissingDeclarations),
    };
    let mut registry = Registry::default();
    let mut declared = BTreeMap::new();
    for Authored(row) in rows {
        registry
            .declare(
                row.canonical.clone(),
                row.minted.clone(),
                Unification::Declared {
                    reason: row.reason,
                    source: source.to_owned(),
                },
            )
            .map_err(RegistryLoadRefusal::Declaration)?;
        for id in [row.canonical, row.minted] {
            match declared.entry(id) {
                std::collections::btree_map::Entry::Vacant(entry) => {
                    entry.insert(row.kind);
                }
                std::collections::btree_map::Entry::Occupied(entry) if *entry.get() == row.kind => {
                }
                std::collections::btree_map::Entry::Occupied(entry) => {
                    return Err(RegistryLoadRefusal::ConflictingKind {
                        entity: entry.key().clone(),
                        a: *entry.get(),
                        b: row.kind,
                    });
                }
            }
        }
    }
    let chains: Vec<_> = registry
        .validate()
        .into_iter()
        .filter(|v| matches!(v, RegistryViolation::ChainedUnification { .. }))
        .collect();
    if !chains.is_empty() {
        return Err(RegistryLoadRefusal::Chains(chains));
    }
    Ok(Identity { registry, declared })
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DeclarationDocument {
    #[serde(default, deserialize_with = "present")]
    unifications: Option<Vec<Authored<Declaration>>>,
    #[serde(default, rename = "_comment", deserialize_with = "present")]
    comment: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Declaration {
    #[serde(deserialize_with = "entity_id")]
    canonical: EntityId,
    #[serde(deserialize_with = "entity_id")]
    minted: EntityId,
    #[serde(deserialize_with = "entity_kind")]
    kind: EntityKind,
    reason: String,
}

struct Authored<T>(T);

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Authored<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_map(AuthoredVisitor(PhantomData))
    }
}

struct AuthoredVisitor<T>(PhantomData<T>);

impl<'de, T: Deserialize<'de>> serde::de::Visitor<'de> for AuthoredVisitor<T> {
    type Value = Authored<T>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("an authored declaration object")
    }

    fn visit_map<A: serde::de::MapAccess<'de>>(self, access: A) -> Result<Self::Value, A::Error> {
        T::deserialize(serde::de::value::MapAccessDeserializer::new(access)).map(Authored)
    }
}

fn present<'de, D: Deserializer<'de>, T: Deserialize<'de>>(
    deserializer: D,
) -> Result<Option<T>, D::Error> {
    T::deserialize(deserializer).map(Some)
}

fn entity_id<'de, D: Deserializer<'de>>(deserializer: D) -> Result<EntityId, D::Error> {
    String::deserialize(deserializer).map(EntityId)
}

fn entity_kind<'de, D: Deserializer<'de>>(deserializer: D) -> Result<EntityKind, D::Error> {
    use serde::de::IntoDeserializer;
    EntityKind::deserialize(String::deserialize(deserializer)?.into_deserializer())
}

fn decode_refusal(error: serde_json::Error) -> RegistryLoadRefusal {
    match error.classify() {
        serde_json::error::Category::Data | serde_json::error::Category::Io => {
            RegistryLoadRefusal::InvalidDocument
        }
        serde_json::error::Category::Syntax => RegistryLoadRefusal::InvalidSyntax,
        serde_json::error::Category::Eof => RegistryLoadRefusal::IncompleteDocument,
    }
}

fn implied_kind(layer: LayerKind, kind: GeometryKind) -> EntityKind {
    match kind {
        GeometryKind::Way => EntityKind::Route,
        GeometryKind::Point | GeometryKind::Memory => EntityKind::Place,
        GeometryKind::Line => EntityKind::Waterbody,
        GeometryKind::Area => match layer {
            LayerKind::Water => EntityKind::Waterbody,
            LayerKind::Relief => EntityKind::Terrain,
            LayerKind::ScriptureClaims => EntityKind::Allotment,
            LayerKind::Territory | LayerKind::Background | LayerKind::Journeys => {
                EntityKind::Polity
            }
        },
    }
}

#[cfg(test)]
mod laws {
    use super::*;
    use proptest::prelude::*;
    use serde_json::json;
    use serde_json::Value;
    use strum::IntoEnumIterator;

    const MAX_DECLARATIONS: usize = 12;

    proptest! {
        #[test]
        fn generated_non_object_documents_are_refused(value in non_object()) {
            prop_assert_eq!(
                load_registry(&value.to_string(), "source").err(),
                Some(RegistryLoadRefusal::InvalidDocument),
                "a scalar or array cannot become an empty registry"
            );
        }

        #[test]
        fn generated_metadata_requires_a_declaration_collection(comment in any::<String>()) {
            let text = json!({"_comment":comment}).to_string();
            prop_assert_eq!(
                load_registry(&text, "source").err(),
                Some(RegistryLoadRefusal::MissingDeclarations),
                "metadata cannot silently omit its declaration collection"
            );
        }

        #[test]
        fn generated_unknown_document_fields_are_refused(token in "[a-z0-9]{0,24}") {
            let text = json!({format!("unification{token}"):[]}).to_string();
            prop_assert_eq!(
                load_registry(&text, "source").err(),
                Some(RegistryLoadRefusal::InvalidDocument),
                "misspelled fields cannot hide declarations"
            );
        }

        #[test]
        fn generated_non_array_collections_are_refused(value in non_array()) {
            let text = json!({"unifications":value}).to_string();
            prop_assert_eq!(
                load_registry(&text, "source").err(),
                Some(RegistryLoadRefusal::InvalidDocument),
                "a declaration collection must be an array"
            );
        }

        #[test]
        fn generated_unknown_kind_spellings_are_refused(token in any::<String>()) {
            let text = json!({"unifications":[{"canonical":"home","minted":"alias","kind":format!("Unknown:{token}"),"reason":"written identity"}]}).to_string();
            prop_assert_eq!(
                load_registry(&text, "source").err(),
                Some(RegistryLoadRefusal::InvalidDocument),
                "unknown spellings cannot become internal entity kinds"
            );
        }

        #[test]
        fn generated_incomplete_rows_are_refused(token in "[a-z0-9]{1,24}") {
            for field in ["canonical", "minted", "kind", "reason"] {
                let mut row = json!({"canonical":format!("home:{token}"),"minted":format!("alias:{token}"),"kind":EntityKind::Place,"reason":"written identity"});
                row.as_object_mut().unwrap().remove(field);
                let text = json!({"unifications":[row]}).to_string();
                prop_assert_eq!(
                    load_registry(&text, "source").err(),
                    Some(RegistryLoadRefusal::InvalidDocument),
                    "all semantic declaration fields are required"
                );
            }
        }

        #[test]
        fn generated_non_string_row_fields_are_refused(number in any::<i64>()) {
            for field in ["canonical", "minted", "kind", "reason"] {
                for value in [
                    json!(null),
                    json!(number),
                    json!(false),
                    json!([]),
                    json!({}),
                ] {
                    let mut row = json!({"canonical":"home","minted":"alias","kind":EntityKind::Place,"reason":"written identity"});
                    row.as_object_mut().unwrap().insert(field.into(), value);
                    let text = json!({"unifications":[row]}).to_string();
                    prop_assert_eq!(
                        load_registry(&text, "source").err(),
                        Some(RegistryLoadRefusal::InvalidDocument),
                        "semantic row fields retain their authored string representation"
                    );
                }
            }
        }

        #[test]
        fn generated_alias_duplicates_have_a_typed_refusal(
            token in "[a-z0-9]{1,24}",
            reason in written_text(),
            source in written_text(),
        ) {
            let minted = EntityId(format!("alias:{token}"));
            let row =
                json!({"canonical":"home","minted":minted.0,"kind":EntityKind::Place,"reason":reason});
            let text = json!({"unifications":[row.clone(),row]}).to_string();
            prop_assert_eq!(
                load_registry(&text, &source).err(),
                Some(RegistryLoadRefusal::Declaration(
                    DeclarationRefusal::Duplicate(minted)
                )),
                "a duplicate alias cannot replace its written decision"
            );
        }

        #[test]
        fn generated_shared_homes_refuse_every_conflicting_kind(
            token in "[a-z0-9]{1,24}",
            reason in written_text(),
            source in written_text(),
            reverse in any::<bool>(),
        ) {
            let home = EntityId(format!("home:{token}"));
            for a in EntityKind::iter() {
                for b in EntityKind::iter().filter(|kind| *kind != a) {
                    let mut rows = vec![
                        json!({"canonical":home.0,"minted":format!("a:{token}"),"kind":a,"reason":reason}),
                        json!({"canonical":home.0,"minted":format!("b:{token}"),"kind":b,"reason":reason}),
                    ];
                    if reverse {
                        rows.reverse();
                    }
                    let (first, second) = if reverse { (b, a) } else { (a, b) };
                    let text = json!({"unifications":rows}).to_string();
                    prop_assert_eq!(
                        load_registry(&text, &source).err(),
                        Some(RegistryLoadRefusal::ConflictingKind {
                            entity: home.clone(),
                            a: first,
                            b: second
                        }),
                        "all declarations for a home must agree on its entity kind"
                    );
                }
            }
        }

        #[test]
        fn generated_alias_sets_preserve_kinds_ids_and_provenance(
            tokens in proptest::collection::btree_set("[a-z0-9]{1,12}", 1..MAX_DECLARATIONS).prop_map(|set| set.into_iter().collect::<Vec<_>>()).prop_shuffle(),
            field_order in Just(vec![0usize,1,2,3]).prop_shuffle(),
            reason in written_text(),
            source in written_text(),
            comment in any::<String>(),
        ) {
            for kind in EntityKind::iter() {
                let rows: Vec<_> = tokens
                    .iter()
                    .map(|token| {
                        ordered_row(
                            "home",
                            &format!("alias:{token}"),
                            kind,
                            &reason,
                            &field_order,
                        )
                    })
                    .collect();
                let text = format!(
                    r#"{{"_comment":{},"unifications":[{}]}}"#,
                    json!(comment),
                    rows.join(",")
                );
                let mut identity = load_registry(&text, &source).expect("consistent written decisions");
                let resolved: Vec<_> = tokens
                    .iter()
                    .map(|token| {
                        identity
                            .resolve(&EntityId(format!("alias:{token}")))
                            .clone()
                    })
                    .collect();
                prop_assert_eq!(
                    resolved,
                    vec![EntityId("home".into()); tokens.len()],
                    "every generated alias retains its written home"
                );
                let reasons: Vec<_> = tokens
                    .iter()
                    .map(|token| {
                        identity
                            .registry()
                            .why(&EntityId(format!("alias:{token}")))
                            .cloned()
                    })
                    .collect();
                prop_assert_eq!(
                    reasons,
                    vec![
                        Some(Unification::Declared {
                            reason: reason.clone(),
                            source: source.clone()
                        });
                        tokens.len()
                    ],
                    "all written reasons and source provenance survive admission"
                );
                prop_assert_eq!(
                    identity.check(),
                    Err(vec![RegistryViolation::DanglingCanonical(EntityId(
                        "home".into()
                    ))]),
                    "unobserved homes have a complete typed refusal"
                );
                identity.witness(
                    &EntityId("home".into()),
                    "Home",
                    LayerKind::Territory,
                    Witness::Atlas,
                    GeometryKind::Area,
                );
                for token in &tokens {
                    identity.witness(
                        &EntityId(format!("alias:{token}")),
                        "Alias",
                        LayerKind::Background,
                        Witness::Basemap,
                        GeometryKind::Area,
                    );
                }
                let kinds: Vec<_> = identity
                    .registry()
                    .entities()
                    .map(|entity| entity.kind)
                    .collect();
                prop_assert_eq!(
                    kinds,
                    vec![kind],
                    "every declared entity kind survives geometry observation"
                );
                prop_assert_eq!(
                    identity.check(),
                    Ok(()),
                    "witnessed consistent homes satisfy the registry laws"
                );
            }
        }

        #[test]
        fn generated_chains_are_refused_in_every_declaration_order(
            token in "[a-z0-9]{1,24}",
            reason in written_text(),
            source in written_text(),
            reverse in any::<bool>(),
        ) {
            let minted = EntityId(format!("alias:{token}"));
            let via = EntityId(format!("middle:{token}"));
            let mut rows = vec![
                json!({"canonical":via.0,"minted":minted.0,"kind":EntityKind::Place,"reason":reason}),
                json!({"canonical":format!("home:{token}"),"minted":via.0,"kind":EntityKind::Place,"reason":reason}),
            ];
            if reverse {
                rows.reverse();
            }
            prop_assert_eq!(
                load_registry(&json!({"unifications":rows}).to_string(), &source).err(),
                Some(RegistryLoadRefusal::Chains(vec![
                    RegistryViolation::ChainedUnification { minted, via }
                ])),
                "unifications resolve in one written hop"
            );
        }

        #[test]
        fn generated_blank_identity_fields_are_refused(
            blank in blank_text(),
            reason in written_text(),
            source in written_text(),
        ) {
            for (canonical, minted) in [(blank.as_str(), "alias"), ("home", blank.as_str())] {
                let text = json!({"unifications":[{"canonical":canonical,"minted":minted,"kind":EntityKind::Place,"reason":reason}]}).to_string();
                prop_assert_eq!(
                    load_registry(&text, &source).err(),
                    Some(RegistryLoadRefusal::Declaration(
                        DeclarationRefusal::BlankIdentity
                    )),
                    "both declaration identities must be nonblank"
                );
            }
        }

        #[test]
        fn generated_blank_reasons_or_sources_are_refused(
            blank in blank_text(),
            reason in written_text(),
            source in written_text(),
        ) {
            for (reason, source) in [
                (blank.as_str(), source.as_str()),
                (reason.as_str(), blank.as_str()),
            ] {
                let text = json!({"unifications":[{"canonical":"home","minted":"alias","kind":EntityKind::Place,"reason":reason}]}).to_string();
                prop_assert_eq!(
                    load_registry(&text, source).err(),
                    Some(RegistryLoadRefusal::Declaration(
                        DeclarationRefusal::MissingProvenance
                    )),
                    "written decisions retain a nonblank reason and source"
                );
            }
        }

        #[test]
        fn generated_geometry_kinds_have_total_entity_kind_inference(token in "[a-z0-9]{1,24}") {
            for layer in LayerKind::iter() {
                for geometry in GeometryKind::iter() {
                    let minted = EntityId(format!("id:{token}"));
                    let mut identity = load_registry("{}", "source").expect("empty registry");
                    identity.witness(&minted, "Witness", layer, Witness::Atlas, geometry);
                    let expected = map_canon::Entity {
                        id: minted.clone(),
                        names: vec!["Witness".into()],
                        kind: expected_kind(layer, geometry),
                        witnesses: vec![WitnessRef {
                            minted_as: minted.clone(),
                            witness: Witness::Atlas,
                            layer,
                            kind: geometry,
                        }],
                    };
                    prop_assert_eq!(
                        identity.registry().get(&minted),
                        Some(&expected),
                        "every typed geometry has a complete witnessed entity"
                    );
                }
            }
        }

        #[test]
        fn generated_empty_collections_and_metadata_are_supported(comment in any::<String>()) {
            for text in [
                "{}".to_owned(),
                "{ \n }".to_owned(),
                "{\"unifications\":[]}".to_owned(),
                json!({"_comment":comment,"unifications":[]}).to_string(),
            ] {
                let identity = load_registry(&text, "source").expect("explicit empty registry");
                prop_assert_eq!(
                    identity.registry(),
                    &Registry::default(),
                    "valid empty declarations preserve the empty registry"
                );
            }
        }

        #[test]
        fn generated_truncated_documents_have_a_typed_refusal(reason in written_text()) {
            let mut text = json!({"unifications":[{"canonical":"home","minted":"alias","kind":EntityKind::Place,"reason":reason}]}).to_string();
            text.pop();
            prop_assert_eq!(
                load_registry(&text, "source").err(),
                Some(RegistryLoadRefusal::IncompleteDocument),
                "a truncated declaration document cannot be admitted"
            );
        }

        #[test]
        fn generated_invalid_syntax_has_a_typed_refusal(token in "[a-z0-9]{1,24}") {
            let text = format!(r#"{{"unifications":[],invalid:{token}}}"#);
            prop_assert_eq!(
                load_registry(&text, "source").err(),
                Some(RegistryLoadRefusal::InvalidSyntax),
                "invalid JSON syntax cannot be admitted"
            );
        }
    }

    fn non_object() -> impl Strategy<Value = Value> {
        prop_oneof![
            scalar(),
            proptest::collection::vec(scalar(), 0..MAX_DECLARATIONS).prop_map(Value::Array)
        ]
    }

    fn non_array() -> impl Strategy<Value = Value> {
        prop_oneof![
            scalar(),
            proptest::collection::btree_map("[a-z]{1,12}", scalar(), 0..MAX_DECLARATIONS)
                .prop_map(|entries| Value::Object(entries.into_iter().collect()))
        ]
    }

    fn scalar() -> impl Strategy<Value = Value> {
        prop_oneof![
            Just(Value::Null),
            any::<bool>().prop_map(Value::Bool),
            any::<i64>().prop_map(|number| json!(number)),
            any::<String>().prop_map(Value::String)
        ]
    }

    fn written_text() -> impl Strategy<Value = String> {
        "[a-zA-Z][a-zA-Z0-9 .:'\\-]{0,48}"
    }

    fn blank_text() -> impl Strategy<Value = String> {
        proptest::collection::vec(
            proptest::sample::select(vec![' ', '\t', '\n', '\r', '\u{2003}']),
            0..16,
        )
        .prop_map(|chars| chars.into_iter().collect())
    }

    fn ordered_row(
        home: &str,
        minted: &str,
        kind: EntityKind,
        reason: &str,
        order: &[usize],
    ) -> String {
        let fields = [
            format!("\"canonical\":{}", json!(home)),
            format!("\"minted\":{}", json!(minted)),
            format!("\"kind\":{}", json!(kind)),
            format!("\"reason\":{}", json!(reason)),
        ];
        let ordered: Vec<_> = order.iter().map(|index| fields[*index].as_str()).collect();
        format!("{{{}}}", ordered.join(","))
    }

    fn expected_kind(layer: LayerKind, geometry: GeometryKind) -> EntityKind {
        match geometry {
            GeometryKind::Way => EntityKind::Route,
            GeometryKind::Point | GeometryKind::Memory => EntityKind::Place,
            GeometryKind::Line => EntityKind::Waterbody,
            GeometryKind::Area => match layer {
                LayerKind::Water => EntityKind::Waterbody,
                LayerKind::Relief => EntityKind::Terrain,
                LayerKind::ScriptureClaims => EntityKind::Allotment,
                LayerKind::Territory | LayerKind::Background | LayerKind::Journeys => {
                    EntityKind::Polity
                }
            },
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
    fn the_authored_registry_keeps_every_active_alias_and_reason() {
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
                GeometryKind::Area,
            );
            identity.witness(
                &minted,
                "alias witness",
                LayerKind::Background,
                Witness::Basemap,
                GeometryKind::Area,
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
