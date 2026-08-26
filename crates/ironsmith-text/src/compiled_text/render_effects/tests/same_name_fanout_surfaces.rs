use super::*;

fn compile_artifact(name: &str, oracle: &str) -> crate::cards::CardDefinition {
    crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), name)
        .card_types(vec![CardType::Artifact])
        .parse_text(oracle)
        .expect("same-name fanout fixture should compile")
}

#[test]
fn moratorium_stone_keeps_three_same_name_sets_in_one_clause() {
    let oracle = "{2}, {T}: Exile target card from a graveyard.\n{2}{W}{B}, {T}, Sacrifice this artifact: Exile target nonland card from a graveyard, all other cards from graveyards with the same name as that card, and all permanents with that name.";
    let definition = compile_artifact("Moratorium Stone", oracle);

    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        oracle
    );
    let debug = format!("{definition:#?}");
    assert!(debug.contains("linked_fanout_primary_0"), "{debug}");
    assert!(debug.contains("linked_fanout_group_0"), "{debug}");
    assert!(debug.matches("SameNameAsTagged").count() >= 2, "{debug}");
}

#[test]
fn unrestricted_third_set_is_not_rewritten_as_same_name() {
    let oracle = "{2}{W}{B}, {T}, Sacrifice this artifact: Exile target nonland card from a graveyard, all other cards from graveyards with the same name as that card, and all permanents.";
    let definition = compile_artifact("Unrestricted Fanout Probe", oracle);
    let rendered = crate::compiled_text::compiled_text_lines(&definition).join("\n");

    assert!(rendered.ends_with("Exile all permanents."), "{rendered}");
    assert!(
        !rendered.ends_with("all permanents with that name."),
        "{rendered}"
    );
}
