use super::shard_16::{assert_oracle_card_parses_strict, parse_oracle_card_definition};
use super::*;

fn assert_compiled_line(card: &str, expected: &str) {
    assert_oracle_card_parses_strict(card);
    let definition = parse_oracle_card_definition(card);
    let compiled = compiled_text_lines(&definition);
    assert!(
        compiled.iter().any(|line| line == expected),
        "{card} must preserve its complete oracle clause: {compiled:#?}"
    );
}

fn assert_compiled_clause(card: &str, expected: &str) {
    assert_oracle_card_parses_strict(card);
    let definition = parse_oracle_card_definition(card);
    let compiled = compiled_text_lines(&definition);
    assert!(
        compiled
            .iter()
            .any(|line| line == expected || line.ends_with(&format!(" — {expected}"))),
        "{card} must preserve its complete oracle clause: {compiled:#?}"
    );
}

fn assert_compiled_fragment(card: &str, expected: &str) {
    assert_oracle_card_parses_strict(card);
    let definition = parse_oracle_card_definition(card);
    let compiled = compiled_text_lines(&definition);
    assert!(
        compiled.iter().any(|line| line.contains(expected)),
        "{card} must preserve its comparison surface and value subject ({expected:?}): {compiled:#?}"
    );
}

#[test]
pub(super) fn linked_same_name_continuous_effects_render_as_one_set() {
    for (card, expected) in [
        (
            "Bile Blight",
            "Target creature and all other creatures with the same name as that creature get -3/-3 until end of turn.",
        ),
        (
            "Echoing Courage",
            "Target creature and all other creatures with the same name as that creature get +2/+2 until end of turn.",
        ),
        (
            "Echoing Decay",
            "Target creature and all other creatures with the same name as that creature get -2/-2 until end of turn.",
        ),
    ] {
        assert_compiled_line(card, expected);
    }
}

#[test]
pub(super) fn radiance_continuous_effects_keep_the_keyword_and_compound_subject() {
    for (card, tail) in [
        ("Surge of Zeal", "gain haste until end of turn"),
        ("Wojek Siren", "get +1/+1 until end of turn"),
    ] {
        assert_compiled_line(
            card,
            &format!(
                "Radiance — Target creature and each other creature that shares a color with it {tail}."
            ),
        );
    }
}

#[test]
pub(super) fn linked_same_name_zone_actions_render_as_one_set() {
    assert_compiled_line(
        "Echoing Return",
        "Return target creature card and all other cards with the same name as that card from your graveyard to your hand.",
    );
    assert_compiled_line(
        "Rat King, Verminister",
        "{T}, Sacrifice three Rats: Return target creature card and all other cards with the same name as that card from your graveyard to the battlefield tapped.",
    );
    assert_compiled_line(
        "Declaration in Stone",
        "Exile target creature and all other creatures its controller controls with the same name as that creature. That player investigates for each nontoken creature exiled this way.",
    );
}

#[test]
pub(super) fn linked_set_followups_refer_to_the_union_of_target_and_fanout() {
    assert_compiled_line(
        "Rally the Righteous",
        "Radiance — Untap target creature and each other creature that shares a color with it. Those creatures get +2/+0 until end of turn.",
    );
    assert_compiled_line(
        "Legion's End",
        "Exile target creature an opponent controls with mana value 2 or less and all other creatures that player controls with the same name as that creature. Then that player reveals their hand and exiles all cards with that name from their hand and graveyard.",
    );
    assert_compiled_line(
        "Moratorium Stone",
        "{2}{W}{B}, {T}, Sacrifice this artifact: Exile target nonland card from a graveyard, all other cards from graveyards with the same name as that card, and all permanents with that name.",
    );
}

#[test]
pub(super) fn prefixed_d20_tables_preserve_their_numeric_rows() {
    for (card, expected) in [
        (
            "Aberrant Mind Sorcerer",
            "When this creature enters, choose target instant or sorcery card in your graveyard, then roll a d20.\n1—9 | You may put that card on top of your library.\n10—20 | Return that card to your hand.",
        ),
        (
            "Contraband Livestock",
            "Exile target creature, then roll a d20.\n1—9 | Its controller creates a 4/4 green Ox creature token.\n10—19 | Its controller creates a 2/2 green Boar creature token.\n20 | Its controller creates a 0/1 white Goat creature token.",
        ),
        (
            "Farideh's Fireball",
            "Farideh's Fireball deals 5 damage to target creature or planeswalker. Roll a d20.\n1—9 | Farideh's Fireball deals 2 damage to each player.\n10—20 | Farideh's Fireball deals 2 damage to each opponent.",
        ),
        (
            "Mathise, Surge Channeler",
            "Whenever you cast an instant or sorcery spell with mana value 3 or greater, roll a d20.\n1—9 | Each player draws a card.\n10—19 | You draw a card.\n20 | Copy that spell. You may choose new targets for the copy.",
        ),
        (
            "Overwhelming Encounter",
            "Creatures you control gain vigilance and trample until end of turn. Roll a d20.\n1—9 | Creatures you control get +2/+2 until end of turn.\n10—19 | Put two +1/+1 counters on each creature you control.\n20 | Put four +1/+1 counters on each creature you control.",
        ),
        (
            "Power of Persuasion",
            "Choose target creature an opponent controls, then roll a d20.\n1—9 | Return it to its owner's hand.\n10—19 | Its owner puts it on their choice of the top or bottom of their library.\n20 | Gain control of it until the end of your next turn.",
        ),
        (
            "Scion of Stygia",
            "When this creature enters, choose target creature an opponent controls, then roll a d20.\n1—9 | Tap that creature.\n10—20 | Tap that creature. It doesn't untap during its controller's next untap step.",
        ),
        (
            "Spiked Pit Trap",
            "{5}, {T}, Sacrifice this artifact: Choose target creature, then roll a d20.\n1—9 | This artifact deals 5 damage to that creature.\n10—20 | This artifact deals 5 damage to that creature. Create a Treasure token.",
        ),
    ] {
        assert_compiled_clause(card, expected);
    }
}

#[test]
pub(super) fn laezels_acrobatics_reexiles_only_the_high_roll_returned_set() {
    assert_compiled_clause(
        "Lae'zel's Acrobatics",
        "Exile all nontoken creatures you control, then roll a d20.\n1—9 | Return those cards to the battlefield under their owner's control at the beginning of the next end step.\n10—20 | Return those cards to the battlefield under their owner's control, then exile them again. Return those cards to the battlefield under their owner's control at the beginning of the next end step.",
    );
}

#[test]
pub(super) fn coin_flip_outcomes_stay_bound_to_the_flip_that_produced_them() {
    for (card, expected) in [
        (
            "Chaotic Goo",
            "At the beginning of your upkeep, you may flip a coin. If you win the flip, put a +1/+1 counter on this creature. If you lose the flip, remove a +1/+1 counter from this creature.",
        ),
        (
            "Fighting Chance",
            "For each blocking creature, flip a coin. If you win the flip, prevent all combat damage that would be dealt by that creature this turn.",
        ),
        (
            "Goblin Bomb",
            "At the beginning of your upkeep, you may flip a coin. If you win the flip, put a fuse counter on this enchantment. If you lose the flip, remove a fuse counter from this enchantment.",
        ),
        (
            "Invert Polarity",
            "Choose target spell, then flip a coin. If you win the flip, gain control of that spell and you may choose new targets for it. If you lose the flip, counter that spell.",
        ),
        (
            "Krark, the Thumbless",
            "Whenever you cast an instant or sorcery spell, flip a coin. If you lose the flip, return that spell to its owner's hand. If you win the flip, copy that spell, and you may choose new targets for the copy.",
        ),
        (
            "Krark, the Thumbless // Krark, the Thumbless",
            "Whenever you cast an instant or sorcery spell, flip a coin. If you lose the flip, return that spell to its owner's hand. If you win the flip, copy that spell, and you may choose new targets for the copy.",
        ),
        (
            "Mijae Djinn",
            "Whenever this creature attacks, flip a coin. If you lose the flip, remove this creature from combat and tap it.",
        ),
    ] {
        assert_compiled_line(card, expected);
    }
}

#[test]
pub(super) fn dynamic_filter_comparisons_preserve_explicit_oracle_surface() {
    for (card, expected) in [
        (
            "Blazing Hope",
            "power greater than or equal to your life total",
        ),
        (
            "Carmen, Cruel Skymarcher",
            "mana value less than or equal to Carmen's power",
        ),
        (
            "Dimension X Pizzasaur",
            "mana value less than or equal to the number of counters among permanents you control",
        ),
        (
            "Dominating Vampire",
            "mana value less than or equal to the number of Vampires you control",
        ),
        (
            "Engulf the Shore",
            "toughness less than or equal to the number of Islands you control",
        ),
        (
            "Fiendish Panda",
            "mana value less than or equal to this creature's power",
        ),
        (
            "Glamdring",
            "mana value less than or equal to that damage",
        ),
        (
            "Hammerhead Tyrant",
            "mana value less than or equal to that spell's mana value",
        ),
        (
            "Lady Octopus, Inspired Inventor",
            "mana value less than or equal to the number of ingenuity counters on Lady Octopus",
        ),
        (
            "Lay Down Arms",
            "mana value less than or equal to the number of Plains you control",
        ),
        (
            "Spawnbroker",
            "power less than or equal to that creature's power",
        ),
        (
            "Squirming Emergence",
            "mana value less than or equal to the number of permanent cards in your graveyard",
        ),
    ] {
        assert_compiled_fragment(card, expected);
    }

    assert_compiled_line(
        "Carmen, Cruel Skymarcher",
        "Whenever a player sacrifices a permanent, put a +1/+1 counter on Carmen and you gain 1 life.",
    );
}

#[test]
pub(super) fn dynamic_comparison_cards_do_not_leak_where_x_or_postfix_surfaces() {
    for (card, expected) in [
        (
            "Carmen, Cruel Skymarcher",
            "Whenever Carmen attacks, return up to one target permanent card with mana value less than or equal to Carmen's power from your graveyard to the battlefield.",
        ),
        (
            "Engulf the Shore",
            "Return all creatures with toughness less than or equal to the number of Islands you control to their owners' hands.",
        ),
        (
            "Glamdring",
            "Whenever equipped creature deals combat damage to a player, you may cast an instant or sorcery spell with mana value less than or equal to that damage from your hand without paying its mana cost.",
        ),
        (
            "Lady Octopus, Inspired Inventor",
            "{T}: You may cast an artifact spell with mana value less than or equal to the number of ingenuity counters on Lady Octopus from your hand without paying its mana cost.",
        ),
        (
            "Squirming Emergence",
            "Return target nonland permanent card with mana value less than or equal to the number of permanent cards in your graveyard from your graveyard to the battlefield.",
        ),
    ] {
        assert_compiled_line(card, expected);
    }
    assert_compiled_line(
        "Glamdring",
        "Equipped creature has first strike and gets +1/+0 for each instant or sorcery card in your graveyard.",
    );
}

#[test]
pub(super) fn numbered_draw_trigger_preserves_first_or_second_semantics() {
    assert_compiled_line(
        "Lady Octopus, Inspired Inventor",
        "Whenever you draw your first or second card each turn, put an ingenuity counter on Lady Octopus.",
    );
}
