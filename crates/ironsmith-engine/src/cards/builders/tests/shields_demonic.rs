#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const SHIELDS_EFFECT: &str =
    "Creatures target player controls get +0/+1 and gain all creature types until end of turn.";
const DEMONIC_ORACLE: &str = "At the beginning of your end step, if you control exactly one creature, create a 5/5 black Demon creature token with flying.";
const EGO_EFFECT: &str =
    "Creatures target player controls get -2/-0 and lose all creature types until end of turn.";

fn end_step(player: PlayerId) -> crate::triggers::TriggerEvent {
    crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::phase::BeginningOfEndStepEvent::new(player),
        crate::provenance::ProvNodeId::default(),
    )
}

#[test]
fn shields_of_velis_vel_keeps_the_shared_target_set_and_all_creature_types() {
    let definition = parse_oracle_card_definition("Shields of Velis Vel");
    let lines = canonical_compiled_lines(&definition);
    assert!(lines.iter().any(|line| line == "Changeling"), "{lines:#?}");
    assert!(
        lines.iter().any(|line| line == SHIELDS_EFFECT),
        "{lines:#?}"
    );

    let program = definition
        .spell_effect
        .as_ref()
        .expect("Shields should have a spell program");
    let debug = format!("{program:#?}");
    assert!(debug.contains("ModifyPowerToughness"), "{debug}");
    assert!(debug.contains("AddAllSubtypesOfFamily"), "{debug}");
    assert!(
        debug.contains("controller: Some") && debug.contains("Target"),
        "{debug}"
    );
}

#[test]
fn demonic_rising_requires_exactly_one_creature_at_trigger_time() {
    let definition = parse_oracle_card_definition("Demonic Rising");
    assert_eq!(canonical_compiled_lines(&definition), vec![DEMONIC_ORACLE]);

    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Demonic Rising should have an end-step trigger");
    let crate::effect::Condition::PlayerControlsExactly {
        player,
        filter,
        count,
    } = triggered
        .intervening_if
        .as_ref()
        .expect("the exact-one clause must remain an intervening condition")
    else {
        panic!("expected exact creature count: {triggered:#?}");
    };
    assert_eq!(*player, PlayerFilter::You);
    assert_eq!(*count, 1);
    assert_eq!(filter.card_types, vec![CardType::Creature]);

    let alice = PlayerId::from_index(0);
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let source_trigger_count = |game: &crate::GameState| {
        crate::triggers::check_triggers(game, &end_step(alice))
            .into_iter()
            .filter(|entry| entry.source == source)
            .count()
    };
    assert_eq!(source_trigger_count(&game), 0);

    let creature = CardDefinitionBuilder::new(CardId::new(), "Only Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    game.create_object_from_definition(&creature, alice, Zone::Battlefield);
    assert_eq!(source_trigger_count(&game), 1);

    game.create_object_from_definition(&creature, alice, Zone::Battlefield);
    assert_eq!(source_trigger_count(&game), 0);
}

#[test]
fn ego_erasure_keeps_the_shared_target_set_and_creature_type_loss() {
    let definition = parse_oracle_card_definition("Ego Erasure");
    let lines = canonical_compiled_lines(&definition);
    assert!(lines.iter().any(|line| line == "Changeling"), "{lines:#?}");
    assert!(lines.iter().any(|line| line == EGO_EFFECT), "{lines:#?}");

    let program = definition
        .spell_effect
        .as_ref()
        .expect("Ego Erasure should have a spell program");
    let debug = format!("{program:#?}");
    assert!(debug.contains("ModifyPowerToughness"), "{debug}");
    assert!(debug.contains("RemoveAllSubtypesOfFamily"), "{debug}");
    assert!(
        debug.contains("controller: Some") && debug.contains("Target"),
        "{debug}"
    );
}
