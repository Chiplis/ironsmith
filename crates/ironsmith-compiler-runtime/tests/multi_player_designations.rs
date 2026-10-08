//! UNVALIDATED implementation-first coverage (cf8 p09): "As this creature
//! enters, choose two players." records both players on the permanent; "one
//! of the chosen players" and "the other chosen player" read that pair
//! (Sower of Discord).
use ironsmith::cards::CardDefinition;
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::effect::Effect;
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::game_loop::{put_triggers_on_stack_with_dm, resolve_stack_entry_with};
use ironsmith::target::ChooseSpec;
use ironsmith::triggers::{TriggerQueue, check_triggers};
use ironsmith::{GameState, PlayerId, Zone};

#[path = "p09_common/mod.rs"]
mod common;

const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);
const C: PlayerId = PlayerId::from_index(2);

fn rows() -> Vec<serde_json::Value> {
    common::rows(include_str!("../../../fixtures/multi_player_designations.json.fixture"))
}

fn sower() -> [CardDefinition; 2] {
    common::definitions(common::row(&rows(), "Sower of Discord"))
}

#[test]
fn sower_of_discord_chooses_two_players_and_reads_the_pair() {
    for definition in sower() {
        let debug = format!("{:?}", definition.abilities);
        assert!(debug.contains("ChoosePlayerAsEnters"), "{debug}");
        assert!(debug.contains("count: 2"), "{debug}");
        assert!(debug.contains("__source_chosen_players__"), "{debug}");
        assert!(debug.contains("DamagedPlayer"), "{debug}");
    }
}

#[test]
fn damage_to_one_chosen_player_makes_the_other_lose_that_much_life() {
    for definition in sower() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20);
        let sower = game.create_object_from_definition(&definition, C, Zone::Battlefield);
        game.set_chosen_players(sower, vec![A, B]);
        let source = game.new_object_id();
        let mut dm = SelectFirstDecisionMaker;
        let outcome = {
            let mut ctx = EffectContext::new(source, C, &mut dm);
            execute_effect(
                &mut game,
                &Effect::deal_damage(3, ChooseSpec::SpecificPlayer(B)),
                &mut ctx,
            )
            .unwrap()
        };
        let mut queue = TriggerQueue::new();
        ironsmith::game_loop::drain_pending_trigger_events(&mut game, &mut queue);
        for event in outcome.events {
            for entry in check_triggers(&game, &event) {
                queue.add(entry);
            }
        }
        put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
        for _ in 0..5 {
            if game.stack_is_empty() {
                break;
            }
            resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        }
        assert_eq!(game.player(B).unwrap().life, 17);
        assert_eq!(game.player(A).unwrap().life, 17, "the other chosen player loses 3");
        assert_eq!(game.player(C).unwrap().life, 20);
    }
}
