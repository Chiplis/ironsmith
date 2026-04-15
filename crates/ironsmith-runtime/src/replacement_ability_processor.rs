//! Replacement ability processor.
//!
//! This module converts static abilities into replacement effects that can be
//! registered with the `ReplacementEffectManager`.
//!
//! Per MTG rules, replacement effects can function outside the battlefield
//! when the source text says so (for example, "from anywhere" effects like
//! Darksteel Colossus). We therefore scan all objects and respect each
//! ability's functional zones instead of assuming every replacement effect
//! comes only from battlefield permanents.

use crate::ability::AbilityKind;
use crate::game_state::GameState;
use crate::replacement::ReplacementEffect;

/// Generate all replacement effects from static abilities in zones where they function.
///
/// This scans all objects for static abilities that generate replacement effects
/// and returns the corresponding `ReplacementEffect` structs.
///
/// This function is called during game state refresh to ensure that static ability
/// replacement effects are properly registered.
pub fn generate_replacement_effects_from_abilities(game: &GameState) -> Vec<ReplacementEffect> {
    let mut effects = Vec::new();

    let object_ids = game.object_ids_in_deterministic_order();

    // Iterate over all objects and apply static abilities only in zones where they function.
    for object_id in object_ids {
        if let Some(object) = game.object(object_id) {
            let controller = object.controller;
            let zone = object.zone;

            // Process each static ability on the object.
            for ability in &object.abilities {
                if let AbilityKind::Static(static_ability) = &ability.kind {
                    if !ability.functions_in(&zone) {
                        continue;
                    }
                    if let Some(effect) =
                        static_ability.generate_replacement_effect(object_id, controller)
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
    use super::generate_replacement_effects_from_abilities;
    use crate::cards::CardDefinitionBuilder;
    use crate::cards::basic_island;
    use crate::game_state::GameState;
    use crate::ids::CardId;
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
        assert_eq!(effect.priority_override, None);
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
        assert_eq!(effect.priority_override, None);
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

    #[test]
    fn test_generate_replacements_respects_nonbattlefield_functional_zones() {
        let alice = PlayerId::from_index(0);
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);

        let darksteel = CardDefinitionBuilder::new(CardId::new(), "Darksteel Test")
            .card_types(vec![
                crate::types::CardType::Artifact,
                crate::types::CardType::Creature,
            ])
            .shuffle_into_library_from_graveyard()
            .build();
        let island = basic_island();

        let darksteel_id = game.create_object_from_definition(&darksteel, alice, Zone::Hand);
        game.create_object_from_definition(&island, alice, Zone::Battlefield);

        let effects = generate_replacement_effects_from_abilities(&game);
        assert!(
            effects.iter().any(|effect| {
                effect.source == darksteel_id
                    && matches!(
                        effect.replacement,
                        ReplacementAction::ChangeDestination(Zone::Library)
                    )
            }),
            "expected nonbattlefield shuffle replacement to be generated from hand"
        );
    }
}
