use super::*;

fn compile_saga_chapter(text: &str) -> crate::cards::CardDefinition {
    crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Entry Counter Probe")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Saga])
        .parse_text(text)
        .expect("next-cast Saga chapter should compile")
}

#[test]
fn next_creature_spell_entry_counter_uses_a_stable_identity_replacement() {
    let text = "II — When you next cast a creature spell this turn, that creature enters with an additional +1/+1 counter on it.";
    let definition = compile_saga_chapter(text);
    let debug = format!("{definition:#?}");

    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        text
    );
    assert!(
        debug.contains("RegisterNextBatchEnterWithCountersEffect"),
        "{debug}"
    );
    assert!(debug.contains("same_stable_id_tag: Some"), "{debug}");
    assert!(!debug.contains("PutCountersEffect"), "{debug}");
}

#[test]
fn ordinary_next_cast_counter_placement_is_not_promoted_to_entry_replacement() {
    let text = "II — When you next cast a creature spell this turn, put a +1/+1 counter on it.";
    let definition = compile_saga_chapter(text);
    let debug = format!("{definition:#?}");

    assert!(debug.contains("PutCountersEffect"), "{debug}");
    assert!(
        !debug.contains("RegisterNextBatchEnterWithCountersEffect"),
        "{debug}"
    );
}
