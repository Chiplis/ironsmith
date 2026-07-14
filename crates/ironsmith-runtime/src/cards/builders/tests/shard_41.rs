use super::shard_16::{assert_oracle_card_parses_strict, parse_oracle_card_definition};
use super::*;

fn assert_compiled_clause(card: &str, expected: &str) {
    assert_oracle_card_parses_strict(card);
    let definition = parse_oracle_card_definition(card);
    let compiled = compiled_text_lines(&definition);
    assert!(
        compiled.iter().any(|line| line == expected),
        "{card} must preserve its complete focused clause: {compiled:#?}"
    );
}

fn assert_number_equal_surface(card: &str) {
    assert_oracle_card_parses_strict(card);
    let definition = parse_oracle_card_definition(card);
    let compiled = compiled_text_lines(&definition);
    assert!(
        compiled.iter().any(|line| {
            let line = line.to_ascii_lowercase();
            line.contains("a number of") && line.contains("equal to")
        }),
        "{card} must preserve its explicit 'a number of ... equal to ...' surface: {compiled:#?}"
    );
}

#[test]
pub(super) fn energy_followups_remain_in_their_conjoined_action_clauses() {
    for (card, expected) in [
        (
            "Greenbelt Rampager",
            "When this creature enters, pay {E}{E}. If you can't, return this creature to its owner's hand and you get {E}.",
        ),
        (
            "Guide of Souls",
            "Whenever another creature you control enters, you gain 1 life and get {E}.",
        ),
        (
            "Reservoir Walker",
            "When this creature enters, you gain 3 life and get {E}{E}{E}.",
        ),
        (
            "Rex, Cyber-Hound",
            "Whenever Rex deals combat damage to a player, that player mills two cards and you get {E}{E}.",
        ),
        (
            "Rogue Refiner",
            "When this creature enters, draw a card and you get {E}{E}.",
        ),
        (
            "Woodweaver's Puzzleknot",
            "When this artifact enters, you gain 3 life and get {E}{E}{E}.",
        ),
        (
            "Woodweaver's Puzzleknot",
            "{2}{G}, Sacrifice this artifact: You gain 3 life and get {E}{E}{E}.",
        ),
    ] {
        assert_compiled_clause(card, expected);
    }
}

#[test]
pub(super) fn energy_gain_preserves_aggregate_value_semantics() {
    let definition = parse_oracle_card_definition("Peema Aether-Seer");
    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Peema Aether-Seer must have its enters trigger");
    let energy = triggered
        .effects
        .flattened_default_effects()
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::EnergyCountersEffect>())
        .expect("Peema Aether-Seer must get energy");
    let Value::GreatestPower(filter) = energy.count.unhinted() else {
        panic!(
            "Peema Aether-Seer's energy amount must retain greatest power: {:?}",
            energy.count
        );
    };
    assert_eq!(filter.card_types, vec![CardType::Creature]);
    assert_eq!(filter.controller, Some(PlayerFilter::You));

    assert_compiled_clause(
        "Peema Aether-Seer",
        "When this creature enters, you get an amount of {E} equal to the greatest power among creatures you control.",
    );
}

#[test]
pub(super) fn number_equal_counts_preserve_their_explicit_oracle_surface() {
    for card in [
        "Amzu, Swarm's Hunger",
        "End-Blaze Epiphany",
        "First Responder",
        "Galuf's Final Act",
        "Geometric Nexus",
        "Jenova, Ancient Calamity",
        "Maester Seymour",
        "Primo, the Unbounded",
        "Reverent Hunter",
        "Ruthless Technomancer",
        "Tormented Thoughts",
        "Vincent Valentine // Galian Beast",
    ] {
        assert_number_equal_surface(card);
    }
}

#[test]
pub(super) fn residual_number_equal_cards_preserve_linkage_and_sentence_structure() {
    for (card, expected) in [
        (
            "Amzu, Swarm's Hunger",
            "Whenever one or more cards leave your graveyard, you may create a 1/1 black and green Insect creature token, then put a number of +1/+1 counters on it equal to the greatest mana value among those cards. Do this only once each turn.",
        ),
        (
            "End-Blaze Epiphany",
            "End-Blaze Epiphany deals X damage to target creature. When that creature dies this turn, exile a number of cards from the top of your library equal to its power, then choose a card exiled this way. Until the end of your next turn, you may play that card.",
        ),
        (
            "First Responder",
            "At the beginning of your end step, you may return another creature you control to its owner's hand, then put a number of +1/+1 counters equal to that creature's power on this creature.",
        ),
        (
            "Geometric Nexus",
            "Whenever a player casts an instant or sorcery spell, put a number of charge counters on this artifact equal to that spell's mana value.",
        ),
        (
            "Geometric Nexus",
            "{6}, {T}, Remove all charge counters from this artifact: Create a 0/0 green and blue Fractal creature token. Put X +1/+1 counters on it, where X is the number of charge counters removed this way.",
        ),
        (
            "Primo, the Unbounded",
            "Primo enters with twice X +1/+1 counters on it.",
        ),
        (
            "Primo, the Unbounded",
            "Whenever one or more creatures you control with base power 0 deal combat damage to a player, create a 0/0 green and blue Fractal creature token. Put a number of +1/+1 counters on it equal to the damage dealt.",
        ),
        (
            "Ruthless Technomancer",
            "When this creature enters, you may sacrifice another creature you control. If you do, create a number of Treasure tokens equal to that creature's power.",
        ),
        (
            "Ruthless Technomancer",
            "{2}{B}, Sacrifice X artifacts: Return target creature card with power X or less from your graveyard to the battlefield. X can't be 0.",
        ),
        (
            "Tormented Thoughts",
            "Target player discards a number of cards equal to the sacrificed creature's power.",
        ),
    ] {
        assert_compiled_clause(card, expected);
    }
}

#[test]
pub(super) fn explicit_card_nouns_survive_zone_erasure_and_linked_set_rendering() {
    for (card, expected_fragment) in [
        ("Carrion Locust", "if it was a creature card"),
        ("Dryad Militant", "instant or sorcery card"),
        ("Elite Spellbinder", "exile a nonland card"),
        ("Extraordinary Journey", "creature card exiled this way"),
        ("Planar Void", "another card"),
        ("Gau, Feral Youth", "if a card left your graveyard"),
        ("Gathering Stone", "card of the chosen type"),
        ("Knowledge Exploitation", "instant or sorcery card"),
        ("Malanthrope", "creature card exiled this way"),
        ("Mastermind Plum", "artifact card was exiled this way"),
        ("Narset Transcendent", "noncreature nonland card"),
        ("Seasoned Pyromancer", "nonland card discarded this way"),
        ("Worldly Tutor", "put the card on top"),
    ] {
        assert_oracle_card_parses_strict(card);
        let definition = parse_oracle_card_definition(card);
        let compiled = compiled_text_lines(&definition)
            .join("\n")
            .to_ascii_lowercase();
        assert!(
            compiled.contains(expected_fragment),
            "{card} must retain `{expected_fragment}` after lowering: {compiled}"
        );
    }
}

#[test]
pub(super) fn optional_singular_exile_permission_keeps_the_same_card_link() {
    assert_oracle_card_parses_strict("Conspiracy Theorist");
    let definition = parse_oracle_card_definition("Conspiracy Theorist");
    let compiled = compiled_text_lines(&definition).join("\n");

    assert!(
        compiled.contains("you may pay {1} and discard a card. If you do, draw a card"),
        "{compiled}"
    );
    assert!(
        compiled.contains(
            "You may exile one of them from your graveyard. If you do, you may cast that card this turn"
        ),
        "{compiled}"
    );
}

#[test]
pub(super) fn mana_source_intervening_if_keeps_its_cast_provenance() {
    assert_oracle_card_parses_strict("Mastermind Plum");
    let definition = parse_oracle_card_definition("Mastermind Plum");
    let compiled = compiled_text_lines(&definition).join("\n");
    assert!(
        compiled.contains(
            "Whenever you cast a spell, if mana from a Treasure was spent to cast it, draw a card"
        ),
        "{compiled}"
    );
}

#[test]
pub(super) fn graveyard_exile_count_followup_keeps_its_sentence_boundary() {
    assert_oracle_card_parses_strict("Malanthrope");
    let definition = parse_oracle_card_definition("Malanthrope");
    let compiled = compiled_text_lines(&definition).join("\n");
    assert!(
        compiled.contains(
            "When this creature enters, exile target player's graveyard. Put a +1/+1 counter on this creature for each creature card exiled this way"
        ),
        "{compiled}"
    );
}

#[test]
pub(super) fn targeted_graveyard_card_collection_stays_plural_through_aggregate_and_move() {
    assert_oracle_card_parses_strict("Command the Dreadhorde");
    let definition = parse_oracle_card_definition("Command the Dreadhorde");
    let compiled = compiled_text_lines(&definition).join("\n");

    assert!(
        compiled.contains("target creature and/or planeswalker cards in graveyards"),
        "{compiled}"
    );
    assert!(
        compiled.contains("the total mana value of those cards"),
        "{compiled}"
    );
    assert!(
        compiled.contains("Put them onto the battlefield under your control"),
        "{compiled}"
    );
}
