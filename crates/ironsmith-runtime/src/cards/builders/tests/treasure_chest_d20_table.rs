#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const ORACLE: &str = "{4}, Sacrifice this artifact: Roll a d20.\n1 | Trapped! — You lose 3 life.\n2—9 | Create five Treasure tokens.\n10—19 | You gain 3 life and draw three cards.\n20 | Search your library for a card. If it's an artifact card, you may put it onto the battlefield. Otherwise, put that card into your hand. Then shuffle.";

fn activated_program(definition: &CardDefinition) -> &crate::resolution::ResolutionProgram {
    definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(&activated.effects),
            _ => None,
        })
        .expect("Treasure Chest must retain its activated result table")
}

#[test]
fn treasure_chest_keeps_the_exact_numeric_table_and_named_first_row() {
    let definition = parse_oracle_card_definition("Treasure Chest");
    assert_eq!(canonical_compiled_lines(&definition).join("\n"), ORACLE);
    assert!(
        definition.spell_effect.is_none(),
        "numeric rows must stay with the activation"
    );
    let debug = format!("{:#?}", activated_program(&definition));
    assert!(debug.contains("RollDieEffect"), "{debug}");
    assert!(
        debug.contains("result_label") && debug.contains("Trapped!"),
        "{debug}"
    );
    assert!(
        debug.contains("BetweenInclusive(\n") && debug.contains("Equal(\n"),
        "{debug}"
    );
}

fn resolve_forced_result(roll: u32) -> (i32, usize) {
    let definition = parse_oracle_card_definition("Treasure Chest");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    game.force_next_die_roll(roll);
    let mut ctx = crate::effects::ExecutionContext::new_default(source, alice);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        source,
        activated_program(&definition),
        None,
        &[],
    )
    .expect("the activated result table should resolve");
    let treasures = game
        .objects_in_zone(Zone::Battlefield)
        .into_iter()
        .filter(|id| {
            game.controller_of_id(*id) == Some(alice)
                && game
                    .object(*id)
                    .is_some_and(|object| object.subtypes.contains(&Subtype::Treasure))
        })
        .count();
    (game.player(alice).expect("Alice").life, treasures)
}

#[test]
fn treasure_chest_numeric_predicates_execute_only_the_forced_result_row() {
    assert_eq!(resolve_forced_result(1), (17, 0));
    assert_eq!(resolve_forced_result(2), (20, 5));
}
