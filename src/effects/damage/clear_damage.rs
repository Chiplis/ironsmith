//! Clear damage effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::executor::{ExecutionContext, ExecutionError, ResolvedTarget};
use crate::game_state::GameState;
use crate::target::ChooseSpec;

/// Effect that clears all damage from a creature.
///
/// This is primarily used by regeneration, which removes all damage
/// from the creature as part of the replacement effect.
///
/// # Fields
///
/// * `target` - The creature to clear damage from
///
/// # Example
///
/// ```ignore
/// // Clear damage from a specific creature
/// let effect = ClearDamageEffect::new(ChooseSpec::SpecificObject(creature_id));
///
/// // Clear damage from the source creature
/// let effect = ClearDamageEffect::source();
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ClearDamageEffect {
    /// The targeting specification.
    pub target: ChooseSpec,
}

impl ClearDamageEffect {
    /// Create a new clear damage effect.
    pub fn new(target: ChooseSpec) -> Self {
        Self { target }
    }

    /// Create an effect that clears damage from the source.
    pub fn source() -> Self {
        Self::new(ChooseSpec::Source)
    }

    /// Create an effect that clears damage from a specific object.
    pub fn specific(object_id: crate::ids::ObjectId) -> Self {
        Self::new(ChooseSpec::SpecificObject(object_id))
    }
}

impl EffectExecutor for ClearDamageEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        // Resolve the target creature
        let target_id = match &self.target {
            ChooseSpec::Source => ctx.source,
            ChooseSpec::SpecificObject(id) => *id,
            ChooseSpec::Object(_) => {
                // Get from resolved targets
                ctx.targets
                    .iter()
                    .find_map(|t| {
                        if let ResolvedTarget::Object(id) = t {
                            Some(*id)
                        } else {
                            None
                        }
                    })
                    .ok_or(ExecutionError::InvalidTarget)?
            }
            _ => return Err(ExecutionError::InvalidTarget),
        };

        // Clear all damage from the creature
        game.clear_damage(target_id);

        Ok(EffectOutcome::resolved())
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "creature to clear damage from"
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
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Green],
            ]))
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
    fn test_clear_damage_basic() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature(&mut game, "Bear", alice);

        // Deal some damage
        game.mark_damage(creature_id, 2);
        assert_eq!(game.damage_on(creature_id), 2);

        let mut ctx = ExecutionContext::new_default(creature_id, alice);
        let effect = ClearDamageEffect::source();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Resolved);
        assert_eq!(game.damage_on(creature_id), 0);
    }

    #[test]
    fn test_clear_damage_specific_target() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature(&mut game, "Bear", alice);
        let source_id = game.new_object_id();

        // Deal some damage
        game.mark_damage(creature_id, 3);
        assert_eq!(game.damage_on(creature_id), 3);

        let mut ctx = ExecutionContext::new_default(source_id, alice);
        let effect = ClearDamageEffect::specific(creature_id);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Resolved);
        assert_eq!(game.damage_on(creature_id), 0);
    }

    #[test]
    fn test_clear_damage_no_damage() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature(&mut game, "Bear", alice);

        // No damage dealt
        assert_eq!(game.damage_on(creature_id), 0);

        let mut ctx = ExecutionContext::new_default(creature_id, alice);
        let effect = ClearDamageEffect::source();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Should succeed even with no damage
        assert_eq!(result.result, EffectResult::Resolved);
        assert_eq!(game.damage_on(creature_id), 0);
    }
}
