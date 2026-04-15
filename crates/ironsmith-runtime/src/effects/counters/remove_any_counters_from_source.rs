//! Remove any number of counters from the source permanent.

use crate::decision::FallbackStrategy;
use crate::decisions::{CounterRemovalSpec, NumberSpec, make_decision_with_fallback};
use crate::effect::EffectOutcome;
use crate::effects::{CostExecutableEffect, CostValidationError, EffectExecutor};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::object::CounterType;

/// Remove any number of counters from the source permanent.
///
/// Used for costs like:
/// - "Remove any number of charge counters from this artifact"
/// - "Remove X storage counters from this land"
#[derive(Debug, Clone, PartialEq)]
pub struct RemoveAnyCountersFromSourceEffect {
    /// Optional counter type restriction.
    pub counter_type: Option<CounterType>,
    /// Whether display should use `X` instead of `any number`.
    pub display_x: bool,
}

impl RemoveAnyCountersFromSourceEffect {
    pub fn any_number(counter_type: Option<CounterType>) -> Self {
        Self {
            counter_type,
            display_x: false,
        }
    }

    pub fn x(counter_type: Option<CounterType>) -> Self {
        Self {
            counter_type,
            display_x: true,
        }
    }

    fn max_removable(&self, game: &GameState, source: crate::ids::ObjectId) -> Result<u32, String> {
        let obj = game
            .object(source)
            .ok_or_else(|| "source not found".to_string())?;
        if obj.zone != crate::zone::Zone::Battlefield {
            return Err("source must be on the battlefield".to_string());
        }

        Ok(if let Some(counter_type) = self.counter_type {
            obj.counters.get(&counter_type).copied().unwrap_or(0)
        } else {
            obj.counters.values().copied().sum::<u32>()
        })
    }

    pub fn cost_display(&self) -> String {
        let amount_text = if self.display_x { "X" } else { "any number of" };
        match self.counter_type {
            Some(counter_type) => format!(
                "Remove {amount_text} {} counter{} from ~",
                counter_type.description(),
                if self.display_x { "" } else { "s" }
            ),
            None => format!("Remove {amount_text} counters from ~"),
        }
    }
}

impl EffectExecutor for RemoveAnyCountersFromSourceEffect {
    fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
        Some(self)
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let max_removable = self
            .max_removable(game, ctx.source)
            .map_err(ExecutionError::Impossible)?;

        let description = if self.display_x {
            "Choose X counters to remove"
        } else {
            "Choose counters to remove"
        };
        let to_remove = make_decision_with_fallback(
            game,
            &mut ctx.decision_maker,
            ctx.controller,
            Some(ctx.source),
            NumberSpec::up_to(ctx.source, max_removable, description),
            FallbackStrategy::Maximum,
        )
        .min(max_removable);

        let mut removed_total = 0u32;
        let mut outcome = EffectOutcome::count(0);
        if let Some(counter_type) = self.counter_type {
            if to_remove > 0
                && let Some((removed, event)) = game.remove_counters(
                    ctx.source,
                    counter_type,
                    to_remove,
                    Some(ctx.source),
                    Some(ctx.controller),
                )
            {
                removed_total = removed;
                outcome = outcome.with_event(event);
            }
        } else {
            let available_counters: Vec<(CounterType, u32)> = game
                .object(ctx.source)
                .map(|obj| {
                    obj.counters
                        .iter()
                        .filter(|(_, count)| **count > 0)
                        .map(|(counter_type, count)| (*counter_type, *count))
                        .collect()
                })
                .unwrap_or_default();

            let selections = make_decision_with_fallback(
                game,
                &mut ctx.decision_maker,
                ctx.controller,
                Some(ctx.source),
                CounterRemovalSpec::new(ctx.source, ctx.source, to_remove, available_counters),
                FallbackStrategy::Maximum,
            );

            for (counter_type, requested) in selections {
                if removed_total >= to_remove {
                    break;
                }
                let remaining = to_remove - removed_total;
                let to_remove_now = requested.min(remaining);
                if to_remove_now == 0 {
                    continue;
                }
                if let Some((removed, event)) = game.remove_counters(
                    ctx.source,
                    counter_type,
                    to_remove_now,
                    Some(ctx.source),
                    Some(ctx.controller),
                ) {
                    removed_total += removed;
                    outcome = outcome.with_event(event);
                }
            }
        }

        if removed_total != to_remove {
            return Ok(EffectOutcome::impossible());
        }

        outcome.set_value(crate::effect::OutcomeValue::Count(removed_total as i32));
        Ok(outcome)
    }

    fn cost_description(&self) -> Option<String> {
        Some(self.cost_display())
    }
}

impl CostExecutableEffect for RemoveAnyCountersFromSourceEffect {
    fn can_execute_as_cost(
        &self,
        game: &GameState,
        source: crate::ids::ObjectId,
        _controller: crate::ids::PlayerId,
    ) -> Result<(), CostValidationError> {
        self.max_removable(game, source)
            .map(|_| ())
            .map_err(CostValidationError::Other)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::costs::{Cost, CostContext, CostPaymentResult};
    use crate::ids::{CardId, PlayerId};
    use crate::types::CardType;
    use crate::{card::CardBuilder, game_state::GameState, zone::Zone};

    fn create_test_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    fn simple_card(name: &str, id: u32) -> crate::card::Card {
        CardBuilder::new(CardId::from_raw(id), name)
            .card_types(vec![CardType::Artifact])
            .build()
    }

    #[test]
    fn display_text() {
        assert_eq!(
            RemoveAnyCountersFromSourceEffect::any_number(Some(CounterType::Charge)).cost_display(),
            "Remove any number of charge counters from ~"
        );
        assert_eq!(
            RemoveAnyCountersFromSourceEffect::x(Some(CounterType::Storage)).cost_display(),
            "Remove X storage counter from ~"
        );
    }

    #[test]
    fn pay_sets_x() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);

        let card = simple_card("Battery", 1);
        let card_id = game.create_object_from_card(&card, alice, Zone::Battlefield);
        if let Some(obj) = game.object_mut(card_id) {
            obj.counters.insert(CounterType::Charge, 3);
        }

        let cost = Cost::effect(RemoveAnyCountersFromSourceEffect::any_number(Some(
            CounterType::Charge,
        )));
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let mut ctx = CostContext::new(card_id, alice, &mut dm);

        let result = cost.pay(&mut game, &mut ctx);
        assert_eq!(result, Ok(CostPaymentResult::Paid));
        assert_eq!(ctx.x_value, Some(3));
        assert_eq!(game.counter_count(card_id, CounterType::Charge), 0);
    }
}
