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
use crate::continuous::{EffectTarget, Modification};
use crate::events::context::EventContext;
use crate::events::traits::{GameEventType, ReplacementMatcher, ReplacementPriority};
use crate::events::zones::matchers::{
    ThisWouldEnterBattlefieldMatcher, WouldEnterBattlefieldMatcher,
};
use crate::game_state::GameState;
use crate::replacement::ReplacementEffect;

#[derive(Debug)]
struct GrantedReplacementMatcher {
    grant_target: Box<dyn ReplacementMatcher>,
    granted_ability: Box<dyn ReplacementMatcher>,
}

impl Clone for GrantedReplacementMatcher {
    fn clone(&self) -> Self {
        Self {
            grant_target: self.grant_target.clone_box(),
            granted_ability: self.granted_ability.clone_box(),
        }
    }
}

impl ReplacementMatcher for GrantedReplacementMatcher {
    fn matches_event(&self, event: &dyn GameEventType, ctx: &EventContext) -> bool {
        self.grant_target.matches_event(event, ctx)
            && self.granted_ability.matches_event(event, ctx)
    }

    fn priority(&self) -> ReplacementPriority {
        self.granted_ability.priority()
    }

    fn display(&self) -> String {
        format!(
            "{} and {}",
            self.grant_target.display(),
            self.granted_ability.display()
        )
    }
}

fn replacement_matcher_for_effect_target(
    target: &EffectTarget,
) -> Option<Box<dyn ReplacementMatcher>> {
    match target {
        EffectTarget::Filter(filter) => {
            Some(Box::new(WouldEnterBattlefieldMatcher::new(filter.clone())))
        }
        EffectTarget::AllPermanents => Some(Box::new(WouldEnterBattlefieldMatcher::any())),
        EffectTarget::AllCreatures => Some(Box::new(WouldEnterBattlefieldMatcher::creature())),
        EffectTarget::Source => Some(Box::new(ThisWouldEnterBattlefieldMatcher)),
        EffectTarget::Specific(object_id) => {
            let filter = crate::target::ObjectFilter {
                specific: Some(*object_id),
                ..crate::target::ObjectFilter::permanent()
            };
            Some(Box::new(WouldEnterBattlefieldMatcher::new(filter)))
        }
        EffectTarget::AttachedTo(_) => None,
    }
}

fn replacement_effects_from_granted_abilities(
    game: &GameState,
    source: crate::ids::ObjectId,
    controller: crate::ids::PlayerId,
    static_ability: &crate::static_abilities::StaticAbility,
) -> Vec<ReplacementEffect> {
    static_ability
        .generate_effects(source, controller, game)
        .into_iter()
        .filter(|effect| {
            crate::continuous::continuous_effect_duration_and_condition_are_active(effect, game)
        })
        .filter_map(|effect| {
            let Modification::AddAbility(granted_ability) = effect.modification else {
                return None;
            };
            let mut replacement =
                granted_ability.generate_replacement_effect(source, controller)?;
            let grant_target = replacement_matcher_for_effect_target(&effect.applies_to)?;
            let granted_ability = replacement.matcher.take()?;
            replacement.matcher = Some(Box::new(GrantedReplacementMatcher {
                grant_target,
                granted_ability,
            }));
            Some(replacement)
        })
        .collect()
}

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
            let controller = game.controller_of(object);
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
                    effects.extend(replacement_effects_from_granted_abilities(
                        game,
                        object_id,
                        controller,
                        static_ability,
                    ));
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

    #[cfg(ironsmith_runtime_parser_tests)]
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
