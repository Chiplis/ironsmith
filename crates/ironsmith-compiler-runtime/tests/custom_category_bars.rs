//! Generic acquired ability groups behind MSEM C745/C750.
use ironsmith::decision::{compute_legal_actions, LegalAction};
use ironsmith::game_loop::resolve_stack_entry_with;
use ironsmith::{ObjectId, Zone};
#[path = "cf8_p08/support.rs"] mod support;
#[path = "cf8_p08/play.rs"] mod play;

fn legal(game: &ironsmith::GameState, source: ObjectId, index: usize) -> bool {
    compute_legal_actions(game, play::A).unwrap().iter().any(|action| matches!(action,
        LegalAction::ActivateAbility { source: id, ability_index } |
        LegalAction::ActivateManaAbility { source: id, ability_index } if *id == source && *ability_index == index))
}

#[test]
fn sequential_groups_require_prior_activations_and_track_once_use_across_turns() {
    let text = "Type: Creature — Human\nPower/Toughness: 1/1\nSequential group 0 step 0 — {0}: This permanent becomes a Wizard in addition to its other types. Put a +1/+1 counter on this permanent. Activate only as a sorcery.\nSequential group 0 step 1 — {0}: This permanent becomes a God in addition to its other types. Put a +1/+1 counter on this permanent. Activate only as a sorcery.";
    for definition in support::definitions("Ordered group", text) {
        let mut game = play::game();
        let source = game.create_object_from_definition(&definition, play::A, Zone::Battlefield);
        let first = play::activated_at(&definition, 0); let second = play::activated_at(&definition, 1);
        assert!(legal(&game, source, first)); assert!(!legal(&game, source, second));
        let mut dm = play::Script::default();
        play::activate(&mut game, source, first, &mut dm);
        assert!(!legal(&game, source, first));
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert!(legal(&game, source, second));
        assert!(game.calculated_subtypes(source).contains(&ironsmith::Subtype::Wizard));
        assert!(game.calculated_subtypes(source).contains(&ironsmith::Subtype::Human));
        play::activate(&mut game, source, second, &mut dm);
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(game.counter_count(source, ironsmith::object::CounterType::PlusOnePlusOne), 2);
        assert!(!legal(&game, source, first)); assert!(!legal(&game, source, second));
        game.next_turn(); game.next_turn(); game.next_turn();
        game.turn.phase = ironsmith::game_state::Phase::FirstMain; game.turn.step = None;
        game.turn.active_player = play::A; game.turn.priority_player = Some(play::A);
        assert!(!legal(&game, source, first)); assert!(!legal(&game, source, second));
    }
}

#[test]
fn exclusive_choice_locks_on_announcement_and_survives_countering_and_control_changes() {
    let text = "Type: Creature — Human\nPower/Toughness: 1/1\nExclusive group 0 step 0 — {0}: You gain 1 life.\nExclusive group 0 step 1 — {0}: You gain 2 life.";
    for definition in support::definitions("Exclusive group", text) {
        let mut game = play::game();
        let source = game.create_object_from_definition(&definition, play::A, Zone::Battlefield);
        let first = play::activated_at(&definition, 0); let second = play::activated_at(&definition, 1);
        assert!(legal(&game, source, first)); assert!(legal(&game, source, second));
        play::activate(&mut game, source, first, &mut play::Script::default());
        assert!(!game.stack.is_empty());
        assert!(!legal(&game, source, second), "the alternative closes before resolution even at instant speed");
        game.stack.pop();
        game.set_current_controller(source, play::B).unwrap();
        game.set_current_controller(source, play::A).unwrap();
        assert!(!legal(&game, source, first)); assert!(!legal(&game, source, second));
        let replacement = game.create_object_from_definition(&definition, play::A, Zone::Battlefield);
        assert!(legal(&game, replacement, first)); assert!(legal(&game, replacement, second));
    }
}

#[test]
fn exclusive_histories_are_scoped_to_their_named_family() {
    let text = "Type: Creature\nPower/Toughness: 1/1\nFamily 41 Exclusive group 0 step 0 — {0}: You gain 1 life.\nFamily 41 Exclusive group 1 step 0 — {0}: You gain 2 life.\nFamily 42 Exclusive group 0 step 0 — {0}: You gain 3 life.";
    for definition in support::definitions("Independent families", text) {
        let mut game = play::game();
        let source = game.create_object_from_definition(&definition, play::A, Zone::Battlefield);
        let indices: Vec<_> = (0..3).map(|n| play::activated_at(&definition,n)).collect();
        play::activate(&mut game, source, indices[0], &mut play::Script::default());
        assert!(!legal(&game, source, indices[1]));
        assert!(legal(&game, source, indices[2]));
    }
}
