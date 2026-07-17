//! Prevent all damage effect implementation.

use super::prevention_helpers::{
    SourceChoiceSelection, choose_source_of_your_choice,
    choose_source_sharing_activation_payment_color, register_prevention_shield,
};
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_objects_from_spec;
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
        if let Some(source_target) = &self.source_target {
            damage_filter.from_specific_source =
                resolve_objects_from_spec(game, source_target, ctx)?
                    .first()
                    .copied();
            if damage_filter.from_specific_source.is_none() {
                return Err(ExecutionError::InvalidTarget);
            }
        }
        if let Some(excluded_source_target) = &self.excluded_source_target {
            damage_filter.excluded_specific_source =
                resolve_objects_from_spec(game, excluded_source_target, ctx)?
                    .first()
                    .copied();
            if damage_filter.excluded_specific_source.is_none() {
                return Err(ExecutionError::InvalidTarget);
            }
        }
        if self.source_of_your_choice {
            let selection = if self.source_choice_shares_activation_mana_color {
                choose_source_sharing_activation_payment_color(game, ctx)
            } else {
                choose_source_of_your_choice(game, ctx)
            };
            match selection {
                SourceChoiceSelection::Chosen(source) => {
                    damage_filter.from_specific_source = Some(source);
                }
                SourceChoiceSelection::NoAvailableSource => return Ok(EffectOutcome::resolved()),
                SourceChoiceSelection::NoChoiceMade => return Ok(EffectOutcome::count(0)),
            }
        }

        let protected = if self.protect_source {
            crate::prevention::PreventionTarget::Permanent(ctx.source)
        } else {
            self.target.clone()
        };
        register_prevention_shield(
            game,
            ctx,
            protected,
            None,
            self.until.clone(),
            damage_filter,
            Vec::new(),
            Vec::new(),
            Vec::new(),
        );

        Ok(EffectOutcome::resolved())
    }

    fn get_target_spec(&self) -> Option<&crate::target::ChooseSpec> {
        self.source_target
            .as_ref()
            .or(self.excluded_source_target.as_ref())
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
