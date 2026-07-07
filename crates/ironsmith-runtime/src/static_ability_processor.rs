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

use crate::FxMap;
use crate::ability::AbilityKind;
use crate::continuous::{
    ContinuousEffect, ContinuousEffectGroupId, EffectSourceType, EffectTarget, Layer,
    TextBoxOverlay,
};
use crate::game_state::GameState;
use crate::ids::ObjectId;
use crate::zone::Zone;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
enum TextBoxQueryScope {
    None,
    Specific(Vec<ObjectId>),
    AllBattlefield,
}

impl TextBoxQueryScope {
    fn includes(&self, object_id: ObjectId) -> bool {
        match self {
            Self::None => false,
            Self::Specific(ids) => ids.contains(&object_id),
            Self::AllBattlefield => true,
        }
    }
}

fn text_box_query_scope(effects: &[ContinuousEffect]) -> TextBoxQueryScope {
    let mut specific_ids = Vec::new();

    for effect in effects {
        if !matches!(
            effect.modification.layer(),
            Layer::Copy | Layer::Control | Layer::Text
        ) {
            continue;
        }

        if let EffectSourceType::Resolution { locked_targets } = &effect.source_type
            && !locked_targets.is_empty()
        {
            for &id in locked_targets {
                if !specific_ids.contains(&id) {
                    specific_ids.push(id);
                }
            }
            continue;
        }

        match &effect.applies_to {
            EffectTarget::Specific(id) | EffectTarget::AttachedTo(id) => {
                if !specific_ids.contains(id) {
                    specific_ids.push(*id);
                }
            }
            EffectTarget::Source => {
                if !specific_ids.contains(&effect.source) {
                    specific_ids.push(effect.source);
                }
            }
            EffectTarget::Filter(_) | EffectTarget::AllPermanents | EffectTarget::AllCreatures => {
                return TextBoxQueryScope::AllBattlefield;
            }
        }
    }

    if specific_ids.is_empty() {
        TextBoxQueryScope::None
    } else {
        TextBoxQueryScope::Specific(specific_ids)
    }
}

fn next_static_effect_group_id(
    source: ObjectId,
    next_group_ordinal: &mut u16,
) -> ContinuousEffectGroupId {
    let group = ContinuousEffectGroupId::static_source(source, *next_group_ordinal);
    *next_group_ordinal = next_group_ordinal.saturating_add(1);
    group
}

struct GeneratedStaticEffect {
    effect: ContinuousEffect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SourceStaticEffectsKey {
    generation_revision: u64,
    object_revision: u64,
    zone: Zone,
    controller: crate::ids::PlayerId,
    entry_timestamp: Option<u64>,
    text_overlay_revision: Option<u64>,
}

#[derive(Debug, Clone)]
struct SourceStaticEffects {
    key: SourceStaticEffectsKey,
    effects: Arc<Vec<ContinuousEffect>>,
}

#[derive(Debug, Default)]
pub(crate) struct StaticEffectsCache {
    per_source: FxMap<ObjectId, SourceStaticEffects>,
}

fn static_effects_share_scope(a: &GeneratedStaticEffect, b: &GeneratedStaticEffect) -> bool {
    let a_effect = &a.effect;
    let b_effect = &b.effect;
    a_effect.source == b_effect.source
        && a_effect.controller == b_effect.controller
        && a_effect.applies_to == b_effect.applies_to
        && a_effect.duration == b_effect.duration
        && a_effect.expires_end_of_turn == b_effect.expires_end_of_turn
        && a_effect.condition == b_effect.condition
        && a_effect.source_type == b_effect.source_type
}

fn should_infer_multilayer_static_group(
    effects: &[GeneratedStaticEffect],
    indices: &[usize],
) -> bool {
    if indices.len() <= 1 {
        return false;
    }

    let has_type_part = indices
        .iter()
        .any(|&idx| effects[idx].effect.modification.layer() == Layer::Type);
    let has_later_part = indices
        .iter()
        .any(|&idx| effects[idx].effect.modification.layer() > Layer::Type);
    let has_ability_removal = indices.iter().any(|&idx| {
        matches!(
            effects[idx].effect.modification,
            crate::continuous::Modification::RemoveAbility(_)
                | crate::continuous::Modification::RemoveAllAbilities
                | crate::continuous::Modification::RemoveAllAbilitiesExceptMana
                | crate::continuous::Modification::SetAbilities(_)
        )
    });
    let has_pt_setting = indices.iter().any(|&idx| {
        matches!(
            effects[idx].effect.modification,
            crate::continuous::Modification::SetPower { .. }
                | crate::continuous::Modification::SetToughness { .. }
                | crate::continuous::Modification::SetPowerToughness { .. }
        )
    });

    (has_type_part && has_later_part) || (has_ability_removal && has_pt_setting)
}

fn assign_inferred_static_effect_groups(
    effects: &mut [GeneratedStaticEffect],
    source: ObjectId,
    next_group_ordinal: &mut u16,
) {
    let mut assigned = vec![false; effects.len()];

    for i in 0..effects.len() {
        if assigned[i] || effects[i].effect.group.is_some() {
            continue;
        }

        let mut group_indices = Vec::new();
        for j in i..effects.len() {
            if assigned[j] || effects[j].effect.group.is_some() {
                continue;
            }
            if static_effects_share_scope(&effects[i], &effects[j]) {
                group_indices.push(j);
            }
        }

        if !should_infer_multilayer_static_group(effects, &group_indices) {
            continue;
        }

        let group = next_static_effect_group_id(source, next_group_ordinal);
        for idx in group_indices {
            effects[idx].effect.group = Some(group);
            assigned[idx] = true;
        }
    }
}

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
    let registered_effects: Vec<ContinuousEffect> =
        game.effect_store.continuous_effects.effects().to_vec();
    let text_box_scope = text_box_query_scope(&registered_effects);
    let mut text_box_cache: FxMap<ObjectId, TextBoxOverlay> = FxMap::default();

    let object_ids = game.object_ids_in_deterministic_order();
    // Iterate over all objects and apply static abilities only in zones where they function.
    for object_id in object_ids {
        if let Some(object) = game.object(object_id) {
            let mut object_effects = Vec::new();
            let mut next_group_ordinal = 1;
            let zone = object.zone;
            let (controller, abilities) =
                if zone == crate::zone::Zone::Battlefield && text_box_scope.includes(object_id) {
                    let overlay = text_box_cache.entry(object_id).or_insert_with(|| {
                        crate::continuous::text_box_characteristics_with_effects(
                            object_id,
                            game.objects_map(),
                            &registered_effects,
                            &game.battlefield,
                            game.commander_objects(),
                            game,
                        )
                        .map(|chars| TextBoxOverlay::new(chars.compiled_card_text, chars.abilities))
                        .unwrap_or_else(|| {
                            TextBoxOverlay::new(
                                object.compiled_card_text.clone(),
                                object.abilities_vec(),
                            )
                        })
                    });
                    (game.controller_of(object), overlay.abilities.clone())
                } else {
                    (game.controller_of(object), object.abilities_vec())
                };

            // Process each static ability on the object
            for ability in &abilities {
                if let AbilityKind::Static(static_ability) = &ability.kind {
                    if !ability.functions_in(&zone) {
                        continue;
                    }
                    // Generate effects directly from the trait method
                    let mut ability_effects =
                        static_ability.generate_effects(object_id, controller, game);
                    // Static ability effect timestamps come from the source object's
                    // current-zone entry timestamp (CR 613.7a/613.7d behavior).
                    if let Some(ts) = game
                        .effect_store
                        .continuous_effects
                        .get_entry_timestamp(object_id)
                    {
                        for effect in &mut ability_effects {
                            effect.timestamp = ts;
                            effect.originating_static_ability = Some(static_ability.clone());
                        }
                    } else {
                        for effect in &mut ability_effects {
                            effect.originating_static_ability = Some(static_ability.clone());
                        }
                    }
                    object_effects.extend(
                        ability_effects
                            .into_iter()
                            .map(|effect| GeneratedStaticEffect { effect }),
                    );
                }
            }
            assign_inferred_static_effect_groups(
                &mut object_effects,
                object_id,
                &mut next_group_ordinal,
            );
            effects.extend(object_effects.into_iter().map(|generated| generated.effect));
        }
    }

    effects
}

pub(crate) fn generate_continuous_effects_from_static_abilities_cached(
    game: &GameState,
    cache: &mut StaticEffectsCache,
) -> Vec<ContinuousEffect> {
    let registered_effects: Vec<ContinuousEffect> =
        game.effect_store.continuous_effects.effects().to_vec();
    let text_box_scope = text_box_query_scope(&registered_effects);
    let mut text_box_cache: FxMap<ObjectId, TextBoxOverlay> = FxMap::default();
    let text_overlay_revision = game.effect_store.continuous_effects.revision();
    let generation_revision = game.mutation_revision();
    let mut effects = Vec::new();
    let object_ids = game.object_ids_in_deterministic_order();
    let mut seen = std::collections::HashSet::new();

    for object_id in object_ids {
        let Some(object) = game.object(object_id) else {
            continue;
        };
        seen.insert(object_id);
        let zone = object.zone;
        let controller = game.controller_of(object);
        let key = SourceStaticEffectsKey {
            generation_revision,
            object_revision: object.last_modified,
            zone,
            controller,
            entry_timestamp: game
                .effect_store
                .continuous_effects
                .get_entry_timestamp(object_id),
            text_overlay_revision: text_box_scope
                .includes(object_id)
                .then_some(text_overlay_revision),
        };

        if let Some(cached) = cache.per_source.get(&object_id)
            && cached.key == key
        {
            effects.extend(cached.effects.iter().cloned());
            continue;
        }

        let source_effects = generate_static_effects_for_source(
            game,
            object_id,
            &registered_effects,
            &text_box_scope,
            &mut text_box_cache,
        );
        effects.extend(source_effects.iter().cloned());
        cache.per_source.insert(
            object_id,
            SourceStaticEffects {
                key,
                effects: Arc::new(source_effects),
            },
        );
    }

    cache.per_source.retain(|id, _| seen.contains(id));
    effects
}

fn generate_static_effects_for_source(
    game: &GameState,
    object_id: ObjectId,
    registered_effects: &[ContinuousEffect],
    text_box_scope: &TextBoxQueryScope,
    text_box_cache: &mut FxMap<ObjectId, TextBoxOverlay>,
) -> Vec<ContinuousEffect> {
    let Some(object) = game.object(object_id) else {
        return Vec::new();
    };
    let mut object_effects = Vec::new();
    let mut next_group_ordinal = 1;
    let zone = object.zone;
    let (controller, abilities) =
        if zone == crate::zone::Zone::Battlefield && text_box_scope.includes(object_id) {
            let overlay = text_box_cache.entry(object_id).or_insert_with(|| {
                crate::continuous::text_box_characteristics_with_effects(
                    object_id,
                    game.objects_map(),
                    registered_effects,
                    &game.battlefield,
                    game.commander_objects(),
                    game,
                )
                .map(|chars| TextBoxOverlay::new(chars.compiled_card_text, chars.abilities))
                .unwrap_or_else(|| {
                    TextBoxOverlay::new(object.compiled_card_text.clone(), object.abilities_vec())
                })
            });
            (game.controller_of(object), overlay.abilities.clone())
        } else {
            (game.controller_of(object), object.abilities_vec())
        };

    for ability in &abilities {
        if let AbilityKind::Static(static_ability) = &ability.kind {
            if !ability.functions_in(&zone) {
                continue;
            }
            let mut ability_effects = static_ability.generate_effects(object_id, controller, game);
            if let Some(ts) = game
                .effect_store
                .continuous_effects
                .get_entry_timestamp(object_id)
            {
                for effect in &mut ability_effects {
                    effect.timestamp = ts;
                    effect.originating_static_ability = Some(static_ability.clone());
                }
            } else {
                for effect in &mut ability_effects {
                    effect.originating_static_ability = Some(static_ability.clone());
                }
            }
            object_effects.extend(
                ability_effects
                    .into_iter()
                    .map(|effect| GeneratedStaticEffect { effect }),
            );
        }
    }
    assign_inferred_static_effect_groups(&mut object_effects, object_id, &mut next_group_ordinal);
    object_effects
        .into_iter()
        .map(|generated| generated.effect)
        .collect()
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
        .effect_store
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

    #[test]
    fn text_box_query_scope_uses_locked_targets_for_resolution_text_effects() {
        let effects = vec![
            ContinuousEffect::from_resolution(
                ObjectId::from_raw(10),
                PlayerId::from_index(0),
                vec![ObjectId::from_raw(11)],
                Modification::SetTextBox(TextBoxOverlay::new(String::new(), Vec::new())),
            ),
            ContinuousEffect::from_resolution(
                ObjectId::from_raw(10),
                PlayerId::from_index(0),
                vec![ObjectId::from_raw(12)],
                Modification::SetTextBox(TextBoxOverlay::new(String::new(), Vec::new())),
            ),
        ];

        assert_eq!(
            text_box_query_scope(&effects),
            TextBoxQueryScope::Specific(vec![ObjectId::from_raw(11), ObjectId::from_raw(12)])
        );
    }

    #[test]
    fn text_box_query_scope_falls_back_to_battlefield_for_filter_based_text_effects() {
        let effects = vec![ContinuousEffect::new(
            ObjectId::from_raw(10),
            PlayerId::from_index(0),
            EffectTarget::Filter(ObjectFilter::creature()),
            Modification::SetTextBox(TextBoxOverlay::new(String::new(), Vec::new())),
        )];

        assert_eq!(
            text_box_query_scope(&effects),
            TextBoxQueryScope::AllBattlefield
        );
    }
}
