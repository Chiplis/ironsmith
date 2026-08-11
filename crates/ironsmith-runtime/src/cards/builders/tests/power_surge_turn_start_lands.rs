#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const ORACLE: &str = "At the beginning of each player's upkeep, this enchantment deals X damage to that player, where X is the number of untapped lands they controlled at the beginning of this turn.";

fn land(name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Land])
        .build()
}

fn triggered_program(definition: &CardDefinition) -> &crate::resolution::ResolutionProgram {
    definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(&triggered.effects),
            _ => None,
        })
        .expect("Power Surge must retain its upkeep trigger")
}

#[test]
fn power_surge_keeps_the_turn_start_land_snapshot_value() {
    let definition = parse_oracle_card_definition("Power Surge");
    assert_eq!(canonical_compiled_lines(&definition), vec![ORACLE]);
    assert!(
        format!("{definition:#?}").contains("UntappedLandsAtTurnStart"),
        "{definition:#?}"
    );
}

#[test]
fn power_surge_uses_the_pre_untap_snapshot_not_current_land_state() {
    let definition = parse_oracle_card_definition("Power Surge");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let first = game.create_object_from_definition(&land("First Mine"), bob, Zone::Battlefield);
    let second = game.create_object_from_definition(&land("Second Mine"), bob, Zone::Battlefield);
    let tapped = game.create_object_from_definition(&land("Tapped Mine"), bob, Zone::Battlefield);
    game.tap(tapped);

    game.record_turn_start_hand_sizes();
    game.tap(first);
    game.tap(second);
    game.untap(tapped);

    let mut ctx = crate::effects::ExecutionContext::new_default(source, alice);
    ctx.iteration.iterated_player = Some(bob);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        source,
        triggered_program(&definition),
        None,
        &[],
    )
    .expect("Power Surge upkeep damage should resolve");

    assert_eq!(game.player(bob).expect("Bob").life, 18);
}
