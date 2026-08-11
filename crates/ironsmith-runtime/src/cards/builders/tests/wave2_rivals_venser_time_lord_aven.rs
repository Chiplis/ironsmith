#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

#[test]
fn rivals_duel_public_card_keeps_correlated_fight_surface() {
    let definition = parse_oracle_card_definition("Rivals' Duel");
    assert_eq!(
        canonical_compiled_lines(&definition),
        [
            "Choose two target creatures that share no creature types. Those creatures fight each other."
        ]
    );

    let debug = format!(
        "{:#?}",
        definition.spell_effect.as_ref().expect("spell program")
    );
    assert!(debug.contains("distinct_creature_types: true"), "{debug}");
    assert!(debug.contains("FightEffect"), "{debug}");
}

#[test]
fn vensers_diffusion_public_card_keeps_both_disjoint_target_domains() {
    let definition = parse_oracle_card_definition("Venser's Diffusion");
    assert_eq!(
        canonical_compiled_lines(&definition),
        ["Return target nonland permanent or suspended card to its owner's hand."]
    );

    let debug = format!(
        "{:#?}",
        definition.spell_effect.as_ref().expect("spell program")
    );
    assert!(debug.contains("excluded_card_types"), "{debug}");
    assert!(debug.contains("Land"), "{debug}");
    assert!(debug.contains("Suspend"), "{debug}");
    assert!(debug.contains("Time"), "{debug}");
}

#[test]
fn time_lord_regeneration_keeps_typed_target_and_nested_time_lord_card() {
    const ORACLE: &str = "Until end of turn, target Time Lord you control gains \"When this creature dies, reveal cards from the top of your library until you reveal a Time Lord creature card. Put that card onto the battlefield and the rest on the bottom of your library in a random order.\"";
    let definition = parse_oracle_card_definition("Time Lord Regeneration");
    assert_eq!(canonical_compiled_lines(&definition), [ORACLE]);

    let debug = format!(
        "{:#?}",
        definition.spell_effect.as_ref().expect("spell program")
    );
    assert!(debug.contains("ApplyContinuousEffect"), "{debug}");
    assert!(debug.matches("TimeLord").count() >= 2, "{debug}");
    assert!(debug.contains("ConsultTopOfLibraryEffect"), "{debug}");
    assert!(debug.contains("EndOfTurn"), "{debug}");
}

#[test]
fn aven_windreader_public_card_keeps_explicit_revealing_player_subject() {
    let definition = parse_oracle_card_definition("Aven Windreader");
    assert_eq!(
        canonical_compiled_lines(&definition),
        [
            "Flying",
            "{1}{U}: Target player reveals the top card of their library.",
        ]
    );

    let activated = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("Aven Windreader must keep its activated reveal ability");
    let effects = activated.effects.flattened_default_effects();
    let [target_effect, reveal_effect] = effects else {
        panic!("expected one target declaration and one reveal effect: {effects:#?}");
    };
    let target = target_effect
        .downcast_ref::<crate::effects::TargetOnlyEffect>()
        .expect("typed player target declaration");
    assert_eq!(
        target.target,
        ChooseSpec::Target(Box::new(ChooseSpec::Player(PlayerFilter::Any)))
    );
    let reveal = reveal_effect
        .downcast_ref::<crate::effects::RevealTopEffect>()
        .expect("typed reveal-top effect");
    assert_eq!(
        reveal.player,
        PlayerFilter::Target(Box::new(PlayerFilter::Any))
    );
}
