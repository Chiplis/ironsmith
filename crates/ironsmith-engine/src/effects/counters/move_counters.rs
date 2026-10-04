//! Move counters effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_objects_for_effect, resolve_bounded_nonnegative_u32};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::ChooseSpec;
pub use ironsmith_core::MoveCountersEffect;

impl EffectExecutor for MoveCountersEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }
        game.clear_pending_decision_controllers();
        let checkpoint = game.clone();
        let context_checkpoint = crate::effects::ExecutionContextCheckpoint::capture(ctx);
        let result = (|| {

            // Targeted moves read the two resolved targets; untargeted moves
            // (graft: this permanent onto the entering creature, CR 702.58a)
            // resolve `from`/`to` through their specs.
            let is_reference = |spec: &ChooseSpec| {
                matches!(spec.base(), ChooseSpec::Source | ChooseSpec::Tagged(_))
            };
            let target_pair = if !is_reference(&self.from) && !is_reference(&self.to) {
                super::assigned_counter_transfer_pair(ctx)
            } else {
                let from = match self.from.base() {
                    ChooseSpec::Source => vec![ctx.source],
                    _ => resolve_objects_for_effect(game, ctx, &self.from)?,
                };
                let to = match self.to.base() {
                    ChooseSpec::Source => vec![ctx.source],
                    _ => resolve_objects_for_effect(game, ctx, &self.to)?,
                };
                from.first().copied().zip(to.first().copied())
            };
            let Some((from_id, to_id)) = target_pair else {
                return Ok(EffectOutcome::target_invalid());
            };
            // CR 122.5: nothing is removed if the counters can't be put onto the
            // second object.
            if game.is_phased_out(from_id)
                || from_id == to_id
                || !super::move_destination_can_receive_counters(game, to_id, self.counter_type)
            {
                return Ok(EffectOutcome::count(0));
            }

            // Get current counter count on source
            let available = game
                .object(from_id)
                .and_then(|obj| obj.counters.get(&self.counter_type).copied())
                .unwrap_or(0);

            let to_move = match &self.count {
                ironsmith_core::effect::CounterMoveAmount::Exact(value) => resolve_bounded_nonnegative_u32(game, value, ctx, available)?,
                ironsmith_core::effect::CounterMoveAmount::AnyNumber => {
                    let spec = crate::decisions::NumberSpec::up_to(ctx.source, available,
                        format!("Choose how many {} counters to move", self.counter_type.description()));
                    let chosen = crate::decisions::make_decision_with_fallback(game, &mut ctx.decision_maker,
                        ctx.controller, Some(ctx.source), spec, crate::decision::FallbackStrategy::Maximum);
                    if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
                    chosen
                }
            }.min(available);

            if to_move == 0 {
                return Ok(EffectOutcome::count(0));
            }

            let mut outcome = super::remove_moved_counters(game, ctx, from_id, self.counter_type, to_move)?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
            }

            // Putting the moved counters is an ordinary placement (CR 122.5).
            let placed = super::put_moved_counters(game, ctx, to_id, self.counter_type, to_move)?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
            }
            outcome = EffectOutcome::aggregate([outcome, placed]);
            outcome.set_value(crate::effect::OutcomeValue::Count(i64::from(to_move)));

            Ok(outcome)
        })();
        if result.is_err() || ctx.decision_maker.awaiting_choice() {
            game.restore_execution_checkpoint(checkpoint, result.is_ok() && ctx.decision_maker.awaiting_choice());
            context_checkpoint.restore(ctx);
            if ctx.decision_maker.awaiting_choice() && result.is_ok() {
                return Ok(EffectOutcome::count(0));
            }
        }
        result
    }

    fn get_target_spec(&self) -> Option<&crate::target::ChooseSpec> {
        Some(&self.from)
    }

    fn target_description(&self) -> &'static str {
        "creature to move counters from"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::effects::ResolvedTarget;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::{CounterType, Object};
    use crate::types::CardType;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn make_creature_card(card_id: u32, name: &str) -> crate::card::Card {
        CardBuilder::new(CardId::from_raw(card_id), name)
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Green],
            ]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build()
    }

    fn create_creature_with_counters(
        game: &mut GameState,
        name: &str,
        controller: PlayerId,
        counter_type: CounterType,
        count: u32,
    ) -> ObjectId {
        let id = game.new_object_id();
        let card = make_creature_card(id.0 as u32, name);
        let mut obj = Object::from_card(id, &card, controller, Zone::Battlefield);
        obj.counters.insert(counter_type, count);
        game.add_object(obj);
        id
    }

    fn create_creature(game: &mut GameState, name: &str, controller: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = make_creature_card(id.0 as u32, name);
        let obj = Object::from_card(id, &card, controller, Zone::Battlefield);
        game.add_object(obj);
        id
    }

    #[test]
    fn test_move_counters() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let from_id = create_creature_with_counters(
            &mut game,
            "Source Creature",
            alice,
            CounterType::PlusOnePlusOne,
            5,
        );
        let to_id = create_creature(&mut game, "Target Creature", alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice).with_targets(vec![
            ResolvedTarget::Object(from_id),
            ResolvedTarget::Object(to_id),
        ]);

        let effect = MoveCountersEffect::plus_one_counters(3);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(3));

        let from_obj = game.object(from_id).unwrap();
        assert_eq!(
            from_obj.counters.get(&CounterType::PlusOnePlusOne),
            Some(&2)
        ); // 5 - 3

        let to_obj = game.object(to_id).unwrap();
        assert_eq!(to_obj.counters.get(&CounterType::PlusOnePlusOne), Some(&3));
    }

    #[test]
    fn test_move_counters_limited_by_available() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let from_id = create_creature_with_counters(
            &mut game,
            "Source Creature",
            alice,
            CounterType::PlusOnePlusOne,
            2,
        );
        let to_id = create_creature(&mut game, "Target Creature", alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice).with_targets(vec![
            ResolvedTarget::Object(from_id),
            ResolvedTarget::Object(to_id),
        ]);

        // Request 5 but only 2 available
        let effect = MoveCountersEffect::plus_one_counters(5);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(2)); // Limited by available

        // When all counters are removed, the entry is removed from the HashMap
        assert_eq!(game.counter_count(from_id, CounterType::PlusOnePlusOne), 0);
        assert_eq!(game.counter_count(to_id, CounterType::PlusOnePlusOne), 2);
    }

    #[test]
    fn test_move_counters_no_counters() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let from_id = create_creature(&mut game, "Source Creature", alice);
        let to_id = create_creature(&mut game, "Target Creature", alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice).with_targets(vec![
            ResolvedTarget::Object(from_id),
            ResolvedTarget::Object(to_id),
        ]);

        let effect = MoveCountersEffect::plus_one_counters(3);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(0));
    }

    #[test]
    fn test_move_counters_insufficient_targets() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let from_id = create_creature_with_counters(
            &mut game,
            "Source Creature",
            alice,
            CounterType::PlusOnePlusOne,
            5,
        );
        let source = game.new_object_id();

        // Only one target provided
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(from_id)]);

        let effect = MoveCountersEffect::plus_one_counters(3);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.status, crate::effect::OutcomeStatus::TargetInvalid);
    }

    #[test]
    fn test_move_counters_clone_box() {
        let effect = MoveCountersEffect::plus_one_counters(1);
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("MoveCountersEffect"));
    }
}


#[cfg(test)]
mod move_removal_replacement_owner_tests {
    use super::*;
    fn check_owner(owner:u8,instead:bool) {
        let mut game=crate::tests::test_helpers::setup_two_player_game();
        let alice=crate::ids::PlayerId::from_index(0);
        let definition=crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(),"Move removal source")
            .card_types(vec![crate::types::CardType::Artifact]).build();
        let source=game.create_object_from_definition(&definition,alice,crate::zone::Zone::Battlefield);
        let destination=game.create_object_from_definition(&definition,alice,crate::zone::Zone::Battlefield);
        let kind=crate::object::CounterType::Charge;
        game.object_mut(source).unwrap().counters.insert(kind,3);
        let action=if instead {crate::replacement::ReplacementAction::Instead(vec![crate::effect::Effect::gain_life(2)])}
            else {crate::replacement::ReplacementAction::Prevent};
        let shield=game.effect_store.replacement_effects.add_one_shot_effect(crate::replacement::ReplacementEffect::with_matcher(source,alice,
            crate::events::counters::matchers::WouldRemoveCountersMatcher::new(crate::target::ObjectFilter::permanent(),Some(kind)),action));
        let effect=match owner {
            0=>crate::effect::Effect::new(MoveCountersEffect::new(kind,2,ChooseSpec::Source,ChooseSpec::SpecificObject(destination))),
            1=>crate::effect::Effect::new(crate::effects::MoveAllCountersEffect::new(ChooseSpec::Source,ChooseSpec::SpecificObject(destination))),
            _=>crate::effect::Effect::new(crate::effects::MoveOneCounterEffect::new(ChooseSpec::Source,ChooseSpec::SpecificObject(destination))),
        };
        let mut ctx=ExecutionContext::new_default(source,alice);
        let outcome=crate::effects::execute_effect(&mut game,&effect,&mut ctx).unwrap();
        assert_eq!(game.counter_count(source,kind),3,"the move's removal component must honor its own replacement");
        assert_eq!(game.player(alice).unwrap().life,if instead{22}else{20});
        assert_eq!(outcome.events_of_type::<crate::events::LifeGainEvent>().count(),usize::from(instead));
        assert_eq!(outcome.events_of_type::<crate::events::MarkersChangedEvent>().filter(|event|event.is_removed()).count(),0);
        assert!(game.effect_store.replacement_effects.get_effect(shield).is_none());
        // This reproduces removal bypass and payload loss. First placement
        // quantity/movement receipt coupling remains a separate rules-backed audit.
        let previous_destination_count=game.counter_count(destination,kind);
        let moved=match owner{0=>2,1=>3,_=>1};
        let next=crate::effects::execute_effect(&mut game,&effect,&mut ctx).unwrap();
        assert_eq!(next.count_or_zero(),moved as i64);
        assert_eq!(game.counter_count(source,kind),3-moved);
        assert_eq!(game.counter_count(destination,kind),previous_destination_count+moved);
        assert_eq!(next.events_of_type::<crate::events::MarkersChangedEvent>().count(),2);
        assert_eq!(next.events_of_type::<crate::events::LifeGainEvent>().count(),0);
        assert_eq!(game.player(alice).unwrap().life,if instead{22}else{20});
    }
    #[test]fn move_counters_honors_removal_prevention(){check_owner(0,false);}
    #[test]fn move_counters_executes_removal_instead_payload(){check_owner(0,true);}
    #[test]fn move_all_counters_honors_removal_prevention(){check_owner(1,false);}
    #[test]fn move_all_counters_executes_removal_instead_payload(){check_owner(1,true);}
    #[test]fn move_one_counter_honors_removal_prevention(){check_owner(2,false);}
    #[test]fn move_one_counter_executes_removal_instead_payload(){check_owner(2,true);}
}

#[cfg(test)]
mod self_move_counter_prohibition_tests {
    use super::*;
    fn check(owner: u8, history_first: bool) {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = crate::ids::PlayerId::from_index(0);
        let definition = crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Self move source")
            .card_types(vec![crate::types::CardType::Artifact]).build();
        let source = game.create_object_from_definition(&definition, alice, crate::zone::Zone::Battlefield);
        game.object_mut(source).unwrap().counters.insert(crate::object::CounterType::Charge, 3);
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(
            crate::replacement::ReplacementEffect::with_matcher(source, alice,
                crate::events::counters::matchers::WouldPutCountersMatcher::any(),
                crate::replacement::ReplacementAction::Modify(crate::replacement::EventModification::Multiply(2))));
        let effect = match owner {
            0 => crate::effect::Effect::new(crate::effects::MoveOneCounterEffect::new(ChooseSpec::Source, ChooseSpec::Source)),
            _ => crate::effect::Effect::new(crate::effects::MoveAllCountersEffect::new(ChooseSpec::Source, ChooseSpec::Source)),
        };
        let mut ctx = ExecutionContext::new_default(source, alice);
        let outcome = crate::effects::execute_effect(&mut game, &effect, &mut ctx).unwrap();
        if history_first { assert!(outcome.events.is_empty(), "an impossible self move cannot publish removal or placement observations"); }
        assert_eq!(game.counter_count(source, crate::object::CounterType::Charge), 3,
            "a same-object move cannot remove and re-add counters through a multiplier");
        assert_eq!(outcome.count_or_zero(), 0);
        assert!(outcome.events.is_empty());
        assert_eq!(game.turn_store.turn_history.event_kind_count(crate::events::EventKind::MarkersChanged), 0);
        assert!(game.effect_store.replacement_effects.get_effect(shield).is_some(), "no counter event means no one-shot consumption");
    }
    #[test] fn move_one_to_itself_cannot_duplicate_counters() { check(0, false); }
    #[test] fn move_one_to_itself_has_no_observations() { check(0, true); }
    #[test] fn move_all_to_itself_cannot_duplicate_counters() { check(1, false); }
    #[test] fn move_all_to_itself_has_no_observations() { check(1, true); }
}

#[cfg(test)]
mod phased_source_move_counter_tests {
    use super::*;
    fn check(history_first: bool) {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = crate::ids::PlayerId::from_index(0);
        let definition = crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Phased movement source")
            .card_types(vec![crate::types::CardType::Artifact]).build();
        let source = game.create_object_from_definition(&definition, alice, crate::zone::Zone::Battlefield);
        let destination = game.create_object_from_definition(&definition, alice, crate::zone::Zone::Battlefield);
        let kind = crate::object::CounterType::Charge;
        game.object_mut(source).unwrap().counters.insert(kind, 3);
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(
            crate::replacement::ReplacementEffect::with_matcher(destination, alice,
                crate::events::counters::matchers::WouldPutCountersMatcher::any(),
                crate::replacement::ReplacementAction::Modify(crate::replacement::EventModification::Multiply(2))));
        game.phase_out(source);
        assert!(game.is_phased_out(source));
        let effect = crate::effect::Effect::new(MoveCountersEffect::new(kind, 2,
            ChooseSpec::Source, ChooseSpec::SpecificObject(destination)));
        let mut ctx = ExecutionContext::new_default(source, alice);
        let outcome = crate::effects::execute_effect(&mut game, &effect, &mut ctx).unwrap();
        if history_first { assert!(outcome.events.is_empty(), "a move with an absent phased source cannot publish placement events"); }
        assert_eq!(game.counter_count(destination, kind), 0,
            "ordinary effects cannot create moved counters from a phased-out source");
        assert_eq!(game.counter_count(source, kind), 3);
        assert_eq!(outcome.count_or_zero(), 0);
        assert!(outcome.events.is_empty());
        assert!(game.effect_store.replacement_effects.get_effect(shield).is_some());
        game.phase_in(source);
        let next = crate::effects::execute_effect(&mut game, &effect, &mut ctx).unwrap();
        assert_eq!(next.count_or_zero(), 2);
        assert_eq!(game.counter_count(source, kind), 1);
        assert_eq!(game.counter_count(destination, kind), 4);
        assert_eq!(next.events_of_type::<crate::events::MarkersChangedEvent>().count(), 2);
        assert!(game.effect_store.replacement_effects.get_effect(shield).is_none());
    }
    #[test] fn phased_source_move_cannot_create_destination_counters() { check(false); }
    #[test] fn phased_source_move_cannot_publish_or_consume_placement() { check(true); }
}

#[cfg(test)]
mod phased_move_owner_parity_tests {
    use super::*;
    fn check(owner: u8, destination_phased: bool) {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = crate::ids::PlayerId::from_index(0);
        let definition = crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Phased move parity")
            .card_types(vec![crate::types::CardType::Artifact]).build();
        let source = game.create_object_from_definition(&definition, alice, crate::zone::Zone::Battlefield);
        let destination = game.create_object_from_definition(&definition, alice, crate::zone::Zone::Battlefield);
        let sponsor = game.create_object_from_definition(&definition, alice, crate::zone::Zone::Battlefield);
        let kind = crate::object::CounterType::Charge;
        game.object_mut(source).unwrap().counters.insert(kind, 3);
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(
            crate::replacement::ReplacementEffect::with_matcher(sponsor, alice,
                crate::events::counters::matchers::WouldPutCountersMatcher::any(),
                crate::replacement::ReplacementAction::Modify(crate::replacement::EventModification::Multiply(2))));
        let phased = if destination_phased { destination } else { source };
        game.phase_out(phased);
        let effect = match owner {
            0 => crate::effect::Effect::new(MoveCountersEffect::new(kind, 2, ChooseSpec::Source, ChooseSpec::SpecificObject(destination))),
            1 => crate::effect::Effect::new(crate::effects::MoveOneCounterEffect::new(ChooseSpec::Source, ChooseSpec::SpecificObject(destination))),
            _ => crate::effect::Effect::new(crate::effects::MoveAllCountersEffect::new(ChooseSpec::Source, ChooseSpec::SpecificObject(destination))),
        };
        struct Choices(usize);
        impl crate::decision::DecisionMaker for Choices {
            fn decide_counters(&mut self, _game: &GameState, ctx: &crate::decisions::context::CountersContext) -> Vec<(crate::object::CounterType, u32)> {
                self.0 += 1;
                vec![(crate::object::CounterType::Charge, u32::try_from(ctx.max_total.min(1)).unwrap())]
            }
        }
        let mut choices = Choices(0);
        let outcome = {
            let mut ctx = ExecutionContext::new(source, alice, &mut choices);
            crate::effects::execute_effect(&mut game, &effect, &mut ctx).unwrap()
        };
        assert_eq!(game.counter_count(source, kind), 3, "an absent movement endpoint cannot remove source counters");
        assert_eq!(game.counter_count(destination, kind), 0);
        assert_eq!(outcome.count_or_zero(), 0);
        assert!(outcome.events.is_empty());
        assert!(game.effect_store.replacement_effects.get_effect(shield).is_some());
        assert_eq!(choices.0, 0, "an absent endpoint cannot require a counter choice");
        game.phase_in(phased);
        let next = {
            let mut ctx = ExecutionContext::new(source, alice, &mut choices);
            crate::effects::execute_effect(&mut game, &effect, &mut ctx).unwrap()
        };
        let moved = match owner { 0 => 2, 1 => 1, _ => 3 };
        assert_eq!(next.count_or_zero(), moved);
        assert_eq!(game.counter_count(source, kind), 3 - moved as u32);
        assert_eq!(game.counter_count(destination, kind), 2 * moved as u32);
        assert_eq!(next.events_of_type::<crate::events::MarkersChangedEvent>().count(), 2);
        assert_eq!(choices.0, usize::from(owner == 1));
        assert!(game.effect_store.replacement_effects.get_effect(shield).is_none());
    }
    #[test] fn fixed_move_ignores_phased_destination() { check(0, true); }
    #[test] fn one_move_ignores_phased_destination() { check(1, true); }
    #[test] fn all_move_ignores_phased_destination() { check(2, true); }
    #[test] fn one_move_does_not_choose_from_phased_source() { check(1, false); }
    #[test] fn all_move_phased_source_control() { check(2, false); }
}

#[cfg(test)]
mod move_component_application_tests {
    use super::*;
    use crate::ids::{CardId, PlayerId};
    use crate::object::CounterType;
    use crate::replacement::{EventModification, ReplacementAction, ReplacementEffect};
    fn setup(owner: u8, counters: u32) -> (GameState, crate::ids::ObjectId, crate::ids::ObjectId, crate::effect::Effect, u32) {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let definition = crate::cards::CardDefinitionBuilder::new(CardId::new(), "Move component fixture")
            .card_types(vec![crate::types::CardType::Artifact]).build();
        let source = game.create_object_from_definition(&definition, alice, crate::zone::Zone::Battlefield);
        let destination = game.create_object_from_definition(&definition, alice, crate::zone::Zone::Battlefield);
        game.object_mut(source).unwrap().counters.insert(CounterType::Charge, counters);
        let (effect, budget) = match owner {
            0 => (crate::effect::Effect::new(MoveCountersEffect::new(CounterType::Charge, 2, ChooseSpec::Source, ChooseSpec::SpecificObject(destination))), 2),
            1 => (crate::effect::Effect::new(crate::effects::MoveAllCountersEffect::new(ChooseSpec::Source, ChooseSpec::SpecificObject(destination))), counters),
            _ => (crate::effect::Effect::new(crate::effects::MoveOneCounterEffect::new(ChooseSpec::Source, ChooseSpec::SpecificObject(destination))), 1),
        };
        (game, source, destination, effect, budget)
    }
    fn replacement(game: &mut GameState, source: crate::ids::ObjectId, action: ReplacementAction) -> crate::replacement::ReplacementEffectId {
        game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(source, PlayerId::from_index(0),
            crate::events::counters::matchers::WouldRemoveCountersMatcher::new(crate::target::ObjectFilter::permanent(), Some(CounterType::Charge)), action))
    }
    fn quantity(owner: u8) {
        let (mut game, source, destination, effect, budget) = setup(owner, 5);
        let removed = if owner == 2 { 0 } else { 1 };
        let shield = replacement(&mut game, source, ReplacementAction::Modify(EventModification::SetTo(removed)));
        let put_shield = game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(source, PlayerId::from_index(0),
            crate::events::counters::matchers::WouldPutCountersMatcher::any(), ReplacementAction::Modify(EventModification::Multiply(2))));
        let mut ctx = ExecutionContext::new_default(source, PlayerId::from_index(0));
        let out = crate::effects::execute_effect(&mut game, &effect, &mut ctx).unwrap();
        assert_eq!(game.counter_count(source, CounterType::Charge), 5 - removed, "resolved removal quantity must be committed");
        // CR 122.5 decomposes the instruction into removal and placement.
        // Replacing one component does not rewrite the other's authored quantity.
        assert_eq!(game.counter_count(destination, CounterType::Charge), 2 * budget);
        assert_eq!(out.count_or_zero(), budget as i64);
        assert!(game.effect_store.replacement_effects.get_effect(shield).is_none());
        assert!(game.effect_store.replacement_effects.get_effect(put_shield).is_none());
        let markers = out.events_of_type::<crate::events::MarkersChangedEvent>().collect::<Vec<_>>();
        assert_eq!(markers.iter().filter(|e| e.is_removed()).map(|e| e.amount).sum::<u32>(), removed);
        assert_eq!(markers.iter().filter(|e| e.is_added()).map(|e| e.amount).sum::<u32>(), 2 * budget);
        assert_eq!(markers.len(), if removed == 0 { 1 } else { 2 });
    }
    struct Answers { pause: bool, pending: bool, calls: usize }
    impl crate::decision::DecisionMaker for Answers {
        fn decide_counters(&mut self, _game: &GameState, ctx: &crate::decisions::context::CountersContext) -> Vec<(CounterType, u32)> {
            assert_eq!(ctx.min_total, 1);
            assert_eq!(ctx.max_total, 1);
            vec![(CounterType::Charge, 1)]
        }
        fn decide_boolean(&mut self, _game: &GameState, _ctx: &crate::decisions::context::BooleanContext) -> bool {
            self.calls += 1; self.pending = self.pause; !self.pending
        }
        fn awaiting_choice(&self) -> bool { self.pending }
    }
    fn pending(owner: u8) {
        let (mut game, source, destination, effect, budget) = setup(owner, 3);
        let shield = replacement(&mut game, source, ReplacementAction::Instead(vec![crate::effect::Effect::may(vec![crate::effect::Effect::gain_life(2)])]));
        let mut answers = Answers { pause: true, pending: false, calls: 0 };
        let out = { let mut ctx = ExecutionContext::new(source, PlayerId::from_index(0), &mut answers); crate::effects::execute_effect(&mut game, &effect, &mut ctx).unwrap() };
        assert!(answers.pending, "removal replacement choice must suspend the move");
        assert_eq!(out.count_or_zero(), 0); assert!(out.events.is_empty());
        assert_eq!(game.counter_count(source, CounterType::Charge), 3);
        assert_eq!(game.counter_count(destination, CounterType::Charge), 0);
        assert_eq!(game.player(PlayerId::from_index(0)).unwrap().life, 20);
        assert!(game.effect_store.replacement_effects.get_effect(shield).is_some());
        answers.pause = false; answers.pending = false;
        let out = { let mut ctx = ExecutionContext::new(source, PlayerId::from_index(0), &mut answers); crate::effects::execute_effect(&mut game, &effect, &mut ctx).unwrap() };
        assert_eq!(answers.calls, 2);
        assert_eq!(game.counter_count(source, CounterType::Charge), 3);
        assert_eq!(game.counter_count(destination, CounterType::Charge), budget);
        assert_eq!(game.player(PlayerId::from_index(0)).unwrap().life, 22);
        assert_eq!(out.count_or_zero(), budget as i64);
        assert_eq!(out.events_of_type::<crate::events::LifeGainEvent>().count(), 1);
        assert_eq!(out.events_of_type::<crate::events::MarkersChangedEvent>().count(), 1);
        assert!(game.effect_store.replacement_effects.get_effect(shield).is_none());
    }
    #[derive(Debug, Clone)] struct Fail;
    impl crate::effects::EffectExecutor for Fail {
        fn execute(&self, _game: &mut GameState, _ctx: &mut ExecutionContext) -> Result<EffectOutcome, ExecutionError> {
            Err(ExecutionError::InternalError("injected move removal failure".into()))
        }
    }
    fn error(owner: u8) {
        let (mut game, source, destination, effect, _) = setup(owner, 3);
        let shield = replacement(&mut game, source, ReplacementAction::Instead(vec![crate::effect::Effect::gain_life(2), crate::effect::Effect::new(Fail)]));
        let mut ctx = ExecutionContext::new_default(source, PlayerId::from_index(0));
        let result = crate::effects::execute_effect(&mut game, &effect, &mut ctx);
        assert!(matches!(result, Err(ExecutionError::InternalError(ref s)) if s == "injected move removal failure"), "replacement payload failure must propagate");
        assert_eq!(game.player(PlayerId::from_index(0)).unwrap().life, 20);
        assert_eq!(game.counter_count(source, CounterType::Charge), 3);
        assert_eq!(game.counter_count(destination, CounterType::Charge), 0);
        assert!(game.effect_store.replacement_effects.get_effect(shield).is_some());
    }
    #[test]
    fn single_counter_move_requires_one_counter_in_its_prompt() {
        let (mut game, source, destination, effect, _) = setup(2, 3);
        struct PickOne { calls: usize }
        impl crate::decision::DecisionMaker for PickOne {
            fn decide_counters(&mut self, _game: &GameState, ctx: &crate::decisions::context::CountersContext) -> Vec<(CounterType, u32)> {
                self.calls += 1;
                assert_eq!(ctx.min_total, 1, "a mandatory one-counter move cannot offer a zero-counter choice");
                assert_eq!(ctx.max_total, 1);
                vec![(CounterType::Charge, 1)]
            }
        }
        let mut answers = PickOne { calls: 0 };
        let out = { let mut ctx = ExecutionContext::new(source, PlayerId::from_index(0), &mut answers); crate::effects::execute_effect(&mut game, &effect, &mut ctx).unwrap() };
        assert_eq!(answers.calls, 1);
        assert_eq!(out.count_or_zero(), 1);
        assert_eq!(game.counter_count(source, CounterType::Charge), 2);
        assert_eq!(game.counter_count(destination, CounterType::Charge), 1);
        assert_eq!(out.events_of_type::<crate::events::MarkersChangedEvent>().count(), 2);
    }
    #[test] fn fixed_move_respects_independent_component_quantities() { quantity(0); }
    #[test] fn all_move_respects_independent_component_quantities() { quantity(1); }
    #[test] fn one_move_respects_independent_component_quantities() { quantity(2); }
    #[test] fn fixed_move_suspends_and_retries_removal_replacement() { pending(0); }
    #[test] fn all_move_suspends_and_retries_removal_replacement() { pending(1); }
    #[test] fn one_move_suspends_and_retries_removal_replacement() { pending(2); }
    #[test] fn fixed_move_propagates_removal_payload_failure() { error(0); }
    #[test] fn all_move_propagates_removal_payload_failure() { error(1); }
    #[test] fn one_move_propagates_removal_payload_failure() { error(2); }
}

#[cfg(test)]
mod chosen_counter_move_amount_tests {
    use super::*;
    use crate::object::CounterType;
    struct Answers { chosen: u32, pause: bool, pending: bool, calls: usize, bounds: Vec<(u32,u32)> }
    impl crate::decision::DecisionMaker for Answers {
        fn decide_number(&mut self, _game: &GameState, ctx: &crate::decisions::context::NumberContext) -> u32 {
            self.calls += 1; self.bounds.push((ctx.min,ctx.max)); self.pending=self.pause; self.chosen
        }
        fn awaiting_choice(&self) -> bool {self.pending}
    }
    fn setup(available:u32) -> (GameState,crate::ids::ObjectId,crate::ids::ObjectId,crate::effect::Effect) {
        let mut game=crate::tests::test_helpers::setup_two_player_game();
        let alice=crate::ids::PlayerId::from_index(0);
        let definition=crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(),"Chosen counter transfer")
            .card_types(vec![crate::types::CardType::Artifact]).build();
        let from=game.create_object_from_definition(&definition,alice,crate::zone::Zone::Battlefield);
        let to=game.create_object_from_definition(&definition,alice,crate::zone::Zone::Battlefield);
        game.object_mut(from).unwrap().counters.insert(CounterType::Charge,available);
        let effect=crate::effect::Effect::new(MoveCountersEffect::any_number(CounterType::Charge,ChooseSpec::Source,ChooseSpec::SpecificObject(to)));
        (game,from,to,effect)
    }
    fn run(game:&mut GameState,source:crate::ids::ObjectId,effect:&crate::effect::Effect,answers:&mut Answers)->EffectOutcome {
        let mut ctx=ExecutionContext::new(source,crate::ids::PlayerId::from_index(0),answers);
        crate::effects::execute_effect(game,effect,&mut ctx).unwrap()
    }
    #[test]
    fn any_number_move_chooses_zero_subset_or_all() {
        for chosen in [0,1,4] {
            let (mut game,from,to,effect)=setup(4);
            let mut answers=Answers{chosen,pause:false,pending:false,calls:0,bounds:vec![]};
            let out=run(&mut game,from,&effect,&mut answers);
            assert_eq!(answers.bounds,vec![(0,4)]);
            assert_eq!(game.counter_count(from,CounterType::Charge),4-chosen);
            assert_eq!(game.counter_count(to,CounterType::Charge),chosen);
            assert_eq!(out.count_or_zero(),chosen as i64);
            assert_eq!(out.events_of_type::<crate::events::MarkersChangedEvent>().count(),if chosen==0{0}else{2});
        }
    }
    #[test]
    fn any_number_move_pending_choice_keeps_both_endpoints_and_shield() {
        let (mut game,from,to,effect)=setup(4);
        let shield=game.effect_store.replacement_effects.add_one_shot_effect(crate::replacement::ReplacementEffect::with_matcher(from,crate::ids::PlayerId::from_index(0),
            crate::events::counters::matchers::WouldPutCountersMatcher::any(),crate::replacement::ReplacementAction::Modify(crate::replacement::EventModification::Multiply(2))));
        let mut answers=Answers{chosen:1,pause:true,pending:false,calls:0,bounds:vec![]};
        let out=run(&mut game,from,&effect,&mut answers);
        assert!(answers.pending);assert!(out.events.is_empty());
        assert_eq!(game.counter_count(from,CounterType::Charge),4);assert_eq!(game.counter_count(to,CounterType::Charge),0);
        assert!(game.effect_store.replacement_effects.get_effect(shield).is_some());
        answers.pause=false;answers.pending=false;
        let out=run(&mut game,from,&effect,&mut answers);
        assert_eq!(answers.calls,2);assert_eq!(out.count_or_zero(),1);
        assert_eq!(game.counter_count(from,CounterType::Charge),3);assert_eq!(game.counter_count(to,CounterType::Charge),2);
        assert!(game.effect_store.replacement_effects.get_effect(shield).is_none());
        assert_eq!(out.events_of_type::<crate::events::MarkersChangedEvent>().count(),2);
    }
    #[test]
    fn any_number_choice_bound_preserves_unsigned_available_counter_count() {
        let (mut game,from,to,effect)=setup(u32::MAX);
        let mut answers=Answers{chosen:1,pause:false,pending:false,calls:0,bounds:vec![]};
        run(&mut game,from,&effect,&mut answers);
        assert_eq!(answers.bounds,vec![(0,u32::MAX)]);
        assert_eq!(game.counter_count(from,CounterType::Charge),u32::MAX-1);assert_eq!(game.counter_count(to,CounterType::Charge),1);
    }
    #[test]
    fn any_number_move_absent_endpoint_does_not_ask_or_consume() {
        for destination in [false,true] {
            let (mut game,from,to,effect)=setup(4);game.phase_out(if destination{to}else{from});
            let mut answers=Answers{chosen:1,pause:false,pending:false,calls:0,bounds:vec![]};
            let out=run(&mut game,from,&effect,&mut answers);
            assert_eq!(answers.calls,0);assert!(out.events.is_empty());
            assert_eq!(game.counter_count(from,CounterType::Charge),4);assert_eq!(game.counter_count(to,CounterType::Charge),0);
        }
    }

    #[test]
    fn any_number_all_preserves_unsigned_quantity_in_movement_receipt() {
        for available in [i32::MAX as u32, i32::MAX as u32 + 1, u32::MAX] {
            let (mut game,from,to,effect)=setup(available);
            let mut answers=Answers{chosen:available,pause:false,pending:false,calls:0,bounds:vec![]};
            let out=run(&mut game,from,&effect,&mut answers);
            assert_eq!(answers.bounds,vec![(0,available)]);
            assert_eq!(game.counter_count(from,CounterType::Charge),0);
            assert_eq!(game.counter_count(to,CounterType::Charge),available);
            assert_eq!(out.events_of_type::<crate::events::MarkersChangedEvent>().count(),2);
            assert_eq!(i64::from(out.count_or_zero()),i64::from(available),
                "movement receipt must preserve the unsigned quantity actually chosen and moved");
        }
    }
}
