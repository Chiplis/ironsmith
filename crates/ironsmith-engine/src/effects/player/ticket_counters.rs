//! Ticket counters effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;
pub use ironsmith_core::TicketCountersEffect;

fn ticket_counter_instruction(
    effect: &TicketCountersEffect,
) -> crate::effects::PlayerCountersEffect {
    crate::effects::PlayerCountersEffect::new(
        crate::object::CounterType::Named("ticket".into()),
        effect.count.clone(),
        effect.player.clone(),
    )
}

impl EffectExecutor for TicketCountersEffect {
    fn supports_simultaneous_player_action(&self) -> bool {
        true
    }

    fn prepare_simultaneous_player_action(
        &self,
        game: &GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Box<dyn crate::effects::SimultaneousEffectProposal>, ExecutionError> {
        ticket_counter_instruction(self).prepare_simultaneous_player_action(game, ctx)
    }

    fn supports_replacement_draw_continuation(&self) -> bool {
        true
    }

    fn prepare_replacement_draw_continuation_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<
        crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
        ExecutionError,
    > {
        ticket_counter_instruction(self)
            .prepare_replacement_draw_continuation_with_outputs(game, ctx)
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.execute_with_outputs(game, ctx)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }

    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        ticket_counter_instruction(self).execute_child_with_outputs(game, ctx)
    }
}

#[cfg(test)]
mod ticket_counter_event_contract_tests {
    use super::*;
    use crate::effect::{Effect, EffectId, Value};
    use crate::effects::{PutCountersEffect, execute_effect};
    use crate::events::MarkersChangedEvent;
    use crate::ids::{CardId, PlayerId};
    use crate::object::CounterType;
    use crate::target::{ChooseSpec, PlayerFilter};
    fn kind() -> CounterType {
        CounterType::Named("ticket".into())
    }
    fn fixture() -> (GameState, crate::ids::ObjectId, PlayerId, PlayerId) {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let card = crate::card::CardBuilder::new(CardId::new(), "Ticket instruction source")
            .card_types(vec![crate::types::CardType::Artifact])
            .build();
        let source = game.create_object_from_card(&card, alice, crate::zone::Zone::Battlefield);
        (game, source, alice, bob)
    }
    fn doubler(
        game: &mut GameState,
        source: crate::ids::ObjectId,
        alice: PlayerId,
        bob: PlayerId,
    ) -> crate::replacement::ReplacementEffectId {
        let replacement =
            crate::static_abilities::StaticAbility::double_player_counters_replacement(
                PlayerFilter::Specific(bob),
                Some(kind()),
                "Double ticket proposal".into(),
            )
            .generate_replacement_effect(source, alice)
            .unwrap();
        game.effect_store
            .replacement_effects
            .add_one_shot_effect(replacement)
    }
    #[test]
    fn ticket_instruction_places_real_counters_on_selected_player() {
        let (mut game, source, alice, bob) = fixture();
        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = Effect::with_id(
            71,
            Effect::new(TicketCountersEffect::new(3, PlayerFilter::Specific(bob))),
        );
        let out = execute_effect(&mut game, &effect, &mut ctx).unwrap();
        assert_eq!(
            game.player(bob).unwrap().counter_count(kind()),
            3,
            "reported receipt must correspond to actual ticket placement"
        );
        assert_eq!(game.player(alice).unwrap().counter_count(kind()), 0);
        assert_eq!(out.as_count(), Some(3));
        let marker = out
            .events
            .iter()
            .find_map(|event| event.downcast::<MarkersChangedEvent>())
            .unwrap();
        assert_eq!(marker.marker, crate::marker::Marker::Counter(kind()));
        assert_eq!(marker.amount, 3);
        assert_eq!(marker.count_after, Some(3));
        assert_eq!(marker.location, crate::marker::MarkerLocation::Player(bob));
        let follow = Effect::new(PutCountersEffect::new(
            CounterType::Charge,
            Value::EffectValue(EffectId(71)),
            ChooseSpec::SpecificObject(source),
        ));
        assert_eq!(
            execute_effect(&mut game, &follow, &mut ctx)
                .unwrap()
                .as_count(),
            Some(3)
        );
        assert_eq!(game.counter_count(source, CounterType::Charge), 3);
    }
    #[test]
    fn ticket_instruction_applies_player_counter_replacement_once() {
        let (mut game, source, alice, bob) = fixture();
        let one_shot = doubler(&mut game, source, alice, bob);
        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = Effect::with_id(
            71,
            Effect::new(TicketCountersEffect::new(3, PlayerFilter::Specific(bob))),
        );
        let out = execute_effect(&mut game, &effect, &mut ctx).unwrap();
        assert_eq!(
            out.as_count(),
            Some(6),
            "ticket counters must reach the supported generic counter replacement"
        );
        assert_eq!(game.player(bob).unwrap().counter_count(kind()), 6);
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(one_shot)
                .is_none()
        );
        let marker = out
            .events
            .iter()
            .find_map(|event| event.downcast::<MarkersChangedEvent>())
            .unwrap();
        assert_eq!(marker.amount, 6);
        assert_eq!(marker.count_after, Some(6));
        let follow = Effect::new(PutCountersEffect::new(
            CounterType::Charge,
            Value::EffectValue(EffectId(71)),
            ChooseSpec::SpecificObject(source),
        ));
        execute_effect(&mut game, &follow, &mut ctx).unwrap();
        assert_eq!(game.counter_count(source, CounterType::Charge), 6);
        let second = Effect::new(TicketCountersEffect::new(3, PlayerFilter::Specific(bob)));
        assert_eq!(
            execute_effect(&mut game, &second, &mut ctx)
                .unwrap()
                .as_count(),
            Some(3)
        );
        assert_eq!(game.player(bob).unwrap().counter_count(kind()), 9);
    }
    #[test]
    fn ticket_instruction_accepts_actual_unsigned_prior_receipt() {
        for amount in [i32::MAX as u32 + 1, u32::MAX] {
            let (mut game, source, alice, bob) = fixture();
            let mut ctx = ExecutionContext::new_default(source, alice);
            let prior = Effect::with_id(
                31,
                Effect::new(PutCountersEffect::new(
                    CounterType::Charge,
                    amount,
                    ChooseSpec::SpecificObject(source),
                )),
            );
            assert_eq!(
                execute_effect(&mut game, &prior, &mut ctx)
                    .unwrap()
                    .as_count(),
                Some(i64::from(amount))
            );
            let effect = Effect::with_id(
                71,
                Effect::new(TicketCountersEffect::new(
                    Value::EffectValue(EffectId(31)),
                    PlayerFilter::Specific(bob),
                )),
            );
            let out = execute_effect(&mut game, &effect, &mut ctx)
                .expect("real unsigned receipt fits ticket counter storage");
            assert_eq!(out.as_count(), Some(i64::from(amount)));
            assert_eq!(game.player(bob).unwrap().counter_count(kind()), amount);
            let marker = out
                .events
                .iter()
                .find_map(|event| event.downcast::<MarkersChangedEvent>())
                .unwrap();
            assert_eq!(marker.amount, amount);
            assert_eq!(marker.count_after, Some(amount));
        }
    }
    #[test]
    fn ticket_instruction_unsigned_overflow_restores_counter_and_one_shot() {
        let (mut game, source, alice, bob) = fixture();
        game.player_mut(bob)
            .unwrap()
            .add_counters(kind(), u32::MAX - 2);
        let one_shot = doubler(&mut game, source, alice, bob);
        game.take_pending_trigger_events();
        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.set_tagged_players("retained", vec![alice]);
        let result =
            TicketCountersEffect::new(2, PlayerFilter::Specific(bob)).execute(&mut game, &mut ctx);
        assert_eq!(
            result.unwrap_err(),
            ExecutionError::ResourceLimitExceeded {
                resource: "player counter placement",
                requested: u128::from(u32::MAX) + 2,
                maximum: u128::from(u32::MAX)
            }
        );
        assert_eq!(
            game.player(bob).unwrap().counter_count(kind()),
            u32::MAX - 2
        );
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(one_shot)
                .is_some()
        );
        assert_eq!(ctx.get_tagged_players("retained"), Some(&vec![alice]));
        assert!(game.take_pending_trigger_events().is_empty());
    }
}
