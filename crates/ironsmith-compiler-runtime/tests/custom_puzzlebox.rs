//! Per-source command-zone setup and exact persistent chosen-card consumption.
use ironsmith::decision::{LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::object::CounterType;
use ironsmith::target::ChooseSpec;
use ironsmith::{CardDefinition, Effect, GameState, ObjectId, PlayerId, Zone};
const A: PlayerId = PlayerId(0);
const SPIRITED: &str = "When this permanent enters, you may activate one of its activated abilities without paying that ability's cost.";
#[path="custom_final_next_wave_support.rs"] mod source_routes;
fn routes(text: &str) -> [CardDefinition; 2] {
    source_routes::definitions(&serde_json::json!({"name":"Ancient Maze","executable_text":text})).unwrap()
}

const SETUP: &str = "Before the game begins, while this card is in the command zone, Put another card you own from outside the game into the command zone. Turn it face down. Give it the locked status in the command zone. Give this card the locked status in the command zone. Remember the moved card as the chosen card.";
fn drain(game: &mut GameState) {
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    ironsmith::game_loop::drain_pending_trigger_events_with_dm(
        game,
        &mut queue,
        &mut SelectFirstDecisionMaker,
    )
    .unwrap();
    assert!(queue.entries.is_empty());
}
fn activate(game: &mut GameState, source: ObjectId, index: usize) {
    game.turn.priority_player = Some(A);
    let action = compute_legal_actions(game, A)
        .unwrap()
        .into_iter()
        .find(|action| {
            matches!(action, LegalAction::ActivateAbility { source: id, ability_index }
            if *id == source && *ability_index == index)
        })
        .expect("printed activation legal");
    use ironsmith::game_loop::{
        PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
        apply_priority_response_with_dm,
    };
    let mut state = PriorityLoopState::new(2);
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut dm = SelectFirstDecisionMaker;
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        &mut dm,
    )
    .unwrap();
    for _ in 0..64 {
        if !state.has_pending_action() {
            ironsmith::game_loop::resolve_stack_entry_with(game, &mut dm).unwrap();
            drain(game);
            return;
        }
        let ironsmith::GameProgress::NeedsDecisionCtx(ctx) = progress else {
            panic!("{progress:?}")
        };
        progress =
            apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, &mut dm).unwrap();
    }
    panic!("activation did not finish");
}

fn outside(game: &mut GameState, owner: PlayerId, name: &str) -> ObjectId {
    let definition = ironsmith_compiler_runtime::compile_to_runtime_definition(
        name,
        "Type: Creature — Fox\nPower/Toughness: 2/2",
        false,
    )
    .unwrap();
    game.create_object_from_definition(&definition, owner, Zone::OutsideGame)
}
#[test]
fn source_setup_is_independent_per_conspiracy_and_remembers_actual_command_arrival() {
    let text = format!(
        "Type: Conspiracy\n{SETUP}\n{{0}}: Remove the locked status from this card in the command zone, then turn the card chosen for this source face up."
    );
    for definition in routes(&text) {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let first = game.create_object_from_definition(&definition, A, Zone::Command);
        let second = game.create_object_from_definition(&definition, A, Zone::Command);
        let a = outside(&mut game, A, "First outside card");
        let b = outside(&mut game, A, "Second outside card");
        let opponent = outside(&mut game, PlayerId(1), "Opponent outside card");
        let mut session =
            ironsmith::pregame_setup::PregameSetupSession::new(&game, vec![]).unwrap();
        for source in [first, second] {
            session
                .run_source_setup_program(&mut game, A, source, 0, &mut SelectFirstDecisionMaker)
                .unwrap();
        }
        let chosen_first = game.chosen_object(first).unwrap().object_id;
        let chosen_second = game.chosen_object(second).unwrap().object_id;
        assert_ne!(chosen_first, chosen_second);
        assert_ne!(chosen_first, a);
        assert_ne!(chosen_first, b);
        for source in [first, second] {
            let chosen = game.chosen_object(source).unwrap().object_id;
            assert_eq!(game.object(chosen).unwrap().zone, Zone::Command);
            assert!(game.is_face_down(chosen));
            assert!(game.has_custom_status(chosen, "locked"));
            assert!(game.has_custom_status(source, "locked"));
            assert!(
                session
                    .run_source_setup_program(
                        &mut game,
                        A,
                        source,
                        0,
                        &mut SelectFirstDecisionMaker
                    )
                    .is_err()
            );
        }
        assert_eq!(game.object(opponent).unwrap().zone, Zone::OutsideGame);
        game.turn.phase = ironsmith::game_state::Phase::FirstMain;
        game.turn.active_player = A;
        game.turn.priority_player = Some(A);
        activate(&mut game, first, 1);
        assert!(!game.is_face_down(chosen_first));
        assert!(game.is_face_down(chosen_second));
        assert!(!game.has_custom_status(first, "locked"));
        assert!(game.has_custom_status(second, "locked"));
    }
}
#[test]
fn chosen_card_reference_does_not_follow_a_departed_card_into_a_later_incarnation() {
    let text =
        format!("Type: Conspiracy\n{SETUP}\n{{0}}: Turn the card chosen for this source face up.");
    for definition in routes(&text) {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let source = game.create_object_from_definition(&definition, A, Zone::Command);
        outside(&mut game, A, "Selected outside card");
        let mut session =
            ironsmith::pregame_setup::PregameSetupSession::new(&game, vec![]).unwrap();
        session
            .run_source_setup_program(&mut game, A, source, 0, &mut SelectFirstDecisionMaker)
            .unwrap();
        let chosen = game.chosen_object(source).unwrap().object_id;
        let stable = game.object(chosen).unwrap().stable_id;
        execute_effect(
            &mut game,
            &Effect::new(ironsmith::effects::MoveToZoneEffect::new(
                ChooseSpec::SpecificObject(chosen),
                Zone::Hand,
                false,
            )),
            &mut EffectContext::new_default(source, A),
        )
        .unwrap();
        let hand = game.find_object_by_stable_id(stable).unwrap();
        execute_effect(
            &mut game,
            &Effect::new(ironsmith::effects::MoveToZoneEffect::new(
                ChooseSpec::SpecificObject(hand),
                Zone::Command,
                false,
            )),
            &mut EffectContext::new_default(source, A),
        )
        .unwrap();
        let returned = game.find_object_by_stable_id(stable).unwrap();
        game.set_face_down(returned);
        game.turn.phase = ironsmith::game_state::Phase::FirstMain;
        game.turn.active_player = A;
        game.turn.priority_player = Some(A);
        activate(&mut game, source, 1);
        assert!(game.is_face_down(returned));
    }
}

#[test]
fn full_printed_source_uses_the_turned_card_for_green_and_otherwise_destinations() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/custom_mechanics/msem-puzzlebox.json"
    ))
    .unwrap();
    let text = fixture["mechanics"][0]["representative_cards"][0]["executable_text"]
        .as_str()
        .unwrap();
    for definition in routes(text) {
        for green in [true, false] {
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let source = game.create_object_from_definition(&definition, A, Zone::Command);
            let payload = ironsmith_compiler_runtime::compile_to_runtime_definition(
                "Chosen payload",
                if green {
                    "Mana cost: {G}\nType: Creature — Fox\nPower/Toughness: 1/1"
                } else {
                    "Mana cost: {U}\nType: Creature — Fox\nPower/Toughness: 1/1"
                },
                false,
            )
            .unwrap();
            let outside = game.create_object_from_definition(&payload, A, Zone::OutsideGame);
            let stable = game.object(outside).unwrap().stable_id;
            let mut session =
                ironsmith::pregame_setup::PregameSetupSession::new(&game, vec![]).unwrap();
            session
                .run_source_setup_program(&mut game, A, source, 0, &mut SelectFirstDecisionMaker)
                .unwrap();
            game.turn.phase = ironsmith::game_state::Phase::FirstMain;
            game.turn.active_player = A;
            game.turn.priority_player = Some(A);
            game.player_mut(A)
                .unwrap()
                .mana_pool
                .add(ironsmith::mana::ManaSymbol::Colorless, 6);
            activate(&mut game, source, 1);
            let moved = game.find_object_by_stable_id(stable).unwrap();
            assert_eq!(
                game.object(moved).unwrap().zone,
                if green { Zone::Hand } else { Zone::Library }
            );
            assert_eq!(game.object(source).unwrap().zone, Zone::Command);
            if !green {
                assert_eq!(game.player(A).unwrap().library.last(), Some(&moved));
            }
            assert!(!game.has_custom_status(source, "locked"));
            assert!(game.has_custom_status(source, "unlocked"));
            assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        }
    }
}

#[test]
fn incomplete_status_sequence_cannot_be_admitted_as_a_paid_activation(){
 for text in [
  "Type: Conspiracy\n{0}: Remove the locked status from this card in the command zone, then.",
  "Type: Conspiracy\n{0}: If this card has the locked status, remove the locked status from this card in the command zone, then.",
 ]{
  let input=serde_json::json!({"name":"Incomplete status sequence","executable_text":text});
  assert!(source_routes::definitions(&input).is_err(),"trailing then must not silently discard a claimed paid instruction");
 }
}
