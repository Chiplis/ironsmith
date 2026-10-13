//! Investigate effect implementation.

use crate::cards::tokens::clue_token_definition;
use crate::effect::{EffectOutcome, Value};
use crate::effects::helpers::{resolve_player_filter, resolve_value};
use crate::effects::{CreateTokenEffect, EffectExecutor};
use crate::effects::{ExecutionContext, ExecutionError};
#[cfg(test)]
use crate::events::KeywordActionEvent;
use crate::events::KeywordActionKind;
use crate::filter::PlayerFilter;
use crate::game_state::GameState;

/// Effect that performs the investigate keyword action.
///
/// Each investigate creates a Clue token as a separate action.
#[derive(Debug, Clone)]
pub struct InvestigateEffect {
    /// How many times to investigate.
    pub count: Value,
    /// The player who investigates.
    pub player: PlayerFilter,
    token: crate::cards::CardDefinition,
}

impl PartialEq for InvestigateEffect {
    fn eq(&self, other: &Self) -> bool { self.count == other.count && self.player == other.player }
}

impl InvestigateEffect {
    fn body_program(
        &self,
        player: PlayerFilter,
    ) -> crate::effects::composition::KeywordCompletionProgram {
        let token = self.token.clone();
        let roles = ironsmith_core::TokenTextRoles::rules_implied(
            ironsmith_core::TokenNameTextRole::SubtypeDerived,
            token.abilities.len(),
        );
        crate::effects::composition::KeywordCompletionProgram {
            action: KeywordActionKind::Investigate,
            amount: 1.into(),
            player: player.clone(),
            body: crate::effect::Effect::new(
                CreateTokenEffect::new(token, 1, player).with_text_roles(roles),
            ),
        }
    }

    /// Create a new investigate effect.
    pub fn new(count: impl Into<Value>, player: PlayerFilter) -> Self {
        Self {
            count: count.into(),
            player,
            token: clue_token_definition(),
        }
    }

    /// The controller investigates N times.
    pub fn you(count: impl Into<Value>) -> Self {
        Self::new(count, PlayerFilter::You)
    }
}

impl EffectExecutor for InvestigateEffect {
    fn visit_child_effects(&self, visitor: &mut dyn FnMut(&crate::effect::Effect)) {
        visitor(&crate::effect::Effect::new(
            self.body_program(self.player.clone()),
        ));
    }

    fn requires_sequential_player_actions(&self) -> bool {
        true
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
        super::lifecycle::execute_token_instruction_with_pending_value(
            game,
            ctx,
            || {
                crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::with_objects(
                    Vec::new(),
                ))
            },
            |game, ctx| {
                let player_id = resolve_player_filter(game, &self.player, ctx)?;
                let count = resolve_value(game, &self.count, ctx)?.max(0) as usize;
                if count == 0 {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::resolved(),
                    ));
                }

                game.reserve_token_repetition_work(count)?;
                let mut outcomes = super::resources::buffer(count)?;
                let mut outputs = crate::effects::CompletedEffectOutputs::aggregate_only(
                    EffectOutcome::resolved(),
                );
                let mut created_clues = 0i64;
                for _ in 0..count {
                    let effect = self.body_program(PlayerFilter::Specific(player_id));
                    let child = effect.execute_child_with_outputs(game, ctx)?;
                    created_clues += child.outcome.output_objects().len() as i64;
                    if ctx.decision_maker.awaiting_choice() {
                        return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                            EffectOutcome::resolved(),
                        ));
                    }
                    outcomes.push(child.outcome.clone());
                    outputs.retain_owned_child(child);
                }

                let mut outcome = EffectOutcome::aggregate(outcomes);
                outcome.set_value(crate::effect::OutcomeValue::Count(created_clues));
                Ok(outputs.project_aggregate(outcome))
            },
        )
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

        let result = InvestigateEffect::you(2)
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

        InvestigateEffect::you(1)
            .execute(&mut game, &mut ctx)
            .expect("investigate resolves");

        let clue_id = *game
            .battlefield
            .first()
            .expect("investigate should create a clue token");
        let clue = game.object(clue_id).expect("clue should exist");
        assert_eq!(clue.name, "Clue Token");
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
        let crate::ability::AbilityKind::Activated(activated) = &clue.abilities[0].kind else {
            panic!("Clue should carry an activated ability")
        };
        assert_eq!(activated.mana_cost.costs().len(), 2);
        assert!(
            activated
                .effects
                .all_effects()
                .into_iter()
                .any(|effect| effect
                    .downcast_ref::<crate::effects::DrawCardsEffect>()
                    .is_some()),
            "Clue should carry the predefined typed draw effect"
        );
    }
}
