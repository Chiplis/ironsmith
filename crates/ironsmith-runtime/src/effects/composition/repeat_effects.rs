use crate::effect::{Effect, EffectOutcome, OutcomeStatus, OutcomeValue};
use crate::effects::{EffectExecutor, SequenceEffect};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::resolve_value;

pub type RepeatEffectsEffect = ironsmith_core::RepeatEffectsEffect<Effect>;

impl EffectExecutor for RepeatEffectsEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn visit_child_effects(&self, visitor: &mut dyn FnMut(&Effect)) {
        for effect in &self.effects {
            visitor(effect);
        }
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let count = resolve_value(game, &self.count, ctx)?.max(0) as usize;
        let sequence = SequenceEffect::new(self.effects.clone());
        let mut all_events = Vec::new();
        let mut all_execution_facts = Vec::new();
        let mut total_count = 0;
        let mut saw_count = false;

        for _ in 0..count {
            let outcome = sequence.execute(game, ctx)?;
            if let OutcomeValue::Count(n) = &outcome.value {
                total_count += *n;
                saw_count = true;
            }
            all_events.extend(outcome.events.clone());
            all_execution_facts.extend(outcome.execution_facts.clone());
            if outcome.status.is_failure() {
                return Ok(EffectOutcome::with_details(
                    outcome.status,
                    outcome.value,
                    all_events,
                    all_execution_facts,
                ));
            }
        }

        Ok(EffectOutcome::with_details(
            OutcomeStatus::Succeeded,
            if saw_count {
                OutcomeValue::Count(total_count)
            } else {
                OutcomeValue::None
            },
            all_events,
            all_execution_facts,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::effect::Effect;
    use crate::ids::{CardId, PlayerId};
    use crate::types::CardType;
    use crate::zone::Zone;

    #[test]
    fn repeat_effects_sums_child_outcome_counts() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        for i in 0..2 {
            let card = CardBuilder::new(CardId::from_raw(920_000 + i), "Library Card")
                .card_types(vec![CardType::Instant])
                .build();
            game.create_object_from_card(&card, alice, Zone::Library);
        }

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = RepeatEffectsEffect::new(
            3,
            vec![Effect::new(crate::effects::DrawCardsEffect::you(1))],
        );

        let outcome = effect
            .execute(&mut game, &mut ctx)
            .expect("repeat draw should resolve");

        assert_eq!(outcome.value, OutcomeValue::Count(2));
        assert_eq!(game.player(alice).expect("alice exists").hand.len(), 2);
    }
}
