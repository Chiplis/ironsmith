//! Unified grant effect implementation.
//!
//! This module provides a generic effect for granting abilities or alternative
//! casting methods to cards. It replaces bespoke effects like `GrantFlashbackUntilEOTEffect`
//! with a unified approach.
//!
//! # Examples
//!
//! ```ignore
//! // Snapcaster Mage: Grant flashback until end of turn
//! Effect::grant(
//!     Grantable::flashback_use_targets_cost(),
//!     target,
//!     GrantDuration::UntilEndOfTurn,
//! )
//!
//! // Some hypothetical card: Grant flying until end of turn
//! Effect::grant(
//!     Grantable::ability(StaticAbility::flying()),
//!     target,
//!     GrantDuration::UntilEndOfTurn,
//! )
//! ```

use crate::alternative_cast::AlternativeCastingMethod;
use crate::effect::{EffectOutcome, EffectResult};
use crate::effects::EffectExecutor;
use crate::effects::helpers::find_target_object;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::grant::{GrantDuration, Grantable};
use crate::grant_registry::GrantSource;
use crate::target::ChooseSpec;
use crate::types::CardType;
use crate::zone::Zone;

/// Effect that grants something to a target card.
///
/// This is the unified effect for granting abilities or alternative casting methods
/// to cards. It handles:
/// - Granting static abilities (flash, flying, etc.)
/// - Granting alternative casting methods (flashback, escape, etc.)
/// - Special case: flashback using target's mana cost (Snapcaster Mage)
///
/// The grant lasts for the specified duration (typically until end of turn).
#[derive(Debug, Clone, PartialEq)]
pub struct GrantEffect {
    /// What to grant (ability, alternative cast, or flashback-use-targets-cost).
    pub grantable: Grantable,
    /// Target specification for the card to grant to.
    pub target: ChooseSpec,
    /// How long the grant lasts.
    pub duration: GrantDuration,
}

impl GrantEffect {
    /// Create a new grant effect.
    pub fn new(grantable: Grantable, target: ChooseSpec, duration: GrantDuration) -> Self {
        Self {
            grantable,
            target,
            duration,
        }
    }

    /// Create an effect that grants flashback until end of turn using the target's mana cost.
    /// This is the Snapcaster Mage pattern.
    pub fn flashback_until_eot(target: ChooseSpec) -> Self {
        Self::new(
            Grantable::flashback_use_targets_cost(),
            target,
            GrantDuration::UntilEndOfTurn,
        )
    }
}

impl EffectExecutor for GrantEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        // Get the target from resolved targets
        let target_id = find_target_object(&ctx.targets)?;

        // Verify the target is still valid
        let obj = game
            .object(target_id)
            .ok_or(ExecutionError::ObjectNotFound(target_id))?;

        let owner = obj.owner;
        let zone = obj.zone;

        // Calculate expiration
        let expires = match self.duration {
            GrantDuration::UntilEndOfTurn => game.turn.turn_number,
            GrantDuration::Forever => u32::MAX,
        };

        let source_id = ctx.source;
        let grant_source = GrantSource::Effect {
            source_id,
            expires_end_of_turn: expires,
        };

        match &self.grantable {
            Grantable::Ability(ability) => {
                // Grant a static ability
                game.grant_registry.grant_ability_to_card(
                    target_id,
                    zone,
                    owner,
                    ability.clone(),
                    grant_source,
                );
                Ok(EffectOutcome::resolved())
            }
            Grantable::AlternativeCast(method) => {
                // Grant an alternative casting method
                game.grant_registry.grant_alternative_cast_to_card(
                    target_id,
                    zone,
                    owner,
                    method.clone(),
                    grant_source,
                );
                Ok(EffectOutcome::resolved())
            }
            Grantable::FlashbackUseTargetsCost => {
                // Special case: grant flashback using the target's mana cost
                // This is the Snapcaster Mage pattern

                // Verify it's an instant or sorcery
                if !obj.has_card_type(CardType::Instant) && !obj.has_card_type(CardType::Sorcery) {
                    return Ok(EffectOutcome::from_result(EffectResult::TargetInvalid));
                }

                // Must be in graveyard for flashback to make sense
                if zone != Zone::Graveyard {
                    return Ok(EffectOutcome::from_result(EffectResult::TargetInvalid));
                }

                // Get the mana cost
                let flashback_cost = match &obj.mana_cost {
                    Some(cost) => cost.clone(),
                    None => return Ok(EffectOutcome::from_result(EffectResult::Impossible)),
                };

                game.grant_registry.grant_alternative_cast_to_card(
                    target_id,
                    Zone::Graveyard,
                    owner,
                    AlternativeCastingMethod::Flashback {
                        cost: flashback_cost,
                    },
                    grant_source,
                );

                Ok(EffectOutcome::resolved())
            }
            Grantable::PlayFrom => {
                // PlayFrom is typically granted via grant_to_filter (Yawgmoth's Will)
                // rather than targeting individual cards. If used here, just grant it.
                game.grant_registry.grant_to_card(
                    target_id,
                    zone,
                    owner,
                    Grantable::PlayFrom,
                    grant_source,
                );
                Ok(EffectOutcome::resolved())
            }
        }
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        match &self.grantable {
            Grantable::FlashbackUseTargetsCost => "instant or sorcery card in a graveyard",
            Grantable::Ability(_) => "card",
            Grantable::AlternativeCast(_) => "card",
            Grantable::PlayFrom => "card",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::filter::ObjectFilter;
    use crate::ids::{CardId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::static_abilities::StaticAbility;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    fn create_instant_in_graveyard(
        game: &mut GameState,
        name: &str,
        owner: PlayerId,
    ) -> crate::ids::ObjectId {
        let card = CardBuilder::new(CardId::from_raw(1), name)
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Blue]]))
            .card_types(vec![CardType::Instant])
            .build();

        game.create_object_from_card(&card, owner, Zone::Graveyard)
    }

    #[test]
    fn test_grant_flashback_until_eot() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let instant_id = create_instant_in_graveyard(&mut game, "Counterspell", alice);

        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.targets = vec![crate::executor::ResolvedTarget::Object(instant_id)];

        let effect = GrantEffect::flashback_until_eot(ChooseSpec::Object(
            ObjectFilter::default().in_zone(Zone::Graveyard),
        ));
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Resolved);

        // Check that flashback was granted
        let grants =
            game.grant_registry
                .get_grants_for_card(&game, instant_id, Zone::Graveyard, alice);
        assert!(!grants.is_empty());
        assert!(matches!(
            &grants[0].grantable,
            Grantable::AlternativeCast(AlternativeCastingMethod::Flashback { .. })
        ));
    }

    #[test]
    fn test_grant_ability() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        // Create a creature in hand
        let card = CardBuilder::new(CardId::from_raw(1), "Test Creature")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Red]]))
            .card_types(vec![CardType::Creature])
            .build();
        let creature_id = game.create_object_from_card(&card, alice, Zone::Hand);

        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.targets = vec![crate::executor::ResolvedTarget::Object(creature_id)];

        let effect = GrantEffect::new(
            Grantable::ability(StaticAbility::flash()),
            ChooseSpec::Object(ObjectFilter::default().in_zone(Zone::Hand)),
            GrantDuration::UntilEndOfTurn,
        );
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Resolved);

        // Check that flash was granted
        let grants = game
            .grant_registry
            .get_grants_for_card(&game, creature_id, Zone::Hand, alice);
        assert!(!grants.is_empty());
        match &grants[0].grantable {
            Grantable::Ability(ability) => assert!(ability.has_flash()),
            _ => panic!("Expected ability grant"),
        }
    }

    #[test]
    fn test_grant_flashback_to_non_instant_sorcery_fails() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        // Create a creature in graveyard
        let card = CardBuilder::new(CardId::from_raw(1), "Test Creature")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Red]]))
            .card_types(vec![CardType::Creature])
            .build();
        let creature_id = game.create_object_from_card(&card, alice, Zone::Graveyard);

        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.targets = vec![crate::executor::ResolvedTarget::Object(creature_id)];

        let effect = GrantEffect::flashback_until_eot(ChooseSpec::Object(
            ObjectFilter::default().in_zone(Zone::Graveyard),
        ));
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Should fail because creature is not instant/sorcery
        assert_eq!(result.result, EffectResult::TargetInvalid);
    }
}
