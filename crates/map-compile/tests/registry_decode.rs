use map_canon::{EntityId, EntityKind};
use map_compile::identity::{load_registry, RegistryLoadRefusal};
use proptest::prelude::*;
use strum::IntoEnumIterator;

proptest! {
    #[test]
    fn duplicate_collection_fields_cannot_erase_authored_identity_decisions(
        alias in "[a-z0-9]{1,24}",
        reverse in any::<bool>(),
    ) {
        let rows = format!(
            r#"[{{"canonical":"home","minted":"alias:{alias}","kind":"Place","reason":"written identity"}}]"#
        );
        let fields = if reverse {
            format!(r#""unifications":[],"unifications":{rows}"#)
        } else {
            format!(r#""unifications":{rows},"unifications":[]"#)
        };
        let actual = load_registry(&format!("{{{fields}}}"), "review source");
        prop_assert_eq!(
            actual.err(),
            Some(RegistryLoadRefusal::InvalidDocument),
            "a duplicate collection must be refused before it erases an alias"
        );
    }

    #[test]
    fn duplicate_row_fields_cannot_replace_an_authored_source_identity(
        alias in "[a-z0-9]{1,24}",
        reverse in any::<bool>(),
    ) {
        let fields = if reverse {
            format!(r#""minted":"replacement:{alias}","minted":"original:{alias}""#)
        } else {
            format!(r#""minted":"original:{alias}","minted":"replacement:{alias}""#)
        };
        let text = format!(
            r#"{{"unifications":[{{"canonical":"home",{fields},"kind":"Place","reason":"written identity"}}]}}"#
        );
        let actual = load_registry(&text, "review source");
        prop_assert_eq!(
            actual.err(),
            Some(RegistryLoadRefusal::InvalidDocument),
            "a duplicate identity field must be refused before it replaces an alias"
        );
    }

    #[test]
    fn every_semantic_row_field_refuses_duplicate_values(
        token in "[a-z0-9]{1,24}",
        order in Just(vec![0usize, 1, 2, 3, 4]).prop_shuffle(),
        kind in proptest::sample::select(EntityKind::iter().collect::<Vec<_>>()),
        replacement_kind in proptest::sample::select(EntityKind::iter().collect::<Vec<_>>()),
    ) {
        for field in ["canonical", "minted", "kind", "reason"] {
            let mut fields = vec![
                format!(r#""canonical":"home:{token}""#),
                format!(r#""minted":"alias:{token}""#),
                format!("\"kind\":{}", serde_json::to_string(&kind).unwrap()),
                format!(r#""reason":"written {token}""#),
            ];
            let replacement = match field {
                "kind" => serde_json::json!(replacement_kind),
                _ => serde_json::json!(format!("replacement:{token}")),
            };
            fields.push(format!(
                "{}:{}",
                serde_json::to_string(field).unwrap(),
                replacement.to_string()
            ));
            let shuffled: Vec<_> = order.iter().map(|index| fields[*index].clone()).collect();
            let text = format!(r#"{{"unifications":[{{{}}}]}}"#, shuffled.join(","));
            prop_assert_eq!(
                load_registry(&text, "review source").err(),
                Some(RegistryLoadRefusal::InvalidDocument),
                "a semantic field cannot discard an authored value"
            );
        }
    }

    #[test]
    fn declaration_rows_must_be_objects(token in "[a-z0-9]{1,24}") {
        let text = serde_json::json!({"unifications":[[format!("home:{token}"),format!("alias:{token}"),"Place","written identity"]]}).to_string();
        prop_assert_eq!(
            load_registry(&text, "review source").err(),
            Some(RegistryLoadRefusal::InvalidDocument),
            "a positional array cannot stand for an authored declaration object"
        );
    }

    #[test]
    fn entity_kinds_require_their_authored_string_spelling(
        kind in proptest::sample::select(EntityKind::iter().collect::<Vec<_>>()),
    ) {
        let spelling = serde_json::to_value(kind).unwrap();
        let mut tagged = serde_json::Map::new();
        tagged.insert(spelling.as_str().unwrap().to_owned(), serde_json::Value::Null);
        let text = serde_json::json!({"unifications":[{"canonical":"home","minted":"alias","kind":tagged,"reason":"written identity"}]}).to_string();
        prop_assert_eq!(load_registry(&text, "source").err(), Some(RegistryLoadRefusal::InvalidDocument), "an object cannot replace an authored kind spelling");
    }

    #[test]
    fn duplicate_metadata_fields_are_refused(
        first in any::<String>(),
        second in any::<String>(),
    ) {
        let text = format!(r#"{{"unifications":[],"_comment":{},"_comment":{}}}"#, serde_json::json!(first), serde_json::json!(second));
        prop_assert_eq!(load_registry(&text, "source").err(), Some(RegistryLoadRefusal::InvalidDocument), "authored metadata cannot silently replace another value");
    }

    #[test]
    fn an_unambiguous_declaration_keeps_its_authored_source_identity(alias in "[a-z0-9]{1,24}") {
        let text = format!(
            r#"{{"unifications":[{{"canonical":"home","minted":"original:{alias}","kind":"Place","reason":"written identity"}}]}}"#
        );
        let identity = load_registry(&text, "review source").expect("an unambiguous declaration loads");
        let actual = identity
            .resolve(&EntityId(format!("original:{alias}")))
            .clone();
        prop_assert_eq!(
            actual,
            EntityId("home".into()),
            "a single declaration keeps its written home"
        );
    }
}

#[test]
fn quarantined_canaan_frame_cannot_be_a_required_live_home() {
    let identity = load_registry(
        include_str!("../../../data/authored/registry.json"),
        "authored registry",
    )
    .expect("authored registry loads");
    let minted = EntityId("basemap:canaan".into());
    assert_eq!(
        identity.resolve(&minted),
        &minted,
        "quarantined Canaan geometry cannot supply a required canonical witness"
    );
}
