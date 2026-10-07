// Existing fixtures expect no added programs. Assert that explicitly rather
// than reducing a rich prepared result and silently dropping programs.
fn require_plain_prepared_zone_outcome<T>(plan: PreparedEventOutcome<T>) -> EventOutcome<T> {
    assert!(
        plan.programs.is_empty(),
        "legacy fixture unexpectedly captured deferred programs"
    );
    plan.original
}

use super::*;
use crate::effect::{Effect, Value};
use crate::ids::{CardId, PlayerId};

fn setup() -> (GameState, ObjectId, PlayerId) {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let card = crate::card::CardBuilder::new(CardId::new(), "Entry program source")
        .card_types(vec![crate::types::CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(2, 2))
        .build();
    let entrant = game.create_object_from_card(&card, alice, Zone::Hand);
    game.take_pending_trigger_events();
    (game, entrant, alice)
}

fn program(fails: bool) -> crate::resolution::ResolutionProgram {
    let mut effects = vec![Effect::gain_life(2)];
    if fails {
        effects.push(Effect::lose_life(Value::X));
    }
    crate::resolution::ResolutionProgram::from_effects(effects)
}

#[test]
fn entry_program_failure_cannot_retain_an_earlier_instruction() {
    for fails in [false, true] {
        let (mut game, entrant, alice) = setup();
        let mut dm = crate::decision::SelectFirstDecisionMaker;
        let result =
            game.execute_entry_programs(entrant, alice, vec![program(fails)], None, &mut dm);
        if fails {
            assert!(matches!(
                &result,
                Err(crate::effects::ExecutionError::UnresolvableValue(_))
            ));
        } else {
            assert!(result.as_ref().unwrap().is_some());
        }
        assert!(!dm.awaiting_choice());
        assert_eq!(
            game.player(alice).unwrap().life,
            if fails { 20 } else { 22 }
        );
        assert_eq!(game.object(entrant).unwrap().zone, Zone::Hand);
        if fails {
            assert!(game.take_pending_trigger_events().is_empty());
        }
    }
}

#[test]
fn central_entry_failure_restores_program_consequences_and_one_shot() {
    for fails in [false, true] {
        let (mut game, entrant, alice) = setup();
        let replacement = game.effect_store.replacement_effects.add_one_shot_effect(
            ReplacementEffect::with_matcher(
                entrant,
                alice,
                crate::events::zones::matchers::ThisWouldEnterBattlefieldMatcher,
                ReplacementAction::AsEntersProgram(program(fails)),
            ),
        );
        let mut dm = crate::decision::SelectFirstDecisionMaker;
        let result =
            game.move_object_with_etb_processing_with_dm(entrant, Zone::Battlefield, &mut dm);
        if fails {
            assert!(matches!(
                &result,
                Err(crate::effects::ExecutionError::UnresolvableValue(_))
            ));
        }
        assert!(!dm.awaiting_choice());
        assert_eq!(
            game.player(alice).unwrap().life,
            if fails { 20 } else { 22 }
        );
        if fails {
            assert!(game.battlefield.is_empty());
            assert_eq!(game.object(entrant).unwrap().zone, Zone::Hand);
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(replacement)
                    .is_some()
            );
            assert!(game.take_pending_trigger_events().is_empty());
        } else {
            let entered = result
                .expect("valid entry program must not fail")
                .assert_completed_without_additions()
                .expect("valid entry program must complete")
                .new_id;
            assert!(game.battlefield.contains(&entered));
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(replacement)
                    .is_none()
            );
        }
    }
}

fn gain_doubler(
    game: &mut GameState,
    source: ObjectId,
    alice: PlayerId,
) -> crate::replacement::ReplacementEffectId {
    game.effect_store
        .replacement_effects
        .add_one_shot_effect(ReplacementEffect::with_matcher(
            source,
            alice,
            crate::events::WouldGainLifeMatcher::new(crate::target::PlayerFilter::Specific(alice)),
            ReplacementAction::Modify(crate::replacement::EventModification::Multiply(2)),
        ))
}

#[test]
fn entry_program_list_error_restores_preceding_program_and_one_shot() {
    for fails in [false, true] {
        let (mut game, entrant, alice) = setup();
        let one_shot = gain_doubler(&mut game, entrant, alice);
        let first = crate::resolution::ResolutionProgram::from_effects(vec![Effect::gain_life(2)]);
        let mut second = vec![Effect::gain_life(1)];
        if fails {
            second.push(Effect::lose_life(Value::X));
        }
        let second = crate::resolution::ResolutionProgram::from_effects(second);
        let mut dm = crate::decision::SelectFirstDecisionMaker;
        let result =
            game.execute_entry_programs(entrant, alice, vec![first, second], None, &mut dm);
        if fails {
            assert!(matches!(
                &result,
                Err(crate::effects::ExecutionError::UnresolvableValue(_))
            ));
        } else {
            assert!(result.as_ref().unwrap().is_some());
        }
        assert!(!dm.awaiting_choice());
        assert_eq!(
            game.player(alice).unwrap().life,
            if fails { 20 } else { 25 }
        );
        assert_eq!(
            game.effect_store
                .replacement_effects
                .get_effect(one_shot)
                .is_some(),
            fails,
        );
        assert_eq!(game.object(entrant).unwrap().zone, Zone::Hand);
        if fails {
            assert!(game.take_pending_trigger_events().is_empty());
        }
    }
}

struct ProgramAnswers {
    pause: bool,
    calls: usize,
    pending: bool,
}

impl crate::decision::DecisionMaker for ProgramAnswers {
    fn decide_boolean(
        &mut self,
        _: &GameState,
        _: &crate::decisions::context::BooleanContext,
    ) -> bool {
        assert!(!self.pending);
        self.calls += 1;
        self.pending = self.pause && self.calls == 2;
        !self.pending
    }

    fn awaiting_choice(&self) -> bool {
        self.pending
    }
}

#[test]
fn entry_program_list_pause_restores_preceding_program_and_replays_once() {
    let (mut game, entrant, alice) = setup();
    let one_shot = gain_doubler(&mut game, entrant, alice);
    let programs = vec![
        crate::resolution::ResolutionProgram::from_effects(vec![
            Effect::gain_life(2),
            Effect::may(vec![Effect::gain_life(1)]),
        ]),
        crate::resolution::ResolutionProgram::from_effects(vec![
            Effect::gain_life(3),
            Effect::may(vec![Effect::gain_life(4)]),
        ]),
    ];
    let mut dm = ProgramAnswers {
        pause: true,
        calls: 0,
        pending: false,
    };
    let result = game.execute_entry_programs(entrant, alice, programs.clone(), None, &mut dm);
    assert!(result.unwrap().is_none());
    assert!(dm.pending);
    assert_eq!(dm.calls, 2);
    assert_eq!(game.player(alice).unwrap().life, 20);
    assert!(
        game.effect_store
            .replacement_effects
            .get_effect(one_shot)
            .is_some()
    );
    assert!(game.take_pending_trigger_events().is_empty());
    let mut replay = ProgramAnswers {
        pause: false,
        calls: 0,
        pending: false,
    };
    let result = game.execute_entry_programs(entrant, alice, programs, None, &mut replay);
    assert!(result.unwrap().is_some());
    assert!(!replay.pending);
    assert_eq!(replay.calls, 2);
    assert_eq!(game.player(alice).unwrap().life, 32);
    assert!(
        game.effect_store
            .replacement_effects
            .get_effect(one_shot)
            .is_none()
    );
    assert_eq!(game.object(entrant).unwrap().zone, Zone::Hand);
}

#[test]
fn central_entry_instead_payload_propagates_failure_and_publishes_actual_events() {
    for fails in [false, true] {
        let (mut game, entrant, alice) = setup();
        let mut effects = vec![Effect::gain_life(2)];
        if fails {
            effects.push(Effect::lose_life(Value::X));
        }
        let one_shot = game.effect_store.replacement_effects.add_one_shot_effect(
            ReplacementEffect::with_matcher(
                entrant,
                alice,
                crate::events::zones::matchers::ThisWouldEnterBattlefieldMatcher,
                ReplacementAction::Instead(effects),
            ),
        );
        let mut dm = crate::decision::SelectFirstDecisionMaker;
        let result =
            game.move_object_with_etb_processing_with_dm(entrant, Zone::Battlefield, &mut dm);
        if fails {
            assert!(matches!(
                result,
                Err(crate::effects::ExecutionError::UnresolvableValue(_))
            ));
        } else {
            let receipt = result.unwrap();
            assert!(!receipt.pending);
            assert!(receipt.programs.is_empty());
            assert!(receipt.original.is_replaced());
        }
        assert_eq!(
            game.player(alice).unwrap().life,
            if fails { 20 } else { 22 }
        );
        assert!(game.battlefield.is_empty());
        assert_eq!(game.object(entrant).unwrap().zone, Zone::Hand);
        assert_eq!(
            game.effect_store
                .replacement_effects
                .get_effect(one_shot)
                .is_some(),
            fails
        );
        let events = game
            .turn_store
            .turn_history
            .projected_records()
            .map(|record| &record.event)
            .collect::<Vec<_>>();
        if fails {
            assert!(events.is_empty());
        } else {
            assert_eq!(events.len(), 1);
            let gain = events[0]
                .downcast::<crate::events::LifeGainEvent>()
                .unwrap();
            assert_eq!(gain.player, alice);
            assert_eq!(gain.amount, 2);
        }
    }
}

#[test]
fn central_entry_instead_payload_pause_restores_and_replays_once() {
    let (mut game, entrant, alice) = setup();
    let one_shot =
        game.effect_store
            .replacement_effects
            .add_one_shot_effect(ReplacementEffect::with_matcher(
                entrant,
                alice,
                crate::events::zones::matchers::ThisWouldEnterBattlefieldMatcher,
                ReplacementAction::Instead(vec![
                    Effect::gain_life(2),
                    Effect::may(vec![Effect::gain_life(1)]),
                    Effect::may(vec![Effect::gain_life(3)]),
                ]),
            ));
    let mut dm = ProgramAnswers {
        pause: true,
        calls: 0,
        pending: false,
    };
    assert!(
        game.move_object_with_etb_processing_with_dm(entrant, Zone::Battlefield, &mut dm,)
            .unwrap()
            .pending
    );
    assert!(dm.pending);
    assert_eq!(dm.calls, 2);
    assert_eq!(game.player(alice).unwrap().life, 20);
    assert!(
        game.effect_store
            .replacement_effects
            .get_effect(one_shot)
            .is_some()
    );
    assert!(game.take_pending_trigger_events().is_empty());
    let mut replay = ProgramAnswers {
        pause: false,
        calls: 0,
        pending: false,
    };
    assert!(
        game.move_object_with_etb_processing_with_dm(entrant, Zone::Battlefield, &mut replay,)
            .unwrap()
            .assert_completed_without_additions()
            .is_none()
    );
    assert!(!replay.pending);
    assert_eq!(game.player(alice).unwrap().life, 26);
    assert!(
        game.effect_store
            .replacement_effects
            .get_effect(one_shot)
            .is_none()
    );
    assert!(game.battlefield.is_empty());
    let events = game
        .turn_store
        .turn_history
        .projected_records()
        .map(|record| &record.event)
        .collect::<Vec<_>>();
    assert_eq!(events.len(), 3);
    let gains = events
        .iter()
        .map(|event| event.downcast::<crate::events::LifeGainEvent>().unwrap())
        .collect::<Vec<_>>();
    assert!(gains.iter().all(|event| event.player == alice));
    assert_eq!(gains.iter().map(|event| event.amount).sum::<u32>(), 6);
}

fn resolving_creature() -> (GameState, ObjectId, PlayerId, PlayerId) {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let card = crate::card::CardBuilder::new(CardId::new(), "Resolving entry fixture")
        .card_types(vec![crate::types::CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(2, 2))
        .build();
    let spell = game.create_object_from_card(&card, bob, Zone::Stack);
    game.set_current_controller(spell, alice)
        .expect("finite controller fixture must refresh successfully");
    game.push_to_stack(crate::game_state::StackEntry::new(spell, alice));
    game.take_pending_trigger_events();
    (game, spell, alice, bob)
}

#[test]
fn resolving_nonowner_spell_commits_the_replacement_controller() {
    let (mut game, spell, alice, bob) = resolving_creature();
    let carol = PlayerId::from_index(2);
    let stable_id = game.object(spell).unwrap().stable_id;
    let one_shot =
        game.effect_store
            .replacement_effects
            .add_one_shot_effect(ReplacementEffect::with_matcher(
                spell,
                alice,
                crate::events::zones::matchers::ThisWouldEnterBattlefieldMatcher,
                ReplacementAction::EnterUnderControl(carol),
            ));
    let mut dm = crate::decision::SelectFirstDecisionMaker;
    crate::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
    let entered = game.find_object_by_stable_id(stable_id).unwrap();
    assert_eq!(game.object(entered).unwrap().zone, Zone::Battlefield);
    assert_eq!(game.object(entered).unwrap().owner, bob);
    assert_eq!(game.current_controller(entered), Some(carol));
    assert!(game.stack.is_empty());
    assert!(
        game.effect_store
            .replacement_effects
            .get_effect(one_shot)
            .is_none()
    );
}

#[test]
fn resolving_spell_entry_failure_restores_the_popped_stack_entry() {
    let (mut game, spell, alice, _) = resolving_creature();
    let one_shot =
        game.effect_store
            .replacement_effects
            .add_one_shot_effect(ReplacementEffect::with_matcher(
                spell,
                alice,
                crate::events::zones::matchers::ThisWouldEnterBattlefieldMatcher,
                ReplacementAction::AsEntersProgram(program(true)),
            ));
    let mut dm = crate::decision::SelectFirstDecisionMaker;
    let result = crate::game_loop::resolve_stack_entry_with(&mut game, &mut dm);
    assert!(matches!(
        result,
        Err(crate::game_loop::GameLoopError::ExecutionFailed(
            crate::effects::ExecutionError::UnresolvableValue(_)
        ))
    ));
    assert_eq!(game.stack.len(), 1);
    assert_eq!(game.stack[0].object_id, spell);
    assert_eq!(game.object(spell).unwrap().zone, Zone::Stack);
    assert_eq!(game.current_controller(spell), Some(alice));
    assert_eq!(game.player(alice).unwrap().life, 20);
    assert!(game.battlefield.is_empty());
    assert!(
        game.effect_store
            .replacement_effects
            .get_effect(one_shot)
            .is_some()
    );
    assert!(game.take_pending_trigger_events().is_empty());
}

#[test]
fn resolving_spell_entry_pause_restores_the_stack_and_replays_once() {
    let (mut game, spell, alice, _) = resolving_creature();
    let stable_id = game.object(spell).unwrap().stable_id;
    let one_shot =
        game.effect_store
            .replacement_effects
            .add_one_shot_effect(ReplacementEffect::with_matcher(
                spell,
                alice,
                crate::events::zones::matchers::ThisWouldEnterBattlefieldMatcher,
                ReplacementAction::AsEntersProgram(
                    crate::resolution::ResolutionProgram::from_effects(vec![
                        Effect::gain_life(2),
                        Effect::may(vec![Effect::gain_life(1)]),
                        Effect::may(vec![Effect::gain_life(3)]),
                    ]),
                ),
            ));
    let mut dm = ProgramAnswers {
        pause: true,
        calls: 0,
        pending: false,
    };
    crate::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
    assert!(dm.pending);
    assert_eq!(dm.calls, 2);
    assert_eq!(game.stack.len(), 1);
    assert_eq!(game.stack[0].object_id, spell);
    assert_eq!(game.player(alice).unwrap().life, 20);
    assert!(game.take_pending_trigger_events().is_empty());
    assert!(
        game.effect_store
            .replacement_effects
            .get_effect(one_shot)
            .is_some()
    );
    let mut replay = ProgramAnswers {
        pause: false,
        calls: 0,
        pending: false,
    };
    crate::game_loop::resolve_stack_entry_with(&mut game, &mut replay).unwrap();
    assert!(!replay.pending);
    assert_eq!(game.player(alice).unwrap().life, 26);
    assert!(game.stack.is_empty());
    let entered = game.find_object_by_stable_id(stable_id).unwrap();
    assert_eq!(game.object(entered).unwrap().zone, Zone::Battlefield);
    assert_eq!(game.current_controller(entered), Some(alice));
    assert!(
        game.effect_store
            .replacement_effects
            .get_effect(one_shot)
            .is_none()
    );
    let events = game.take_pending_trigger_events();
    let gains = events
        .iter()
        .filter_map(|event| event.downcast::<crate::events::LifeGainEvent>())
        .collect::<Vec<_>>();
    assert_eq!(gains.len(), 3);
    assert_eq!(gains.iter().map(|event| event.amount).sum::<u32>(), 6);
}

#[test]
fn both_land_apis_commit_the_replacement_controller() {
    for priority_api in [false, true] {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        game.turn.phase = crate::game_state::Phase::FirstMain;
        game.turn.step = None;
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);
        let card = crate::card::CardBuilder::new(CardId::new(), "Land controller fixture")
            .card_types(vec![crate::types::CardType::Land])
            .build();
        let land = game.create_object_from_card(&card, alice, Zone::Hand);
        let stable_id = game.object(land).unwrap().stable_id;
        let one_shot = game.effect_store.replacement_effects.add_one_shot_effect(
            ReplacementEffect::with_matcher(
                land,
                alice,
                crate::events::zones::matchers::ThisWouldEnterBattlefieldMatcher,
                ReplacementAction::EnterUnderControl(bob),
            ),
        );
        let mut dm = crate::decision::SelectFirstDecisionMaker;
        if priority_api {
            let mut queue = crate::triggers::TriggerQueue::new();
            let mut state = crate::game_loop::PriorityLoopState::new(2);
            crate::game_loop::apply_priority_response_with_dm(
                &mut game,
                &mut queue,
                &mut state,
                &crate::PriorityResponse::PriorityAction(crate::decision::LegalAction::PlayLand {
                    land_id: land,
                }),
                &mut dm,
            )
            .unwrap();
        } else {
            crate::special_actions::perform(
                crate::special_actions::SpecialAction::PlayLand { card_id: land },
                &mut game,
                alice,
                &mut dm,
            )
            .unwrap();
        }
        let entered = game.find_object_by_stable_id(stable_id).unwrap();
        assert_eq!(game.object(entered).unwrap().zone, Zone::Battlefield);
        assert_eq!(game.object(entered).unwrap().owner, alice);
        assert_eq!(game.current_controller(entered), Some(bob));
        assert_eq!(game.player(alice).unwrap().lands_played_this_turn, 1);
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(one_shot)
                .is_none()
        );
    }
}

fn modal_land_setup() -> (GameState, ObjectId, PlayerId) {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    game.turn.phase = crate::game_state::Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    let front_id = CardId::new();
    let back_id = CardId::new();
    // Explicit PlayLandBackFace is offered for land/land modal pairs.
    // Creature/land modal pairs use the ordinary PlayLand action instead.
    let front = crate::cards::CardDefinitionBuilder::new(front_id, "Entry rollback front")
        .card_types(vec![crate::types::CardType::Land])
        .other_face(back_id)
        .other_face_name("Entry rollback land")
        .linked_face_layout(crate::card::LinkedFaceLayout::TransformLike)
        .build();
    let back = crate::cards::CardDefinitionBuilder::new(back_id, "Entry rollback land")
        .card_types(vec![crate::types::CardType::Land])
        .other_face(front_id)
        .other_face_name("Entry rollback front")
        .linked_face_layout(crate::card::LinkedFaceLayout::TransformLike)
        .build();
    game.register_linked_face_definition(&front);
    game.register_linked_face_definition(&back);
    let land = game.create_object_from_definition(&front, alice, Zone::Hand);
    game.take_pending_trigger_events();
    (game, land, alice)
}

fn play_modal_land(
    game: &mut GameState,
    land: ObjectId,
    alice: PlayerId,
    priority_api: bool,
    queue: &mut crate::triggers::TriggerQueue,
    state: &mut crate::game_loop::PriorityLoopState,
    dm: &mut impl crate::decision::DecisionMaker,
) -> Result<(), crate::effects::ExecutionError> {
    if priority_api {
        crate::game_loop::apply_priority_response_with_dm(
            game,
            queue,
            state,
            &crate::PriorityResponse::PriorityAction(
                crate::decision::LegalAction::PlayLandBackFace { land_id: land },
            ),
            dm,
        )
        .map(|_| ())
        .map_err(|error| match error {
            crate::game_loop::GameLoopError::ExecutionFailed(error) => error,
            other => panic!("unexpected land-play error: {other:?}"),
        })
    } else {
        crate::special_actions::perform(
            crate::special_actions::SpecialAction::PlayLandBackFace { card_id: land },
            game,
            alice,
            dm,
        )
        .map_err(|error| match error {
            crate::special_actions::ActionError::ExecutionFailure { error, .. } => error,
            other => panic!("unexpected land-play error: {other:?}"),
        })
    }
}

#[test]
fn both_land_apis_restore_selected_face_and_program_prefix_on_error() {
    for priority_api in [false, true] {
        for fails in [false, true] {
            let (mut game, land, alice) = modal_land_setup();
            let stable = game.object(land).unwrap().stable_id;
            let one_shot = game.effect_store.replacement_effects.add_one_shot_effect(
                ReplacementEffect::with_matcher(
                    land,
                    alice,
                    crate::events::zones::matchers::ThisWouldEnterBattlefieldMatcher,
                    ReplacementAction::AsEntersProgram(program(fails)),
                ),
            );
            let mut queue = crate::triggers::TriggerQueue::new();
            let mut state = crate::game_loop::PriorityLoopState::new(2);
            let state_before = format!("{state:?}");
            let mut dm = crate::decision::SelectFirstDecisionMaker;
            let result = play_modal_land(
                &mut game,
                land,
                alice,
                priority_api,
                &mut queue,
                &mut state,
                &mut dm,
            );
            assert_eq!(
                game.player(alice).unwrap().life,
                if fails { 20 } else { 22 }
            );
            assert_eq!(
                game.player(alice).unwrap().lands_played_this_turn,
                u32::from(!fails)
            );
            assert_eq!(
                game.effect_store
                    .replacement_effects
                    .get_effect(one_shot)
                    .is_some(),
                fails
            );
            if fails {
                assert!(matches!(
                    result,
                    Err(crate::effects::ExecutionError::UnresolvableValue(_))
                ));
                let object = game.object(land).unwrap();
                assert_eq!(object.name, "Entry rollback front");
                assert_eq!(object.zone, Zone::Hand);
                assert!(object.has_card_type(crate::types::CardType::Land));
                assert!(object.has_card_type(crate::types::CardType::Land));
                assert!(game.battlefield.is_empty());
                assert!(game.take_pending_trigger_events().is_empty());
                assert!(queue.is_empty());
                assert_eq!(format!("{state:?}"), state_before);
            } else {
                result.unwrap();
                let object = game
                    .object(game.find_object_by_stable_id(stable).unwrap())
                    .unwrap();
                assert_eq!(object.name, "Entry rollback land");
                assert_eq!(object.zone, Zone::Battlefield);
                assert!(object.has_card_type(crate::types::CardType::Land));
            }
        }
    }
}

#[test]
fn both_land_apis_restore_selected_face_on_pause_and_replay_once() {
    for priority_api in [false, true] {
        let (mut game, land, alice) = modal_land_setup();
        let stable = game.object(land).unwrap().stable_id;
        let one_shot = game.effect_store.replacement_effects.add_one_shot_effect(
            ReplacementEffect::with_matcher(
                land,
                alice,
                crate::events::zones::matchers::ThisWouldEnterBattlefieldMatcher,
                ReplacementAction::AsEntersProgram(
                    crate::resolution::ResolutionProgram::from_effects(vec![
                        Effect::gain_life(2),
                        Effect::may(vec![Effect::gain_life(1)]),
                        Effect::may(vec![Effect::gain_life(3)]),
                    ]),
                ),
            ),
        );
        let mut queue = crate::triggers::TriggerQueue::new();
        let mut state = crate::game_loop::PriorityLoopState::new(2);
        let state_before = format!("{state:?}");
        let mut dm = ProgramAnswers {
            pause: true,
            calls: 0,
            pending: false,
        };
        play_modal_land(
            &mut game,
            land,
            alice,
            priority_api,
            &mut queue,
            &mut state,
            &mut dm,
        )
        .unwrap();
        assert!(dm.pending);
        assert_eq!(dm.calls, 2);
        assert_eq!(game.object(land).unwrap().name, "Entry rollback front");
        assert_eq!(game.object(land).unwrap().zone, Zone::Hand);
        assert!(
            game.object(land)
                .unwrap()
                .has_card_type(crate::types::CardType::Land)
        );
        assert_eq!(game.player(alice).unwrap().life, 20);
        assert_eq!(game.player(alice).unwrap().lands_played_this_turn, 0);
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(one_shot)
                .is_some()
        );
        assert!(game.battlefield.is_empty());
        assert!(game.take_pending_trigger_events().is_empty());
        assert!(queue.is_empty());
        assert_eq!(format!("{state:?}"), state_before);
        let mut replay = ProgramAnswers {
            pause: false,
            calls: 0,
            pending: false,
        };
        play_modal_land(
            &mut game,
            land,
            alice,
            priority_api,
            &mut queue,
            &mut state,
            &mut replay,
        )
        .unwrap();
        assert!(!replay.pending);
        let object = game
            .object(game.find_object_by_stable_id(stable).unwrap())
            .unwrap();
        assert_eq!(object.name, "Entry rollback land");
        assert_eq!(object.zone, Zone::Battlefield);
        assert!(object.has_card_type(crate::types::CardType::Land));
        assert_eq!(game.player(alice).unwrap().life, 26);
        assert_eq!(game.player(alice).unwrap().lands_played_this_turn, 1);
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(one_shot)
                .is_none()
        );
    }
}

#[test]
fn zone_entry_continuation_retains_history_and_zone_cause_matchers() {
    for matching_cause in [false, true] {
        let (mut game, entrant, alice) = setup();
        let bob = PlayerId::from_index(1);
        let cause = crate::events::cause::EventCause::from_effect(
            entrant,
            if matching_cause { bob } else { alice },
        );
        let redirect = game.effect_store.replacement_effects.add_resolution_effect(
            ReplacementEffect::with_matcher(
                entrant,
                alice,
                crate::events::zones::matchers::WouldChangeZoneMatcher::new(
                    crate::target::ObjectFilter::specific(entrant),
                    Some(Zone::Hand),
                    None,
                ),
                ReplacementAction::ChangeDestination(Zone::Battlefield),
            ),
        );
        let controller = game.effect_store.replacement_effects.add_one_shot_effect(
            ReplacementEffect::with_matcher(
                entrant,
                alice,
                crate::events::zones::matchers::WouldChangeZoneMatcher::new(
                    crate::target::ObjectFilter::specific(entrant),
                    Some(Zone::Hand),
                    Some(Zone::Battlefield),
                )
                .with_cause_filter(
                    crate::events::cause::CauseFilter::effect_like()
                        .with_controller(crate::events::cause::ControllerFilter::Player(bob)),
                ),
                ReplacementAction::EnterUnderControl(bob),
            ),
        );
        let snapshot = crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
            game.object(entrant).unwrap(),
            &game,
        );
        let zone = crate::events::ZoneChangeEvent::with_cause(
            entrant,
            Zone::Hand,
            Zone::Graveyard,
            cause.clone(),
            Some(snapshot),
        );
        let mut state = TraitEventProcessingState {
            defer_battlefield_entry: true,
            zone_change_context: Some(zone.clone()),
            ..Default::default()
        };
        let mut dm = crate::decision::SelectFirstDecisionMaker;
        let result = process_with_dm_and_additional_effects_and_applied_state(
            &mut game,
            crate::events::Event::new_with_provenance(zone, Default::default()),
            &mut dm,
            &[],
            &Default::default(),
            &Default::default(),
            None,
            &mut state,
        );
        let event = result
            .expect("finite destination processing succeeds")
            .into_event()
            .unwrap();
        assert_eq!(event.kind(), crate::events::EventKind::ZoneChange);
        assert!(state.was_applied(redirect));
        assert!(!state.was_applied(controller));
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(controller)
                .is_some()
        );
        let context = ReplacementEventContext::new(&game, event, &state);
        let result = process_etb_from_zone_change_context(&mut game, context, &mut dm).unwrap();
        // Without carried history the persistent broad zone redirect applies
        // again to the entry carrier and loses this resolved entry proposal.
        assert!(!result.prevented);
        // The carrier records initial control even when no replacement changes
        // it; commit must not reconstruct this field from the old instruction.
        assert_eq!(
            result.controller_override,
            Some(if matching_cause { bob } else { alice })
        );
        let prepared = game
            .prepare_etb_entry_with_controller_and_dm(entrant, result, Some(alice), &mut dm)
            .unwrap()
            .unwrap();
        let entered = game
            .commit_prepared_etb_with_cause_and_options_and_dm(
                entrant,
                prepared,
                Some(alice),
                cause.clone(),
                true,
                &mut dm,
            )
            .unwrap()
            .assert_completed_without_additions()
            .unwrap()
            .new_id;
        assert_eq!(game.object(entered).unwrap().zone, Zone::Battlefield);
        assert_eq!(
            game.current_controller(entered),
            Some(if matching_cause { bob } else { alice })
        );
        assert_eq!(
            game.effect_store
                .replacement_effects
                .get_effect(controller)
                .is_some(),
            !matching_cause
        );
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(redirect)
                .is_some()
        );
        let events = game.take_pending_trigger_events();
        let changes = events
            .iter()
            .filter_map(|event| event.downcast::<crate::events::ZoneChangeEvent>())
            .collect::<Vec<_>>();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].from, Zone::Hand);
        assert_eq!(changes[0].to, Zone::Battlefield);
        assert_eq!(changes[0].cause.source, cause.source);
        assert_eq!(changes[0].cause.source_controller, cause.source_controller);
        assert_eq!(changes[0].cause.cause_type, cause.cause_type);
    }
}

#[test]
fn zone_entry_continuation_rejects_an_incompatible_carrier() {
    let (mut game, entrant, alice) = setup();
    let mut dm = crate::decision::SelectFirstDecisionMaker;
    let context = ReplacementEventContext::new(
        &game,
        crate::events::Event::zone_change(
            entrant,
            Zone::Hand,
            Zone::Graveyard,
            crate::events::cause::EventCause::from_effect(entrant, alice),
            None,
        ),
        &TraitEventProcessingState::default(),
    );
    assert!(matches!(
        process_etb_from_zone_change_context(&mut game, context, &mut dm),
        Err(crate::effects::ExecutionError::InternalError(_))
    ));
    assert_eq!(game.object(entrant).unwrap().zone, Zone::Hand);
    assert!(game.battlefield.is_empty());
    assert!(game.take_pending_trigger_events().is_empty());
}

#[test]
fn typed_zone_preparation_retains_entry_receipt_and_rolls_back_program_error() {
    for fails in [false, true] {
        let (mut game, entrant, alice) = setup();
        let redirect = game.effect_store.replacement_effects.add_one_shot_effect(
            ReplacementEffect::with_matcher(
                entrant,
                alice,
                crate::events::zones::matchers::WouldChangeZoneMatcher::new(
                    crate::target::ObjectFilter::specific(entrant),
                    Some(Zone::Hand),
                    Some(Zone::Graveyard),
                ),
                ReplacementAction::ChangeDestination(Zone::Battlefield),
            ),
        );
        let program_id = game.effect_store.replacement_effects.add_one_shot_effect(
            ReplacementEffect::with_matcher(
                entrant,
                alice,
                crate::events::zones::matchers::ThisWouldEnterBattlefieldMatcher,
                ReplacementAction::AsEntersProgram(program(fails)),
            ),
        );
        let cause = crate::events::cause::EventCause::from_effect(entrant, alice);
        let mut dm = crate::decision::SelectFirstDecisionMaker;
        let result = prepare_zone_change_with_context_and_additional_effects(
            &mut game,
            entrant,
            Zone::Hand,
            Zone::Graveyard,
            cause.clone(),
            &mut dm,
            &[],
            None,
        )
        .map(require_plain_prepared_zone_outcome);
        assert_eq!(
            game.player(alice).unwrap().life,
            if fails { 20 } else { 22 }
        );
        assert_eq!(game.object(entrant).unwrap().zone, Zone::Hand);
        assert!(game.battlefield.is_empty());
        assert_eq!(
            game.effect_store
                .replacement_effects
                .get_effect(redirect)
                .is_some(),
            fails
        );
        assert_eq!(
            game.effect_store
                .replacement_effects
                .get_effect(program_id)
                .is_some(),
            fails
        );
        if fails {
            assert!(matches!(
                result,
                Err(crate::effects::ExecutionError::UnresolvableValue(_))
            ));
            assert!(game.take_pending_trigger_events().is_empty());
        } else {
            let EventOutcome::Proceed(prepared) = result.unwrap() else {
                panic!("entry should be prepared");
            };
            assert_eq!(prepared.final_zone, Zone::Battlefield);
            assert!(prepared.context.applied_effects.contains(&redirect));
            assert!(prepared.context.applied_effects.contains(&program_id));
            assert!(
                prepared
                    .entry
                    .as_ref()
                    .unwrap()
                    .zone_entry_lookback
                    .is_some()
            );
            let original_name = game.object(entrant).unwrap().name.clone();
            // Prove commit uses preparation's frozen LKI, not a late snapshot.
            game.object_mut(entrant).unwrap().name = "Changed after preparation".into();
            let entered = commit_prepared_zone_change(&mut game, entrant, prepared, &mut dm)
                .unwrap()
                .assert_without_additions()
                .into_result()
                .expect("original zone commit must proceed");
            assert_eq!(game.object(entered).unwrap().zone, Zone::Battlefield);
            let events = game.take_pending_trigger_events();
            assert_eq!(
                events
                    .iter()
                    .filter(|event| event.downcast::<crate::events::LifeGainEvent>().is_some())
                    .count(),
                1
            );
            let changes = events
                .iter()
                .filter_map(|event| event.downcast::<crate::events::ZoneChangeEvent>())
                .collect::<Vec<_>>();
            assert_eq!(changes.len(), 1);
            assert_eq!(changes[0].snapshot.as_ref().unwrap().name, original_name);
            assert_eq!(changes[0].cause.source, cause.source);
            assert_eq!(changes[0].cause.source_controller, cause.source_controller);
        }
    }
}

#[test]
fn typed_zone_handoff_keeps_unapplied_temporary_entry_modifiers() {
    let (mut game, entrant, alice) = setup();
    let bob = PlayerId::from_index(1);
    let extra = vec![ReplacementEffect::with_matcher(
        entrant,
        alice,
        crate::events::zones::matchers::WouldChangeZoneMatcher::new(
            crate::target::ObjectFilter::specific(entrant),
            Some(Zone::Hand),
            Some(Zone::Battlefield),
        ),
        ReplacementAction::EnterUnderControl(bob),
    )];
    let mut dm = crate::decision::SelectFirstDecisionMaker;
    let EventOutcome::Proceed(prepared) = prepare_zone_change_with_context_and_additional_effects(
        &mut game,
        entrant,
        Zone::Hand,
        Zone::Battlefield,
        crate::events::cause::EventCause::from_effect(entrant, alice),
        &mut dm,
        &extra,
        None,
    )
    .map(require_plain_prepared_zone_outcome)
    .unwrap() else {
        panic!("entry should be prepared");
    };
    assert!(
        prepared
            .context
            .applied_effect_keys
            .contains(&extra[0].application_key())
    );
    let entered = commit_prepared_zone_change(&mut game, entrant, prepared, &mut dm)
        .unwrap()
        .assert_without_additions()
        .into_result()
        .expect("original zone commit must proceed");
    assert_eq!(game.current_controller(entered), Some(bob));
}

fn compound_setup(follow_ups: Vec<Effect>) -> (GameState, ObjectId, PlayerId, ReplacementEffectId) {
    let (mut game, entrant, alice) = setup();
    let id =
        game.effect_store
            .replacement_effects
            .add_one_shot_effect(ReplacementEffect::with_matcher(
                entrant,
                alice,
                crate::events::zones::matchers::WouldChangeZoneMatcher::new(
                    crate::target::ObjectFilter::specific(entrant),
                    Some(Zone::Hand),
                    Some(Zone::Graveyard),
                ),
                ReplacementAction::ExileWithSourceLinkCountersThen {
                    counters: vec![(CounterType::Ice, 2)],
                    effects: follow_ups,
                },
            ));
    (game, entrant, alice, id)
}

#[test]
fn typed_compound_zone_replacement_restores_move_counters_links_and_follow_up_on_error() {
    for fails in [false, true] {
        let mut effects = vec![Effect::gain_life(2)];
        if fails {
            effects.push(Effect::lose_life(Value::X));
        }
        let (mut game, entrant, alice, id) = compound_setup(effects);
        let mut dm = crate::decision::SelectFirstDecisionMaker;
        let result = prepare_zone_change_with_context_and_additional_effects(
            &mut game,
            entrant,
            Zone::Hand,
            Zone::Graveyard,
            crate::events::cause::EventCause::from_effect(entrant, alice),
            &mut dm,
            &[],
            None,
        )
        .map(require_plain_prepared_zone_outcome);
        assert_eq!(
            game.player(alice).unwrap().life,
            if fails { 20 } else { 22 }
        );
        assert_eq!(
            game.effect_store
                .replacement_effects
                .get_effect(id)
                .is_some(),
            fails
        );
        if fails {
            assert!(matches!(
                result,
                Err(crate::effects::ExecutionError::UnresolvableValue(_))
            ));
            assert_eq!(game.object(entrant).unwrap().zone, Zone::Hand);
            assert!(game.exile.is_empty());
            assert!(game.get_exiled_with_source_links(entrant).is_empty());
            assert!(game.take_pending_trigger_events().is_empty());
        } else {
            assert!(matches!(result.unwrap(), EventOutcome::Replaced));
            let moved = game.current_object_id_after_zone_change(entrant).unwrap();
            assert_ne!(moved, entrant);
            assert_eq!(game.object(moved).unwrap().zone, Zone::Exile);
            assert_eq!(game.counter_count(moved, CounterType::Ice), 2);
            assert_eq!(game.get_exiled_with_source_links(entrant), &[moved]);
            let events = game.take_pending_trigger_events();
            assert_eq!(
                events
                    .iter()
                    .filter(|event| event.kind() == crate::events::EventKind::ZoneChange)
                    .count(),
                1
            );
            assert_eq!(
                events
                    .iter()
                    .filter(|event| event.kind() == crate::events::EventKind::LifeGain)
                    .count(),
                1
            );
        }
    }
}

#[test]
fn typed_compound_zone_replacement_pause_restores_before_replay() {
    let (mut game, entrant, alice, id) = compound_setup(vec![
        Effect::gain_life(2),
        Effect::may(vec![Effect::gain_life(1)]),
        Effect::may(vec![Effect::gain_life(3)]),
    ]);
    let cause = crate::events::cause::EventCause::from_effect(entrant, alice);
    let mut dm = ProgramAnswers {
        pause: true,
        calls: 0,
        pending: false,
    };
    let result = prepare_zone_change_with_context_and_additional_effects(
        &mut game,
        entrant,
        Zone::Hand,
        Zone::Graveyard,
        cause.clone(),
        &mut dm,
        &[],
        None,
    )
    .map(require_plain_prepared_zone_outcome)
    .unwrap();
    assert!(dm.pending);
    assert_eq!(dm.calls, 2);
    assert!(matches!(result, EventOutcome::Prevented));
    assert_eq!(game.player(alice).unwrap().life, 20);
    assert_eq!(game.object(entrant).unwrap().zone, Zone::Hand);
    assert!(game.exile.is_empty());
    assert!(game.get_exiled_with_source_links(entrant).is_empty());
    assert!(
        game.effect_store
            .replacement_effects
            .get_effect(id)
            .is_some()
    );
    assert!(game.take_pending_trigger_events().is_empty());
    let mut replay = ProgramAnswers {
        pause: false,
        calls: 0,
        pending: false,
    };
    let result = prepare_zone_change_with_context_and_additional_effects(
        &mut game,
        entrant,
        Zone::Hand,
        Zone::Graveyard,
        cause,
        &mut replay,
        &[],
        None,
    )
    .map(require_plain_prepared_zone_outcome)
    .unwrap();
    assert!(matches!(result, EventOutcome::Replaced));
    assert_eq!(game.player(alice).unwrap().life, 26);
    let moved = game.current_object_id_after_zone_change(entrant).unwrap();
    assert_eq!(game.get_exiled_with_source_links(entrant), &[moved]);
    assert_eq!(game.counter_count(moved, CounterType::Ice), 2);
    assert!(
        game.effect_store
            .replacement_effects
            .get_effect(id)
            .is_none()
    );
    let events = game.take_pending_trigger_events();
    assert_eq!(
        events
            .iter()
            .filter(|event| event.kind() == crate::events::EventKind::ZoneChange)
            .count(),
        1
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| event.kind() == crate::events::EventKind::LifeGain)
            .count(),
        3
    );
}

#[test]
fn typed_compound_battlefield_counters_participate_in_entry_replacements_once() {
    let (mut game, entrant, alice) = setup();
    let move_id =
        game.effect_store
            .replacement_effects
            .add_one_shot_effect(ReplacementEffect::with_matcher(
                entrant,
                alice,
                crate::events::zones::matchers::WouldChangeZoneMatcher::new(
                    crate::target::ObjectFilter::specific(entrant),
                    Some(Zone::Hand),
                    Some(Zone::Graveyard),
                ),
                ReplacementAction::MoveToZoneWithCounters {
                    zone: Zone::Battlefield,
                    counters: vec![(CounterType::PlusOnePlusOne, 3)],
                },
            ));
    // Use the production counter-doubler's matcher: the explicit placement
    // matcher intentionally only accepts PutCounters, not entering events.
    let doubler = prefix_card(&mut game, alice, Zone::Battlefield, "Counter doubler");
    let double_id = game.effect_store.replacement_effects.add_resolution_effect(
        crate::static_abilities::StaticAbility::double_counters_replacement(
            crate::target::ObjectFilter::permanent(),
            None,
            "If counters would be put on a permanent, put twice that many instead.".into(),
        )
        .generate_replacement_effect(doubler, alice)
        .unwrap(),
    );
    let mut dm = crate::decision::SelectFirstDecisionMaker;
    let result = prepare_zone_change_with_context_and_additional_effects(
        &mut game,
        entrant,
        Zone::Hand,
        Zone::Graveyard,
        crate::events::cause::EventCause::from_effect(entrant, alice),
        &mut dm,
        &[],
        None,
    )
    .map(require_plain_prepared_zone_outcome)
    .unwrap();
    assert!(matches!(result, EventOutcome::Replaced));
    let moved = game.current_object_id_after_zone_change(entrant).unwrap();
    assert_eq!(game.object(moved).unwrap().zone, Zone::Battlefield);
    assert_eq!(game.counter_count(moved, CounterType::PlusOnePlusOne), 6);
    assert!(
        game.effect_store
            .replacement_effects
            .get_effect(move_id)
            .is_none()
    );
    assert!(
        game.effect_store
            .replacement_effects
            .get_effect(double_id)
            .is_some()
    );
    assert!(game.get_exiled_with_source_links(entrant).is_empty());
    let events = game.take_pending_trigger_events();
    assert_eq!(
        events
            .iter()
            .filter(|event| event.kind() == crate::events::EventKind::ZoneChange)
            .count(),
        1
    );
}

fn prefix_card(game: &mut GameState, alice: PlayerId, zone: Zone, name: &str) -> ObjectId {
    let card = crate::card::CardBuilder::new(CardId::new(), name)
        .card_types(vec![crate::types::CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(2, 2))
        .build();
    game.create_object_from_card(&card, alice, zone)
}

fn prefix_entry_replacements(
    game: &mut GameState,
    source: ObjectId,
    entrant: ObjectId,
    alice: PlayerId,
    from: Zone,
    to: Zone,
    mode: usize,
) -> (ReplacementEffectId, ReplacementEffectId) {
    let redirect =
        game.effect_store
            .replacement_effects
            .add_one_shot_effect(ReplacementEffect::with_matcher(
                source,
                alice,
                crate::events::zones::matchers::WouldChangeZoneMatcher::new(
                    crate::target::ObjectFilter::specific(entrant),
                    Some(from),
                    Some(to),
                ),
                ReplacementAction::ChangeDestination(Zone::Battlefield),
            ));
    let mut effects = vec![Effect::gain_life(2)];
    if mode == 1 {
        effects.push(Effect::lose_life(Value::X));
    }
    if mode == 2 {
        effects.push(Effect::may(vec![Effect::gain_life(1)]));
        effects.push(Effect::may(vec![Effect::gain_life(3)]));
    }
    let entry =
        game.effect_store
            .replacement_effects
            .add_one_shot_effect(ReplacementEffect::with_matcher(
                entrant,
                alice,
                crate::events::zones::matchers::ThisWouldEnterBattlefieldMatcher,
                ReplacementAction::AsEntersProgram(
                    crate::resolution::ResolutionProgram::from_effects(effects),
                ),
            ));
    (redirect, entry)
}

fn execute_ending_prefix(
    game: &mut GameState,
    ctx: &mut crate::effects::ExecutionContext,
    combat: bool,
) -> Result<crate::effect::EffectOutcome, crate::effects::ExecutionError> {
    use crate::effects::EffectExecutor;
    if combat {
        crate::effects::EndCombatPhaseEffect::new().execute(game, ctx)
    } else {
        crate::effects::EndTurnEffect::new(crate::target::PlayerFilter::You).execute(game, ctx)
    }
}

#[test]
fn ending_entry_failure_or_pause_restores_stack_notifications_and_scheduler_prefix() {
    for combat in [false, true] {
        for mode in 0..3 {
            let (mut game, source, alice) = setup();
            game.turn.active_player = alice;
            game.turn.priority_player = Some(alice);
            game.turn.phase = crate::game_state::Phase::Combat;
            let later = prefix_card(&mut game, alice, Zone::Stack, "Later entry");
            let earlier = prefix_card(&mut game, alice, Zone::Stack, "Earlier exile");
            let earlier_stable = game.object(earlier).unwrap().stable_id;
            let later_stable = game.object(later).unwrap().stable_id;
            game.push_to_stack(crate::game_state::StackEntry::new(later, alice));
            game.push_to_stack(crate::game_state::StackEntry::new(earlier, alice));
            game.take_pending_trigger_events();
            let seed = crate::triggers::TriggerEvent::new_with_provenance(
                crate::events::LifeGainEvent::new(alice, 9),
                Default::default(),
            );
            game.queue_trigger_event(seed.provenance(), seed);
            let ids = prefix_entry_replacements(
                &mut game,
                source,
                later,
                alice,
                Zone::Stack,
                Zone::Exile,
                mode,
            );
            let mut dm = ProgramAnswers {
                pause: mode == 2,
                calls: 0,
                pending: false,
            };
            let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut dm);
            let result = execute_ending_prefix(&mut game, &mut ctx, combat);
            drop(ctx);
            if mode != 0 {
                if mode == 1 {
                    assert!(matches!(
                        result,
                        Err(crate::effects::ExecutionError::UnresolvableValue(_))
                    ));
                } else {
                    result.unwrap();
                    assert!(dm.pending);
                    assert_eq!(dm.calls, 2);
                }
                assert_eq!(game.stack.len(), 2);
                assert_eq!(game.stack[0].object_id, later);
                assert_eq!(game.stack[1].object_id, earlier);
                assert_eq!(game.object(earlier).unwrap().zone, Zone::Stack);
                assert_eq!(game.object(later).unwrap().zone, Zone::Stack);
                assert!(game.exile.is_empty());
                assert!(game.battlefield.is_empty());
                assert_eq!(game.player(alice).unwrap().life, 20);
                assert_eq!(game.effect_store.pending_trigger_events.len(), 1);
                assert_eq!(game.turn.priority_player, Some(alice));
                assert!(!game.turn_store.end_turn_procedure_pending);
                assert!(!game.turn_store.end_combat_phase_procedure_pending);
                assert!(
                    game.effect_store
                        .replacement_effects
                        .get_effect(ids.0)
                        .is_some()
                );
                assert!(
                    game.effect_store
                        .replacement_effects
                        .get_effect(ids.1)
                        .is_some()
                );
                if mode == 1 {
                    continue;
                }
                let mut replay = ProgramAnswers {
                    pause: false,
                    calls: 0,
                    pending: false,
                };
                let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut replay);
                execute_ending_prefix(&mut game, &mut ctx, combat).unwrap();
                drop(ctx);
                assert!(!replay.pending);
            } else {
                result.unwrap();
            }
            assert!(game.stack.is_empty());
            let earlier = game.find_object_by_stable_id(earlier_stable).unwrap();
            let later = game.find_object_by_stable_id(later_stable).unwrap();
            assert_eq!(game.object(earlier).unwrap().zone, Zone::Exile);
            assert_eq!(game.object(later).unwrap().zone, Zone::Battlefield);
            assert_eq!(
                game.player(alice).unwrap().life,
                if mode == 0 { 22 } else { 26 }
            );
            assert_eq!(game.turn.priority_player, None);
            assert_eq!(game.turn_store.end_turn_procedure_pending, !combat);
            assert_eq!(game.turn_store.end_combat_phase_procedure_pending, combat);
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(ids.0)
                    .is_none()
            );
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(ids.1)
                    .is_none()
            );
            let events = game.take_pending_trigger_events();
            assert_eq!(
                events
                    .iter()
                    .filter(|event| event.kind() == crate::events::EventKind::ZoneChange)
                    .count(),
                2
            );
            let gains = events
                .iter()
                .filter_map(|event| event.downcast::<crate::events::LifeGainEvent>())
                .collect::<Vec<_>>();
            assert_eq!(gains.len(), if mode == 0 { 1 } else { 3 });
            assert_eq!(
                gains.iter().map(|event| event.amount).sum::<u32>(),
                if mode == 0 { 2 } else { 6 }
            );
        }
    }
}

#[test]
fn graveyard_return_entry_failure_or_pause_restores_earlier_selected_card() {
    for mode in 0..3 {
        let (mut game, source, alice) = setup();
        let earlier = prefix_card(&mut game, alice, Zone::Graveyard, "Earlier return");
        let later = prefix_card(&mut game, alice, Zone::Graveyard, "Later entry");
        let earlier_stable = game.object(earlier).unwrap().stable_id;
        let later_stable = game.object(later).unwrap().stable_id;
        let snapshots = [earlier, later].map(|id| {
            crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                game.object(id).unwrap(),
                &game,
            )
        });
        game.take_pending_trigger_events();
        let ids = prefix_entry_replacements(
            &mut game,
            source,
            later,
            alice,
            Zone::Graveyard,
            Zone::Hand,
            mode,
        );
        let effect = Effect::return_from_graveyard_to_hand(crate::target::ChooseSpec::Tagged(
            "return prefix".into(),
        ));
        let mut dm = ProgramAnswers {
            pause: mode == 2,
            calls: 0,
            pending: false,
        };
        let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut dm);
        for snapshot in &snapshots {
            ctx.tag_object("return prefix", snapshot.clone());
        }
        let result = crate::effects::execute_effect(&mut game, &effect, &mut ctx);
        if mode != 0 {
            if mode == 1 {
                assert!(matches!(
                    result,
                    Err(crate::effects::ExecutionError::UnresolvableValue(_))
                ));
            } else {
                result.unwrap();
                assert!(ctx.decision_maker.awaiting_choice());
            }
            assert_eq!(game.object(earlier).unwrap().zone, Zone::Graveyard);
            assert_eq!(game.object(later).unwrap().zone, Zone::Graveyard);
            assert_eq!(game.player(alice).unwrap().life, 20);
            assert!(game.battlefield.is_empty());
            assert_eq!(game.player(alice).unwrap().hand.as_slice(), &[source]);
            assert!(game.take_pending_trigger_events().is_empty());
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(ids.0)
                    .is_some()
            );
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(ids.1)
                    .is_some()
            );
            assert_eq!(
                ctx.get_tagged_all("return prefix")
                    .unwrap()
                    .iter()
                    .map(|snapshot| snapshot.object_id)
                    .collect::<Vec<_>>(),
                vec![earlier, later]
            );
            drop(ctx);
            if mode == 1 {
                continue;
            }
            let mut replay = ProgramAnswers {
                pause: false,
                calls: 0,
                pending: false,
            };
            let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut replay);
            for snapshot in &snapshots {
                ctx.tag_object("return prefix", snapshot.clone());
            }
            crate::effects::execute_effect(&mut game, &effect, &mut ctx).unwrap();
        } else {
            result.unwrap();
        }
        let earlier = game.find_object_by_stable_id(earlier_stable).unwrap();
        let later = game.find_object_by_stable_id(later_stable).unwrap();
        assert_eq!(game.object(earlier).unwrap().zone, Zone::Hand);
        assert_eq!(game.object(later).unwrap().zone, Zone::Battlefield);
        assert_eq!(
            game.player(alice).unwrap().life,
            if mode == 0 { 22 } else { 26 }
        );
        assert!(game.player(alice).unwrap().graveyard.is_empty());
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(ids.0)
                .is_none()
        );
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(ids.1)
                .is_none()
        );
        let events = game.take_pending_trigger_events();
        assert_eq!(
            events
                .iter()
                .filter(|event| event.kind() == crate::events::EventKind::ZoneChange)
                .count(),
            2
        );
        assert_eq!(
            events
                .iter()
                .filter_map(|event| event.downcast::<crate::events::LifeGainEvent>())
                .map(|event| event.amount)
                .sum::<u32>(),
            if mode == 0 { 2 } else { 6 }
        );
    }
}

#[test]
fn shuffle_entry_redirect_commits_and_failure_or_pause_restores_the_library_prefix() {
    for mode in 0..3 {
        let (mut game, source, alice) = setup();
        prefix_card(&mut game, alice, Zone::Library, "Library first");
        prefix_card(&mut game, alice, Zone::Library, "Library second");
        let library_before = game.player(alice).unwrap().library.as_slice().to_vec();
        let earlier = prefix_card(&mut game, alice, Zone::Graveyard, "Earlier shuffle");
        let later = prefix_card(&mut game, alice, Zone::Graveyard, "Later entry");
        let earlier_stable = game.object(earlier).unwrap().stable_id;
        let later_stable = game.object(later).unwrap().stable_id;
        let snapshots = [earlier, later].map(|id| {
            crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                game.object(id).unwrap(),
                &game,
            )
        });
        game.take_pending_trigger_events();
        let ids = prefix_entry_replacements(
            &mut game,
            source,
            later,
            alice,
            Zone::Graveyard,
            Zone::Library,
            mode,
        );
        let effect = Effect::shuffle_objects_into_library(
            crate::target::ChooseSpec::Tagged("shuffle prefix".into()),
            crate::target::PlayerFilter::You,
        );
        let mut dm = ProgramAnswers {
            pause: mode == 2,
            calls: 0,
            pending: false,
        };
        let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut dm);
        for snapshot in &snapshots {
            ctx.tag_object("shuffle prefix", snapshot.clone());
        }
        let result = crate::effects::execute_effect(&mut game, &effect, &mut ctx);
        let completed = if mode != 0 {
            if mode == 1 {
                assert!(matches!(
                    result,
                    Err(crate::effects::ExecutionError::UnresolvableValue(_))
                ));
            } else {
                result.unwrap();
                assert!(ctx.decision_maker.awaiting_choice());
            }
            assert_eq!(
                game.player(alice).unwrap().library.as_slice(),
                library_before.as_slice()
            );
            assert_eq!(game.object(earlier).unwrap().zone, Zone::Graveyard);
            assert_eq!(game.object(later).unwrap().zone, Zone::Graveyard);
            assert_eq!(game.player(alice).unwrap().life, 20);
            assert!(game.battlefield.is_empty());
            assert!(game.take_pending_trigger_events().is_empty());
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(ids.0)
                    .is_some()
            );
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(ids.1)
                    .is_some()
            );
            assert_eq!(
                ctx.get_tagged_all("shuffle prefix")
                    .unwrap()
                    .iter()
                    .map(|snapshot| snapshot.object_id)
                    .collect::<Vec<_>>(),
                vec![earlier, later]
            );
            drop(ctx);
            if mode == 1 {
                continue;
            }
            let mut replay = ProgramAnswers {
                pause: false,
                calls: 0,
                pending: false,
            };
            let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut replay);
            for snapshot in &snapshots {
                ctx.tag_object("shuffle prefix", snapshot.clone());
            }
            crate::effects::execute_effect(&mut game, &effect, &mut ctx).unwrap()
        } else {
            result.unwrap()
        };
        let earlier = game.find_object_by_stable_id(earlier_stable).unwrap();
        let later = game.find_object_by_stable_id(later_stable).unwrap();
        assert_eq!(game.object(earlier).unwrap().zone, Zone::Library);
        assert_eq!(game.object(later).unwrap().zone, Zone::Battlefield);
        assert_eq!(
            game.player(alice).unwrap().library.len(),
            library_before.len() + 1
        );
        assert_eq!(
            game.player(alice).unwrap().life,
            if mode == 0 { 22 } else { 26 }
        );
        assert!(game.player(alice).unwrap().graveyard.is_empty());
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(ids.0)
                .is_none()
        );
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(ids.1)
                .is_none()
        );
        assert_eq!(
            completed
                .events
                .iter()
                .filter(|event| event.kind() == crate::events::EventKind::ShuffleLibrary)
                .count(),
            1
        );
        let crate::effect::OutcomeValue::Objects(shuffled) = completed.value else {
            panic!("one card actually reached the library");
        };
        assert_eq!(shuffled, vec![earlier]);
        let events = game.take_pending_trigger_events();
        assert_eq!(
            events
                .iter()
                .filter(|event| event.kind() == crate::events::EventKind::ZoneChange)
                .count(),
            2
        );
        assert_eq!(
            events
                .iter()
                .filter_map(|event| event.downcast::<crate::events::LifeGainEvent>())
                .map(|event| event.amount)
                .sum::<u32>(),
            if mode == 0 { 2 } else { 6 }
        );
    }
}

struct AuraSwapAnswers(ProgramAnswers);
impl crate::decision::DecisionMaker for AuraSwapAnswers {
    fn decide_objects(
        &mut self,
        _: &GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        ctx.candidates
            .iter()
            .filter(|candidate| candidate.legal)
            .take(1)
            .map(|candidate| candidate.id)
            .collect()
    }
    fn decide_boolean(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::BooleanContext,
    ) -> bool {
        crate::decision::DecisionMaker::decide_boolean(&mut self.0, game, ctx)
    }
    fn awaiting_choice(&self) -> bool {
        self.0.pending
    }
}

#[test]
fn aura_swap_entry_preserves_zone_scope_and_rolls_back_failure_or_pause() {
    for suppress_controller in [false, true] {
        for mode in 0..3 {
            let (mut game, _, alice) = setup();
            let bob = PlayerId::from_index(1);
            let target = prefix_card(&mut game, alice, Zone::Battlefield, "Enchanted target");
            let aura = crate::cards::CardDefinitionBuilder::new(CardId::new(), "Exchange Aura")
                .card_types(vec![crate::types::CardType::Enchantment])
                .subtypes(vec![crate::types::Subtype::Aura])
                .enchants(crate::target::ObjectFilter::creature())
                .build();
            let outgoing = game.create_object_from_definition(&aura, alice, Zone::Battlefield);
            let incoming = game.create_object_from_definition(&aura, alice, Zone::Hand);
            assert!(game.attach_object_to_target(
                outgoing,
                crate::object::AttachmentTarget::Object(target)
            ));
            let outgoing_stable = game.object(outgoing).unwrap().stable_id;
            let incoming_stable = game.object(incoming).unwrap().stable_id;
            let redirect = game.effect_store.replacement_effects.add_one_shot_effect(
                ReplacementEffect::with_matcher(
                    outgoing,
                    alice,
                    crate::events::zones::matchers::WouldChangeZoneMatcher::new(
                        crate::target::ObjectFilter::specific(outgoing),
                        Some(Zone::Battlefield),
                        Some(Zone::Hand),
                    ),
                    ReplacementAction::ChangeDestination(Zone::Exile),
                ),
            );
            // Keep this effect temporary: it must survive the zone-to-entry
            // handoff and also honor the execution context's suppression.
            let temporary = ReplacementEffect::with_matcher(
                incoming,
                alice,
                crate::events::zones::matchers::WouldChangeZoneMatcher::new(
                    crate::target::ObjectFilter::specific(incoming),
                    Some(Zone::Hand),
                    Some(Zone::Battlefield),
                ),
                ReplacementAction::EnterUnderControl(bob),
            );
            let temporary_key = temporary.application_key();
            let mut effects = vec![Effect::gain_life(2)];
            if mode == 1 {
                effects.push(Effect::lose_life(Value::X));
            }
            if mode == 2 {
                effects.push(Effect::may(vec![Effect::gain_life(1)]));
                effects.push(Effect::may(vec![Effect::gain_life(3)]));
            }
            let entry = game.effect_store.replacement_effects.add_one_shot_effect(
                ReplacementEffect::with_matcher(
                    incoming,
                    alice,
                    crate::events::zones::matchers::ThisWouldEnterBattlefieldMatcher,
                    ReplacementAction::AsEntersProgram(
                        crate::resolution::ResolutionProgram::from_effects(effects),
                    ),
                ),
            );
            game.take_pending_trigger_events();
            let mut dm = AuraSwapAnswers(ProgramAnswers {
                pause: mode == 2,
                calls: 0,
                pending: false,
            });
            let mut ctx = crate::effects::ExecutionContext::new(outgoing, alice, &mut dm);
            ctx.replacement
                .additional_replacement_effects
                .push(temporary.clone());
            if suppress_controller {
                ctx.replacement
                    .suppressed_replacement_effect_keys
                    .insert(temporary_key.clone());
            }
            let result = crate::effects::execute_effect(&mut game, &Effect::aura_swap(), &mut ctx);
            if mode != 0 {
                if mode == 1 {
                    assert!(matches!(
                        result,
                        Err(crate::effects::ExecutionError::UnresolvableValue(_))
                    ));
                } else {
                    result.unwrap();
                    assert!(ctx.decision_maker.awaiting_choice());
                }
                assert_eq!(game.object(outgoing).unwrap().zone, Zone::Battlefield);
                assert_eq!(
                    game.object(outgoing).unwrap().attached_to,
                    Some(crate::object::AttachmentTarget::Object(target))
                );
                assert_eq!(game.object(incoming).unwrap().zone, Zone::Hand);
                assert_eq!(game.player(alice).unwrap().life, 20);
                assert!(game.take_pending_trigger_events().is_empty());
                assert!(
                    game.effect_store
                        .replacement_effects
                        .get_effect(redirect)
                        .is_some()
                );
                assert!(
                    game.effect_store
                        .replacement_effects
                        .get_effect(entry)
                        .is_some()
                );
                assert_eq!(ctx.replacement.additional_replacement_effects.len(), 1);
                drop(ctx);
                if mode == 1 {
                    continue;
                }
                let mut replay = AuraSwapAnswers(ProgramAnswers {
                    pause: false,
                    calls: 0,
                    pending: false,
                });
                let mut ctx = crate::effects::ExecutionContext::new(outgoing, alice, &mut replay);
                ctx.replacement
                    .additional_replacement_effects
                    .push(temporary);
                if suppress_controller {
                    ctx.replacement
                        .suppressed_replacement_effect_keys
                        .insert(temporary_key);
                }
                crate::effects::execute_effect(&mut game, &Effect::aura_swap(), &mut ctx).unwrap();
            } else {
                result.unwrap();
            }
            let outgoing = game.find_object_by_stable_id(outgoing_stable).unwrap();
            let incoming = game.find_object_by_stable_id(incoming_stable).unwrap();
            assert_eq!(game.object(outgoing).unwrap().zone, Zone::Exile);
            assert_eq!(game.object(incoming).unwrap().zone, Zone::Battlefield);
            assert_eq!(
                game.current_controller(incoming),
                Some(if suppress_controller { alice } else { bob })
            );
            assert_eq!(
                game.object(incoming).unwrap().attached_to,
                Some(crate::object::AttachmentTarget::Object(target))
            );
            // This registered program belongs to Alice; changing the entrant's
            // controller must not change the replacement source's controller.
            assert_eq!(
                game.player(alice).unwrap().life,
                if mode == 0 { 22 } else { 26 }
            );
            assert_eq!(game.player(bob).unwrap().life, 20);
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(redirect)
                    .is_none()
            );
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(entry)
                    .is_none()
            );
            let events = game.take_pending_trigger_events();
            assert_eq!(
                events
                    .iter()
                    .filter(|event| event.kind() == crate::events::EventKind::ZoneChange)
                    .count(),
                2
            );
            assert_eq!(
                events
                    .iter()
                    .filter(|event| event.kind() == crate::events::EventKind::LifeGain)
                    .count(),
                if mode == 0 { 1 } else { 3 }
            );
        }
    }
}

#[test]
fn batch_entry_preserves_temporary_scope_cause_and_failure_or_pause() {
    for suppress_controller in [false, true] {
        for mode in 0..3 {
            let (mut game, source, alice) = setup();
            let bob = PlayerId::from_index(1);
            let first = prefix_card(&mut game, alice, Zone::Hand, "First batch entrant");
            let second = prefix_card(&mut game, alice, Zone::Hand, "Second batch entrant");
            let first_stable = game.object(first).unwrap().stable_id;
            let second_stable = game.object(second).unwrap().stable_id;
            let original_first_name = game.object(first).unwrap().name.clone();
            let original_second_name = game.object(second).unwrap().name.clone();
            let temporary = ReplacementEffect::with_matcher(
                source,
                alice,
                crate::events::zones::matchers::WouldChangeZoneMatcher::new(
                    crate::target::ObjectFilter::specific(second),
                    Some(Zone::Hand),
                    Some(Zone::Battlefield),
                )
                .with_cause_filter(
                    crate::events::cause::CauseFilter::effect_like()
                        .with_controller(crate::events::cause::ControllerFilter::Player(bob)),
                ),
                ReplacementAction::EnterUnderControl(bob),
            );
            let temporary_key = temporary.application_key();
            let first_program = game.effect_store.replacement_effects.add_one_shot_effect(
                ReplacementEffect::with_matcher(
                    first,
                    alice,
                    crate::events::zones::matchers::ThisWouldEnterBattlefieldMatcher,
                    ReplacementAction::AsEntersProgram(program(false)),
                ),
            );
            let mut effects = vec![Effect::gain_life(2)];
            if mode == 1 {
                effects.push(Effect::lose_life(Value::X));
            }
            if mode == 2 {
                effects.push(Effect::may(vec![Effect::gain_life(1)]));
                effects.push(Effect::may(vec![Effect::gain_life(3)]));
            }
            let second_program = game.effect_store.replacement_effects.add_one_shot_effect(
                ReplacementEffect::with_matcher(
                    second,
                    alice,
                    crate::events::zones::matchers::ThisWouldEnterBattlefieldMatcher,
                    ReplacementAction::AsEntersProgram(
                        crate::resolution::ResolutionProgram::from_effects(effects),
                    ),
                ),
            );
            let cause = crate::events::cause::EventCause::from_effect(source, bob);
            let requests = || {
                vec![
                    (
                        first,
                        crate::effects::zones::BattlefieldEntryOptions::specific(alice, false),
                    ),
                    (
                        second,
                        crate::effects::zones::BattlefieldEntryOptions::specific(alice, true),
                    ),
                ]
            };
            game.take_pending_trigger_events();
            let hand_before = game.player(alice).unwrap().hand.as_slice().to_vec();
            let mut dm = ProgramAnswers {
                pause: mode == 2,
                calls: 0,
                pending: false,
            };
            let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut dm);
            ctx.cause = cause.clone();
            ctx.replacement
                .additional_replacement_effects
                .push(temporary.clone());
            if suppress_controller {
                ctx.replacement
                    .suppressed_replacement_effect_keys
                    .insert(temporary_key.clone());
            }
            let result = crate::effects::zones::move_to_battlefield_batch_with_options(
                &mut game,
                &mut ctx,
                requests(),
            );
            let completed = if mode != 0 {
                if mode == 1 {
                    assert!(matches!(
                        result,
                        Err(crate::effects::ExecutionError::UnresolvableValue(_))
                    ));
                } else {
                    result.unwrap();
                    assert!(ctx.decision_maker.awaiting_choice());
                }
                assert_eq!(
                    game.player(alice).unwrap().hand.as_slice(),
                    hand_before.as_slice()
                );
                assert!(game.battlefield.is_empty());
                assert_eq!(game.player(alice).unwrap().life, 20);
                assert!(game.take_pending_trigger_events().is_empty());
                assert!(
                    game.effect_store
                        .replacement_effects
                        .get_effect(first_program)
                        .is_some()
                );
                assert!(
                    game.effect_store
                        .replacement_effects
                        .get_effect(second_program)
                        .is_some()
                );
                assert_eq!(ctx.cause.source_controller, cause.source_controller);
                assert_eq!(ctx.replacement.additional_replacement_effects.len(), 1);
                drop(ctx);
                if mode == 1 {
                    continue;
                }
                let mut replay = ProgramAnswers {
                    pause: false,
                    calls: 0,
                    pending: false,
                };
                let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut replay);
                ctx.cause = cause.clone();
                ctx.replacement
                    .additional_replacement_effects
                    .push(temporary);
                if suppress_controller {
                    ctx.replacement
                        .suppressed_replacement_effect_keys
                        .insert(temporary_key);
                }
                crate::effects::zones::move_to_battlefield_batch_with_options(
                    &mut game,
                    &mut ctx,
                    requests(),
                )
                .unwrap()
            } else {
                result.unwrap()
            };
            let first = game.find_object_by_stable_id(first_stable).unwrap();
            let second = game.find_object_by_stable_id(second_stable).unwrap();
            assert_eq!(
                completed
                    .iter()
                    .map(|receipt| receipt.assert_without_additions().clone())
                    .collect::<Vec<_>>(),
                vec![
                    crate::effects::zones::BattlefieldEntryOutcome::Moved(first),
                    crate::effects::zones::BattlefieldEntryOutcome::Moved(second),
                ]
            );
            assert_eq!(game.current_controller(first), Some(alice));
            assert_eq!(
                game.current_controller(second),
                Some(if suppress_controller { alice } else { bob })
            );
            assert!(!game.is_tapped(first));
            assert!(game.is_tapped(second));
            assert_eq!(
                game.player(alice).unwrap().life,
                if mode == 0 { 24 } else { 28 }
            );
            assert_eq!(game.player(bob).unwrap().life, 20);
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(first_program)
                    .is_none()
            );
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(second_program)
                    .is_none()
            );
            let events = game.take_pending_trigger_events();
            let changes = events
                .iter()
                .filter_map(|event| {
                    crate::events::downcast_event::<crate::events::ZoneChangeEvent>(event.inner())
                })
                .collect::<Vec<_>>();
            assert_eq!(changes.len(), 2);
            assert!(changes.iter().all(|change| change.from == Zone::Hand
                && change.to == Zone::Battlefield
                && change.cause.source_controller == cause.source_controller));
            assert_eq!(
                changes
                    .iter()
                    .map(|change| change.snapshot.as_ref().unwrap().name.clone())
                    .collect::<Vec<_>>(),
                vec![original_first_name, original_second_name]
            );
            assert_eq!(
                events
                    .iter()
                    .filter(|event| event.kind() == crate::events::EventKind::LifeGain)
                    .count(),
                if mode == 0 { 2 } else { 4 }
            );
        }
    }
}

#[test]
fn typed_zone_entry_phase_handoff_retains_history_temporary_effects_and_original_snapshot() {
    let (mut game, entrant, alice) = setup();
    let bob = PlayerId::from_index(1);
    let original_name = game.object(entrant).unwrap().name.clone();
    let redirect =
        game.effect_store
            .replacement_effects
            .add_one_shot_effect(ReplacementEffect::with_matcher(
                entrant,
                alice,
                crate::events::zones::matchers::WouldChangeZoneMatcher::new(
                    crate::target::ObjectFilter::specific(entrant),
                    Some(Zone::Hand),
                    Some(Zone::Graveyard),
                ),
                ReplacementAction::ChangeDestination(Zone::Battlefield),
            ));
    let redirect_key = game
        .effect_store
        .replacement_effects
        .get_effect(redirect)
        .unwrap()
        .application_key();
    let entry =
        game.effect_store
            .replacement_effects
            .add_one_shot_effect(ReplacementEffect::with_matcher(
                entrant,
                alice,
                crate::events::zones::matchers::ThisWouldEnterBattlefieldMatcher,
                ReplacementAction::AsEntersProgram(program(false)),
            ));
    let temporary = ReplacementEffect::with_matcher(
        entrant,
        alice,
        crate::events::zones::matchers::WouldChangeZoneMatcher::new(
            crate::target::ObjectFilter::specific(entrant),
            Some(Zone::Hand),
            Some(Zone::Battlefield),
        ),
        ReplacementAction::EnterUnderControl(bob),
    );
    let cause = crate::events::cause::EventCause::from_effect(entrant, alice);
    let mut dm = crate::decision::SelectFirstDecisionMaker;
    let EventOutcome::Proceed(PreparedZoneProposal::Battlefield(proposal)) =
        prepare_zone_change_proposal_scoped(
            &mut game,
            entrant,
            Zone::Hand,
            Zone::Graveyard,
            cause.clone(),
            &mut dm,
            &[temporary],
            None,
            None,
            None,
        )
        .map(require_plain_prepared_zone_outcome)
        .unwrap()
    else {
        panic!("expected an unresolved entry handoff");
    };
    assert_eq!(game.object(entrant).unwrap().zone, Zone::Hand);
    assert_eq!(game.player(alice).unwrap().life, 20);
    assert!(
        game.effect_store
            .replacement_effects
            .get_effect(entry)
            .is_some()
    );
    assert!(
        game.effect_store
            .replacement_effects
            .get_effect(redirect)
            .is_none()
    );
    assert!(proposal.context.applied_effects.contains(&redirect));
    assert!(proposal.context.applied_effect_keys.contains(&redirect_key));
    assert_eq!(proposal.additional_effects.len(), 1);
    game.object_mut(entrant).unwrap().name = "Changed after zone proposal".into();
    game.take_pending_trigger_events();
    let EventOutcome::Proceed(prepared) = complete_zone_entry_proposal(
        &mut game,
        entrant,
        proposal,
        &mut dm,
        vec![(CounterType::PlusOnePlusOne, 3)],
        None,
    )
    .map(require_plain_prepared_zone_outcome)
    .unwrap() else {
        panic!("entry must finish");
    };
    assert_eq!(game.object(entrant).unwrap().zone, Zone::Hand);
    let entered = commit_prepared_zone_change(&mut game, entrant, prepared, &mut dm)
        .unwrap()
        .assert_without_additions()
        .into_result()
        .expect("original zone commit must proceed");
    assert_eq!(game.object(entered).unwrap().zone, Zone::Battlefield);
    assert_eq!(game.current_controller(entered), Some(bob));
    assert_eq!(game.counter_count(entered, CounterType::PlusOnePlusOne), 3);
    assert_eq!(game.player(alice).unwrap().life, 22);
    assert!(
        game.effect_store
            .replacement_effects
            .get_effect(entry)
            .is_none()
    );
    let events = game.take_pending_trigger_events();
    let changes = events
        .iter()
        .filter_map(|event| {
            crate::events::downcast_event::<crate::events::ZoneChangeEvent>(event.inner())
        })
        .collect::<Vec<_>>();
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].snapshot.as_ref().unwrap().name, original_name);
    assert_eq!(changes[0].cause.source_controller, cause.source_controller);
    assert_eq!(
        events
            .iter()
            .filter(|event| event.kind() == crate::events::EventKind::LifeGain)
            .count(),
        1
    );
}

#[test]
fn general_move_entry_receipt_preserves_authored_options_history_and_rollback() {
    for redirected in [false, true] {
        for mode in 0..3 {
            let (mut game, source, alice) = setup();
            let bob = PlayerId::from_index(1);
            let first = prefix_card(&mut game, alice, Zone::Hand, "First movement entrant");
            let second = prefix_card(&mut game, alice, Zone::Hand, "Second movement entrant");
            let first_stable = game.object(first).unwrap().stable_id;
            let second_stable = game.object(second).unwrap().stable_id;
            let snapshots = [first, second].map(|id| {
                crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                    game.object(id).unwrap(),
                    &game,
                )
            });
            let destination = if redirected {
                Zone::Graveyard
            } else {
                Zone::Battlefield
            };
            let mut registrations = Vec::new();
            if redirected {
                for object in [first, second] {
                    registrations.push(game.effect_store.replacement_effects.add_one_shot_effect(
                        ReplacementEffect::with_matcher(
                            object,
                            alice,
                            crate::events::zones::matchers::WouldChangeZoneMatcher::new(
                                crate::target::ObjectFilter::specific(object),
                                Some(Zone::Hand),
                                Some(Zone::Graveyard),
                            ),
                            ReplacementAction::ChangeDestination(Zone::Battlefield),
                        ),
                    ));
                }
            }
            registrations.push(game.effect_store.replacement_effects.add_one_shot_effect(
                ReplacementEffect::with_matcher(
                    second,
                    alice,
                    crate::events::zones::matchers::WouldChangeZoneMatcher::new(
                        crate::target::ObjectFilter::specific(second),
                        Some(Zone::Hand),
                        Some(Zone::Battlefield),
                    ),
                    ReplacementAction::EnterUnderControl(bob),
                ),
            ));
            registrations.push(game.effect_store.replacement_effects.add_one_shot_effect(
                ReplacementEffect::with_matcher(
                    second,
                    alice,
                    crate::events::zones::matchers::ThisWouldEnterBattlefieldMatcher,
                    ReplacementAction::EnterUntapped,
                ),
            ));
            registrations.push(game.effect_store.replacement_effects.add_one_shot_effect(
                ReplacementEffect::with_matcher(
                    first,
                    alice,
                    crate::events::zones::matchers::ThisWouldEnterBattlefieldMatcher,
                    ReplacementAction::AsEntersProgram(program(false)),
                ),
            ));
            let mut effects = vec![Effect::gain_life(2)];
            if mode == 1 {
                effects.push(Effect::lose_life(Value::X));
            }
            if mode == 2 {
                effects.push(Effect::may(vec![Effect::gain_life(1)]));
                effects.push(Effect::may(vec![Effect::gain_life(3)]));
            }
            registrations.push(game.effect_store.replacement_effects.add_one_shot_effect(
                ReplacementEffect::with_matcher(
                    second,
                    alice,
                    crate::events::zones::matchers::ThisWouldEnterBattlefieldMatcher,
                    ReplacementAction::AsEntersProgram(
                        crate::resolution::ResolutionProgram::from_effects(effects),
                    ),
                ),
            ));
            let effect = Effect::new(
                crate::effects::MoveToZoneEffect::new(
                    crate::target::ChooseSpec::Tagged("entry movement".into()),
                    destination,
                    false,
                )
                .tapped()
                .under_you_control()
                .with_entry_counter(
                    ironsmith_core::BattlefieldEntryCounterSpec::new(
                        CounterType::PlusOnePlusOne,
                        3,
                        ironsmith_core::BattlefieldEntryCounterSurface::Inline,
                    ),
                ),
            );
            let cause = crate::events::cause::EventCause::from_effect(source, alice);
            game.take_pending_trigger_events();
            let hand_before = game.player(alice).unwrap().hand.as_slice().to_vec();
            let mut dm = ProgramAnswers {
                pause: mode == 2,
                calls: 0,
                pending: false,
            };
            let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut dm);
            ctx.cause = cause.clone();
            for snapshot in &snapshots {
                ctx.tag_object("entry movement", snapshot.clone());
            }
            let result = crate::effects::execute_effect(&mut game, &effect, &mut ctx);
            let completed = if mode != 0 {
                if mode == 1 {
                    assert!(matches!(
                        result,
                        Err(crate::effects::ExecutionError::UnresolvableValue(_))
                    ));
                } else {
                    result.unwrap();
                    assert!(ctx.decision_maker.awaiting_choice());
                }
                assert_eq!(
                    game.player(alice).unwrap().hand.as_slice(),
                    hand_before.as_slice()
                );
                assert!(game.battlefield.is_empty());
                assert_eq!(game.player(alice).unwrap().life, 20);
                assert!(game.take_pending_trigger_events().is_empty());
                assert!(registrations.iter().all(|id| {
                    game.effect_store
                        .replacement_effects
                        .get_effect(*id)
                        .is_some()
                }));
                assert_eq!(
                    ctx.get_tagged_all("entry movement")
                        .unwrap()
                        .iter()
                        .map(|snapshot| snapshot.object_id)
                        .collect::<Vec<_>>(),
                    vec![first, second]
                );
                drop(ctx);
                if mode == 1 {
                    continue;
                }
                let mut replay = ProgramAnswers {
                    pause: false,
                    calls: 0,
                    pending: false,
                };
                let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut replay);
                ctx.cause = cause.clone();
                for snapshot in &snapshots {
                    ctx.tag_object("entry movement", snapshot.clone());
                }
                crate::effects::execute_effect(&mut game, &effect, &mut ctx).unwrap()
            } else {
                result.unwrap()
            };
            let first = game.find_object_by_stable_id(first_stable).unwrap();
            let second = game.find_object_by_stable_id(second_stable).unwrap();
            assert_eq!(game.object(first).unwrap().zone, Zone::Battlefield);
            assert_eq!(game.object(second).unwrap().zone, Zone::Battlefield);
            assert_eq!(game.current_controller(first), Some(alice));
            assert_eq!(game.current_controller(second), Some(bob));
            assert!(game.is_tapped(first));
            assert!(!game.is_tapped(second));
            for object in [first, second] {
                assert_eq!(game.counter_count(object, CounterType::PlusOnePlusOne), 3);
            }
            assert_eq!(
                game.player(alice).unwrap().life,
                if mode == 0 { 24 } else { 28 }
            );
            assert!(registrations.iter().all(|id| {
                game.effect_store
                    .replacement_effects
                    .get_effect(*id)
                    .is_none()
            }));
            let crate::effect::OutcomeValue::Objects(ids) = completed.value else {
                panic!("expected actual moved objects");
            };
            assert_eq!(ids, vec![first, second]);
            let events = game.take_pending_trigger_events();
            let changes = events
                .iter()
                .filter_map(|event| {
                    crate::events::downcast_event::<crate::events::ZoneChangeEvent>(event.inner())
                })
                .collect::<Vec<_>>();
            assert_eq!(changes.len(), 2);
            assert!(changes.iter().all(|change| change.from == Zone::Hand
                && change.to == Zone::Battlefield
                && change.cause.source_controller == cause.source_controller));
            assert_eq!(
                events
                    .iter()
                    .filter(|event| event.kind() == crate::events::EventKind::LifeGain)
                    .count(),
                if mode == 0 { 2 } else { 4 }
            );
        }
    }
}

#[test]
fn general_move_nonbattlefield_counters_resolve_original_filter_and_apply_replacements() {
    let (mut game, entrant, alice) = setup();
    let original_stable = game.object(entrant).unwrap().stable_id;
    let doubler = game.effect_store.replacement_effects.add_resolution_effect(
        ReplacementEffect::with_matcher(
            entrant,
            alice,
            crate::events::counters::matchers::WouldPutCountersMatcher::new(
                crate::target::ObjectFilter::default(),
                None,
            ),
            ReplacementAction::DoubleCounters { counter_type: None },
        ),
    );
    let effect = Effect::new(
        crate::effects::MoveToZoneEffect::new(
            crate::target::ChooseSpec::SpecificObject(entrant),
            Zone::Exile,
            false,
        )
        .with_entry_counter(
            ironsmith_core::BattlefieldEntryCounterSpec::new(
                CounterType::Time,
                Value::X,
                ironsmith_core::BattlefieldEntryCounterSurface::Inline,
            )
            .for_matching_object(crate::target::ObjectFilter::specific(entrant)),
        ),
    );
    game.take_pending_trigger_events();
    let mut dm = crate::decision::SelectFirstDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(entrant, alice, &mut dm).with_x(3);
    let result = crate::effects::execute_effect(&mut game, &effect, &mut ctx).unwrap();
    let moved = game.find_object_by_stable_id(original_stable).unwrap();
    assert_eq!(game.object(moved).unwrap().zone, Zone::Exile);
    assert_eq!(game.counter_count(moved, CounterType::Time), 6);
    assert!(
        game.effect_store
            .replacement_effects
            .get_effect(doubler)
            .is_some()
    );
    let crate::effect::OutcomeValue::Objects(ids) = result.value else {
        panic!("expected actual exile result");
    };
    assert_eq!(ids, vec![moved]);
    let events = game.take_pending_trigger_events();
    assert_eq!(
        events
            .iter()
            .filter(|event| event.kind() == crate::events::EventKind::ZoneChange)
            .count(),
        1
    );
    let changes = events
        .iter()
        .filter_map(|event| {
            crate::events::downcast_event::<crate::events::MarkersChangedEvent>(event.inner())
        })
        .collect::<Vec<_>>();
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].amount, 6);
}

fn transformed_entry_card(zone: Zone) -> (GameState, ObjectId, PlayerId) {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let front_id = CardId::from_raw(9_100_001);
    let back_id = CardId::from_raw(9_100_002);
    let front = crate::cards::CardDefinitionBuilder::new(front_id, "Prepared entry front")
        .card_types(vec![crate::types::CardType::Creature])
        .subtypes(vec![crate::types::Subtype::Human])
        .power_toughness(crate::card::PowerToughness::fixed(2, 2))
        .other_face(back_id)
        .other_face_name("Prepared entry back")
        .linked_face_layout(crate::card::LinkedFaceLayout::TransformLike)
        .build();
    let back = crate::cards::CardDefinitionBuilder::new(back_id, "Prepared entry back")
        .card_types(vec![crate::types::CardType::Creature])
        .subtypes(vec![crate::types::Subtype::Wolf])
        .power_toughness(crate::card::PowerToughness::fixed(4, 4))
        .other_face(front_id)
        .other_face_name("Prepared entry front")
        .linked_face_layout(crate::card::LinkedFaceLayout::TransformLike)
        .build();
    game.register_linked_face_definition(&front);
    game.register_linked_face_definition(&back);
    let entrant = game.create_object_from_definition(&front, alice, zone);
    game.take_pending_trigger_events();
    (game, entrant, alice)
}

#[test]
fn transformed_entry_face_does_not_overwrite_resolved_copy_or_characteristics() {
    for zone in [Zone::Hand, Zone::Exile] {
        for copy_mode in 0..3 {
            let (mut game, entrant, alice) = transformed_entry_card(zone);
            let bob = PlayerId::from_index(1);
            let template =
                crate::card::CardBuilder::new(CardId::new(), "Replacement copy template")
                    .card_types(vec![
                        crate::types::CardType::Artifact,
                        crate::types::CardType::Creature,
                    ])
                    .subtypes(vec![crate::types::Subtype::Golem])
                    .power_toughness(crate::card::PowerToughness::fixed(6, 6))
                    .build();
            let copy_source = game.create_object_from_card(&template, alice, Zone::Battlefield);
            let replacement = if copy_mode == 0 {
                ReplacementAction::EnterWithCharacteristics {
                    added_card_types: vec![crate::types::CardType::Artifact],
                    added_subtypes: vec![crate::types::Subtype::Zombie],
                    set_base_power_toughness: Some((7, 8)),
                }
            } else {
                ReplacementAction::EnterAsCopy {
                    source: copy_source,
                    enters_tapped: false,
                    copy_duration: (copy_mode == 2).then_some(crate::effect::Until::EndOfTurn),
                    linked_exile_objects: Vec::new(),
                    additional_counters: vec![(CounterType::PlusOnePlusOne, 4)],
                    name_override: None,
                    added_colors: crate::color::ColorSet::new(),
                    added_card_types: Vec::new(),
                    removes_other_card_types: false,
                    added_supertypes: Vec::new(),
                    removed_supertypes: Vec::new(),
                    added_subtypes: vec![crate::types::Subtype::Zombie],
                    added_abilities: Vec::new(),
                    set_base_power_toughness: Some((7, 8)),
                    copy_followups: Vec::new(),
                }
            };
            let identity = game.effect_store.replacement_effects.add_one_shot_effect(
                ReplacementEffect::with_matcher(
                    entrant,
                    alice,
                    crate::events::zones::matchers::ThisWouldEnterBattlefieldMatcher,
                    replacement,
                ),
            );
            let control = game.effect_store.replacement_effects.add_one_shot_effect(
                ReplacementEffect::with_matcher(
                    entrant,
                    alice,
                    crate::events::zones::matchers::ThisWouldEnterBattlefieldMatcher,
                    ReplacementAction::EnterUnderControl(bob),
                ),
            );
            game.take_pending_trigger_events();
            let mut dm = crate::decision::SelectFirstDecisionMaker;
            let mut ctx = crate::effects::ExecutionContext::new(entrant, alice, &mut dm);
            let outcome = crate::effects::zones::move_to_battlefield_with_options(
                &mut game,
                &mut ctx,
                entrant,
                crate::effects::zones::BattlefieldEntryOptions::specific(alice, false)
                    .transformed(true)
                    .with_initial_counters(vec![(CounterType::PlusOnePlusOne, 2)]),
            )
            .unwrap();
            let crate::effects::zones::BattlefieldEntryOutcome::Moved(entered) = outcome
                .as_ref()
                .expect("completed entry receipt")
                .assert_without_additions()
                .clone()
            else {
                panic!("must enter");
            };
            assert_eq!(game.current_controller(entered), Some(bob));
            // Counters modify the resolved 7/8 base in the ordinary layer order.
            let count = if copy_mode == 0 { 2 } else { 6 };
            assert_eq!(
                game.counter_count(entered, CounterType::PlusOnePlusOne),
                count
            );
            assert_eq!(game.current_power(entered), Some(7 + count as i32));
            assert_eq!(game.current_toughness(entered), Some(8 + count as i32));
            assert!(
                game.current_card_types(entered)
                    .unwrap()
                    .contains(&crate::types::CardType::Artifact)
            );
            let subtypes = game.current_subtypes(entered).unwrap();
            assert!(subtypes.contains(&crate::types::Subtype::Zombie));
            assert_eq!(
                subtypes.contains(&crate::types::Subtype::Wolf),
                copy_mode == 0
            );
            assert_eq!(
                game.current_name(entered).as_deref(),
                Some(if copy_mode == 0 {
                    "Prepared entry back"
                } else {
                    "Replacement copy template"
                })
            );
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(identity)
                    .is_none()
            );
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(control)
                    .is_none()
            );
            let events = game.take_pending_trigger_events();
            let changes = events
                .iter()
                .filter_map(|event| {
                    crate::events::downcast_event::<crate::events::ZoneChangeEvent>(event.inner())
                })
                .collect::<Vec<_>>();
            assert_eq!(changes.len(), 1);
            assert_eq!(
                changes[0].snapshot.as_ref().unwrap().name.to_string(),
                "Prepared entry front"
            );
        }
    }
}

#[test]
fn transformed_entry_redirect_restores_default_face_without_applying_battlefield_definition() {
    for zone in [Zone::Hand, Zone::Graveyard] {
        let (mut game, entrant, alice) = transformed_entry_card(zone);
        let stable = game.object(entrant).unwrap().stable_id;
        let redirect = game.effect_store.replacement_effects.add_one_shot_effect(
            ReplacementEffect::with_matcher(
                entrant,
                alice,
                crate::events::zones::matchers::ThisWouldEnterBattlefieldMatcher,
                ReplacementAction::ChangeDestination(Zone::Exile),
            ),
        );
        let mut dm = crate::decision::SelectFirstDecisionMaker;
        let mut ctx = crate::effects::ExecutionContext::new(entrant, alice, &mut dm);
        let outcome = crate::effects::zones::move_to_battlefield_with_options(
            &mut game,
            &mut ctx,
            entrant,
            crate::effects::zones::BattlefieldEntryOptions::specific(alice, false)
                .transformed(true),
        )
        .unwrap();
        assert!(
            matches!(outcome.as_ref().expect("completed redirected entry receipt").assert_without_additions(), crate::effects::zones::BattlefieldEntryOutcome::Redirected(receipt) if receipt.final_zone == Zone::Exile && receipt.new_object_ids.len() == 1)
        );
        let moved = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.object(moved).unwrap().zone, Zone::Exile);
        assert_eq!(
            game.current_name(moved).as_deref(),
            Some("Prepared entry front")
        );
        assert!(
            game.current_subtypes(moved)
                .unwrap()
                .contains(&crate::types::Subtype::Human)
        );
        assert!(
            !game
                .current_subtypes(moved)
                .unwrap()
                .contains(&crate::types::Subtype::Wolf)
        );
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(redirect)
                .is_none()
        );
        let events = game.take_pending_trigger_events();
        assert_eq!(
            events
                .iter()
                .filter(|event| event.kind() == crate::events::EventKind::ZoneChange)
                .count(),
            1
        );
    }
}

#[test]
fn general_move_entry_redirect_reports_the_actual_committed_object() {
    let (mut game, source, alice) = setup();
    let entrant = prefix_card(&mut game, alice, Zone::Hand, "Redirected entry result");
    let stable = game.object(entrant).unwrap().stable_id;
    let redirect =
        game.effect_store
            .replacement_effects
            .add_one_shot_effect(ReplacementEffect::with_matcher(
                entrant,
                alice,
                crate::events::zones::matchers::ThisWouldEnterBattlefieldMatcher,
                ReplacementAction::ChangeDestination(Zone::Exile),
            ));
    let effect = Effect::new(crate::effects::MoveToZoneEffect::new(
        crate::target::ChooseSpec::SpecificObject(entrant),
        Zone::Battlefield,
        false,
    ));
    game.take_pending_trigger_events();
    let mut dm = crate::decision::SelectFirstDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut dm);
    let outcome = crate::effects::execute_effect(&mut game, &effect, &mut ctx).unwrap();
    let moved = game.find_object_by_stable_id(stable).unwrap();
    // First establish real movement, independently of its returned disposition.
    assert_ne!(moved, entrant);
    assert_eq!(game.object(moved).unwrap().zone, Zone::Exile);
    assert!(
        game.effect_store
            .replacement_effects
            .get_effect(redirect)
            .is_none()
    );
    let events = game.take_pending_trigger_events();
    assert_eq!(
        events
            .iter()
            .filter(|event| event.kind() == crate::events::EventKind::ZoneChange)
            .count(),
        1
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| event.kind() == crate::events::EventKind::EnterBattlefield)
            .count(),
        0
    );
    let crate::effect::OutcomeValue::Objects(ids) = &outcome.value else {
        panic!("a committed redirected move must retain its actual object result: {outcome:?}");
    };
    assert_eq!(ids, &vec![moved]);
}

#[test]
fn redirected_entry_does_not_install_battlefield_only_continuous_modifications() {
    for redirect in [false, true] {
        let (mut game, entrant, alice) = setup();
        let stable_id = game.object(entrant).unwrap().stable_id;
        if redirect {
            game.effect_store.replacement_effects.add_one_shot_effect(
                ReplacementEffect::with_matcher(
                    entrant,
                    alice,
                    crate::events::zones::matchers::WouldChangeZoneMatcher::new(
                        crate::target::ObjectFilter::specific(entrant),
                        Some(Zone::Hand),
                        Some(Zone::Battlefield),
                    ),
                    ReplacementAction::ChangeDestination(Zone::Exile),
                ),
            );
        }
        let mut ctx = crate::effects::ExecutionContext::new_default(entrant, alice);
        crate::effects::zones::move_to_battlefield_with_options(
            &mut game,
            &mut ctx,
            entrant,
            crate::effects::zones::BattlefieldEntryOptions::specific(alice, false)
                .with_entry_modifications(vec![crate::continuous::Modification::AddCardTypes(
                    vec![crate::types::CardType::Artifact],
                )]),
        )
        .unwrap();
        let actual = game.find_object_by_stable_id(stable_id).unwrap();
        assert_eq!(
            game.object(actual).unwrap().zone,
            if redirect {
                Zone::Exile
            } else {
                Zone::Battlefield
            }
        );
        let effects = game
            .effect_store
            .continuous_effects
            .effects_for_object(actual);
        assert_eq!(effects.len(), if redirect { 0 } else { 1 });
        if !redirect {
            assert!(
                game.calculated_characteristics(actual)
                    .unwrap()
                    .card_types
                    .contains(&crate::types::CardType::Artifact)
            );
        }
    }
}

#[test]
fn entry_redirect_back_to_library_keeps_object_identity_and_state() {
    use crate::effects::EffectExecutor;
    let (mut game, entrant, alice) = setup();
    let entrant = game.move_object_by_effect(entrant, Zone::Library).unwrap();
    game.object_mut(entrant)
        .unwrap()
        .add_counters(CounterType::Time, 3);
    let original_library = game.player(alice).unwrap().library.clone();
    let replacement =
        game.effect_store
            .replacement_effects
            .add_one_shot_effect(ReplacementEffect::with_matcher(
                entrant,
                alice,
                crate::events::zones::matchers::WouldChangeZoneMatcher::new(
                    crate::target::ObjectFilter::specific(entrant),
                    Some(Zone::Library),
                    Some(Zone::Battlefield),
                ),
                ReplacementAction::ChangeDestination(Zone::Library),
            ));
    game.take_pending_trigger_events();
    let mut ctx = crate::effects::ExecutionContext::new_default(entrant, alice);
    let outcome = crate::effects::zones::MoveToZoneEffect::new(
        crate::target::ChooseSpec::SpecificObject(entrant),
        Zone::Battlefield,
        false,
    )
    .execute(&mut game, &mut ctx)
    .unwrap();
    // CR400.7 gives a new object only on a zone change;400.8/400.10
    // specify the same-zone exile/command exceptions, not a library exception.
    assert_eq!(
        game.object(entrant).map(|object| object.zone),
        Some(Zone::Library)
    );
    assert_eq!(game.counter_count(entrant, CounterType::Time), 3);
    assert_eq!(game.player(alice).unwrap().library, original_library);
    let performed_count = match &outcome.value {
        crate::effect::OutcomeValue::None => 0,
        crate::effect::OutcomeValue::Count(count) => *count,
        crate::effect::OutcomeValue::Objects(ids) => i64::try_from(ids.len()).unwrap(),
        value => panic!("unexpected movement result {value:?}"),
    };
    assert_eq!(
        performed_count, 0,
        "a redirect that cannot move reports no performed movement"
    );
    assert!(game.battlefield.is_empty());
    assert!(
        game.effect_store
            .replacement_effects
            .get_effect(replacement)
            .is_none()
    );
    assert!(game.take_pending_trigger_events().is_empty());
}

#[test]
fn same_battlefield_destination_does_not_run_entry_programs_or_reset_state() {
    use crate::effects::EffectExecutor;
    for redirected in [false, true] {
        let (mut game, entrant, alice) = setup();
        let entrant = game
            .move_object_by_effect(entrant, Zone::Battlefield)
            .unwrap();
        game.object_mut(entrant)
            .unwrap()
            .add_counters(CounterType::Time, 3);
        game.tap(entrant);
        let original_battlefield = game.battlefield.clone();
        let entry_program = game.effect_store.replacement_effects.add_one_shot_effect(
            ReplacementEffect::with_matcher(
                entrant,
                alice,
                crate::events::zones::matchers::ThisWouldEnterBattlefieldMatcher,
                ReplacementAction::AsEntersProgram(program(false)),
            ),
        );
        let redirect = redirected.then(|| {
            game.effect_store.replacement_effects.add_one_shot_effect(
                ReplacementEffect::with_matcher(
                    entrant,
                    alice,
                    crate::events::zones::matchers::WouldChangeZoneMatcher::new(
                        crate::target::ObjectFilter::specific(entrant),
                        Some(Zone::Battlefield),
                        Some(Zone::Hand),
                    ),
                    ReplacementAction::ChangeDestination(Zone::Battlefield),
                ),
            )
        });
        game.take_pending_trigger_events();
        let mut ctx = crate::effects::ExecutionContext::new_default(entrant, alice);
        let outcome = crate::effects::zones::MoveToZoneEffect::new(
            crate::target::ChooseSpec::SpecificObject(entrant),
            if redirected {
                Zone::Hand
            } else {
                Zone::Battlefield
            },
            false,
        )
        .execute(&mut game, &mut ctx)
        .unwrap();
        assert_eq!(
            game.player(alice).unwrap().life,
            20,
            "no entry means no as-enters program"
        );
        assert_eq!(
            game.object(entrant).map(|object| object.zone),
            Some(Zone::Battlefield)
        );
        assert_eq!(game.counter_count(entrant, CounterType::Time), 3);
        assert!(game.is_tapped(entrant));
        assert_eq!(game.battlefield, original_battlefield);
        let performed_count = match &outcome.value {
            crate::effect::OutcomeValue::None => 0,
            crate::effect::OutcomeValue::Count(count) => *count,
            crate::effect::OutcomeValue::Objects(ids) => i64::try_from(ids.len()).unwrap(),
            value => panic!("unexpected movement result {value:?}"),
        };
        assert_eq!(performed_count, 0);
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(entry_program)
                .is_some()
        );
        if let Some(redirect) = redirect {
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(redirect)
                    .is_none()
            );
        }
        assert!(game.take_pending_trigger_events().is_empty());
    }
}

#[test]
fn same_zone_primitive_preserves_state_except_explicit_identity_renewal_zones() {
    for zone in [
        Zone::Hand,
        Zone::Library,
        Zone::Graveyard,
        Zone::Battlefield,
        Zone::Stack,
        Zone::Exile,
        Zone::Command,
    ] {
        let (mut game, entrant, _) = setup();
        let entrant = if zone == Zone::Hand {
            entrant
        } else {
            game.move_object_by_effect(entrant, zone).unwrap()
        };
        let stable_id = game.object(entrant).unwrap().stable_id;
        game.object_mut(entrant)
            .unwrap()
            .add_counters(CounterType::Time, 3);
        game.take_pending_trigger_events();
        let result = game.move_object_by_effect(entrant, zone).unwrap();
        let renews_identity = matches!(zone, Zone::Exile | Zone::Command);
        assert_eq!(
            result != entrant,
            renews_identity,
            "same-zone identity policy for {zone:?}"
        );
        assert_eq!(game.object(result).unwrap().stable_id, stable_id);
        assert_eq!(game.object(result).unwrap().zone, zone);
        if !renews_identity {
            assert_eq!(game.counter_count(result, CounterType::Time), 3);
        }
        assert!(game.take_pending_trigger_events().is_empty());
    }
}

#[test]
fn library_position_instruction_preserves_identity_and_counters() {
    use crate::effects::EffectExecutor;
    for to_top in [false, true] {
        let (mut game, entrant, alice) = setup();
        let entrant = game.move_object_by_effect(entrant, Zone::Library).unwrap();
        let card = crate::card::CardBuilder::new(CardId::new(), "Other library card")
            .card_types(vec![crate::types::CardType::Creature])
            .build();
        let other = game.create_object_from_card(&card, alice, Zone::Library);
        // Force the target away from the requested end so this checks actual positioning.
        game.set_player_library_order_with_audit(
            alice,
            if to_top {
                vec![entrant, other]
            } else {
                vec![other, entrant]
            },
            "same-zone positioning test setup",
        );
        game.object_mut(entrant)
            .unwrap()
            .add_counters(CounterType::Time, 3);
        game.take_pending_trigger_events();
        let mut ctx = crate::effects::ExecutionContext::new_default(entrant, alice);
        let outcome = crate::effects::zones::MoveToZoneEffect::new(
            crate::target::ChooseSpec::SpecificObject(entrant),
            Zone::Library,
            to_top,
        )
        .execute(&mut game, &mut ctx)
        .unwrap();
        assert_eq!(outcome.status, crate::effect::OutcomeStatus::Succeeded);
        assert!(matches!(
            outcome.value,
            crate::effect::OutcomeValue::Count(0)
        ));
        assert_eq!(
            game.object(entrant).map(|object| object.zone),
            Some(Zone::Library)
        );
        assert_eq!(game.counter_count(entrant, CounterType::Time), 3);
        assert_eq!(
            game.player(alice).unwrap().library,
            if to_top {
                vec![other, entrant]
            } else {
                vec![entrant, other]
            }
        );
        assert!(game.take_pending_trigger_events().is_empty());
    }
}

struct ReservedEntryProgramChoices {
    forbidden: Vec<ObjectId>,
    offered: Vec<Vec<ObjectId>>,
    pause_second: bool,
    pending: bool,
}
impl crate::decision::DecisionMaker for ReservedEntryProgramChoices {
    fn decide_objects(
        &mut self,
        _: &GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        let candidates = ctx
            .candidates
            .iter()
            .filter(|candidate| candidate.legal)
            .map(|candidate| candidate.id)
            .collect::<Vec<_>>();
        assert!(
            candidates.iter().all(|id| !self.forbidden.contains(id)),
            "CR614.13a: simultaneous entrants cannot be chosen to change zones"
        );
        self.offered.push(candidates.clone());
        if self.pause_second && self.offered.len() == 2 {
            self.pending = true;
            return Vec::new();
        }
        candidates.into_iter().take(1).collect()
    }
    fn awaiting_choice(&self) -> bool {
        self.pending
    }
}

#[test]
fn generic_entry_discard_programs_exclude_all_simultaneous_entrants() {
    check_generic_entry_discard_reservations(false);
}

#[test]
fn pending_reserved_entry_discard_restores_first_payment_before_replay() {
    check_generic_entry_discard_reservations(true);
}

fn check_generic_entry_discard_reservations(pause_second: bool) {
    let (mut game, first, alice) = setup();
    let second = prefix_card(&mut game, alice, Zone::Hand, "Second simultaneous entrant");
    let spare_a = prefix_card(&mut game, alice, Zone::Hand, "First discardable card");
    let spare_b = prefix_card(&mut game, alice, Zone::Hand, "Second discardable card");
    let spare_c = prefix_card(&mut game, alice, Zone::Hand, "Unchosen discardable card");
    let spare_stables = [spare_a, spare_b].map(|id| game.object(id).unwrap().stable_id);
    let entry_stables = [first, second].map(|id| game.object(id).unwrap().stable_id);
    let mut shields = Vec::new();
    for entrant in [first, second] {
        shields.push(game.effect_store.replacement_effects.add_one_shot_effect(
            ReplacementEffect::with_matcher(
                entrant,
                alice,
                crate::events::zones::matchers::ThisWouldEnterBattlefieldMatcher,
                ReplacementAction::AsEntersProgram(
                    crate::resolution::ResolutionProgram::from_effects(vec![Effect::discard(1)]),
                ),
            ),
        ));
    }
    game.take_pending_trigger_events();
    let requests = || {
        vec![
            (
                first,
                crate::effects::zones::BattlefieldEntryOptions::preserve(false),
            ),
            (
                second,
                crate::effects::zones::BattlefieldEntryOptions::preserve(false),
            ),
        ]
    };
    if pause_second {
        let hand_before = game.player(alice).unwrap().hand.as_slice().to_vec();
        let mut paused = ReservedEntryProgramChoices {
            forbidden: vec![first, second],
            offered: Vec::new(),
            pause_second: true,
            pending: false,
        };
        let mut ctx = crate::effects::ExecutionContext::new(first, alice, &mut paused);
        let outcomes = crate::effects::zones::move_to_battlefield_batch_with_options(
            &mut game,
            &mut ctx,
            requests(),
        )
        .unwrap();
        assert!(ctx.decision_maker.awaiting_choice());
        assert!(outcomes.iter().all(|result| matches!(
            result.assert_without_additions(),
            crate::effects::zones::BattlefieldEntryOutcome::Prevented
        )));
        drop(ctx);
        assert_eq!(paused.offered.len(), 2);
        assert_eq!(paused.offered[0].len(), 3);
        assert_eq!(paused.offered[1].len(), 2);
        assert_eq!(
            game.player(alice).unwrap().hand.as_slice(),
            hand_before.as_slice()
        );
        assert!(game.player(alice).unwrap().graveyard.is_empty());
        assert!(game.battlefield.is_empty());
        assert!(game.take_pending_trigger_events().is_empty());
        assert!(shields.iter().all(|id| {
            game.effect_store
                .replacement_effects
                .get_effect(*id)
                .is_some()
        }));
    }
    let mut dm = ReservedEntryProgramChoices {
        forbidden: vec![first, second],
        offered: Vec::new(),
        pause_second: false,
        pending: false,
    };
    let mut ctx = crate::effects::ExecutionContext::new(first, alice, &mut dm);
    let outcomes = crate::effects::zones::move_to_battlefield_batch_with_options(
        &mut game,
        &mut ctx,
        requests(),
    )
    .expect("both entry programs choose available nonentrant cards");
    assert!(outcomes.iter().all(|result| matches!(
        result.assert_without_additions(),
        crate::effects::zones::BattlefieldEntryOutcome::Moved(_)
    )));
    assert!(!ctx.decision_maker.awaiting_choice());
    drop(ctx);
    assert_eq!(dm.offered.len(), 2);
    assert_eq!(dm.offered[0].len(), 3);
    assert_eq!(
        dm.offered[1].len(),
        2,
        "one physical card cannot pay both entry choices"
    );
    for stable in entry_stables {
        let id = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.object(id).unwrap().zone, Zone::Battlefield);
    }
    for stable in spare_stables {
        let id = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.object(id).unwrap().zone, Zone::Graveyard);
    }
    assert_eq!(game.player(alice).unwrap().hand.as_slice(), &[spare_c]);
    assert_eq!(game.object(spare_c).unwrap().zone, Zone::Hand);
    assert_eq!(game.player(alice).unwrap().graveyard.len(), 2);
    assert!(shields.into_iter().all(|id| {
        game.effect_store
            .replacement_effects
            .get_effect(id)
            .is_none()
    }));
    let events = game.take_pending_trigger_events();
    assert_eq!(
        events
            .iter()
            .filter(|event| event.kind() == crate::events::EventKind::EnterBattlefield)
            .count(),
        2
    );
}

#[test]
fn nested_entry_replacement_mill_skips_simultaneous_library_entrants() {
    check_nested_entry_library_reservations(false);
}

#[test]
fn nested_entry_replacement_top_exile_skips_simultaneous_library_entrants() {
    check_nested_entry_library_reservations(true);
}

fn check_nested_entry_library_reservations(exile_top: bool) {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let spare_a = prefix_card(&mut game, alice, Zone::Library, "First millable card");
    let spare_b = prefix_card(&mut game, alice, Zone::Library, "Second millable card");
    let first = prefix_card(&mut game, alice, Zone::Library, "First library entrant");
    let second = prefix_card(&mut game, alice, Zone::Library, "Second library entrant");
    game.set_player_library_order_with_audit(
        alice,
        vec![spare_a, spare_b, second, first],
        "entry reservation fixture",
    );
    let entry_stables = [first, second].map(|id| game.object(id).unwrap().stable_id);
    let spare_stables = [spare_a, spare_b].map(|id| game.object(id).unwrap().stable_id);
    let entry =
        game.effect_store
            .replacement_effects
            .add_one_shot_effect(ReplacementEffect::with_matcher(
                first,
                alice,
                crate::events::zones::matchers::ThisWouldEnterBattlefieldMatcher,
                ReplacementAction::AsEntersProgram(
                    crate::resolution::ResolutionProgram::from_effects(vec![Effect::lose_life(1)]),
                ),
            ));
    let mill =
        game.effect_store
            .replacement_effects
            .add_one_shot_effect(ReplacementEffect::with_matcher(
                first,
                alice,
                crate::events::life::matchers::WouldLoseLifeMatcher::you(),
                ReplacementAction::Instead(vec![if exile_top {
                    Effect::exile_top_of_library_player(2, crate::target::PlayerFilter::You)
                } else {
                    Effect::mill(2)
                }]),
            ));
    game.take_pending_trigger_events();
    let mut dm = crate::decision::SelectFirstDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(first, alice, &mut dm);
    let outcomes = crate::effects::zones::move_to_battlefield_batch_with_options(
        &mut game,
        &mut ctx,
        vec![
            (
                first,
                crate::effects::zones::BattlefieldEntryOptions::preserve(false),
            ),
            (
                second,
                crate::effects::zones::BattlefieldEntryOptions::preserve(false),
            ),
        ],
    )
    .expect("CR614.13c nested milling skips library cards already entering");
    assert!(outcomes.iter().all(|result| matches!(
        result.assert_without_additions(),
        crate::effects::zones::BattlefieldEntryOutcome::Moved(_)
    )));
    assert!(!ctx.decision_maker.awaiting_choice());
    drop(ctx);
    assert_eq!(game.player(alice).unwrap().life, 20);
    for stable in entry_stables {
        let id = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.object(id).unwrap().zone, Zone::Battlefield);
    }
    for stable in spare_stables {
        let id = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(
            game.object(id).unwrap().zone,
            if exile_top {
                Zone::Exile
            } else {
                Zone::Graveyard
            }
        );
    }
    assert!(game.player(alice).unwrap().library.is_empty());
    assert_eq!(
        game.player(alice).unwrap().graveyard.len(),
        if exile_top { 0 } else { 2 }
    );
    assert_eq!(game.exile.len(), if exile_top { 2 } else { 0 });
    assert!(
        game.effect_store
            .replacement_effects
            .get_effect(entry)
            .is_none()
    );
    assert!(
        game.effect_store
            .replacement_effects
            .get_effect(mill)
            .is_none()
    );
    let events = game.take_pending_trigger_events();
    assert_eq!(
        events
            .iter()
            .filter(|event| event.kind() == crate::events::EventKind::EnterBattlefield)
            .count(),
        2
    );
}

#[test]
fn effect_backed_cost_keeps_temporary_replacement_scope() {
    let (mut game, source, alice) = setup();
    let temporary = ReplacementEffect::with_matcher(
        source,
        alice,
        crate::events::life::matchers::WouldLoseLifeMatcher::you(),
        ReplacementAction::Double,
    );
    let cost = crate::cost::TotalCost::from_costs(vec![
        crate::costs::Cost::try_effect(Effect::lose_life(2)).unwrap(),
    ]);
    let mut dm = crate::decision::SelectFirstDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut dm);
    ctx.replacement
        .additional_replacement_effects
        .push(temporary.clone());
    crate::special_actions::pay_total_cost_with_choice_in_context(
        &mut game,
        alice,
        source,
        &cost,
        crate::costs::PaymentReason::Effect,
        &mut ctx,
    )
    .expect("effect-backed payment honors the caller's temporary replacement");
    assert_eq!(game.player(alice).unwrap().life, 16);
    assert_eq!(ctx.replacement.additional_replacement_effects.len(), 1);
    assert_eq!(
        ctx.replacement.additional_replacement_effects[0].application_key(),
        temporary.application_key()
    );
}

#[test]
fn effect_backed_cost_keeps_parent_replacement_suppression_history() {
    let (mut game, source, alice) = setup();
    let shield =
        game.effect_store
            .replacement_effects
            .add_one_shot_effect(ReplacementEffect::with_matcher(
                source,
                alice,
                crate::events::life::matchers::WouldLoseLifeMatcher::you(),
                ReplacementAction::Double,
            ));
    let key = game
        .effect_store
        .replacement_effects
        .get_effect(shield)
        .unwrap()
        .application_key();
    let cost = crate::cost::TotalCost::from_costs(vec![
        crate::costs::Cost::try_effect(Effect::lose_life(2)).unwrap(),
    ]);
    let mut dm = crate::decision::SelectFirstDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut dm);
    ctx.replacement
        .suppressed_replacement_effects
        .insert(shield);
    ctx.replacement
        .suppressed_replacement_effect_keys
        .insert(key.clone());
    crate::special_actions::pay_total_cost_with_choice_in_context(
        &mut game,
        alice,
        source,
        &cost,
        crate::costs::PaymentReason::Effect,
        &mut ctx,
    )
    .expect("payment inherits parent replacement history");
    assert_eq!(game.player(alice).unwrap().life, 18);
    assert!(
        game.effect_store
            .replacement_effects
            .get_effect(shield)
            .is_some()
    );
    assert!(
        ctx.replacement
            .suppressed_replacement_effects
            .contains(&shield)
    );
    assert!(
        ctx.replacement
            .suppressed_replacement_effect_keys
            .contains(&key)
    );
}

struct ReservedEntryCostChoices {
    objects: ReservedEntryProgramChoices,
    payment_questions: usize,
}
impl crate::decision::DecisionMaker for ReservedEntryCostChoices {
    fn decide_boolean(
        &mut self,
        _: &GameState,
        _: &crate::decisions::context::BooleanContext,
    ) -> bool {
        self.payment_questions += 1;
        true
    }
    fn decide_objects(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        crate::decision::DecisionMaker::decide_objects(&mut self.objects, game, ctx)
    }
    fn awaiting_choice(&self) -> bool {
        self.objects.pending
    }
}

#[test]
fn entry_discard_cost_excludes_all_simultaneous_entrants() {
    let (mut game, first, alice) = setup();
    let second = prefix_card(&mut game, alice, Zone::Hand, "Second cost-boundary entrant");
    let spare_a = prefix_card(&mut game, alice, Zone::Hand, "First eligible cost card");
    let spare_b = prefix_card(&mut game, alice, Zone::Hand, "Unchosen eligible cost card");
    let entry_stables = [first, second].map(|id| game.object(id).unwrap().stable_id);
    let discarded_stable = game.object(spare_a).unwrap().stable_id;
    let cost = crate::cost::TotalCost::from_costs(vec![
        crate::costs::Cost::try_effect(Effect::discard(1)).unwrap(),
    ]);
    let shield =
        game.effect_store
            .replacement_effects
            .add_one_shot_effect(ReplacementEffect::with_matcher(
                first,
                alice,
                crate::events::zones::matchers::ThisWouldEnterBattlefieldMatcher,
                ReplacementAction::AsEntersProgram(
                    crate::resolution::ResolutionProgram::from_effects(vec![
                        Effect::unless_pays_total_cost(
                            vec![Effect::gain_life(5)],
                            crate::target::PlayerFilter::You,
                            cost,
                        ),
                    ]),
                ),
            ));
    game.take_pending_trigger_events();
    let mut dm = ReservedEntryCostChoices {
        objects: ReservedEntryProgramChoices {
            forbidden: vec![first, second],
            offered: Vec::new(),
            pause_second: false,
            pending: false,
        },
        payment_questions: 0,
    };
    let mut ctx = crate::effects::ExecutionContext::new(first, alice, &mut dm);
    let outcomes = crate::effects::zones::move_to_battlefield_batch_with_options(
        &mut game,
        &mut ctx,
        vec![
            (
                first,
                crate::effects::zones::BattlefieldEntryOptions::preserve(false),
            ),
            (
                second,
                crate::effects::zones::BattlefieldEntryOptions::preserve(false),
            ),
        ],
    )
    .expect("entry cost chooses an eligible nonentrant");
    assert!(!ctx.decision_maker.awaiting_choice());
    assert!(outcomes.iter().all(|result| matches!(
        result.assert_without_additions(),
        crate::effects::zones::BattlefieldEntryOutcome::Moved(_)
    )));
    drop(ctx);
    assert_eq!(dm.payment_questions, 1);
    assert_eq!(dm.objects.offered, vec![vec![spare_a, spare_b]]);
    for stable in entry_stables {
        let id = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.object(id).unwrap().zone, Zone::Battlefield);
    }
    let discarded = game.find_object_by_stable_id(discarded_stable).unwrap();
    assert_eq!(game.object(discarded).unwrap().zone, Zone::Graveyard);
    assert_eq!(
        game.player(alice).unwrap().graveyard.as_slice(),
        &[discarded]
    );
    assert_eq!(game.player(alice).unwrap().hand.as_slice(), &[spare_b]);
    assert_eq!(
        game.player(alice).unwrap().life,
        20,
        "accepted cost skips the consequence"
    );
    assert!(
        game.effect_store
            .replacement_effects
            .get_effect(shield)
            .is_none()
    );
    let events = game.take_pending_trigger_events();
    assert_eq!(
        events
            .iter()
            .filter(|event| event.kind() == crate::events::EventKind::EnterBattlefield)
            .count(),
        2
    );
}

#[test]
fn entry_discard_cost_preflight_does_not_count_reserved_cards() {
    let (game, first, alice) = setup();
    let mut game = game;
    let second = prefix_card(
        &mut game,
        alice,
        Zone::Hand,
        "Only other reserved cost card",
    );
    let cost = crate::cost::TotalCost::from_costs(vec![
        crate::costs::Cost::try_effect(Effect::discard(1)).unwrap(),
    ]);
    let mut dm = crate::decision::SelectFirstDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(first, alice, &mut dm);
    ctx.replacement
        .entry_reserved_objects
        .extend([first, second]);
    let hand_before = game.player(alice).unwrap().hand.as_slice().to_vec();
    let result = crate::special_actions::can_pay_total_cost_with_reason_in_context(
        &game,
        alice,
        first,
        &cost,
        crate::costs::PaymentReason::Effect,
        &mut ctx,
    );
    assert!(
        matches!(
            result,
            Err(crate::cost::CostPaymentError::InsufficientCardsInHand)
        ),
        "reserved cards cannot make a selected discard cost payable: {result:?}"
    );
    assert_eq!(
        game.player(alice).unwrap().hand.as_slice(),
        hand_before.as_slice()
    );
    assert_eq!(ctx.replacement.entry_reserved_objects.len(), 2);
}

#[test]
fn selected_discard_payment_keeps_temporary_replacement_scope() {
    check_discard_execution_scope(true, false);
}
#[test]
fn selected_discard_payment_keeps_parent_suppression_history() {
    check_discard_execution_scope(true, true);
}
#[test]
fn discard_effect_keeps_temporary_replacement_scope() {
    check_discard_execution_scope(false, false);
}
#[test]
fn discard_effect_keeps_parent_suppression_history() {
    check_discard_execution_scope(false, true);
}
fn check_discard_execution_scope(as_cost: bool, suppressed: bool) {
    use crate::effects::EffectExecutor;
    let (mut game, source, alice) = setup();
    let card = prefix_card(&mut game, alice, Zone::Hand, "Scoped discard card");
    let stable = game.object(card).unwrap().stable_id;
    let replacement = ReplacementEffect::with_matcher(
        source,
        alice,
        crate::events::cards::matchers::WouldDiscardMatcher::you(),
        ReplacementAction::ChangeDestination(Zone::Exile),
    );
    let shield = suppressed.then(|| {
        game.effect_store
            .replacement_effects
            .add_one_shot_effect(replacement.clone())
    });
    let key = shield.map(|id| {
        game.effect_store
            .replacement_effects
            .get_effect(id)
            .unwrap()
            .application_key()
    });
    let mut dm = crate::decision::SelectFirstDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut dm);
    if let Some(shield) = shield {
        ctx.replacement
            .suppressed_replacement_effects
            .insert(shield);
        ctx.replacement
            .suppressed_replacement_effect_keys
            .insert(key.clone().unwrap());
    } else {
        ctx.replacement
            .additional_replacement_effects
            .push(replacement);
    }
    // Keep the source unavailable so the effect and selected-cost callers
    // both discard the same physical card without introducing another choice.
    ctx.replacement.entry_reserved_objects.insert(source);
    if as_cost {
        let cost = crate::cost::TotalCost::from_costs(vec![
            crate::costs::Cost::try_effect(Effect::discard(1)).unwrap(),
        ]);
        crate::special_actions::pay_total_cost_with_choice_in_context(
            &mut game,
            alice,
            source,
            &cost,
            crate::costs::PaymentReason::Effect,
            &mut ctx,
        )
        .expect("redirected discard still pays a discard cost");
    } else {
        crate::effects::DiscardEffect::you(1)
            .execute(&mut game, &mut ctx)
            .unwrap();
    }
    assert!(!ctx.decision_maker.awaiting_choice());
    let actual = game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(
        game.object(actual).unwrap().zone,
        if suppressed {
            Zone::Graveyard
        } else {
            Zone::Exile
        }
    );
    assert_eq!(game.player(alice).unwrap().hand.as_slice(), &[source]);
    assert_eq!(game.player(alice).unwrap().life, 20);
    if let Some(shield) = shield {
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(shield)
                .is_some()
        );
        assert!(
            ctx.replacement
                .suppressed_replacement_effects
                .contains(&shield)
        );
        assert!(
            ctx.replacement
                .suppressed_replacement_effect_keys
                .contains(&key.unwrap())
        );
    } else {
        assert_eq!(ctx.replacement.additional_replacement_effects.len(), 1);
    }
}

#[test]
fn entry_program_source_departure_failure_restores_batch_checkpoint() {
    let (mut game, first, alice) = setup();
    let second = prefix_card(&mut game, alice, Zone::Hand, "Second discard-hand entrant");
    let spare_a = prefix_card(&mut game, alice, Zone::Hand, "Discard-hand spare A");
    let spare_b = prefix_card(&mut game, alice, Zone::Hand, "Discard-hand spare B");
    let hand_before = game.player(alice).unwrap().hand.as_slice().to_vec();
    let shield =
        game.effect_store
            .replacement_effects
            .add_one_shot_effect(ReplacementEffect::with_matcher(
                first,
                alice,
                crate::events::zones::matchers::ThisWouldEnterBattlefieldMatcher,
                ReplacementAction::AsEntersProgram(
                    crate::resolution::ResolutionProgram::from_effects(
                        vec![Effect::discard_hand()],
                    ),
                ),
            ));
    game.take_pending_trigger_events();
    let mut dm = crate::decision::SelectFirstDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(first, alice, &mut dm);
    let outcomes = crate::effects::zones::move_to_battlefield_batch_with_options(
        &mut game,
        &mut ctx,
        vec![
            (
                first,
                crate::effects::zones::BattlefieldEntryOptions::preserve(false),
            ),
            (
                second,
                crate::effects::zones::BattlefieldEntryOptions::preserve(false),
            ),
        ],
    );
    // The low-level entry adapter rejects a source removed by its program.
    // This asserts atomic failure only; it is not a rule that automatic
    // discard-hand effects must skip entering cards.
    assert!(matches!(
        outcomes,
        Err(crate::effects::ExecutionError::InvalidTarget)
    ));
    assert!(!ctx.decision_maker.awaiting_choice());
    assert_eq!(
        game.player(alice).unwrap().hand.as_slice(),
        hand_before.as_slice()
    );
    for id in [first, second, spare_a, spare_b] {
        assert_eq!(game.object(id).unwrap().zone, Zone::Hand);
    }
    assert!(game.battlefield.is_empty());
    assert!(game.player(alice).unwrap().graveyard.is_empty());
    assert_eq!(game.player(alice).unwrap().life, 20);
    assert!(
        game.effect_store
            .replacement_effects
            .get_effect(shield)
            .is_some()
    );
    assert!(game.take_pending_trigger_events().is_empty());
}

#[test]
fn discard_instead_payload_failure_reports_error_and_restores_prefix() {
    use crate::effects::EffectExecutor;
    let (mut game, source, alice) = setup();
    let card = prefix_card(&mut game, alice, Zone::Hand, "Failed discard payload card");
    let shield =
        game.effect_store
            .replacement_effects
            .add_one_shot_effect(ReplacementEffect::with_matcher(
                source,
                alice,
                crate::events::cards::matchers::WouldDiscardMatcher::you(),
                ReplacementAction::Instead(vec![Effect::gain_life(2), Effect::lose_life(Value::X)]),
            ));
    let hand_before = game.player(alice).unwrap().hand.as_slice().to_vec();
    game.take_pending_trigger_events();
    let mut dm = crate::decision::SelectFirstDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut dm);
    ctx.replacement.entry_reserved_objects.insert(source);
    let result = crate::effects::DiscardEffect::you(1).execute(&mut game, &mut ctx);
    assert!(
        matches!(
            result,
            Err(crate::effects::ExecutionError::UnresolvableValue(_))
        ),
        "the enclosing discard must receive its Instead payload failure"
    );
    assert!(!ctx.decision_maker.awaiting_choice());
    assert_eq!(game.player(alice).unwrap().life, 20);
    assert_eq!(
        game.player(alice).unwrap().hand.as_slice(),
        hand_before.as_slice()
    );
    assert_eq!(game.object(card).unwrap().zone, Zone::Hand);
    assert!(
        game.effect_store
            .replacement_effects
            .get_effect(shield)
            .is_some()
    );
    assert!(game.take_pending_trigger_events().is_empty());
}

struct DiscardPayloadBoolean {
    pause: bool,
    pending: bool,
    questions: usize,
}
impl crate::decision::DecisionMaker for DiscardPayloadBoolean {
    fn decide_boolean(
        &mut self,
        _: &GameState,
        _: &crate::decisions::context::BooleanContext,
    ) -> bool {
        self.questions += 1;
        self.pending = self.pause;
        !self.pause
    }
    fn awaiting_choice(&self) -> bool {
        self.pending
    }
}
#[test]
fn pending_discard_instead_payload_restores_prefix_and_replays_once() {
    use crate::effects::EffectExecutor;
    let (mut game, source, alice) = setup();
    let card = prefix_card(&mut game, alice, Zone::Hand, "Pending discard payload card");
    let shield =
        game.effect_store
            .replacement_effects
            .add_one_shot_effect(ReplacementEffect::with_matcher(
                source,
                alice,
                crate::events::cards::matchers::WouldDiscardMatcher::you(),
                ReplacementAction::Instead(vec![
                    Effect::gain_life(2),
                    Effect::may(vec![Effect::gain_life(4)]),
                ]),
            ));
    let hand_before = game.player(alice).unwrap().hand.as_slice().to_vec();
    game.take_pending_trigger_events();
    let mut paused = DiscardPayloadBoolean {
        pause: true,
        pending: false,
        questions: 0,
    };
    let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut paused);
    ctx.replacement.entry_reserved_objects.insert(source);
    crate::effects::DiscardEffect::you(1)
        .execute(&mut game, &mut ctx)
        .unwrap();
    assert!(ctx.decision_maker.awaiting_choice());
    drop(ctx);
    assert_eq!(paused.questions, 1);
    assert_eq!(game.player(alice).unwrap().life, 20);
    assert_eq!(
        game.player(alice).unwrap().hand.as_slice(),
        hand_before.as_slice()
    );
    assert!(
        game.effect_store
            .replacement_effects
            .get_effect(shield)
            .is_some()
    );
    assert!(game.take_pending_trigger_events().is_empty());
    let mut replay = DiscardPayloadBoolean {
        pause: false,
        pending: false,
        questions: 0,
    };
    let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut replay);
    ctx.replacement.entry_reserved_objects.insert(source);
    let replay_outcome = crate::effects::DiscardEffect::you(1)
        .execute(&mut game, &mut ctx)
        .unwrap();
    assert!(!ctx.decision_maker.awaiting_choice());
    drop(ctx);
    assert_eq!(replay.questions, 1);
    assert_eq!(game.player(alice).unwrap().life, 26);
    assert_eq!(
        game.player(alice).unwrap().hand.as_slice(),
        hand_before.as_slice()
    );
    assert_eq!(game.object(card).unwrap().zone, Zone::Hand);
    assert!(
        game.effect_store
            .replacement_effects
            .get_effect(shield)
            .is_none()
    );
    assert_eq!(
        replay_outcome
            .events
            .iter()
            .filter(|event| event.kind() == crate::events::EventKind::LifeGain)
            .count(),
        2
    );
    assert!(
        game.take_pending_trigger_events().is_empty(),
        "the enclosing resolution publishes returned observations once"
    );
}

#[test]
fn multi_card_discard_failure_restores_earlier_card() {
    check_discard_instruction_checkpoint(false, false);
}
#[test]
fn discard_hand_failure_restores_earlier_card() {
    check_discard_instruction_checkpoint(true, false);
}
#[test]
fn pending_multi_card_discard_restores_earlier_card_before_replay() {
    check_discard_instruction_checkpoint(false, true);
}
#[test]
fn pending_discard_hand_restores_earlier_card_before_replay() {
    check_discard_instruction_checkpoint(true, true);
}
fn check_discard_instruction_checkpoint(discard_hand: bool, pending: bool) {
    use crate::effects::EffectExecutor;
    let (mut game, source, alice) = setup();
    let source = game
        .move_object_by_effect(source, Zone::Battlefield)
        .unwrap();
    let first = prefix_card(
        &mut game,
        alice,
        Zone::Hand,
        "Earlier discard in instruction",
    );
    let second = prefix_card(
        &mut game,
        alice,
        Zone::Hand,
        "Later discard with replacement",
    );
    let first_stable = game.object(first).unwrap().stable_id;
    let last = if pending {
        Effect::may(vec![Effect::gain_life(4)])
    } else {
        Effect::lose_life(Value::X)
    };
    let shield =
        game.effect_store
            .replacement_effects
            .add_one_shot_effect(ReplacementEffect::with_matcher(
                second,
                alice,
                crate::events::cards::matchers::WouldDiscardMatcher::you()
                    .with_card_filter(crate::filter::ObjectFilter::source()),
                ReplacementAction::Instead(vec![Effect::gain_life(2), last]),
            ));
    game.take_pending_trigger_events();
    let mut dm = DiscardPayloadBoolean {
        pause: pending,
        pending: false,
        questions: 0,
    };
    let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut dm);
    let outcome = if discard_hand {
        crate::effects::DiscardHandEffect::you().execute(&mut game, &mut ctx)
    } else {
        crate::effects::DiscardEffect::you(2).execute(&mut game, &mut ctx)
    };
    if pending {
        assert!(outcome.is_ok());
        assert!(ctx.decision_maker.awaiting_choice());
    } else {
        assert!(matches!(
            outcome,
            Err(crate::effects::ExecutionError::UnresolvableValue(_))
        ));
    }
    drop(ctx);
    assert_eq!(game.player(alice).unwrap().life, 20);
    assert_eq!(
        game.player(alice).unwrap().hand.as_slice(),
        &[first, second]
    );
    assert_eq!(game.object(first).unwrap().zone, Zone::Hand);
    assert_eq!(game.object(second).unwrap().zone, Zone::Hand);
    assert!(game.player(alice).unwrap().graveyard.is_empty());
    assert!(
        game.effect_store
            .replacement_effects
            .get_effect(shield)
            .is_some()
    );
    assert!(game.take_pending_trigger_events().is_empty());
    if pending {
        assert_eq!(dm.questions, 1);
        let mut replay = DiscardPayloadBoolean {
            pause: false,
            pending: false,
            questions: 0,
        };
        let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut replay);
        let outcome = if discard_hand {
            crate::effects::DiscardHandEffect::you().execute(&mut game, &mut ctx)
        } else {
            crate::effects::DiscardEffect::you(2).execute(&mut game, &mut ctx)
        }
        .unwrap();
        assert!(!ctx.decision_maker.awaiting_choice());
        assert!(matches!(
            outcome.value,
            crate::effect::OutcomeValue::Count(1)
        ));
        drop(ctx);
        assert_eq!(replay.questions, 1);
        let discarded = game.find_object_by_stable_id(first_stable).unwrap();
        assert_eq!(game.object(discarded).unwrap().zone, Zone::Graveyard);
        assert_eq!(
            game.player(alice).unwrap().graveyard.as_slice(),
            &[discarded]
        );
        assert_eq!(game.player(alice).unwrap().hand.as_slice(), &[second]);
        assert_eq!(game.player(alice).unwrap().life, 26);
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(shield)
                .is_none()
        );
        let mut events = game.take_pending_trigger_events();
        events.extend(outcome.events);
        assert_eq!(
            events
                .iter()
                .filter(|event| event.kind() == crate::events::EventKind::LifeGain)
                .count(),
            2
        );
        assert_eq!(
            events
                .iter()
                .filter(|event| event.kind() == crate::events::EventKind::CardDiscarded)
                .count(),
            1
        );
    }
}

#[test]
fn compound_discard_counter_placement_keeps_temporary_replacements() {
    check_compound_discard_scope(true);
}
#[test]
fn compound_discard_follow_up_keeps_temporary_replacements() {
    check_compound_discard_scope(false);
}
fn check_compound_discard_scope(counters: bool) {
    use crate::effects::EffectExecutor;
    let (mut game, source, alice) = setup();
    let source = game
        .move_object_by_effect(source, Zone::Battlefield)
        .unwrap();
    let card = prefix_card(&mut game, alice, Zone::Hand, "Compound scoped discard");
    let stable = game.object(card).unwrap().stable_id;
    let shield =
        game.effect_store
            .replacement_effects
            .add_one_shot_effect(ReplacementEffect::with_matcher(
                source,
                alice,
                crate::events::cards::matchers::WouldDiscardMatcher::you(),
                ReplacementAction::ExileWithSourceLinkCountersThen {
                    counters: if counters {
                        vec![(CounterType::Time, 2)]
                    } else {
                        Vec::new()
                    },
                    effects: if counters {
                        Vec::new()
                    } else {
                        vec![Effect::gain_life(2)]
                    },
                },
            ));
    game.take_pending_trigger_events();
    let mut dm = crate::decision::SelectFirstDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut dm);
    let temporary = if counters {
        ReplacementEffect::with_matcher(
            source,
            alice,
            crate::events::counters::matchers::WouldPutCountersMatcher::new(
                crate::filter::ObjectFilter::default(),
                Some(CounterType::Time),
            ),
            ReplacementAction::Double,
        )
    } else {
        ReplacementEffect::with_matcher(
            source,
            alice,
            crate::events::life::matchers::WouldGainLifeMatcher::you(),
            ReplacementAction::Double,
        )
    };
    ctx.replacement
        .additional_replacement_effects
        .push(temporary);
    let outcome = crate::effects::DiscardEffect::you(1)
        .execute(&mut game, &mut ctx)
        .unwrap();
    assert!(!ctx.decision_maker.awaiting_choice());
    let actual = game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(game.object(actual).unwrap().zone, Zone::Exile);
    if counters {
        assert_eq!(game.counter_count(actual, CounterType::Time), 4);
    } else {
        assert_eq!(game.player(alice).unwrap().life, 24);
    }
    assert_eq!(game.get_exiled_with_source_links(source), vec![actual]);
    assert!(
        game.effect_store
            .replacement_effects
            .get_effect(shield)
            .is_none()
    );
    assert!(matches!(
        outcome.value,
        crate::effect::OutcomeValue::Count(1)
    ));
    assert_eq!(ctx.replacement.additional_replacement_effects.len(), 1);
}

#[test]
fn compound_discard_follow_up_failure_restores_move_counters_link_and_shield() {
    use crate::effects::EffectExecutor;
    let (mut game, source, alice) = setup();
    let source = game
        .move_object_by_effect(source, Zone::Battlefield)
        .unwrap();
    let card = prefix_card(&mut game, alice, Zone::Hand, "Failing compound discard");
    let shield =
        game.effect_store
            .replacement_effects
            .add_one_shot_effect(ReplacementEffect::with_matcher(
                source,
                alice,
                crate::events::cards::matchers::WouldDiscardMatcher::you(),
                ReplacementAction::ExileWithSourceLinkCountersThen {
                    counters: vec![(CounterType::Time, 2)],
                    effects: vec![Effect::gain_life(2), Effect::lose_life(Value::X)],
                },
            ));
    game.take_pending_trigger_events();
    let mut dm = crate::decision::SelectFirstDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut dm);
    let result = crate::effects::DiscardEffect::you(1).execute(&mut game, &mut ctx);
    assert!(matches!(
        result,
        Err(crate::effects::ExecutionError::UnresolvableValue(_))
    ));
    assert!(!ctx.decision_maker.awaiting_choice());
    assert_eq!(game.player(alice).unwrap().life, 20);
    assert_eq!(game.player(alice).unwrap().hand.as_slice(), &[card]);
    assert_eq!(game.object(card).unwrap().zone, Zone::Hand);
    assert_eq!(game.counter_count(card, CounterType::Time), 0);
    assert!(game.exile.is_empty());
    assert!(game.get_exiled_with_source_links(source).is_empty());
    assert!(
        game.effect_store
            .replacement_effects
            .get_effect(shield)
            .is_some()
    );
    assert!(game.take_pending_trigger_events().is_empty());
}

#[test]
fn nonempty_additional_replacement_executes_payload_and_original_event() {
    let (mut game, source, alice) = setup();
    let source = game
        .move_object_by_effect(source, Zone::Battlefield)
        .unwrap();
    let shield =
        game.effect_store
            .replacement_effects
            .add_one_shot_effect(ReplacementEffect::with_matcher(
                source,
                alice,
                crate::events::life::matchers::WouldGainLifeMatcher::you(),
                ReplacementAction::Additionally(vec![Effect::gain_life(3)]),
            ));
    game.take_pending_trigger_events();
    let mut ctx = crate::effects::ExecutionContext::new_default(source, alice);
    let outcome =
        crate::effects::execute_effect(&mut game, &Effect::gain_life(2), &mut ctx).unwrap();
    assert!(!ctx.decision_maker.awaiting_choice());
    assert_eq!(
        game.player(alice).unwrap().life,
        25,
        "Additionally must retain the original event and execute its nonempty payload"
    );
    assert!(
        game.effect_store
            .replacement_effects
            .get_effect(shield)
            .is_none()
    );
    let mut events = game.take_pending_trigger_events();
    events.extend(outcome.events);
    let mut amounts = events
        .iter()
        .filter_map(|event| {
            event
                .downcast::<crate::events::LifeGainEvent>()
                .map(|gain| gain.amount)
        })
        .collect::<Vec<_>>();
    amounts.sort();
    assert_eq!(amounts, vec![2, 3]);
}

#[test]
fn nonempty_additional_replacement_executes_a_different_event_family() {
    let (mut game, source, alice) = setup();
    let source = game
        .move_object_by_effect(source, Zone::Battlefield)
        .unwrap();
    let library_card = prefix_card(&mut game, alice, Zone::Library, "Additional draw card");
    let stable = game.object(library_card).unwrap().stable_id;
    let shield =
        game.effect_store
            .replacement_effects
            .add_one_shot_effect(ReplacementEffect::with_matcher(
                source,
                alice,
                crate::events::life::matchers::WouldGainLifeMatcher::you(),
                ReplacementAction::Additionally(vec![Effect::draw(1)]),
            ));
    game.take_pending_trigger_events();
    let mut ctx = crate::effects::ExecutionContext::new_default(source, alice);
    let outcome =
        crate::effects::execute_effect(&mut game, &Effect::gain_life(2), &mut ctx).unwrap();
    assert!(!ctx.decision_maker.awaiting_choice());
    assert_eq!(game.player(alice).unwrap().life, 22);
    let drawn = game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(game.object(drawn).unwrap().zone, Zone::Hand);
    assert_eq!(game.player(alice).unwrap().hand.as_slice(), &[drawn]);
    assert!(game.player(alice).unwrap().library.is_empty());
    assert!(
        game.effect_store
            .replacement_effects
            .get_effect(shield)
            .is_none()
    );
    let mut events = game.take_pending_trigger_events();
    events.extend(outcome.events);
    assert_eq!(
        events
            .iter()
            .filter(|event| event.kind() == crate::events::EventKind::LifeGain)
            .count(),
        1
    );
}

#[test]
fn additional_payload_failure_restores_original_event_and_payload_prefix() {
    let (mut game, source, alice) = setup();
    let source = game
        .move_object_by_effect(source, Zone::Battlefield)
        .unwrap();
    let shield =
        game.effect_store
            .replacement_effects
            .add_one_shot_effect(ReplacementEffect::with_matcher(
                source,
                alice,
                crate::events::life::matchers::WouldGainLifeMatcher::you(),
                ReplacementAction::Additionally(vec![
                    Effect::gain_life(3),
                    Effect::lose_life(Value::X),
                ]),
            ));
    game.take_pending_trigger_events();
    let mut ctx = crate::effects::ExecutionContext::new_default(source, alice);
    let result = crate::effects::execute_effect(&mut game, &Effect::gain_life(2), &mut ctx);
    assert!(
        matches!(
            result,
            Err(crate::effects::ExecutionError::UnresolvableValue(_))
        ),
        "an added payload failure must propagate to the owning instruction"
    );
    assert!(!ctx.decision_maker.awaiting_choice());
    assert_eq!(game.player(alice).unwrap().life, 20);
    assert!(
        game.effect_store
            .replacement_effects
            .get_effect(shield)
            .is_some()
    );
    assert!(game.take_pending_trigger_events().is_empty());
}

#[test]
fn pending_additional_payload_restores_original_event_and_replays_once() {
    let (mut game, source, alice) = setup();
    let source = game
        .move_object_by_effect(source, Zone::Battlefield)
        .unwrap();
    let shield =
        game.effect_store
            .replacement_effects
            .add_one_shot_effect(ReplacementEffect::with_matcher(
                source,
                alice,
                crate::events::life::matchers::WouldGainLifeMatcher::you(),
                ReplacementAction::Additionally(vec![
                    Effect::gain_life(3),
                    Effect::may(vec![Effect::gain_life(4)]),
                ]),
            ));
    game.take_pending_trigger_events();
    let mut paused = DiscardPayloadBoolean {
        pause: true,
        pending: false,
        questions: 0,
    };
    let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut paused);
    let outcome =
        crate::effects::execute_effect(&mut game, &Effect::gain_life(2), &mut ctx).unwrap();
    assert!(ctx.decision_maker.awaiting_choice());
    assert!(outcome.events.is_empty());
    drop(ctx);
    assert_eq!(paused.questions, 1);
    assert_eq!(game.player(alice).unwrap().life, 20);
    assert!(
        game.effect_store
            .replacement_effects
            .get_effect(shield)
            .is_some()
    );
    assert!(game.take_pending_trigger_events().is_empty());
    let mut replay = DiscardPayloadBoolean {
        pause: false,
        pending: false,
        questions: 0,
    };
    let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut replay);
    let outcome =
        crate::effects::execute_effect(&mut game, &Effect::gain_life(2), &mut ctx).unwrap();
    assert!(!ctx.decision_maker.awaiting_choice());
    drop(ctx);
    assert_eq!(replay.questions, 1);
    assert_eq!(game.player(alice).unwrap().life, 29);
    assert!(
        game.effect_store
            .replacement_effects
            .get_effect(shield)
            .is_none()
    );
    let mut events = game.take_pending_trigger_events();
    events.extend(outcome.events);
    let mut amounts = events
        .iter()
        .filter_map(|event| {
            event
                .downcast::<crate::events::LifeGainEvent>()
                .map(|gain| gain.amount)
        })
        .collect::<Vec<_>>();
    amounts.sort();
    assert_eq!(amounts, vec![2, 3, 4]);
}

#[test]
fn draw_adapter_replaced_outcome_requires_executed_payload() {
    check_draw_replacement_execution_contract(true);
}

#[test]
fn draw_effect_executes_instead_payload_without_drawing_original_card() {
    check_draw_replacement_execution_contract(false);
}

fn check_draw_replacement_execution_contract(legacy_adapter: bool) {
    let (mut game, source, alice) = setup();
    let source = game
        .move_object_by_effect(source, Zone::Battlefield)
        .unwrap();
    let library_card = prefix_card(
        &mut game,
        alice,
        Zone::Library,
        "Retained replaced draw card",
    );
    let shield =
        game.effect_store
            .replacement_effects
            .add_one_shot_effect(ReplacementEffect::with_matcher(
                source,
                alice,
                crate::events::cards::matchers::WouldDrawCardMatcher::you(),
                ReplacementAction::Instead(vec![Effect::gain_life(3)]),
            ));
    game.take_pending_trigger_events();
    let mut events = Vec::new();
    if legacy_adapter {
        let mut dm = crate::decision::SelectFirstDecisionMaker;
        let outcome = process_draw(&mut game, alice, 1, false, &mut dm).unwrap();
        let ResolvedDrawOutcome::Replaced {
            context,
            source: actual_source,
            controller,
            payload,
        } = outcome
        else {
            panic!("draw adapter must return an executed replacement receipt");
        };
        assert_eq!(actual_source, source);
        assert_eq!(controller, alice);
        assert!(context.applied_effects.contains(&shield));
        let draw = crate::events::downcast_event::<crate::events::DrawEvent>(context.event.inner())
            .unwrap();
        assert_eq!(draw.player, alice);
        assert_eq!(draw.count, 1);
        events.extend(payload.events);
        assert!(!dm.awaiting_choice());
    } else {
        let mut ctx = crate::effects::ExecutionContext::new_default(source, alice);
        let outcome =
            crate::effects::execute_effect(&mut game, &Effect::draw(1), &mut ctx).unwrap();
        assert!(!ctx.decision_maker.awaiting_choice());
        assert!(matches!(
            outcome.value,
            crate::effect::OutcomeValue::Count(0)
        ));
        events.extend(outcome.events);
    }
    assert_eq!(
        game.player(alice).unwrap().life,
        23,
        "Replaced promises the payload has executed; returning a marker alone is insufficient"
    );
    assert_eq!(game.object(library_card).unwrap().zone, Zone::Library);
    assert_eq!(
        game.player(alice).unwrap().library.as_slice(),
        &[library_card]
    );
    assert!(game.player(alice).unwrap().hand.is_empty());
    assert!(
        game.effect_store
            .replacement_effects
            .get_effect(shield)
            .is_none()
    );
    events.extend(game.take_pending_trigger_events());
    let amounts = events
        .iter()
        .filter_map(|event| {
            event
                .downcast::<crate::events::LifeGainEvent>()
                .map(|gain| gain.amount)
        })
        .collect::<Vec<_>>();
    assert_eq!(amounts, vec![3]);
}

#[test]
fn draw_adapter_payload_failure_propagates_and_restores_checkpoint() {
    let (mut game, source, alice) = setup();
    let source = game
        .move_object_by_effect(source, Zone::Battlefield)
        .unwrap();
    let library_card = prefix_card(&mut game, alice, Zone::Library, "Failed draw payload card");
    let shield =
        game.effect_store
            .replacement_effects
            .add_one_shot_effect(ReplacementEffect::with_matcher(
                source,
                alice,
                crate::events::cards::matchers::WouldDrawCardMatcher::you(),
                ReplacementAction::Instead(vec![Effect::gain_life(2), Effect::lose_life(Value::X)]),
            ));
    game.take_pending_trigger_events();
    let mut dm = crate::decision::SelectFirstDecisionMaker;
    let result = process_draw(&mut game, alice, 1, false, &mut dm);
    assert!(matches!(
        result,
        Err(crate::effects::ExecutionError::UnresolvableValue(_))
    ));
    assert!(!dm.awaiting_choice());
    assert_eq!(game.player(alice).unwrap().life, 20);
    assert_eq!(
        game.player(alice).unwrap().library.as_slice(),
        &[library_card]
    );
    assert!(game.player(alice).unwrap().hand.is_empty());
    assert!(
        game.effect_store
            .replacement_effects
            .get_effect(shield)
            .is_some()
    );
    assert!(game.take_pending_trigger_events().is_empty());
}

#[test]
fn draw_adapter_pending_payload_restores_checkpoint_and_replays_once() {
    let (mut game, source, alice) = setup();
    let source = game
        .move_object_by_effect(source, Zone::Battlefield)
        .unwrap();
    let library_card = prefix_card(&mut game, alice, Zone::Library, "Pending draw payload card");
    let shield =
        game.effect_store
            .replacement_effects
            .add_one_shot_effect(ReplacementEffect::with_matcher(
                source,
                alice,
                crate::events::cards::matchers::WouldDrawCardMatcher::you(),
                ReplacementAction::Instead(vec![
                    Effect::gain_life(2),
                    Effect::may(vec![Effect::gain_life(4)]),
                ]),
            ));
    game.take_pending_trigger_events();
    let mut paused = DiscardPayloadBoolean {
        pause: true,
        pending: false,
        questions: 0,
    };
    let outcome = process_draw(&mut game, alice, 1, false, &mut paused).unwrap();
    assert!(matches!(outcome, ResolvedDrawOutcome::Pending));
    assert!(paused.awaiting_choice());
    assert_eq!(paused.questions, 1);
    assert_eq!(game.player(alice).unwrap().life, 20);
    assert_eq!(
        game.player(alice).unwrap().library.as_slice(),
        &[library_card]
    );
    assert!(game.player(alice).unwrap().hand.is_empty());
    assert!(
        game.effect_store
            .replacement_effects
            .get_effect(shield)
            .is_some()
    );
    assert!(game.take_pending_trigger_events().is_empty());
    let mut replay = DiscardPayloadBoolean {
        pause: false,
        pending: false,
        questions: 0,
    };
    let outcome = process_draw(&mut game, alice, 1, false, &mut replay).unwrap();
    let ResolvedDrawOutcome::Replaced {
        context, payload, ..
    } = outcome
    else {
        panic!("answered draw replacement must complete its receipt");
    };
    assert!(!replay.awaiting_choice());
    assert_eq!(replay.questions, 1);
    assert!(context.applied_effects.contains(&shield));
    assert_eq!(game.player(alice).unwrap().life, 26);
    assert_eq!(
        game.player(alice).unwrap().library.as_slice(),
        &[library_card]
    );
    assert!(game.player(alice).unwrap().hand.is_empty());
    assert!(
        game.effect_store
            .replacement_effects
            .get_effect(shield)
            .is_none()
    );
    let mut events = payload.events;
    events.extend(game.take_pending_trigger_events());
    let mut amounts = events
        .iter()
        .filter_map(|event| {
            event
                .downcast::<crate::events::LifeGainEvent>()
                .map(|gain| gain.amount)
        })
        .collect::<Vec<_>>();
    amounts.sort();
    assert_eq!(amounts, vec![2, 4]);
}

#[test]
fn additional_object_counters_success() {
    check_additional_counter_contract(false, 0);
}

#[test]
fn additional_object_counters_failure() {
    check_additional_counter_contract(false, 1);
}

#[test]
fn additional_object_counters_pending_replay() {
    check_additional_counter_contract(false, 2);
}

#[test]
fn additional_object_counters_original_prevented() {
    check_additional_counter_contract(false, 3);
}

#[test]
fn additional_player_counters_success() {
    check_additional_counter_contract(true, 0);
}

#[test]
fn additional_player_counters_failure() {
    check_additional_counter_contract(true, 1);
}

#[test]
fn additional_player_counters_pending_replay() {
    check_additional_counter_contract(true, 2);
}

#[test]
fn additional_player_counters_original_prevented() {
    check_additional_counter_contract(true, 3);
}

fn run_additional_counter_proposal(
    game: &mut GameState,
    ctx: &mut crate::effects::ExecutionContext,
    player_target: bool,
) -> Result<crate::effect::EffectOutcome, crate::effects::ExecutionError> {
    if player_target {
        let event = crate::events::Event::put_player_counters(
            ctx.controller,
            crate::object::CounterType::Energy,
            2,
            ctx.cause.clone(),
        );
        crate::effects::counters::execute_player_counter_placement(game, ctx, event)
    } else {
        let event = crate::events::Event::put_counters(
            ctx.source,
            crate::object::CounterType::PlusOnePlusOne,
            2,
            ctx.cause.clone(),
        );
        crate::effects::counters::execute_object_counter_placement(game, ctx, event)
    }
}

fn check_additional_counter_contract(player_target: bool, mode: u8) {
    let (mut game, source, alice) = setup();
    let source = game
        .move_object_by_effect(source, Zone::Battlefield)
        .unwrap();
    let counter_type = if player_target {
        crate::object::CounterType::Energy
    } else {
        crate::object::CounterType::PlusOnePlusOne
    };
    let mut replacement = if player_target {
        crate::static_abilities::StaticAbility::double_player_counters_replacement(
            crate::target::PlayerFilter::Specific(alice),
            Some(counter_type),
            "Synthetic added counter program matcher".into(),
        )
        .generate_replacement_effect(source, alice)
        .unwrap()
    } else {
        ReplacementEffect::with_matcher(
            source,
            alice,
            crate::events::counters::matchers::WouldPutCountersMatcher::new(
                crate::filter::ObjectFilter::source(),
                Some(counter_type),
            ),
            ReplacementAction::Double,
        )
    };
    let prevention = if mode == 3 {
        let mut prevention = replacement.clone();
        prevention.replacement = ReplacementAction::Prevent;
        Some(prevention)
    } else {
        None
    };
    let mut effects = vec![Effect::gain_life(3)];
    if mode == 1 {
        effects.push(Effect::lose_life(Value::X));
    }
    if mode == 2 {
        effects.push(Effect::may(vec![Effect::gain_life(4)]));
    }
    replacement.replacement = ReplacementAction::Additionally(effects);
    // Explicitly order the synthetic self-replacement ahead of the separate
    // prevention, so this tests retained additions rather than registration order.
    replacement.priority_override = Some(crate::events::ReplacementPriority::SelfReplacement);
    let shield = game
        .effect_store
        .replacement_effects
        .add_one_shot_effect(replacement);
    let prevention_shield = prevention.map(|prevention| {
        game.effect_store
            .replacement_effects
            .add_one_shot_effect(prevention)
    });
    game.take_pending_trigger_events();
    let mut dm = DiscardPayloadBoolean {
        pause: mode == 2,
        pending: false,
        questions: 0,
    };
    let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut dm);
    let result = run_additional_counter_proposal(&mut game, &mut ctx, player_target);
    if mode == 1 {
        assert!(matches!(
            result,
            Err(crate::effects::ExecutionError::UnresolvableValue(_))
        ));
        assert!(!ctx.decision_maker.awaiting_choice());
        drop(ctx);
        assert_eq!(game.player(alice).unwrap().life, 20);
        let count = if player_target {
            game.player(alice).unwrap().counter_count(counter_type)
        } else {
            game.counter_count(source, counter_type)
        };
        assert_eq!(count, 0);
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(shield)
                .is_some()
        );
        assert!(game.take_pending_trigger_events().is_empty());
        return;
    }
    let mut outcome = result.unwrap();
    if mode == 2 {
        assert!(ctx.decision_maker.awaiting_choice());
        assert!(outcome.events.is_empty());
        drop(ctx);
        assert_eq!(dm.questions, 1);
        assert_eq!(game.player(alice).unwrap().life, 20);
        let count = if player_target {
            game.player(alice).unwrap().counter_count(counter_type)
        } else {
            game.counter_count(source, counter_type)
        };
        assert_eq!(count, 0);
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(shield)
                .is_some()
        );
        assert!(game.take_pending_trigger_events().is_empty());
        let mut replay = DiscardPayloadBoolean {
            pause: false,
            pending: false,
            questions: 0,
        };
        let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut replay);
        outcome = run_additional_counter_proposal(&mut game, &mut ctx, player_target).unwrap();
        assert!(!ctx.decision_maker.awaiting_choice());
        drop(ctx);
        assert_eq!(replay.questions, 1);
    } else {
        assert!(!ctx.decision_maker.awaiting_choice());
        drop(ctx);
    }
    let expected = if mode == 3 { 0 } else { 2 };
    let count = if player_target {
        game.player(alice).unwrap().counter_count(counter_type)
    } else {
        game.counter_count(source, counter_type)
    };
    assert_eq!(count, expected);
    assert!(
        matches!(outcome.value, crate::effect::OutcomeValue::Count(value) if value == expected as i64)
    );
    assert_eq!(
        game.player(alice).unwrap().life,
        if mode == 2 { 27 } else { 23 }
    );
    assert!(
        game.effect_store
            .replacement_effects
            .get_effect(shield)
            .is_none()
    );
    if let Some(prevention_shield) = prevention_shield {
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(prevention_shield)
                .is_none()
        );
    }
    let mut events = game.take_pending_trigger_events();
    events.extend(outcome.events);
    assert_eq!(
        events
            .iter()
            .filter(|event| event.kind() == crate::events::EventKind::MarkersChanged)
            .count(),
        if mode == 3 { 0 } else { 1 }
    );
    let mut amounts = events
        .iter()
        .filter_map(|event| {
            event
                .downcast::<crate::events::LifeGainEvent>()
                .map(|gain| gain.amount)
        })
        .collect::<Vec<_>>();
    amounts.sort();
    assert_eq!(amounts, if mode == 2 { vec![3, 4] } else { vec![3] });
}

#[test]
fn draw_effect_uses_parent_temporary_replacement_scope() {
    let (mut game, source, alice) = setup();
    let source = game
        .move_object_by_effect(source, Zone::Battlefield)
        .unwrap();
    let library_card = prefix_card(&mut game, alice, Zone::Library, "Scoped draw card");
    game.take_pending_trigger_events();
    let mut ctx = crate::effects::ExecutionContext::new_default(source, alice);
    ctx.replacement
        .additional_replacement_effects
        .push(ReplacementEffect::with_matcher(
            source,
            alice,
            crate::events::cards::matchers::WouldDrawCardMatcher::you(),
            ReplacementAction::Instead(vec![Effect::gain_life(3)]),
        ));
    let outcome = crate::effects::execute_effect(&mut game, &Effect::draw(1), &mut ctx).unwrap();
    assert!(!ctx.decision_maker.awaiting_choice());
    assert_eq!(ctx.replacement.additional_replacement_effects.len(), 1);
    assert_eq!(game.player(alice).unwrap().life, 23);
    assert_eq!(
        game.player(alice).unwrap().library.as_slice(),
        &[library_card]
    );
    assert!(game.player(alice).unwrap().hand.is_empty());
    assert!(matches!(
        outcome.value,
        crate::effect::OutcomeValue::Count(0)
    ));
    let mut events = game.take_pending_trigger_events();
    events.extend(outcome.events);
    assert_eq!(
        events
            .iter()
            .filter(|event| event.kind() == crate::events::EventKind::LifeGain)
            .count(),
        1
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| event.kind() == crate::events::EventKind::CardsDrawn)
            .count(),
        0
    );
}

#[test]
fn draw_instead_payload_keeps_preceding_replacement_history() {
    let (mut game, source, alice) = setup();
    let source = game
        .move_object_by_effect(source, Zone::Battlefield)
        .unwrap();
    prefix_card(&mut game, alice, Zone::Library, "First nested draw card");
    prefix_card(&mut game, alice, Zone::Library, "Second nested draw card");
    let mut double = ReplacementEffect::with_matcher(
        source,
        alice,
        crate::events::cards::matchers::WouldDrawCardMatcher::you(),
        ReplacementAction::Double,
    );
    double.priority_override = Some(crate::events::ReplacementPriority::SelfReplacement);
    let double = game
        .effect_store
        .replacement_effects
        .add_resolution_effect(double);
    let shield =
        game.effect_store
            .replacement_effects
            .add_one_shot_effect(ReplacementEffect::with_matcher(
                source,
                alice,
                crate::events::cards::matchers::WouldDrawCardMatcher::you(),
                ReplacementAction::Instead(vec![Effect::draw(1)]),
            ));
    game.take_pending_trigger_events();
    let mut ctx = crate::effects::ExecutionContext::new_default(source, alice);
    let outcome = crate::effects::execute_effect(&mut game, &Effect::draw(1), &mut ctx).unwrap();
    assert!(!ctx.decision_maker.awaiting_choice());
    assert_eq!(
        game.player(alice).unwrap().hand.len(),
        1,
        "a replacement already applied before Instead cannot reapply to that modified event"
    );
    assert_eq!(game.player(alice).unwrap().library.len(), 1);
    assert!(matches!(
        outcome.value,
        crate::effect::OutcomeValue::Count(1)
    ));
    assert!(
        game.effect_store
            .replacement_effects
            .get_effect(double)
            .is_some()
    );
    assert!(
        game.effect_store
            .replacement_effects
            .get_effect(shield)
            .is_none()
    );
    assert!(ctx.replacement.suppressed_replacement_effects.is_empty());
    let mut events = game.take_pending_trigger_events();
    events.extend(outcome.events);
    let drawn = events
        .iter()
        .filter_map(|event| event.downcast::<crate::events::CardsDrawnEvent>())
        .map(|event| event.amount())
        .sum::<u32>();
    assert_eq!(drawn, 1);
}

#[test]
fn draw_effect_payload_failure_restores_prefix_and_shield() {
    let (mut game, source, alice) = setup();
    let source = game
        .move_object_by_effect(source, Zone::Battlefield)
        .unwrap();
    let library_card = prefix_card(&mut game, alice, Zone::Library, "Failed actual draw card");
    let shield =
        game.effect_store
            .replacement_effects
            .add_one_shot_effect(ReplacementEffect::with_matcher(
                source,
                alice,
                crate::events::cards::matchers::WouldDrawCardMatcher::you(),
                ReplacementAction::Instead(vec![Effect::gain_life(2), Effect::lose_life(Value::X)]),
            ));
    game.take_pending_trigger_events();
    let mut ctx = crate::effects::ExecutionContext::new_default(source, alice);
    let result = crate::effects::execute_effect(&mut game, &Effect::draw(1), &mut ctx);
    assert!(matches!(
        result,
        Err(crate::effects::ExecutionError::UnresolvableValue(_))
    ));
    assert!(!ctx.decision_maker.awaiting_choice());
    assert_eq!(game.player(alice).unwrap().life, 20);
    assert_eq!(
        game.player(alice).unwrap().library.as_slice(),
        &[library_card]
    );
    assert!(game.player(alice).unwrap().hand.is_empty());
    assert!(
        game.effect_store
            .replacement_effects
            .get_effect(shield)
            .is_some()
    );
    assert!(game.take_pending_trigger_events().is_empty());
}

#[test]
fn draw_effect_pending_payload_stops_tail_and_restores_before_replay() {
    let (mut game, source, alice) = setup();
    let source = game
        .move_object_by_effect(source, Zone::Battlefield)
        .unwrap();
    let library_card = prefix_card(&mut game, alice, Zone::Library, "Pending actual draw card");
    let shield =
        game.effect_store
            .replacement_effects
            .add_one_shot_effect(ReplacementEffect::with_matcher(
                source,
                alice,
                crate::events::cards::matchers::WouldDrawCardMatcher::you(),
                ReplacementAction::Instead(vec![
                    Effect::gain_life(2),
                    Effect::may(vec![Effect::gain_life(4)]),
                    Effect::gain_life(8),
                ]),
            ));
    game.take_pending_trigger_events();
    let mut paused = DiscardPayloadBoolean {
        pause: true,
        pending: false,
        questions: 0,
    };
    let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut paused);
    let outcome = crate::effects::execute_effect(&mut game, &Effect::draw(1), &mut ctx).unwrap();
    assert!(ctx.decision_maker.awaiting_choice());
    assert!(outcome.events.is_empty());
    drop(ctx);
    assert_eq!(paused.questions, 1);
    assert_eq!(game.player(alice).unwrap().life, 20);
    assert_eq!(
        game.player(alice).unwrap().library.as_slice(),
        &[library_card]
    );
    assert!(game.player(alice).unwrap().hand.is_empty());
    assert!(
        game.effect_store
            .replacement_effects
            .get_effect(shield)
            .is_some()
    );
    assert!(game.take_pending_trigger_events().is_empty());
    let mut replay = DiscardPayloadBoolean {
        pause: false,
        pending: false,
        questions: 0,
    };
    let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut replay);
    let outcome = crate::effects::execute_effect(&mut game, &Effect::draw(1), &mut ctx).unwrap();
    assert!(!ctx.decision_maker.awaiting_choice());
    drop(ctx);
    assert_eq!(replay.questions, 1);
    assert_eq!(game.player(alice).unwrap().life, 34);
    assert_eq!(
        game.player(alice).unwrap().library.as_slice(),
        &[library_card]
    );
    assert!(game.player(alice).unwrap().hand.is_empty());
    assert!(
        game.effect_store
            .replacement_effects
            .get_effect(shield)
            .is_none()
    );
    let mut events = game.take_pending_trigger_events();
    events.extend(outcome.events);
    let mut amounts = events
        .iter()
        .filter_map(|event| {
            event
                .downcast::<crate::events::LifeGainEvent>()
                .map(|gain| gain.amount)
        })
        .collect::<Vec<_>>();
    amounts.sort();
    assert_eq!(amounts, vec![2, 4, 8]);
}

fn additional_untap_fixture(
    effects: Vec<Effect>,
) -> (GameState, ObjectId, ObjectId, PlayerId, ReplacementEffectId) {
    let (mut game, permanent, alice) = setup();
    let permanent = game
        .move_object_by_effect(permanent, Zone::Battlefield)
        .unwrap();
    game.tap(permanent);
    let source = game.create_object_from_card(
        &crate::card::CardBuilder::new(CardId::new(), "Additional untap source")
            .card_types(vec![crate::types::CardType::Enchantment])
            .build(),
        alice,
        Zone::Battlefield,
    );
    let mut effect = ReplacementEffect::with_matcher(
        source,
        alice,
        crate::events::permanents::matchers::WouldBecomeUntappedMatcher::new(
            crate::target::ObjectFilter::specific(permanent),
        ),
        ReplacementAction::Additionally(effects),
    );
    effect.priority_override = Some(crate::events::ReplacementPriority::SelfReplacement);
    let shield = game
        .effect_store
        .replacement_effects
        .add_one_shot_effect(effect);
    game.take_pending_trigger_events();
    (game, source, permanent, alice, shield)
}

#[test]
fn additional_untap_preserves_primary_count_and_actual_observations() {
    let (mut game, source, permanent, alice, shield) =
        additional_untap_fixture(vec![Effect::gain_life(3)]);
    let mut ctx = crate::effects::ExecutionContext::new_default(source, alice);
    let outcome = process_untap_with_execution_context(&mut game, permanent, &mut ctx).unwrap();
    assert_eq!(outcome.value, crate::effect::OutcomeValue::Count(1));
    assert!(!game.is_tapped(permanent));
    assert_eq!(game.player(alice).unwrap().life, 23);
    assert!(
        game.effect_store
            .replacement_effects
            .get_effect(shield)
            .is_none()
    );
    let mut events = game.take_pending_trigger_events();
    events.extend(outcome.events);
    assert_eq!(
        events
            .iter()
            .filter_map(|event| event.downcast::<crate::events::PermanentUntappedEvent>())
            .count(),
        1
    );
    assert_eq!(
        events
            .iter()
            .filter_map(|event| event.downcast::<crate::events::LifeGainEvent>())
            .map(|event| event.amount)
            .collect::<Vec<_>>(),
        vec![3]
    );
}

#[test]
fn additional_untap_binds_the_captured_permanent_for_added_object_actions() {
    let effect = Effect::put_counters(
        crate::object::CounterType::PlusOnePlusOne,
        2,
        crate::target::ChooseSpec::target(crate::target::ChooseSpec::creature()),
    );
    let (mut game, source, permanent, alice, shield) = additional_untap_fixture(vec![effect]);
    let mut ctx = crate::effects::ExecutionContext::new_default(source, alice);
    let outcome = process_untap_with_execution_context(&mut game, permanent, &mut ctx).unwrap();
    assert_eq!(outcome.value, crate::effect::OutcomeValue::Count(1));
    assert!(!game.is_tapped(permanent));
    assert_eq!(
        game.object(permanent)
            .unwrap()
            .counters
            .get(&crate::object::CounterType::PlusOnePlusOne)
            .copied()
            .unwrap_or(0),
        2
    );
    assert!(game.object(source).unwrap().counters.is_empty());
    assert!(
        game.effect_store
            .replacement_effects
            .get_effect(shield)
            .is_none()
    );
    assert_eq!(
        outcome
            .events
            .iter()
            .filter_map(|event| event.downcast::<crate::events::PermanentUntappedEvent>())
            .count(),
        1
    );
}

#[test]
fn additional_untap_failure_restores_original_untap_prefix_and_shield() {
    let (mut game, source, permanent, alice, shield) =
        additional_untap_fixture(vec![Effect::gain_life(3), Effect::lose_life(Value::X)]);
    let mut ctx = crate::effects::ExecutionContext::new_default(source, alice);
    let result = process_untap_with_execution_context(&mut game, permanent, &mut ctx);
    assert!(matches!(
        result,
        Err(crate::effects::ExecutionError::UnresolvableValue(_))
    ));
    assert!(game.is_tapped(permanent));
    assert_eq!(game.player(alice).unwrap().life, 20);
    assert!(
        game.effect_store
            .replacement_effects
            .get_effect(shield)
            .is_some()
    );
    assert!(game.take_pending_trigger_events().is_empty());
}

#[test]
fn additional_untap_pending_restores_original_then_replays_once() {
    let (mut game, source, permanent, alice, shield) = additional_untap_fixture(vec![
        Effect::gain_life(3),
        Effect::may(vec![Effect::gain_life(4)]),
    ]);
    let mut paused = DiscardPayloadBoolean {
        pause: true,
        pending: false,
        questions: 0,
    };
    let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut paused);
    let outcome = process_untap_with_execution_context(&mut game, permanent, &mut ctx).unwrap();
    assert!(ctx.decision_maker.awaiting_choice());
    assert!(outcome.events.is_empty());
    drop(ctx);
    assert_eq!(paused.questions, 1);
    assert!(game.is_tapped(permanent));
    assert_eq!(game.player(alice).unwrap().life, 20);
    assert!(
        game.effect_store
            .replacement_effects
            .get_effect(shield)
            .is_some()
    );
    assert!(game.take_pending_trigger_events().is_empty());
    let mut replay = DiscardPayloadBoolean {
        pause: false,
        pending: false,
        questions: 0,
    };
    let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut replay);
    let outcome = process_untap_with_execution_context(&mut game, permanent, &mut ctx).unwrap();
    assert!(!ctx.decision_maker.awaiting_choice());
    drop(ctx);
    assert_eq!(replay.questions, 1);
    assert_eq!(outcome.value, crate::effect::OutcomeValue::Count(1));
    assert!(!game.is_tapped(permanent));
    assert_eq!(game.player(alice).unwrap().life, 27);
    assert!(
        game.effect_store
            .replacement_effects
            .get_effect(shield)
            .is_none()
    );
    let mut events = game.take_pending_trigger_events();
    events.extend(outcome.events);
    assert_eq!(
        events
            .iter()
            .filter_map(|event| event.downcast::<crate::events::PermanentUntappedEvent>())
            .count(),
        1
    );
    let mut gains = events
        .iter()
        .filter_map(|event| event.downcast::<crate::events::LifeGainEvent>())
        .map(|event| event.amount)
        .collect::<Vec<_>>();
    gains.sort();
    assert_eq!(gains, vec![3, 4]);
}

#[test]
fn additional_untap_retains_added_program_when_original_is_later_prevented() {
    let (mut game, source, permanent, alice, shield) =
        additional_untap_fixture(vec![Effect::gain_life(3)]);
    game.effect_store
        .replacement_effects
        .add_resolution_effect(ReplacementEffect::with_matcher(
            source,
            alice,
            crate::events::permanents::matchers::WouldBecomeUntappedMatcher::new(
                crate::target::ObjectFilter::specific(permanent),
            ),
            ReplacementAction::Prevent,
        ));
    let mut ctx = crate::effects::ExecutionContext::new_default(source, alice);
    let outcome = process_untap_with_execution_context(&mut game, permanent, &mut ctx).unwrap();
    assert_eq!(outcome.value, crate::effect::OutcomeValue::Count(0));
    assert!(game.is_tapped(permanent));
    assert_eq!(game.player(alice).unwrap().life, 23);
    assert!(
        game.effect_store
            .replacement_effects
            .get_effect(shield)
            .is_none()
    );
    let mut events = game.take_pending_trigger_events();
    events.extend(outcome.events);
    assert_eq!(
        events
            .iter()
            .filter_map(|event| event.downcast::<crate::events::PermanentUntappedEvent>())
            .count(),
        0
    );
    assert_eq!(
        events
            .iter()
            .filter_map(|event| event.downcast::<crate::events::LifeGainEvent>())
            .map(|event| event.amount)
            .collect::<Vec<_>>(),
        vec![3]
    );
}

#[test]
fn additional_object_counter_program_targets_the_captured_object() {
    let (mut game, recipient, alice) = setup();
    let recipient = game
        .move_object_by_effect(recipient, Zone::Battlefield)
        .unwrap();
    let source = game.create_object_from_card(
        &crate::card::CardBuilder::new(CardId::new(), "Distinct counter program source")
            .card_types(vec![crate::types::CardType::Enchantment])
            .build(),
        alice,
        Zone::Battlefield,
    );
    let shield =
        game.effect_store
            .replacement_effects
            .add_one_shot_effect(ReplacementEffect::with_matcher(
                source,
                alice,
                crate::events::counters::matchers::WouldPutCountersMatcher::new(
                    crate::target::ObjectFilter::specific(recipient),
                    Some(crate::object::CounterType::Charge),
                ),
                ReplacementAction::Additionally(vec![Effect::put_counters(
                    crate::object::CounterType::PlusOnePlusOne,
                    2,
                    crate::target::ChooseSpec::target(crate::target::ChooseSpec::creature()),
                )]),
            ));
    let mut ctx = crate::effects::ExecutionContext::new_default(source, alice);
    let event = crate::events::Event::put_counters(
        recipient,
        crate::object::CounterType::Charge,
        1,
        ctx.cause.clone(),
    );
    let outcome =
        crate::effects::counters::execute_object_counter_placement(&mut game, &mut ctx, event)
            .unwrap();
    assert_eq!(outcome.value, crate::effect::OutcomeValue::Count(1));
    assert_eq!(
        game.counter_count(recipient, crate::object::CounterType::Charge),
        1
    );
    assert_eq!(
        game.counter_count(recipient, crate::object::CounterType::PlusOnePlusOne),
        2
    );
    assert!(game.object(source).unwrap().counters.is_empty());
    assert!(
        game.effect_store
            .replacement_effects
            .get_effect(shield)
            .is_none()
    );
    let mut events = game.take_pending_trigger_events();
    events.extend(outcome.events);
    let mut amounts = events
        .iter()
        .filter_map(|event| event.downcast::<crate::events::MarkersChangedEvent>())
        .map(|event| event.amount)
        .collect::<Vec<_>>();
    amounts.sort();
    assert_eq!(amounts, vec![1, 2]);
}

#[test]
fn additional_player_counter_program_targets_recipient_instead_of_source_controller() {
    let (mut game, source, alice) = setup();
    let source = game
        .move_object_by_effect(source, Zone::Battlefield)
        .unwrap();
    let bob = PlayerId::from_index(1);
    let mut replacement =
        crate::static_abilities::StaticAbility::double_player_counters_replacement(
            crate::target::PlayerFilter::Specific(bob),
            Some(crate::object::CounterType::Energy),
            "Added player-counter target program".into(),
        )
        .generate_replacement_effect(source, alice)
        .unwrap();
    replacement.replacement = ReplacementAction::Additionally(vec![Effect::gain_life_target(3)]);
    let shield = game
        .effect_store
        .replacement_effects
        .add_one_shot_effect(replacement);
    let mut ctx = crate::effects::ExecutionContext::new_default(source, alice);
    ctx.targets = vec![crate::effects::ResolvedTarget::Player(alice)];
    let event = crate::events::Event::put_player_counters(
        bob,
        crate::object::CounterType::Energy,
        2,
        ctx.cause.clone(),
    );
    let outcome =
        crate::effects::counters::execute_player_counter_placement(&mut game, &mut ctx, event)
            .unwrap();
    assert_eq!(outcome.value, crate::effect::OutcomeValue::Count(2));
    assert_eq!(
        game.player(bob)
            .unwrap()
            .counter_count(crate::object::CounterType::Energy),
        2
    );
    assert_eq!(game.player(alice).unwrap().life, 20);
    assert_eq!(game.player(bob).unwrap().life, 23);
    assert!(
        matches!(ctx.targets.as_slice(), [crate::effects::ResolvedTarget::Player(player)] if *player==alice)
    );
    assert!(
        game.effect_store
            .replacement_effects
            .get_effect(shield)
            .is_none()
    );
    let mut events = game.take_pending_trigger_events();
    events.extend(outcome.events);
    let gains = events
        .iter()
        .filter_map(|event| event.downcast::<crate::events::LifeGainEvent>())
        .collect::<Vec<_>>();
    assert_eq!(gains.len(), 1);
    assert_eq!(gains[0].player, bob);
    assert_eq!(gains[0].amount, 3);
}

fn execute_added_token_case(
    game: &mut GameState,
    ctx: &mut crate::effects::ExecutionContext,
    kind: u8,
) -> Result<crate::effect::EffectOutcome, crate::effects::ExecutionError> {
    use crate::effects::EffectExecutor;
    match kind {
        0 => {
            let token = crate::cards::CardDefinitionBuilder::new(
                CardId::new(),
                "Additional program test token",
            )
            .token()
            .card_types(vec![crate::types::CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(1, 1))
            .build();
            crate::effects::CreateTokenEffect::one(token).execute(game, ctx)
        }
        1 => crate::effects::CreateTokenCopyEffect::one(crate::target::ChooseSpec::SpecificObject(
            ctx.source,
        ))
        .execute(game, ctx),
        2 => {
            crate::effects::IncubateEffect::you(Value::Fixed(1), Value::Fixed(1)).execute(game, ctx)
        }
        _ => unreachable!(),
    }
}

fn check_added_token_program(kind: u8, mode: u8) {
    let (mut game, source, alice) = setup();
    let source = game
        .move_object_by_effect(source, Zone::Battlefield)
        .unwrap();
    let mut effects = vec![Effect::gain_life(2)];
    if mode == 1 {
        effects.push(Effect::lose_life(Value::X));
    }
    if mode == 2 {
        effects.push(Effect::may(vec![Effect::gain_life(4)]));
    }
    let shield =
        game.effect_store
            .replacement_effects
            .add_one_shot_effect(ReplacementEffect::with_matcher(
                source,
                alice,
                crate::events::tokens::matchers::WouldCreateTokensUnderControlMatcher::new(
                    crate::target::PlayerFilter::Specific(alice),
                ),
                ReplacementAction::Additionally(effects),
            ));
    game.take_pending_trigger_events();
    let before_next_id = game.next_object_id_counter();
    let before_live = game.objects_in_deterministic_order().len();
    let mut dm = DiscardPayloadBoolean {
        pause: mode == 2,
        pending: false,
        questions: 0,
    };
    let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut dm);
    let result = execute_added_token_case(&mut game, &mut ctx, kind);
    if mode == 1 {
        assert!(
            matches!(
                result,
                Err(crate::effects::ExecutionError::UnresolvableValue(_))
            ),
            "added token program failure must propagate"
        );
        drop(ctx);
        assert_eq!(game.player(alice).unwrap().life, 20);
        assert_eq!(game.battlefield, vec![source]);
        assert_eq!(game.objects_in_deterministic_order().len(), before_live);
        assert_eq!(game.next_object_id_counter(), before_next_id);
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(shield)
                .is_some()
        );
        assert!(game.take_pending_trigger_events().is_empty());
        return;
    }
    let mut outcome = result.unwrap();
    if mode == 2 {
        assert!(
            ctx.decision_maker.awaiting_choice(),
            "added token program must expose its pending question"
        );
        assert!(outcome.events.is_empty());
        drop(ctx);
        assert_eq!(dm.questions, 1);
        assert_eq!(game.player(alice).unwrap().life, 20);
        assert_eq!(game.battlefield, vec![source]);
        assert_eq!(game.objects_in_deterministic_order().len(), before_live);
        assert_eq!(game.next_object_id_counter(), before_next_id);
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(shield)
                .is_some()
        );
        assert!(game.take_pending_trigger_events().is_empty());
        let mut replay = DiscardPayloadBoolean {
            pause: false,
            pending: false,
            questions: 0,
        };
        let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut replay);
        outcome = execute_added_token_case(&mut game, &mut ctx, kind).unwrap();
        assert!(!ctx.decision_maker.awaiting_choice());
        drop(ctx);
        assert_eq!(replay.questions, 1);
    } else {
        assert!(!ctx.decision_maker.awaiting_choice());
        drop(ctx);
    }
    let ids = outcome
        .objects()
        .expect("the original token result must retain its object value");
    assert_eq!(ids.len(), 1);
    assert!(
        game.object(ids[0])
            .is_some_and(|object| object.zone == Zone::Battlefield)
    );
    assert_eq!(game.battlefield.len(), 2);
    assert_eq!(
        game.player(alice).unwrap().life,
        if mode == 2 { 26 } else { 22 }
    );
    assert!(
        game.effect_store
            .replacement_effects
            .get_effect(shield)
            .is_none()
    );
    let mut events = game.take_pending_trigger_events();
    events.extend(outcome.events);
    assert_eq!(
        events
            .iter()
            .filter_map(|event| event.downcast::<crate::events::CreateTokensEvent>())
            .map(|event| event.count)
            .sum::<u32>(),
        1
    );
    let mut gains = events
        .iter()
        .filter_map(|event| event.downcast::<crate::events::LifeGainEvent>())
        .map(|event| event.amount)
        .collect::<Vec<_>>();
    gains.sort();
    assert_eq!(gains, if mode == 2 { vec![2, 4] } else { vec![2] });
}

#[test]
fn additional_ordinary_tokens_success() {
    check_added_token_program(0, 0);
}

#[test]
fn additional_ordinary_tokens_failure() {
    check_added_token_program(0, 1);
}

#[test]
fn additional_ordinary_tokens_pending_replay() {
    check_added_token_program(0, 2);
}

#[test]
fn additional_token_copies_success() {
    check_added_token_program(1, 0);
}

#[test]
fn additional_token_copies_failure() {
    check_added_token_program(1, 1);
}

#[test]
fn additional_token_copies_pending_replay() {
    check_added_token_program(1, 2);
}

#[test]
fn additional_incubate_success() {
    check_added_token_program(2, 0);
}

#[test]
fn additional_incubate_failure() {
    check_added_token_program(2, 1);
}

#[test]
fn additional_incubate_pending_replay() {
    check_added_token_program(2, 2);
}

fn check_additional_draw_program(mode: u8) {
    let (mut game, source, alice) = setup();
    let source = game
        .move_object_by_effect(source, Zone::Battlefield)
        .unwrap();
    for index in 0..3 {
        prefix_card(
            &mut game,
            alice,
            Zone::Library,
            &format!("Added draw {index}"),
        );
    }
    let library = game.player(alice).unwrap().library.clone();
    let mut effects = vec![Effect::gain_life(3)];
    if mode == 1 {
        effects.push(Effect::lose_life(Value::X));
    }
    if mode == 2 {
        effects.push(Effect::may(vec![Effect::gain_life(4)]));
    }
    let shield =
        game.effect_store
            .replacement_effects
            .add_one_shot_effect(ReplacementEffect::with_matcher(
                source,
                alice,
                crate::events::cards::matchers::WouldDrawCardMatcher::you(),
                ReplacementAction::Additionally(effects),
            ));
    game.take_pending_trigger_events();
    let before_live = game.objects_in_deterministic_order().len();
    let before_id = game.next_object_id_counter();
    let mut dm = DiscardPayloadBoolean {
        pause: mode == 2,
        pending: false,
        questions: 0,
    };
    let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut dm);
    let result = crate::effects::execute_effect(&mut game, &Effect::draw(1), &mut ctx);
    if mode == 1 {
        assert!(matches!(
            result,
            Err(crate::effects::ExecutionError::UnresolvableValue(_))
        ));
        assert!(!ctx.decision_maker.awaiting_choice());
        drop(ctx);
        assert!(game.player(alice).unwrap().hand.is_empty());
        assert_eq!(game.player(alice).unwrap().library, library);
        assert_eq!(game.player(alice).unwrap().life, 20);
        assert_eq!(game.objects_in_deterministic_order().len(), before_live);
        assert_eq!(game.next_object_id_counter(), before_id);
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(shield)
                .is_some()
        );
        assert!(game.take_pending_trigger_events().is_empty());
        return;
    }
    let mut outcome = result.unwrap();
    if mode == 2 {
        assert!(ctx.decision_maker.awaiting_choice());
        assert!(outcome.events.is_empty());
        drop(ctx);
        assert_eq!(dm.questions, 1);
        assert!(game.player(alice).unwrap().hand.is_empty());
        assert_eq!(game.player(alice).unwrap().library, library);
        assert_eq!(game.player(alice).unwrap().life, 20);
        assert_eq!(game.objects_in_deterministic_order().len(), before_live);
        assert_eq!(game.next_object_id_counter(), before_id);
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(shield)
                .is_some()
        );
        assert!(game.take_pending_trigger_events().is_empty());
        let mut replay = DiscardPayloadBoolean {
            pause: false,
            pending: false,
            questions: 0,
        };
        let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut replay);
        outcome = crate::effects::execute_effect(&mut game, &Effect::draw(1), &mut ctx).unwrap();
        assert!(!ctx.decision_maker.awaiting_choice());
        drop(ctx);
        assert_eq!(replay.questions, 1);
    } else {
        assert!(!ctx.decision_maker.awaiting_choice());
        drop(ctx);
    }
    assert_eq!(outcome.count_or_zero(), 1);
    assert_eq!(game.player(alice).unwrap().hand.len(), 1);
    assert_eq!(game.player(alice).unwrap().library.len(), 2);
    assert_eq!(
        game.player(alice).unwrap().life,
        if mode == 2 { 27 } else { 23 }
    );
    assert!(
        game.effect_store
            .replacement_effects
            .get_effect(shield)
            .is_none()
    );
    let mut events = game.take_pending_trigger_events();
    events.extend(outcome.events);
    assert_eq!(
        events
            .iter()
            .filter_map(|event| event.downcast::<crate::events::CardsDrawnEvent>())
            .map(|draw| draw.amount())
            .sum::<u32>(),
        1
    );
    let mut gains = events
        .iter()
        .filter_map(|event| event.downcast::<crate::events::LifeGainEvent>())
        .map(|gain| gain.amount)
        .collect::<Vec<_>>();
    gains.sort();
    assert_eq!(gains, if mode == 2 { vec![3, 4] } else { vec![3] });
}

#[test]
fn additional_draw_original_and_payload_success() {
    check_additional_draw_program(0);
}
#[test]
fn additional_draw_error_restores_original_and_prefix() {
    check_additional_draw_program(1);
}
#[test]
fn additional_draw_pending_replays_original_and_payload_once() {
    check_additional_draw_program(2);
}

#[test]
fn additional_draw_program_observes_each_original_before_next_draw() {
    let (mut game, source, alice) = setup();
    let source = game
        .move_object_by_effect(source, Zone::Battlefield)
        .unwrap();
    for index in 0..3 {
        prefix_card(
            &mut game,
            alice,
            Zone::Library,
            &format!("Sequenced draw {index}"),
        );
    }
    let shield = game
        .effect_store
        .replacement_effects
        .add_effect(ReplacementEffect::with_matcher(
            source,
            alice,
            crate::events::cards::matchers::WouldDrawCardMatcher::you(),
            ReplacementAction::Additionally(vec![Effect::gain_life(Value::CardsInHand(
                crate::target::PlayerFilter::You,
            ))]),
        ));
    game.take_pending_trigger_events();
    let mut ctx = crate::effects::ExecutionContext::new_default(source, alice);
    let outcome = crate::effects::execute_effect(&mut game, &Effect::draw(2), &mut ctx).unwrap();
    assert!(!ctx.decision_maker.awaiting_choice());
    assert_eq!(outcome.count_or_zero(), 2);
    assert_eq!(game.player(alice).unwrap().hand.len(), 2);
    assert_eq!(game.player(alice).unwrap().life, 23);
    assert!(
        game.effect_store
            .replacement_effects
            .get_effect(shield)
            .is_some()
    );
    let mut events = game.take_pending_trigger_events();
    events.extend(outcome.events);
    let gains = events
        .iter()
        .filter_map(|event| event.downcast::<crate::events::LifeGainEvent>())
        .map(|gain| gain.amount)
        .collect::<Vec<_>>();
    assert_eq!(gains, vec![1, 2]);
    assert_eq!(
        events
            .iter()
            .filter_map(|event| event.downcast::<crate::events::CardsDrawnEvent>())
            .map(|draw| draw.amount())
            .sum::<u32>(),
        2
    );
}

#[test]
fn additional_draw_nested_draw_keeps_history_local_to_each_original() {
    let (mut game, source, alice) = setup();
    let source = game
        .move_object_by_effect(source, Zone::Battlefield)
        .unwrap();
    for index in 0..6 {
        prefix_card(
            &mut game,
            alice,
            Zone::Library,
            &format!("Nested addition {index}"),
        );
    }
    let shield = game
        .effect_store
        .replacement_effects
        .add_effect(ReplacementEffect::with_matcher(
            source,
            alice,
            crate::events::cards::matchers::WouldDrawCardMatcher::you(),
            ReplacementAction::Additionally(vec![Effect::draw(1)]),
        ));
    game.take_pending_trigger_events();
    let mut ctx = crate::effects::ExecutionContext::new_default(source, alice);
    let outcome = crate::effects::execute_effect(&mut game, &Effect::draw(2), &mut ctx).unwrap();
    assert!(!ctx.decision_maker.awaiting_choice());
    assert_eq!(
        outcome.count_or_zero(),
        2,
        "added draws do not replace the original instruction's quantity"
    );
    assert_eq!(game.player(alice).unwrap().hand.len(), 4);
    assert_eq!(game.player(alice).unwrap().library.len(), 2);
    assert!(
        game.effect_store
            .replacement_effects
            .get_effect(shield)
            .is_some()
    );
    assert!(ctx.replacement.suppressed_replacement_effects.is_empty());
    let mut events = game.take_pending_trigger_events();
    events.extend(outcome.events);
    assert_eq!(
        events
            .iter()
            .filter_map(|event| event.downcast::<crate::events::CardsDrawnEvent>())
            .map(|draw| draw.amount())
            .sum::<u32>(),
        4
    );
}

fn check_keyword_replacement_receipt(proliferate: bool, temporary: bool) {
    let (mut game, source, alice) = setup();
    let source = game
        .move_object_by_effect(source, Zone::Battlefield)
        .unwrap();
    let bob = PlayerId::from_index(1);
    let replacement_source = prefix_card(
        &mut game,
        bob,
        Zone::Battlefield,
        "Keyword replacement source",
    );
    let action = if proliferate {
        crate::events::KeywordActionKind::Proliferate
    } else {
        crate::events::KeywordActionKind::Learn
    };
    let replacement = ReplacementEffect::with_matcher(
        replacement_source,
        bob,
        crate::events::WouldKeywordActionMatcher::new(
            action,
            crate::target::ObjectFilter::default(),
        ),
        ReplacementAction::Instead(vec![Effect::gain_life(3)]),
    );
    let shield = if temporary {
        None
    } else {
        Some(
            game.effect_store
                .replacement_effects
                .add_one_shot_effect(replacement.clone()),
        )
    };
    game.take_pending_trigger_events();
    let mut ctx = crate::effects::ExecutionContext::new_default(source, alice);
    if temporary {
        ctx.replacement
            .additional_replacement_effects
            .push(replacement);
    }
    let outcome = if proliferate {
        crate::effects::execute_effect(
            &mut game,
            &Effect::new(crate::effects::ProliferateEffect::new(1)),
            &mut ctx,
        )
    } else {
        crate::effects::execute_effect(
            &mut game,
            &Effect::new(crate::effects::LearnEffect::new()),
            &mut ctx,
        )
    }
    .unwrap();
    assert!(!ctx.decision_maker.awaiting_choice());
    assert_eq!(
        game.player(bob).unwrap().life,
        23,
        "payload must run as captured replacement controller"
    );
    assert_eq!(game.player(alice).unwrap().life, 20);
    assert_eq!(ctx.source, source);
    assert_eq!(ctx.controller, alice);
    assert!(ctx.replacement.suppressed_replacement_effects.is_empty());
    assert!(
        ctx.replacement
            .suppressed_replacement_effect_keys
            .is_empty()
    );
    if temporary {
        assert_eq!(ctx.replacement.additional_replacement_effects.len(), 1);
    }
    if let Some(shield) = shield {
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(shield)
                .is_none()
        );
    }
    let mut events = game.take_pending_trigger_events();
    events.extend(outcome.events);
    let gains = events
        .iter()
        .filter_map(|event| event.downcast::<crate::events::LifeGainEvent>())
        .collect::<Vec<_>>();
    assert_eq!(gains.len(), 1);
    assert_eq!(gains[0].player, bob);
    assert_eq!(gains[0].amount, 3);
    assert!(!events.iter().any(|event| {
        event
            .downcast::<crate::events::KeywordActionEvent>()
            .is_some_and(|keyword| keyword.action == action)
    }));
}

#[test]
fn learn_temporary_replacement_uses_captured_controller() {
    check_keyword_replacement_receipt(false, true);
}
#[test]
fn proliferate_temporary_replacement_uses_captured_controller() {
    check_keyword_replacement_receipt(true, true);
}
#[test]
fn learn_consumed_one_shot_retains_captured_controller() {
    check_keyword_replacement_receipt(false, false);
}
#[test]
fn proliferate_consumed_one_shot_retains_captured_controller() {
    check_keyword_replacement_receipt(true, false);
}

#[test]
fn expanded_registered_choice_retains_program_when_original_is_prevented() {
    check_expanded_choice_continuation(false);
}

#[test]
fn expanded_ephemeral_choice_retains_program_when_original_is_prevented() {
    check_expanded_choice_continuation(true);
}

fn check_expanded_choice_continuation(ephemeral: bool) {
    let (mut game, source, alice) = setup();
    let source = game
        .move_object_by_effect(source, Zone::Battlefield)
        .unwrap();
    let mut addition = ReplacementEffect::with_matcher(
        source,
        alice,
        crate::events::life::matchers::WouldGainLifeMatcher::you(),
        ReplacementAction::Additionally(vec![Effect::gain_life(3)]),
    );
    addition.priority_override = Some(crate::events::ReplacementPriority::SelfReplacement);
    let addition = game
        .effect_store
        .replacement_effects
        .add_one_shot_effect(addition);
    let double = ReplacementEffect::with_matcher(
        source,
        alice,
        crate::events::life::matchers::WouldGainLifeMatcher::you(),
        ReplacementAction::Double,
    );
    let prevent = ReplacementEffect::with_matcher(
        source,
        alice,
        crate::events::life::matchers::WouldGainLifeMatcher::you(),
        ReplacementAction::Prevent,
    );
    let mut temporary = Vec::new();
    let (double, prevent) = if ephemeral {
        temporary = vec![double, prevent];
        assign_ephemeral_effect_ids(&mut temporary, u64::MAX / 2);
        (temporary[0].id, temporary[1].id)
    } else {
        (
            game.effect_store
                .replacement_effects
                .add_one_shot_effect(double),
            game.effect_store
                .replacement_effects
                .add_one_shot_effect(prevent),
        )
    };
    game.take_pending_trigger_events();
    let pending = process_trait_event_with_additional_effects(
        &mut game,
        crate::events::Event::life_gain(alice, 2),
        &temporary,
    )
    .expect("finite replacement fixture evaluates successfully");
    let (original, programs) = pending.clone().into_expansion();
    assert_eq!(programs.len(), 1);
    let TraitEventResult::NeedsChoice {
        applicable_effects,
        applied_effects,
        ..
    } = original
    else {
        panic!("the remaining competing replacements must produce a choice");
    };
    assert!(applicable_effects.contains(&double));
    assert!(applicable_effects.contains(&prevent));
    assert!(applied_effects.contains(&addition));
    assert_eq!(game.player(alice).unwrap().life, 20);
    assert!(game.take_pending_trigger_events().is_empty());
    assert!(
        game.effect_store
            .replacement_effects
            .get_effect(addition)
            .is_none()
    );
    let invalid = continue_replacement_choice_with_scope(
        &mut game,
        pending.clone(),
        ReplacementEffectId(u64::MAX),
        None,
        &temporary,
        None,
    );
    assert!(matches!(
        invalid,
        Err(crate::effects::ExecutionError::InternalError(_))
    ));
    assert_eq!(game.player(alice).unwrap().life, 20);
    assert!(game.take_pending_trigger_events().is_empty());
    if !ephemeral {
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(double)
                .is_some()
        );
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(prevent)
                .is_some()
        );
    }
    let continued =
        continue_replacement_choice_with_scope(&mut game, pending, double, None, &temporary, None)
            .unwrap();
    let (original, programs) = continued.into_expansion();
    assert!(matches!(original, TraitEventResult::Prevented));
    assert_eq!(
        programs.len(),
        1,
        "prevention of the original branch must retain earlier additions"
    );
    let program = &programs[0];
    assert_eq!(program.source, source);
    assert_eq!(program.controller, alice);
    assert_eq!(program.source_snapshot.as_ref().unwrap().object_id, source);
    assert_eq!(program.effects.len(), 1);
    let captured = crate::events::downcast_event::<crate::events::LifeGainEvent>(
        program.context.event.inner(),
    )
    .unwrap();
    assert_eq!(captured.amount, 2);
    assert!(program.context.applied_effects.contains(&addition));
    assert!(
        !program.context.applied_effects.contains(&double),
        "an effect applied after this branch split belongs to the original branch only"
    );
    assert!(!program.context.applied_effects.contains(&prevent));
    assert_eq!(
        game.player(alice).unwrap().life,
        20,
        "choice continuation prepares programs rather than executing them during matching"
    );
    assert!(game.take_pending_trigger_events().is_empty());
    if !ephemeral {
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(double)
                .is_none()
        );
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(prevent)
                .is_none()
        );
    }
}

struct PreferPreparedZoneAddition(ObjectId);
impl crate::decision::DecisionMaker for PreferPreparedZoneAddition {
    fn decide_options(
        &mut self,
        _: &GameState,
        context: &crate::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        vec![
            context
                .options
                .iter()
                .find(|option| option.legal && option.object_id == Some(self.0))
                .or_else(|| context.options.iter().find(|option| option.legal))
                .expect("legal replacement choice")
                .index,
        ]
    }
}

fn prepared_zone_addition_sources(game: &mut GameState, alice: PlayerId) -> (ObjectId, ObjectId) {
    let card = crate::card::CardBuilder::new(CardId::new(), "Prepared program source")
        .card_types(vec![crate::types::CardType::Artifact])
        .build();
    (
        game.create_object_from_card(&card, alice, Zone::Battlefield),
        game.create_object_from_card(&card, PlayerId::from_index(1), Zone::Battlefield),
    )
}

#[test]
fn prepared_zone_continuation_retains_additions_in_application_order() {
    let (mut game, object, alice) = setup();
    let (earlier_source, later_source) = prepared_zone_addition_sources(&mut game, alice);
    let bob = PlayerId::from_index(1);
    let matcher = |to| {
        crate::events::zones::matchers::WouldChangeZoneMatcher::new(
            crate::target::ObjectFilter::specific(object),
            Some(Zone::Hand),
            Some(to),
        )
    };
    let early =
        game.effect_store
            .replacement_effects
            .add_one_shot_effect(ReplacementEffect::with_matcher(
                earlier_source,
                alice,
                matcher(Zone::Graveyard),
                ReplacementAction::Additionally(vec![Effect::gain_life(3)]),
            ));
    let destination =
        game.effect_store
            .replacement_effects
            .add_one_shot_effect(ReplacementEffect::with_matcher(
                later_source,
                bob,
                matcher(Zone::Graveyard),
                ReplacementAction::InteractiveChooseDestination {
                    destinations: vec![Zone::Exile],
                    description: "Resolve the destination".into(),
                },
            ));
    let later =
        game.effect_store
            .replacement_effects
            .add_one_shot_effect(ReplacementEffect::with_matcher(
                later_source,
                bob,
                matcher(Zone::Exile),
                ReplacementAction::Additionally(vec![Effect::gain_life(7)]),
            ));
    let mut dm = PreferPreparedZoneAddition(earlier_source);
    let plan = prepare_zone_change_proposal_scoped(
        &mut game,
        object,
        Zone::Hand,
        Zone::Graveyard,
        crate::events::cause::EventCause::effect(),
        &mut dm,
        &[],
        None,
        None,
        None,
    )
    .unwrap();
    let EventOutcome::Proceed(PreparedZoneProposal::Ready(prepared)) = plan.original else {
        panic!("ready exile proposal");
    };
    assert_eq!(prepared.final_zone, Zone::Exile);
    assert_eq!(
        plan.programs.len(),
        2,
        "no loss or duplication across the destination rescan"
    );
    assert_eq!(plan.programs[0].controller, alice);
    assert_eq!(plan.programs[1].controller, bob);
    assert_eq!(
        crate::events::downcast_event::<crate::events::ZoneChangeEvent>(
            plan.programs[0].context.event.inner()
        )
        .unwrap()
        .to,
        Zone::Graveyard
    );
    assert_eq!(
        crate::events::downcast_event::<crate::events::ZoneChangeEvent>(
            plan.programs[1].context.event.inner()
        )
        .unwrap()
        .to,
        Zone::Exile
    );
    assert!(plan.programs[1].context.applied_effects.contains(&early));
    assert!(
        plan.programs[1]
            .context
            .applied_effects
            .contains(&destination)
    );
    assert!(plan.programs[1].context.applied_effects.contains(&later));
    assert_eq!(game.object(object).unwrap().zone, Zone::Hand);
    assert_eq!(game.player(alice).unwrap().life, 20);
    assert_eq!(game.player(bob).unwrap().life, 20);
}

fn prepared_zone_terminal_keeps_addition(mode: u8) {
    let (mut game, object, alice) = setup();
    let (earlier_source, later_source) = prepared_zone_addition_sources(&mut game, alice);
    let bob = PlayerId::from_index(1);
    let matcher = || {
        crate::events::zones::matchers::WouldChangeZoneMatcher::new(
            crate::target::ObjectFilter::specific(object),
            Some(Zone::Hand),
            Some(Zone::Graveyard),
        )
    };
    game.effect_store
        .replacement_effects
        .add_one_shot_effect(ReplacementEffect::with_matcher(
            earlier_source,
            alice,
            matcher(),
            ReplacementAction::Additionally(vec![Effect::gain_life(3)]),
        ));
    let terminal = match mode {
        0 => ReplacementAction::Prevent,
        1 => ReplacementAction::ChangeDestination(Zone::Hand),
        _ => ReplacementAction::Instead(vec![Effect::gain_life(5)]),
    };
    game.effect_store
        .replacement_effects
        .add_one_shot_effect(ReplacementEffect::with_matcher(
            later_source,
            bob,
            matcher(),
            terminal,
        ));
    let mut dm = PreferPreparedZoneAddition(earlier_source);
    let plan = prepare_zone_change_proposal_scoped(
        &mut game,
        object,
        Zone::Hand,
        Zone::Graveyard,
        crate::events::cause::EventCause::effect(),
        &mut dm,
        &[],
        None,
        None,
        None,
    )
    .unwrap();
    assert_eq!(
        plan.programs.len(),
        1,
        "terminal original must retain its appended program"
    );
    assert_eq!(plan.programs[0].source, earlier_source);
    match mode {
        0 => assert!(matches!(plan.original, EventOutcome::Prevented)),
        1 => assert!(matches!(plan.original, EventOutcome::NotApplicable)),
        _ => assert!(matches!(plan.original, EventOutcome::Replaced)),
    }
    assert_eq!(game.object(object).unwrap().zone, Zone::Hand);
    assert_eq!(
        game.player(alice).unwrap().life,
        20,
        "addition waits for its owner"
    );
    assert_eq!(
        game.player(bob).unwrap().life,
        if mode == 2 { 25 } else { 20 }
    );
}
#[test]
fn prepared_zone_prevented_original_keeps_addition() {
    prepared_zone_terminal_keeps_addition(0);
}
#[test]
fn prepared_zone_same_zone_noop_keeps_addition() {
    prepared_zone_terminal_keeps_addition(1);
}
#[test]
fn prepared_zone_instead_original_keeps_addition() {
    prepared_zone_terminal_keeps_addition(2);
}

#[test]
fn prepared_zone_entry_phase_keeps_departure_addition_unexecuted() {
    let (mut game, object, alice) = setup();
    let (earlier_source, later_source) = prepared_zone_addition_sources(&mut game, alice);
    let matcher = || {
        crate::events::zones::matchers::WouldChangeZoneMatcher::new(
            crate::target::ObjectFilter::specific(object),
            Some(Zone::Hand),
            Some(Zone::Graveyard),
        )
    };
    game.effect_store
        .replacement_effects
        .add_one_shot_effect(ReplacementEffect::with_matcher(
            earlier_source,
            alice,
            matcher(),
            ReplacementAction::Additionally(vec![Effect::gain_life(3)]),
        ));
    game.effect_store
        .replacement_effects
        .add_one_shot_effect(ReplacementEffect::with_matcher(
            later_source,
            alice,
            matcher(),
            ReplacementAction::ChangeDestination(Zone::Battlefield),
        ));
    let mut dm = PreferPreparedZoneAddition(earlier_source);
    let plan = prepare_zone_change_scoped(
        &mut game,
        object,
        Zone::Hand,
        Zone::Graveyard,
        crate::events::cause::EventCause::effect(),
        &mut dm,
        &[],
        None,
        None,
        Vec::new(),
        None,
    )
    .unwrap();
    let EventOutcome::Proceed(prepared) = plan.original else {
        panic!("ready entry");
    };
    assert_eq!(prepared.final_zone, Zone::Battlefield);
    assert!(prepared.entry.is_some());
    assert_eq!(plan.programs.len(), 1);
    assert_eq!(plan.programs[0].source, earlier_source);
    assert_eq!(game.object(object).unwrap().zone, Zone::Hand);
    assert_eq!(game.player(alice).unwrap().life, 20);
}

fn draw_adapter_addition_fixture(
    action: Option<ReplacementAction>,
) -> (
    GameState,
    ObjectId,
    PlayerId,
    ObjectId,
    ReplacementEffectId,
    Option<ReplacementEffectId>,
) {
    let (mut game, _, alice) = setup();
    let (early, late) = prepared_zone_addition_sources(&mut game, alice);
    let bob = PlayerId::from_index(1);
    let card = prefix_card(&mut game, alice, Zone::Library, "Uncommitted original draw");
    let early_id =
        game.effect_store
            .replacement_effects
            .add_one_shot_effect(ReplacementEffect::with_matcher(
                early,
                bob,
                crate::events::cards::matchers::WouldDrawCardMatcher::any_player(),
                ReplacementAction::Additionally(vec![Effect::gain_life(3)]),
            ));
    let late_id = action.map(|action| {
        game.effect_store
            .replacement_effects
            .add_one_shot_effect(ReplacementEffect::with_matcher(
                late,
                bob,
                crate::events::cards::matchers::WouldDrawCardMatcher::any_player(),
                action,
            ))
    });
    game.take_pending_trigger_events();
    (game, early, alice, card, early_id, late_id)
}

fn assert_draw_adapter_retains_addition(action: Option<ReplacementAction>, kind: u8) {
    let (mut game, early, alice, card, early_id, late_id) = draw_adapter_addition_fixture(action);
    let bob = PlayerId::from_index(1);
    let alice_hand_before = game.player(alice).unwrap().hand.as_slice().to_vec();
    let bob_hand_before = game.player(bob).unwrap().hand.as_slice().to_vec();
    let mut dm = PreferPreparedZoneAddition(early);
    let (original, programs) = process_draw(&mut game, alice, 2, true, &mut dm)
        .unwrap()
        .into_expansion();
    assert!(!dm.awaiting_choice());
    assert_eq!(programs.len(), 1);
    assert_eq!(programs[0].source, early);
    assert_eq!(programs[0].controller, bob);
    assert_eq!(
        programs[0].source_snapshot.as_ref().unwrap().object_id,
        early
    );
    assert!(programs[0].context.applied_effects.contains(&early_id));
    let captured = crate::events::downcast_event::<crate::events::DrawEvent>(
        programs[0].context.event.inner(),
    )
    .unwrap();
    assert_eq!(captured.player, alice);
    assert_eq!(captured.count, 2);
    assert!(captured.is_first_this_turn);
    match kind {
        0 | 3 => {
            let ResolvedDrawOutcome::Proceed(event) = original else {
                panic!("complete original draw proposal");
            };
            let draw =
                crate::events::downcast_event::<crate::events::DrawEvent>(event.inner()).unwrap();
            assert_eq!(draw.player, if kind == 3 { bob } else { alice });
            assert_eq!(draw.count, 2);
        }
        1 => assert!(matches!(original, ResolvedDrawOutcome::Prevented)),
        2 => {
            let ResolvedDrawOutcome::Replaced {
                context,
                payload,
                controller,
                ..
            } = original
            else {
                panic!("Instead receipt");
            };
            assert_eq!(controller, bob);
            assert!(context.applied_effects.contains(&early_id));
            assert!(context.applied_effects.contains(&late_id.unwrap()));
            let gains = payload
                .events
                .iter()
                .filter_map(|e| e.downcast::<crate::events::LifeGainEvent>())
                .collect::<Vec<_>>();
            assert_eq!(gains.len(), 1);
            assert_eq!(gains[0].amount, 5);
        }
        _ => unreachable!(),
    }
    assert_eq!(game.player(alice).unwrap().life, 20);
    assert_eq!(
        game.player(bob).unwrap().life,
        if kind == 2 { 25 } else { 20 },
        "added instructions must wait for the original draw commit owner"
    );
    assert_eq!(game.object(card).unwrap().zone, Zone::Library);
    assert_eq!(game.player(alice).unwrap().library.as_slice(), &[card]);
    assert_eq!(
        game.player(alice).unwrap().hand.as_slice(),
        alice_hand_before.as_slice()
    );
    assert_eq!(
        game.player(bob).unwrap().hand.as_slice(),
        bob_hand_before.as_slice()
    );
    assert!(
        game.effect_store
            .replacement_effects
            .get_effect(early_id)
            .is_none()
    );
    if let Some(id) = late_id {
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(id)
                .is_none()
        );
    }
    assert!(game.take_pending_trigger_events().is_empty());
}

#[test]
fn draw_adapter_retains_addition_without_committing_original() {
    assert_draw_adapter_retains_addition(None, 0);
}
#[test]
fn draw_adapter_prevented_original_retains_addition() {
    assert_draw_adapter_retains_addition(Some(ReplacementAction::Prevent), 1);
}
#[test]
fn draw_adapter_instead_original_retains_addition_and_payload() {
    assert_draw_adapter_retains_addition(
        Some(ReplacementAction::Instead(vec![Effect::gain_life(5)])),
        2,
    );
}
#[test]
fn draw_adapter_redirect_preserves_original_and_captured_addition_recipients() {
    assert_draw_adapter_retains_addition(Some(ReplacementAction::RedirectDrawToController), 3);
}

struct PreparedDrawAnswers {
    early: ObjectId,
    pause: bool,
    pending: bool,
}
impl crate::decision::DecisionMaker for PreparedDrawAnswers {
    fn decide_options(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        PreferPreparedZoneAddition(self.early).decide_options(game, ctx)
    }
    fn decide_boolean(
        &mut self,
        _: &GameState,
        _: &crate::decisions::context::BooleanContext,
    ) -> bool {
        self.pending = self.pause;
        !self.pause
    }
    fn awaiting_choice(&self) -> bool {
        self.pending
    }
}

#[test]
fn draw_adapter_pending_original_does_not_expose_uncommitted_additions() {
    let (mut game, early, alice, card, early_id, late_id) =
        draw_adapter_addition_fixture(Some(ReplacementAction::Instead(vec![
            Effect::gain_life(5),
            Effect::may(vec![Effect::gain_life(4)]),
        ])));
    let bob = PlayerId::from_index(1);
    let mut dm = PreparedDrawAnswers {
        early,
        pause: true,
        pending: false,
    };
    let (original, programs) = process_draw(&mut game, alice, 1, false, &mut dm)
        .unwrap()
        .into_expansion();
    assert!(matches!(original, ResolvedDrawOutcome::Pending));
    assert!(programs.is_empty());
    assert!(dm.awaiting_choice());
    assert_eq!(game.player(bob).unwrap().life, 20);
    assert_eq!(game.object(card).unwrap().zone, Zone::Library);
    for id in [early_id, late_id.unwrap()] {
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(id)
                .is_some()
        );
    }
    assert!(game.take_pending_trigger_events().is_empty());
    let mut dm = PreparedDrawAnswers {
        early,
        pause: false,
        pending: false,
    };
    let (original, programs) = process_draw(&mut game, alice, 1, false, &mut dm)
        .unwrap()
        .into_expansion();
    assert!(matches!(original, ResolvedDrawOutcome::Replaced { .. }));
    assert_eq!(programs.len(), 1);
    assert_eq!(
        game.player(bob).unwrap().life,
        29,
        "only the answered Instead program executes during preparation"
    );
    for id in [early_id, late_id.unwrap()] {
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(id)
                .is_none()
        );
    }
}

#[test]
fn draw_adapter_original_payload_error_restores_earlier_addition_consumption() {
    let (mut game, early, alice, card, early_id, late_id) =
        draw_adapter_addition_fixture(Some(ReplacementAction::Instead(vec![
            Effect::gain_life(5),
            Effect::lose_life(Value::X),
        ])));
    let mut dm = PreferPreparedZoneAddition(early);
    assert!(matches!(
        process_draw(&mut game, alice, 1, false, &mut dm),
        Err(crate::effects::ExecutionError::UnresolvableValue(_))
    ));
    assert_eq!(game.player(PlayerId::from_index(1)).unwrap().life, 20);
    assert_eq!(game.object(card).unwrap().zone, Zone::Library);
    for id in [early_id, late_id.unwrap()] {
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(id)
                .is_some()
        );
    }
    assert!(game.take_pending_trigger_events().is_empty());
}

mod captured_affected_player_contract_tests {
    use super::*;
    fn check(zone_event: bool, remove: bool) {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = crate::ids::PlayerId::from_index(0);
        let bob = crate::ids::PlayerId::from_index(1);
        assert_eq!(game.turn.active_player, alice);
        let card = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Affected original")
            .card_types(vec![crate::types::CardType::Creature])
            .build();
        let from = if zone_event {
            Zone::Hand
        } else {
            Zone::Battlefield
        };
        let original = game.create_object_from_card(&card, bob, from);
        let source_card =
            crate::card::CardBuilder::new(crate::ids::CardId::new(), "Affected replacement")
                .card_types(vec![crate::types::CardType::Artifact])
                .build();
        let source = game.create_object_from_card(&source_card, alice, Zone::Battlefield);
        let effects = vec![crate::effect::Effect::new(
            crate::effects::GainLifeEffect::with_filter(
                3,
                crate::target::PlayerFilter::IteratedPlayer,
            ),
        )];
        let replacement = if zone_event {
            crate::replacement::ReplacementEffect::with_matcher(
                source,
                alice,
                crate::events::zones::matchers::WouldChangeZoneMatcher::new(
                    crate::target::ObjectFilter::specific(original),
                    Some(from),
                    Some(Zone::Graveyard),
                ),
                crate::replacement::ReplacementAction::Instead(effects),
            )
        } else {
            crate::replacement::ReplacementEffect::with_matcher(
                source,
                alice,
                crate::events::permanents::matchers::WouldBeDestroyedMatcher::new(
                    crate::target::ObjectFilter::specific(original),
                ),
                crate::replacement::ReplacementAction::Instead(effects),
            )
        };
        game.effect_store
            .replacement_effects
            .add_one_shot_effect(replacement);
        let mut dm = crate::decision::SelectFirstDecisionMaker;
        let event = if zone_event {
            Event::zone_change(
                original,
                from,
                Zone::Graveyard,
                crate::events::cause::EventCause::from_effect(source, alice),
                None,
            )
        } else {
            Event::destroy(original, Some(source))
        };
        let result = process_with_dm_and_additional_effects(&mut game, event, &mut dm, &[])
            .expect("finite replacement fixture evaluates successfully");
        let TraitEventResult::Replaced {
            context,
            effects,
            source,
            controller,
            ..
        } = result
        else {
            panic!("replacement was captured before the original event commits");
        };
        if remove {
            assert!(
                game.move_object_by_effect(original, Zone::Graveyard)
                    .is_some()
            );
            assert!(game.object(original).is_none());
        }
        game.take_pending_trigger_events();
        let mut parent = crate::effects::ExecutionContext::new(source, alice, &mut dm);
        parent.iteration.iterated_player = Some(alice);
        let outcome = crate::effects::replacement::execute_replacement_payload(
            &mut game,
            &mut parent,
            &effects,
            source,
            controller,
            &context,
            None,
        )
        .unwrap();
        assert_eq!(
            game.player(bob).unwrap().life,
            23,
            "captured affected player survives the original object's departure"
        );
        assert_eq!(
            game.player(alice).unwrap().life,
            20,
            "the active player is not a substitute for lost event context"
        );
        assert_eq!(
            outcome
                .events
                .iter()
                .filter_map(|e| e.downcast::<crate::events::LifeGainEvent>())
                .map(|e| (e.player, e.amount))
                .collect::<Vec<_>>(),
            vec![(bob, 3)]
        );
        assert_eq!(parent.iteration.iterated_player, Some(alice));
    }
    #[test]
    fn destroy_payload_keeps_captured_player_after_original_departure() {
        check(false, true);
    }
    #[test]
    fn zone_payload_keeps_captured_player_after_original_departure() {
        check(true, true);
    }
    #[test]
    fn destroy_payload_live_original_control() {
        check(false, false);
    }
    #[test]
    fn zone_payload_live_original_control() {
        check(true, false);
    }
}

#[test]
fn draw_adapter_nested_search_preserves_actual_pending_controller_and_resumes_without_leak() {
    struct Answers {
        selected: ObjectId,
        controller: PlayerId,
        pause: bool,
        pending: bool,
        invalid: bool,
        choices: usize,
    }
    impl crate::decision::DecisionMaker for Answers {
        fn decide_objects(
            &mut self,
            game: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            assert_eq!(game.controlling_player_for(ctx.player), self.controller);
            ctx.candidates
                .iter()
                .filter(|card| card.legal)
                .map(|card| card.id)
                .collect()
        }
        fn decide_options(
            &mut self,
            game: &GameState,
            ctx: &crate::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            assert_eq!(game.controlling_player_for(ctx.player), self.controller);
            assert_eq!(ctx.options.len(), 2);
            self.choices += 1;
            self.pending = self.pause;
            if self.invalid {
                vec![usize::MAX]
            } else {
                vec![
                    ctx.options
                        .iter()
                        .find(|option| option.object_id == Some(self.selected))
                        .unwrap()
                        .index,
                ]
            }
        }
        fn awaiting_choice(&self) -> bool {
            self.pending
        }
    }
    for invalid in [false, true] {
        let mut game = GameState::new(
            vec![
                "Alice".into(),
                "Bob".into(),
                "Charlie".into(),
                "Diana".into(),
            ],
            20,
        );
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let charlie = PlayerId::from_index(2);
        let diana = PlayerId::from_index(3);
        let agent = crate::cards::CardDefinitionBuilder::new(
            CardId::new(),
            "Draw Search Controller Probe",
        )
        .card_types(vec![crate::types::CardType::Creature])
        .with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::control_opponents_while_searching_libraries(),
        ))
        .with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::opponent_search_exile_found_cards(),
        ))
        .build();
        let older = game.create_object_from_definition(&agent, bob, Zone::Battlefield);
        game.create_object_from_definition(&agent, diana, Zone::Battlefield);
        let parent = prefix_card(
            &mut game,
            alice,
            Zone::Battlefield,
            "Draw replacement source",
        );
        let found = prefix_card(
            &mut game,
            charlie,
            Zone::Library,
            "Draw replacement found card",
        );
        let stable = game.object(found).unwrap().stable_id;
        let original_draw_card = prefix_card(&mut game, alice, Zone::Library, "Original draw card");
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(
            ReplacementEffect::with_matcher(
                parent,
                alice,
                crate::events::cards::matchers::WouldDrawCardMatcher::you(),
                ReplacementAction::Instead(vec![Effect::new(
                    crate::effects::SearchLibraryEffect::to_hand(
                        crate::target::ObjectFilter::default(),
                        crate::target::PlayerFilter::Specific(charlie),
                        false,
                    ),
                )]),
            ),
        );
        game.take_pending_trigger_events();
        let before_ids = game.next_object_id_counter();
        let mut paused = Answers {
            selected: older,
            controller: diana,
            pause: true,
            pending: false,
            invalid: false,
            choices: 0,
        };
        let result = process_draw(&mut game, alice, 1, false, &mut paused).unwrap();
        assert!(matches!(result, ResolvedDrawOutcome::Pending));
        assert!(paused.pending);
        assert_eq!(paused.choices, 1);
        assert_eq!(game.object(found).unwrap().zone, Zone::Library);
        assert_eq!(game.object(original_draw_card).unwrap().zone, Zone::Library);
        assert!(game.exile.is_empty());
        assert_eq!(game.next_object_id_counter(), before_ids);
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(shield)
                .is_some()
        );
        assert!(game.take_pending_trigger_events().is_empty());
        assert_eq!(
            game.controlling_player_for(charlie),
            diana,
            "draw adapter rollback must retain the actual nested prompt's controller"
        );
        let mut resumed = Answers {
            selected: older,
            controller: diana,
            pause: false,
            pending: false,
            invalid,
            choices: 0,
        };
        let result = process_draw(&mut game, alice, 1, false, &mut resumed);
        assert!(!resumed.pending);
        assert_eq!(resumed.choices, 1);
        if invalid {
            assert!(result.is_err());
            assert_eq!(game.object(found).unwrap().zone, Zone::Library);
            assert!(game.exile.is_empty());
            assert_eq!(game.next_object_id_counter(), before_ids);
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(shield)
                    .is_some()
            );
            assert!(game.take_pending_trigger_events().is_empty());
        } else {
            assert!(matches!(
                result.unwrap(),
                ResolvedDrawOutcome::Replaced { .. }
            ));
            let arrival = game.find_object_by_stable_id(stable).unwrap();
            assert_eq!(game.object(arrival).unwrap().zone, Zone::Exile);
            for player in [alice, bob, charlie, diana] {
                assert_eq!(
                    game.effect_store.grant_registry.card_can_play_from_zone(
                        &game,
                        arrival,
                        Zone::Exile,
                        player
                    ),
                    player == bob
                );
            }
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(shield)
                    .is_none()
            );
        }
        assert_eq!(game.object(original_draw_card).unwrap().zone, Zone::Library);
        assert_eq!(game.controlling_player_for(charlie), charlie);
    }
}

mod counter_owner_nested_prompt_contract_tests {
    use super::*;
    struct Answers {
        selected: ObjectId,
        controller: PlayerId,
        pause: bool,
        pending: bool,
        invalid: bool,
        choices: usize,
    }
    impl crate::decision::DecisionMaker for Answers {
        fn decide_objects(
            &mut self,
            game: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            assert_eq!(game.controlling_player_for(ctx.player), self.controller);
            ctx.candidates
                .iter()
                .filter(|card| card.legal)
                .map(|card| card.id)
                .collect()
        }
        fn decide_options(
            &mut self,
            game: &GameState,
            ctx: &crate::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            assert_eq!(game.controlling_player_for(ctx.player), self.controller);
            assert_eq!(ctx.options.len(), 2);
            self.choices += 1;
            self.pending = self.pause;
            if self.invalid {
                vec![usize::MAX]
            } else {
                vec![
                    ctx.options
                        .iter()
                        .find(|option| option.object_id == Some(self.selected))
                        .unwrap()
                        .index,
                ]
            }
        }
        fn awaiting_choice(&self) -> bool {
            self.pending
        }
    }
    fn execute(
        game: &mut GameState,
        source: ObjectId,
        object: bool,
        public: bool,
        dm: &mut dyn crate::decision::DecisionMaker,
    ) -> Result<crate::effect::EffectOutcome, crate::effects::ExecutionError> {
        let alice = PlayerId::from_index(0);
        let cause = crate::events::cause::EventCause::from_effect(source, alice);
        let mut ctx =
            crate::effects::ExecutionContext::new(source, alice, dm).with_cause(cause.clone());
        if public {
            use crate::effects::EffectExecutor;
            return if object {
                crate::effects::PutCountersEffect::new(
                    crate::object::CounterType::Charge,
                    1,
                    crate::target::ChooseSpec::SpecificObject(source),
                )
                .execute(game, &mut ctx)
            } else {
                crate::effects::PlayerCountersEffect::new(
                    crate::object::CounterType::Energy,
                    1,
                    crate::target::PlayerFilter::Specific(alice),
                )
                .execute(game, &mut ctx)
            };
        }
        let event = if object {
            Event::put_counters(source, crate::object::CounterType::Charge, 1, cause)
        } else {
            Event::put_player_counters(alice, crate::object::CounterType::Energy, 1, cause)
        };
        if object {
            crate::effects::counters::execute_object_counter_placement(game, &mut ctx, event)
        } else {
            crate::effects::counters::execute_player_counter_placement(game, &mut ctx, event)
        }
    }
    fn check(object: bool, instead: bool, public: bool) {
        for invalid in [false, true] {
            let mut game = GameState::new(
                vec![
                    "Alice".into(),
                    "Bob".into(),
                    "Charlie".into(),
                    "Diana".into(),
                ],
                20,
            );
            let alice = PlayerId::from_index(0);
            let bob = PlayerId::from_index(1);
            let charlie = PlayerId::from_index(2);
            let diana = PlayerId::from_index(3);
            let agent = crate::cards::CardDefinitionBuilder::new(
                CardId::new(),
                "Counter Search Controller Probe",
            )
            .card_types(vec![crate::types::CardType::Creature])
            .with_ability(crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::control_opponents_while_searching_libraries(
                ),
            ))
            .with_ability(crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::opponent_search_exile_found_cards(),
            ))
            .build();
            let older = game.create_object_from_definition(&agent, bob, Zone::Battlefield);
            game.create_object_from_definition(&agent, diana, Zone::Battlefield);
            let parent = prefix_card(
                &mut game,
                alice,
                Zone::Battlefield,
                "Counter replacement source",
            );
            let found = prefix_card(
                &mut game,
                charlie,
                Zone::Library,
                "Counter replacement found card",
            );
            let stable = game.object(found).unwrap().stable_id;
            let payload = vec![Effect::new(crate::effects::SearchLibraryEffect::to_hand(
                crate::target::ObjectFilter::default(),
                crate::target::PlayerFilter::Specific(charlie),
                false,
            ))];
            let action = if instead {
                ReplacementAction::Instead(payload)
            } else {
                ReplacementAction::Additionally(payload)
            };
            let mut replacement = if object {
                ReplacementEffect::with_matcher(
                    parent,
                    alice,
                    crate::events::counters::matchers::WouldPutCountersMatcher::new(
                        crate::target::ObjectFilter::specific(parent),
                        Some(crate::object::CounterType::Charge),
                    ),
                    action.clone(),
                )
            } else {
                crate::static_abilities::StaticAbility::double_player_counters_replacement(
                    crate::target::PlayerFilter::Specific(alice),
                    Some(crate::object::CounterType::Energy),
                    "Counter proposal".into(),
                )
                .generate_replacement_effect(parent, alice)
                .unwrap()
            };
            replacement.replacement = action;
            let shield = game
                .effect_store
                .replacement_effects
                .add_one_shot_effect(replacement);
            game.take_pending_trigger_events();
            let before_ids = game.next_object_id_counter();
            let objects = game.objects_in_deterministic_order().len();
            let mut paused = Answers {
                selected: older,
                controller: diana,
                pause: true,
                pending: false,
                invalid: false,
                choices: 0,
            };
            let outcome = execute(&mut game, parent, object, public, &mut paused).unwrap();
            assert!(paused.pending);
            assert_eq!(paused.choices, 1);
            assert!(outcome.events.is_empty());
            assert_eq!(game.object(found).unwrap().zone, Zone::Library);
            assert!(game.exile.is_empty());
            assert_eq!(game.next_object_id_counter(), before_ids);
            assert_eq!(game.objects_in_deterministic_order().len(), objects);
            assert_eq!(
                game.counter_count(parent, crate::object::CounterType::Charge),
                0
            );
            assert_eq!(
                game.player(alice)
                    .unwrap()
                    .counter_count(crate::object::CounterType::Energy),
                0
            );
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(shield)
                    .is_some()
            );
            assert!(game.take_pending_trigger_events().is_empty());
            assert_eq!(
                game.controlling_player_for(charlie),
                diana,
                "counter owner rollback must retain the actual nested prompt controller"
            );
            let mut resumed = Answers {
                selected: older,
                controller: diana,
                pause: false,
                pending: false,
                invalid,
                choices: 0,
            };
            let result = execute(&mut game, parent, object, public, &mut resumed);
            assert!(!resumed.pending);
            assert_eq!(resumed.choices, 1);
            if invalid {
                assert!(result.is_err());
                assert_eq!(game.object(found).unwrap().zone, Zone::Library);
                assert!(game.exile.is_empty());
                assert_eq!(game.next_object_id_counter(), before_ids);
                assert_eq!(
                    game.counter_count(parent, crate::object::CounterType::Charge),
                    0
                );
                assert_eq!(
                    game.player(alice)
                        .unwrap()
                        .counter_count(crate::object::CounterType::Energy),
                    0
                );
                assert!(
                    game.effect_store
                        .replacement_effects
                        .get_effect(shield)
                        .is_some()
                );
                assert!(game.take_pending_trigger_events().is_empty());
            } else {
                let outcome = result.unwrap();
                assert_eq!(outcome.count_or_zero(), i64::from(!instead));
                let arrival = game.find_object_by_stable_id(stable).unwrap();
                assert_eq!(game.object(arrival).unwrap().zone, Zone::Exile);
                for player in [alice, bob, charlie, diana] {
                    assert_eq!(
                        game.effect_store.grant_registry.card_can_play_from_zone(
                            &game,
                            arrival,
                            Zone::Exile,
                            player
                        ),
                        player == bob
                    );
                }
                assert_eq!(
                    game.counter_count(parent, crate::object::CounterType::Charge),
                    u32::from(object && !instead)
                );
                assert_eq!(
                    game.player(alice)
                        .unwrap()
                        .counter_count(crate::object::CounterType::Energy),
                    u32::from(!object && !instead)
                );
                assert!(
                    game.effect_store
                        .replacement_effects
                        .get_effect(shield)
                        .is_none()
                );
            }
            assert_eq!(game.controlling_player_for(charlie), charlie);
        }
    }
    #[test]
    fn object_addition_retains_nested_prompt_and_replays() {
        check(true, false, false);
    }
    #[test]
    fn object_instead_retains_nested_prompt_and_replays() {
        check(true, true, false);
    }
    #[test]
    fn player_addition_retains_nested_prompt_and_replays() {
        check(false, false, false);
    }
    #[test]
    fn player_instead_retains_nested_prompt_and_replays() {
        check(false, true, false);
    }
    #[test]
    fn public_object_addition_retains_nested_prompt_and_replays() {
        check(true, false, true);
    }
    #[test]
    fn public_object_instead_retains_nested_prompt_and_replays() {
        check(true, true, true);
    }
    #[test]
    fn public_player_addition_retains_nested_prompt_and_replays() {
        check(false, false, true);
    }
    #[test]
    fn public_player_instead_retains_nested_prompt_and_replays() {
        check(false, true, true);
    }
}

mod remaining_counter_owner_nested_prompt_contract_tests {
    use super::*;
    struct Answers {
        selected: ObjectId,
        controller: PlayerId,
        pause: bool,
        pending: bool,
        invalid: bool,
        choices: usize,
    }
    impl crate::decision::DecisionMaker for Answers {
        fn decide_objects(
            &mut self,
            game: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            assert_eq!(game.controlling_player_for(ctx.player), self.controller);
            ctx.candidates
                .iter()
                .filter(|card| card.legal)
                .map(|card| card.id)
                .collect()
        }
        fn decide_options(
            &mut self,
            game: &GameState,
            ctx: &crate::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            assert_eq!(game.controlling_player_for(ctx.player), self.controller);
            assert_eq!(ctx.options.len(), 2);
            self.choices += 1;
            self.pending = self.pause;
            if self.invalid {
                vec![usize::MAX]
            } else {
                vec![
                    ctx.options
                        .iter()
                        .find(|option| option.object_id == Some(self.selected))
                        .unwrap()
                        .index,
                ]
            }
        }
        fn decide_proliferate(
            &mut self,
            game: &GameState,
            ctx: &crate::decisions::context::ProliferateContext,
        ) -> crate::decisions::specs::ProliferateResponse {
            assert_eq!(ctx.player, PlayerId::from_index(0));
            assert_eq!(game.controlling_player_for(ctx.player), ctx.player);
            crate::decisions::specs::ProliferateResponse {
                permanents: ctx.eligible_permanents.iter().map(|(id, _)| *id).collect(),
                players: ctx.eligible_players.iter().map(|(id, _)| *id).collect(),
            }
        }
        fn awaiting_choice(&self) -> bool {
            self.pending
        }
    }

    fn execute(
        game: &mut GameState,
        parent: ObjectId,
        donor: Option<ObjectId>,
        mode: usize,
        dm: &mut Answers,
    ) -> Result<crate::effect::EffectOutcome, crate::effects::ExecutionError> {
        use crate::effects::EffectExecutor;
        let alice = PlayerId::from_index(0);
        let mut ctx = crate::effects::ExecutionContext::new(parent, alice, dm)
            .with_cause(crate::events::cause::EventCause::from_effect(parent, alice));
        match mode {
            0 => crate::effects::DoubleCountersEffect::new(
                Some(crate::object::CounterType::Charge),
                crate::target::ChooseSpec::Source,
            )
            .execute(game, &mut ctx),
            1 => crate::effects::DoubleCountersEffect::new(
                Some(crate::object::CounterType::Energy),
                crate::target::ChooseSpec::SourceController,
            )
            .execute(game, &mut ctx),
            2 => crate::effects::MoveCountersEffect::new(
                crate::object::CounterType::Charge,
                1,
                crate::target::ChooseSpec::SpecificObject(donor.unwrap()),
                crate::target::ChooseSpec::Source,
            )
            .execute(game, &mut ctx),
            3 | 4 => crate::effects::ProliferateEffect::new(1).execute(game, &mut ctx),
            _ => unreachable!(),
        }
    }
    fn check(mode: usize, instead: bool) {
        for invalid in [false, true] {
            let object = matches!(mode, 0 | 2 | 3);
            let mut game = GameState::new(
                vec![
                    "Alice".into(),
                    "Bob".into(),
                    "Charlie".into(),
                    "Diana".into(),
                ],
                20,
            );
            let alice = PlayerId::from_index(0);
            let bob = PlayerId::from_index(1);
            let charlie = PlayerId::from_index(2);
            let diana = PlayerId::from_index(3);
            let agent = crate::cards::CardDefinitionBuilder::new(
                CardId::new(),
                "Outer counter Search Controller Probe",
            )
            .card_types(vec![crate::types::CardType::Creature])
            .with_ability(crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::control_opponents_while_searching_libraries(
                ),
            ))
            .with_ability(crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::opponent_search_exile_found_cards(),
            ))
            .build();
            let older = game.create_object_from_definition(&agent, bob, Zone::Battlefield);
            game.create_object_from_definition(&agent, diana, Zone::Battlefield);
            let parent = prefix_card(
                &mut game,
                alice,
                Zone::Battlefield,
                "Outer counter replacement source",
            );
            let found = prefix_card(
                &mut game,
                charlie,
                Zone::Library,
                "Outer counter replacement found card",
            );
            let stable = game.object(found).unwrap().stable_id;
            let donor = if mode == 2 {
                let id = prefix_card(&mut game, alice, Zone::Battlefield, "Counter donor");
                game.object_mut(id)
                    .unwrap()
                    .counters
                    .insert(crate::object::CounterType::Charge, 1);
                Some(id)
            } else {
                None
            };
            let initial = if mode == 2 { 0 } else { 1 };
            if object {
                if initial > 0 {
                    game.object_mut(parent)
                        .unwrap()
                        .counters
                        .insert(crate::object::CounterType::Charge, initial);
                }
            } else {
                game.player_mut(alice).unwrap().energy_counters = initial;
            }
            let payload = vec![Effect::new(crate::effects::SearchLibraryEffect::to_hand(
                crate::target::ObjectFilter::default(),
                crate::target::PlayerFilter::Specific(charlie),
                false,
            ))];
            let action = if instead {
                ReplacementAction::Instead(payload)
            } else {
                ReplacementAction::Additionally(payload)
            };
            let mut replacement = if object {
                ReplacementEffect::with_matcher(
                    parent,
                    alice,
                    crate::events::counters::matchers::WouldPutCountersMatcher::new(
                        crate::target::ObjectFilter::specific(parent),
                        Some(crate::object::CounterType::Charge),
                    ),
                    action.clone(),
                )
            } else {
                crate::static_abilities::StaticAbility::double_player_counters_replacement(
                    crate::target::PlayerFilter::Specific(alice),
                    Some(crate::object::CounterType::Energy),
                    "Outer counter proposal".into(),
                )
                .generate_replacement_effect(parent, alice)
                .unwrap()
            };
            replacement.replacement = action;
            let shield = game
                .effect_store
                .replacement_effects
                .add_one_shot_effect(replacement);
            game.take_pending_trigger_events();
            let ids = game.next_object_id_counter();
            let objects = game.objects_in_deterministic_order().len();
            let mut paused = Answers {
                selected: older,
                controller: diana,
                pause: true,
                pending: false,
                invalid: false,
                choices: 0,
            };
            let outcome = execute(&mut game, parent, donor, mode, &mut paused).unwrap();
            assert!(paused.pending);
            assert_eq!(paused.choices, 1);
            assert!(outcome.events.is_empty());
            assert_eq!(game.object(found).unwrap().zone, Zone::Library);
            assert!(game.exile.is_empty());
            assert_eq!(game.next_object_id_counter(), ids);
            assert_eq!(game.objects_in_deterministic_order().len(), objects);
            assert_eq!(
                game.counter_count(parent, crate::object::CounterType::Charge),
                if object { initial } else { 0 }
            );
            assert_eq!(
                game.player(alice)
                    .unwrap()
                    .counter_count(crate::object::CounterType::Energy),
                if object { 0 } else { initial }
            );
            if let Some(donor) = donor {
                assert_eq!(
                    game.counter_count(donor, crate::object::CounterType::Charge),
                    1
                );
            }
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(shield)
                    .is_some()
            );
            assert!(game.take_pending_trigger_events().is_empty());
            assert_eq!(
                game.controlling_player_for(charlie),
                diana,
                "outer counter owner rollback must retain the actual nested prompt controller"
            );
            let mut resumed = Answers {
                selected: older,
                controller: diana,
                pause: false,
                pending: false,
                invalid,
                choices: 0,
            };
            let result = execute(&mut game, parent, donor, mode, &mut resumed);
            assert!(!resumed.pending);
            assert_eq!(resumed.choices, 1);
            if invalid {
                assert!(result.is_err());
                assert_eq!(game.object(found).unwrap().zone, Zone::Library);
                assert!(game.exile.is_empty());
                assert_eq!(game.next_object_id_counter(), ids);
                assert_eq!(
                    game.counter_count(parent, crate::object::CounterType::Charge),
                    if object { initial } else { 0 }
                );
                assert_eq!(
                    game.player(alice)
                        .unwrap()
                        .counter_count(crate::object::CounterType::Energy),
                    if object { 0 } else { initial }
                );
                if let Some(donor) = donor {
                    assert_eq!(
                        game.counter_count(donor, crate::object::CounterType::Charge),
                        1
                    );
                }
                assert!(
                    game.effect_store
                        .replacement_effects
                        .get_effect(shield)
                        .is_some()
                );
                assert!(game.take_pending_trigger_events().is_empty());
            } else {
                let _outcome = result.unwrap();
                let arrival = game.find_object_by_stable_id(stable).unwrap();
                assert_eq!(game.object(arrival).unwrap().zone, Zone::Exile);
                for player in [alice, bob, charlie, diana] {
                    assert_eq!(
                        game.effect_store.grant_registry.card_can_play_from_zone(
                            &game,
                            arrival,
                            Zone::Exile,
                            player
                        ),
                        player == bob
                    );
                }
                assert_eq!(
                    game.counter_count(parent, crate::object::CounterType::Charge),
                    if object {
                        initial + u32::from(!instead)
                    } else {
                        0
                    }
                );
                assert_eq!(
                    game.player(alice)
                        .unwrap()
                        .counter_count(crate::object::CounterType::Energy),
                    if object {
                        0
                    } else {
                        initial + u32::from(!instead)
                    }
                );
                if let Some(donor) = donor {
                    assert_eq!(
                        game.counter_count(donor, crate::object::CounterType::Charge),
                        0
                    );
                }
                assert!(
                    game.effect_store
                        .replacement_effects
                        .get_effect(shield)
                        .is_none()
                );
            }
            assert_eq!(game.controlling_player_for(charlie), charlie);
        }
    }
    #[test]
    fn double_object_addition_retains_nested_prompt_and_replays() {
        check(0, false);
    }
    #[test]
    fn double_object_instead_retains_nested_prompt_and_replays() {
        check(0, true);
    }
    #[test]
    fn double_player_addition_retains_nested_prompt_and_replays() {
        check(1, false);
    }
    #[test]
    fn double_player_instead_retains_nested_prompt_and_replays() {
        check(1, true);
    }
    #[test]
    fn move_addition_retains_nested_prompt_and_replays() {
        check(2, false);
    }
    #[test]
    fn move_instead_retains_nested_prompt_and_replays() {
        check(2, true);
    }
    #[test]
    fn proliferate_object_addition_retains_nested_prompt_and_replays() {
        check(3, false);
    }
    #[test]
    fn proliferate_object_instead_retains_nested_prompt_and_replays() {
        check(3, true);
    }
    #[test]
    fn proliferate_player_addition_retains_nested_prompt_and_replays() {
        check(4, false);
    }
    #[test]
    fn proliferate_player_instead_retains_nested_prompt_and_replays() {
        check(4, true);
    }
}

mod public_damage_error_contract_tests {
    use super::*;
    fn check(simultaneous: bool) {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = prefix_card(&mut game, alice, Zone::Battlefield, "Damage error source");
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(
            ReplacementEffect::with_matcher(
                source,
                alice,
                crate::events::damage::matchers::DamageToPlayerMatcher::to_any_player(),
                ReplacementAction::Instead(vec![Effect::gain_life(3), Effect::lose_life(Value::X)]),
            ),
        );
        game.take_pending_trigger_events();
        let ids = game.next_object_id_counter();
        let objects = game.objects_in_deterministic_order().len();
        let cause = crate::events::cause::EventCause::from_effect(source, alice);
        let completed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            if simultaneous {
                let events = vec![SimultaneousDamageEvent {
                    source,
                    target: DamageTarget::Player(bob),
                    amount: 3,
                    is_combat: false,
                    unpreventable: false,
                    cause: cause.clone(),
                    source_snapshot: None,
                }];
                let failure =
                    process_simultaneous_damage_assignments_with_event(&mut game, &events)
                        .expect_err("invalid replacement payload must return an error");
                assert_eq!(failure.source, source);
                assert!(
                    matches!(failure.error, crate::effects::ExecutionError::UnresolvableValue(ref message) if message == "X value not set")
                );
            } else {
                let failure = process_damage_assignments_with_event(
                    &mut game,
                    source,
                    DamageTarget::Player(bob),
                    3,
                    false,
                    cause,
                )
                .expect_err("invalid replacement payload must return an error");
                assert!(
                    matches!(failure, crate::effects::ExecutionError::UnresolvableValue(ref message) if message == "X value not set")
                );
            }
        }));
        assert!(
            completed.is_ok(),
            "public damage processors must propagate replacement payload errors without panicking"
        );
        assert_eq!(game.player(alice).unwrap().life, 20);
        assert_eq!(game.player(bob).unwrap().life, 20);
        assert_eq!(game.next_object_id_counter(), ids);
        assert_eq!(game.objects_in_deterministic_order().len(), objects);
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(shield)
                .is_some()
        );
        assert!(game.take_pending_trigger_events().is_empty());
    }
    #[test]
    fn single_public_damage_error_does_not_panic() {
        check(false);
    }
    #[test]
    fn simultaneous_public_damage_error_does_not_panic() {
        check(true);
    }
}

mod live_combat_nested_prompt_contract_tests {
    use super::*;
    struct Answers {
        selected: ObjectId,
        controller: PlayerId,
        pause: bool,
        pending: bool,
        invalid: bool,
        choices: usize,
    }
    impl crate::decision::DecisionMaker for Answers {
        fn decide_objects(
            &mut self,
            game: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            assert_eq!(game.controlling_player_for(ctx.player), self.controller);
            ctx.candidates
                .iter()
                .filter(|card| card.legal)
                .map(|card| card.id)
                .collect()
        }
        fn decide_options(
            &mut self,
            game: &GameState,
            ctx: &crate::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            assert_eq!(game.controlling_player_for(ctx.player), self.controller);
            assert_eq!(ctx.options.len(), 2);
            self.choices += 1;
            self.pending = self.pause;
            if self.invalid {
                vec![usize::MAX]
            } else {
                vec![
                    ctx.options
                        .iter()
                        .find(|option| option.object_id == Some(self.selected))
                        .unwrap()
                        .index,
                ]
            }
        }
        fn awaiting_choice(&self) -> bool {
            self.pending
        }
    }

    fn execute(
        game: &mut GameState,
        combat: &crate::combat_state::CombatState,
        dm: &mut Answers,
        processor: usize,
    ) -> Result<
        Vec<crate::game_loop::CombatDamageEvent>,
        crate::game_loop::CombatDamageAssignmentError,
    > {
        if processor == 0 {
            return crate::game_loop::try_execute_combat_damage_step_with_dm(
                game, combat, false, dm,
            );
        }
        let source = combat.attackers[0].creature;
        let bob = PlayerId::from_index(1);
        let cause =
            crate::events::cause::EventCause::from_combat_damage(source, PlayerId::from_index(0));
        let results = if processor == 1 {
            vec![
                process_damage_assignments_with_event_with_source_snapshot_opts_with_dm(
                    game,
                    source,
                    DamageTarget::Player(bob),
                    2,
                    true,
                    false,
                    cause,
                    None,
                    dm,
                )
                .map_err(|error| DamageProcessingError { source, error })?,
            ]
        } else {
            process_simultaneous_damage_assignments_with_event_with_dm(
                game,
                &[SimultaneousDamageEvent {
                    source,
                    target: DamageTarget::Player(bob),
                    amount: 2,
                    is_combat: true,
                    unpreventable: false,
                    cause,
                    source_snapshot: None,
                }],
                dm,
            )?
        };
        if !dm.awaiting_choice() {
            assert_eq!(results.len(), 1);
            let result = &results[0];
            assert!(result.replacement_prevented);
            assert!(result.assignments.is_empty());
            assert!(result.programs.is_empty());
            assert!(
                result.payload_outcome.is_some(),
                "retain the completed replacement payload observation"
            );
        }
        // Core cases use Instead: no original combat damage event occurred.
        Ok(Vec::new())
    }
    fn check(instead: bool, general: bool, processor: usize) {
        for invalid in [false, true] {
            let mut game = GameState::new(
                vec![
                    "Alice".into(),
                    "Bob".into(),
                    "Charlie".into(),
                    "Diana".into(),
                ],
                20,
            );
            let alice = PlayerId::from_index(0);
            let bob = PlayerId::from_index(1);
            let charlie = PlayerId::from_index(2);
            let diana = PlayerId::from_index(3);
            let agent = crate::cards::CardDefinitionBuilder::new(
                CardId::new(),
                "Combat Search Controller Probe",
            )
            .card_types(vec![crate::types::CardType::Creature])
            .with_ability(crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::control_opponents_while_searching_libraries(
                ),
            ))
            .with_ability(crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::opponent_search_exile_found_cards(),
            ))
            .build();
            let older = game.create_object_from_definition(&agent, bob, Zone::Battlefield);
            game.create_object_from_definition(&agent, diana, Zone::Battlefield);
            let source = prefix_card(
                &mut game,
                alice,
                Zone::Battlefield,
                "Combat replacement source",
            );
            let found = prefix_card(
                &mut game,
                charlie,
                Zone::Library,
                "Combat replacement found card",
            );
            let stable = game.object(found).unwrap().stable_id;
            let payload = vec![Effect::new(crate::effects::SearchLibraryEffect::to_hand(
                crate::target::ObjectFilter::default(),
                crate::target::PlayerFilter::Specific(charlie),
                false,
            ))];
            let action = if instead {
                ReplacementAction::Instead(payload)
            } else {
                ReplacementAction::Additionally(payload)
            };
            let shield = game.effect_store.replacement_effects.add_one_shot_effect(
                ReplacementEffect::with_matcher(
                    source,
                    alice,
                    crate::events::damage::matchers::DamageToPlayerMatcher::to_any_player(),
                    action,
                ),
            );
            let mut combat = crate::combat_state::CombatState {
                attackers: vec![crate::combat_state::AttackerInfo {
                    creature: source,
                    target: crate::combat_state::AttackTarget::Player(bob),
                }],
                ..crate::combat_state::CombatState::default()
            };
            if general {
                let card = crate::card::CardBuilder::new(CardId::new(), "Zero power combatant")
                    .card_types(vec![crate::types::CardType::Creature])
                    .power_toughness(crate::card::PowerToughness::fixed(0, 4))
                    .build();
                let attacker = game.create_object_from_card(&card, alice, Zone::Battlefield);
                let blocker = game.create_object_from_card(&card, bob, Zone::Battlefield);
                combat.attackers.push(crate::combat_state::AttackerInfo {
                    creature: attacker,
                    target: crate::combat_state::AttackTarget::Player(bob),
                });
                combat.blockers.insert(attacker, vec![blocker]);
            }
            game.take_pending_trigger_events();
            let ids = game.next_object_id_counter();
            let objects = game.objects_in_deterministic_order().len();
            let mut paused = Answers {
                selected: older,
                controller: diana,
                pause: true,
                pending: false,
                invalid: false,
                choices: 0,
            };
            let events = execute(&mut game, &combat, &mut paused, processor).unwrap();
            assert!(paused.pending);
            assert_eq!(paused.choices, 1);
            assert!(events.is_empty());
            for player in [alice, bob, charlie, diana] {
                assert_eq!(game.player(player).unwrap().life, 20);
            }
            assert_eq!(game.object(found).unwrap().zone, Zone::Library);
            assert!(game.exile.is_empty());
            assert_eq!(game.next_object_id_counter(), ids);
            assert_eq!(game.objects_in_deterministic_order().len(), objects);
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(shield)
                    .is_some()
            );
            assert!(game.take_pending_trigger_events().is_empty());
            assert_eq!(game.effect_store.trigger_matching_holds, 0);
            assert!(
                !game
                    .effect_store
                    .prevention_effects
                    .follow_ups_are_deferred()
            );
            assert_eq!(
                game.controlling_player_for(charlie),
                diana,
                "live combat owner rollback must retain the actual nested prompt controller"
            );
            let mut resumed = Answers {
                selected: older,
                controller: diana,
                pause: false,
                pending: false,
                invalid,
                choices: 0,
            };
            let result = execute(&mut game, &combat, &mut resumed, processor);
            assert!(!resumed.pending);
            assert_eq!(resumed.choices, 1);
            if invalid {
                let error = result
                    .expect_err("invalid nested answer must propagate a combat execution error");
                assert!(matches!(
                    error.kind,
                    crate::game_loop::CombatDamageAssignmentErrorKind::Execution(_)
                ));
                for player in [alice, bob, charlie, diana] {
                    assert_eq!(game.player(player).unwrap().life, 20);
                }
                assert_eq!(game.object(found).unwrap().zone, Zone::Library);
                assert!(game.exile.is_empty());
                assert_eq!(game.next_object_id_counter(), ids);
                assert!(
                    game.effect_store
                        .replacement_effects
                        .get_effect(shield)
                        .is_some()
                );
                assert!(game.take_pending_trigger_events().is_empty());
            } else {
                let events = result.unwrap();
                let damage: u32 = events
                    .iter()
                    .filter(|event| event.source == source)
                    .map(|event| event.amount)
                    .sum();
                assert_eq!(damage, if instead { 0 } else { 2 });
                assert_eq!(
                    game.player(bob).unwrap().life,
                    if instead { 20 } else { 18 }
                );
                let arrival = game.find_object_by_stable_id(stable).unwrap();
                assert_eq!(game.object(arrival).unwrap().zone, Zone::Exile);
                for player in [alice, bob, charlie, diana] {
                    assert_eq!(
                        game.effect_store.grant_registry.card_can_play_from_zone(
                            &game,
                            arrival,
                            Zone::Exile,
                            player
                        ),
                        player == bob
                    );
                }
                assert!(
                    game.effect_store
                        .replacement_effects
                        .get_effect(shield)
                        .is_none()
                );
            }
            assert_eq!(game.controlling_player_for(charlie), charlie);
            assert_eq!(game.effect_store.trigger_matching_holds, 0);
            assert!(
                !game
                    .effect_store
                    .prevention_effects
                    .follow_ups_are_deferred()
            );
        }
    }
    #[test]
    fn unblocked_addition_retains_nested_prompt_and_replays() {
        check(false, false, 0);
    }
    #[test]
    fn unblocked_instead_retains_nested_prompt_and_replays() {
        check(true, false, 0);
    }
    #[test]
    fn general_addition_retains_nested_prompt_and_replays() {
        check(false, true, 0);
    }
    #[test]
    fn general_instead_retains_nested_prompt_and_replays() {
        check(true, true, 0);
    }
    #[test]
    fn single_processor_instead_retains_nested_prompt_and_replays() {
        check(true, false, 1);
    }
    #[test]
    fn simultaneous_processor_instead_retains_nested_prompt_and_replays() {
        check(true, false, 2);
    }
}

mod discard_owner_nested_prompt_contract_tests {
    use super::*;
    struct Answers {
        selected: ObjectId,
        controller: PlayerId,
        pause: bool,
        pending: bool,
        invalid: bool,
        choices: usize,
    }
    impl crate::decision::DecisionMaker for Answers {
        fn decide_objects(
            &mut self,
            game: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            assert_eq!(game.controlling_player_for(ctx.player), self.controller);
            ctx.candidates
                .iter()
                .filter(|card| card.legal)
                .map(|card| card.id)
                .collect()
        }
        fn decide_options(
            &mut self,
            game: &GameState,
            ctx: &crate::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            assert_eq!(game.controlling_player_for(ctx.player), self.controller);
            assert_eq!(ctx.options.len(), 2);
            self.choices += 1;
            self.pending = self.pause;
            if self.invalid {
                vec![usize::MAX]
            } else {
                vec![
                    ctx.options
                        .iter()
                        .find(|option| option.object_id == Some(self.selected))
                        .unwrap()
                        .index,
                ]
            }
        }
        fn awaiting_choice(&self) -> bool {
            self.pending
        }
    }

    fn execute(
        game: &mut GameState,
        source: ObjectId,
        dm: &mut Answers,
    ) -> Result<Option<DiscardResult>, crate::effects::ExecutionError> {
        execute_discard(
            game,
            source,
            PlayerId::from_index(0),
            crate::events::cause::EventCause::effect(),
            false,
            crate::provenance::ProvNodeId::default(),
            dm,
        )
    }
    fn check(instead: bool) {
        for invalid in [false, true] {
            let mut game = GameState::new(
                vec![
                    "Alice".into(),
                    "Bob".into(),
                    "Charlie".into(),
                    "Diana".into(),
                ],
                20,
            );
            let alice = PlayerId::from_index(0);
            let bob = PlayerId::from_index(1);
            let charlie = PlayerId::from_index(2);
            let diana = PlayerId::from_index(3);
            let agent = crate::cards::CardDefinitionBuilder::new(
                CardId::new(),
                "Combat Search Controller Probe",
            )
            .card_types(vec![crate::types::CardType::Creature])
            .with_ability(crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::control_opponents_while_searching_libraries(
                ),
            ))
            .with_ability(crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::opponent_search_exile_found_cards(),
            ))
            .build();
            let older = game.create_object_from_definition(&agent, bob, Zone::Battlefield);
            game.create_object_from_definition(&agent, diana, Zone::Battlefield);
            let source = prefix_card(&mut game, alice, Zone::Hand, "Discard replacement source");
            let found = prefix_card(
                &mut game,
                charlie,
                Zone::Library,
                "Combat replacement found card",
            );
            let stable = game.object(found).unwrap().stable_id;
            let payload = vec![Effect::new(crate::effects::SearchLibraryEffect::to_hand(
                crate::target::ObjectFilter::default(),
                crate::target::PlayerFilter::Specific(charlie),
                false,
            ))];
            let action = if instead {
                ReplacementAction::Instead(payload)
            } else {
                ReplacementAction::Additionally(payload)
            };
            let shield = game.effect_store.replacement_effects.add_one_shot_effect(
                ReplacementEffect::with_matcher(
                    source,
                    alice,
                    crate::events::cards::matchers::WouldDiscardMatcher::any_player(),
                    action,
                ),
            );
            game.take_pending_trigger_events();
            let ids = game.next_object_id_counter();
            let objects = game.objects_in_deterministic_order().len();
            let mut paused = Answers {
                selected: older,
                controller: diana,
                pause: true,
                pending: false,
                invalid: false,
                choices: 0,
            };
            let events = execute(&mut game, source, &mut paused).unwrap();
            assert!(paused.pending);
            assert_eq!(paused.choices, 1);
            assert!(events.is_none());
            assert_eq!(game.object(source).unwrap().zone, Zone::Hand);
            for player in [alice, bob, charlie, diana] {
                assert_eq!(game.player(player).unwrap().life, 20);
            }
            assert_eq!(game.object(found).unwrap().zone, Zone::Library);
            assert!(game.exile.is_empty());
            assert_eq!(game.next_object_id_counter(), ids);
            assert_eq!(game.objects_in_deterministic_order().len(), objects);
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(shield)
                    .is_some()
            );
            assert!(game.take_pending_trigger_events().is_empty());
            assert_eq!(game.effect_store.trigger_matching_holds, 0);
            assert!(
                !game
                    .effect_store
                    .prevention_effects
                    .follow_ups_are_deferred()
            );
            assert_eq!(
                game.controlling_player_for(charlie),
                diana,
                "root discard owner rollback must retain the actual nested prompt controller"
            );
            let mut resumed = Answers {
                selected: older,
                controller: diana,
                pause: false,
                pending: false,
                invalid,
                choices: 0,
            };
            let result = execute(&mut game, source, &mut resumed);
            assert!(!resumed.pending);
            assert_eq!(resumed.choices, 1);
            if invalid {
                let error = result
                    .expect_err("invalid nested answer must propagate a discard execution error");
                assert!(matches!(
                    error,
                    crate::effects::ExecutionError::InternalError(_)
                ));
                assert_eq!(game.object(source).unwrap().zone, Zone::Hand);
                for player in [alice, bob, charlie, diana] {
                    assert_eq!(game.player(player).unwrap().life, 20);
                }
                assert_eq!(game.object(found).unwrap().zone, Zone::Library);
                assert!(game.exile.is_empty());
                assert_eq!(game.next_object_id_counter(), ids);
                assert!(
                    game.effect_store
                        .replacement_effects
                        .get_effect(shield)
                        .is_some()
                );
                assert!(game.take_pending_trigger_events().is_empty());
            } else {
                let receipt = result
                    .unwrap()
                    .expect("a completed root discard must report its original outcome");
                assert_eq!(receipt.prevented, instead);
                if instead {
                    assert_eq!(game.object(source).unwrap().zone, Zone::Hand);
                } else {
                    let arrival = receipt
                        .new_id
                        .expect("the original discard must move its card");
                    assert_eq!(game.object(arrival).unwrap().zone, Zone::Graveyard);
                }
                for player in [alice, bob, charlie, diana] {
                    assert_eq!(game.player(player).unwrap().life, 20);
                }
                let arrival = game.find_object_by_stable_id(stable).unwrap();
                assert_eq!(game.object(arrival).unwrap().zone, Zone::Exile);
                for player in [alice, bob, charlie, diana] {
                    assert_eq!(
                        game.effect_store.grant_registry.card_can_play_from_zone(
                            &game,
                            arrival,
                            Zone::Exile,
                            player
                        ),
                        player == bob
                    );
                }
                assert!(
                    game.effect_store
                        .replacement_effects
                        .get_effect(shield)
                        .is_none()
                );
            }
            assert_eq!(game.controlling_player_for(charlie), charlie);
            assert_eq!(game.effect_store.trigger_matching_holds, 0);
            assert!(
                !game
                    .effect_store
                    .prevention_effects
                    .follow_ups_are_deferred()
            );
        }
    }
    #[test]
    fn root_addition_retains_nested_prompt_and_replays() {
        check(false);
    }
    #[test]
    fn root_instead_retains_nested_prompt_and_replays() {
        check(true);
    }
}

mod deferred_cast_cost_error_contract_tests {
    use super::*;
    struct Accept;
    impl crate::decision::DecisionMaker for Accept {
        fn decide_boolean(
            &mut self,
            _: &GameState,
            _: &crate::decisions::context::BooleanContext,
        ) -> bool {
            true
        }
    }
    fn check(madness_owner: bool) {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let cost = crate::cost::TotalCost::from_costs(vec![
            crate::costs::Cost::try_effect(Effect::lose_life(1)).unwrap(),
        ]);
        let definition = crate::cards::CardDefinitionBuilder::new(
            CardId::new(),
            "Deferred replacement cost probe",
        )
        .card_types(vec![crate::types::CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(2, 2))
        .alternative_cast(crate::alternative_cast::AlternativeCastingMethod::Madness {
            total_cost: cost,
        })
        .build();
        let source = game.create_object_from_definition(&definition, alice, Zone::Exile);
        game.set_madness_exiled(source);
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(
            ReplacementEffect::with_matcher(
                source,
                alice,
                crate::events::life::matchers::WouldLoseLifeMatcher::you(),
                ReplacementAction::Instead(vec![Effect::gain_life(3), Effect::gain_life(Value::X)]),
            ),
        );
        game.take_pending_trigger_events();
        let ids = game.next_object_id_counter();
        let objects = game.objects_in_deterministic_order().len();
        let mut dm = Accept;
        let result = if madness_owner {
            let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut dm);
            crate::effects::EffectExecutor::execute(
                &crate::effects::MayCastForMadnessCostEffect::new(),
                &mut game,
                &mut ctx,
            )
            .map(|_| ())
        } else {
            game.authorize_madness_cast(source);
            let result = crate::game_loop::cast_spell_from_resolving_effect(
                &mut game,
                source,
                Zone::Exile,
                alice,
                &crate::alternative_cast::CastingMethod::Alternative(0),
                false,
                None,
                crate::provenance::ProvNodeId::default(),
                &mut dm,
            );
            game.revoke_madness_cast(source);
            match result {
                Err(crate::game_loop::GameLoopError::ExecutionFailed(error)) => Err(error),
                Err(error) => {
                    panic!("a replacement execution failure must remain typed: {error:?}")
                }
                Ok(_) => panic!(
                    "an unresolved replacement value must not become a cancelled or successful cast"
                ),
            }
        };
        assert_eq!(
            result,
            Err(crate::effects::ExecutionError::UnresolvableValue(
                "X value not set".into()
            ))
        );
        assert_eq!(game.player(alice).unwrap().life, 20);
        assert_eq!(game.object(source).unwrap().zone, Zone::Exile);
        assert!(game.is_madness_exiled(source));
        assert!(game.stack.is_empty());
        assert_eq!(game.next_object_id_counter(), ids);
        assert_eq!(game.objects_in_deterministic_order().len(), objects);
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(shield)
                .is_some()
        );
        assert!(game.take_pending_trigger_events().is_empty());
    }
    #[test]
    fn shared_cast_retains_replacement_execution_error_and_rollback() {
        check(false);
    }
    #[test]
    fn madness_owner_retains_replacement_execution_error_and_rollback() {
        check(true);
    }
}

mod destroy_owner_nested_prompt_contract_tests {
    use super::*;
    struct Answers {
        selected: ObjectId,
        controller: PlayerId,
        pause: bool,
        pending: bool,
        invalid: bool,
        choices: usize,
    }
    impl crate::decision::DecisionMaker for Answers {
        fn decide_objects(
            &mut self,
            game: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            assert_eq!(game.controlling_player_for(ctx.player), self.controller);
            ctx.candidates
                .iter()
                .filter(|card| card.legal)
                .map(|card| card.id)
                .collect()
        }
        fn decide_options(
            &mut self,
            game: &GameState,
            ctx: &crate::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            assert_eq!(game.controlling_player_for(ctx.player), self.controller);
            assert_eq!(ctx.options.len(), 2);
            self.choices += 1;
            self.pending = self.pause;
            if self.invalid {
                vec![usize::MAX]
            } else {
                vec![
                    ctx.options
                        .iter()
                        .find(|option| option.object_id == Some(self.selected))
                        .unwrap()
                        .index,
                ]
            }
        }
        fn awaiting_choice(&self) -> bool {
            self.pending
        }
    }

    fn execute(
        game: &mut GameState,
        source: ObjectId,
        dm: &mut Answers,
    ) -> Result<Option<DestroyOutcome>, crate::effects::ExecutionError> {
        process_destroy(game, source, Some(source), dm)
    }
    fn check(instead: bool) {
        for invalid in [false, true] {
            let mut game = GameState::new(
                vec![
                    "Alice".into(),
                    "Bob".into(),
                    "Charlie".into(),
                    "Diana".into(),
                ],
                20,
            );
            let alice = PlayerId::from_index(0);
            let bob = PlayerId::from_index(1);
            let charlie = PlayerId::from_index(2);
            let diana = PlayerId::from_index(3);
            let agent = crate::cards::CardDefinitionBuilder::new(
                CardId::new(),
                "Combat Search Controller Probe",
            )
            .card_types(vec![crate::types::CardType::Creature])
            .with_ability(crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::control_opponents_while_searching_libraries(
                ),
            ))
            .with_ability(crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::opponent_search_exile_found_cards(),
            ))
            .build();
            let older = game.create_object_from_definition(&agent, bob, Zone::Battlefield);
            game.create_object_from_definition(&agent, diana, Zone::Battlefield);
            let source = prefix_card(
                &mut game,
                alice,
                Zone::Battlefield,
                "Destroy replacement source",
            );
            let found = prefix_card(
                &mut game,
                charlie,
                Zone::Library,
                "Combat replacement found card",
            );
            let stable = game.object(found).unwrap().stable_id;
            let payload = vec![Effect::new(crate::effects::SearchLibraryEffect::to_hand(
                crate::target::ObjectFilter::default(),
                crate::target::PlayerFilter::Specific(charlie),
                false,
            ))];
            let action = if instead {
                ReplacementAction::Instead(payload)
            } else {
                ReplacementAction::Additionally(payload)
            };
            let shield = game.effect_store.replacement_effects.add_one_shot_effect(
                ReplacementEffect::with_matcher(
                    source,
                    alice,
                    crate::events::permanents::matchers::WouldBeDestroyedMatcher::new(
                        crate::target::ObjectFilter::specific(source),
                    ),
                    action,
                ),
            );
            game.take_pending_trigger_events();
            let ids = game.next_object_id_counter();
            let objects = game.objects_in_deterministic_order().len();
            let mut paused = Answers {
                selected: older,
                controller: diana,
                pause: true,
                pending: false,
                invalid: false,
                choices: 0,
            };
            let events = execute(&mut game, source, &mut paused).unwrap();
            assert!(paused.pending);
            assert_eq!(paused.choices, 1);
            assert!(events.is_none());
            assert_eq!(game.object(source).unwrap().zone, Zone::Battlefield);
            for player in [alice, bob, charlie, diana] {
                assert_eq!(game.player(player).unwrap().life, 20);
            }
            assert_eq!(game.object(found).unwrap().zone, Zone::Library);
            assert!(game.exile.is_empty());
            assert_eq!(game.next_object_id_counter(), ids);
            assert_eq!(game.objects_in_deterministic_order().len(), objects);
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(shield)
                    .is_some()
            );
            assert!(game.take_pending_trigger_events().is_empty());
            assert_eq!(game.effect_store.trigger_matching_holds, 0);
            assert!(
                !game
                    .effect_store
                    .prevention_effects
                    .follow_ups_are_deferred()
            );
            assert_eq!(
                game.controlling_player_for(charlie),
                diana,
                "root destroy owner rollback must retain the actual nested prompt controller"
            );
            let mut resumed = Answers {
                selected: older,
                controller: diana,
                pause: false,
                pending: false,
                invalid,
                choices: 0,
            };
            let result = execute(&mut game, source, &mut resumed);
            assert!(!resumed.pending);
            assert_eq!(resumed.choices, 1);
            if invalid {
                let error = result
                    .expect_err("invalid nested answer must propagate a destroy execution error");
                assert!(matches!(
                    error,
                    crate::effects::ExecutionError::InternalError(_)
                ));
                assert_eq!(game.object(source).unwrap().zone, Zone::Battlefield);
                for player in [alice, bob, charlie, diana] {
                    assert_eq!(game.player(player).unwrap().life, 20);
                }
                assert_eq!(game.object(found).unwrap().zone, Zone::Library);
                assert!(game.exile.is_empty());
                assert_eq!(game.next_object_id_counter(), ids);
                assert!(
                    game.effect_store
                        .replacement_effects
                        .get_effect(shield)
                        .is_some()
                );
                assert!(game.take_pending_trigger_events().is_empty());
            } else {
                let receipt = result
                    .unwrap()
                    .expect("a completed root destroy must report its original outcome");
                if instead {
                    assert!(matches!(receipt, EventOutcome::Replaced));
                    assert_eq!(game.object(source).unwrap().zone, Zone::Battlefield);
                } else {
                    assert!(matches!(receipt, EventOutcome::Proceed(Zone::Graveyard)));
                    assert!(game.object(source).is_none());
                    assert_eq!(game.player(alice).unwrap().graveyard.len(), 1);
                }
                for player in [alice, bob, charlie, diana] {
                    assert_eq!(game.player(player).unwrap().life, 20);
                }
                let arrival = game.find_object_by_stable_id(stable).unwrap();
                assert_eq!(game.object(arrival).unwrap().zone, Zone::Exile);
                for player in [alice, bob, charlie, diana] {
                    assert_eq!(
                        game.effect_store.grant_registry.card_can_play_from_zone(
                            &game,
                            arrival,
                            Zone::Exile,
                            player
                        ),
                        player == bob
                    );
                }
                assert!(
                    game.effect_store
                        .replacement_effects
                        .get_effect(shield)
                        .is_none()
                );
            }
            assert_eq!(game.controlling_player_for(charlie), charlie);
            assert_eq!(game.effect_store.trigger_matching_holds, 0);
            assert!(
                !game
                    .effect_store
                    .prevention_effects
                    .follow_ups_are_deferred()
            );
        }
    }
    #[test]
    fn root_addition_retains_nested_prompt_and_replays() {
        check(false);
    }
    #[test]
    fn root_instead_retains_nested_prompt_and_replays() {
        check(true);
    }
}

mod destruction_primary_observation_contract_tests {
    use super::*;
    #[test]
    fn regeneration_payload_is_observed_without_counting_regenerated_permanent_as_destroyed() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let artifact =
            crate::card::CardBuilder::new(CardId::new(), "Destruction observation probe")
                .card_types(vec![
                    crate::types::CardType::Artifact,
                    crate::types::CardType::Creature,
                ])
                .power_toughness(crate::card::PowerToughness::fixed(2, 2))
                .build();
        let first = game.create_object_from_card(&artifact, alice, Zone::Battlefield);
        let second = game.create_object_from_card(&artifact, bob, Zone::Battlefield);
        let protected = game.create_object_from_card(&artifact, alice, Zone::Battlefield);
        game.set_current_controller(protected, bob)
            .expect("finite controller fixture must refresh successfully");
        let source = prefix_card(
            &mut game,
            alice,
            Zone::Hand,
            "Destruction instruction source",
        );
        let mut ctx = crate::effects::ExecutionContext::new_default(source, alice);
        crate::effects::execute_effect(
            &mut game,
            &Effect::regenerate(
                crate::target::ChooseSpec::SpecificObject(protected),
                crate::effect::Until::EndOfTurn,
            ),
            &mut ctx,
        )
        .unwrap();
        game.take_pending_trigger_events();
        let outcome = crate::effects::execute_effect(
            &mut game,
            &Effect::new(crate::effects::DestroyEffect::all(
                crate::target::ObjectFilter::artifact(),
            ))
            .tag("destroyed"),
            &mut ctx,
        )
        .unwrap();
        assert_eq!(
            outcome.count_or_zero(),
            2,
            "the original destruction must report two successful departures"
        );
        assert!(game.object(first).is_none());
        assert!(game.object(second).is_none());
        assert_eq!(game.object(protected).unwrap().zone, Zone::Battlefield);
        assert!(game.is_tapped(protected));
        let events = outcome
            .events
            .iter()
            .chain(game.effect_store.pending_trigger_events.iter())
            .collect::<Vec<_>>();
        assert!(
            events.iter().any(|event| event
                .downcast::<crate::events::PermanentTappedEvent>()
                .is_some_and(|event| event.permanent == protected)),
            "the regeneration payload must still publish its actual tap"
        );
        let memories = outcome
            .affected_object_memory()
            .expect("destroyed objects require last-known memory");
        assert!(
            !memories.iter().any(|memory| memory.object_id == protected),
            "replacement payload objects must not contaminate the original instruction's affected memories"
        );
        let tags = ctx
            .get_tagged_all("destroyed")
            .expect("the authored destruction tag must bind its successful originals");
        assert_eq!(tags.len(), 2);
        assert!(!tags.iter().any(|snapshot| snapshot.object_id == protected));
        assert_eq!(
            tags.iter()
                .filter(|snapshot| snapshot.controller == bob)
                .count(),
            1
        );
    }
}

mod destroy_effect_owner_nested_prompt_contract_tests {
    use super::*;
    struct Answers {
        selected: ObjectId,
        controller: PlayerId,
        pause: bool,
        pending: bool,
        invalid: bool,
        choices: usize,
    }
    impl crate::decision::DecisionMaker for Answers {
        fn decide_objects(
            &mut self,
            game: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            assert_eq!(game.controlling_player_for(ctx.player), self.controller);
            ctx.candidates
                .iter()
                .filter(|card| card.legal)
                .map(|card| card.id)
                .collect()
        }
        fn decide_options(
            &mut self,
            game: &GameState,
            ctx: &crate::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            assert_eq!(game.controlling_player_for(ctx.player), self.controller);
            assert_eq!(ctx.options.len(), 2);
            self.choices += 1;
            self.pending = self.pause;
            if self.invalid {
                vec![usize::MAX]
            } else {
                vec![
                    ctx.options
                        .iter()
                        .find(|option| option.object_id == Some(self.selected))
                        .unwrap()
                        .index,
                ]
            }
        }
        fn awaiting_choice(&self) -> bool {
            self.pending
        }
    }

    fn execute(
        game: &mut GameState,
        source: ObjectId,
        dm: &mut Answers,
        owner: usize,
    ) -> Result<crate::effect::EffectOutcome, crate::effects::ExecutionError> {
        let alice = PlayerId::from_index(0);
        let spec = if owner >= 2 {
            crate::target::ChooseSpec::all(crate::target::ObjectFilter::specific(source))
        } else {
            crate::target::ChooseSpec::target(crate::target::ChooseSpec::SpecificObject(source))
        };
        let effect = if owner % 2 == 0 {
            Effect::new(crate::effects::DestroyEffect::with_spec(spec))
        } else {
            Effect::new(crate::effects::DestroyNoRegenerationEffect::with_spec(spec))
        };
        let mut ctx = crate::effects::ExecutionContext::new(source, alice, dm);
        crate::effects::execute_effect(game, &effect, &mut ctx)
    }
    fn check(instead: bool, owner: usize) {
        for invalid in [false, true] {
            let mut game = GameState::new(
                vec![
                    "Alice".into(),
                    "Bob".into(),
                    "Charlie".into(),
                    "Diana".into(),
                ],
                20,
            );
            let alice = PlayerId::from_index(0);
            let bob = PlayerId::from_index(1);
            let charlie = PlayerId::from_index(2);
            let diana = PlayerId::from_index(3);
            let agent = crate::cards::CardDefinitionBuilder::new(
                CardId::new(),
                "Combat Search Controller Probe",
            )
            .card_types(vec![crate::types::CardType::Creature])
            .with_ability(crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::control_opponents_while_searching_libraries(
                ),
            ))
            .with_ability(crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::opponent_search_exile_found_cards(),
            ))
            .build();
            let older = game.create_object_from_definition(&agent, bob, Zone::Battlefield);
            game.create_object_from_definition(&agent, diana, Zone::Battlefield);
            let source = prefix_card(
                &mut game,
                alice,
                Zone::Battlefield,
                "Destroy replacement source",
            );
            let found = prefix_card(
                &mut game,
                charlie,
                Zone::Library,
                "Combat replacement found card",
            );
            let stable = game.object(found).unwrap().stable_id;
            let payload = vec![Effect::new(crate::effects::SearchLibraryEffect::to_hand(
                crate::target::ObjectFilter::default(),
                crate::target::PlayerFilter::Specific(charlie),
                false,
            ))];
            let action = if instead {
                ReplacementAction::Instead(payload)
            } else {
                ReplacementAction::Additionally(payload)
            };
            let shield = game.effect_store.replacement_effects.add_one_shot_effect(
                ReplacementEffect::with_matcher(
                    source,
                    alice,
                    crate::events::permanents::matchers::WouldBeDestroyedMatcher::new(
                        crate::target::ObjectFilter::specific(source),
                    ),
                    action,
                ),
            );
            if owner % 2 == 1 {
                game.add_regeneration_shield(source, 2);
            }
            game.take_pending_trigger_events();
            let ids = game.next_object_id_counter();
            let objects = game.objects_in_deterministic_order().len();
            let mut paused = Answers {
                selected: older,
                controller: diana,
                pause: true,
                pending: false,
                invalid: false,
                choices: 0,
            };
            let events = execute(&mut game, source, &mut paused, owner).unwrap();
            assert!(paused.pending);
            assert_eq!(paused.choices, 1);
            assert_eq!(events.count_or_zero(), 0);
            assert_eq!(game.object(source).unwrap().zone, Zone::Battlefield);
            assert_eq!(
                game.regeneration_shield_count(source),
                if owner % 2 == 1 { 2 } else { 0 }
            );
            for player in [alice, bob, charlie, diana] {
                assert_eq!(game.player(player).unwrap().life, 20);
            }
            assert_eq!(game.object(found).unwrap().zone, Zone::Library);
            assert!(game.exile.is_empty());
            assert_eq!(game.next_object_id_counter(), ids);
            assert_eq!(game.objects_in_deterministic_order().len(), objects);
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(shield)
                    .is_some()
            );
            assert!(game.take_pending_trigger_events().is_empty());
            assert_eq!(game.effect_store.trigger_matching_holds, 0);
            assert!(
                !game
                    .effect_store
                    .prevention_effects
                    .follow_ups_are_deferred()
            );
            assert_eq!(
                game.controlling_player_for(charlie),
                diana,
                "effect destroy owner rollback must retain the actual nested prompt controller"
            );
            let mut resumed = Answers {
                selected: older,
                controller: diana,
                pause: false,
                pending: false,
                invalid,
                choices: 0,
            };
            let result = execute(&mut game, source, &mut resumed, owner);
            assert!(!resumed.pending);
            assert_eq!(resumed.choices, 1);
            if invalid {
                let error = result
                    .expect_err("invalid nested answer must propagate a destroy execution error");
                assert!(matches!(
                    error,
                    crate::effects::ExecutionError::InternalError(_)
                ));
                assert_eq!(game.object(source).unwrap().zone, Zone::Battlefield);
                assert_eq!(
                    game.regeneration_shield_count(source),
                    if owner % 2 == 1 { 2 } else { 0 }
                );
                for player in [alice, bob, charlie, diana] {
                    assert_eq!(game.player(player).unwrap().life, 20);
                }
                assert_eq!(game.object(found).unwrap().zone, Zone::Library);
                assert!(game.exile.is_empty());
                assert_eq!(game.next_object_id_counter(), ids);
                assert!(
                    game.effect_store
                        .replacement_effects
                        .get_effect(shield)
                        .is_some()
                );
                assert!(game.take_pending_trigger_events().is_empty());
            } else {
                let receipt = result.unwrap();
                if instead {
                    assert_eq!(receipt.count_or_zero(), 0);
                    assert_eq!(game.object(source).unwrap().zone, Zone::Battlefield);
                    assert_eq!(
                        game.regeneration_shield_count(source),
                        if owner % 2 == 1 { 2 } else { 0 }
                    );
                } else {
                    if owner >= 2 {
                        assert_eq!(receipt.count_or_zero(), 1);
                    } else {
                        assert!(receipt.status.is_success());
                    }
                    assert_eq!(receipt.affected_object_memory().unwrap().len(), 1);
                    assert_eq!(
                        receipt.affected_object_memory().unwrap()[0].object_id,
                        source
                    );
                    assert!(game.object(source).is_none());
                    assert_eq!(game.player(alice).unwrap().graveyard.len(), 1);
                }
                for player in [alice, bob, charlie, diana] {
                    assert_eq!(game.player(player).unwrap().life, 20);
                }
                let arrival = game.find_object_by_stable_id(stable).unwrap();
                assert_eq!(game.object(arrival).unwrap().zone, Zone::Exile);
                for player in [alice, bob, charlie, diana] {
                    assert_eq!(
                        game.effect_store.grant_registry.card_can_play_from_zone(
                            &game,
                            arrival,
                            Zone::Exile,
                            player
                        ),
                        player == bob
                    );
                }
                assert!(
                    game.effect_store
                        .replacement_effects
                        .get_effect(shield)
                        .is_none()
                );
            }
            assert_eq!(game.controlling_player_for(charlie), charlie);
            assert_eq!(game.effect_store.trigger_matching_holds, 0);
            assert!(
                !game
                    .effect_store
                    .prevention_effects
                    .follow_ups_are_deferred()
            );
        }
    }
    #[test]
    fn single_addition_retains_nested_prompt_and_replays() {
        check(false, 0);
    }
    #[test]
    fn single_instead_retains_nested_prompt_and_replays() {
        check(true, 0);
    }
    #[test]
    fn single_no_regeneration_addition_retains_nested_prompt_and_replays() {
        check(false, 1);
    }
    #[test]
    fn single_no_regeneration_instead_retains_nested_prompt_and_replays() {
        check(true, 1);
    }
    #[test]
    fn simultaneous_addition_retains_nested_prompt_and_replays() {
        check(false, 2);
    }
    #[test]
    fn simultaneous_instead_retains_nested_prompt_and_replays() {
        check(true, 2);
    }
    #[test]
    fn simultaneous_no_regeneration_addition_retains_nested_prompt_and_replays() {
        check(false, 3);
    }
    #[test]
    fn simultaneous_no_regeneration_instead_retains_nested_prompt_and_replays() {
        check(true, 3);
    }
}

mod discard_primary_observation_contract_tests {
    use super::*;
    fn check(instead: bool) {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = prefix_card(
            &mut game,
            alice,
            Zone::Battlefield,
            "Discard instruction source",
        );
        let selected = prefix_card(&mut game, alice, Zone::Hand, "Original discard card");
        let selected_stable = game.object(selected).unwrap().stable_id;
        let auxiliary = prefix_card(
            &mut game,
            alice,
            Zone::Battlefield,
            "Replacement counter recipient",
        );
        let payload = vec![Effect::new(crate::effects::PutCountersEffect::new(
            crate::object::CounterType::PlusOnePlusOne,
            1,
            crate::target::ChooseSpec::SpecificObject(auxiliary),
        ))];
        let action = if instead {
            ReplacementAction::Instead(payload)
        } else {
            ReplacementAction::Additionally(payload)
        };
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(
            ReplacementEffect::with_matcher(
                source,
                alice,
                crate::events::cards::matchers::WouldDiscardMatcher::you(),
                action,
            ),
        );
        game.take_pending_trigger_events();
        let mut ctx = crate::effects::ExecutionContext::new_default(source, alice);
        let outcome = crate::effects::execute_effect(
            &mut game,
            &Effect::new(crate::effects::DiscardEffect::you(1)).tag("discarded"),
            &mut ctx,
        )
        .unwrap();
        assert_eq!(outcome.count_or_zero(), if instead { 0 } else { 1 });
        let arrival = game.find_object_by_stable_id(selected_stable).unwrap();
        assert_eq!(
            game.object(arrival).unwrap().zone,
            if instead { Zone::Hand } else { Zone::Graveyard }
        );
        assert_eq!(
            game.counter_count(auxiliary, crate::object::CounterType::PlusOnePlusOne),
            1
        );
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(shield)
                .is_none()
        );
        assert!(
            outcome
                .events
                .iter()
                .chain(game.effect_store.pending_trigger_events.iter())
                .any(|event| event
                    .downcast::<crate::events::MarkersChangedEvent>()
                    .is_some_and(|event| event.object() == Some(auxiliary) && event.is_added())),
            "replacement counter observation must remain published"
        );
        let memories = outcome.affected_object_memory().unwrap_or(&[]);
        assert!(
            !memories.iter().any(|memory| memory.object_id == auxiliary),
            "replacement counter recipient was not discarded by the authored instruction"
        );
        if instead {
            assert!(memories.is_empty());
        } else {
            assert!(
                memories
                    .iter()
                    .any(|memory| memory.stable_id == selected_stable)
            );
        }
        assert!(
            ctx.get_tagged_all("discarded")
                .is_none_or(|tags| tags.iter().all(|snapshot| snapshot.object_id != auxiliary))
        );
    }
    #[test]
    fn additional_payload_counter_is_observed_without_becoming_discarded() {
        check(false);
    }
    #[test]
    fn instead_payload_counter_is_observed_without_becoming_discarded() {
        check(true);
    }
}

mod deferred_energy_cast_fallback_contract_tests {
    use super::*;
    struct Accept;
    impl crate::decision::DecisionMaker for Accept {
        fn decide_boolean(
            &mut self,
            _: &GameState,
            _: &crate::decisions::context::BooleanContext,
        ) -> bool {
            true
        }
    }
    fn check(madness_owner: bool) {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        game.player_mut(alice).unwrap().energy_counters = 2;
        let cost = crate::cost::TotalCost::from_costs(vec![
            crate::costs::Cost::try_effect(Effect::new(crate::effects::PayEnergyEffect::new(
                3,
                crate::target::ChooseSpec::Player(crate::target::PlayerFilter::You),
            )))
            .unwrap(),
        ]);
        let definition =
            crate::cards::CardDefinitionBuilder::new(CardId::new(), "Deferred energy cast source")
                .card_types(vec![crate::types::CardType::Creature])
                .power_toughness(crate::card::PowerToughness::fixed(2, 2))
                .alternative_cast(crate::alternative_cast::AlternativeCastingMethod::Madness {
                    total_cost: cost,
                })
                .build();
        let source = game.create_object_from_definition(&definition, alice, Zone::Exile);
        let stable = game.object(source).unwrap().stable_id;
        game.set_madness_exiled(source);
        game.take_pending_trigger_events();
        let ids = game.next_object_id_counter();
        let objects = game.objects_in_deterministic_order().len();
        let mut dm = Accept;
        if madness_owner {
            let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut dm);
            crate::effects::EffectExecutor::execute(
                &crate::effects::MayCastForMadnessCostEffect::new(),
                &mut game,
                &mut ctx,
            )
            .expect("ordinary energy inability must take the madness graveyard fallback");
            let arrival = game.find_object_by_stable_id(stable).unwrap();
            assert_eq!(game.object(arrival).unwrap().zone, Zone::Graveyard);
            assert!(!game.is_madness_exiled(source));
        } else {
            game.authorize_madness_cast(source);
            let result = crate::game_loop::cast_spell_from_resolving_effect(
                &mut game,
                source,
                Zone::Exile,
                alice,
                &crate::alternative_cast::CastingMethod::Alternative(0),
                false,
                None,
                crate::provenance::ProvNodeId::default(),
                &mut dm,
            );
            game.revoke_madness_cast(source);
            assert_eq!(
                result.expect("ordinary energy inability must cancel without an execution error"),
                None
            );
            assert_eq!(game.object(source).unwrap().zone, Zone::Exile);
            assert!(game.is_madness_exiled(source));
            assert_eq!(game.next_object_id_counter(), ids);
            assert!(game.take_pending_trigger_events().is_empty());
        }
        assert!(game.stack.is_empty());
        assert_eq!(game.objects_in_deterministic_order().len(), objects);
        assert_eq!(game.player(alice).unwrap().life, 20);
        assert_eq!(game.player(alice).unwrap().energy_counters, 2);
    }
    #[test]
    fn shared_cast_unpayable_energy_cancels_with_complete_rollback() {
        check(false);
    }
    #[test]
    fn madness_owner_unpayable_energy_executes_graveyard_fallback() {
        check(true);
    }
}

mod replacement_original_followup_gameplay_contract_tests {
    use super::*;
    struct Accept;
    impl crate::decision::DecisionMaker for Accept {
        fn decide_boolean(
            &mut self,
            _: &GameState,
            _: &crate::decisions::context::BooleanContext,
        ) -> bool {
            true
        }
    }
    fn check(optional: bool) {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = prefix_card(
            &mut game,
            alice,
            Zone::Battlefield,
            "Original followup source",
        );
        let original = prefix_card(
            &mut game,
            alice,
            Zone::Battlefield,
            "Original destroy target",
        );
        let auxiliary = prefix_card(
            &mut game,
            alice,
            Zone::Battlefield,
            "Auxiliary counter recipient",
        );
        let destruction = game.effect_store.replacement_effects.add_one_shot_effect(
            ReplacementEffect::with_matcher(
                source,
                alice,
                crate::events::permanents::matchers::WouldBeDestroyedMatcher::new(
                    crate::target::ObjectFilter::specific(original),
                ),
                ReplacementAction::Additionally(vec![Effect::new(
                    crate::effects::PutCountersEffect::new(
                        crate::object::CounterType::PlusOnePlusOne,
                        1,
                        crate::target::ChooseSpec::SpecificObject(auxiliary),
                    ),
                )]),
            ),
        );
        let counter = game.effect_store.replacement_effects.add_one_shot_effect(
            ReplacementEffect::with_matcher(
                source,
                alice,
                crate::events::counters::matchers::WouldPutCountersMatcher::new(
                    crate::target::ObjectFilter::specific(auxiliary),
                    Some(crate::object::CounterType::PlusOnePlusOne),
                ),
                ReplacementAction::Prevent,
            ),
        );
        game.take_pending_trigger_events();
        let action = Effect::new(crate::effects::DestroyEffect::target(
            crate::target::ChooseSpec::SpecificObject(original),
        ));
        let effects = if optional {
            Effect::may_if_do(881, action, vec![Effect::gain_life(7)])
        } else {
            Effect::do_if_do(881, action, vec![Effect::gain_life(7)])
        };
        let mut dm = Accept;
        let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut dm);
        for effect in &effects {
            crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
        }
        assert!(game.object(original).is_none());
        assert_eq!(game.object(auxiliary).unwrap().zone, Zone::Battlefield);
        assert_eq!(
            game.counter_count(auxiliary, crate::object::CounterType::PlusOnePlusOne),
            0
        );
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(destruction)
                .is_none()
        );
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(counter)
                .is_none()
        );
        assert_eq!(game.player(alice).unwrap().graveyard.len(), 1);
        assert_eq!(
            game.player(alice).unwrap().life,
            27,
            "preventing only the auxiliary counter action cannot suppress the original destruction followup"
        );
    }
    #[test]
    fn if_you_do_uses_original_destruction_despite_auxiliary_prevention() {
        check(false);
    }
    #[test]
    fn accepted_may_if_you_do_uses_original_destruction_despite_auxiliary_prevention() {
        check(true);
    }
}

mod scalar_instead_original_observation_contract_tests {
    use super::*;
    #[test]
    fn counter_instead_payload_recipient_is_observed_without_becoming_original_affected_object() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = prefix_card(
            &mut game,
            alice,
            Zone::Battlefield,
            "Counter instruction source",
        );
        let original = prefix_card(
            &mut game,
            alice,
            Zone::Battlefield,
            "Original counter recipient",
        );
        let auxiliary = prefix_card(
            &mut game,
            alice,
            Zone::Battlefield,
            "Counter replacement recipient",
        );
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(
            ReplacementEffect::with_matcher(
                source,
                alice,
                crate::events::counters::matchers::WouldPutCountersMatcher::new(
                    crate::target::ObjectFilter::specific(original),
                    Some(crate::object::CounterType::PlusOnePlusOne),
                ),
                ReplacementAction::Instead(vec![Effect::new(
                    crate::effects::PutCountersEffect::new(
                        crate::object::CounterType::PlusOnePlusOne,
                        1,
                        crate::target::ChooseSpec::SpecificObject(auxiliary),
                    ),
                )]),
            ),
        );
        game.take_pending_trigger_events();
        let mut ctx = crate::effects::ExecutionContext::new_default(source, alice);
        let outcome = crate::effects::execute_effect(
            &mut game,
            &Effect::new(crate::effects::PutCountersEffect::new(
                crate::object::CounterType::PlusOnePlusOne,
                1,
                crate::target::ChooseSpec::SpecificObject(original),
            ))
            .tag("placed"),
            &mut ctx,
        )
        .unwrap();
        assert_eq!(outcome.count_or_zero(), 0);
        assert_eq!(outcome.status, crate::effect::OutcomeStatus::Replaced);
        assert_eq!(
            game.counter_count(original, crate::object::CounterType::PlusOnePlusOne),
            0
        );
        assert_eq!(
            game.counter_count(auxiliary, crate::object::CounterType::PlusOnePlusOne),
            1
        );
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(shield)
                .is_none()
        );
        assert!(
            outcome
                .events
                .iter()
                .chain(game.effect_store.pending_trigger_events.iter())
                .any(|event| event
                    .downcast::<crate::events::MarkersChangedEvent>()
                    .is_some_and(|event| event.object() == Some(auxiliary) && event.is_added()))
        );
        assert!(
            !outcome
                .affected_object_memory()
                .unwrap_or(&[])
                .iter()
                .any(|memory| memory.object_id == auxiliary)
        );
        assert!(
            ctx.get_tagged_all("placed")
                .is_none_or(|tags| tags.iter().all(|snapshot| snapshot.object_id != auxiliary))
        );
    }
    #[test]
    fn life_instead_payload_movement_is_observed_without_becoming_original_affected_object() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = prefix_card(
            &mut game,
            alice,
            Zone::Battlefield,
            "Life instruction source",
        );
        let auxiliary = prefix_card(
            &mut game,
            alice,
            Zone::Battlefield,
            "Life replacement exile object",
        );
        let stable = game.object(auxiliary).unwrap().stable_id;
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(
            ReplacementEffect::with_matcher(
                source,
                alice,
                crate::events::life::matchers::WouldGainLifeMatcher::you(),
                ReplacementAction::Instead(vec![Effect::exile(
                    crate::target::ChooseSpec::SpecificObject(auxiliary),
                )]),
            ),
        );
        game.take_pending_trigger_events();
        let mut ctx = crate::effects::ExecutionContext::new_default(source, alice);
        let outcome = crate::effects::execute_effect(
            &mut game,
            &Effect::gain_life(1).tag("gained"),
            &mut ctx,
        )
        .unwrap();
        assert_eq!(outcome.count_or_zero(), 0);
        assert_eq!(outcome.status, crate::effect::OutcomeStatus::Replaced);
        assert_eq!(game.player(alice).unwrap().life, 20);
        let arrival = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.object(arrival).unwrap().zone, Zone::Exile);
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(shield)
                .is_none()
        );
        assert!(
            game.turn_store
                .turn_history
                .projected_records()
                .any(|record| record
                    .event
                    .downcast::<crate::events::ZoneChangeEvent>()
                    .is_some_and(
                        |event| event.objects.contains(&auxiliary) && event.to == Zone::Exile
                    ))
        );
        assert!(
            !outcome
                .affected_object_memory()
                .unwrap_or(&[])
                .iter()
                .any(|memory| memory.stable_id == stable)
        );
        assert!(
            ctx.get_tagged_all("gained")
                .is_none_or(|tags| tags.iter().all(|snapshot| snapshot.stable_id != stable))
        );
    }
}

mod player_counter_original_observation_contract_tests {
    use super::*;
    #[test]
    fn player_counter_instead_payload_recipient_is_observed_without_becoming_original_affected_object()
     {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = prefix_card(
            &mut game,
            alice,
            Zone::Battlefield,
            "Counter instruction source",
        );
        let original = prefix_card(
            &mut game,
            alice,
            Zone::Battlefield,
            "Original counter recipient",
        );
        let auxiliary = prefix_card(
            &mut game,
            alice,
            Zone::Battlefield,
            "Counter replacement recipient",
        );
        let mut replacement =
            crate::static_abilities::StaticAbility::double_player_counters_replacement(
                crate::target::PlayerFilter::Specific(alice),
                Some(crate::object::CounterType::Energy),
                "Player counter original-result probe".into(),
            )
            .generate_replacement_effect(source, alice)
            .unwrap();
        replacement.replacement =
            ReplacementAction::Instead(vec![Effect::new(crate::effects::PutCountersEffect::new(
                crate::object::CounterType::PlusOnePlusOne,
                1,
                crate::target::ChooseSpec::SpecificObject(auxiliary),
            ))]);
        let shield = game
            .effect_store
            .replacement_effects
            .add_one_shot_effect(replacement);
        game.take_pending_trigger_events();
        let mut ctx = crate::effects::ExecutionContext::new_default(source, alice);
        let outcome = crate::effects::execute_effect(
            &mut game,
            &Effect::new(crate::effects::PlayerCountersEffect::new(
                crate::object::CounterType::Energy,
                1,
                crate::target::PlayerFilter::Specific(alice),
            ))
            .tag("placed"),
            &mut ctx,
        )
        .unwrap();
        assert_eq!(outcome.count_or_zero(), 0);
        assert_eq!(outcome.status, crate::effect::OutcomeStatus::Replaced);
        assert_eq!(game.player(alice).unwrap().energy_counters, 0);
        assert_eq!(
            game.counter_count(original, crate::object::CounterType::PlusOnePlusOne),
            0
        );
        assert_eq!(
            game.counter_count(auxiliary, crate::object::CounterType::PlusOnePlusOne),
            1
        );
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(shield)
                .is_none()
        );
        assert!(
            outcome
                .events
                .iter()
                .chain(game.effect_store.pending_trigger_events.iter())
                .any(|event| event
                    .downcast::<crate::events::MarkersChangedEvent>()
                    .is_some_and(|event| event.object() == Some(auxiliary) && event.is_added()))
        );
        assert!(
            !outcome
                .affected_object_memory()
                .unwrap_or(&[])
                .iter()
                .any(|memory| memory.object_id == auxiliary)
        );
        assert!(
            ctx.get_tagged_all("placed")
                .is_none_or(|tags| tags.iter().all(|snapshot| snapshot.object_id != auxiliary))
        );
    }
}

mod untap_owner_nested_prompt_contract_tests {
    use super::*;
    struct Answers {
        selected: ObjectId,
        controller: PlayerId,
        pause: bool,
        pending: bool,
        invalid: bool,
        choices: usize,
    }
    impl crate::decision::DecisionMaker for Answers {
        fn decide_objects(
            &mut self,
            game: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            assert_eq!(game.controlling_player_for(ctx.player), self.controller);
            ctx.candidates
                .iter()
                .filter(|card| card.legal)
                .map(|card| card.id)
                .collect()
        }
        fn decide_options(
            &mut self,
            game: &GameState,
            ctx: &crate::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            assert_eq!(game.controlling_player_for(ctx.player), self.controller);
            assert_eq!(ctx.options.len(), 2);
            self.choices += 1;
            self.pending = self.pause;
            if self.invalid {
                vec![usize::MAX]
            } else {
                vec![
                    ctx.options
                        .iter()
                        .find(|option| option.object_id == Some(self.selected))
                        .unwrap()
                        .index,
                ]
            }
        }
        fn awaiting_choice(&self) -> bool {
            self.pending
        }
    }

    fn execute(
        game: &mut GameState,
        source: ObjectId,
        dm: &mut Answers,
        public: bool,
    ) -> Result<crate::effect::EffectOutcome, crate::effects::ExecutionError> {
        if public {
            let mut ctx =
                crate::effects::ExecutionContext::new(source, PlayerId::from_index(0), dm);
            crate::effects::execute_effect(
                game,
                &Effect::new(crate::effects::UntapEffect::with_spec(
                    crate::target::ChooseSpec::SpecificObject(source),
                )),
                &mut ctx,
            )
        } else {
            process_untap(game, source, dm)
        }
    }
    fn check(instead: bool, public: bool) {
        for invalid in [false, true] {
            let mut game = GameState::new(
                vec![
                    "Alice".into(),
                    "Bob".into(),
                    "Charlie".into(),
                    "Diana".into(),
                ],
                20,
            );
            let alice = PlayerId::from_index(0);
            let bob = PlayerId::from_index(1);
            let charlie = PlayerId::from_index(2);
            let diana = PlayerId::from_index(3);
            let agent = crate::cards::CardDefinitionBuilder::new(
                CardId::new(),
                "Combat Search Controller Probe",
            )
            .card_types(vec![crate::types::CardType::Creature])
            .with_ability(crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::control_opponents_while_searching_libraries(
                ),
            ))
            .with_ability(crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::opponent_search_exile_found_cards(),
            ))
            .build();
            let older = game.create_object_from_definition(&agent, bob, Zone::Battlefield);
            game.create_object_from_definition(&agent, diana, Zone::Battlefield);
            let source = prefix_card(
                &mut game,
                alice,
                Zone::Battlefield,
                "Untap replacement source",
            );
            let found = prefix_card(
                &mut game,
                charlie,
                Zone::Library,
                "Combat replacement found card",
            );
            let stable = game.object(found).unwrap().stable_id;
            let payload = vec![Effect::new(crate::effects::SearchLibraryEffect::to_hand(
                crate::target::ObjectFilter::default(),
                crate::target::PlayerFilter::Specific(charlie),
                false,
            ))];
            let action = if instead {
                ReplacementAction::Instead(payload)
            } else {
                ReplacementAction::Additionally(payload)
            };
            let shield = game.effect_store.replacement_effects.add_one_shot_effect(
                ReplacementEffect::with_matcher(
                    source,
                    alice,
                    crate::events::permanents::matchers::WouldBecomeUntappedMatcher::new(
                        crate::target::ObjectFilter::specific(source),
                    ),
                    action,
                ),
            );
            game.tap(source);
            game.take_pending_trigger_events();
            let ids = game.next_object_id_counter();
            let objects = game.objects_in_deterministic_order().len();
            let mut paused = Answers {
                selected: older,
                controller: diana,
                pause: true,
                pending: false,
                invalid: false,
                choices: 0,
            };
            let events = execute(&mut game, source, &mut paused, public).unwrap();
            assert!(paused.pending);
            assert_eq!(paused.choices, 1);
            assert_eq!(events.count_or_zero(), 0);
            assert_eq!(game.object(source).unwrap().zone, Zone::Battlefield);
            assert!(game.is_tapped(source));
            for player in [alice, bob, charlie, diana] {
                assert_eq!(game.player(player).unwrap().life, 20);
            }
            assert_eq!(game.object(found).unwrap().zone, Zone::Library);
            assert!(game.exile.is_empty());
            assert_eq!(game.next_object_id_counter(), ids);
            assert_eq!(game.objects_in_deterministic_order().len(), objects);
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(shield)
                    .is_some()
            );
            assert!(game.take_pending_trigger_events().is_empty());
            assert_eq!(game.effect_store.trigger_matching_holds, 0);
            assert!(
                !game
                    .effect_store
                    .prevention_effects
                    .follow_ups_are_deferred()
            );
            assert_eq!(
                game.controlling_player_for(charlie),
                diana,
                "untap owner rollback must retain the actual nested prompt controller"
            );
            let mut resumed = Answers {
                selected: older,
                controller: diana,
                pause: false,
                pending: false,
                invalid,
                choices: 0,
            };
            let result = execute(&mut game, source, &mut resumed, public);
            assert!(!resumed.pending);
            assert_eq!(resumed.choices, 1);
            if invalid {
                let error = result
                    .expect_err("invalid nested answer must propagate a untap execution error");
                assert!(matches!(
                    error,
                    crate::effects::ExecutionError::InternalError(_)
                ));
                assert_eq!(game.object(source).unwrap().zone, Zone::Battlefield);
                assert!(game.is_tapped(source));
                for player in [alice, bob, charlie, diana] {
                    assert_eq!(game.player(player).unwrap().life, 20);
                }
                assert_eq!(game.object(found).unwrap().zone, Zone::Library);
                assert!(game.exile.is_empty());
                assert_eq!(game.next_object_id_counter(), ids);
                assert!(
                    game.effect_store
                        .replacement_effects
                        .get_effect(shield)
                        .is_some()
                );
                assert!(game.take_pending_trigger_events().is_empty());
            } else {
                let receipt = result.unwrap();
                if instead {
                    assert_eq!(receipt.count_or_zero(), 0);
                    assert_eq!(game.object(source).unwrap().zone, Zone::Battlefield);
                    assert!(game.is_tapped(source));
                } else {
                    assert_eq!(receipt.count_or_zero(), 1);
                    assert_eq!(game.object(source).unwrap().zone, Zone::Battlefield);
                    assert!(!game.is_tapped(source));
                    assert!(game.player(alice).unwrap().graveyard.is_empty());
                }
                for player in [alice, bob, charlie, diana] {
                    assert_eq!(game.player(player).unwrap().life, 20);
                }
                let arrival = game.find_object_by_stable_id(stable).unwrap();
                assert_eq!(game.object(arrival).unwrap().zone, Zone::Exile);
                for player in [alice, bob, charlie, diana] {
                    assert_eq!(
                        game.effect_store.grant_registry.card_can_play_from_zone(
                            &game,
                            arrival,
                            Zone::Exile,
                            player
                        ),
                        player == bob
                    );
                }
                assert!(
                    game.effect_store
                        .replacement_effects
                        .get_effect(shield)
                        .is_none()
                );
            }
            assert_eq!(game.controlling_player_for(charlie), charlie);
            assert_eq!(game.effect_store.trigger_matching_holds, 0);
            assert!(
                !game
                    .effect_store
                    .prevention_effects
                    .follow_ups_are_deferred()
            );
        }
    }
    #[test]
    fn core_addition_retains_nested_prompt_and_replays() {
        check(false, false);
    }
    #[test]
    fn core_instead_retains_nested_prompt_and_replays() {
        check(true, false);
    }
    #[test]
    fn public_addition_retains_nested_prompt_and_replays() {
        check(false, true);
    }
    #[test]
    fn public_instead_retains_nested_prompt_and_replays() {
        check(true, true);
    }
}

mod untap_original_observation_contract_tests {
    use super::*;
    fn check(public: bool) {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = prefix_card(
            &mut game,
            alice,
            Zone::Battlefield,
            "Untap instruction source",
        );
        let original = prefix_card(
            &mut game,
            alice,
            Zone::Battlefield,
            "Original untap subject",
        );
        let auxiliary = prefix_card(
            &mut game,
            alice,
            Zone::Battlefield,
            "Untap replacement counter recipient",
        );
        game.tap(original);
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(
            ReplacementEffect::with_matcher(
                source,
                alice,
                crate::events::permanents::matchers::WouldBecomeUntappedMatcher::new(
                    crate::target::ObjectFilter::specific(original),
                ),
                ReplacementAction::Instead(vec![Effect::new(
                    crate::effects::PutCountersEffect::new(
                        crate::object::CounterType::PlusOnePlusOne,
                        1,
                        crate::target::ChooseSpec::SpecificObject(auxiliary),
                    ),
                )]),
            ),
        );
        game.take_pending_trigger_events();
        let mut ctx = crate::effects::ExecutionContext::new_default(source, alice);
        let outcome = if public {
            crate::effects::execute_effect(
                &mut game,
                &Effect::new(crate::effects::UntapEffect::with_spec(
                    crate::target::ChooseSpec::SpecificObject(original),
                ))
                .tag("untapped"),
                &mut ctx,
            )
            .unwrap()
        } else {
            process_untap_with_execution_context(&mut game, original, &mut ctx).unwrap()
        };
        assert_eq!(outcome.count_or_zero(), 0);
        assert_eq!(outcome.status, crate::effect::OutcomeStatus::Replaced);
        assert!(game.is_tapped(original));
        assert_eq!(
            game.counter_count(auxiliary, crate::object::CounterType::PlusOnePlusOne),
            1
        );
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(shield)
                .is_none()
        );
        assert!(
            outcome
                .events
                .iter()
                .chain(game.effect_store.pending_trigger_events.iter())
                .any(|event| event
                    .downcast::<crate::events::MarkersChangedEvent>()
                    .is_some_and(|event| event.object() == Some(auxiliary) && event.is_added()))
        );
        assert!(
            !outcome
                .affected_object_memory()
                .unwrap_or(&[])
                .iter()
                .any(|memory| memory.object_id == auxiliary),
            "replacement counter recipient is not an object untapped by the original instruction"
        );
        if public {
            assert!(
                ctx.get_tagged_all("untapped")
                    .is_none_or(|tags| tags.iter().all(|snapshot| snapshot.object_id != auxiliary))
            );
        }
    }
    #[test]
    fn core_instead_counter_is_observed_without_becoming_untapped() {
        check(false);
    }
    #[test]
    fn public_instead_counter_is_observed_without_becoming_untapped() {
        check(true);
    }
}

mod token_damage_original_observation_contract_tests {
    use super::*;
    fn check(kind: u8) {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = prefix_card(
            &mut game,
            alice,
            Zone::Battlefield,
            "Original instruction source",
        );
        let auxiliary = prefix_card(
            &mut game,
            alice,
            Zone::Battlefield,
            "Replacement counter recipient",
        );
        let payload =
            ReplacementAction::Instead(vec![Effect::new(crate::effects::PutCountersEffect::new(
                crate::object::CounterType::PlusOnePlusOne,
                1,
                crate::target::ChooseSpec::SpecificObject(auxiliary),
            ))]);
        let replacement = if kind == 3 {
            ReplacementEffect::with_matcher(
                source,
                alice,
                crate::events::damage::matchers::DamageToPlayerMatcher::to_any_player(),
                payload,
            )
        } else {
            ReplacementEffect::with_matcher(
                source,
                alice,
                crate::events::tokens::matchers::WouldCreateTokensUnderControlMatcher::new(
                    crate::target::PlayerFilter::Specific(alice),
                ),
                payload,
            )
        };
        let shield = game
            .effect_store
            .replacement_effects
            .add_one_shot_effect(replacement);
        game.take_pending_trigger_events();
        let ids = game.next_object_id_counter();
        let mut ctx = crate::effects::ExecutionContext::new_default(source, alice);
        let outcome = if kind == 3 {
            crate::effects::execute_effect(
                &mut game,
                &Effect::deal_damage(
                    3,
                    crate::target::ChooseSpec::Player(crate::target::PlayerFilter::Specific(bob)),
                )
                .tag("damaged"),
                &mut ctx,
            )
            .unwrap()
        } else {
            execute_added_token_case(&mut game, &mut ctx, kind).unwrap()
        };
        assert_eq!(outcome.count_or_zero(), 0);
        assert_eq!(game.player(alice).unwrap().life, 20);
        assert_eq!(game.player(bob).unwrap().life, 20);
        assert_eq!(game.battlefield.len(), 2);
        assert_eq!(game.next_object_id_counter(), ids);
        assert_eq!(
            game.counter_count(auxiliary, crate::object::CounterType::PlusOnePlusOne),
            1
        );
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(shield)
                .is_none()
        );
        assert!(
            outcome
                .events
                .iter()
                .chain(game.effect_store.pending_trigger_events.iter())
                .any(|event| event
                    .downcast::<crate::events::MarkersChangedEvent>()
                    .is_some_and(|event| event.object() == Some(auxiliary) && event.is_added()))
        );
        assert!(
            !outcome
                .affected_object_memory()
                .unwrap_or(&[])
                .iter()
                .any(|memory| memory.object_id == auxiliary),
            "replacement counter recipient is not affected by the original token or damage instruction"
        );
        if kind == 3 {
            assert!(
                ctx.get_tagged_all("damaged")
                    .is_none_or(|tags| tags.iter().all(|snapshot| snapshot.object_id != auxiliary))
            );
        }
    }
    #[test]
    fn token_instead_counter_does_not_become_created_object() {
        check(0);
    }
    #[test]
    fn token_copy_instead_counter_does_not_become_created_object() {
        check(1);
    }
    #[test]
    fn incubate_instead_counter_does_not_become_created_object() {
        check(2);
    }
    #[test]
    fn damage_instead_counter_does_not_become_damaged_object() {
        check(3);
    }
}

mod targeted_untap_primary_policy_contract_tests {
    use super::*;
    fn check(instead: bool) {
        use crate::effect::EffectPredicateRuntimeExt;
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = prefix_card(&mut game, alice, Zone::Battlefield, "Targeted untap source");
        let original = prefix_card(
            &mut game,
            alice,
            Zone::Battlefield,
            "Targeted untap subject",
        );
        let auxiliary = prefix_card(
            &mut game,
            alice,
            Zone::Battlefield,
            "Untap counter recipient",
        );
        game.tap(original);
        let payload = vec![Effect::new(crate::effects::PutCountersEffect::new(
            crate::object::CounterType::PlusOnePlusOne,
            1,
            crate::target::ChooseSpec::SpecificObject(auxiliary),
        ))];
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(
            ReplacementEffect::with_matcher(
                source,
                alice,
                crate::events::permanents::matchers::WouldBecomeUntappedMatcher::new(
                    crate::target::ObjectFilter::specific(original),
                ),
                if instead {
                    ReplacementAction::Instead(payload)
                } else {
                    ReplacementAction::Additionally(payload)
                },
            ),
        );
        game.take_pending_trigger_events();
        let mut ctx = crate::effects::ExecutionContext::new_default(source, alice)
            .with_targets(vec![crate::effects::ResolvedTarget::Object(original)]);
        let outcome = crate::effects::execute_effect(
            &mut game,
            &Effect::new(crate::effects::UntapEffect::target(
                crate::target::ChooseSpec::SpecificObject(original),
            )),
            &mut ctx,
        )
        .unwrap();
        assert_eq!(game.is_tapped(original), instead);
        assert_eq!(
            game.counter_count(auxiliary, crate::object::CounterType::PlusOnePlusOne),
            1
        );
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(shield)
                .is_none()
        );
        assert!(
            outcome
                .events
                .iter()
                .chain(game.effect_store.pending_trigger_events.iter())
                .any(|event| event
                    .downcast::<crate::events::MarkersChangedEvent>()
                    .is_some_and(|event| event.object() == Some(auxiliary) && event.is_added()))
        );
        assert_eq!(outcome.status, crate::effect::OutcomeStatus::Succeeded);
        assert!(matches!(outcome.value, crate::effect::OutcomeValue::None));
        assert_eq!(
            outcome.instruction_result().status,
            outcome.status,
            "target policy must update retained original status"
        );
        assert!(
            matches!(
                outcome.instruction_result().value,
                crate::effect::OutcomeValue::None
            ),
            "target policy must update retained original value"
        );
        assert!(crate::effect::EffectPredicate::Succeeded.evaluate_outcome(&outcome));
        assert!(
            !outcome
                .affected_object_memory()
                .unwrap_or(&[])
                .iter()
                .any(|memory| memory.object_id == auxiliary)
        );
    }
    #[test]
    fn targeted_instead_retains_resolution_policy_in_primary_result() {
        check(true);
    }
    #[test]
    fn targeted_addition_retains_resolution_policy_in_primary_result() {
        check(false);
    }
}

mod mana_original_observation_contract_tests {
    use super::*;
    fn check(instead: bool) {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = prefix_card(
            &mut game,
            alice,
            Zone::Battlefield,
            "Mana instruction source",
        );
        let auxiliary = prefix_card(
            &mut game,
            alice,
            Zone::Battlefield,
            "Mana replacement counter recipient",
        );
        let payload = vec![Effect::new(crate::effects::PutCountersEffect::new(
            crate::object::CounterType::PlusOnePlusOne,
            1,
            crate::target::ChooseSpec::SpecificObject(auxiliary),
        ))];
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(
            ReplacementEffect::with_matcher(
                source,
                alice,
                crate::events::mana::matchers::ManaProducedBySourceMatcher::new(
                    crate::target::ObjectFilter::default(),
                ),
                if instead {
                    ReplacementAction::Instead(payload)
                } else {
                    ReplacementAction::Additionally(payload)
                },
            ),
        );
        game.take_pending_trigger_events();
        let mut ctx = crate::effects::ExecutionContext::new_default(source, alice);
        let outcome = crate::effects::execute_effect(
            &mut game,
            &Effect::new(crate::effects::AddManaEffect::you(vec![
                crate::mana::ManaSymbol::Green,
            ]))
            .tag("produced"),
            &mut ctx,
        )
        .unwrap();
        assert_eq!(
            game.player(alice).unwrap().mana_pool.total(),
            u32::from(!instead)
        );
        assert_eq!(
            game.counter_count(auxiliary, crate::object::CounterType::PlusOnePlusOne),
            1
        );
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(shield)
                .is_none()
        );
        assert!(
            outcome
                .events
                .iter()
                .chain(game.effect_store.pending_trigger_events.iter())
                .any(|event| event
                    .downcast::<crate::events::MarkersChangedEvent>()
                    .is_some_and(|event| event.object() == Some(auxiliary) && event.is_added()))
        );
        let expected = if instead {
            Vec::new()
        } else {
            vec![crate::mana::ManaSymbol::Green]
        };
        assert!(
            matches!(&outcome.value, crate::effect::OutcomeValue::ManaAdded(mana) if mana == &expected)
        );
        assert!(
            !outcome
                .affected_object_memory()
                .unwrap_or(&[])
                .iter()
                .any(|memory| memory.object_id == auxiliary),
            "replacement counter recipient is not affected by the original mana instruction"
        );
        assert!(
            matches!(&outcome.instruction_result().value, crate::effect::OutcomeValue::ManaAdded(mana) if mana == &expected),
            "mana receipt conversion must update retained original result"
        );
        assert!(
            ctx.get_tagged_all("produced")
                .is_none_or(|tags| tags.iter().all(|snapshot| snapshot.object_id != auxiliary))
        );
    }
    #[test]
    fn instead_counter_is_observed_without_becoming_mana_result() {
        check(true);
    }
    #[test]
    fn addition_preserves_original_mana_receipt_value() {
        check(false);
    }
}

mod damage_consequence_primary_contract_tests {
    use super::*;
    fn check(infect: bool) {
        use crate::effect::EffectPredicateRuntimeExt;
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let definition =
            crate::cards::CardDefinitionBuilder::new(CardId::new(), "Damage consequence source")
                .card_types(vec![crate::types::CardType::Creature])
                .power_toughness(crate::card::PowerToughness::fixed(3, 3))
                .with_ability(crate::ability::Ability::static_ability(if infect {
                    crate::static_abilities::StaticAbility::infect()
                } else {
                    crate::static_abilities::StaticAbility::lifelink()
                }))
                .build();
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let auxiliary = prefix_card(
            &mut game,
            alice,
            Zone::Battlefield,
            "Consequence replacement counter recipient",
        );
        let payload =
            ReplacementAction::Instead(vec![Effect::new(crate::effects::PutCountersEffect::new(
                crate::object::CounterType::PlusOnePlusOne,
                1,
                crate::target::ChooseSpec::SpecificObject(auxiliary),
            ))]);
        let replacement = if infect {
            let mut replacement =
                crate::static_abilities::StaticAbility::double_player_counters_replacement(
                    crate::target::PlayerFilter::Specific(bob),
                    Some(crate::object::CounterType::Poison),
                    "Counter consequence probe".into(),
                )
                .generate_replacement_effect(source, alice)
                .unwrap();
            replacement.replacement = payload;
            replacement
        } else {
            ReplacementEffect::with_matcher(
                source,
                alice,
                crate::events::life::matchers::WouldGainLifeMatcher::you(),
                payload,
            )
        };
        let shield = game
            .effect_store
            .replacement_effects
            .add_one_shot_effect(replacement);
        game.take_pending_trigger_events();
        let mut ctx = crate::effects::ExecutionContext::new_default(source, alice);
        let outcome = crate::effects::execute_effect(
            &mut game,
            &Effect::deal_damage(3, crate::target::ChooseSpec::SpecificPlayer(bob)),
            &mut ctx,
        )
        .unwrap();
        assert_eq!(outcome.count_or_zero(), 3);
        assert_eq!(game.player(alice).unwrap().life, 20);
        assert_eq!(game.player(bob).unwrap().life, if infect { 20 } else { 17 });
        assert_eq!(
            game.player(bob)
                .unwrap()
                .counter_count(crate::object::CounterType::Poison),
            0
        );
        assert_eq!(
            game.counter_count(auxiliary, crate::object::CounterType::PlusOnePlusOne),
            1
        );
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(shield)
                .is_none()
        );
        assert!(outcome.events.iter().any(|event| {
            event
                .downcast::<crate::events::DamageEvent>()
                .is_some_and(|damage| {
                    damage.amount == 3 && damage.target == crate::events::DamageTarget::Player(bob)
                })
        }));
        assert!(
            outcome
                .events
                .iter()
                .chain(game.effect_store.pending_trigger_events.iter())
                .any(|event| event
                    .downcast::<crate::events::MarkersChangedEvent>()
                    .is_some_and(|event| event.object() == Some(auxiliary) && event.is_added()))
        );
        assert_eq!(
            outcome.instruction_result().count_or_zero(),
            3,
            "replacing a damage consequence cannot erase the original damage amount"
        );
        assert!(crate::effect::EffectPredicate::Succeeded.evaluate_outcome(&outcome));
        assert!(
            !outcome
                .affected_object_memory()
                .unwrap_or(&[])
                .iter()
                .any(|memory| memory.object_id == auxiliary)
        );
    }
    #[test]
    fn replaced_lifelink_preserves_primary_damage_quantity() {
        check(false);
    }
    #[test]
    fn replaced_infect_counter_preserves_primary_damage_quantity() {
        check(true);
    }
}

mod mana_owner_nested_prompt_contract_tests {
    use super::*;
    struct Answers {
        selected: ObjectId,
        controller: PlayerId,
        pause: bool,
        pending: bool,
        invalid: bool,
        choices: usize,
    }
    impl crate::decision::DecisionMaker for Answers {
        fn decide_objects(
            &mut self,
            game: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            assert_eq!(game.controlling_player_for(ctx.player), self.controller);
            ctx.candidates
                .iter()
                .filter(|card| card.legal)
                .map(|card| card.id)
                .collect()
        }
        fn decide_options(
            &mut self,
            game: &GameState,
            ctx: &crate::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            assert_eq!(game.controlling_player_for(ctx.player), self.controller);
            assert_eq!(ctx.options.len(), 2);
            self.choices += 1;
            self.pending = self.pause;
            if self.invalid {
                vec![usize::MAX]
            } else {
                vec![
                    ctx.options
                        .iter()
                        .find(|option| option.object_id == Some(self.selected))
                        .unwrap()
                        .index,
                ]
            }
        }
        fn awaiting_choice(&self) -> bool {
            self.pending
        }
    }

    fn execute(
        game: &mut GameState,
        source: ObjectId,
        dm: &mut Answers,
        public: bool,
    ) -> Result<crate::effect::EffectOutcome, crate::effects::ExecutionError> {
        let mut ctx = crate::effects::ExecutionContext::new(source, PlayerId::from_index(0), dm);
        let effect = if public {
            Effect::new(crate::effects::AddManaEffect::you(vec![
                crate::mana::ManaSymbol::Green,
            ]))
        } else {
            Effect::new(crate::effects::AddColorlessManaEffect::you(1))
        };
        crate::effects::execute_effect(game, &effect, &mut ctx)
    }
    fn check(instead: bool, public: bool) {
        for invalid in [false, true] {
            let mut game = GameState::new(
                vec![
                    "Alice".into(),
                    "Bob".into(),
                    "Charlie".into(),
                    "Diana".into(),
                ],
                20,
            );
            let alice = PlayerId::from_index(0);
            let bob = PlayerId::from_index(1);
            let charlie = PlayerId::from_index(2);
            let diana = PlayerId::from_index(3);
            let agent = crate::cards::CardDefinitionBuilder::new(
                CardId::new(),
                "Combat Search Controller Probe",
            )
            .card_types(vec![crate::types::CardType::Creature])
            .with_ability(crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::control_opponents_while_searching_libraries(
                ),
            ))
            .with_ability(crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::opponent_search_exile_found_cards(),
            ))
            .build();
            let older = game.create_object_from_definition(&agent, bob, Zone::Battlefield);
            game.create_object_from_definition(&agent, diana, Zone::Battlefield);
            let source = prefix_card(
                &mut game,
                alice,
                Zone::Battlefield,
                "Mana replacement source",
            );
            let found = prefix_card(
                &mut game,
                charlie,
                Zone::Library,
                "Combat replacement found card",
            );
            let stable = game.object(found).unwrap().stable_id;
            let payload = vec![Effect::new(crate::effects::SearchLibraryEffect::to_hand(
                crate::target::ObjectFilter::default(),
                crate::target::PlayerFilter::Specific(charlie),
                false,
            ))];
            let action = if instead {
                ReplacementAction::Instead(payload)
            } else {
                ReplacementAction::Additionally(payload)
            };
            let shield = game.effect_store.replacement_effects.add_one_shot_effect(
                ReplacementEffect::with_matcher(
                    source,
                    alice,
                    crate::events::mana::matchers::ManaProducedBySourceMatcher::new(
                        crate::target::ObjectFilter::specific(source),
                    ),
                    action,
                ),
            );
            game.take_pending_trigger_events();
            let ids = game.next_object_id_counter();
            let objects = game.objects_in_deterministic_order().len();
            let mut paused = Answers {
                selected: older,
                controller: diana,
                pause: true,
                pending: false,
                invalid: false,
                choices: 0,
            };
            let events = execute(&mut game, source, &mut paused, public).unwrap();
            assert!(paused.pending);
            assert_eq!(paused.choices, 1);
            assert_eq!(events.count_or_zero(), 0);
            assert_eq!(game.object(source).unwrap().zone, Zone::Battlefield);
            assert_eq!(game.player(alice).unwrap().mana_pool.total(), 0);
            for player in [alice, bob, charlie, diana] {
                assert_eq!(game.player(player).unwrap().life, 20);
            }
            assert_eq!(game.object(found).unwrap().zone, Zone::Library);
            assert!(game.exile.is_empty());
            assert_eq!(game.next_object_id_counter(), ids);
            assert_eq!(game.objects_in_deterministic_order().len(), objects);
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(shield)
                    .is_some()
            );
            assert!(game.take_pending_trigger_events().is_empty());
            assert_eq!(game.effect_store.trigger_matching_holds, 0);
            assert!(
                !game
                    .effect_store
                    .prevention_effects
                    .follow_ups_are_deferred()
            );
            assert_eq!(
                game.controlling_player_for(charlie),
                diana,
                "mana owner rollback must retain the actual nested prompt controller"
            );
            let mut resumed = Answers {
                selected: older,
                controller: diana,
                pause: false,
                pending: false,
                invalid,
                choices: 0,
            };
            let result = execute(&mut game, source, &mut resumed, public);
            assert!(!resumed.pending);
            assert_eq!(resumed.choices, 1);
            if invalid {
                let error = result
                    .expect_err("invalid nested answer must propagate a mana execution error");
                assert!(matches!(
                    error,
                    crate::effects::ExecutionError::InternalError(_)
                ));
                assert_eq!(game.object(source).unwrap().zone, Zone::Battlefield);
                assert_eq!(game.player(alice).unwrap().mana_pool.total(), 0);
                for player in [alice, bob, charlie, diana] {
                    assert_eq!(game.player(player).unwrap().life, 20);
                }
                assert_eq!(game.object(found).unwrap().zone, Zone::Library);
                assert!(game.exile.is_empty());
                assert_eq!(game.next_object_id_counter(), ids);
                assert!(
                    game.effect_store
                        .replacement_effects
                        .get_effect(shield)
                        .is_some()
                );
                assert!(game.take_pending_trigger_events().is_empty());
            } else {
                let receipt = result.unwrap();
                if instead {
                    assert!(receipt.mana().unwrap().is_empty());
                    assert_eq!(game.object(source).unwrap().zone, Zone::Battlefield);
                    assert_eq!(game.player(alice).unwrap().mana_pool.total(), 0);
                } else {
                    assert_eq!(receipt.mana().unwrap().len(), 1);
                    assert_eq!(game.object(source).unwrap().zone, Zone::Battlefield);
                    assert_eq!(game.player(alice).unwrap().mana_pool.total(), 1);
                    assert!(game.player(alice).unwrap().graveyard.is_empty());
                }
                for player in [alice, bob, charlie, diana] {
                    assert_eq!(game.player(player).unwrap().life, 20);
                }
                let arrival = game.find_object_by_stable_id(stable).unwrap();
                assert_eq!(game.object(arrival).unwrap().zone, Zone::Exile);
                for player in [alice, bob, charlie, diana] {
                    assert_eq!(
                        game.effect_store.grant_registry.card_can_play_from_zone(
                            &game,
                            arrival,
                            Zone::Exile,
                            player
                        ),
                        player == bob
                    );
                }
                assert!(
                    game.effect_store
                        .replacement_effects
                        .get_effect(shield)
                        .is_none()
                );
            }
            assert_eq!(game.controlling_player_for(charlie), charlie);
            assert_eq!(game.effect_store.trigger_matching_holds, 0);
            assert!(
                !game
                    .effect_store
                    .prevention_effects
                    .follow_ups_are_deferred()
            );
        }
    }
    #[test]
    fn colorless_addition_retains_nested_prompt_and_replays() {
        check(false, false);
    }
    #[test]
    fn colorless_instead_retains_nested_prompt_and_replays() {
        check(true, false);
    }
    #[test]
    fn colored_addition_retains_nested_prompt_and_replays() {
        check(false, true);
    }
    #[test]
    fn colored_instead_retains_nested_prompt_and_replays() {
        check(true, true);
    }
}

mod token_owner_nested_prompt_contract_tests {
    use super::*;
    struct Answers {
        selected: ObjectId,
        controller: PlayerId,
        pause: bool,
        pending: bool,
        invalid: bool,
        choices: usize,
    }
    impl crate::decision::DecisionMaker for Answers {
        fn decide_objects(
            &mut self,
            game: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            assert_eq!(game.controlling_player_for(ctx.player), self.controller);
            ctx.candidates
                .iter()
                .filter(|card| card.legal)
                .map(|card| card.id)
                .collect()
        }
        fn decide_options(
            &mut self,
            game: &GameState,
            ctx: &crate::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            assert_eq!(game.controlling_player_for(ctx.player), self.controller);
            assert_eq!(ctx.options.len(), 2);
            self.choices += 1;
            self.pending = self.pause;
            if self.invalid {
                vec![usize::MAX]
            } else {
                vec![
                    ctx.options
                        .iter()
                        .find(|option| option.object_id == Some(self.selected))
                        .unwrap()
                        .index,
                ]
            }
        }
        fn awaiting_choice(&self) -> bool {
            self.pending
        }
    }

    fn execute(
        game: &mut GameState,
        source: ObjectId,
        dm: &mut Answers,
        kind: u8,
    ) -> Result<crate::effect::EffectOutcome, crate::effects::ExecutionError> {
        let mut ctx = crate::effects::ExecutionContext::new(source, PlayerId::from_index(0), dm);
        execute_added_token_case(game, &mut ctx, kind)
    }
    fn check(instead: bool, kind: u8) {
        for invalid in [false, true] {
            let mut game = GameState::new(
                vec![
                    "Alice".into(),
                    "Bob".into(),
                    "Charlie".into(),
                    "Diana".into(),
                ],
                20,
            );
            let alice = PlayerId::from_index(0);
            let bob = PlayerId::from_index(1);
            let charlie = PlayerId::from_index(2);
            let diana = PlayerId::from_index(3);
            let agent = crate::cards::CardDefinitionBuilder::new(
                CardId::new(),
                "Combat Search Controller Probe",
            )
            .card_types(vec![crate::types::CardType::Creature])
            .with_ability(crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::control_opponents_while_searching_libraries(
                ),
            ))
            .with_ability(crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::opponent_search_exile_found_cards(),
            ))
            .build();
            let older = game.create_object_from_definition(&agent, bob, Zone::Battlefield);
            game.create_object_from_definition(&agent, diana, Zone::Battlefield);
            let source = prefix_card(
                &mut game,
                alice,
                Zone::Battlefield,
                "Token replacement source",
            );
            let found = prefix_card(
                &mut game,
                charlie,
                Zone::Library,
                "Combat replacement found card",
            );
            let stable = game.object(found).unwrap().stable_id;
            let payload = vec![Effect::new(crate::effects::SearchLibraryEffect::to_hand(
                crate::target::ObjectFilter::default(),
                crate::target::PlayerFilter::Specific(charlie),
                false,
            ))];
            let action = if instead {
                ReplacementAction::Instead(payload)
            } else {
                ReplacementAction::Additionally(payload)
            };
            let shield = game.effect_store.replacement_effects.add_one_shot_effect(
                ReplacementEffect::with_matcher(
                    source,
                    alice,
                    crate::events::tokens::matchers::WouldCreateTokensUnderControlMatcher::new(
                        crate::target::PlayerFilter::Specific(alice),
                    ),
                    action,
                ),
            );
            game.take_pending_trigger_events();
            let ids = game.next_object_id_counter();
            let objects = game.objects_in_deterministic_order().len();
            let battlefield_before = game.battlefield.len();
            let mut paused = Answers {
                selected: older,
                controller: diana,
                pause: true,
                pending: false,
                invalid: false,
                choices: 0,
            };
            let events = execute(&mut game, source, &mut paused, kind).unwrap();
            assert!(paused.pending);
            assert_eq!(paused.choices, 1);
            assert_eq!(events.count_or_zero(), 0);
            assert_eq!(game.object(source).unwrap().zone, Zone::Battlefield);
            for player in [alice, bob, charlie, diana] {
                assert_eq!(game.player(player).unwrap().life, 20);
            }
            assert_eq!(game.object(found).unwrap().zone, Zone::Library);
            assert!(game.exile.is_empty());
            assert_eq!(game.next_object_id_counter(), ids);
            assert_eq!(game.objects_in_deterministic_order().len(), objects);
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(shield)
                    .is_some()
            );
            assert!(game.take_pending_trigger_events().is_empty());
            assert_eq!(game.effect_store.trigger_matching_holds, 0);
            assert!(
                !game
                    .effect_store
                    .prevention_effects
                    .follow_ups_are_deferred()
            );
            assert_eq!(
                game.controlling_player_for(charlie),
                diana,
                "token owner rollback must retain the actual nested prompt controller"
            );
            let mut resumed = Answers {
                selected: older,
                controller: diana,
                pause: false,
                pending: false,
                invalid,
                choices: 0,
            };
            let result = execute(&mut game, source, &mut resumed, kind);
            assert!(!resumed.pending);
            assert_eq!(resumed.choices, 1);
            if invalid {
                let error = result
                    .expect_err("invalid nested answer must propagate a token execution error");
                assert!(matches!(
                    error,
                    crate::effects::ExecutionError::InternalError(_)
                ));
                assert_eq!(game.object(source).unwrap().zone, Zone::Battlefield);
                for player in [alice, bob, charlie, diana] {
                    assert_eq!(game.player(player).unwrap().life, 20);
                }
                assert_eq!(game.object(found).unwrap().zone, Zone::Library);
                assert!(game.exile.is_empty());
                assert_eq!(game.next_object_id_counter(), ids);
                assert!(
                    game.effect_store
                        .replacement_effects
                        .get_effect(shield)
                        .is_some()
                );
                assert!(game.take_pending_trigger_events().is_empty());
            } else {
                let receipt = result.unwrap();
                if instead {
                    assert!(receipt.output_objects().is_empty());
                    assert_eq!(game.object(source).unwrap().zone, Zone::Battlefield);
                } else {
                    assert_eq!(receipt.output_objects().len(), 1);
                    assert_eq!(
                        game.object(receipt.output_objects()[0]).unwrap().zone,
                        Zone::Battlefield
                    );
                    assert_eq!(game.object(source).unwrap().zone, Zone::Battlefield);

                    assert!(game.player(alice).unwrap().graveyard.is_empty());
                }
                assert_eq!(
                    game.battlefield.len(),
                    battlefield_before + usize::from(!instead)
                );
                for player in [alice, bob, charlie, diana] {
                    assert_eq!(game.player(player).unwrap().life, 20);
                }
                let arrival = game.find_object_by_stable_id(stable).unwrap();
                assert_eq!(game.object(arrival).unwrap().zone, Zone::Exile);
                for player in [alice, bob, charlie, diana] {
                    assert_eq!(
                        game.effect_store.grant_registry.card_can_play_from_zone(
                            &game,
                            arrival,
                            Zone::Exile,
                            player
                        ),
                        player == bob
                    );
                }
                assert!(
                    game.effect_store
                        .replacement_effects
                        .get_effect(shield)
                        .is_none()
                );
            }
            assert_eq!(game.controlling_player_for(charlie), charlie);
            assert_eq!(game.effect_store.trigger_matching_holds, 0);
            assert!(
                !game
                    .effect_store
                    .prevention_effects
                    .follow_ups_are_deferred()
            );
        }
    }
    #[test]
    fn token_addition_retains_nested_prompt_and_replays() {
        check(false, 0);
    }
    #[test]
    fn token_instead_retains_nested_prompt_and_replays() {
        check(true, 0);
    }
    #[test]
    fn copy_addition_retains_nested_prompt_and_replays() {
        check(false, 1);
    }
    #[test]
    fn copy_instead_retains_nested_prompt_and_replays() {
        check(true, 1);
    }
    #[test]
    fn incubate_addition_retains_nested_prompt_and_replays() {
        check(false, 2);
    }
    #[test]
    fn incubate_instead_retains_nested_prompt_and_replays() {
        check(true, 2);
    }
}

#[test]
fn public_zone_replacement_conversion_keeps_cause_for_later_entry_matchers() {
    struct PreferEntry {
        source: ObjectId,
        choices: usize,
    }
    impl crate::decision::DecisionMaker for PreferEntry {
        fn decide_options(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            self.choices += 1;
            let option = ctx
                .options
                .iter()
                .find(|option| option.legal && option.object_id == Some(self.source))
                .or_else(|| ctx.options.iter().find(|option| option.legal))
                .unwrap();
            vec![option.index]
        }
    }
    for matching_cause in [false, true] {
        let (mut game, entrant, alice) = setup();
        let bob = PlayerId::from_index(1);
        let watcher_card =
            crate::card::CardBuilder::new(CardId::new(), "Carrier cause watcher").build();
        let watcher = game.create_object_from_card(&watcher_card, alice, Zone::Battlefield);
        game.effect_store.replacement_effects.add_resolution_effect(
            ReplacementEffect::with_matcher(
                entrant,
                alice,
                crate::events::zones::matchers::ThisWouldEnterBattlefieldMatcher,
                ReplacementAction::EnterTapped,
            ),
        );
        let control = game.effect_store.replacement_effects.add_one_shot_effect(
            ReplacementEffect::with_matcher(
                watcher,
                alice,
                crate::events::zones::matchers::WouldChangeZoneMatcher::new(
                    crate::target::ObjectFilter::specific(entrant),
                    Some(Zone::Hand),
                    Some(Zone::Battlefield),
                )
                .with_cause_filter(
                    crate::events::cause::CauseFilter::effect_like()
                        .with_controller(crate::events::cause::ControllerFilter::Player(bob)),
                ),
                ReplacementAction::EnterUnderControl(bob),
            ),
        );
        let cause = crate::events::cause::EventCause::from_effect(
            watcher,
            if matching_cause { bob } else { alice },
        );
        let zone = crate::events::ZoneChangeEvent::with_cause(
            entrant,
            Zone::Hand,
            Zone::Battlefield,
            cause,
            Some(crate::snapshot::ObjectSnapshot::from_object(
                game.object(entrant).unwrap(),
                &game,
            )),
        );
        let mut dm = PreferEntry {
            source: entrant,
            choices: 0,
        };
        let event = process_with_dm(
            &mut game,
            crate::events::Event::new_with_provenance(zone, Default::default()),
            &mut dm,
        )
        .unwrap()
        .into_event()
        .unwrap();
        let entry =
            crate::events::downcast_event::<crate::events::EnterBattlefieldEvent>(event.inner())
                .unwrap();
        assert!(entry.enters_tapped);
        assert_eq!(
            entry.controller_override,
            if matching_cause { Some(bob) } else { None },
            "a carrier conversion must preserve cause-dependent applicability"
        );
        assert_eq!(
            game.effect_store
                .replacement_effects
                .get_effect(control)
                .is_some(),
            !matching_cause
        );
        assert_eq!(dm.choices, usize::from(matching_cause));
        assert_eq!(
            game.object(entrant).unwrap().zone,
            Zone::Hand,
            "processing proposals does not commit entry"
        );
    }
}

#[test]
fn public_converted_entry_prompt_retains_zone_context_without_external_state() {
    let (mut game, entrant, alice) = setup();
    let bob = PlayerId::from_index(1);
    let watcher_card =
        crate::card::CardBuilder::new(CardId::new(), "Captured carrier watcher").build();
    let watcher = game.create_object_from_card(&watcher_card, alice, Zone::Battlefield);
    game.effect_store.replacement_effects.add_resolution_effect(
        ReplacementEffect::with_matcher(
            entrant,
            alice,
            crate::events::zones::matchers::ThisWouldEnterBattlefieldMatcher,
            ReplacementAction::EnterTapped,
        )
        .with_priority_override(crate::events::ReplacementPriority::SelfReplacement),
    );
    let cause_matcher = || {
        crate::events::zones::matchers::WouldChangeZoneMatcher::new(
            crate::target::ObjectFilter::specific(entrant),
            Some(Zone::Hand),
            Some(Zone::Battlefield),
        )
        .with_cause_filter(
            crate::events::cause::CauseFilter::effect_like()
                .with_controller(crate::events::cause::ControllerFilter::Player(bob)),
        )
    };
    let control = game.effect_store.replacement_effects.add_resolution_effect(
        ReplacementEffect::with_matcher(
            watcher,
            alice,
            cause_matcher(),
            ReplacementAction::EnterUnderControl(bob),
        ),
    );
    let counter = game.effect_store.replacement_effects.add_resolution_effect(
        ReplacementEffect::with_matcher(
            watcher,
            alice,
            cause_matcher(),
            ReplacementAction::EnterWithCounters {
                counter_type: crate::object::CounterType::PlusOnePlusOne,
                count: Value::Fixed(1),
                count_condition: None,
                otherwise_count: None,
                added_subtypes: Vec::new(),
                added_abilities: Vec::new(),
            },
        ),
    );
    let cause = crate::events::cause::EventCause::from_effect(watcher, bob);
    let event = crate::events::Event::zone_change(
        entrant,
        Zone::Hand,
        Zone::Battlefield,
        cause,
        Some(crate::snapshot::ObjectSnapshot::from_object(
            game.object(entrant).unwrap(),
            &game,
        )),
    );
    let pending = process_trait_event(&mut game, event).unwrap();
    let TraitEventResult::NeedsChoice {
        applicable_effects,
        event,
        ..
    } = &pending
    else {
        panic!("both cause-matched replacements must remain offered after entry conversion");
    };
    assert!(applicable_effects.contains(&control) && applicable_effects.contains(&counter));
    assert_eq!(event.kind(), crate::events::EventKind::EnterBattlefield);
    let result =
        continue_replacement_choice_with_scope(&mut game, pending, control, None, &[], None)
            .unwrap();
    let event = result.into_event().unwrap();
    let entry =
        crate::events::downcast_event::<crate::events::EnterBattlefieldEvent>(event.inner())
            .unwrap();
    assert!(entry.enters_tapped);
    assert_eq!(entry.controller_override, Some(bob));
    assert_eq!(
        entry.enters_with_counters,
        vec![(crate::object::CounterType::PlusOnePlusOne, 1)],
        "the remaining zone matcher must survive captured choice restoration"
    );
    assert_eq!(game.object(entrant).unwrap().zone, Zone::Hand);
}

#[test]
fn public_entry_redirect_preserves_original_zone_cause_and_lki() {
    let (mut game, entrant, alice) = setup();
    let bob = PlayerId::from_index(1);
    let watcher_card =
        crate::card::CardBuilder::new(CardId::new(), "Redirect metadata watcher").build();
    let watcher = game.create_object_from_card(&watcher_card, alice, Zone::Battlefield);
    game.effect_store.replacement_effects.add_resolution_effect(
        ReplacementEffect::with_matcher(
            entrant,
            alice,
            crate::events::zones::matchers::ThisWouldEnterBattlefieldMatcher,
            ReplacementAction::EnterTapped,
        )
        .with_priority_override(crate::events::ReplacementPriority::SelfReplacement),
    );
    game.effect_store
        .replacement_effects
        .add_resolution_effect(ReplacementEffect::with_matcher(
            watcher,
            alice,
            crate::events::zones::matchers::WouldChangeZoneMatcher::new(
                crate::target::ObjectFilter::specific(entrant),
                Some(Zone::Hand),
                Some(Zone::Battlefield),
            )
            .with_cause_filter(
                crate::events::cause::CauseFilter::any()
                    .with_controller(crate::events::cause::ControllerFilter::Player(bob)),
            ),
            ReplacementAction::ChangeDestination(Zone::Exile),
        ));
    let cause = crate::events::cause::EventCause::from_cost(watcher, bob);
    let snapshot =
        crate::snapshot::ObjectSnapshot::from_object(game.object(entrant).unwrap(), &game);
    let stable = snapshot.stable_id;
    let tag = crate::tag::TagKey::from("entry-origin");
    let zone = crate::events::ZoneChangeEvent::with_cause(
        entrant,
        Zone::Hand,
        Zone::Battlefield,
        cause.clone(),
        Some(snapshot.clone()),
    )
    .with_object_tag(tag.clone(), snapshot);
    let event = crate::events::Event::new_with_provenance(zone, Default::default());
    let resolved = process_trait_event(&mut game, event)
        .unwrap()
        .into_event()
        .unwrap();
    let zone =
        crate::events::downcast_event::<crate::events::ZoneChangeEvent>(resolved.inner()).unwrap();
    assert_eq!(zone.from, Zone::Hand);
    assert_eq!(zone.to, Zone::Exile);
    assert_eq!(zone.objects, vec![entrant]);
    assert_eq!(
        zone.cause.cause_type, cause.cause_type,
        "destination replacement must preserve the operation's original cause"
    );
    assert_eq!(zone.cause.source, cause.source);
    assert_eq!(zone.cause.source_controller, cause.source_controller);
    assert_eq!(zone.snapshot.as_ref().unwrap().stable_id, stable);
    assert_eq!(zone.snapshots.len(), 1);
    assert_eq!(zone.snapshots[0].stable_id, stable);
    assert_eq!(zone.object_tags.get(&tag).unwrap().len(), 1);
    assert_eq!(zone.object_tags.get(&tag).unwrap()[0].stable_id, stable);
    assert_eq!(game.object(entrant).unwrap().zone, Zone::Hand);
}
