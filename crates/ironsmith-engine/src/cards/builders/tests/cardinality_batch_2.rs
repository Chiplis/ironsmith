#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

fn canonical_text(name: &str) -> String {
    canonical_compiled_lines(&parse_oracle_card_definition(name)).join("\n")
}

#[test]
fn on_wings_of_gold_merges_the_shared_relative_subject() {
    assert_eq!(
        canonical_text("On Wings of Gold"),
        "Creatures you control that are Zombies and/or tokens get +1/+1 and have flying.\nWhenever one or more cards leave your graveyard, create a 1/1 white Zombie creature token."
    );
}

#[test]
fn turf_war_collection_choice_renders_without_internal_tags() {
    assert_eq!(
        canonical_text("Turf War"),
        "When this enchantment enters, for each player, put a contested counter on target land that player controls.\nWhenever a creature deals combat damage to a player, if that player controls one or more lands with contested counters on them, that creature's controller gains control of one of those lands of their choice and untaps it."
    );
}

#[test]
fn vulpine_harvester_binds_the_target_metric_to_the_attack_group() {
    assert_eq!(
        canonical_text("Vulpine Harvester"),
        "Whenever one or more Phyrexians you control attack, return target artifact card from your graveyard to the battlefield if its mana value is less than or equal to their total power."
    );
}

#[test]
fn yarus_keeps_the_resolution_gate_between_return_and_turn() {
    assert_eq!(
        canonical_text("Yarus, Roar of the Old Gods"),
        "Other creatures you control have haste.\nWhenever one or more face-down creatures you control deal combat damage to a player, draw a card.\nWhenever a face-down creature you control dies, return it to the battlefield face down under its owner's control if it's a permanent card, then turn it face up."
    );
}

#[test]
fn frozen_cardinality_batch_two_renders_exact_oracle_semantics() {
    for (name, expected) in [
        (
            "On Wings of Gold",
            "Creatures you control that are Zombies and/or tokens get +1/+1 and have flying.\nWhenever one or more cards leave your graveyard, create a 1/1 white Zombie creature token.",
        ),
        (
            "Tocasia's Welcome",
            "Whenever one or more creatures you control with mana value 3 or less enter, draw a card. This ability triggers only once each turn.",
        ),
        (
            "All Is Dust",
            "Each player sacrifices all permanents they control that are one or more colors.",
        ),
        (
            "Turf War",
            "When this enchantment enters, for each player, put a contested counter on target land that player controls.\nWhenever a creature deals combat damage to a player, if that player controls one or more lands with contested counters on them, that creature's controller gains control of one of those lands of their choice and untaps it.",
        ),
        (
            "Ugin's Construct",
            "When this creature enters, sacrifice a permanent that's one or more colors.",
        ),
        (
            "Vulpine Harvester",
            "Whenever one or more Phyrexians you control attack, return target artifact card from your graveyard to the battlefield if its mana value is less than or equal to their total power.",
        ),
        (
            "Yarus, Roar of the Old Gods",
            "Other creatures you control have haste.\nWhenever one or more face-down creatures you control deal combat damage to a player, draw a card.\nWhenever a face-down creature you control dies, return it to the battlefield face down under its owner's control if it's a permanent card, then turn it face up.",
        ),
        (
            "Backdraft",
            "Choose a player who cast one or more sorcery spells this turn. Backdraft deals damage to that player equal to half the damage dealt by one of those sorcery spells this turn, rounded down.",
        ),
    ] {
        assert_eq!(
            canonical_text(name),
            expected,
            "{name} should preserve the complete frozen-set oracle surface"
        );
    }
}

#[test]
fn frozen_repeat_counts_keep_their_for_each_surface() {
    for (name, expected) in [
        (
            "Recall",
            "Discard X cards, then return a card from your graveyard to your hand for each card discarded this way. Exile Recall.",
        ),
        (
            "Rofellos's Gift",
            "Reveal any number of green cards in your hand. Return an enchantment card from your graveyard to your hand for each card revealed this way.",
        ),
    ] {
        assert_eq!(
            canonical_text(name),
            expected,
            "{name} should keep one repeated action per affected card"
        );
    }
}

#[test]
fn vulpine_harvester_uses_the_exact_triggering_attack_group_power() {
    let definition = parse_oracle_card_definition("Vulpine Harvester");
    let debug = format!("{:#?}", definition.abilities);
    assert!(
        debug.contains("TotalPower") && debug.contains("__attacking_group"),
        "Vulpine Harvester should compare mana value with the captured attack group's total power; got {debug}"
    );
    assert!(
        debug.contains("ConditionalEffect"),
        "Vulpine Harvester should retain its resolution-time mana-value gate; got {debug}"
    );
}

#[test]
fn yarus_returns_face_down_then_turns_the_same_object_face_up() {
    let definition = parse_oracle_card_definition("Yarus, Roar of the Old Gods");
    let debug = format!("{:#?}", definition.abilities);
    assert!(
        debug.contains("enters_face_down: true"),
        "Yarus should return the dying permanent card face down; got {debug}"
    );
    assert!(
        debug.contains("TurnFaceUpEffect"),
        "Yarus should turn the returned object face up after the conditional return; got {debug}"
    );
}

#[test]
fn turf_war_lets_the_damaged_player_choose_a_contested_land() {
    let definition = parse_oracle_card_definition("Turf War");
    let debug = format!("{:#?}", definition.abilities);
    assert!(
        debug.contains("ChooseObjectsEffect")
            && debug.contains("chooser: DamagedPlayer")
            && debug.contains("Named(\n")
            && debug.contains("\"contested\""),
        "Turf War should ask the damaged player to choose a contested land they control; got {debug}"
    );
    assert!(
        debug.contains("ChangeControllerToPlayer")
            && debug.contains("ControllerOf")
            && debug.contains("\"triggering\""),
        "the damaging creature's controller should gain control of the chosen land; got {debug}"
    );
}
