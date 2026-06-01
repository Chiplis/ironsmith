//! Make a source assign no combat damage.

pub use ironsmith_core::AssignNoCombatDamageEffect;

use crate::effect::{EffectOutcome, Until};
use crate::effects::helpers::resolve_objects_for_effect;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::ChooseSpec;

impl EffectExecutor for AssignNoCombatDamageEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let source_id = *resolve_objects_for_effect(game, ctx, &self.source)?
            .first()
            .ok_or(ExecutionError::InvalidTarget)?;

        if matches!(self.until, Until::EndOfTurn) {
            game.set_assigns_no_combat_damage(source_id);
        }

        Ok(EffectOutcome::resolved())
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        if self.source.is_target() {
            Some(&self.source)
        } else {
            None
        }
    }

    fn get_target_count(&self) -> Option<crate::effect::ChoiceCount> {
        if self.source.is_target() {
            Some(self.source.count())
        } else {
            None
        }
    }

    fn target_description(&self) -> &'static str {
        "source to assign no combat damage"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::PlayerId;

    #[test]
    fn marks_source_without_using_damage_prevention() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        game.effect_store
            .cant_effects
            .set_damage_cant_be_prevented(true);
        AssignNoCombatDamageEffect::new(ChooseSpec::Source, Until::EndOfTurn)
            .execute(&mut game, &mut ctx)
            .expect("assign-no-combat-damage effect should resolve");

        assert!(game.assigns_no_combat_damage(source));
        assert!(!game.can_prevent_damage());
    }
}
