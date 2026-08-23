use super::*;

#[cfg(ironsmith_runtime_parser_tests)]
fn rendered_card(name: &str, card_types: Vec<CardType>, oracle: &str) -> String {
    let definition = crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), name)
        .card_types(card_types)
        .parse_text(oracle)
        .unwrap_or_else(|error| panic!("{name} should parse: {error}"));
    crate::compiled_text::unprocessed_compiled_lines(&definition).join("\n")
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn eternal_flame_keeps_both_dynamic_damage_results() {
    let oracle = "Eternal Flame deals X damage to target opponent or planeswalker and half X \
                  damage, rounded up, to you, where X is the number of Mountains you control.";

    assert_eq!(
        rendered_card("Eternal Flame", vec![CardType::Sorcery], oracle),
        oracle
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn magmasaur_keeps_the_failed_choice_sacrifice_and_damage_sequence() {
    let oracle = "This creature enters with five +1/+1 counters on it.\n\
                  At the beginning of your upkeep, you may remove a +1/+1 counter from this \
                  creature. If you don't, sacrifice this creature and it deals damage equal to \
                  the number of +1/+1 counters on it to each creature without flying and each \
                  player.";

    assert_eq!(
        rendered_card("Magmasaur", vec![CardType::Creature], oracle),
        oracle
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn feast_keeps_the_shared_creatures_died_count_surface() {
    let oracle = "At the beginning of your end step, if one or more creatures died this turn, \
                  you gain that much life and distribute that many +1/+1 counters among any \
                  number of creatures you control.";

    assert_eq!(
        rendered_card(
            "Feast of the Victorious Dead",
            vec![CardType::Enchantment],
            oracle,
        ),
        oracle
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn kalitas_keeps_destroyed_creature_stats_on_the_created_token() {
    let oracle = "{B}{B}{B}, {T}: Destroy target creature. If that creature dies this way, \
                  create a black Vampire creature token. Its power is equal to that creature's \
                  power and its toughness is equal to that creature's toughness.";

    assert_eq!(
        rendered_card(
            "Kalitas, Bloodchief of Ghet",
            vec![CardType::Creature],
            oracle,
        ),
        oracle
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn wrath_of_the_skies_keeps_the_paid_energy_threshold() {
    let oracle = "You get X {E}, then you may pay any amount of {E}. Destroy each artifact, \
                  creature, and enchantment with mana value less than or equal to the amount of \
                  {E} paid this way.";

    assert_eq!(
        rendered_card("Wrath of the Skies", vec![CardType::Sorcery], oracle),
        oracle
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn created_collection_with_a_permanent_triggered_grant_uses_have() {
    let oracle = "At the beginning of your end step, create a number of 1/1 black and green Pest \
                  creature tokens equal to the number of +1/+1 counters on this creature. They \
                  have \"When this token dies, you gain 1 life.\"";

    assert_eq!(
        rendered_card("Pest Maker", vec![CardType::Creature], oracle),
        oracle
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn temporary_quoted_activated_grant_keeps_inner_terminal_period() {
    let oracle = "Until end of turn, target creature gains haste and \"{0}: Untap this creature. \
                  Activate only once.\"\n\
                  Draw a card at the beginning of the next turn's upkeep.";

    assert_eq!(
        rendered_card("Vital Touch", vec![CardType::Instant], oracle),
        oracle
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn dynamic_life_payment_hand_reveal_selection_exile_stays_one_pipeline() {
    let oracle = "When this creature enters, pay any amount of life. Target opponent reveals that \
                  many cards from their hand. You choose one of them and exile it.";

    assert_eq!(
        rendered_card("Life-Payment Confessor", vec![CardType::Creature], oracle),
        oracle
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn granted_permanent_abilities_keep_pronoun_and_self_type_surfaces() {
    let oracle = "You may put an artifact or creature card from your hand onto the battlefield. \
                  That permanent gains haste, \"When this permanent leaves the battlefield, it \
                  deals damage equal to its mana value to each creature,\" and \"At the beginning \
                  of your end step, sacrifice this permanent.\"";

    assert_eq!(
        rendered_card("Velocity Probe", vec![CardType::Sorcery], oracle),
        oracle
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn quoted_cant_be_blocked_grant_remains_a_typed_quoted_restriction() {
    let oracle = "−7: Target artifact you control becomes a 9/9 Construct artifact creature and \
                  gains vigilance, indestructible, and \"This creature can't be blocked.\"";

    assert_eq!(
        rendered_card(
            "Masked Inquisitor Probe",
            vec![CardType::Planeswalker],
            oracle,
        ),
        oracle
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn conditional_copy_grant_keeps_haste_and_the_delayed_sacrifice() {
    let oracle = "Flying\n\
                  Whenever you cast a spell from exile, copy it. You may choose new targets for \
                  the copy. If it's a permanent spell, the copy gains haste and \"At the \
                  beginning of the end step, sacrifice this permanent.\"";

    assert_eq!(
        rendered_card("Exile Copy Probe", vec![CardType::Creature], oracle),
        oracle
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn permanent_count_pump_keeps_the_plus_one_for_each_surface_and_owner_scope() {
    let oracle = "Umbris gets +1/+1 for each card your opponents own in exile.\n\
                  Whenever Umbris or another Nightmare or Horror you control enters, \
                  target opponent exiles cards from the top of their library until they exile a \
                  land card.";
    let definition = crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Umbris")
        .supertypes(vec![crate::types::Supertype::Legendary])
        .card_types(vec![CardType::Creature])
        .parse_text(oracle)
        .expect("Umbris should parse");

    assert_eq!(
        crate::compiled_text::unprocessed_compiled_lines(&definition).join("\n"),
        oracle
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn per_opponent_hand_exile_keeps_each_permission_linked_to_its_cards_constraints() {
    let oracle = "Vigilance\n\
                  When this creature enters, each opponent exiles a card from their hand and may \
                  play that card for as long as it remains exiled. Each spell cast this way costs \
                  {1} more to cast. Each land played this way enters tapped.";

    assert_eq!(
        rendered_card(
            "Linked Exile Permission Probe",
            vec![CardType::Creature],
            oracle,
        ),
        oracle
    );
}
