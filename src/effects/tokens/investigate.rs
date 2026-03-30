//! Investigate effect implementation.

use crate::cards::tokens::clue_token_definition;
use crate::effect::{EffectOutcome, Value};
use crate::effects::helpers::resolve_value;
use crate::effects::{CreateTokenEffect, EffectExecutor};
use crate::events::{KeywordActionEvent, KeywordActionKind};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::triggers::TriggerEvent;

/// Effect that performs the investigate keyword action.
///
/// Each investigate creates a Clue token as a separate action.
#[derive(Debug, Clone, PartialEq)]
pub struct InvestigateEffect {
    /// How many times to investigate.
    pub count: Value,
}

impl InvestigateEffect {
    /// Create a new investigate effect.
    pub fn new(count: impl Into<Value>) -> Self {
        Self {
            count: count.into(),
        }
    }
}

impl EffectExecutor for InvestigateEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let count = resolve_value(game, &self.count, ctx)?.max(0) as usize;
        if count == 0 {
            return Ok(EffectOutcome::resolved());
        }

        let mut outcomes = Vec::with_capacity(count);
        let mut action_events = Vec::with_capacity(count);
        for _ in 0..count {
            let effect = CreateTokenEffect::you(clue_token_definition(), 1);
            outcomes.push(effect.execute(game, ctx)?);
            action_events.push(TriggerEvent::new_with_provenance(
                KeywordActionEvent::new(
                    KeywordActionKind::Investigate,
                    ctx.controller,
                    ctx.source,
                    1,
                ),
                ctx.provenance,
            ));
        }

        let created_clues = outcomes
            .iter()
            .map(|outcome| outcome.output_objects().len() as i32)
            .sum();
        let mut outcome = EffectOutcome::aggregate(outcomes).with_events(action_events);
        outcome.set_value(crate::effect::OutcomeValue::Count(created_clues));
        Ok(outcome)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    #[test]
    fn investigate_twice_sums_created_clues_in_summary() {
        let mut game = setup_game();
        let alice = crate::ids::PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let result = InvestigateEffect::new(2)
            .execute(&mut game, &mut ctx)
            .expect("investigate resolves");

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(2));
        assert_eq!(game.battlefield.len(), 2);
        let investigate_events = result
            .events
            .iter()
            .filter(|event| {
                event
                    .downcast::<KeywordActionEvent>()
                    .is_some_and(|action| action.action == KeywordActionKind::Investigate)
            })
            .count();
        assert_eq!(investigate_events, 2);
    }

    #[test]
    fn investigate_creates_runtime_clue_with_expected_properties() {
        let mut game = setup_game();
        let alice = crate::ids::PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        InvestigateEffect::new(1)
            .execute(&mut game, &mut ctx)
            .expect("investigate resolves");

        let clue_id = *game
            .battlefield
            .first()
            .expect("investigate should create a clue token");
        let clue = game.object(clue_id).expect("clue should exist");
        assert_eq!(clue.name, "Clue");
        assert!(
            game.object_has_card_type(clue_id, crate::types::CardType::Artifact),
            "Clue should be an artifact token"
        );
        assert!(
            game.calculated_subtypes(clue_id)
                .contains(&crate::types::Subtype::Clue),
            "Clue should have the Clue subtype"
        );
        assert!(
            game.current_colors(clue_id)
                .is_some_and(|colors| colors.is_empty()),
            "Clue should be colorless"
        );
        assert_eq!(
            clue.abilities.len(),
            1,
            "Clue should have one activated ability"
        );
        assert_eq!(
            clue.abilities[0].text.as_deref(),
            Some("{2}, Sacrifice this artifact: Draw a card."),
            "Clue should carry the predefined draw ability"
        );
    }
}
