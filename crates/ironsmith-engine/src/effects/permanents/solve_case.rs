use crate::effect::EffectOutcome;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::zone::Zone;

pub use ironsmith_core::SetClassLevelEffect;
pub use ironsmith_core::SolveCaseEffect;
pub use ironsmith_core::{DayNightDesignation, SetDayNightEffect};

impl EffectExecutor for SetDayNightEffect {
    fn execute(
        &self,
        game: &mut GameState,
        _ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let daytime = self.designation == DayNightDesignation::Day;
        // CR 731.2: setting the designation it already has changes nothing;
        // a real change queues "day becomes night"/"night becomes day" and
        // transforms day/nightbound permanents (CR 702.145). The designation
        // change is one atomic world action.
        crate::effects::composition::execute_world_checkpoint_transaction(game, |game| {
            let already = game.has_day_night() && game.is_daytime() == daytime;
            game.set_daytime(daytime);
            Ok(EffectOutcome::count(i32::from(!already)))
        })
    }
}

impl EffectExecutor for SetClassLevelEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        crate::effects::composition::execute_compound(game, ctx, |game, ctx| {
            let Some(source) = game.object(ctx.source) else {
                return Ok(EffectOutcome::target_invalid());
            };
            if source.zone != Zone::Battlefield {
                return Ok(EffectOutcome::target_invalid());
            }
            let changed = game.set_class_level(ctx.source, self.level);
            let outcome = EffectOutcome::count(i32::from(changed));
            if !changed {
                return Ok(outcome);
            }
            // CR 716.2a: "When this Class becomes level N" triggers on the level
            // change itself.
            crate::effects::composition::complete_keyword_action_with_result(
                game,
                ctx,
                outcome,
                crate::events::KeywordActionEvent::new(
                    crate::events::KeywordActionKind::GainClassLevel,
                    ctx.controller,
                    ctx.source,
                    self.level,
                ),
            )
        })
    }
}

impl EffectExecutor for SolveCaseEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        crate::effects::composition::execute_compound(game, ctx, |game, ctx| {
            let Some(source) = game.object(ctx.source) else {
                return Ok(EffectOutcome::target_invalid());
            };
            if source.zone != Zone::Battlefield {
                return Ok(EffectOutcome::target_invalid());
            }

            let changed = game.solve_case(ctx.source);
            Ok(EffectOutcome::count(i32::from(changed)))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::effect::OutcomeValue;
    use crate::ids::{CardId, PlayerId};
    use crate::types::CardType;

    #[test]
    fn solve_case_marks_source_without_adding_counters() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let card = CardBuilder::new(CardId::from_raw(72_303), "Reusable Case")
            .card_types(vec![CardType::Enchantment])
            .build();
        let source = game.create_object_from_card(&card, alice, Zone::Battlefield);
        let mut ctx = ExecutionContext::new_default(source, alice);

        let outcome = SolveCaseEffect::new()
            .execute(&mut game, &mut ctx)
            .expect("solve case effect should execute");

        assert_eq!(outcome.value, OutcomeValue::Count(1));
        assert!(game.is_case_solved(source));
        assert!(
            game.object(source)
                .expect("source should exist")
                .counters
                .is_empty(),
            "solving a Case is not represented by visible counters"
        );
    }
}
