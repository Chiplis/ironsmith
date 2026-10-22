//! Semantic-loss fix wave (2026-10-21), package MB: real-text compile and
//! runtime checks for the behaviour each fix restored.

use super::*;

fn mb_compile(name: &str, card_types: Vec<CardType>, text: &str) -> crate::CardDefinition {
    crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), name)
        .card_types(card_types)
        .parse_text(text)
        .unwrap_or_else(|error| panic!("{name} should compile: {error:?}"))
}

/// Whitespace-free Debug form of the definition, for structural assertions.
fn mb_compact(definition: &crate::CardDefinition) -> String {
    format!("{definition:?}").split_whitespace().collect()
}

fn mb_text(definition: &crate::CardDefinition) -> String {
    crate::compiled_text::compiled_text_lines(definition).join("\n")
}

// Fireflux Squad: the consult follow-up keeps "tapped and attacking".
#[test]
fn mb_consult_followup_put_enters_tapped_and_attacking() {
    let definition = mb_compile(
        "Fireflux Squad",
        vec![CardType::Creature],
        "Whenever this creature attacks, you may exile another target attacking creature you control. If you do, reveal cards from the top of your library until you reveal a creature card. Put that card onto the battlefield tapped and attacking and the rest on the bottom of your library in a random order.",
    );
    let compact = mb_compact(&definition);
    assert!(
        compact.contains("enters_tapped:true,enters_attacking:true"),
        "{compact}"
    );
}

// Gaze of Justice: a color adjective after the count keeps the count.
#[test]
fn mb_counted_tap_with_color_keeps_count() {
    let definition = mb_compile(
        "Gaze of Justice",
        vec![CardType::Sorcery],
        "As an additional cost to cast this spell, tap three untapped white creatures you control.\nExile target creature.",
    );
    let compact = mb_compact(&definition);
    assert!(
        compact.contains("ChoiceCount{min:3,max:Some(3)"),
        "{compact}"
    );
    assert!(mb_text(&definition).contains("tap three white untapped creatures you control"));
}

// Giada: "for each Angel you already control" keeps the controller.
#[test]
fn mb_already_control_count_keeps_controller() {
    let definition = mb_compile(
        "Giada, Font of Hope",
        vec![CardType::Creature],
        "Each other Angel you control enters with an additional +1/+1 counter on it for each Angel you already control.",
    );
    let compact = mb_compact(&definition);
    let count_at = compact.find("Count(ObjectFilter").expect("for-each count");
    let count = &compact[count_at..];
    let angel_at = count.find("Angel").expect("Angel count filter");
    assert!(
        count[..angel_at].contains("controller:Some(You)"),
        "{count}"
    );
}

// Gilanra / Scaled Nurturer / Lapis Orb: the spend rider is a mana payload,
// not an independent "whenever you cast" trigger.
#[test]
fn mb_spend_mana_rider_with_qualified_spell_is_a_mana_payload() {
    let definition = mb_compile(
        "Gilanra, Caller of Wirewood",
        vec![CardType::Creature],
        "{T}: Add {G}. When you spend this mana to cast a spell with mana value 6 or greater, draw a card.",
    );
    let compact = mb_compact(&definition);
    assert!(compact.contains("on_spend:[ManaSpendPayload"), "{compact}");
    assert!(compact.contains("GreaterThanOrEqual(6)"), "{compact}");
    assert!(!compact.contains("SpellCastTrigger"), "{compact}");
}

#[test]
fn mb_spend_mana_rider_life_gain_is_a_mana_payload() {
    let definition = mb_compile(
        "Scaled Nurturer",
        vec![CardType::Creature],
        "{T}: Add {G}. When you spend this mana to cast a Dragon creature spell, you gain 2 life.",
    );
    let compact = mb_compact(&definition);
    assert!(compact.contains("on_spend:[ManaSpendPayload"), "{compact}");
    assert!(compact.contains("GainLifeEffect"), "{compact}");
    assert!(!compact.contains("SpellCastTrigger"), "{compact}");
}

#[test]
fn mb_spend_mana_rider_scry_two_renders_with_the_mana_ability() {
    let definition = mb_compile(
        "Lapis Orb of Dragonkind",
        vec![CardType::Artifact],
        "{T}: Add {U}. When you spend this mana to cast a Dragon creature spell, scry 2.",
    );
    assert!(!mb_compact(&definition).contains("SpellCastTrigger"));
    assert_eq!(
        mb_text(&definition),
        "{T}: Add {U}. When you spend this mana to cast a Dragon creature spell, scry 2."
    );
}

// Rickety Gazebo: "from among them" in a comma-then member reads the milled cards.
#[test]
fn mb_mill_then_return_from_among_them_reads_milled_cards() {
    let definition = mb_compile(
        "Rickety Gazebo",
        vec![CardType::Sorcery],
        "Mill four cards, then return up to two permanent cards from among them to your hand.",
    );
    let compact = mb_compact(&definition);
    assert!(compact.contains("milled"), "{compact}");
    assert!(
        !compact.contains("ReturnToHandEffect{spec:WithCount(Object(ObjectFilter{zone:Some(Battlefield)"),
        "{compact}"
    );
    assert!(mb_text(&definition).contains("from among them"));
}

// Hakbal / Ixidron: each/all subjects affect every matching object.
#[test]
fn mb_each_subject_explores_all() {
    let definition = mb_compile(
        "Hakbal of the Surging Soul",
        vec![CardType::Creature],
        "At the beginning of combat on your turn, each Merfolk creature you control explores.",
    );
    let compact = mb_compact(&definition);
    assert!(compact.contains("ExploreEffect{target:All("), "{compact}");
}

#[test]
fn mb_all_subject_turns_every_creature_face_down() {
    let definition = mb_compile(
        "Ixidron",
        vec![CardType::Creature],
        "As this creature enters, turn all other nontoken creatures face down.",
    );
    let compact = mb_compact(&definition);
    assert!(compact.contains("TurnFaceDownEffect{target:All("), "{compact}");
}

// Henrika: "and/or" keyword lists match any listed keyword.
#[test]
fn mb_and_or_keyword_list_keeps_every_keyword() {
    let definition = mb_compile(
        "Henrika, Infernal Seer",
        vec![CardType::Creature],
        "{1}{B}{B}: Each creature you control with flying, deathtouch, and/or lifelink gets +1/+0 until end of turn.",
    );
    let compact = mb_compact(&definition);
    for keyword in ["Flying", "Deathtouch", "Lifelink"] {
        assert!(compact.contains(&format!("static_abilities:[{keyword}]")), "{keyword}: {compact}");
    }
}

// Herald of Ilharg: "that spell" in a cast-trigger body is the cast spell.
#[test]
fn mb_that_spell_condition_binds_to_cast_spell() {
    let definition = mb_compile(
        "Herald of Ilharg",
        vec![CardType::Creature],
        "Whenever you cast a creature spell, put two +1/+1 counters on this creature. If that spell has mana value 5 or greater, this creature deals damage equal to the number of counters on it to each opponent.",
    );
    let compact = mb_compact(&definition);
    assert!(
        compact.contains("TaggedObjectMatches(TagKey(\"triggering\")"),
        "{compact}"
    );
    assert!(!compact.contains("condition:SourceMatches"), "{compact}");
}

// Hexhaven: a relative-clause "this turn" stays in the become subject filter.
#[test]
fn mb_become_subject_keeps_attacked_this_turn() {
    let definition = mb_compile(
        "Hexhaven Dueling Arena",
        vec![CardType::Land],
        "{2}, {T}: Target creature that attacked this turn becomes prepared.",
    );
    assert!(mb_compact(&definition).contains("attacked_this_turn:true"));
}

// Homura: "return it to the battlefield flipped" flips the returned card.
#[test]
fn mb_return_flipped_flips_returned_card() {
    let definition = mb_compile(
        "Homura, Human Ascendant",
        vec![CardType::Creature],
        "When this creature dies, return it to the battlefield flipped.",
    );
    assert!(mb_compact(&definition).contains("FlipEffect"));
    assert!(mb_text(&definition).contains("to the battlefield flipped"));
}

// Isildur: the named player chooses the counted cards from their hand.
#[test]
fn mb_counted_hand_exile_is_chosen_by_named_player() {
    let definition = mb_compile(
        "Isildur's Fateful Strike",
        vec![CardType::Instant],
        "Destroy target creature. If its controller has more than four cards in hand, they exile cards from their hand equal to the difference.",
    );
    let compact = mb_compact(&definition);
    assert!(compact.contains("ChooseObjectsEffect"), "{compact}");
    assert!(!compact.contains("chooser:You"), "{compact}");
    assert!(compact.contains("count_value:Some("), "{compact}");
}

// Isu: "pay {G}, {W}, or {U}" is one payment among alternatives.
#[test]
fn mb_disjunctive_symbol_payment_is_one_pip() {
    let definition = mb_compile(
        "Isu the Abominable",
        vec![CardType::Creature],
        "Whenever another snow permanent you control enters, you may pay {G}, {W}, or {U}. If you do, put a +1/+1 counter on this creature.",
    );
    let compact = mb_compact(&definition);
    assert!(compact.contains("pips:[[Green,White,Blue]]"), "{compact}");
    assert!(mb_text(&definition).contains("pay {G}, {W}, or {U}"));
}

// Jace: "except they're not legendary" removes Legendary.
#[test]
fn mb_token_copy_not_legendary_removes_supertype() {
    let definition = mb_compile(
        "Jace, Cunning Castaway",
        vec![CardType::Planeswalker],
        "−5: Create two tokens that are copies of Jace, except they're not legendary.",
    );
    let compact = mb_compact(&definition);
    assert!(compact.contains("removed_supertypes:[Legendary]"), "{compact}");
    assert!(!compact.contains("added_supertypes:[Legendary]"), "{compact}");
}

// Kodama: "put onto the battlefield with this ability" is a real predicate.
#[test]
fn mb_put_with_this_ability_condition_is_not_empty() {
    let definition = mb_compile(
        "Kodama of the East Tree",
        vec![CardType::Creature],
        "Whenever another permanent you control enters, if it wasn't put onto the battlefield with this ability, you may put a permanent card with equal or lesser mana value from your hand onto the battlefield.",
    );
    assert!(mb_compact(&definition).contains("put_onto_battlefield_with_source:true"));
}

// Kyren Toy: "equal to X plus one" is an amount, not an {X} mana symbol.
#[test]
fn mb_equal_to_amount_is_not_a_mana_symbol() {
    let definition = mb_compile(
        "Kyren Toy",
        vec![CardType::Artifact],
        "{T}, Remove X charge counters from this artifact: Add an amount of {C} equal to X plus one.",
    );
    let compact = mb_compact(&definition);
    assert!(compact.contains("AddScaledManaEffect{mana:[Colorless]"), "{compact}");
}
