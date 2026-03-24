//! Prevent all combat damage from a chosen source.

use super::prevention_helpers::register_prevention_shield;
use crate::effect::{EffectOutcome, Until};
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_objects_for_effect;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::prevention::{DamageFilter, PreventionTarget};
use crate::target::ChooseSpec;

/// Effect that prevents all combat damage from a chosen source.
#[derive(Debug, Clone, PartialEq)]
pub struct PreventAllCombatDamageFromEffect {
    /// Source to filter prevented combat damage by.
    pub source: ChooseSpec,
    /// Duration of the prevention shield.
    pub until: Until,
}

impl PreventAllCombatDamageFromEffect {
    pub fn new(source: ChooseSpec, until: Until) -> Self {
        Self { source, until }
    }
}

impl EffectExecutor for PreventAllCombatDamageFromEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let source_id = *resolve_objects_for_effect(game, ctx, &self.source)?
            .first()
            .ok_or(ExecutionError::InvalidTarget)?;

        if !game.can_prevent_damage() {
            return Ok(EffectOutcome::resolved());
        }

        let mut filter = DamageFilter::combat();
        filter.from_specific_source = Some(source_id);

        register_prevention_shield(
            game,
            ctx,
            PreventionTarget::All,
            None,
            self.until.clone(),
            filter,
            Vec::new(),
        );
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
        "source to prevent combat damage from"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executor::ResolvedTarget;
    use crate::ids::PlayerId;
    use crate::target::ObjectFilter;

    #[test]
    fn creates_combat_prevention_shield_for_selected_source() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let selected_source = game.new_object_id();
        let effect_source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(effect_source, alice)
            .with_targets(vec![ResolvedTarget::Object(selected_source)])
            .with_target_assignments(vec![crate::game_state::TargetAssignment {
                spec: ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature())),
                range: 0..1,
            }]);
        let effect = PreventAllCombatDamageFromEffect::new(
            ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature())),
            Until::EndOfTurn,
        );
        effect
            .execute(&mut game, &mut ctx)
            .expect("prevent-all-from effect should resolve");

        let shields = game.prevention_effects.shields();
        assert_eq!(shields.len(), 1);
        assert!(shields[0].damage_filter.combat_only);
        assert_eq!(
            shields[0].damage_filter.from_specific_source,
            Some(selected_source)
        );
    }
}
