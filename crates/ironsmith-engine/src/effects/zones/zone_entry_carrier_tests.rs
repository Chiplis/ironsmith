// Legacy fixtures have no added instructions; assert this rather than silently
// dropping the richer receipt while checking original entry behavior.
fn require_plain_committed_zone_receipt(receipt: crate::events::processing::PreparedEventOutcome<AppliedZoneChange>) -> EventOutcome<AppliedZoneChange> {
    assert!(receipt.programs.is_empty(), "legacy fixture unexpectedly retained additions");
    receipt.original
}

use super::*;
use crate::effect::{Effect, Value};
use crate::ids::{CardId, PlayerId};
use crate::replacement::{ReplacementAction, ReplacementEffect};

fn setup() -> (GameState, ObjectId, PlayerId, PlayerId) {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let card = crate::card::CardBuilder::new(CardId::new(), "Zone entry carrier fixture")
        .card_types(vec![crate::types::CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(2, 2))
        .build();
    let entrant = game.create_object_from_card(&card, alice, Zone::Hand);
    game.take_pending_trigger_events();
    (game, entrant, alice, bob)
}

fn redirect_to_battlefield(
    game: &mut GameState,
    entrant: ObjectId,
    alice: PlayerId,
) -> crate::replacement::ReplacementEffectId {
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
        ))
}

#[test]
fn zone_redirect_commits_the_resolved_entry_controller() {
    for changes_controller in [false, true] {
        let (mut game, entrant, alice, bob) = setup();
        let redirect = redirect_to_battlefield(&mut game, entrant, alice);
        let controller_replacement = changes_controller.then(|| {
            game.effect_store.replacement_effects.add_one_shot_effect(
                ReplacementEffect::with_matcher(
                    entrant,
                    alice,
                    crate::events::zones::matchers::ThisWouldEnterBattlefieldMatcher,
                    ReplacementAction::EnterUnderControl(bob),
                ),
            )
        });
        let mut dm = crate::decision::SelectFirstDecisionMaker;
        let result = apply_zone_change(
            &mut game,
            entrant,
            Zone::Hand,
            Zone::Graveyard,
            crate::events::cause::EventCause::from_effect(entrant, alice),
            &mut dm,
        ).map(require_plain_committed_zone_receipt)
        .unwrap()
        .into_result()
        .expect("redirected zone change must proceed");
        let entered = result.new_object_id.unwrap();
        assert_eq!(result.final_zone, Zone::Battlefield);
        assert_eq!(game.object(entered).unwrap().zone, Zone::Battlefield);
        assert_eq!(game.object(entered).unwrap().owner, alice);
        assert_eq!(
            game.current_controller(entered),
            Some(if changes_controller { bob } else { alice })
        );
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(redirect)
                .is_none()
        );
        if let Some(replacement) = controller_replacement {
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(replacement)
                    .is_none()
            );
        }
    }
}

#[test]
fn zone_redirect_executes_entry_program_and_restores_the_whole_move_on_error() {
    for fails in [false, true] {
        let (mut game, entrant, alice, _) = setup();
        let stable = game.object(entrant).unwrap().stable_id;
        let redirect = redirect_to_battlefield(&mut game, entrant, alice);
        let mut effects = vec![Effect::gain_life(2)];
        if fails {
            effects.push(Effect::lose_life(Value::X));
        }
        let entry_program = game.effect_store.replacement_effects.add_one_shot_effect(
            ReplacementEffect::with_matcher(
                entrant,
                alice,
                crate::events::zones::matchers::ThisWouldEnterBattlefieldMatcher,
                ReplacementAction::AsEntersProgram(
                    crate::resolution::ResolutionProgram::from_effects(effects),
                ),
            ),
        );
        let mut dm = crate::decision::SelectFirstDecisionMaker;
        let result = apply_zone_change(
            &mut game,
            entrant,
            Zone::Hand,
            Zone::Graveyard,
            crate::events::cause::EventCause::from_effect(entrant, alice),
            &mut dm,
        ).map(require_plain_committed_zone_receipt);
        assert!(!dm.awaiting_choice());
        assert_eq!(
            game.player(alice).unwrap().life,
            if fails { 20 } else { 22 }
        );
        if fails {
            assert!(matches!(&result, Err(crate::effects::ExecutionError::UnresolvableValue(_))));
            assert_eq!(game.object(entrant).unwrap().zone, Zone::Hand);
            assert!(game.battlefield.is_empty());
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(redirect)
                    .is_some()
            );
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(entry_program)
                    .is_some()
            );
            assert!(game.take_pending_trigger_events().is_empty());
        } else {
            assert!(result.unwrap().is_proceed());
            let entered = game.find_object_by_stable_id(stable).unwrap();
            assert_eq!(game.object(entered).unwrap().zone, Zone::Battlefield);
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(redirect)
                    .is_none()
            );
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(entry_program)
                    .is_none()
            );
            let events = game.take_pending_trigger_events();
            let gains = events
                .iter()
                .filter_map(|event| event.downcast::<crate::events::LifeGainEvent>())
                .collect::<Vec<_>>();
            assert_eq!(gains.len(), 1);
            assert_eq!(gains[0].amount, 2);
        }
    }
}


mod additional_move_owner_contract_tests {
    use crate::effect::{Effect, EffectOutcome, Value};
    use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
    use crate::game_state::GameState;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::replacement::{ReplacementAction, ReplacementEffect};
    use crate::snapshot::ObjectSnapshot;
    use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter};
    use crate::zone::Zone;
    struct Answers { alice: PlayerId, to: Zone, pending: bool, pause: bool, questions: usize, owner: u8, random_before: u64 }
    impl crate::decision::DecisionMaker for Answers {
        fn decide_options(&mut self, _: &GameState, _: &crate::decisions::context::SelectOptionsContext) -> Vec<usize> { vec![0] }
        fn decide_boolean(&mut self, game: &GameState, _: &crate::decisions::context::BooleanContext) -> bool {
            self.questions += 1;
            let originals = match self.to { Zone::Hand => &game.player(self.alice).unwrap().hand, Zone::Exile => &game.exile, Zone::Library => &game.player(self.alice).unwrap().library, _ => unreachable!() };
            assert_eq!(originals.len(), 2, "all original moves precede added programs");
            if self.owner == 5 { assert_eq!(game.irreversible_random_count(), self.random_before, "replacement programs precede the subsequent authored shuffle"); }
            self.pending = self.pause; !self.pause
        }
        fn awaiting_choice(&self) -> bool { self.pending }
    }
    fn run_owner(owner: u8, game: &mut GameState, ctx: &mut ExecutionContext) -> Result<EffectOutcome, ExecutionError> {
        let from = if owner == 2 || owner == 5 { Zone::Graveyard } else { Zone::Battlefield };
        let spec = ChooseSpec::all(ObjectFilter::creature().in_zone(from).owned_by(PlayerFilter::You));
        match owner {
            0 => crate::effects::ReturnToHandEffect::with_spec(spec).execute(game, ctx),
            1 => crate::effects::ExileEffect::all(ObjectFilter::creature().you_control()).execute(game, ctx),
            2 => crate::effects::ReturnFromGraveyardToHandEffect::new(spec, false).execute(game, ctx),
            3 => crate::effects::MoveToLibraryNthFromTopEffect::new(spec, Value::Fixed(2)).execute(game, ctx),
            4 => crate::effects::MoveToLibraryTopOrBottomChoiceEffect::new(spec).execute(game, ctx),
            5 => crate::effects::ShuffleObjectsIntoLibraryEffect::new(spec, PlayerFilter::You).execute(game, ctx),
            _ => unreachable!(),
        }
    }
    fn assert_primary(owner: u8, outcome: &EffectOutcome) {
        if owner < 2 { assert_eq!(outcome.count_or_zero(), 2); }
        else { assert_eq!(outcome.objects().unwrap().len(), 2); }
        if owner == 5 { assert_eq!(outcome.events.iter().filter_map(|e| e.downcast::<crate::events::ShuffleLibraryEvent>()).count(), 1); }
    }
    fn check_owner(owner: u8, mode: u8) {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1);
        let from = if owner == 2 || owner == 5 { Zone::Graveyard } else { Zone::Battlefield };
        let to = if owner == 0 || owner == 2 { Zone::Hand } else if owner == 1 { Zone::Exile } else { Zone::Library };
        let card = crate::card::CardBuilder::new(CardId::new(), "Original move fixture").card_types(vec![crate::types::CardType::Creature]).build();
        let originals = (0..2).map(|_| game.create_object_from_card(&card, alice, from)).collect::<Vec<_>>();
        let tracked = game.object(originals[0]).unwrap().stable_id;
        let source_card = crate::card::CardBuilder::new(CardId::new(), "Move fixture source").card_types(vec![crate::types::CardType::Artifact]).build();
        let source = game.create_object_from_card(&source_card, bob, Zone::Battlefield);
        let parent = ObjectSnapshot::from_object(game.object(source).unwrap(), &game);
        let effects = if mode == 3 { vec![Effect::new(crate::effects::PutCountersEffect::new(crate::object::CounterType::PlusOnePlusOne, 1, ChooseSpec::tagged("it")))] }
            else if mode == 1 { vec![Effect::gain_life(3), Effect::lose_life(Value::X)] }
            else { vec![Effect::gain_life(3), Effect::may(vec![Effect::gain_life(4)])] };
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(source, bob,
            crate::events::zones::matchers::WouldChangeZoneMatcher::new(ObjectFilter::specific(originals[0]), Some(from), Some(to)), ReplacementAction::Additionally(effects)));
        game.take_pending_trigger_events();
        let before_ids = game.next_object_id_counter(); let random_before = game.irreversible_random_count();
        let mut dm = Answers { alice, to, pending: false, pause: mode == 2, questions: 0, owner, random_before };
        let mut ctx = ExecutionContext::new(source, alice, &mut dm); ctx.set_tagged_objects("it", vec![parent.clone()]);
        let result = run_owner(owner, &mut game, &mut ctx);
        if mode == 1 { assert!(matches!(result, Err(ExecutionError::UnresolvableValue(_)))); }
        else {
            let outcome = result.unwrap();
            if mode == 2 { assert!(ctx.decision_maker.awaiting_choice()); assert!(outcome.events.is_empty()); }
            else {
                assert_primary(owner, &outcome);
                if mode == 3 {
                    let arrived = game.find_object_by_stable_id(tracked).unwrap();
                    assert_eq!(game.object(arrived).unwrap().counters.get(&crate::object::CounterType::PlusOnePlusOne), Some(&1));
                    assert!(!game.object(source).unwrap().counters.contains_key(&crate::object::CounterType::PlusOnePlusOne));
                    assert!(outcome.execution_facts.iter().filter_map(|fact| match fact { crate::effect::ExecutionFact::AffectedObjectMemory(memory) => Some(memory.as_slice()), _ => None }).flatten().any(|m| m.object_id == arrived && m.zone == to), "added counter facts reach the owner");
                    let original_arrival_memory = outcome.affected_object_memory().unwrap_or(&[]).iter()
                        .filter(|memory| memory.object_id == arrived && memory.zone == to).count();
                    assert_eq!(original_arrival_memory, usize::from(owner == 2),
                        "the return-to-hand owner records its own arrival once; auxiliary counter memory does not duplicate it");
                    if owner == 2 {
                        let complete_arrival_memory = outcome.execution_facts.iter().filter_map(|fact| match fact {
                            crate::effect::ExecutionFact::AffectedObjectMemory(memory) => Some(memory.as_slice()), _ => None,
                        }).flatten().filter(|memory| memory.object_id == arrived && memory.zone == to).count();
                        assert_eq!(complete_arrival_memory, 2, "both original arrival and auxiliary counter observations are retained");
                    }
                } else {
                    assert_eq!(game.player(bob).unwrap().life, 27);
                    assert_eq!(outcome.events.iter().filter_map(|e| e.downcast::<crate::events::LifeGainEvent>()).map(|e| (e.player,e.amount)).collect::<Vec<_>>(), vec![(bob,3),(bob,4)]);
                    if owner == 5 { assert!(outcome.events.last().unwrap().downcast::<crate::events::ShuffleLibraryEvent>().is_some()); }
                }
            }
        }
        assert_eq!(ctx.source, source); assert_eq!(ctx.controller, alice); assert_eq!(ctx.get_tagged_all("it").unwrap()[0].object_id, parent.object_id); drop(ctx);
        if mode == 1 || mode == 2 {
            assert!(originals.iter().all(|id| game.object(*id).is_some_and(|o| o.zone == from)));
            assert!(game.player(alice).unwrap().library.is_empty()); assert!(game.player(alice).unwrap().hand.is_empty()); assert!(game.exile.is_empty());
            assert_eq!(game.player(bob).unwrap().life, 20); assert_eq!(game.next_object_id_counter(), before_ids); assert_eq!(game.irreversible_random_count(), random_before);
            assert!(game.effect_store.replacement_effects.get_effect(shield).is_some()); assert!(game.take_pending_trigger_events().is_empty());
        } else { assert!(game.effect_store.replacement_effects.get_effect(shield).is_none()); }
        if mode == 2 {
            assert_eq!(dm.questions, 1); let mut dm = Answers { alice, to, pending: false, pause: false, questions: 0, owner, random_before };
            let mut ctx = ExecutionContext::new(source, alice, &mut dm); let outcome = run_owner(owner, &mut game, &mut ctx).unwrap();
            assert!(!ctx.decision_maker.awaiting_choice()); drop(ctx); assert_eq!(dm.questions,1); assert_primary(owner,&outcome); assert_eq!(game.player(bob).unwrap().life,27);
            assert!(game.effect_store.replacement_effects.get_effect(shield).is_none());
            assert_eq!(outcome.events.iter().filter_map(|e| e.downcast::<crate::events::LifeGainEvent>()).map(|e| e.amount).collect::<Vec<_>>(), vec![3,4]);
        }
    }
    #[test] fn additional_return_hand_original_then_payload() { check_owner(0, 0); }
    #[test] fn additional_return_hand_error_rollback() { check_owner(0, 1); }
    #[test] fn additional_return_hand_pending_replay() { check_owner(0, 2); }
    #[test] fn additional_return_hand_arrival_binding() { check_owner(0, 3); }
    #[test] fn additional_exile_original_then_payload() { check_owner(1, 0); }
    #[test] fn additional_exile_error_rollback() { check_owner(1, 1); }
    #[test] fn additional_exile_pending_replay() { check_owner(1, 2); }
    #[test] fn additional_exile_arrival_binding() { check_owner(1, 3); }
    #[test] fn additional_return_graveyard_original_then_payload() { check_owner(2, 0); }
    #[test] fn additional_return_graveyard_error_rollback() { check_owner(2, 1); }
    #[test] fn additional_return_graveyard_pending_replay() { check_owner(2, 2); }
    #[test] fn additional_return_graveyard_arrival_binding() { check_owner(2, 3); }
    #[test] fn additional_library_nth_original_then_payload() { check_owner(3, 0); }
    #[test] fn additional_library_nth_error_rollback() { check_owner(3, 1); }
    #[test] fn additional_library_nth_pending_replay() { check_owner(3, 2); }
    #[test] fn additional_library_nth_arrival_binding() { check_owner(3, 3); }
    #[test] fn additional_library_choice_original_then_payload() { check_owner(4, 0); }
    #[test] fn additional_library_choice_error_rollback() { check_owner(4, 1); }
    #[test] fn additional_library_choice_pending_replay() { check_owner(4, 2); }
    #[test] fn additional_library_choice_arrival_binding() { check_owner(4, 3); }
    #[test] fn additional_shuffle_library_original_then_payload() { check_owner(5, 0); }
    #[test] fn additional_shuffle_library_error_rollback() { check_owner(5, 1); }
    #[test] fn additional_shuffle_library_pending_replay() { check_owner(5, 2); }
    #[test] fn additional_shuffle_library_arrival_binding() { check_owner(5, 3); }
}


mod library_batch_owner_contract_tests {
    use crate::effect::Value;
    use crate::effects::{EffectExecutor, ExecutionContext};
    use crate::game_state::GameState;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::target::{ChooseSpec, ObjectFilter};
    use crate::zone::Zone;
    struct Positions { questions: usize, pause_second: bool, pending: bool }
    impl crate::decision::DecisionMaker for Positions {
        fn decide_options(&mut self, _: &GameState, _: &crate::decisions::context::SelectOptionsContext) -> Vec<usize> {
            self.questions += 1;
            if self.pause_second && self.questions == 2 { self.pending = true; Vec::new() } else { vec![0] }
        }
        fn awaiting_choice(&self) -> bool { self.pending }
    }
    fn setup() -> (GameState, PlayerId, ObjectId, Vec<ObjectId>) {
        let mut game = crate::tests::test_helpers::setup_two_player_game(); let alice = PlayerId::from_index(0);
        let card = crate::card::CardBuilder::new(CardId::new(), "Library original").card_types(vec![crate::types::CardType::Creature]).build();
        let originals = (0..2).map(|_| game.create_object_from_card(&card, alice, Zone::Battlefield)).collect::<Vec<_>>();
        let source_card = crate::card::CardBuilder::new(CardId::new(), "Library source").card_types(vec![crate::types::CardType::Artifact]).build();
        let source = game.create_object_from_card(&source_card, alice, Zone::Battlefield);
        game.take_pending_trigger_events(); (game, alice, source, originals)
    }
    fn check_prevented(choice: bool) {
        let (mut game, alice, source, originals) = setup();
        let second_stable = game.object(originals[1]).unwrap().stable_id;
        game.effect_store.replacement_effects.add_one_shot_effect(crate::replacement::ReplacementEffect::with_matcher(source, alice,
            crate::events::zones::matchers::WouldChangeZoneMatcher::new(ObjectFilter::specific(originals[0]), Some(Zone::Battlefield), Some(Zone::Library)), crate::replacement::ReplacementAction::Prevent));
        let mut dm = Positions { questions: 0, pause_second: false, pending: false }; let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        let spec = ChooseSpec::all(ObjectFilter::creature().you_control());
        let outcome = if choice { crate::effects::MoveToLibraryTopOrBottomChoiceEffect::new(spec).execute(&mut game, &mut ctx) }
            else { crate::effects::MoveToLibraryNthFromTopEffect::new(spec, Value::Fixed(2)).execute(&mut game, &mut ctx) }.unwrap();
        assert_eq!(outcome.objects().map(|ids| ids.len()), Some(1), "preventing one original cannot stop the remaining original moves");
        assert_eq!(game.object(originals[0]).unwrap().zone, Zone::Battlefield);
        let second = game.find_object_by_stable_id(second_stable).unwrap(); assert_eq!(game.object(second).unwrap().zone, Zone::Library);
        assert_eq!(game.player(alice).unwrap().library, vec![second]);
    }
    #[test] fn nth_position_prevention_does_not_abort_remaining_objects() { check_prevented(false); }
    #[test] fn chosen_position_prevention_does_not_abort_remaining_objects() { check_prevented(true); }
    #[test] fn second_position_prompt_restores_all_original_moves_and_replays_once() {
        let (mut game, alice, source, originals) = setup(); let before_ids = game.next_object_id_counter();
        let spec = ChooseSpec::all(ObjectFilter::creature().you_control()); let effect = crate::effects::MoveToLibraryTopOrBottomChoiceEffect::new(spec);
        let mut dm = Positions { questions: 0, pause_second: true, pending: false }; let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        let outcome = effect.execute(&mut game, &mut ctx).unwrap(); assert!(ctx.decision_maker.awaiting_choice()); assert!(outcome.events.is_empty()); drop(ctx);
        assert_eq!(game.next_object_id_counter(), before_ids, "a later position prompt restores earlier moved object IDs");
        assert!(originals.iter().all(|id| game.object(*id).is_some_and(|o| o.zone == Zone::Battlefield)));
        assert!(game.player(alice).unwrap().library.is_empty()); assert!(game.take_pending_trigger_events().is_empty()); assert_eq!(dm.questions,2);
        let mut dm = Positions { questions: 0, pause_second: false, pending: false }; let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        let outcome = effect.execute(&mut game, &mut ctx).unwrap(); assert!(!ctx.decision_maker.awaiting_choice()); drop(ctx);
        assert_eq!(outcome.objects().unwrap().len(),2); assert_eq!(game.player(alice).unwrap().library.len(),2); assert_eq!(dm.questions,2);
        assert!(originals.iter().all(|id| game.object(*id).is_none()));
    }
}
