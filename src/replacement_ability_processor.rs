//! Replacement ability processor.
//!
//! This module converts static abilities on permanents into replacement effects
//! that can be registered with the ReplacementEffectManager.
//!
//! Per MTG rules, certain static abilities generate replacement effects that
//! modify how permanents enter the battlefield, how damage is dealt, etc.
//! These effects need to be tracked separately from continuous effects.

use crate::ability::AbilityKind;
use crate::game_state::GameState;
use crate::replacement::ReplacementEffect;

/// Generate all replacement effects from static abilities on the battlefield.
///
/// This scans all permanents for static abilities that generate replacement effects
/// and returns the corresponding ReplacementEffect structs.
///
/// This function is called during game state refresh to ensure that static ability
/// replacement effects are properly registered.
pub fn generate_replacement_effects_from_abilities(game: &GameState) -> Vec<ReplacementEffect> {
    let mut effects = Vec::new();

    // Iterate over all permanents on the battlefield
    for &permanent_id in &game.battlefield {
        if let Some(permanent) = game.object(permanent_id) {
            let controller = permanent.controller;

            // Process each ability on the permanent
            for ability in &permanent.abilities {
                if let AbilityKind::Static(static_ability) = &ability.kind {
                    // Use the trait method to generate replacement effects
                    if let Some(effect) =
                        static_ability.generate_replacement_effect(permanent_id, controller)
                    {
                        effects.push(effect);
                    }
                }
            }
        }
    }

    effects
}

#[cfg(test)]
mod tests {
    use crate::ids::{ObjectId, PlayerId};
    use crate::replacement::ReplacementAction;
    use crate::static_abilities::StaticAbility;
    use crate::zone::Zone;

    #[test]
    fn test_enters_tapped_generates_replacement() {
        let ability = StaticAbility::enters_tapped_ability();
        let effect =
            ability.generate_replacement_effect(ObjectId::from_raw(1), PlayerId::from_index(0));

        assert!(effect.is_some());
        let effect = effect.unwrap();
        assert!(effect.self_replacement);
        // Now using trait-based matcher instead of ReplacementCondition enum
        assert!(
            effect.matcher.is_some(),
            "EntersTapped should use a trait-based matcher"
        );
        assert!(matches!(effect.replacement, ReplacementAction::EnterTapped));
    }

    #[test]
    fn test_flying_does_not_generate_replacement() {
        let ability = StaticAbility::flying();
        let effect =
            ability.generate_replacement_effect(ObjectId::from_raw(1), PlayerId::from_index(0));

        assert!(effect.is_none());
    }

    #[test]
    fn test_shuffle_into_library_generates_replacement() {
        let ability = StaticAbility::shuffle_into_library_from_graveyard();
        let effect =
            ability.generate_replacement_effect(ObjectId::from_raw(1), PlayerId::from_index(0));

        assert!(effect.is_some());
        let effect = effect.unwrap();
        assert!(effect.self_replacement);
        // Now using trait-based matcher instead of ReplacementCondition enum
        assert!(
            effect.matcher.is_some(),
            "ShuffleIntoLibraryFromGraveyard should use a trait-based matcher"
        );
        assert!(matches!(
            effect.replacement,
            ReplacementAction::ChangeDestination(Zone::Library)
        ));
    }
}
