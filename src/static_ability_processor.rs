//! Static ability processor.
//!
//! This module generates continuous effects from static abilities on permanents
//! using the trait-based `StaticAbility` system.
//!
//! # Why Some Abilities Return Empty Vectors
//!
//! Many static abilities like Flying, Vigilance, Trample, etc. return empty
//! vectors from `generate_effects()`. This is intentional:
//!
//! **Self-granting keywords** are abilities that only affect the object they're
//! on. They don't need to be converted into continuous effects because they're
//! checked directly on the Object when relevant:
//!
//! - **Flying**: Checked during declare blockers step
//! - **First Strike**: Checked during combat damage assignment
//! - **Indestructible**: Checked when destruction would occur
//! - **Hexproof**: Checked when targeting validation happens
//!
//! These are stored on the Object's `abilities` list and can be looked up with
//! trait methods like `ability.has_flying()` or through calculated
//! characteristics when continuous effects might modify them.
//!
//! **Effect-generating abilities** like Anthems ("Creatures you control get +1/+1")
//! and ability grants ("Creatures you control have flying") DO create continuous
//! effects because they affect other objects.
//!
//! # MTG Rules Reference
//!
//! Per Rule 611.3a, static abilities generate continuous effects that apply
//! dynamically to all objects matching their criteria, as opposed to resolution
//! effects which lock their targets at resolution time (Rule 611.2c).

use crate::ability::AbilityKind;
use crate::continuous::ContinuousEffect;
use crate::game_state::GameState;

/// Generate all continuous effects from static abilities in zones where they function.
///
/// This scans all objects for static abilities and generates the corresponding
/// continuous effects. These effects have `source_type: StaticAbility`, which
/// means they apply dynamically (the filter is re-evaluated each time).
///
/// This function is called during characteristic calculation to ensure that
/// static ability effects are properly integrated into the layer system.
pub fn generate_continuous_effects_from_static_abilities(
    game: &GameState,
) -> Vec<ContinuousEffect> {
    let mut effects = Vec::new();

    let mut object_ids: Vec<_> = game.objects_iter().map(|object| object.id).collect();
    object_ids.sort_unstable();

    // Iterate over all objects and apply static abilities only in zones where they function.
    for object_id in object_ids {
        if let Some(object) = game.object(object_id) {
            let controller = object.controller;
            let zone = object.zone;

            // Process each static ability on the object
            for ability in &object.abilities {
                if let AbilityKind::Static(static_ability) = &ability.kind {
                    if !ability.functions_in(&zone) {
                        continue;
                    }
                    if !static_ability.is_active(game, object_id) {
                        continue;
                    }
                    // Generate effects directly from the trait method
                    let mut ability_effects =
                        static_ability.generate_effects(object_id, controller, game);
                    // Static ability effect timestamps come from the source object's
                    // current-zone entry timestamp (CR 613.7a/613.7d behavior).
                    if let Some(ts) = game.continuous_effects.get_entry_timestamp(object_id) {
                        for effect in &mut ability_effects {
                            effect.timestamp = ts;
                        }
                    }
                    effects.extend(ability_effects);
                }
            }
        }
    }

    effects
}

/// Get all continuous effects including both registered effects and static ability effects.
///
/// This combines:
/// - Effects registered in the ContinuousEffectManager (from spells/abilities that resolved)
/// - Effects generated dynamically from static abilities in their functional zones
///
/// This is the main entry point for getting all effects that should be applied
/// during characteristic calculation.
pub fn get_all_continuous_effects(game: &GameState) -> Vec<ContinuousEffect> {
    // Get registered effects (from resolved spells/abilities), cloned
    let mut effects: Vec<ContinuousEffect> = game
        .continuous_effects
        .effects_sorted()
        .into_iter()
        .cloned()
        .collect();

    // Add effects from static abilities
    let static_effects = generate_continuous_effects_from_static_abilities(game);
    effects.reserve(static_effects.len());
    effects.extend(static_effects);

    effects
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::continuous::{EffectSourceType, Modification};
    use crate::ids::{ObjectId, PlayerId};
    use crate::static_abilities::StaticAbility;
    use crate::target::ObjectFilter;

    #[test]
    fn test_anthem_generates_effect() {
        let anthem = StaticAbility::anthem(ObjectFilter::creature().you_control(), 1, 1);

        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let effects =
            anthem.generate_effects(ObjectId::from_raw(1), PlayerId::from_index(0), &game);

        assert_eq!(effects.len(), 1);
        let effect = &effects[0];
        assert!(matches!(
            effect.modification,
            Modification::ModifyPowerToughness {
                power: 1,
                toughness: 1
            }
        ));
        assert!(matches!(
            effect.source_type,
            EffectSourceType::StaticAbility
        ));
    }

    #[test]
    fn test_self_granting_keywords_no_effect() {
        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);

        // Flying doesn't generate continuous effects
        let flying = StaticAbility::flying();
        let effects =
            flying.generate_effects(ObjectId::from_raw(1), PlayerId::from_index(0), &game);
        assert!(effects.is_empty());

        // Trample doesn't generate continuous effects
        let trample = StaticAbility::trample();
        let effects =
            trample.generate_effects(ObjectId::from_raw(1), PlayerId::from_index(0), &game);
        assert!(effects.is_empty());
    }

    #[test]
    fn test_grant_ability_generates_effect() {
        let grant = StaticAbility::grant_ability(
            ObjectFilter::creature().you_control(),
            StaticAbility::haste(),
        );

        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let effects = grant.generate_effects(ObjectId::from_raw(1), PlayerId::from_index(0), &game);

        assert_eq!(effects.len(), 1);
        let effect = &effects[0];
        assert!(matches!(effect.modification, Modification::AddAbility(_)));
    }
}
