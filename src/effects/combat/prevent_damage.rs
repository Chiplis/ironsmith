//! Prevent damage effect implementation.

use crate::effect::{EffectOutcome, Until, Value};
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_value;
use crate::executor::{ExecutionContext, ExecutionError, ResolvedTarget};
use crate::game_state::GameState;
use crate::prevention::{DamageFilter, PreventionShield, PreventionTarget};
use crate::target::{ChooseSpec, PlayerFilter};

/// Effect that prevents N damage to a target.
///
/// Creates a prevention shield that prevents the next N damage to the target
/// for the specified duration.
/// Per Rule 615.6, this creates a shield that tracks remaining prevention.
///
/// # Fields
///
/// * `amount` - Amount of damage to prevent
/// * `target` - What to protect
///
/// # Example
///
/// ```ignore
/// // Prevent the next 3 damage to target creature or player
/// let effect = PreventDamageEffect::new(3, ChooseSpec::AnyTarget, Until::EndOfTurn);
///
/// // Prevent the next 2 damage to you
/// let effect = PreventDamageEffect::to_you(2, Until::EndOfTurn);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct PreventDamageEffect {
    /// Amount of damage to prevent.
    pub amount: Value,
    /// What to protect.
    pub target: ChooseSpec,
    /// Duration for the prevention shield.
    pub duration: Until,
    /// Filter for what damage this shield applies to.
    pub damage_filter: DamageFilter,
}

impl PreventDamageEffect {
    /// Create a new prevent damage effect with explicit duration.
    pub fn new(amount: impl Into<Value>, target: ChooseSpec, duration: Until) -> Self {
        Self {
            amount: amount.into(),
            target,
            duration,
            damage_filter: DamageFilter::all(),
        }
    }

    /// Prevent damage to yourself with explicit duration.
    pub fn to_you(amount: impl Into<Value>, duration: Until) -> Self {
        Self::new(amount, ChooseSpec::SourceController, duration)
    }

    /// Prevent damage to target creature or player with explicit duration.
    pub fn any_target(amount: impl Into<Value>, duration: Until) -> Self {
        Self::new(amount, ChooseSpec::AnyTarget, duration)
    }

    /// Set a damage filter for this prevention effect.
    pub fn with_filter(mut self, filter: DamageFilter) -> Self {
        self.damage_filter = filter;
        self
    }
}

impl EffectExecutor for PreventDamageEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let amount = resolve_value(game, &self.amount, ctx)?.max(0) as u32;

        // Resolve the target for the prevention shield
        let protected = resolve_prevention_target(game, &self.target, ctx)?;

        // Create the prevention shield
        let shield = PreventionShield::new(
            ctx.source,
            ctx.controller,
            protected,
            Some(amount),
            self.duration.clone(),
        )
        .with_filter(self.damage_filter.clone());

        // Add the shield to the prevention effect manager
        game.prevention_effects.add_shield(shield);

        Ok(EffectOutcome::resolved())
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "target to protect"
    }
}

/// Resolve a ChooseSpec to a PreventionTarget.
fn resolve_prevention_target(
    game: &GameState,
    target_spec: &ChooseSpec,
    ctx: &ExecutionContext,
) -> Result<PreventionTarget, ExecutionError> {
    match target_spec {
        ChooseSpec::Source => Ok(PreventionTarget::Permanent(ctx.source)),

        ChooseSpec::SourceController => Ok(PreventionTarget::Player(ctx.controller)),

        ChooseSpec::Player(filter) => {
            match filter {
                PlayerFilter::You => Ok(PreventionTarget::You),
                PlayerFilter::Opponent => {
                    // For "target opponent", use the first target player
                    for target in &ctx.targets {
                        if let ResolvedTarget::Player(player_id) = target {
                            return Ok(PreventionTarget::Player(*player_id));
                        }
                    }
                    Err(ExecutionError::InvalidTarget)
                }
                PlayerFilter::Specific(id) => Ok(PreventionTarget::Player(*id)),
                PlayerFilter::Any => {
                    // Use selected target
                    for target in &ctx.targets {
                        if let ResolvedTarget::Player(player_id) = target {
                            return Ok(PreventionTarget::Player(*player_id));
                        }
                    }
                    Err(ExecutionError::InvalidTarget)
                }
                _ => {
                    // For other filters, try to find a matching target
                    for target in &ctx.targets {
                        if let ResolvedTarget::Player(player_id) = target {
                            return Ok(PreventionTarget::Player(*player_id));
                        }
                    }
                    Err(ExecutionError::InvalidTarget)
                }
            }
        }

        ChooseSpec::Object(filter) => {
            // For object filters, use a specific permanent if one is targeted
            for target in &ctx.targets {
                if let ResolvedTarget::Object(object_id) = target {
                    return Ok(PreventionTarget::Permanent(*object_id));
                }
            }
            // Fallback to filter-based target
            Ok(PreventionTarget::PermanentsMatching(filter.clone()))
        }

        ChooseSpec::AnyTarget => {
            // Check for any type of target
            if let Some(target) = ctx.targets.first() {
                match target {
                    ResolvedTarget::Object(object_id) => {
                        return Ok(PreventionTarget::Permanent(*object_id));
                    }
                    ResolvedTarget::Player(player_id) => {
                        return Ok(PreventionTarget::Player(*player_id));
                    }
                }
            }
            Err(ExecutionError::InvalidTarget)
        }

        ChooseSpec::SourceOwner => {
            if let Some(source) = game.object(ctx.source) {
                Ok(PreventionTarget::Player(source.owner))
            } else {
                Err(ExecutionError::ObjectNotFound(ctx.source))
            }
        }

        _ => {
            // For other target specs, try resolved targets
            if let Some(target) = ctx.targets.first() {
                match target {
                    ResolvedTarget::Object(object_id) => {
                        return Ok(PreventionTarget::Permanent(*object_id));
                    }
                    ResolvedTarget::Player(player_id) => {
                        return Ok(PreventionTarget::Player(*player_id));
                    }
                }
            }
            Err(ExecutionError::InvalidTarget)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::effect::EffectResult;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::Object;
    use crate::types::CardType;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    fn make_creature_card(card_id: u32, name: &str) -> crate::card::Card {
        CardBuilder::new(CardId::from_raw(card_id), name)
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build()
    }

    fn create_creature(game: &mut GameState, name: &str, controller: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = make_creature_card(id.0 as u32, name);
        let obj = Object::from_card(id, &card, controller, Zone::Battlefield);
        game.add_object(obj);
        id
    }

    #[test]
    fn test_prevent_damage_to_you() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = PreventDamageEffect::to_you(3, Until::EndOfTurn);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Resolved);
        assert_eq!(game.prevention_effects.shields().len(), 1);
    }

    #[test]
    fn test_prevent_damage_to_creature() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let creature = create_creature(&mut game, "Bear", alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature)]);

        let effect = PreventDamageEffect::any_target(2, Until::EndOfTurn);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Resolved);
        assert_eq!(game.prevention_effects.shields().len(), 1);
    }

    #[test]
    fn test_prevent_damage_to_player() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Player(bob)]);

        let effect = PreventDamageEffect::any_target(5, Until::EndOfTurn);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Resolved);
        assert_eq!(game.prevention_effects.shields().len(), 1);
    }

    #[test]
    fn test_prevent_damage_variable_amount() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice).with_x(7);
        let effect =
            PreventDamageEffect::new(Value::X, ChooseSpec::SourceController, Until::EndOfTurn);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Resolved);
    }

    #[test]
    fn test_prevent_damage_source() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let creature = create_creature(&mut game, "Self-protecting", alice);

        let mut ctx = ExecutionContext::new_default(creature, alice);
        let effect = PreventDamageEffect::new(2, ChooseSpec::Source, Until::EndOfTurn);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Resolved);
    }

    #[test]
    fn test_prevent_damage_clone_box() {
        let effect = PreventDamageEffect::to_you(3, Until::EndOfTurn);
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("PreventDamageEffect"));
    }
}
