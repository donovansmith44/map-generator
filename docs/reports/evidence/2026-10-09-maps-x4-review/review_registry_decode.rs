use map_canon::EntityId;
use map_compile::identity::load_registry;

#[test]
fn duplicate_collection_fields_cannot_erase_authored_identity_decisions() {
    for index in 0..32 {
        let text = format!(
            r#"{{"unifications":[{{"canonical":"home","minted":"alias:{index}","kind":"Place","reason":"written identity"}}],"unifications":[]}}"#
        );
        let actual = load_registry(&text, "review source");
        assert!(actual.is_err(), "a duplicate collection must be refused before it erases an alias");
    }
}

#[test]
fn duplicate_row_fields_cannot_replace_an_authored_source_identity() {
    for index in 0..32 {
        let text = format!(
            r#"{{"unifications":[{{"canonical":"home","minted":"original:{index}","minted":"replacement:{index}","kind":"Place","reason":"written identity"}}]}}"#
        );
        let actual = load_registry(&text, "review source");
        assert!(actual.is_err(), "a duplicate identity field must be refused before it replaces an alias");
    }
}

#[test]
fn an_unambiguous_declaration_keeps_its_authored_source_identity() {
    for index in 0..32 {
        let text = format!(
            r#"{{"unifications":[{{"canonical":"home","minted":"original:{index}","kind":"Place","reason":"written identity"}}]}}"#
        );
        let identity = load_registry(&text, "review source").expect("an unambiguous declaration loads");
        let actual = identity.resolve(&EntityId(format!("original:{index}"))).clone();
        assert_eq!(actual, EntityId("home".into()), "a single declaration keeps its written home");
    }
}
