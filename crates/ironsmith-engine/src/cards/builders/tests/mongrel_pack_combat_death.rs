#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const ORACLE: &str =
    "When this creature dies during combat, create four 1/1 green Dog creature tokens.";

fn death_event(game: &crate::GameState, source: ObjectId) -> crate::triggers::TriggerEvent {
    let snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(source).expect("Mongrel Pack exists"),
        game,
    );
    crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::zones::ZoneChangeEvent::with_cause(
            source,
            Zone::Battlefield,
            Zone::Graveyard,
            crate::events::cause::EventCause::effect(),
            Some(snapshot.clone()),
        ),
        crate::provenance::ProvNodeId::default(),
    )
    .with_lookback_source_snapshots(vec![snapshot])
}

#[test]
fn mongrel_pack_keeps_typed_during_combat_death_surface() {
    let definition = parse_oracle_card_definition("Mongrel Pack");
    assert_eq!(canonical_compiled_lines(&definition).join("\n"), ORACLE);
    let trigger = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => triggered
                .trigger
                .downcast_ref::<crate::triggers::zone_changes::ZoneChangeTrigger>(
            ),
            _ => None,
        })
        .expect("Mongrel Pack has a typed death trigger");
    assert_eq!(
        trigger.timing,
        Some(ironsmith_core::TriggerTimingRestriction::DuringCombat)
    );
}

#[test]
fn mongrel_pack_death_matches_only_during_combat() {
    let definition = parse_oracle_card_definition("Mongrel Pack");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let event = death_event(&game, source);

    game.turn.phase = crate::game_state::Phase::FirstMain;
    assert!(
        crate::triggers::check_triggers(&game, &event)
            .into_iter()
            .all(|entry| entry.source != source),
        "a main-phase death must not trigger"
    );

    game.turn.phase = crate::game_state::Phase::Combat;
    assert_eq!(
        crate::triggers::check_triggers(&game, &event)
            .into_iter()
            .filter(|entry| entry.source == source)
            .count(),
        1,
        "the same death event during combat must trigger exactly once"
    );
}
