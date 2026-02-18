#![cfg(feature = "parser-tests-full")]

use ironsmith::{
    cards::CardDefinitionBuilder, compiled_text::compiled_lines, ids::CardId, types::CardType,
};

fn rendered_lines(text: &str, name: &str, card_types: &[CardType]) -> String {
    let mut builder = CardDefinitionBuilder::new(CardId::new(), name);

    if !card_types.is_empty() {
        builder = builder.card_types(card_types.to_vec());
    }

    let def = builder
        .parse_text(text)
        .expect("high-priority semantic mismatch oracle text should parse");
    compiled_lines(&def).join(" ").to_ascii_lowercase()
}

#[test]
fn regression_semantic_mismatch_flowstone_sculpture_choice_clause() {
    let rendered = rendered_lines(
        "{2}, Discard a card: Put a +1/+1 counter on this creature or this creature gains flying, first strike, or trample. (This effect lasts indefinitely.)",
        "Flowstone Sculpture",
        &[CardType::Creature],
    );

    assert!(
        rendered.contains("flying")
            && rendered.contains("first strike")
            && rendered.contains("trample"),
        "expected choice-based modal modes to be preserved, got {rendered}"
    );
}

#[test]
fn regression_semantic_mismatch_mountain_titan_casting_condition() {
    let rendered = rendered_lines(
        "{1}{R}{R}: Until end of turn, whenever you cast a black spell, put a +1/+1 counter on this creature.",
        "Mountain Titan",
        &[CardType::Creature],
    );

    assert!(
        rendered.contains("until end of turn"),
        "expected activated ability duration clause to remain, got {rendered}"
    );
    assert!(
        rendered.contains("whenever you cast a black spell"),
        "expected trigger-condition qualifier to remain, got {rendered}"
    );
}

#[test]
fn regression_semantic_mismatch_root_greevil_choice_qualifier() {
    let rendered = rendered_lines(
        "{2}{G}, {T}, Sacrifice this creature: Destroy all enchantments of the color of your choice.",
        "Root Greevil",
        &[CardType::Creature],
    );

    assert!(
        rendered.contains("color of your choice"),
        "expected color choice qualifier to be preserved, got {rendered}"
    );
}

#[test]
fn regression_semantic_mismatch_vesuva_copy_and_enter_tapped() {
    let rendered = rendered_lines(
        "You may have this land enter tapped as a copy of any land on the battlefield.",
        "Vesuva",
        &[CardType::Land],
    );

    assert!(
        rendered.contains("enters the battlefield tapped")
            && rendered.contains("copy of any land on the battlefield"),
        "expected copy + enter-tapped replacement to remain, got {rendered}"
    );
}

#[test]
fn regression_semantic_mismatch_reckless_blaze_triggered_dies_clause() {
    let rendered = rendered_lines(
        "Reckless Blaze deals 5 damage to each creature. Whenever a creature you control dealt damage this way dies this turn, add {R}.",
        "Reckless Blaze",
        &[CardType::Sorcery],
    );

    assert!(
        rendered.contains("whenever"),
        "expected triggered death clause to be preserved, got {rendered}"
    );
    assert!(
        rendered.contains("dies this turn"),
        "expected duration condition on death trigger to be preserved, got {rendered}"
    );
    assert!(
        rendered.contains("add {r}"),
        "expected colorless mana gain clause to remain, got {rendered}"
    );
}

#[test]
fn regression_semantic_mismatch_admonition_angel_nonland_permanent_target() {
    let rendered = rendered_lines(
        "Flying\nLandfall — Whenever a land you control enters, you may exile target nonland permanent other than this creature.\nWhen this creature leaves the battlefield, return all cards exiled with it to the battlefield under their owners' control.",
        "Admonition Angel",
        &[CardType::Creature],
    );

    assert!(
        rendered.contains("nonland permanent"),
        "expected landfall target to remain a nonland permanent, got {rendered}"
    );
    assert!(
        !rendered.contains("nonland creature"),
        "landfall target should not narrow to creatures, got {rendered}"
    );
}
