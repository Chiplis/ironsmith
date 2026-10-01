//! Add mana effect implementation.

use super::choice_helpers::{credit_mana_symbols_from_context, mana_added_value_outcome};
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::mana::ManaSymbol;
pub use ironsmith_core::AddManaEffect;

/// Effect that adds specific mana symbols to a player's mana pool.
///
/// # Fields
///
/// * `mana` - The mana symbols to add
/// * `player` - Which player receives the mana
///
/// # Example
///
/// ```ignore
/// // Add two green mana
/// let effect = AddManaEffect::new(vec![ManaSymbol::Green, ManaSymbol::Green], PlayerFilter::You);
/// ```
impl EffectExecutor for AddManaEffect {
    fn directly_produces_mana(&self) -> bool {
        !self.mana.is_empty()
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;

        let mana =
            credit_mana_symbols_from_context(game, player_id, self.mana.iter().copied(), ctx)?;

        Ok(mana_added_value_outcome(ctx, player_id, mana))
    }

    fn producible_mana_symbols(
        &self,
        _game: &GameState,
        _source: crate::ids::ObjectId,
        _controller: crate::ids::PlayerId,
    ) -> Option<Vec<ManaSymbol>> {
        Some(self.mana.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::PlayerId;
    use crate::target::PlayerFilter;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    #[test]
    fn test_add_mana_single_color() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = AddManaEffect::you(vec![ManaSymbol::Green]);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(
            result.value,
            crate::effect::OutcomeValue::ManaAdded(vec![ManaSymbol::Green])
        );
        assert_eq!(result.events.len(), 1);
        let event = result.events[0]
            .downcast::<crate::events::ManaAddedEvent>()
            .expect("add mana should emit a ManaAddedEvent");
        assert_eq!(event.source, source);
        assert_eq!(event.controller, alice);
        assert_eq!(event.player, alice);
        assert_eq!(event.mana, vec![ManaSymbol::Green]);
        assert_eq!(game.player(alice).unwrap().mana_pool.green, 1);
    }

    #[test]
    fn test_add_mana_multiple_same_color() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = AddManaEffect::you(vec![ManaSymbol::Green, ManaSymbol::Green]);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(
            result.value,
            crate::effect::OutcomeValue::ManaAdded(vec![ManaSymbol::Green, ManaSymbol::Green])
        );
        assert_eq!(game.player(alice).unwrap().mana_pool.green, 2);
    }

    #[test]
    fn test_add_mana_multiple_colors() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = AddManaEffect::you(vec![ManaSymbol::Red, ManaSymbol::Blue, ManaSymbol::Green]);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(
            result.value,
            crate::effect::OutcomeValue::ManaAdded(vec![
                ManaSymbol::Red,
                ManaSymbol::Blue,
                ManaSymbol::Green
            ])
        );
        assert_eq!(game.player(alice).unwrap().mana_pool.red, 1);
        assert_eq!(game.player(alice).unwrap().mana_pool.blue, 1);
        assert_eq!(game.player(alice).unwrap().mana_pool.green, 1);
    }

    #[test]
    fn test_add_mana_to_opponent() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = AddManaEffect::new(vec![ManaSymbol::White], PlayerFilter::Specific(bob));
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(
            result.value,
            crate::effect::OutcomeValue::ManaAdded(vec![ManaSymbol::White])
        );
        assert_eq!(game.player(alice).unwrap().mana_pool.white, 0);
        assert_eq!(game.player(bob).unwrap().mana_pool.white, 1);
    }

    #[test]
    fn test_add_mana_empty() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = AddManaEffect::you(vec![]);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::ManaAdded(vec![]));
    }

    #[test]
    fn test_add_mana_clone_box() {
        let effect = AddManaEffect::you(vec![ManaSymbol::Black]);
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("AddManaEffect"));
    }
}


#[cfg(test)]
mod replacement_contract_tests {
    use super::*;
    use crate::effect::{Effect, Value};
    use crate::ids::{CardId, PlayerId};
    use crate::replacement::{ReplacementAction, ReplacementEffect};
    use crate::target::ObjectFilter;
    use crate::zone::Zone;

    struct PauseManaPayload { pause: bool, pending: bool, questions: usize }
    impl crate::decision::DecisionMaker for PauseManaPayload {
        fn decide_boolean(&mut self, _game: &GameState, _ctx: &crate::decisions::context::BooleanContext) -> bool {
            self.questions += 1;
            if self.pause { self.pending = true; false } else { true }
        }
        fn awaiting_choice(&self) -> bool { self.pending }
    }

    fn check_mana_replacement_owner(temporary: bool, mode: u8) {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let definition = crate::card::CardBuilder::new(CardId::new(), "Mana contract source")
            .card_types(vec![crate::types::CardType::Artifact]).build();
        let source = game.create_object_from_card(&definition, alice, Zone::Battlefield);
        let replacement_source = game.create_object_from_card(&definition, bob, Zone::Battlefield);
        let mut effects = vec![Effect::gain_life(2)];
        if mode == 1 { effects.push(Effect::lose_life(Value::X)); }
        if mode == 2 { effects.push(Effect::may(vec![Effect::gain_life(4)])); }
        effects.push(Effect::gain_life(8));
        let replacement = ReplacementEffect::with_matcher(replacement_source, bob,
            crate::events::mana::matchers::ManaProducedBySourceMatcher::new(ObjectFilter::default()),
            ReplacementAction::Instead(effects));
        let shield = if temporary { None } else { Some(game.effect_store.replacement_effects.add_one_shot_effect(replacement.clone())) };
        game.take_pending_trigger_events();
        let before_objects = game.objects_in_deterministic_order().len();
        let before_id = game.next_object_id_counter();
        let effect = AddManaEffect::you(vec![ManaSymbol::Green]);
        let mut dm = PauseManaPayload { pause: mode == 2, pending: false, questions: 0 };
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);
        if temporary { ctx.replacement.additional_replacement_effects.push(replacement.clone()); }
        let result = effect.execute(&mut game, &mut ctx);
        if mode == 1 {
            assert!(matches!(result, Err(ExecutionError::UnresolvableValue(_))), "replacement errors must propagate");
        } else {
            let outcome = result.expect("replacement execution must succeed or suspend");
            if mode == 2 {
                assert!(ctx.decision_maker.awaiting_choice(), "replacement payload must expose its pending choice");
                assert!(outcome.events.is_empty());
            } else {
                assert!(!ctx.decision_maker.awaiting_choice());
                assert_eq!(game.player(bob).unwrap().life, 30, "payload uses captured replacement controller");
                let mut events = game.take_pending_trigger_events();events.extend(outcome.events);
                let gains = events.iter().filter_map(|event| event.downcast::<crate::events::LifeGainEvent>()).collect::<Vec<_>>();
                assert_eq!(gains.iter().map(|gain| gain.amount).collect::<Vec<_>>(), vec![2,8]);
                assert!(gains.iter().all(|gain| gain.player == bob));
                assert!(!events.iter().any(|event| event.downcast::<crate::events::ManaAddedEvent>().is_some()));
            }
        }
        assert_eq!(ctx.source, source);assert_eq!(ctx.controller, alice);
        assert!(ctx.replacement.suppressed_replacement_effects.is_empty());
        assert!(ctx.replacement.suppressed_replacement_effect_keys.is_empty());
        if temporary { assert_eq!(ctx.replacement.additional_replacement_effects.len(), 1); }
        drop(ctx);
        assert_eq!(game.player(alice).unwrap().life,20);
        assert_eq!(game.player(alice).unwrap().mana_pool.green,0);
        assert_eq!(game.player(bob).unwrap().mana_pool.green,0);
        assert_eq!(game.objects_in_deterministic_order().len(),before_objects);
        assert_eq!(game.next_object_id_counter(),before_id);
        if mode != 0 {
            assert_eq!(game.player(bob).unwrap().life,20);
            assert!(game.take_pending_trigger_events().is_empty());
        }
        if let Some(shield) = shield { assert_eq!(game.effect_store.replacement_effects.get_effect(shield).is_some(), mode != 0); }
        if mode == 2 {
            assert_eq!(dm.questions,1);
            let mut replay = PauseManaPayload { pause:false, pending:false, questions:0 };
            let mut ctx = ExecutionContext::new_default(source,alice).with_decision_maker(&mut replay);
            if temporary { ctx.replacement.additional_replacement_effects.push(replacement); }
            let outcome = effect.execute(&mut game,&mut ctx).unwrap();
            assert!(!ctx.decision_maker.awaiting_choice());drop(ctx);
            assert_eq!(replay.questions,1);
            assert_eq!(game.player(bob).unwrap().life,34);
            assert_eq!(game.player(alice).unwrap().mana_pool.green,0);
            if let Some(shield) = shield { assert!(game.effect_store.replacement_effects.get_effect(shield).is_none()); }
            let mut events = game.take_pending_trigger_events();events.extend(outcome.events);
            assert_eq!(events.iter().filter_map(|event| event.downcast::<crate::events::LifeGainEvent>()).map(|gain| gain.amount).collect::<Vec<_>>(),vec![2,4,8]);
            assert!(!events.iter().any(|event| event.downcast::<crate::events::ManaAddedEvent>().is_some()));
        }
    }
    #[test]
    fn mana_persistent_instead_uses_captured_controller_and_receipt() { check_mana_replacement_owner(false,0); }
    #[test]
    fn mana_temporary_instead_keeps_parent_scope() { check_mana_replacement_owner(true,0); }
    #[test]
    fn mana_instead_error_restores_whole_owner() { check_mana_replacement_owner(false,1); }
    #[test]
    fn mana_instead_pending_replays_once() { check_mana_replacement_owner(false,2); }

    fn check_additional_mana_owner(mode: u8) {
        let mut game=crate::tests::test_helpers::setup_two_player_game();
        let alice=PlayerId::from_index(0);let bob=PlayerId::from_index(1);
        let definition=crate::card::CardBuilder::new(CardId::new(),"Additional mana owner probe")
            .card_types(vec![crate::types::CardType::Artifact]).build();
        let source=game.create_object_from_card(&definition,alice,Zone::Battlefield);
        let replacement_source=game.create_object_from_card(&definition,bob,Zone::Battlefield);
        let effects=if mode==3 { vec![Effect::add_mana(vec![ManaSymbol::White])] } else {
            let mut effects=vec![Effect::gain_life(2)];
            if mode==1 { effects.push(Effect::lose_life(Value::X)); }
            if mode==2 { effects.push(Effect::may(vec![Effect::gain_life(4)])); }
            effects.push(Effect::gain_life(8));effects
        };
        let replacement=ReplacementEffect::with_matcher(replacement_source,bob,
            crate::events::mana::matchers::ManaProducedBySourceMatcher::new(ObjectFilter::default()),ReplacementAction::Additionally(effects));
        let shield=if mode==3 { game.effect_store.replacement_effects.add_effect(replacement) }
            else { game.effect_store.replacement_effects.add_one_shot_effect(replacement) };
        game.take_pending_trigger_events();let before_id=game.next_object_id_counter();
        let effect=AddManaEffect::you(vec![ManaSymbol::Green]);
        let mut dm=PauseManaPayload { pause:mode==2,pending:false,questions:0 };
        let mut ctx=ExecutionContext::new_default(source,alice).with_decision_maker(&mut dm);
        let result=effect.execute(&mut game,&mut ctx);
        if mode==1 { assert!(matches!(result,Err(ExecutionError::UnresolvableValue(_)))); }
        else {
            let outcome=result.unwrap();
            if mode==2 { assert!(ctx.decision_maker.awaiting_choice());assert!(outcome.events.is_empty()); }
            else {
                assert!(!ctx.decision_maker.awaiting_choice());
                assert_eq!(outcome.value,crate::effect::OutcomeValue::ManaAdded(vec![ManaSymbol::Green]));
                assert_eq!(game.player(alice).unwrap().mana_pool.green,1);
                if mode==3 { assert_eq!(game.player(bob).unwrap().mana_pool.white,1);assert_eq!(game.player(bob).unwrap().life,20); }
                else { assert_eq!(game.player(bob).unwrap().life,30); }
                let mut events=game.take_pending_trigger_events();events.extend(outcome.events);
                let mana=events.iter().filter_map(|event|event.downcast::<crate::events::ManaAddedEvent>()).collect::<Vec<_>>();
                assert_eq!(mana.len(),if mode==3 {2}else{1});
                assert_eq!(mana[0].player,alice);assert_eq!(mana[0].mana,vec![ManaSymbol::Green]);
                if mode==3 { assert_eq!(mana[1].player,bob);assert_eq!(mana[1].source,replacement_source);assert_eq!(mana[1].mana,vec![ManaSymbol::White]); }
                else { assert_eq!(events.iter().filter_map(|event|event.downcast::<crate::events::LifeGainEvent>()).map(|gain|gain.amount).collect::<Vec<_>>(),vec![2,8]); }
            }
        }
        assert_eq!(ctx.source,source);assert_eq!(ctx.controller,alice);
        assert!(ctx.replacement.suppressed_replacement_effects.is_empty());assert!(ctx.replacement.suppressed_replacement_effect_keys.is_empty());drop(ctx);
        assert_eq!(game.player(alice).unwrap().life,20);assert_eq!(game.next_object_id_counter(),before_id);
        assert_eq!(game.effect_store.replacement_effects.get_effect(shield).is_some(),mode!=0);
        if mode==1 || mode==2 {
            assert_eq!(game.player(alice).unwrap().mana_pool.green,0);assert_eq!(game.player(bob).unwrap().life,20);assert!(game.take_pending_trigger_events().is_empty());
        }
        if mode==2 {
            assert_eq!(dm.questions,1);let mut replay=PauseManaPayload { pause:false,pending:false,questions:0 };
            let mut ctx=ExecutionContext::new_default(source,alice).with_decision_maker(&mut replay);
            let outcome=effect.execute(&mut game,&mut ctx).unwrap();assert!(!ctx.decision_maker.awaiting_choice());drop(ctx);
            assert_eq!(replay.questions,1);assert_eq!(outcome.value,crate::effect::OutcomeValue::ManaAdded(vec![ManaSymbol::Green]));
            assert_eq!(game.player(alice).unwrap().mana_pool.green,1);assert_eq!(game.player(bob).unwrap().life,34);
            assert!(game.effect_store.replacement_effects.get_effect(shield).is_none());
            let mut events=game.take_pending_trigger_events();events.extend(outcome.events);
            assert_eq!(events.iter().filter_map(|event|event.downcast::<crate::events::LifeGainEvent>()).map(|gain|gain.amount).collect::<Vec<_>>(),vec![2,4,8]);
            assert_eq!(events.iter().filter(|event|event.downcast::<crate::events::ManaAddedEvent>().is_some()).count(),1);
        }
    }
    #[test] fn additional_mana_keeps_original_receipt_and_payload() { check_additional_mana_owner(0); }
    #[test] fn additional_mana_error_restores_original_and_prefix() { check_additional_mana_owner(1); }
    #[test] fn additional_mana_pending_restores_then_replays_once() { check_additional_mana_owner(2); }
    #[test] fn additional_nested_mana_preserves_application_history() { check_additional_mana_owner(3); }
}
