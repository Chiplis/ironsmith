#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;
use crate::effect::{Effect, Until};
use crate::effects::{ExecutionContext, GrantAbilitiesTargetEffect, execute_effect};
use crate::static_abilities::StaticAbility;

#[test]
fn drop_tower_preserves_the_compound_roll_linked_duration() {
    let oracle = "Visit — Target creature gains flying until end of turn, or until any player rolls a 1, whichever comes first.";
    let definition = parse_oracle_card_definition("Drop Tower");

    assert_eq!(compiled_text_lines(&definition).join("\n"), oracle);

    let debug = format!("{definition:#?}");
    assert!(
        debug.contains("EndOfTurnOrAnyPlayerRolls")
            && debug.contains("result: 1")
            && debug.contains("Flying"),
        "the duration and granted ability must remain structural: {debug}"
    );
}

#[test]
fn compound_roll_linked_duration_ignores_old_rolls_and_expires_on_a_new_matching_roll() {
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let source_definition = CardDefinitionBuilder::new(CardId::new(), "Duration Source")
        .card_types(vec![CardType::Artifact])
        .build();
    let target_definition = CardDefinitionBuilder::new(CardId::new(), "Duration Target")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let source = game.create_object_from_definition(&source_definition, alice, Zone::Battlefield);
    let target = game.create_object_from_definition(&target_definition, alice, Zone::Battlefield);

    // A matching result from earlier in the turn does not satisfy "until any
    // player rolls" because the duration starts only when this effect resolves.
    game.turn_store.turn_history.record_die_roll(alice, 1);

    let grant = GrantAbilitiesTargetEffect::new(
        ChooseSpec::SpecificObject(target),
        [StaticAbility::flying()],
        Until::EndOfTurnOrAnyPlayerRolls {
            result: 1,
            matching_rolls_observed: 0,
        },
    );
    let mut ctx = ExecutionContext::new_default(source, alice);
    execute_effect(&mut game, &Effect::new(grant), &mut ctx)
        .expect("the compound-duration grant should resolve");

    assert!(
        game.object_has_static_ability_id(target, StaticAbilityId::Flying),
        "a matching roll before resolution must not expire the grant"
    );

    game.force_next_die_roll(2);
    let mut bob_ctx = ExecutionContext::new_default(source, bob);
    execute_effect(
        &mut game,
        &Effect::roll_die(6, PlayerFilter::You),
        &mut bob_ctx,
    )
    .expect("the opponent's nonmatching roll should resolve");
    assert!(
        game.object_has_static_ability_id(target, StaticAbilityId::Flying),
        "a nonmatching roll by an opponent must not expire the grant"
    );

    game.force_next_die_roll(1);
    execute_effect(
        &mut game,
        &Effect::roll_die(6, PlayerFilter::You),
        &mut bob_ctx,
    )
    .expect("the opponent's matching roll should resolve");
    assert!(
        !game.object_has_static_ability_id(target, StaticAbilityId::Flying),
        "a new matching roll by any player must expire the grant"
    );
}
