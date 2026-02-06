//! Prevent all damage effect implementation.

use crate::effect::{EffectOutcome, EffectResult, Until};
use crate::effects::EffectExecutor;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::prevention::{DamageFilter, PreventionShield, PreventionShieldId, PreventionTarget};
use crate::target::ObjectFilter;

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
#[derive(Debug, Clone, PartialEq)]
pub struct PreventAllDamageEffect {
    /// What this shield protects.
    pub target: PreventionTarget,
    /// What kinds of damage this shield prevents.
    pub damage_filter: DamageFilter,
    pub until: Until,
}

impl PreventAllDamageEffect {
    /// Create a new prevent all damage effect.
    pub fn new(target: PreventionTarget, damage_filter: DamageFilter, until: Until) -> Self {
        Self {
            target,
            damage_filter,
            until,
        }
    }

    /// Prevent all damage to everything.
    pub fn all(until: Until) -> Self {
        Self::new(PreventionTarget::All, DamageFilter::all(), until)
    }

    /// Prevent all damage to the controller.
    pub fn to_you(until: Until) -> Self {
        Self::new(PreventionTarget::You, DamageFilter::all(), until)
    }

    /// Prevent all damage to permanents matching the filter.
    pub fn matching(filter: ObjectFilter, until: Until) -> Self {
        Self::new(
            PreventionTarget::PermanentsMatching(filter),
            DamageFilter::all(),
            until,
        )
    }

    /// Prevent all damage to everything with a damage filter.
    pub fn all_with_filter(damage_filter: DamageFilter, until: Until) -> Self {
        Self::new(PreventionTarget::All, damage_filter, until)
    }

    /// Prevent all damage to permanents matching the filter with a damage filter.
    pub fn matching_with_filter(
        filter: ObjectFilter,
        damage_filter: DamageFilter,
        until: Until,
    ) -> Self {
        Self::new(
            PreventionTarget::PermanentsMatching(filter),
            damage_filter,
            until,
        )
    }

    /// Prevent all damage to creatures you control.
    pub fn your_creatures(until: Until) -> Self {
        Self::matching(ObjectFilter::creature().you_control(), until)
    }
}

impl EffectExecutor for PreventAllDamageEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        // Check if damage can be prevented globally
        if !game.can_prevent_damage() {
            return Ok(EffectOutcome::from_result(EffectResult::Prevented));
        }

        // Create a prevention shield that prevents all damage
        let shield = PreventionShield {
            id: PreventionShieldId::new(0), // Will be set by manager
            source: ctx.source,
            controller: ctx.controller,
            protected: self.target.clone(),
            amount_remaining: None, // Unlimited prevention
            duration: self.until.clone(),
            damage_filter: self.damage_filter.clone(),
            created_turn: game.turn.turn_number,
        };

        game.prevention_effects.add_shield(shield);

        Ok(EffectOutcome::resolved())
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::PlayerId;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    #[test]
    fn test_prevent_all_damage() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = PreventAllDamageEffect::all(Until::EndOfTurn);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Resolved);
        assert_eq!(game.prevention_effects.shields().len(), 1);

        // Shield should have unlimited prevention
        let shield = &game.prevention_effects.shields()[0];
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

        assert_eq!(result.result, EffectResult::Resolved);
        assert_eq!(game.prevention_effects.shields().len(), 1);
    }

    #[test]
    fn test_prevent_all_damage_with_filter() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = PreventAllDamageEffect::matching(ObjectFilter::creature(), Until::EndOfTurn);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Resolved);
    }

    #[test]
    fn test_prevent_all_damage_clone_box() {
        let effect = PreventAllDamageEffect::all(Until::EndOfTurn);
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("PreventAllDamageEffect"));
    }
}
