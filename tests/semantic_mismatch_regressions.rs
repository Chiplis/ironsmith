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
        rendered.contains("put a +1/+1 counter on a creature"),
        "expected at least one preserved mode from the choice clause, got {rendered}"
    );
}

#[test]
fn regression_semantic_mismatch_mountain_titan_casting_condition() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Mountain Titan")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "{1}{R}{R}: Until end of turn, whenever you cast a black spell, put a +1/+1 counter on this creature.",
        )
        .expect_err("Mountain Titan clause is currently unsupported");
    let rendered = format!("{err:?}").to_ascii_lowercase();

    assert!(
        rendered.contains("unsupported until-end-of-turn permission clause"),
        "expected explicit unsupported error for Mountain Titan clause, got {rendered}"
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
        rendered.contains("choose one")
            && rendered.contains("white enchantment")
            && rendered.contains("blue enchantment")
            && rendered.contains("black enchantment")
            && rendered.contains("red enchantment")
            && rendered.contains("green enchantment"),
        "expected color-choice semantics to be preserved as explicit modes, got {rendered}"
    );
}

#[test]
fn regression_semantic_mismatch_vesuva_copy_and_enter_tapped() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Vesuva")
        .card_types(vec![CardType::Land])
        .parse_text("You may have this land enter tapped as a copy of any land on the battlefield.")
        .expect_err("Vesuva enters-as-copy replacement is currently unsupported");
    let rendered = format!("{err:?}").to_ascii_lowercase();

    assert!(
        rendered.contains("unsupported enters-as-copy replacement clause"),
        "expected explicit unsupported error for Vesuva clause, got {rendered}"
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

#[test]
fn regression_semantic_mismatch_asmira_graveyard_battlefield_clause() {
    let rendered = rendered_lines(
        "Flying\nAt the beginning of each end step, put a +1/+1 counter on Asmira for each creature put into your graveyard from the battlefield this turn.",
        "Asmira, Holy Avenger",
        &[CardType::Creature],
    );

    assert!(
        rendered.contains("from the battlefield this turn"),
        "expected battlefield graveyard timing clause to remain, got {rendered}"
    );
}

#[test]
fn regression_semantic_mismatch_entered_battlefield_under_control_this_turn() {
    let rendered = rendered_lines(
        "Flying, vigilance\n{T}: Put a +1/+1 counter on each creature that entered the battlefield under your control this turn.",
        "Shaile, Dean of Radiance // Embrose, Dean of Shadow",
        &[CardType::Creature],
    );

    assert!(
        rendered.contains("entered the battlefield under your control this turn"),
        "expected entry-timing clause to remain, got {rendered}"
    );
}
