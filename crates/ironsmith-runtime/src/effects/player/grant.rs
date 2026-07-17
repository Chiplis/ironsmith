//! Unified grant effect implementation.
//!
//! This module provides a generic effect for granting abilities or alternative
//! casting methods to cards.
//!
//! # Examples
//!
//! ```ignore
//! // Grant flashback until end of turn using the card's mana cost
//! Effect::grant(
//!     Grantable::flashback_from_cards_mana_cost(),
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

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_single_object_for_effect;
use crate::effects::player::grant_by_spec::next_turn_number_for_player;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::grant::{DerivedAlternativeCastRuntimeExt, GrantDuration, Grantable};
use crate::grant_registry::GrantSource;
use crate::target::ChooseSpec;
pub type GrantEffect = ironsmith_core::GrantEffect<Grantable, GrantDuration>;

/// Effect that grants something to a target card.
///
/// This is the unified effect for granting abilities or alternative casting methods
/// to cards. It handles:
/// - Granting static abilities (flash, flying, etc.)
/// - Granting alternative casting methods (flashback, escape, etc.)
/// - Derived alternative casting methods that use the granted card's mana cost
///
/// The grant lasts for the specified duration (typically until end of turn).
impl EffectExecutor for GrantEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let target_id = resolve_single_object_for_effect(game, ctx, &self.target)?;

        let obj = game
            .object(target_id)
            .ok_or(ExecutionError::ObjectNotFound(target_id))?;

        let owner = obj.owner;
        let zone = obj.zone;

        // Calculate expiration
        let expires = match self.duration {
            GrantDuration::UntilEndOfTurn => game.turn.turn_number,
            GrantDuration::Forever => u32::MAX,
            GrantDuration::UntilYourNextTurnEnd => {
                next_turn_number_for_player(game, ctx.controller)
            }
        };

        let source_id = ctx.source;
        let grant_source = match self.duration {
            GrantDuration::UntilYourNextTurnEnd => {
                GrantSource::until_player_next_turn_end(source_id, ctx.controller, expires)
            }
            GrantDuration::UntilEndOfTurn | GrantDuration::Forever => GrantSource::Effect {
                source_id,
                expires_end_of_turn: expires,
            },
        };

        match &self.grantable {
            Grantable::Ability(ability) => {
                // Grant a static ability
                game.effect_store.grant_registry.grant_ability_to_card(
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
                game.effect_store
                    .grant_registry
                    .grant_alternative_cast_to_card(
                        target_id,
                        zone,
                        owner,
                        method.clone(),
                        grant_source,
                    );
                Ok(EffectOutcome::resolved())
            }
            Grantable::DerivedAlternativeCast(spec) => {
                if spec.materialize_for(obj).is_none() {
                    return Ok(EffectOutcome::target_invalid());
                }

                game.effect_store.grant_registry.grant_to_card(
                    target_id,
                    zone,
                    owner,
                    self.grantable.clone(),
                    grant_source,
                );
                Ok(EffectOutcome::resolved())
            }
            Grantable::PlayFrom => {
                // PlayFrom is typically granted via grant_to_filter (Yawgmoth's Will)
                // rather than targeting individual cards. If used here, just grant it.
                game.effect_store.grant_registry.grant_to_card(
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

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        match &self.grantable {
            Grantable::DerivedAlternativeCast(_) => "card",
            Grantable::Ability(_) => "card",
            Grantable::AlternativeCast(_) => "card",
            Grantable::PlayFrom => "card",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alternative_cast::AlternativeCastingMethod;
    use crate::card::CardBuilder;
    use crate::filter::ObjectFilter;
    use crate::ids::{CardId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::static_abilities::StaticAbility;
    use crate::types::CardType;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
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
    fn test_grant_derived_flashback_until_eot() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let instant_id = create_instant_in_graveyard(&mut game, "Counterspell", alice);

        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.targets = vec![crate::effects::ResolvedTarget::Object(instant_id)];

        let effect = GrantEffect::new(
            Grantable::flashback_from_cards_mana_cost(),
            ChooseSpec::Object(ObjectFilter::default().in_zone(Zone::Graveyard)),
            GrantDuration::UntilEndOfTurn,
        );
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.status, crate::effect::OutcomeStatus::Succeeded);

        // Check that flashback was granted
        let grants = game.effect_store.grant_registry.get_grants_for_card(
            &game,
            instant_id,
            Zone::Graveyard,
            alice,
        );
        assert!(!grants.is_empty());
        assert!(matches!(
            &grants[0].grantable,
            Grantable::DerivedAlternativeCast(
                crate::grant::DerivedAlternativeCast::FlashbackFromCardManaCost { .. }
            )
        ));

        let granted_casts = game
            .effect_store
            .grant_registry
            .granted_alternative_casts_for_card(&game, instant_id, Zone::Graveyard, alice);
        assert!(matches!(
            granted_casts.first().map(|grant| &grant.method),
            Some(AlternativeCastingMethod::Flashback { .. })
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
        ctx.targets = vec![crate::effects::ResolvedTarget::Object(creature_id)];

        let effect = GrantEffect::new(
            Grantable::ability(StaticAbility::flash()),
            ChooseSpec::Object(ObjectFilter::default().in_zone(Zone::Hand)),
            GrantDuration::UntilEndOfTurn,
        );
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.status, crate::effect::OutcomeStatus::Succeeded);

        // Check that flash was granted
        let grants = game.effect_store.grant_registry.get_grants_for_card(
            &game,
            creature_id,
            Zone::Hand,
            alice,
        );
        assert!(!grants.is_empty());
        match &grants[0].grantable {
            Grantable::Ability(ability) => assert!(ability.has_flash()),
            _ => panic!("Expected ability grant"),
        }
    }

    #[test]
    fn targeted_grant_lasts_through_the_controllers_next_turn() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(2), "Next-turn Creature")
            .card_types(vec![CardType::Creature])
            .build();
        let creature_id = game.create_object_from_card(&card, bob, Zone::Hand);
        let flash = StaticAbility::flash();

        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.targets = vec![crate::effects::ResolvedTarget::Object(creature_id)];
        GrantEffect::new(
            Grantable::ability(flash.clone()),
            ChooseSpec::Object(ObjectFilter::default().in_zone(Zone::Hand)),
            GrantDuration::UntilYourNextTurnEnd,
        )
        .execute(&mut game, &mut ctx)
        .expect("the next-turn duration must be executable");

        assert_eq!(
            game.effect_store.grant_registry.grants[0].source,
            GrantSource::until_player_next_turn_end(source, alice, 3)
        );
        for turn_number in [1, 2, 3] {
            game.turn.turn_number = turn_number;
            assert!(game.effect_store.grant_registry.card_has_granted_ability(
                &game,
                creature_id,
                Zone::Hand,
                bob,
                &flash,
            ));
        }
        game.turn.turn_number = 4;
        assert!(!game.effect_store.grant_registry.card_has_granted_ability(
            &game,
            creature_id,
            Zone::Hand,
            bob,
            &flash,
        ));
    }

    #[test]
    fn targeted_next_turn_grant_uses_a_queued_extra_turn() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(3), "Extra-turn Creature")
            .card_types(vec![CardType::Creature])
            .build();
        let creature_id = game.create_object_from_card(&card, alice, Zone::Hand);
        game.turn_store.extra_turns.push(alice);

        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.targets = vec![crate::effects::ResolvedTarget::Object(creature_id)];
        GrantEffect::new(
            Grantable::ability(StaticAbility::flash()),
            ChooseSpec::Object(ObjectFilter::default().in_zone(Zone::Hand)),
            GrantDuration::UntilYourNextTurnEnd,
        )
        .execute(&mut game, &mut ctx)
        .expect("the next-turn duration must be executable");

        assert_eq!(
            game.effect_store.grant_registry.grants[0].source,
            GrantSource::until_player_next_turn_end(source, alice, 2)
        );
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
        ctx.targets = vec![crate::effects::ResolvedTarget::Object(creature_id)];

        let effect = GrantEffect::new(
            Grantable::flashback_from_cards_mana_cost(),
            ChooseSpec::Object(ObjectFilter::default().in_zone(Zone::Graveyard)),
            GrantDuration::UntilEndOfTurn,
        );
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Should fail because creature is not instant/sorcery
        assert_eq!(result.status, crate::effect::OutcomeStatus::TargetInvalid);
    }
}
