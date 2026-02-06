//! Grant abilities to all matching creatures effect implementation.

use crate::continuous::{EffectSourceType, EffectTarget, Modification};
use crate::effect::{Effect, EffectOutcome, Until};
use crate::effects::{ApplyContinuousEffect, EffectExecutor};
use crate::executor::{ExecutionContext, ExecutionError, execute_effect};
use crate::filter::ObjectFilter;
use crate::game_state::GameState;
use crate::static_abilities::StaticAbility;
use crate::zone::Zone;

/// Effect that grants multiple abilities to all creatures matching a filter for a duration.
///
/// Creates continuous effects that add the specified abilities to matching creatures.
/// Used by Akroma's Will and similar cards.
///
/// # Fields
///
/// * `filter` - Filter to determine which creatures are affected
/// * `abilities` - List of static abilities to grant
///
/// # Example
///
/// ```ignore
/// // Akroma's Will granting multiple abilities
/// let effect = GrantAbilitiesAllEffect::new(
///     ObjectFilter::creature().controlled_by(PlayerFilter::You),
///     vec![StaticAbility::flying(), StaticAbility::vigilance(), StaticAbility::lifelink()],
///     Until::EndOfTurn,
/// );
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct GrantAbilitiesAllEffect {
    /// Filter for which creatures are affected.
    pub filter: ObjectFilter,
    /// Abilities to grant.
    pub abilities: Vec<StaticAbility>,
    /// Duration for the granted abilities.
    pub duration: Until,
}

impl GrantAbilitiesAllEffect {
    /// Create a new grant abilities all effect with explicit duration.
    pub fn new(filter: ObjectFilter, abilities: Vec<StaticAbility>, duration: Until) -> Self {
        Self {
            filter,
            abilities,
            duration,
        }
    }
}

impl EffectExecutor for GrantAbilitiesAllEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        // Per MTG Rule 611.2c, effects from resolving spells/abilities lock their
        // targets at resolution time. We capture which objects match the filter NOW,
        // and the effect will only apply to those specific objects.
        let filter_ctx = game.filter_context_for(ctx.controller, Some(ctx.source));
        let locked_targets: Vec<_> = game
            .battlefield
            .iter()
            .filter_map(|&id| game.object(id))
            .filter(|obj| obj.zone == Zone::Battlefield)
            .filter(|obj| self.filter.matches(obj, &filter_ctx, game))
            .map(|obj| obj.id)
            .collect();

        if self.abilities.is_empty() {
            return Ok(EffectOutcome::resolved());
        }

        let mut outcomes = Vec::new();
        for ability in &self.abilities {
            let apply = ApplyContinuousEffect::new(
                EffectTarget::Filter(self.filter.clone()),
                Modification::AddAbility(ability.clone()),
                self.duration.clone(),
            )
            .with_source_type(EffectSourceType::Resolution {
                locked_targets: locked_targets.clone(),
            });

            outcomes.push(execute_effect(game, &Effect::new(apply), ctx)?);
        }

        Ok(EffectOutcome::aggregate(outcomes))
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::effect::EffectResult;
    use crate::filter::PlayerFilter;
    use crate::ids::{CardId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::types::{CardType, Subtype};
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    fn create_creature(game: &mut GameState, name: &str, owner: PlayerId) -> crate::ids::ObjectId {
        let card = CardBuilder::new(CardId::from_raw(1), name)
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::White],
            ]))
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Human])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();

        game.create_object_from_card(&card, owner, Zone::Battlefield)
    }

    #[test]
    fn test_grant_single_ability() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let _creature = create_creature(&mut game, "Soldier", alice);

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = GrantAbilitiesAllEffect::new(
            ObjectFilter::creature(),
            vec![StaticAbility::flying()],
            Until::EndOfTurn,
        );
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Resolved);

        // Should have one continuous effect
        assert_eq!(game.continuous_effects.effects_sorted().len(), 1);
    }

    #[test]
    fn test_grant_multiple_abilities() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let _creature = create_creature(&mut game, "Soldier", alice);

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = GrantAbilitiesAllEffect::new(
            ObjectFilter::creature(),
            vec![
                StaticAbility::flying(),
                StaticAbility::vigilance(),
                StaticAbility::lifelink(),
            ],
            Until::EndOfTurn,
        );
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Resolved);

        // Should have three continuous effects (one per ability)
        assert_eq!(game.continuous_effects.effects_sorted().len(), 3);
    }

    #[test]
    fn test_filter_only_your_creatures() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();

        let _alice_creature = create_creature(&mut game, "Alice's Soldier", alice);
        let _bob_creature = create_creature(&mut game, "Bob's Soldier", bob);

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = GrantAbilitiesAllEffect::new(
            ObjectFilter::creature().controlled_by(PlayerFilter::You),
            vec![StaticAbility::flying()],
            Until::EndOfTurn,
        );
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Resolved);

        // Should have one continuous effect
        let effects = game.continuous_effects.effects_sorted();
        assert_eq!(effects.len(), 1);

        // Effect should filter to controller's creatures only
        match &effects[0].applies_to {
            EffectTarget::Filter(f) => {
                assert!(matches!(f.controller, Some(PlayerFilter::You)));
            }
            _ => panic!("Expected Filter target"),
        }
    }

    #[test]
    fn test_empty_abilities_list() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect =
            GrantAbilitiesAllEffect::new(ObjectFilter::creature(), vec![], Until::EndOfTurn);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Resolved);

        // Should have no continuous effects
        assert_eq!(game.continuous_effects.effects_sorted().len(), 0);
    }

    #[test]
    fn test_clone_box() {
        let effect = GrantAbilitiesAllEffect::new(
            ObjectFilter::creature(),
            vec![StaticAbility::flying()],
            Until::EndOfTurn,
        );
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("GrantAbilitiesAllEffect"));
    }
}
