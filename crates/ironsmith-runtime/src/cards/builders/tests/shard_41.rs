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
pub(super) fn exile_play_permission_and_additional_land_keep_printed_sentences() {
    assert_oracle_card_parses_strict("Sword of Forge and Frontier");
    let definition = parse_oracle_card_definition("Sword of Forge and Frontier");
    let compiled = compiled_text_lines(&definition).join("\n");
    assert!(
        compiled.contains(
            "exile the top two cards of your library. You may play those cards this turn. You may play an additional land this turn"
        ),
        "{compiled}"
    );
}

#[test]
pub(super) fn two_card_hidden_exile_choice_keeps_its_complement_and_permission_link() {
    assert_oracle_card_parses_strict("Siphon Insight");
    let definition = parse_oracle_card_definition("Siphon Insight");
    let compiled = compiled_text_lines(&definition).join("\n");
    assert!(
        compiled.contains(
            "Look at the top two cards of target opponent's library. Exile one of them face down and put the other on the bottom of that library. You may play the exiled card for as long as it remains exiled, and you may spend mana as though it were mana of any color to cast that spell"
        ),
        "{compiled}"
    );
}

#[test]
pub(super) fn optional_hand_exile_keeps_its_permission_and_this_way_tax_linked() {
    assert_oracle_card_parses_strict("Elite Spellbinder");
    let definition = parse_oracle_card_definition("Elite Spellbinder");
    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Elite Spellbinder must have its enters trigger");
    let effects = triggered.effects.flattened_default_effects();
    assert!(
        !effects.iter().any(|effect| effect
            .downcast_ref::<crate::effects::cards::ImprintFromHandEffect>()
            .is_some()),
        "an opponent-hand exile must not use the controller-hand-only imprint executor: {effects:#?}"
    );
    let may = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::MayEffect>())
        .expect("opponent-hand exile must remain optional");
    let [choose_effect, exile_effect] = may.effects.as_slice() else {
        panic!("optional opponent-hand exile must be a linked choose/exile pair: {may:#?}");
    };
    let choose = choose_effect
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()
        .expect("optional exile must choose a card");
    let exile_spec = if let Some(exile) = exile_effect.downcast_ref::<crate::effects::ExileEffect>()
    {
        assert!(!exile.face_down);
        &exile.spec
    } else if let Some(exile) = exile_effect.downcast_ref::<crate::effects::MoveToZoneEffect>() {
        assert_eq!(exile.zone, Zone::Exile);
        assert!(!exile.enters_face_down);
        &exile.target
    } else {
        panic!("optional exile must exile the chosen card: {exile_effect:#?}");
    };
    assert_eq!(choose.chooser, PlayerFilter::You);
    assert_eq!(choose.zone, Some(Zone::Hand));
    assert!(matches!(
        &choose.filter.owner,
        Some(PlayerFilter::Target(_))
    ));
    assert!(matches!(exile_spec.base(), ChooseSpec::Tagged(tag) if tag == &choose.tag));

    let compiled = compiled_text_lines(&definition);
    assert!(
        compiled.iter().any(|line| line
            == "When this creature enters, look at target opponent's hand. You may exile a nonland card from it. For as long as that card remains exiled, its owner may play it. A spell cast this way costs {2} more to cast."),
        "{compiled:#?}"
    );
}

#[test]
pub(super) fn discard_redraw_mana_value_ladder_keeps_distinct_linked_choices() {
    assert_oracle_card_parses_strict("Queen Kayla bin-Kroog");
    let definition = parse_oracle_card_definition("Queen Kayla bin-Kroog");
    let compiled = compiled_text_lines(&definition).join("\n");
    assert!(
        compiled.contains(
            "Discard all the cards in your hand, then draw that many cards. You may choose an artifact or creature card with mana value 1 you discarded this way, then do the same for artifact or creature cards with mana values 2 and 3. Return those cards to the battlefield"
        ),
        "{compiled}"
    );
}

#[test]
pub(super) fn next_spell_grant_keeps_the_spells_cast_origin() {
    assert_oracle_card_parses_strict("Narset Transcendent");
    let definition = parse_oracle_card_definition("Narset Transcendent");
    let compiled = compiled_text_lines(&definition).join("\n");
    assert!(
        compiled.contains(
            "When you next cast an instant or sorcery spell from your hand this turn, it gains rebound"
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

#[test]
pub(super) fn each_opponents_top_card_keeps_hidden_collection_and_play_permission() {
    assert_oracle_card_parses_strict("Mindleecher");
    let definition = parse_oracle_card_definition("Mindleecher");
    let raw = format!("{definition:#?}");
    assert!(
        raw.contains("ForPlayersEffect")
            && raw.contains("chooser: You")
            && raw.contains("top_only: true")
            && raw.contains("face_down: true")
            && raw.contains("GrantPlayTaggedEffect"),
        "Mindleecher must deterministically exile each opponent's top card into one hidden permission collection: {raw}"
    );
    let compiled = compiled_text_lines(&definition);
    assert!(
        compiled.iter().any(|line| line
            == "Whenever this creature mutates, exile the top card of each opponent's library face down. You may look at and play those cards for as long as they remain exiled."),
        "{compiled:#?}"
    );
}

#[test]
pub(super) fn mass_exile_deploy_and_return_keeps_one_linked_collection_program() {
    assert_compiled_clause(
        "Worlds Within Worlds",
        "Exile all creatures. Each player may put any number of creature cards from their hand onto the battlefield. Then put all cards exiled this way into their owners' hands. Exile Worlds Within Worlds.",
    );
}
