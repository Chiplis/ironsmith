//! Named quotas share exact-object scope across players and retain native rollback.
use ironsmith::{GameState, PlayerId, Zone};
use ironsmith::action_use_receipts::{ActionUseKey, consume_this_turn, remaining_this_turn};
#[test]
fn objectwide_and_per_ability_allowances_are_independent_and_reset_only_by_turn_or_incarnation() {
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    let definition = ironsmith_compiler_runtime::compile_to_runtime_definition("Quota witness", "Type: Artifact\n{0}: You gain 1 life.", false).unwrap();
    let source = game.create_object_from_definition(&definition, PlayerId(0), Zone::Battlefield);
    let shared = ActionUseKey { source, ability: None, definition: None, group: "marking".into() };
    let ability0 = ActionUseKey { source, ability: Some(ironsmith::continuous::AbilityOrigin::Printed(0)), definition: None, group: "alternate".into() };
    let ability1 = ActionUseKey { source, ability: Some(ironsmith::continuous::AbilityOrigin::Printed(1)), definition: None, group: "alternate".into() };
    let checkpoint = game.clone();
    consume_this_turn(&mut game, &shared, 1).unwrap();
    assert!(consume_this_turn(&mut game, &shared, 1).is_err());
    game.set_current_controller(source, PlayerId(1)).unwrap();
    assert_eq!(remaining_this_turn(&game, &shared, 1), 0, "controller changes do not replenish a shared permanent budget");
    consume_this_turn(&mut game, &ability0, 1).unwrap();
    assert_eq!(remaining_this_turn(&game, &ability1, 1), 1);
    let transformed_ability = ActionUseKey { definition: Some(ironsmith_core::LinkedExileDefinition([7;32])), ..ability0.clone() };
    assert_eq!(remaining_this_turn(&game, &transformed_ability, 1), 1, "a changed typed ability definition is independent even at the same printed index");
    game = checkpoint;
    assert_eq!(remaining_this_turn(&game, &shared, 1), 1, "native checkpoint rollback restores the selected-use receipt");
    consume_this_turn(&mut game, &shared, 1).unwrap();
    game.turn.turn_number += 1;
    assert_eq!(remaining_this_turn(&game, &shared, 1), 1);
    let other = game.create_object_from_definition(&definition, PlayerId(0), Zone::Battlefield);
    let new_object = ActionUseKey { source: other, ..shared.clone() };
    assert_eq!(remaining_this_turn(&game, &new_object, 1), 1);
}
