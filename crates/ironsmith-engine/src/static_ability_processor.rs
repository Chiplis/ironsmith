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
    ContinuousEffect, ContinuousEffectGroupId, EffectSourceType, EffectTarget, Layer, Modification,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SourceStaticEffectsKey {
    generation_revision: u64,
    continuous_effect_revision: u64,
    object_revision: u64,
    zone: Zone,
    controller: crate::ids::PlayerId,
    object_timestamp: Option<u64>,
    text_overlay_revision: Option<u64>,
}

#[derive(Debug, Clone)]
struct SourceStaticEffects {
    key: SourceStaticEffectsKey,
    abilities: Arc<Vec<crate::ability::Ability>>,
    direct_effects: Arc<Vec<ContinuousEffect>>,
}

struct SourceStaticEffectEntry {
    object_id: ObjectId,
    abilities: Vec<crate::ability::Ability>,
    effects: Vec<ContinuousEffect>,
}

fn append_late_static_effects(
    game: &GameState,
    sources: &mut [SourceStaticEffectEntry],
    mut available_effects: Vec<ContinuousEffect>,
) {
    // Grants can add a static ability that emits another continuous effect.
    // Iterate to a small fixed point so nested grants work without allowing a
    // cyclic static-ability dependency to recurse indefinitely.
    let mut previous_round: Option<(Vec<ContinuousEffect>, Vec<ObjectId>)> = None;
    let mut recipient_chars = std::collections::HashMap::default();
    for _ in 0..8 {
        // Effects below the ability layer are the only ones that can change
        // which abilities an object has, and they are the same list for every
        // recipient, so build it once per round rather than per object.
        let before_pt = available_effects
            .iter()
            .filter(|effect| effect.modification.layer() <= Layer::Ability)
            .cloned()
            .collect::<Vec<_>>();
        let grant_may_emit_effects = available_effects
            .iter()
            .any(registered_grant_may_emit_late_effects);
        let recipients = sources
            .iter()
            .filter(|source| {
                (grant_may_emit_effects || abilities_grant_continuous_levels(&source.abilities))
                    && game
                        .object(source.object_id)
                        .is_some_and(|object| object.zone == Zone::Battlefield)
            })
            .map(|source| source.object_id)
            .collect::<Vec<_>>();
        // One batched pass shares the layer pipeline — including the CR 613.8
        // dependency sort, which is global rather than per recipient — instead
        // of repeating it for every object on the battlefield. A later round
        // that only added effects above the ability layer feeds this the same
        // inputs, so the previous round's result still stands.
        let round_inputs = (before_pt, recipients);
        if previous_round.as_ref() != Some(&round_inputs) {
            recipient_chars = if round_inputs.1.is_empty() {
                std::collections::HashMap::default()
            } else {
                crate::continuous::calculate_characteristics_batch_with_effects(
                    &round_inputs.1,
                    game.objects_map(),
                    &round_inputs.0,
                    &game.battlefield,
                    game.commander_objects(),
                    game,
                )
            };
            previous_round = Some(round_inputs);
        }
        let mut added = Vec::new();
        for source in sources.iter_mut() {
            let Some(chars) = recipient_chars.get(&source.object_id) else {
                continue;
            };
            let late = generate_granted_late_static_effects(
                game,
                source.object_id,
                &available_effects,
                chars,
                &source.abilities,
            );
            for effect in late {
                if !available_effects.contains(&effect) {
                    available_effects.push(effect.clone());
                    source.effects.push(effect.clone());
                    added.push(effect);
                }
            }
        }
        if added.is_empty() {
            break;
        }
    }

    for source in sources {
        let mut next_group_ordinal = 1;
        assign_inferred_static_effect_groups(
            &mut source.effects,
            source.object_id,
            &mut next_group_ordinal,
        );
    }
}

#[derive(Debug, Default, Clone)]
pub(crate) struct StaticEffectsCache {
    per_source: crate::game_state::PersistentMap<ObjectId, SourceStaticEffects>,
}

fn static_effects_share_scope(a_effect: &ContinuousEffect, b_effect: &ContinuousEffect) -> bool {
    a_effect.source == b_effect.source
        && a_effect.controller == b_effect.controller
        && a_effect.applies_to == b_effect.applies_to
        && a_effect.duration == b_effect.duration
        && a_effect.expires_end_of_turn == b_effect.expires_end_of_turn
        && a_effect.condition == b_effect.condition
        && a_effect.source_type == b_effect.source_type
}

fn should_infer_multilayer_static_group(effects: &[ContinuousEffect], indices: &[usize]) -> bool {
    if indices.len() <= 1 {
        return false;
    }

    let has_type_part = indices
        .iter()
        .any(|&idx| effects[idx].modification.layer() == Layer::Type);
    let has_later_part = indices
        .iter()
        .any(|&idx| effects[idx].modification.layer() > Layer::Type);
    let has_ability_removal = indices.iter().any(|&idx| {
        matches!(
            effects[idx].modification,
            crate::continuous::Modification::RemoveAbility(_)
                | crate::continuous::Modification::RemoveAllAbilities
                | crate::continuous::Modification::RemoveAllAbilitiesExceptMana
                | crate::continuous::Modification::SetAbilities(_)
        )
    });
    let has_pt_setting = indices.iter().any(|&idx| {
        matches!(
            effects[idx].modification,
            crate::continuous::Modification::SetPower { .. }
                | crate::continuous::Modification::SetToughness { .. }
                | crate::continuous::Modification::SetPowerToughness { .. }
        )
    });

    (has_type_part && has_later_part) || (has_ability_removal && has_pt_setting)
}

fn assign_inferred_static_effect_groups(
    effects: &mut [ContinuousEffect],
    source: ObjectId,
    next_group_ordinal: &mut u16,
) {
    let mut assigned = vec![false; effects.len()];

    for i in 0..effects.len() {
        if assigned[i] || effects[i].group.is_some() {
            continue;
        }

        let mut group_indices = Vec::new();
        for j in i..effects.len() {
            if assigned[j] || effects[j].group.is_some() {
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
            effects[idx].group = Some(group);
            assigned[idx] = true;
        }
    }
}

fn source_abilities(
    game: &GameState,
    object_id: ObjectId,
    registered_effects: &[ContinuousEffect],
    text_box_scope: &TextBoxQueryScope,
    text_box_cache: &mut FxMap<ObjectId, TextBoxOverlay>,
) -> Vec<crate::ability::Ability> {
    let object = game
        .object(object_id)
        .expect("static-effect source should exist");
    if object.zone == Zone::Battlefield && text_box_scope.includes(object_id) {
        let overlay = text_box_cache.entry(object_id).or_insert_with(|| {
            crate::continuous::text_box_characteristics_with_effects(
                object_id,
                game.objects_map(),
                registered_effects,
                &game.battlefield,
                game.commander_objects(),
                game,
            )
            .map(|chars| {
                TextBoxOverlay::new(chars.compiled_card_text, chars.abilities)
                    .with_ability_labels(chars.ability_labels)
            })
            .unwrap_or_else(|| {
                TextBoxOverlay::new(object.compiled_card_text.clone(), object.abilities_vec())
                    .with_ability_labels(object.ability_labels.clone())
            })
        });
        overlay.abilities.clone()
    } else {
        object.abilities_vec()
    }
}

fn generate_direct_static_effects(
    game: &GameState,
    object_id: ObjectId,
    abilities: &[crate::ability::Ability],
) -> Vec<ContinuousEffect> {
    let object = game
        .object(object_id)
        .expect("static-effect source should exist");
    let controller = game.controller_of(object);
    let mut effects = Vec::new();
    for ability in abilities {
        let AbilityKind::Static(static_ability) = &ability.kind else {
            continue;
        };
        if !ability.functions_in(&object.zone) {
            continue;
        }
        let mut ability_effects = static_ability.generate_effects(object_id, controller, game);
        let object_timestamp = game
            .effect_store
            .continuous_effects
            .get_object_timestamp(object_id);
        for effect in &mut ability_effects {
            if let Some(ts) = object_timestamp {
                effect.timestamp = ts;
            }
            effect.originating_static_ability = Some(static_ability.clone());
            if effect_is_characteristic_defining(effect, object_id) {
                effect.source_type = EffectSourceType::CharacteristicDefining;
            }
        }
        effects.extend(ability_effects);
    }
    effects
}

/// CR 604.3a: a static ability printed on an object (or given to it by a copy
/// or text-changing effect) is characteristic-defining when it defines the
/// object's own colors, subtypes, power or toughness, affects no other object,
/// and is not conditional. Power/toughness abilities already tag themselves;
/// this catches color and subtype shapes such as changeling, devoid and
/// "~ is all colors" so they apply first in their layer (CR 613.2) and never
/// depend on non-CDA effects (CR 613.8c). Abilities another effect grants are
/// handled separately and are never CDAs.
fn effect_is_characteristic_defining(effect: &ContinuousEffect, source: ObjectId) -> bool {
    if effect.condition.is_some() {
        return false;
    }
    let applies_to_self = match &effect.applies_to {
        EffectTarget::Source => true,
        EffectTarget::Specific(id) => *id == source,
        // "~ is colorless" compiles to a filter that names only the source.
        EffectTarget::Filter(filter) => {
            filter.source && {
                let mut source_only = crate::target::ObjectFilter::source();
                source_only.source_surface = filter.source_surface.clone();
                *filter == source_only
            }
        }
        _ => false,
    };
    if !applies_to_self {
        return false;
    }
    matches!(
        effect.modification,
        Modification::SetColors(_)
            | Modification::AddColors(_)
            | Modification::MakeColorless
            | Modification::AddSubtypes(_)
            | Modification::SetSubtypes(_)
            | Modification::AddAllSubtypesOfFamily(_)
    )
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
    let registered_effects: Vec<ContinuousEffect> =
        game.effect_store.continuous_effects.effects().to_vec();
    let text_box_scope = text_box_query_scope(&registered_effects);
    let mut text_box_cache: FxMap<ObjectId, TextBoxOverlay> = FxMap::default();
    let mut sources = Vec::new();

    let object_ids = game.object_ids_in_deterministic_order();
    // Iterate over all objects and apply static abilities only in zones where they function.
    for object_id in object_ids {
        let Some(object) = game.object(object_id) else {
            continue;
        };
        if object.zone == crate::zone::Zone::Battlefield && game.is_phased_out(object_id) {
            continue;
        }
        let abilities = source_abilities(
            game,
            object_id,
            &registered_effects,
            &text_box_scope,
            &mut text_box_cache,
        );
        let effects = generate_direct_static_effects(game, object_id, &abilities);
        sources.push(SourceStaticEffectEntry {
            object_id,
            abilities,
            effects,
        });
    }

    // First expose all direct static grants, then derive effects from static
    // abilities that those grants added. This matters for nested grants such
    // as Agatha's Soul Cauldron granting a copied-ability static ability.
    let mut effects = sources
        .iter()
        .flat_map(|source| source.effects.iter().cloned())
        .collect::<Vec<_>>();
    let mut available_effects = registered_effects;
    available_effects.extend(effects.iter().cloned());
    append_late_static_effects(game, &mut sources, available_effects);
    effects = sources
        .iter()
        .flat_map(|source| source.effects.iter().cloned())
        .collect();

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
    let continuous_effect_revision = game.effect_store.continuous_effects.revision();
    let generation_revision = game.mutation_revision();
    let object_ids = game.object_ids_in_deterministic_order();
    let mut seen = std::collections::HashSet::new();
    let mut sources = Vec::new();

    for object_id in object_ids {
        let Some(object) = game.object(object_id) else {
            continue;
        };
        seen.insert(object_id);
        if object.zone == crate::zone::Zone::Battlefield && game.is_phased_out(object_id) {
            cache.per_source.remove(&object_id);
            continue;
        }
        let zone = object.zone;
        let controller = game.controller_of(object);
        let key = SourceStaticEffectsKey {
            generation_revision,
            continuous_effect_revision,
            object_revision: object.last_modified,
            zone,
            controller,
            object_timestamp: game
                .effect_store
                .continuous_effects
                .get_object_timestamp(object_id),
            text_overlay_revision: text_box_scope
                .includes(object_id)
                .then_some(text_overlay_revision),
        };

        if let Some(cached) = cache.per_source.get(&object_id)
            && cached.key == key
        {
            sources.push(SourceStaticEffectEntry {
                object_id,
                abilities: cached.abilities.as_ref().clone(),
                effects: cached.direct_effects.as_ref().clone(),
            });
            continue;
        }

        let abilities = source_abilities(
            game,
            object_id,
            &registered_effects,
            &text_box_scope,
            &mut text_box_cache,
        );
        let direct_effects = generate_direct_static_effects(game, object_id, &abilities);
        sources.push(SourceStaticEffectEntry {
            object_id,
            abilities: abilities.clone(),
            effects: direct_effects.clone(),
        });
        cache.per_source.insert(
            object_id,
            SourceStaticEffects {
                key,
                abilities: Arc::new(abilities),
                direct_effects: Arc::new(direct_effects),
            },
        );
    }

    cache.per_source.retain(|id, _| seen.contains(id));

    let mut effects = sources
        .iter()
        .flat_map(|source| source.effects.iter().cloned())
        .collect::<Vec<_>>();
    let mut available_effects = registered_effects;
    available_effects.extend(effects.iter().cloned());
    append_late_static_effects(game, &mut sources, available_effects);
    effects = sources
        .iter()
        .flat_map(|source| source.effects.iter().cloned())
        .collect();

    effects
}

/// Granted static abilities may themselves generate effects in the ability
/// layer or later layers. Read their recipients after grants/removals without
/// replacing the earlier text-box abilities used for layers before six.
/// Whether a granted ability could itself emit a later-layer continuous effect.
///
/// Flag-only keywords and nonstatic abilities cannot, so an ordinary ability
/// grant on the battlefield must not force a characteristic calculation for
/// every recipient.
fn registered_grant_may_emit_late_effects(effect: &ContinuousEffect) -> bool {
    use crate::continuous::Modification;
    match &effect.modification {
        Modification::AddAbility(ability) => ability.may_generate_continuous_effects(),
        Modification::AddAbilityGeneric(ability) => match &ability.kind {
            AbilityKind::Static(ability) => ability.may_generate_continuous_effects(),
            _ => false,
        },
        _ => false,
    }
}

/// Whether a level-up ability can grant something that emits continuous effects.
fn abilities_grant_continuous_levels(text_abilities: &[crate::ability::Ability]) -> bool {
    text_abilities.iter().any(|ability| {
        let AbilityKind::Static(ability) = &ability.kind else {
            return false;
        };
        ability.level_abilities().is_some_and(|levels| {
            levels.iter().any(|tier| {
                tier.abilities
                    .iter()
                    .any(|ability| ability.may_generate_continuous_effects())
            })
        })
    })
}

fn generate_granted_late_static_effects(
    game: &GameState,
    object_id: ObjectId,
    registered: &[ContinuousEffect],
    chars: &crate::continuous::CalculatedCharacteristics,
    text_abilities: &[crate::ability::Ability],
) -> Vec<ContinuousEffect> {
    use crate::continuous::{Modification, PtSublayer};
    let Some(object) = game.object(object_id) else {
        return Vec::new();
    };
    let mut result = Vec::new();
    for (ability_index, ability) in chars.abilities.iter().enumerate() {
        let AbilityKind::Static(granted) = &ability.kind else {
            continue;
        };
        if !ability.functions_in(&Zone::Battlefield) || text_abilities.iter().any(|original|
            matches!(&original.kind, AbilityKind::Static(original) if original.instance_id() == granted.instance_id())) {
            continue;
        }
        let originating_source = chars
            .abilities
            .origin(ability_index)
            .and_then(crate::continuous::AbilityOrigin::effect_source);
        for mut effect in granted.generate_effects(object_id, chars.controller, game) {
            if effect.modification.layer() < Layer::Ability {
                continue;
            }
            if let Some(source) = originating_source
                && matches!(effect.applies_to, EffectTarget::Source)
            {
                // A static ability granted by another permanent keeps that
                // permanent as the source for source-relative filters (for
                // example, Agatha's "exiled with this" relationship), while
                // still applying the generated effect to this recipient.
                effect.source = source;
                effect.applies_to = EffectTarget::Specific(object_id);
            }
            // An ability acquired through a grant is not an intrinsic CDA.
            if let Modification::SetPowerToughness { sublayer, .. }
            | Modification::SetPower { sublayer, .. }
            | Modification::SetToughness { sublayer, .. } = &mut effect.modification
                && *sublayer == PtSublayer::CharacteristicDefining
            {
                *sublayer = PtSublayer::Setting;
            }
            effect.source_type = EffectSourceType::StaticAbility;
            effect.originating_static_ability = Some(granted.clone());
            effect.timestamp = registered.iter().filter(|candidate| match &candidate.modification {
                Modification::AddAbility(ability) => ability.instance_id() == granted.instance_id(),
                Modification::AddAbilityGeneric(ability) => matches!(&ability.kind, AbilityKind::Static(ability) if ability.instance_id() == granted.instance_id()),
                _ => false,
            }).filter(|candidate| {
                if !crate::continuous::continuous_effect_condition_is_active(candidate, game) { return false; }
                if let EffectSourceType::Resolution { locked_targets } = &candidate.source_type {
                    return locked_targets.contains(&object_id);
                }
                match &candidate.applies_to {
                    EffectTarget::AllPermanents => true,
                    EffectTarget::AllCreatures => chars.card_types.contains(&crate::types::CardType::Creature),
                    EffectTarget::Specific(id) => *id == object_id,
                    EffectTarget::Source => candidate.source == object_id,
                    EffectTarget::Filter(filter) => crate::continuous::filter_matches_with_characteristics(
                        filter, object, &chars, game, candidate.controller, candidate.source),
                    EffectTarget::AttachedTo(id) => game.object(*id).is_some_and(|source|
                        source.attached_to == Some(crate::object::AttachmentTarget::Object(object_id))),
                }
            }).map(|candidate| candidate.timestamp).max().unwrap_or(effect.timestamp);
            result.push(effect);
        }
    }
    result
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
    fn flag_only_grants_skip_late_static_characteristic_reads() {
        use crate::ability::Ability;
        use crate::card::{CardBuilder, PowerToughness};
        use crate::ids::CardId;
        use crate::types::CardType;
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let card = CardBuilder::new(CardId::from_raw(99120), "Grant recipient")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let source = game.create_object_from_card(&card, alice, Zone::Battlefield);
        for modification in [
            Modification::AddAbility(StaticAbility::flying()),
            Modification::AddAbilityGeneric(Ability::static_ability(StaticAbility::from_model(
                crate::static_abilities::CompiledStaticAbility::flying(),
            ))),
            Modification::AddAbilityGeneric(Ability::triggered(
                crate::triggers::Trigger::this_attacks(),
                vec![],
            )),
        ] {
            let registered = vec![
                ContinuousEffect::new(source, alice, EffectTarget::AllPermanents, modification)
                    .with_condition(crate::ConditionExpr::YourTurn),
            ];
            // The gate now lives in `append_late_static_effects`, which decides
            // which recipients are worth a characteristic calculation at all.
            assert!(
                !registered
                    .iter()
                    .any(registered_grant_may_emit_late_effects)
            );
            let mut sources = vec![SourceStaticEffectEntry {
                object_id: source,
                abilities: Vec::new(),
                effects: Vec::new(),
            }];
            let before = game.work_counters();
            append_late_static_effects(&game, &mut sources, registered.clone());
            assert!(sources[0].effects.is_empty());
            assert_eq!(
                game.work_counters().dependency_sorts,
                before.dependency_sorts
            );
        }
    }

    #[test]
    fn effect_generating_grants_still_apply_and_respect_ability_removal() {
        use crate::ability::Ability;
        use crate::card::{CardBuilder, PowerToughness};
        use crate::ids::CardId;
        use crate::types::CardType;
        for generic in [false, true] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let alice = game.players[0].id;
            let card = CardBuilder::new(CardId::from_raw(99121), "Anthem recipient")
                .card_types(vec![CardType::Creature])
                .power_toughness(PowerToughness::fixed(2, 2))
                .build();
            let source = game.create_object_from_card(&card, alice, Zone::Battlefield);
            let anthem = StaticAbility::anthem(ObjectFilter::creature().you_control(), 1, 1);
            let modification = if generic {
                Modification::AddAbilityGeneric(Ability::static_ability(anthem))
            } else {
                Modification::AddAbility(anthem)
            };
            game.effect_store
                .continuous_effects
                .add_effect(ContinuousEffect::from_resolution(
                    source,
                    alice,
                    vec![source],
                    modification,
                ));
            assert_eq!(game.calculated_power(source), Some(3));
            game.refresh_continuous_state();
            assert_eq!(game.calculated_power(source), Some(3));
            game.effect_store
                .continuous_effects
                .add_effect(ContinuousEffect::from_resolution(
                    source,
                    alice,
                    vec![source],
                    Modification::RemoveAllAbilities,
                ));
            game.mark_continuous_state_dirty();
            assert_eq!(game.calculated_power(source), Some(2));
            game.refresh_continuous_state();
            assert_eq!(game.calculated_power(source), Some(2));
        }
    }
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
        // One grant kind now emits one modification for both surfaces.
        assert!(matches!(
            effect.modification,
            Modification::AddAbilityGeneric(_)
        ));
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
