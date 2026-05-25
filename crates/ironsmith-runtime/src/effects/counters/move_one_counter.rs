use crate::decision::FallbackStrategy;
use crate::decisions::{CounterRemovalSpec, make_decision_with_fallback};
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_objects_for_effect;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
pub use ironsmith_core::MoveOneCounterEffect;

impl EffectExecutor for MoveOneCounterEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let target_pair = if ctx.target_assignments.is_empty() && ctx.targets.len() >= 2 {
            ctx.resolve_two_object_targets()
        } else {
            let from = resolve_objects_for_effect(game, ctx, &self.from)?;
            let to = resolve_objects_for_effect(game, ctx, &self.to)?;
            from.first().copied().zip(to.first().copied())
        };
        let Some((from_id, to_id)) = target_pair else {
            return Ok(EffectOutcome::target_invalid());
        };

        let available_counters = game
            .object(from_id)
            .map(|obj| {
                obj.counters
                    .iter()
                    .filter(|(_, count)| **count > 0)
                    .map(|(ct, count)| (*ct, *count))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if available_counters.is_empty() {
            return Ok(EffectOutcome::count(0));
        }

        let spec = CounterRemovalSpec::new(ctx.source, from_id, 1, available_counters);
        let selections = make_decision_with_fallback(
            game,
            &mut ctx.decision_maker,
            ctx.controller,
            Some(ctx.source),
            spec,
            FallbackStrategy::Maximum,
        );

        for (counter_type, to_remove) in selections {
            if to_remove == 0 {
                continue;
            }
            let Some((removed, remove_event)) = game.remove_counters(
                from_id,
                counter_type,
                1,
                Some(ctx.source),
                Some(ctx.controller),
            ) else {
                continue;
            };
            if removed == 0 {
                continue;
            }
            let mut outcome = EffectOutcome::count(1).with_event(remove_event);
            if let Some(add_event) = game.add_counters_with_source(
                to_id,
                counter_type,
                1,
                Some(ctx.source),
                Some(ctx.controller),
            ) {
                outcome = outcome.with_event(add_event);
            }
            return Ok(outcome);
        }

        Ok(EffectOutcome::count(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::effect::OutcomeValue;
    use crate::effects::ResolvedTarget;
    use crate::ids::{CardId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::{CounterType, Object};
    use crate::target::ChooseSpec;
    use crate::types::CardType;
    use crate::zone::Zone;

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

    #[test]
    fn move_one_counter_moves_single_chosen_counter() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let controller = PlayerId::from_index(0);
        let source = game.new_object_id();
        let from_id = game.new_object_id();
        let to_id = game.new_object_id();
        let source_card = make_creature_card(from_id.0 as u32, "Source");
        let mut from_obj = Object::from_card(from_id, &source_card, controller, Zone::Battlefield);
        from_obj.counters.insert(CounterType::PlusOnePlusOne, 2);
        let target_card = make_creature_card(to_id.0 as u32, "Dest");
        let to_obj = Object::from_card(to_id, &target_card, controller, Zone::Battlefield);
        game.add_object(from_obj);
        game.add_object(to_obj);

        let mut ctx = ExecutionContext::new_default(source, controller).with_targets(vec![
            ResolvedTarget::Object(from_id),
            ResolvedTarget::Object(to_id),
        ]);
        let effect = MoveOneCounterEffect::new(ChooseSpec::permanent(), ChooseSpec::permanent());
        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("execute move one");

        assert_eq!(result.value, OutcomeValue::Count(1));
        assert_eq!(game.counter_count(from_id, CounterType::PlusOnePlusOne), 1);
        assert_eq!(game.counter_count(to_id, CounterType::PlusOnePlusOne), 1);
    }

    #[test]
    fn move_one_counter_with_no_available_counters_returns_zero() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let controller = PlayerId::from_index(0);
        let source = game.new_object_id();
        let from_id = game.new_object_id();
        let to_id = game.new_object_id();
        let source_card = make_creature_card(from_id.0 as u32, "Source");
        let target_card = make_creature_card(to_id.0 as u32, "Dest");
        game.add_object(Object::from_card(
            from_id,
            &source_card,
            controller,
            Zone::Battlefield,
        ));
        game.add_object(Object::from_card(
            to_id,
            &target_card,
            controller,
            Zone::Battlefield,
        ));

        let mut ctx = ExecutionContext::new_default(source, controller).with_targets(vec![
            ResolvedTarget::Object(from_id),
            ResolvedTarget::Object(to_id),
        ]);
        let effect = MoveOneCounterEffect::new(ChooseSpec::permanent(), ChooseSpec::permanent());
        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("execute move one");

        assert_eq!(result.value, OutcomeValue::Count(0));
    }
}
