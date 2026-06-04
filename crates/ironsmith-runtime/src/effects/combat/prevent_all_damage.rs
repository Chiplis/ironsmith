//! Prevent all damage effect implementation.

use super::prevention_helpers::register_prevention_shield;
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
pub use ironsmith_core::PreventAllDamageEffect;

/// Effect that prevents all damage until end of turn.
///
/// Can optionally filter to only prevent damage to certain permanents.
///
/// # Fields
///
/// * `filter` - Optional filter for which permanents to protect
///
/// # Example
///
/// ```ignore
/// // Prevent all damage this turn (Fog)
/// let effect = PreventAllDamageEffect::all();
///
/// // Prevent all damage to creatures you control this turn
/// let effect = PreventAllDamageEffect::matching(
///     ObjectFilter::creature().you_control()
/// );
/// ```
impl EffectExecutor for PreventAllDamageEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        // Check if damage can be prevented globally
        if !game.can_prevent_damage() {
            return Ok(EffectOutcome::prevented());
        }

        let mut damage_filter = self.damage_filter.clone();
        if self.source_of_your_choice {
            let mut candidates = Vec::new();
            candidates.extend(game.stack.iter().map(|entry| entry.object_id));
            candidates.extend(game.battlefield.iter().copied());
            candidates.sort_by_key(|id| id.0);
            candidates.dedup();

            if candidates.is_empty() {
                return Ok(EffectOutcome::resolved());
            }

            let selectable = candidates
                .iter()
                .copied()
                .map(|id| {
                    let name = game
                        .object(id)
                        .map(|object| object.name.clone())
                        .unwrap_or_else(|| format!("object {}", id.0));
                    crate::decisions::context::SelectableObject::new(id, name)
                })
                .collect::<Vec<_>>();
            let select_ctx = crate::decisions::context::SelectObjectsContext::new(
                ctx.controller,
                Some(ctx.source),
                "Choose a source",
                selectable,
                1,
                Some(1),
            );
            let chosen_source = ctx
                .decision_maker
                .decide_objects(game, &select_ctx)
                .into_iter()
                .find(|id| candidates.contains(id));
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
            }
            let Some(chosen_source) = chosen_source else {
                return Ok(EffectOutcome::count(0));
            };
            damage_filter.from_specific_source = Some(chosen_source);
        }

        register_prevention_shield(
            game,
            ctx,
            self.target.clone(),
            None,
            self.until.clone(),
            damage_filter,
            Vec::new(),
            Vec::new(),
            Vec::new(),
        );

        Ok(EffectOutcome::resolved())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effect::Until;
    use crate::ids::PlayerId;
    use crate::target::ObjectFilter;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    #[test]
    fn test_prevent_all_damage() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = PreventAllDamageEffect::all(Until::EndOfTurn);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.status, crate::effect::OutcomeStatus::Succeeded);
        assert_eq!(game.effect_store.prevention_effects.shields().len(), 1);

        // Shield should have unlimited prevention
        let shield = &game.effect_store.prevention_effects.shields()[0];
        assert!(shield.amount_remaining.is_none());
    }

    #[test]
    fn test_prevent_all_damage_to_your_creatures() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = PreventAllDamageEffect::your_creatures(Until::EndOfTurn);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.status, crate::effect::OutcomeStatus::Succeeded);
        assert_eq!(game.effect_store.prevention_effects.shields().len(), 1);
    }

    #[test]
    fn test_prevent_all_damage_with_filter() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = PreventAllDamageEffect::matching(ObjectFilter::creature(), Until::EndOfTurn);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.status, crate::effect::OutcomeStatus::Succeeded);
    }

    #[test]
    fn test_prevent_all_damage_clone_box() {
        let effect = PreventAllDamageEffect::all(Until::EndOfTurn);
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("PreventAllDamageEffect"));
    }
}
