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
    continuous_context_revision: u64,
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
    abilities: Arc<crate::continuous::CalculatedAbilities>,
    direct_effects: Arc<Vec<ContinuousEffect>>,
}

type StaticGroupRoots = FxMap<crate::continuous::AbilityOrigin, crate::continuous::AbilityOrigin>;

struct SourceStaticEffectEntry {
    object_id: ObjectId,
    abilities: crate::continuous::CalculatedAbilities,
    effects: Vec<ContinuousEffect>,
    group_roots: StaticGroupRoots,
}

/// Keep source-line composition separate from each component's occurrence
/// identity. Compiler lowering already emits a typed marker before the parts.
fn static_group_roots(abilities: &crate::continuous::CalculatedAbilities) -> StaticGroupRoots {
    let mut roots = StaticGroupRoots::default();
    for (index, ability) in abilities.iter().enumerate() {
        let AbilityKind::Static(marker) = &ability.kind else { continue; };
        let Some(count) = marker.source_line_static_group_member_count() else { continue; };
        if count < 2 || count > abilities.len().saturating_sub(index + 1) { continue; }
        let Some(root) = abilities.origin(index) else { continue; };
        let members = index + 1..index + 1 + count;
        // Incomplete/stale marker metadata is not permission to join unrelated
        // occurrences. Every adjacent component must retain a paired origin.
        if members.clone().any(|slot| abilities.origin(slot).is_none()
            || !matches!(&abilities[slot].kind, AbilityKind::Static(_))) { continue; }
        for slot in members {
            roots.insert(abilities.origin(slot).unwrap().clone(), root.clone());
        }
    }
    roots
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
            source.group_roots.extend(static_group_roots(&chars.abilities));
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
            &source.group_roots,
            &mut next_group_ordinal,
        );
    }
}

#[derive(Debug, Default, Clone)]
pub(crate) struct StaticEffectsCache {
    per_source: crate::game_state::PersistentMap<ObjectId, SourceStaticEffects>,
    checked_revision: Option<u64>,
    refreshed_revision: Option<u64>,
}
impl StaticEffectsCache {
    pub(crate) fn has_checked_snapshot(&self, revision: u64) -> bool {
        self.checked_revision == Some(revision)
    }
    pub(crate) fn mark_checked_snapshot(&mut self, revision: u64) {
        self.checked_revision = Some(revision);
        self.refreshed_revision = None;
    }
    pub(crate) fn invalidate_checked_snapshot(&mut self) {
        self.checked_revision = None;
        self.refreshed_revision = None;
    }
    pub(crate) fn has_refreshed_snapshot(&self, revision: u64) -> bool {
        self.refreshed_revision == Some(revision)
    }
    pub(crate) fn mark_refreshed_snapshot(&mut self, revision: u64) {
        self.refreshed_revision = Some(revision);
    }
}

fn static_effects_share_scope(a_effect: &ContinuousEffect, b_effect: &ContinuousEffect,
    group_roots: &StaticGroupRoots) -> bool {
    // CR 613.6 carries later parts of one effect forward after its ability is
    // removed. Sharing a recipient/filter does not make independent abilities
    // parts of that effect. Sibling branches retain one generating occurrence.
    let same_occurrence = match (&a_effect.originating_ability, &b_effect.originating_ability) {
        (Some(a), Some(b)) => a.host == b.host
            && group_roots.get(&a.ability).unwrap_or(&a.ability)
                == group_roots.get(&b.ability).unwrap_or(&b.ability)
            && a.printed_face == b.printed_face,
        _ => false,
    };
    same_occurrence && a_effect.source == b_effect.source
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
                | crate::continuous::Modification::RemoveStaticAbilityFamily(_)
                | crate::continuous::Modification::RemoveAllAbilities
                | crate::continuous::Modification::RemoveLandRulesTextAbilities
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
    group_roots: &StaticGroupRoots,
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
            if static_effects_share_scope(&effects[i], &effects[j], group_roots) {
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
    text_box_cache: &mut FxMap<ObjectId, crate::continuous::CalculatedAbilities>,
) -> crate::continuous::CalculatedAbilities {
    let object = game.object(object_id).expect("static-effect source should exist");
    let mut abilities = if object.zone == Zone::Battlefield && text_box_scope.includes(object_id) {
        text_box_cache.entry(object_id).or_insert_with(|| {
            crate::continuous::text_box_characteristics_with_effects(
                object_id, game.objects_map(), registered_effects,
                &game.battlefield, game.commander_objects(), game,
            ).map(|chars| chars.abilities)
                .unwrap_or_else(|| object.abilities.clone().into())
        }).clone()
    } else { object.abilities.clone().into() };
    // A later host timestamp makes its granted effects share that timestamp.
    // Keep their prior acquisition order even when component merge storage is
    // arranged differently (CR 613.7a). The registered slots/origins are intact.
    let mut grants: Vec<_> = object.temporary_static_ability_grants.iter().enumerate().collect();
    grants.sort_by_key(|(index, _)| object.temporary_static_ability_grants
        .origin(*index).expect("temporary grant has a paired origin").acquired_at());
    for (index, grant) in grants {
        if grant.is_expired(game.turn.turn_number) { continue; }
        let Some(ability) = grant.materialize() else { continue; };
        let origin = object.temporary_static_ability_grants.origin(index)
            .expect("temporary grant has a paired origin").clone();
        // A temporary grant is not printed/copied rules text. Source color and
        // subtype definitions use the host's ordinary zone (CR 113.6), not a CDA's all-zone default.
        // Other grant families keep their existing policy.
        let is_characteristic_definition = ability.characteristic_defining_colors().is_some()
            || ability.characteristic_defining_subtypes().is_some();
        let ability = crate::ability::Ability::static_ability(ability);
        let ability = if is_characteristic_definition {
            let zone = if object.has_card_type(crate::types::CardType::Instant)
                || object.has_card_type(crate::types::CardType::Sorcery)
            { Zone::Stack } else { Zone::Battlefield };
            ability.in_zones(vec![zone])
        } else { ability };
        abilities.push_with_origin(ability, crate::continuous::AbilityOrigin::Temporary(origin));
    }
    abilities
}

fn generate_direct_static_effects(
    game: &GameState,
    object_id: ObjectId,
    abilities: &crate::continuous::CalculatedAbilities,
) -> Vec<ContinuousEffect> {
    let object = game
        .object(object_id)
        .expect("static-effect source should exist");
    let controller = game.controller_of(object);
    let mut effects = Vec::new();
    for (slot, ability) in abilities.iter().enumerate() {
        let AbilityKind::Static(static_ability) = &ability.kind else {
            continue;
        };
        if !ability.functions_in(&object.zone) {
            continue;
        }
        let mut ability_effects = static_ability.generate_effects(object_id, controller, game);
        let origin = abilities.origin(slot).expect("static source has paired origins").clone();
        let acquired_at = match &origin {
            crate::continuous::AbilityOrigin::Temporary(origin) => origin.acquired_at(),
            _ => None,
        };
        let object_timestamp = game.effect_store.continuous_effects
            .get_object_timestamp(object_id);
        // CR 613.7a: a granted static effect uses the later of its object's
        // timestamp and the timestamp of the effect that created the ability.
        let timestamp = match (object_timestamp, acquired_at) {
            (Some(object), Some(grant)) => Some(object.max(grant)),
            (object, grant) => object.or(grant),
        };
        for (branch, effect) in ability_effects.iter_mut().enumerate() {
            if let Some(ts) = timestamp {
                effect.timestamp = ts;
            }
            effect.originating_static_ability = Some(static_ability.clone());
            let face = matches!(&origin, crate::continuous::AbilityOrigin::Printed(_))
                .then_some(object.card).flatten();
            effect.originating_ability = Some(Box::new(crate::continuous::ContinuousAbilityOrigin {
                host: object_id, ability: origin.clone(), printed_face: face, branch,
            }));
            if effect.originating_ability.as_ref().is_some_and(|origin| origin.ability.is_rules_text())
                && effect_is_characteristic_defining(effect, object_id)
            {
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
        EffectTarget::Filter(filter) => filter.is_source_only(),
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
    let mut text_box_cache: FxMap<ObjectId, crate::continuous::CalculatedAbilities> = FxMap::default();
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
            group_roots: static_group_roots(&abilities),
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
    let mut text_box_cache: FxMap<ObjectId, crate::continuous::CalculatedAbilities> = FxMap::default();
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
            continuous_context_revision: game.continuous_context_revision(),
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
                group_roots: static_group_roots(cached.abilities.as_ref()),
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
            group_roots: static_group_roots(&abilities),
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
        Modification::SetAbilities(abilities) => abilities.iter().any(crate::linked_exile::is_class_linked_exile_wrapper),
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
    text_abilities: &crate::continuous::CalculatedAbilities,
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
        let Some(origin) = chars.abilities.origin(ability_index).cloned() else { continue; };
        if !ability.functions_in(&Zone::Battlefield) || (0..text_abilities.len())
            .any(|index| text_abilities.origin(index) == Some(&origin)) { continue; }
        let originating_source = chars
            .abilities
            .origin(ability_index)
            .and_then(crate::continuous::AbilityOrigin::effect_source);
        for (branch, mut effect) in granted.generate_effects(object_id, chars.controller, game).into_iter().enumerate() {
            if effect.modification.layer() < Layer::Ability {
                continue;
            }
            if let Some(source) = originating_source
                && matches!(effect.applies_to, EffectTarget::Source)
                && !crate::continuous::AbilityEffectOrigin::is_class_linked_exile_effect(&effect)
                && !(crate::continuous::AbilityEffectOrigin::is_source_class_level_effect(&effect)
                    && registered.iter().any(|registered| {
                        matches!(&origin, crate::continuous::AbilityOrigin::Effect { effect, .. }
                            if *effect == crate::continuous::AbilityEffectOrigin::from(registered))
                            && matches!(&registered.modification, Modification::SetAbilities(abilities)
                                if abilities.iter().any(crate::linked_exile::is_class_linked_exile_wrapper))
                    }))
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
            let face = matches!(&origin, crate::continuous::AbilityOrigin::Printed(_))
                .then_some(object.card).flatten();
            effect.originating_ability = Some(Box::new(crate::continuous::ContinuousAbilityOrigin {
                host: object_id, ability: origin.clone(), printed_face: face, branch,
            }));
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

/// Work bounds for checked discovery. Reaching a bound is an error, never a
/// successful partial snapshot. Callers may choose bounds for their host.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StaticEffectDiscoveryLimits {
    pub max_rounds: usize,
    pub max_generated_effects: usize,
}

impl Default for StaticEffectDiscoveryLimits {
    fn default() -> Self {
        Self { max_rounds: 128, max_generated_effects: 16_384 }
    }
}

/// Failure to establish a complete static-effect snapshot. This describes an
/// engine computation boundary, not a Magic dependency cycle or game result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StaticEffectDiscoveryError {
    TextChangeDomain(crate::continuous::text_changes::TextChangeDomainError),
    RoundLimit { maximum: usize, generated_effects: usize },
    EffectLimit { maximum: usize, completed_rounds: usize },
    MissingGeneratingOrigin { host: ObjectId },
    MissingControllerSource { source: ObjectId },
    UnavailableCharacteristics { object: ObjectId },
    /// An existing signed scalar cannot represent this exact quantity.
    ScalarRange { resource: &'static str, value: i128 },
    NumericChoiceEvidence { detail: &'static str },
}

impl std::fmt::Display for StaticEffectDiscoveryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TextChangeDomain(error) => write!(f, "text-changing model domain is incomplete: {error}"),
            Self::RoundLimit { maximum, generated_effects } => write!(f,
                "static-effect discovery did not converge within {maximum} rounds ({generated_effects} generated effects)"),
            Self::EffectLimit { maximum, completed_rounds } => write!(f,
                "static-effect discovery exceeded {maximum} generated effects after {completed_rounds} rounds"),
            Self::MissingGeneratingOrigin { host } => write!(f,
                "static-effect discovery lost generating occurrence on {host:?}"),
            Self::MissingControllerSource { source } => write!(f,
                "continuous source-controller context unavailable for {source:?}"),
            Self::ScalarRange { resource, value } => write!(f,
                "{resource} value {value} exceeds the engine's signed scalar representation"),
            Self::NumericChoiceEvidence { detail } => write!(f,"numeric choice evidence unavailable: {detail}"),
            Self::UnavailableCharacteristics { object } => write!(f,
                "continuous characteristics unavailable for existing object {object:?}"),
        }
    }
}
impl std::error::Error for StaticEffectDiscoveryError {}

/// All unspent-mana selectors share the existing signed scalar domain. Check
/// the wide aggregate before any infallible legacy adapter or cached query can
/// reinterpret a large pool as a negative/small value.
pub(crate) fn validate_mana_scalar_domain(game: &GameState) -> Result<(), StaticEffectDiscoveryError> {
    let total: u128 = game.players.iter().map(|player| u128::from(player.mana_pool.total_wide())).sum();
    if total > i32::MAX as u128 {
        return Err(StaticEffectDiscoveryError::ScalarRange { resource: "unspent mana", value: total as i128 });
    }
    Ok(())
}

fn validate_final_pt_characteristics(game: &GameState, effects: &[ContinuousEffect])
    -> Result<(), StaticEffectDiscoveryError>
{
    let ids: Vec<_> = game.object_ids_in_deterministic_order().into_iter().filter(|id|
        game.object(*id).is_some_and(|object| object.zone != Zone::Battlefield || !game.is_phased_out(*id))).collect();
    let chars = crate::continuous::calculate_characteristics_batch_with_effects(&ids,
        game.objects_map(), effects, &game.battlefield, game.commander_objects(), game);
    for value in chars.values() { value.validate_numeric_range()?; }
    Ok(())
}

/// Discover a complete snapshot without publishing static effects or a static snapshot.
///
/// This is the fallible migration boundary. The legacy Vec-returning query
/// remains separate until its commit/resume owners can propagate this error.
/// Independent equal definitions are keyed by full generating occurrence and
/// branch, rather than trait-object PartialEq or semantic equality.
pub fn try_generate_continuous_effects_from_static_abilities(
    game: &GameState,
    limits: StaticEffectDiscoveryLimits,
) -> Result<Vec<ContinuousEffect>, StaticEffectDiscoveryError> {
    validate_mana_scalar_domain(game)?;
    let mut mana_pt = false;
    let registered = game.effect_store.continuous_effects.effects().to_vec();
    validate_static_controller_sources(game, &registered)?;
    let scope = text_box_query_scope(&registered);
    let mut text_cache = FxMap::default();
    let mut sources = Vec::new();
    for object_id in game.object_ids_in_deterministic_order() {
        let Some(object) = game.object(object_id) else { continue; };
        if object.zone == Zone::Battlefield && game.is_phased_out(object_id) { continue; }
        let abilities = source_abilities(game, object_id, &registered, &scope, &mut text_cache);
        for ability in abilities.iter().filter(|ability| ability.functions_in(&object.zone)) {
            if let AbilityKind::Static(ability) = &ability.kind {
                mana_pt |= ability.validate_mana_scalar_ranges(game, object_id, game.controller_of(object))?;
            }
        }
        sources.push(SourceStaticEffectEntry { object_id,
            group_roots: static_group_roots(&abilities),
            effects: generate_direct_static_effects(game, object_id, &abilities), abilities });
    }
    let mut available = registered.clone();
    available.extend(sources.iter().flat_map(|source| source.effects.iter().cloned()));
    validate_static_controller_sources(game, &available)?;
    let mut emitted = std::collections::HashSet::new();
    let mut generated = 0;
    for source in &sources {
        for effect in &source.effects {
            let origin = effect.originating_ability.as_deref().ok_or(
                StaticEffectDiscoveryError::MissingGeneratingOrigin { host: source.object_id })?;
            emitted.insert(origin.clone());
            generated += 1;
            if generated > limits.max_generated_effects {
                return Err(StaticEffectDiscoveryError::EffectLimit {
                    maximum: limits.max_generated_effects, completed_rounds: 0 });
            }
        }
    }
    for round in 0..limits.max_rounds {
        let before_pt = available.iter().filter(|effect|
            effect.modification.layer() <= Layer::Ability).cloned().collect::<Vec<_>>();
        let grant_may_emit = available.iter().any(registered_grant_may_emit_late_effects);
        let recipients = sources.iter().filter(|source|
            (grant_may_emit || abilities_grant_continuous_levels(&source.abilities))
                && game.object(source.object_id).is_some_and(|object| object.zone == Zone::Battlefield))
            .map(|source| source.object_id).collect::<Vec<_>>();
        let chars = crate::continuous::calculate_characteristics_batch_with_effects(
            &recipients, game.objects_map(), &before_pt, &game.battlefield,
            game.commander_objects(), game);
        let mut added = false;
        for source in &mut sources {
            let Some(chars) = chars.get(&source.object_id) else { continue; };
            source.group_roots.extend(static_group_roots(&chars.abilities));
            for ability in chars.abilities.iter().filter(|ability| ability.functions_in(&Zone::Battlefield)) {
                if let AbilityKind::Static(ability) = &ability.kind {
                    mana_pt |= ability.validate_mana_scalar_ranges(game, source.object_id, chars.controller)?;
                }
            }
            let late = generate_granted_late_static_effects(game, source.object_id,
                &available, chars, &source.abilities);
            for effect in late {
                let origin = effect.originating_ability.as_deref().ok_or(
                    StaticEffectDiscoveryError::MissingGeneratingOrigin { host: source.object_id })?;
                validate_static_controller_sources(game, std::slice::from_ref(&effect))?;
                if !emitted.insert(origin.clone()) { continue; }
                if generated == limits.max_generated_effects {
                    return Err(StaticEffectDiscoveryError::EffectLimit {
                        maximum: limits.max_generated_effects, completed_rounds: round });
                }
                generated += 1;
                available.push(effect.clone()); source.effects.push(effect); added = true;
            }
        }
        if !added {
            let mut effects = Vec::new();
            for source in &mut sources {
                let mut ordinal = 1;
                assign_inferred_static_effect_groups(&mut source.effects, source.object_id,
                    &source.group_roots, &mut ordinal);
                effects.append(&mut source.effects);
            }
            let has_pt = mana_pt || registered.iter().chain(effects.iter())
                .any(|effect| effect.modification.layer() == Layer::PowerToughness)
                || game.objects_map().values().any(|object|
                    object.counters.iter().any(|(counter, count)| *count != 0 && counter.pt_delta().is_some()));
            let has_text_rewrite = registered.iter().chain(effects.iter())
                .any(|effect| matches!(effect.modification, Modification::RewriteText(_)));
            if has_pt || has_text_rewrite {
                let mut complete = registered.clone(); complete.extend(effects.iter().cloned());
                validate_final_pt_characteristics(game, &complete)?;
            }
            return Ok(effects);
        }
    }
    Err(StaticEffectDiscoveryError::RoundLimit {
        maximum: limits.max_rounds, generated_effects: generated })
}

fn validate_static_controller_sources(
    game: &GameState, effects: &[ContinuousEffect],
) -> Result<(), StaticEffectDiscoveryError> {
    for effect in effects {
        if effect.has_source_controller_context() {
            let source = effect.source_controller_context_host();
            if game.object(source).is_none() {
                return Err(StaticEffectDiscoveryError::MissingControllerSource { source });
            }
        }
    }
    Ok(())
}

/// Include registered effects only after static discovery has completed.
pub fn try_get_all_continuous_effects(
    game: &GameState,
    limits: StaticEffectDiscoveryLimits,
) -> Result<Vec<ContinuousEffect>, StaticEffectDiscoveryError> {
    let statics = try_generate_continuous_effects_from_static_abilities(game, limits)?;
    let mut effects = game.effect_store.continuous_effects.effects_sorted()
        .into_iter().cloned().collect::<Vec<_>>();
    effects.extend(statics);
    Ok(effects)
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
                abilities: Vec::new().into(),
                effects: Vec::new(),
                group_roots: StaticGroupRoots::default(),
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

#[cfg(test)]
mod inferred_group_occurrence_gameplay_tests {
    use super::*;
    use crate::{ability::Ability, card::{CardBuilder, PowerToughness}, effect::Value,
        ids::PlayerId, static_abilities::{StaticAbility, StaticAbilityKind, StaticAbilityId}, types::CardType};

    #[derive(Debug, Clone)]
    struct Emits(Vec<crate::continuous::Modification>);
    impl StaticAbilityKind for Emits {
        fn id(&self) -> StaticAbilityId { StaticAbilityId::Anthem }
        fn display(&self) -> String { "Layer instruction fixture".into() }
        fn generate_effects(&self, source: ObjectId, controller: PlayerId, _game: &GameState)
            -> Vec<ContinuousEffect> {
            self.0.iter().map(|modification| ContinuousEffect::new(source, controller,
                EffectTarget::Source, modification.clone())).collect()
        }
    }

    fn fixture() -> (GameState, ObjectId) {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let card = CardBuilder::new(crate::ids::CardId::new(), "Static group recipient")
            .card_types(vec![CardType::Artifact])
            .power_toughness(PowerToughness::fixed(2, 2)).build();
        let source = game.create_object_from_card(&card, alice, Zone::Battlefield);
        (game, source)
    }

    fn type_and_pt() -> (crate::continuous::Modification, crate::continuous::Modification) {
        use crate::continuous::{Modification, PtSublayer};
        (Modification::AddCardTypes(vec![CardType::Creature]),
            Modification::SetPowerToughness { power: Value::Fixed(7), toughness: Value::Fixed(7),
                sublayer: PtSublayer::Setting })
    }

    #[test]
    fn independent_static_ability_does_not_inherit_started_multipart_group_after_loss() {
        use crate::continuous::Modification;
        for combined in [false, true] {
            for remove in [false, true] {
                let (mut game, source) = fixture();
                let (typed, pt) = type_and_pt();
                let payloads = if combined { vec![vec![typed, pt]] }
                    else { vec![vec![typed], vec![pt]] };
                for modifications in payloads {
                    game.object_mut(source).unwrap().abilities_mut().push(
                        Ability::static_ability(StaticAbility::new(Emits(modifications))));
                }
                if remove {
                    game.object_mut(source).unwrap().abilities_mut().push(
                        Ability::static_ability(StaticAbility::new(Emits(vec![Modification::RemoveAllAbilities]))));
                }
                let expected = if !combined && remove { 2 } else { 7 };
                // Dirty/wholesale and refreshed/cached readers must agree.
                for refreshed in [false, true] {
                    if refreshed { game.refresh_continuous_state(); }
                    let chars = game.current_characteristics(source).unwrap();
                    assert_eq!(chars.power, Some(expected), "combined={combined},remove={remove},refreshed={refreshed}");
                    assert_eq!(chars.toughness, Some(expected));
                    assert!(chars.card_types.contains(&CardType::Creature));
                    assert_eq!(chars.abilities.is_empty(), remove);
                }
                let checkpoint = game.clone();
                let chars = checkpoint.current_characteristics(source).unwrap();
                assert_eq!((chars.power, chars.toughness), (Some(expected), Some(expected)));
            }
        }
    }

    #[test]
    fn cloned_multipart_static_occurrences_keep_separate_groups_through_refresh() {
        let (mut game, source) = fixture();
        let (typed, pt) = type_and_pt();
        let parent = StaticAbility::new(Emits(vec![typed, pt]));
        for _ in 0..2 {
            game.object_mut(source).unwrap().abilities_mut().push(Ability::static_ability(parent.clone()));
        }
        let before = get_all_continuous_effects(&game);
        let groups = before.iter().filter_map(|effect| effect.group).collect::<std::collections::HashSet<_>>();
        assert_eq!(before.len(), 4);
        assert_eq!(groups.len(), 2);
        for group in &groups { assert_eq!(before.iter().filter(|effect| effect.group == Some(*group)).count(), 2); }
        game.refresh_continuous_state();
        let after = game.all_continuous_effects();
        assert_eq!(after.iter().filter_map(|effect| effect.group).collect::<std::collections::HashSet<_>>(), groups);
        let checkpoint = game.clone();
        assert_eq!(checkpoint.all_continuous_effects().iter().filter_map(|effect| effect.group)
            .collect::<std::collections::HashSet<_>>(), groups);
    }

    #[test]
    fn marked_static_parts_survive_loss_without_preserving_an_independent_later_effect() {
        use crate::continuous::{Modification, PtSublayer};
        for compiled_marker in [false, true] {
            let (mut game, source) = fixture();
            let marker = if compiled_marker {
                StaticAbility::from_model(crate::static_abilities::CompiledStaticAbility::source_line_static_group(2))
            } else { StaticAbility::source_line_static_group(2) };
            game.object_mut(source).unwrap().abilities_mut().push(Ability::static_ability(marker));
            let (typed, pt) = type_and_pt();
            for modification in [typed, pt,
                Modification::SetPowerToughness { power: Value::Fixed(11), toughness: Value::Fixed(11),
                    sublayer: PtSublayer::Setting }, Modification::RemoveAllAbilities] {
                game.object_mut(source).unwrap().abilities_mut().push(
                    Ability::static_ability(StaticAbility::new(Emits(vec![modification]))));
            }
            for refreshed in [false, true] {
                if refreshed { game.refresh_continuous_state(); }
                let chars = game.current_characteristics(source).unwrap();
                assert_eq!((chars.power, chars.toughness), (Some(7), Some(7)),
                    "compiled_marker={compiled_marker},refreshed={refreshed}");
                assert!(chars.abilities.is_empty());
            }
            let effects = game.all_continuous_effects();
            assert_eq!(effects.len(), 4);
            assert!(effects[0].group.is_some());
            assert_eq!(effects[0].group, effects[1].group);
            assert_eq!(effects[2].group, None);
            assert_eq!(effects[3].group, None);
        }
    }

    #[test]
    fn cloned_marked_static_lines_keep_distinct_group_roots_and_component_origins() {
        let (mut game, source) = fixture();
        let marker = StaticAbility::source_line_static_group(2);
        let (typed, pt) = type_and_pt();
        let parts = [StaticAbility::new(Emits(vec![typed])), StaticAbility::new(Emits(vec![pt]))];
        for _ in 0..2 {
            game.object_mut(source).unwrap().abilities_mut().push(Ability::static_ability(marker.clone()));
            for part in &parts {
                game.object_mut(source).unwrap().abilities_mut().push(Ability::static_ability(part.clone()));
            }
        }
        let before = get_all_continuous_effects(&game);
        let groups = before.iter().filter_map(|effect| effect.group).collect::<std::collections::HashSet<_>>();
        let origins = before.iter().filter_map(|effect| effect.originating_ability.clone())
            .collect::<std::collections::HashSet<_>>();
        assert_eq!(before.len(), 4);
        assert_eq!(groups.len(), 2);
        assert_eq!(origins.len(), 4, "components keep their independent occurrence identities");
        for group in &groups { assert_eq!(before.iter().filter(|effect| effect.group == Some(*group)).count(), 2); }
        game.refresh_continuous_state();
        let after = game.all_continuous_effects();
        assert_eq!(after.iter().filter_map(|effect| effect.group).collect::<std::collections::HashSet<_>>(), groups);
        assert_eq!(after.iter().filter_map(|effect| effect.originating_ability.clone())
            .collect::<std::collections::HashSet<_>>(), origins);
    }
}

#[cfg(test)]
mod checked_discovery_tests {
    use super::*;
    use crate::{ability::{Ability, AbilityKind}, card::CardBuilder, ids::{CardId, PlayerId},
        static_abilities::{StaticAbility, StaticAbilityId, StaticAbilityKind},
        target::{ObjectFilter, PlayerFilter}, types::CardType};
    use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};

    fn game_with_parent(depth: usize) -> (GameState, ObjectId) {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let card = CardBuilder::new(CardId::new(), "Finite static recipient")
            .card_types(vec![CardType::Artifact]).build();
        let source = game.create_object_from_card(&card, alice, Zone::Battlefield);
        let mut model = ironsmith_core::StaticAbility::double_life_change_replacement(
            PlayerFilter::You, false, "Double life gain");
        for _ in 0..depth {
            model = ironsmith_core::StaticAbility::grant_object_ability_for_filter(
                ObjectFilter::source(), ironsmith_core::Ability::static_ability(model), "Source has ability");
        }
        let parent = StaticAbility::from_model(model);
        for _ in 0..2 { game.object_mut(source).unwrap().abilities_mut()
            .push(Ability::static_ability(parent.clone())); }
        (game, source)
    }

    #[test]
    fn checked_discovery_completes_finite_graphs_and_preserves_independent_leaf_origins() {
        std::thread::Builder::new().stack_size(128 * 1024 * 1024).spawn(|| {
            for depth in [1, 8, 9, 10, 12] {
                let (game, source) = game_with_parent(depth);
                let revision = game.effect_store.continuous_effects.revision();
                let limits = StaticEffectDiscoveryLimits::default();
                let effects = try_get_all_continuous_effects(&game, limits).unwrap();
                assert_eq!(effects.len(), depth * 2, "depth={depth}");
                let origins = effects.iter().map(|effect| effect.originating_ability.clone().unwrap())
                    .collect::<std::collections::HashSet<_>>();
                assert_eq!(origins.len(), effects.len());
                let chars = game.calculated_characteristics_with_effects(source, &effects).unwrap();
                let leaf_origins = chars.abilities.iter().enumerate().filter_map(|(slot, ability)| {
                    let AbilityKind::Static(ability) = &ability.kind else { return None; };
                    (ability.id() == StaticAbilityId::DoubleLifeChangeReplacement)
                        .then(|| chars.abilities.origin(slot).unwrap().clone())
                }).collect::<std::collections::HashSet<_>>();
                assert_eq!(leaf_origins.len(), 2, "depth={depth}");
                let again = try_get_all_continuous_effects(&game, limits).unwrap();
                assert_eq!(again.iter().map(|effect| effect.originating_ability.clone().unwrap())
                    .collect::<std::collections::HashSet<_>>(), origins);
                assert_eq!(game.effect_store.continuous_effects.revision(), revision);
                assert!(game.effect_store.continuous_effects.static_ability_effects().is_empty());
                assert_eq!(game.players[0].life, 20, "pure discovery cannot commit gameplay");
                assert!(matches!(try_get_all_continuous_effects(&game,
                    StaticEffectDiscoveryLimits { max_rounds: 128, max_generated_effects: 1 }),
                    Err(StaticEffectDiscoveryError::EffectLimit { maximum: 1, completed_rounds: 0 })));
            }
        }).unwrap().join().unwrap();
    }

    #[derive(Debug, Clone)]
    struct RegrantParent(Arc<AtomicUsize>);
    impl StaticAbilityKind for RegrantParent {
        fn id(&self) -> StaticAbilityId { StaticAbilityId::GrantObjectAbilityForFilter }
        fn display(&self) -> String { "Regrant printed parent".into() }
        fn generate_effects(&self, source: ObjectId, controller: PlayerId, game: &GameState)
            -> Vec<ContinuousEffect> {
            assert!(self.0.fetch_add(1, Ordering::SeqCst) < 32_768, "fixture work ceiling");
            let AbilityKind::Static(parent) = &game.object(source).unwrap().abilities[0].kind
                else { panic!("fixture parent missing") };
            vec![ContinuousEffect::new(source, controller, EffectTarget::Source,
                Modification::AddAbility(parent.clone()))]
        }
    }

    #[derive(Debug, Clone)]
    struct EnterWithUnboundedDiscovery;
    impl crate::effects::EffectExecutor for EnterWithUnboundedDiscovery {
        fn execute(&self, game: &mut GameState, ctx: &mut crate::effects::ExecutionContext)
            -> Result<crate::effect::EffectOutcome, crate::effects::ExecutionError> {
            let card = CardBuilder::new(CardId::new(), "Checked entry receipt")
                .card_types(vec![CardType::Artifact]).build();
            let id = game.create_object_from_card(&card, ctx.controller, Zone::Battlefield);
            game.object_mut(id).unwrap().abilities_mut().push(Ability::static_ability(
                StaticAbility::new(RegrantParent(Arc::new(AtomicUsize::new(0))))));
            Ok(crate::effect::EffectOutcome::with_objects(vec![id]).with_events(vec![
                crate::triggers::TriggerEvent::new_with_provenance(
                    crate::events::EnterBattlefieldEvent::new(id, Zone::Command), ctx.provenance),
            ]))
        }
    }

    #[test]
    fn completed_entry_discovery_failure_rolls_back_the_original_instruction_and_history() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let next = game.next_object_id_counter();
        let before_provenance = game.provenance_graph().node_count();
        let mut dm = crate::decision::SelectFirstDecisionMaker;
        let mut ctx = crate::effects::ExecutionContext::new(ObjectId::from_raw(9001), PlayerId(0), &mut dm);
        let result = crate::effects::execute_effect(&mut game,
            &crate::effect::Effect::new(EnterWithUnboundedDiscovery), &mut ctx);
        assert!(matches!(result, Err(crate::effects::ExecutionError::ContinuousDiscovery(_))), "{result:?}");
        assert!(game.battlefield.is_empty());
        assert_eq!(game.next_object_id_counter(), next);
        assert_eq!(game.provenance_graph().node_count(), before_provenance);
        assert!(game.turn_store.turn_history.event_records.is_empty());
        assert!(game.turn_store.turn_history.staged_event_records.is_empty());
        assert!(game.effect_store.pending_trigger_events.is_empty());
    }

    #[test]
    fn checked_discovery_returns_typed_failure_without_publishing_a_partial_graph() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let card = CardBuilder::new(CardId::new(), "Dynamic grant recipient")
            .card_types(vec![CardType::Artifact]).build();
        let source = game.create_object_from_card(&card, alice, Zone::Battlefield);
        let calls = Arc::new(AtomicUsize::new(0));
        game.object_mut(source).unwrap().abilities_mut()
            .push(Ability::static_ability(StaticAbility::new(RegrantParent(calls.clone()))));
        let revision = game.effect_store.continuous_effects.revision();
        let result = try_get_all_continuous_effects(&game,
            StaticEffectDiscoveryLimits { max_rounds: 8, max_generated_effects: 128 });
        assert!(matches!(result, Err(StaticEffectDiscoveryError::RoundLimit {
            maximum: 8, generated_effects: 9 })), "{result:?}");
        assert!(calls.load(Ordering::SeqCst) < 256);
        assert_eq!(game.effect_store.continuous_effects.revision(), revision);
        assert!(game.effect_store.continuous_effects.static_ability_effects().is_empty());
        assert_eq!(game.players[0].life, 20);
        let result = try_generate_continuous_effects_from_static_abilities(&game,
            StaticEffectDiscoveryLimits { max_rounds: 16, max_generated_effects: 4 });
        assert!(matches!(result, Err(StaticEffectDiscoveryError::EffectLimit {
            maximum: 4, completed_rounds: 3 })), "{result:?}");
        assert!(game.effect_store.continuous_effects.static_ability_effects().is_empty());
    }

    #[test]
    fn checked_owner_finishes_deep_life_replacements_from_dirty_and_legacy_clean_states() {
        use crate::effects::EffectExecutor;
        std::thread::Builder::new().stack_size(128 * 1024 * 1024).spawn(|| {
            for depth in [1, 8, 9, 10, 12] {
                for legacy_clean in [false, true] {
                    for generic in [false, true] {
                        let (mut game, source) = game_with_parent(depth);
                        let alice = game.players[0].id;
                        if legacy_clean { game.refresh_continuous_state().unwrap(); }
                        let mut ctx = crate::effects::EffectContext::new_default(source, alice);
                        let outcome = if generic {
                            crate::effects::execute_effect(&mut game, &crate::effect::Effect::gain_life(1), &mut ctx)
                        } else {
                            crate::effects::GainLifeEffect::you(1).execute(&mut game, &mut ctx)
                        }.unwrap();
                        assert_eq!(game.player(alice).unwrap().life, 24,
                            "depth={depth},legacy_clean={legacy_clean},generic={generic}");
                        assert_eq!(outcome.count_or_zero(), 4); assert_eq!(outcome.events.len(), 1);
                        let effects = crate::replacement_ability_processor::generate_replacement_effects_from_abilities(&game).unwrap();
                        assert_eq!(effects.len(), 2);
                        assert_eq!(effects.iter().map(|effect| effect.application_key())
                            .collect::<std::collections::HashSet<_>>().len(), 2);
                    }
                }
            }
        }).unwrap().join().unwrap();
    }

    #[test]
    fn failed_replacement_publication_and_full_refresh_preserve_the_previous_snapshot() {
        std::thread::Builder::new().stack_size(128 * 1024 * 1024).spawn(|| {
            let (mut game, finite_source) = game_with_parent(1);
            game.refresh_continuous_state().unwrap();
            let alice = game.players[0].id;
            let shield = game.effect_store.replacement_effects.add_one_shot_effect(
                crate::replacement::ReplacementEffect::with_matcher(finite_source, alice,
                    crate::events::life::matchers::WouldGainLifeMatcher::you(),
                    crate::replacement::ReplacementAction::Modify(
                        crate::replacement::EventModification::Add(1))));
            let before_keys = game.effect_store.replacement_effects.effects().iter()
                .map(|effect| effect.application_key()).collect::<Vec<_>>();
            assert_eq!(before_keys.len(), 3);
            let before_origins = game.effect_store.continuous_effects.static_ability_effects().iter()
                .map(|effect| effect.originating_ability.clone()).collect::<Vec<_>>();
            let card = CardBuilder::new(CardId::new(), "Unresolved publication recipient")
                .card_types(vec![CardType::Artifact]).build();
            let source = game.create_object_from_card(&card, alice, Zone::Battlefield);
            let calls = Arc::new(AtomicUsize::new(0));
            game.object_mut(source).unwrap().abilities_mut().push(Ability::static_ability(
                StaticAbility::new(RegrantParent(calls.clone()))));
            let revision = game.effect_store.continuous_effects.revision();
            for full_refresh in [false, true] {
                calls.store(0, Ordering::SeqCst);
                let result = if full_refresh { game.refresh_continuous_state() }
                    else { game.update_replacement_effects() };
                assert!(matches!(result, Err(StaticEffectDiscoveryError::RoundLimit {
                    maximum: 128, .. })), "{result:?}");
                assert_eq!(game.effect_store.replacement_effects.effects().iter()
                    .map(|effect| effect.application_key()).collect::<Vec<_>>(), before_keys);
                assert_eq!(game.effect_store.continuous_effects.static_ability_effects().iter()
                    .map(|effect| effect.originating_ability.clone()).collect::<Vec<_>>(), before_origins);
                assert_eq!(game.effect_store.continuous_effects.revision(), revision);
                assert!(!game.continuous_state_is_clean());
                assert!(game.effect_store.replacement_effects.get_effect(shield).is_some());
                assert!(game.effect_store.replacement_effects.is_one_shot(shield));
                assert_eq!(game.player(alice).unwrap().life, 20);
                assert!(game.stack.is_empty());
                assert!(calls.load(Ordering::SeqCst) < 32_768);
            }
        }).unwrap().join().unwrap();
    }

    #[test]
    fn discovery_failure_stops_direct_and_generic_execution_without_mutation_or_one_shot_use() {
        use crate::effects::EffectExecutor;
        std::thread::Builder::new().stack_size(128 * 1024 * 1024).spawn(|| {
            for generic in [false, true] {
                let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                let alice = game.players[0].id;
                let card = CardBuilder::new(CardId::new(), "Unresolved discovery recipient")
                    .card_types(vec![CardType::Artifact]).build();
                let source = game.create_object_from_card(&card, alice, Zone::Battlefield);
                let calls = Arc::new(AtomicUsize::new(0));
                game.object_mut(source).unwrap().abilities_mut().push(Ability::static_ability(
                    StaticAbility::new(RegrantParent(calls.clone()))));
                let shield = game.effect_store.replacement_effects.add_one_shot_effect(
                    crate::replacement::ReplacementEffect::with_matcher(source, alice,
                        crate::events::life::matchers::WouldGainLifeMatcher::you(),
                        crate::replacement::ReplacementAction::Modify(
                            crate::replacement::EventModification::Add(1))));
                let revision = game.effect_store.continuous_effects.revision();
                let mut ctx = crate::effects::EffectContext::new_default(source, alice);
                let result = if generic {
                    crate::effects::execute_effect(&mut game, &crate::effect::Effect::gain_life(1), &mut ctx)
                } else {
                    crate::effects::GainLifeEffect::you(1).execute(&mut game, &mut ctx)
                };
                assert!(matches!(result, Err(crate::effects::ExecutionError::ContinuousDiscovery(
                    StaticEffectDiscoveryError::RoundLimit { maximum: 128, generated_effects: 129 }))),
                    "{result:?}");
                assert_eq!(game.player(alice).unwrap().life, 20);
                assert_eq!(game.effect_store.continuous_effects.revision(), revision);
                assert!(game.effect_store.continuous_effects.static_ability_effects().is_empty());
                assert!(game.effect_store.replacement_effects.get_effect(shield).is_some());
                assert!(game.effect_store.replacement_effects.is_one_shot(shield));
                assert!(game.stack.is_empty()); assert!(ctx.executing_effect.is_none());
                assert!(!ctx.decision_maker.awaiting_choice());
                assert!(calls.load(Ordering::SeqCst) < 32_768);
            }
        }).unwrap().join().unwrap();
    }
#[test]
fn controller_setter_does_not_commit_when_complete_discovery_fails() {
    std::thread::Builder::new()
        .stack_size(128 * 1024 * 1024)
        .spawn(|| {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let alice = game.players[0].id;
            let bob = game.players[1].id;
            let card = CardBuilder::new(CardId::new(), "Unresolved control recipient")
                .card_types(vec![CardType::Artifact])
                .build();
            let source = game.create_object_from_card(&card, alice, Zone::Battlefield);
            let calls = Arc::new(AtomicUsize::new(0));
            game.object_mut(source)
                .expect("source exists")
                .abilities_mut()
                .push(Ability::static_ability(StaticAbility::new(RegrantParent(
                    calls,
                ))));
            assert!(
                matches!(
                    game.try_all_continuous_effects(),
                    Err(StaticEffectDiscoveryError::RoundLimit {
                        maximum: 128,
                        generated_effects: 129
                    })
                ),
                "fixture actually fails complete discovery before the setter"
            );
            let revision = game.effect_store.continuous_effects.revision();
            let sickness = game.is_summoning_sick(source);
            let resolution_effects = game.effect_store.continuous_effects.effects().len();
            let error = game.set_current_controller(source, bob)
                .expect_err("unresolved discovery must return an explicit control failure");
            assert!(matches!(error, StaticEffectDiscoveryError::RoundLimit {
                maximum: 128, generated_effects: 129
            }), "{error:?}");
            assert_eq!(
                game.effect_store.continuous_effects.effects().len(),
                resolution_effects,
                "an unresolved discovery failure must not commit the control effect"
            );
            assert_eq!(
                game.effect_store.continuous_effects.revision(),
                revision,
                "failed control must preserve continuous state revision"
            );
            assert_eq!(
                game.is_summoning_sick(source),
                sickness,
                "failed control must preserve summoning sickness"
            );
        })
        .expect("fixture worker starts")
        .join()
        .expect("fixture worker completes");
}

fn assert_finite_prospective_entry_replacement_is_complete(generic: bool) {
    use crate::effects::EffectExecutor;
    std::thread::Builder::new().stack_size(128 * 1024 * 1024).spawn(move || {
        for depth in [1, 8, 9, 10, 12] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let alice = game.players[0].id;
            let card = CardBuilder::new(CardId::new(), "Prospective nested entry recipient")
                .card_types(vec![CardType::Land]).build();
            let source = game.create_object_from_card(&card, alice, Zone::Hand);
            let mut model = ironsmith_core::StaticAbility::enters_untapped_for_filter(ObjectFilter::source());
            for _ in 0..depth {
                model = ironsmith_core::StaticAbility::grant_object_ability_for_filter(
                    ObjectFilter::source(), ironsmith_core::Ability::static_ability(model), "Source has ability");
            }
            game.object_mut(source).expect("source exists").abilities_mut()
                .push(Ability::static_ability(StaticAbility::from_model(model)));
            game.try_all_continuous_effects().expect("original hand state has complete discovery");
            let instruction = crate::effects::PutOntoBattlefieldEffect::you_control(
                crate::target::ChooseSpec::SpecificObject(source), true);
            let mut ctx = crate::effects::ExecutionContext::new_default(source, alice);
            if generic {
                crate::effects::execute_effect(&mut game, &crate::effect::Effect::new(instruction), &mut ctx)
            } else {
                instruction.execute(&mut game, &mut ctx)
            }.expect("finite prospective discovery must resolve entry");
            assert!(!ctx.decision_maker.awaiting_choice(), "single entry replacement needs no unanswered choice");
            let entered = game.current_object_id_after_zone_change(source).expect("entry gives current identity");
            assert_eq!(game.object(entered).expect("entered object").zone, Zone::Battlefield);
            assert!(!game.is_tapped(entered), "depth={depth}, generic={generic}: prospective nested untapper must replace authored tapped entry");
        }
    }).expect("fixture worker starts").join().expect("fixture worker completes");
}

#[test]
fn direct_entry_uses_complete_finite_prospective_replacement_discovery() {
    assert_finite_prospective_entry_replacement_is_complete(false);
}
#[test]
fn generic_entry_uses_complete_finite_prospective_replacement_discovery() {
    assert_finite_prospective_entry_replacement_is_complete(true);
}

#[test]
fn checked_prospective_entry_query_discovers_deep_grants_and_preserves_live_state() {
    std::thread::Builder::new().stack_size(128 * 1024 * 1024).spawn(|| {
        for depth in [1, 8, 9, 10, 12] {
            for controlled_by_owner in [true, false] {
                let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                let alice = game.players[0].id;
                let controller = game.players[usize::from(!controlled_by_owner)].id;
                let card = CardBuilder::new(CardId::new(), "Checked prospective recipient")
                    .card_types(vec![CardType::Land]).build();
                let source = game.create_object_from_card(&card, alice, Zone::Hand);
                let mut model = ironsmith_core::StaticAbility::enters_untapped_for_filter(ObjectFilter::source());
                for _ in 0..depth {
                    model = ironsmith_core::StaticAbility::grant_object_ability_for_filter(
                        ObjectFilter::source(), ironsmith_core::Ability::static_ability(model), "Source has ability");
                }
                game.object_mut(source).expect("source exists").abilities_mut()
                    .push(Ability::static_ability(StaticAbility::from_model(model)));
                let revision = game.effect_store.continuous_effects.revision();
                let sickness = game.is_summoning_sick(source);
                let mut event = crate::events::EnterBattlefieldEvent::new(source, Zone::Hand)
                    .with_controller_override(controller);
                event.enters_with_counters.push((crate::object::CounterType::Charge, 3));
                let preview = event.try_prospective_game_state(&game)
                    .expect("finite prospective discovery succeeds").expect("source exists");
                assert_eq!(preview.current_controller(source), Some(controller));
                assert_eq!(preview.object(source).unwrap().counters.get(&crate::object::CounterType::Charge), Some(&3));
                let replacements = crate::replacement_ability_processor::generate_replacement_effects_from_abilities(&preview)
                    .expect("checked prospective replacements are complete");
                assert!(replacements.iter().any(|effect| matches!(effect.replacement, crate::replacement::ReplacementAction::EnterUntapped)),
                    "depth={depth}, owner={controlled_by_owner}: discover the leaf replacement");
                assert_eq!(game.object(source).unwrap().zone, Zone::Hand);
                assert!(game.object(source).unwrap().counters.is_empty());
                assert_eq!(game.effect_store.continuous_effects.revision(), revision);
                assert_eq!(game.is_summoning_sick(source), sickness);
                assert!(game.battlefield.is_empty());
            }
        }
    }).expect("fixture worker starts").join().expect("fixture worker completes");
}

#[test]
fn checked_prospective_entry_query_distinguishes_missing_from_failed_discovery() {
    std::thread::Builder::new().stack_size(128 * 1024 * 1024).spawn(|| {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let card = CardBuilder::new(CardId::new(), "Nonfinite prospective recipient")
            .card_types(vec![CardType::Artifact]).build();
        let source = game.create_object_from_card(&card, alice, Zone::Hand);
        game.object_mut(source).unwrap().abilities_mut().push(Ability::static_ability(
            StaticAbility::new(RegrantParent(Arc::new(AtomicUsize::new(0))))));
        game.try_all_continuous_effects().expect("hand world is finite");
        let revision = game.effect_store.continuous_effects.revision();
        let event = crate::events::EnterBattlefieldEvent::new(source, Zone::Hand);
        assert!(matches!(event.try_prospective_game_state(&game),
            Err(StaticEffectDiscoveryError::RoundLimit { maximum: 128, generated_effects: 129 })));
        assert_eq!(game.object(source).unwrap().zone, Zone::Hand);
        assert_eq!(game.effect_store.continuous_effects.revision(), revision);
        assert!(game.battlefield.is_empty());
        let missing = crate::events::EnterBattlefieldEvent::new(ObjectId(u64::MAX), Zone::Hand);
        assert!(missing.try_prospective_game_state(&game).expect("missing is not discovery failure").is_none());
    }).expect("fixture worker starts").join().expect("fixture worker completes");
}

#[test]
fn direct_and_generic_entry_discovery_failures_preserve_live_zone_and_effects() {
    use crate::effects::EffectExecutor;
    std::thread::Builder::new().stack_size(128 * 1024 * 1024).spawn(|| {
        for generic in [false, true] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let alice = game.players[0].id;
            let card = CardBuilder::new(CardId::new(), "Failed prospective entry")
                .card_types(vec![CardType::Artifact]).build();
            let source = game.create_object_from_card(&card, alice, Zone::Hand);
            game.object_mut(source).unwrap().abilities_mut().push(Ability::static_ability(
                StaticAbility::new(RegrantParent(Arc::new(AtomicUsize::new(0))))));
            game.refresh_continuous_state().expect("original hand world is finite");
            let effects = game.effect_store.continuous_effects.effects().len();
            let revision = game.effect_store.continuous_effects.revision();
            let replacement_count = game.effect_store.replacement_effects.effects().len();
            let instruction = crate::effects::PutOntoBattlefieldEffect::you_control(
                crate::target::ChooseSpec::SpecificObject(source), true);
            let mut ctx = crate::effects::ExecutionContext::new_default(source, alice);
            let result = if generic {
                crate::effects::execute_effect(&mut game, &crate::effect::Effect::new(instruction), &mut ctx)
            } else {
                instruction.execute(&mut game, &mut ctx)
            };
            assert!(matches!(result, Err(crate::effects::ExecutionError::ContinuousDiscovery(
                StaticEffectDiscoveryError::RoundLimit { maximum: 128, generated_effects: 129 }))),
                "generic={generic}: discovery error must reach caller");
            assert_eq!(game.object(source).unwrap().zone, Zone::Hand);
            assert!(game.battlefield.is_empty());
            assert!(!game.is_tapped(source));
            assert_eq!(game.effect_store.continuous_effects.effects().len(), effects);
            assert_eq!(game.effect_store.continuous_effects.revision(), revision);
            assert_eq!(game.effect_store.replacement_effects.effects().len(), replacement_count);
            assert!(!ctx.decision_maker.awaiting_choice());
        }
    }).expect("fixture worker starts").join().expect("fixture worker completes");
}

#[test]
fn deep_prospective_counter_prohibitions_apply_at_entry_without_suppressing_control_counters() {
    std::thread::Builder::new().stack_size(128 * 1024 * 1024).spawn(|| {
        for depth in [1, 10, 12] {
            for prohibited in [false, true] {
                let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                let alice = game.players[0].id;
                let card = CardBuilder::new(CardId::new(), "Deep counter recipient")
                    .card_types(vec![CardType::Artifact]).build();
                let source = game.create_object_from_card(&card, alice, Zone::Hand);
                if prohibited {
                    let mut model = ironsmith_core::StaticAbility::cant_have_counters_placed();
                    for _ in 0..depth {
                        model = ironsmith_core::StaticAbility::grant_object_ability_for_filter(
                            ObjectFilter::source(), ironsmith_core::Ability::static_ability(model), "Source has ability");
                    }
                    game.object_mut(source).unwrap().abilities_mut()
                        .push(Ability::static_ability(StaticAbility::from_model(model)));
                }
                let mut ctx = crate::effects::ExecutionContext::new_default(source, alice);
                let receipt = game.move_object_with_etb_processing_with_initial_counters_with_dm(
                    source, Zone::Battlefield, vec![(crate::object::CounterType::Charge, 3)],
                    &mut ctx.decision_maker).expect("finite entry succeeds");
                let entered = receipt.assert_completed_without_additions().expect("entry is not prevented");
                let object = game.object(entered.new_id).expect("permanent entered");
                assert_eq!(object.zone, Zone::Battlefield);
                assert_eq!(object.counters.get(&crate::object::CounterType::Charge).copied().unwrap_or(0),
                    if prohibited { 0 } else { 3 }, "depth={depth}, prohibited={prohibited}");
                assert!(!ctx.decision_maker.awaiting_choice());
                assert!(game.object(source).is_none(), "entry actually changes object identity");
            }
        }
    }).expect("fixture worker starts").join().expect("fixture worker completes");
}

#[test]
fn raw_entry_matcher_uses_complete_prospective_keyword_grants() {
    use crate::events::ReplacementMatcher;
    std::thread::Builder::new().stack_size(128 * 1024 * 1024).spawn(|| {
        for depth in [1, 8, 9, 10, 12] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let alice = game.players[0].id;
            let card = CardBuilder::new(CardId::new(), "Prospective type recipient")
                .card_types(vec![CardType::Artifact]).build();
            let source = game.create_object_from_card(&card, alice, Zone::Hand);
            let mut model = ironsmith_core::StaticAbility::flying();
            for _ in 0..depth {
                model = ironsmith_core::StaticAbility::grant_object_ability_for_filter(
                    ObjectFilter::source(), ironsmith_core::Ability::static_ability(model), "Source has ability");
            }
            game.object_mut(source).unwrap().abilities_mut()
                .push(Ability::static_ability(StaticAbility::from_model(model)));
            let event = crate::events::EnterBattlefieldEvent::new(source, Zone::Hand);
            let preview = event.try_prospective_game_state(&game).expect("finite world").expect("source exists");
            assert!(preview.current_abilities(source).unwrap().iter().any(|ability| matches!(
                &ability.kind, crate::ability::AbilityKind::Static(ability)
                    if ability.id() == crate::static_abilities::StaticAbilityId::Flying)),
                "depth={depth}: checked-world keyword control");
            let matcher = crate::events::zones::matchers::WouldEnterBattlefieldMatcher::new(
                ObjectFilter::permanent().with_static_ability(crate::static_abilities::StaticAbilityId::Flying));
            let ctx = crate::events::EventContext::for_controller(alice, &game);
            assert!(matcher.matches_event(&event, &ctx.clone().with_prospective_etb_game(Some(&preview))).expect("finite matcher fixture evaluates successfully"),
                "checked supplied world is an actual positive control");
            assert!(matcher.matches_event(&event, &ctx).expect("finite matcher fixture evaluates successfully"),
                "depth={depth}: raw matcher must discover the same complete prospective flying ability");
        }
    }).expect("fixture worker starts").join().expect("fixture worker completes");
}

#[test]
fn fallible_entry_matching_uses_complete_keyword_grants() {
    use crate::events::ReplacementMatcher;
    std::thread::Builder::new().stack_size(128 * 1024 * 1024).spawn(|| {
        for depth in [1, 8, 9, 10, 12] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let alice = game.players[0].id;
            let card = CardBuilder::new(CardId::new(), "Prospective type recipient")
                .card_types(vec![CardType::Artifact]).build();
            let source = game.create_object_from_card(&card, alice, Zone::Hand);
            let mut model = ironsmith_core::StaticAbility::flying();
            for _ in 0..depth {
                model = ironsmith_core::StaticAbility::grant_object_ability_for_filter(
                    ObjectFilter::source(), ironsmith_core::Ability::static_ability(model), "Source has ability");
            }
            game.object_mut(source).unwrap().abilities_mut()
                .push(Ability::static_ability(StaticAbility::from_model(model)));
            let event = crate::events::EnterBattlefieldEvent::new(source, Zone::Hand);
            let preview = event.try_prospective_game_state(&game).expect("finite world").expect("source exists");
            assert!(preview.current_abilities(source).unwrap().iter().any(|ability| matches!(
                &ability.kind, crate::ability::AbilityKind::Static(ability)
                    if ability.id() == crate::static_abilities::StaticAbilityId::Flying)),
                "depth={depth}: checked-world keyword control");
            let matcher = crate::events::zones::matchers::WouldEnterBattlefieldMatcher::new(
                ObjectFilter::permanent().with_static_ability(crate::static_abilities::StaticAbilityId::Flying));
            let ctx = crate::events::EventContext::for_controller(alice, &game);
            assert!(matcher.matches_entry_event(&event, &ctx.clone().with_prospective_etb_game(Some(&preview))).expect("supplied finite prospective query"),
                "checked supplied world is an actual positive control");
            assert!(matcher.matches_entry_event(&event, &ctx).expect("finite prospective query"),
                "depth={depth}: raw matcher must discover the same complete prospective flying ability");
        }
    }).expect("fixture worker starts").join().expect("fixture worker completes");
}

#[test]
fn fallible_entry_matching_reports_nonfinite_prospective_discovery() {
    use crate::events::ReplacementMatcher;
    std::thread::Builder::new().stack_size(128 * 1024 * 1024).spawn(|| {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let card = CardBuilder::new(CardId::new(), "Nonfinite matcher recipient")
            .card_types(vec![CardType::Artifact]).build();
        let source = game.create_object_from_card(&card, alice, Zone::Hand);
        game.object_mut(source).unwrap().abilities_mut().push(Ability::static_ability(
            StaticAbility::new(RegrantParent(Arc::new(AtomicUsize::new(0))))));
        game.try_all_continuous_effects().expect("original world finite");
        let revision = game.effect_store.continuous_effects.revision();
        let event = crate::events::EnterBattlefieldEvent::new(source, Zone::Hand);
        let matcher = crate::events::zones::matchers::WouldEnterBattlefieldMatcher::any();
        let ctx = crate::events::EventContext::for_replacement_effect(alice, source, &game);
        assert!(matches!(matcher.matches_entry_event(&event, &ctx),
            Err(StaticEffectDiscoveryError::RoundLimit { maximum: 128, generated_effects: 129 })));
        assert_eq!(game.object(source).unwrap().zone, Zone::Hand);
        assert_eq!(game.effect_store.continuous_effects.revision(), revision);
        assert!(game.battlefield.is_empty());
    }).expect("fixture worker starts").join().expect("fixture worker completes");
}

#[test]
fn processor_and_continuation_restore_one_shot_after_copy_discovery_failure() {
    use crate::events::processing::{process_trait_event, continue_replacement_choice_with_scope};
    use crate::replacement::{ReplacementAction, ReplacementEffect};
    std::thread::Builder::new().stack_size(128 * 1024 * 1024).spawn(|| {
        for continuation in [false, true] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let alice = game.players[0].id;
            let card = CardBuilder::new(CardId::new(), "Copy failure recipient").card_types(vec![CardType::Artifact]).build();
            let recipient = game.create_object_from_card(&card, alice, Zone::Hand);
            let donor = game.create_object_from_card(&card, alice, Zone::Hand);
            let source = game.create_object_from_card(&card, alice, Zone::Battlefield);
            game.object_mut(donor).unwrap().abilities_mut().push(Ability::static_ability(
                StaticAbility::new(RegrantParent(Arc::new(AtomicUsize::new(0))))));
            game.try_all_continuous_effects().expect("original hand-donor world finite");
            let event = crate::events::EnterBattlefieldEvent::new(recipient, Zone::Hand);
            event.try_prospective_game_state(&game).expect("initial entry finite").expect("recipient exists");
            let mut copied = event.clone(); copied.enters_as_copy_of = Some(donor);
            assert!(matches!(copied.try_prospective_game_state(&game),
                Err(StaticEffectDiscoveryError::RoundLimit { maximum: 128, generated_effects: 129 })));
            let matcher = crate::events::zones::matchers::WouldEnterBattlefieldMatcher::new(ObjectFilter::specific(recipient));
            let copy_id = game.effect_store.replacement_effects.add_one_shot_effect(
                ReplacementEffect::with_matcher(source, alice, matcher.clone(), ReplacementAction::EnterAsCopy {
                    source: donor, enters_tapped: false, copy_duration: None, linked_exile_objects: vec![],
                    additional_counters: vec![], name_override: None, added_colors: Default::default(),
                    added_card_types: vec![], removes_other_card_types: false, added_supertypes: vec![],
                    removed_supertypes: vec![], added_subtypes: vec![], added_abilities: vec![],
                    set_base_power_toughness: None, copy_followups: vec![],
                }));
            if continuation {
                game.effect_store.replacement_effects.add_one_shot_effect(
                    ReplacementEffect::with_matcher(source, alice, matcher, ReplacementAction::EnterTapped));
            }
            let before = game.effect_store.replacement_effects.one_shot_effects_snapshot();
            let revision = game.effect_store.continuous_effects.revision();
            let result = if continuation {
                let pending = process_trait_event(&mut game, crate::events::Event::new_with_provenance(event, Default::default())).expect("initial choice finite");
                assert!(matches!(pending, crate::events::processing::TraitEventResult::NeedsChoice { .. }));
                continue_replacement_choice_with_scope(&mut game, pending, copy_id, None, &[], None)
            } else { process_trait_event(&mut game, crate::events::Event::new_with_provenance(event, Default::default())) };
            assert!(matches!(result, Err(crate::effects::ExecutionError::ContinuousDiscovery(
                StaticEffectDiscoveryError::RoundLimit { maximum: 128, generated_effects: 129 }))), "continuation={continuation}");
            assert_eq!(game.effect_store.replacement_effects.one_shot_effects_snapshot(), before);
            assert_eq!(game.effect_store.continuous_effects.revision(), revision);
            assert_eq!(game.object(recipient).unwrap().zone, Zone::Hand);
            assert_eq!(game.object(donor).unwrap().zone, Zone::Hand);
            assert_eq!(game.battlefield, vec![source]);
        }
    }).expect("fixture worker starts").join().expect("fixture worker completes");
}

#[test]
fn multi_entry_matching_does_not_import_a_simultaneous_siblings_static_grant() {
    use crate::events::ReplacementMatcher;
    for existing_grantor in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let card = CardBuilder::new(CardId::new(), "Batch grantor").card_types(vec![CardType::Artifact]).build();
        let grantor = game.create_object_from_card(&card, alice,
            if existing_grantor { Zone::Battlefield } else { Zone::Hand });
        game.object_mut(grantor).unwrap().abilities_mut().push(Ability::static_ability(StaticAbility::from_model(
            ironsmith_core::StaticAbility::grant_object_ability_for_filter(ObjectFilter::land(),
                ironsmith_core::Ability::static_ability(ironsmith_core::StaticAbility::flying()), "Lands have flying"))));
        let land = CardBuilder::new(CardId::new(), "Batch land").card_types(vec![CardType::Land]).build();
        let entrant = game.create_object_from_card(&land, alice, Zone::Hand);
        let mut event = crate::events::ZoneChangeEvent::with_cause(entrant, Zone::Hand, Zone::Battlefield,
            crate::events::cause::EventCause::effect(), None);
        if !existing_grantor { event.objects.insert(0, grantor); }
        let matcher = crate::events::zones::matchers::WouldEnterBattlefieldMatcher::new(
            ObjectFilter::specific(entrant).with_static_ability(crate::static_abilities::StaticAbilityId::Flying));
        let context = crate::events::EventContext::for_controller(alice, &game);
        assert_eq!(matcher.matches_event(&event, &context).expect("finite batch matching"), existing_grantor,
            "only a pre-existing grantor may grant flying to its simultaneous sibling");
        assert_eq!(game.object(entrant).unwrap().zone, Zone::Hand);
        assert_eq!(game.object(grantor).unwrap().zone,
            if existing_grantor { Zone::Battlefield } else { Zone::Hand });
    }
}

#[test]
fn multi_entry_matching_reports_failed_member_before_accepting_an_earlier_match() {
    use crate::events::ReplacementMatcher;
    std::thread::Builder::new().stack_size(128 * 1024 * 1024).spawn(|| {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let card = CardBuilder::new(CardId::new(), "Batch recipient").card_types(vec![CardType::Artifact]).build();
        let first = game.create_object_from_card(&card, alice, Zone::Hand);
        let failing = game.create_object_from_card(&card, alice, Zone::Hand);
        game.object_mut(failing).unwrap().abilities_mut().push(Ability::static_ability(
            StaticAbility::new(RegrantParent(Arc::new(AtomicUsize::new(0))))));
        game.try_all_continuous_effects().expect("original batch world finite");
        let mut event = crate::events::ZoneChangeEvent::with_cause(first, Zone::Hand, Zone::Battlefield,
            crate::events::cause::EventCause::effect(), None);
        event.objects.push(failing);
        let matcher = crate::events::zones::matchers::WouldEnterBattlefieldMatcher::any();
        let context = crate::events::EventContext::for_controller(alice, &game);
        assert!(matches!(matcher.matches_event(&event, &context),
            Err(StaticEffectDiscoveryError::RoundLimit { maximum: 128, generated_effects: 129 })),
            "an earlier successful member cannot conceal later discovery failure");
        assert!(game.battlefield.is_empty());
        assert_eq!(game.object(first).unwrap().zone, Zone::Hand);
        assert_eq!(game.object(failing).unwrap().zone, Zone::Hand);
    }).expect("fixture worker starts").join().expect("fixture worker completes");
}

#[test]
fn merged_component_matching_preserves_card_only_partition_controls() {
    use crate::events::ReplacementMatcher;
    for merged in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let card = CardBuilder::new(CardId::new(), "Partition host")
            .card_types(vec![CardType::Creature]).build();
        let host = game.create_object_from_card(&card, alice, Zone::Battlefield);
        if merged {
            let token = game.create_object_from_card(&card, alice, Zone::Stack);
            game.object_mut(token).unwrap().kind = crate::object::ObjectKind::Token;
            game.merge_mutating_creature_spell(token, host, true).expect("token-top fixture merges");
        } else {
            game.object_mut(host).unwrap().kind = crate::object::ObjectKind::Token;
        }
        let event = crate::events::ZoneChangeEvent::with_cause(host, Zone::Battlefield,
            Zone::Graveyard, crate::events::cause::EventCause::effect(), None);
        let matcher = crate::events::zones::matchers::WouldChangeZoneMatcher::new(
            ObjectFilter::default().nontoken(), Some(Zone::Battlefield), Some(Zone::Graveyard));
        let context = crate::events::EventContext::for_controller(alice, &game);
        assert_eq!(matcher.matches_merged_card_component_only(&event, &context)
            .expect("finite component query"), merged);
        assert_eq!(game.object(host).unwrap().zone, Zone::Battlefield);
    }
}

#[test]
fn merged_component_matching_reports_discovery_failure_without_mutation() {
    use crate::events::ReplacementMatcher;
    std::thread::Builder::new().stack_size(128 * 1024 * 1024).spawn(|| {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let card = CardBuilder::new(CardId::new(), "Unresolved partition world")
            .card_types(vec![CardType::Artifact]).build();
        let source = game.create_object_from_card(&card, alice, Zone::Battlefield);
        game.object_mut(source).unwrap().abilities_mut().push(Ability::static_ability(
            StaticAbility::new(RegrantParent(Arc::new(AtomicUsize::new(0))))));
        assert!(matches!(game.try_all_continuous_effects(),
            Err(StaticEffectDiscoveryError::RoundLimit { maximum: 128, generated_effects: 129 })),
            "the original query actually fails complete discovery");
        let revision = game.effect_store.continuous_effects.revision();
        let event = crate::events::ZoneChangeEvent::with_cause(source, Zone::Battlefield,
            Zone::Graveyard, crate::events::cause::EventCause::effect(), None);
        let matcher = crate::events::zones::matchers::WouldChangeZoneMatcher::new(
            ObjectFilter::default().nontoken(), Some(Zone::Battlefield), Some(Zone::Graveyard));
        let context = crate::events::EventContext::for_controller(alice, &game);
        assert!(matches!(matcher.matches_merged_card_component_only(&event, &context),
            Err(StaticEffectDiscoveryError::RoundLimit { maximum: 128, generated_effects: 129 })),
            "failed discovery must not become a successful non-match");
        assert_eq!(game.effect_store.continuous_effects.revision(), revision);
        assert_eq!(game.object(source).unwrap().zone, Zone::Battlefield);
        assert_eq!(game.battlefield, vec![source]);
    }).expect("fixture worker starts").join().expect("fixture worker completes");
}

#[derive(Debug, Clone)]
struct CounterStoppedRegrantParent(Arc<AtomicUsize>);
impl StaticAbilityKind for CounterStoppedRegrantParent {
    fn id(&self) -> StaticAbilityId { StaticAbilityId::GrantObjectAbilityForFilter }
    fn display(&self) -> String { "Regrant until entry counter is assembled".into() }
    fn generate_effects(&self, source: ObjectId, controller: PlayerId, game: &GameState)
        -> Vec<ContinuousEffect> {
        assert!(self.0.fetch_add(1, Ordering::SeqCst) < 32_768, "fixture work ceiling");
        if game.counter_count(source, crate::object::CounterType::Charge) != 0 { return Vec::new(); }
        vec![ContinuousEffect::new(source, controller, EffectTarget::Source,
            Modification::AddAbility(StaticAbility::new(self.clone())))]
    }
}

fn check_counter_stopped_copy_entry_assembly(temporary: bool) {
    use crate::replacement::{ReplacementAction, ReplacementEffect};
    std::thread::Builder::new().stack_size(128 * 1024 * 1024).spawn(move || {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let card = CardBuilder::new(CardId::new(), "Entry assembly recipient")
            .card_types(vec![CardType::Artifact]).build();
        let recipient = game.create_object_from_card(&card, alice, Zone::Hand);
        let donor = game.create_object_from_card(&card, alice, Zone::Hand);
        let source = game.create_object_from_card(&card, alice, Zone::Battlefield);
        game.object_mut(donor).unwrap().abilities_mut().push(Ability::static_ability(
            StaticAbility::new(CounterStoppedRegrantParent(Arc::new(AtomicUsize::new(0))))));
        game.try_all_continuous_effects().expect("original world finite");
        let duration = temporary.then_some(crate::effect::Until::EndOfTurn);
        let proposal = crate::events::EnterBattlefieldEvent::new(recipient, Zone::Hand)
            .with_copy_of(donor).with_copy_duration(duration.clone())
            .with_counters(crate::object::CounterType::Charge, 1);
        proposal.try_prospective_game_state(&game).expect("fully assembled copy proposal finite")
            .expect("recipient exists");
        let matcher = crate::events::zones::matchers::WouldEnterBattlefieldMatcher::new(ObjectFilter::specific(recipient));
        game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(
            source, alice, matcher, ReplacementAction::EnterAsCopy {
                source: donor, enters_tapped: false, copy_duration: duration,
                linked_exile_objects: vec![], additional_counters: vec![(crate::object::CounterType::Charge, 1)],
                name_override: None, added_colors: Default::default(), added_card_types: vec![],
                removes_other_card_types: false, added_supertypes: vec![], removed_supertypes: vec![],
                added_subtypes: vec![], added_abilities: vec![], set_base_power_toughness: None,
                copy_followups: vec![],
            }));
        let mut ctx = crate::effects::ExecutionContext::new_default(source, alice);
        let receipt = game.move_object_with_etb_processing_with_dm(recipient, Zone::Battlefield,
            &mut ctx.decision_maker).expect("complete finite proposal must not be rejected while partially assembled");
        let entered = receipt.assert_completed_without_additions().expect("entry completes");
        assert_eq!(game.object(entered.new_id).unwrap().zone, Zone::Battlefield);
        assert_eq!(game.counter_count(entered.new_id, crate::object::CounterType::Charge), 1);
        assert_eq!(game.object(donor).unwrap().zone, Zone::Hand);
        game.try_all_continuous_effects().expect("committed final world finite");
    }).expect("fixture worker starts").join().expect("fixture worker completes");
}

#[test]
fn entry_assembly_counter_stopped_permanent_copy_control() {
    check_counter_stopped_copy_entry_assembly(false);
}

#[test]
fn entry_assembly_counter_stopped_temporary_copy_is_not_validated_midway() {
    check_counter_stopped_copy_entry_assembly(true);
}

#[derive(Debug, Clone)]
struct NewIdentityRegrantParent {
    original: ObjectId,
    calls: Arc<AtomicUsize>,
}
impl StaticAbilityKind for NewIdentityRegrantParent {
    fn id(&self) -> StaticAbilityId { StaticAbilityId::GrantObjectAbilityForFilter }
    fn display(&self) -> String { "Regrant only after physical identity renewal".into() }
    fn generate_effects(&self, source: ObjectId, controller: PlayerId, _game: &GameState)
        -> Vec<ContinuousEffect> {
        assert!(self.calls.fetch_add(1, Ordering::SeqCst) < 32_768, "fixture work ceiling");
        if source == self.original { return Vec::new(); }
        vec![ContinuousEffect::new(source, controller, EffectTarget::Source,
            Modification::AddAbility(StaticAbility::new(self.clone())))]
    }
}

#[test]
fn entry_assembly_final_query_failure_restores_physical_state_and_one_shot() {
    use crate::replacement::{ReplacementAction, ReplacementEffect};
    std::thread::Builder::new().stack_size(128 * 1024 * 1024).spawn(|| {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let card = CardBuilder::new(CardId::new(), "Final query recipient")
            .card_types(vec![CardType::Artifact]).build();
        let recipient = game.create_object_from_card(&card, alice, Zone::Hand);
        let source = game.create_object_from_card(&card, alice, Zone::Battlefield);
        game.object_mut(recipient).unwrap().abilities_mut().push(Ability::static_ability(
            StaticAbility::new(NewIdentityRegrantParent {
                original: recipient, calls: Arc::new(AtomicUsize::new(0)),
            })));
        game.try_all_continuous_effects().expect("original world finite");
        crate::events::EnterBattlefieldEvent::new(recipient, Zone::Hand)
            .try_prospective_game_state(&game).expect("pre-move prospective identity finite")
            .expect("recipient exists");
        game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(
            source, alice, crate::events::zones::matchers::WouldEnterBattlefieldMatcher::new(
                ObjectFilter::specific(recipient)), ReplacementAction::EnterTapped));
        let one_shots = game.effect_store.replacement_effects.one_shot_effects_snapshot();
        let revision = game.effect_store.continuous_effects.revision();
        let mut ctx = crate::effects::ExecutionContext::new_default(source, alice);
        let result = game.move_object_with_etb_processing_with_dm(recipient, Zone::Battlefield,
            &mut ctx.decision_maker);
        assert!(matches!(result, Err(crate::effects::ExecutionError::ContinuousDiscovery(
            StaticEffectDiscoveryError::RoundLimit { maximum: 128, generated_effects: 129 }))),
            "complete committed identity must be validated before publication");
        assert_eq!(game.object(recipient).unwrap().zone, Zone::Hand);
        assert_eq!(game.battlefield, vec![source]);
        assert!(!game.is_tapped(recipient));
        assert_eq!(game.effect_store.continuous_effects.revision(), revision);
        assert_eq!(game.effect_store.replacement_effects.one_shot_effects_snapshot(), one_shots);
        assert!(!ctx.decision_maker.awaiting_choice());
    }).expect("fixture worker starts").join().expect("fixture worker completes");
}

#[test]
fn prospective_entry_controller_is_a_base_fact_below_existing_control_effects() {
    use crate::events::ReplacementMatcher;
    for entering_under_owner in [true, false] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let source_card = CardBuilder::new(CardId::new(), "Existing control source")
            .card_types(vec![CardType::Artifact]).build();
        let source = game.create_object_from_card(&source_card, alice, Zone::Battlefield);
        let land_card = CardBuilder::new(CardId::new(), "Entry control recipient")
            .card_types(vec![CardType::Land]).build();
        let entrant = game.create_object_from_card(&land_card, alice, Zone::Hand);
        game.effect_store.continuous_effects.add_effect(ContinuousEffect::new(source, alice,
            EffectTarget::Filter(ObjectFilter::land()), Modification::ChangeController(alice)));
        game.try_all_continuous_effects().expect("original control world finite");
        let revision = game.effect_store.continuous_effects.revision();
        let mut event = crate::events::EnterBattlefieldEvent::new(entrant, Zone::Hand);
        event.controller_override = Some(if entering_under_owner { alice } else { bob });
        let prospective = event.try_prospective_game_state(&game)
            .expect("entry control world finite").expect("entrant exists");
        assert_eq!(prospective.current_controller(entrant), Some(alice),
            "authored entry control is the base controller; the existing control-layer effect still applies");
        let matcher = crate::events::zones::matchers::WouldEnterBattlefieldMatcher::new(
            ObjectFilter::land().you_control());
        let context = crate::events::EventContext::for_replacement_effect(alice, source, &game)
            .with_prospective_etb_game(Some(&prospective));
        assert!(matcher.matches_event(&event, &context).expect("finite controlled-land query"),
            "replacement applicability must see the derived prospective controller");
        assert_eq!(game.object(entrant).unwrap().zone, Zone::Hand);
        assert_eq!(game.battlefield, vec![source]);
        assert_eq!(game.effect_store.continuous_effects.revision(), revision);
    }
}

#[test]
fn filtered_controller_queries_remain_finite_and_do_not_publish_partial_layer_results() {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = game.players[0].id;
    let bob = game.players[1].id;
    let source_card = CardBuilder::new(CardId::new(), "Finite control source")
        .card_types(vec![CardType::Artifact]).build();
    let source = game.create_object_from_card(&source_card, alice, Zone::Battlefield);
    let land_card = CardBuilder::new(CardId::new(), "Finite control recipient")
        .card_types(vec![CardType::Land]).build();
    let land = game.create_object_from_card(&land_card, bob, Zone::Battlefield);
    let control = game.effect_store.continuous_effects.add_effect(ContinuousEffect::new(source, alice,
        EffectTarget::Filter(ObjectFilter::land()), Modification::ChangeController(alice)));
    assert_eq!(game.current_controller(land), Some(alice), "dirty lookup is finite");
    assert_eq!(game.current_controller(source), Some(alice), "nonmatching source is finite");
    game.refresh_continuous_state().expect("filtered-control discovery completes");
    assert_eq!(game.current_characteristics(land).unwrap().controller, alice);
    assert_eq!(game.current_controller(land), Some(alice), "enclosing layer results cannot poison the resolved cache");
    game.effect_store.continuous_effects.remove_effect(control);
    game.refresh_continuous_state().expect("control removal completes");
    assert_eq!(game.current_characteristics(land).unwrap().controller, bob);
    assert_eq!(game.current_controller(land), Some(bob), "removed control does not survive in the cache");
    assert_eq!(game.current_controller(source), Some(alice));
}

#[test]
fn filtered_control_duration_excludes_itself_and_latches_after_source_control_loss() {
    for self_controlling in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let card = CardBuilder::new(CardId::new(), "Filtered duration recipient")
            .card_types(vec![CardType::Land]).build();
        let land = game.create_object_from_card(&card, bob, Zone::Battlefield);
        let source = if self_controlling { land } else {
            let card = CardBuilder::new(CardId::new(), "Filtered duration source")
                .card_types(vec![CardType::Artifact]).build();
            game.create_object_from_card(&card, alice, Zone::Battlefield)
        };
        let control = game.effect_store.continuous_effects.add_effect(
            ContinuousEffect::new(source, alice, EffectTarget::Filter(ObjectFilter::land()),
                Modification::ChangeController(alice)).until(crate::effect::Until::YouStopControllingThis));
        let expected = if self_controlling { bob } else { alice };
        assert_eq!(game.current_controller(land), Some(expected), "filtered duration must exclude its own control contribution");
        game.refresh_continuous_state().expect("filtered duration discovery is finite");
        assert_eq!(game.current_characteristics(land).unwrap().controller, expected);
        assert_eq!(game.current_controller(land), Some(expected));
        if !self_controlling {
            let source_control = game.effect_store.continuous_effects.add_effect(
                ContinuousEffect::gain_control(source, bob, source, bob));
            assert_eq!(game.current_controller(source), Some(bob));
            assert_eq!(game.current_controller(land), Some(bob), "losing the source ends filtered control");
            game.refresh_continuous_state().expect("source control loss is finite");
            assert!(!crate::continuous::continuous_effect_duration_and_condition_are_active(
                game.effect_store.continuous_effects.effects().iter().find(|effect| effect.id == control)
                    .expect("duration effect remains registered"), &game), "ended duration stays inactive");
            game.effect_store.continuous_effects.remove_effect(source_control);
            game.refresh_continuous_state().expect("source control return is finite");
            assert_eq!(game.current_controller(source), Some(alice));
            assert_eq!(game.current_controller(land), Some(bob), "expired filtered duration must not restart");
            assert_eq!(game.current_characteristics(land).unwrap().controller, bob);
        } else {
            assert!(!crate::continuous::continuous_effect_duration_and_condition_are_active(
                game.effect_store.continuous_effects.effects().iter().find(|effect| effect.id == control)
                    .expect("duration effect remains registered"), &game), "ended duration stays inactive");
        }
    }
}

#[derive(Debug, Clone)]
struct ControlEnabledRegrantParent(PlayerId, Arc<AtomicUsize>);
impl StaticAbilityKind for ControlEnabledRegrantParent {
    fn id(&self) -> StaticAbilityId { StaticAbilityId::GrantObjectAbilityForFilter }
    fn display(&self) -> String { "Regrant after the source changes controller".into() }
    fn generate_effects(&self, source: ObjectId, controller: PlayerId, _game: &GameState)
        -> Vec<ContinuousEffect> {
        assert!(self.1.fetch_add(1, Ordering::SeqCst) < 32_768, "fixture work ceiling");
        if controller != self.0 { return Vec::new(); }
        vec![ContinuousEffect::new(source, controller, EffectTarget::Source,
            Modification::AddAbility(StaticAbility::new(self.clone())))]
    }
}

#[test]
fn controller_setter_rolls_back_when_the_changed_controller_enables_failed_discovery() {
    std::thread::Builder::new().stack_size(128 * 1024 * 1024).spawn(|| {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let card = CardBuilder::new(CardId::new(), "Post-control discovery recipient")
            .card_types(vec![CardType::Artifact]).build();
        let source = game.create_object_from_card(&card, alice, Zone::Battlefield);
        let calls = Arc::new(AtomicUsize::new(0));
        game.object_mut(source).unwrap().abilities_mut().push(Ability::static_ability(
            StaticAbility::new(ControlEnabledRegrantParent(bob, calls.clone()))));
        game.remove_summoning_sickness(source);
        game.refresh_continuous_state().expect("original Alice-controlled graph is finite");
        assert_eq!(game.current_controller(source), Some(alice));
        assert!(!game.is_summoning_sick(source));
        let revision = game.effect_store.continuous_effects.revision();
        let resolution_effects = game.effect_store.continuous_effects.effects().len();
        let object_revision = game.object(source).unwrap().last_modified;
        let error = game.set_current_controller(source, bob)
            .expect_err("control change enables a graph that cannot complete");
        assert!(matches!(error, StaticEffectDiscoveryError::RoundLimit {
            maximum: 128, generated_effects: 129
        }), "{error:?}");
        assert_eq!(game.current_controller(source), Some(alice));
        assert_eq!(game.effect_store.continuous_effects.effects().len(), resolution_effects);
        assert_eq!(game.effect_store.continuous_effects.revision(), revision);
        assert_eq!(game.object(source).unwrap().last_modified, object_revision);
        assert!(!game.is_summoning_sick(source), "failed control cannot make the original source sick");
        assert!(game.effect_store.continuous_effects.static_ability_effects().is_empty());
        game.try_all_continuous_effects().expect("original graph still completes after rollback");
        assert!(calls.load(Ordering::SeqCst) < 32_768);
    }).expect("fixture worker starts").join().expect("fixture worker completes");
}

#[test]
fn ability_resolution_keeps_captured_controller_after_source_control_changes() {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = game.players[0].id;
    let bob = game.players[1].id;
    let card = CardBuilder::new(CardId::new(), "Captured ability source")
        .card_types(vec![CardType::Artifact]).build();
    let source = game.create_object_from_card(&card, bob, Zone::Battlefield);
    game.push_to_stack(crate::game_state::StackEntry::ability(source, bob,
        crate::resolution::ResolutionProgram::from_effects(vec![crate::Effect::gain_life(1)])));
    game.effect_store.continuous_effects.add_effect(ContinuousEffect::gain_control(source, alice, source, alice));
    game.refresh_continuous_state().expect("source control discovery completes");
    assert_eq!(game.current_controller(source), Some(alice));
    crate::game_loop::resolve_stack_entry(&mut game).expect("captured ability resolves");
    assert_eq!(game.player(bob).unwrap().life, 21);
    assert_eq!(game.player(alice).unwrap().life, 20);
    assert_eq!(game.current_controller(source), Some(alice));
    assert_eq!(game.object(source).unwrap().initial_controller, bob);
}

#[test]
fn spell_resolution_discovery_error_preserves_stack_and_game_state() {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = game.players[0].id;
    let bob = game.players[1].id;
    let card = CardBuilder::new(CardId::new(), "Failed spell discovery")
        .card_types(vec![CardType::Sorcery]).build();
    let spell = game.create_object_from_card(&card, alice, Zone::Stack);
    game.object_mut(spell).unwrap().spell_effect = Some(
        crate::resolution::ResolutionProgram::from_effects(vec![crate::Effect::gain_life(1)]).into());
    game.push_to_stack(crate::game_state::StackEntry::new(spell, bob));
    let missing = ObjectId::from_raw(999_998);
    game.effect_store.continuous_effects.add_effect(ContinuousEffect::new(missing, alice,
        EffectTarget::AllCreatures, Modification::AddAbility(StaticAbility::flying()))
        .with_originating_static_ability(StaticAbility::flying()));
    let revision = game.effect_store.continuous_effects.revision();
    let object_revision = game.object(spell).unwrap().last_modified;
    let error = crate::game_loop::resolve_stack_entry(&mut game).expect_err("incomplete discovery cannot resolve a spell");
    assert_eq!(error, crate::game_loop::GameLoopError::ExecutionFailed(
        crate::effects::ExecutionError::ContinuousDiscovery(
            StaticEffectDiscoveryError::MissingControllerSource { source: missing })));
    assert_eq!(game.stack.len(), 1);
    assert_eq!(game.stack[0].object_id, spell);
    assert_eq!(game.stack[0].controller, bob);
    assert_eq!(game.object(spell).unwrap().zone, Zone::Stack);
    assert_eq!(game.object(spell).unwrap().initial_controller, bob);
    assert_eq!(game.object(spell).unwrap().last_modified, object_revision);
    assert_eq!(game.effect_store.continuous_effects.revision(), revision);
    assert_eq!(game.player(alice).unwrap().life, 20);
    assert_eq!(game.player(bob).unwrap().life, 20);
    assert!(game.take_pending_trigger_events().is_empty());
}

#[test]
fn resolving_permanent_preserves_caster_beneath_continuing_control_and_color_effects() {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = game.players[0].id;
    let bob = game.players[1].id;
    let card = CardBuilder::new(CardId::new(), "Continuing spell characteristics")
        .card_types(vec![CardType::Artifact]).build();
    let spell = game.create_object_from_card(&card, alice, Zone::Stack);
    let stable_id = game.object(spell).unwrap().stable_id;
    game.push_to_stack(crate::game_state::StackEntry::new(spell, bob));
    let control = game.effect_store.continuous_effects.add_effect(
        ContinuousEffect::gain_control(spell, alice, spell, alice)
            .with_source_type(EffectSourceType::Resolution { locked_targets: vec![spell] }));
    game.effect_store.continuous_effects.add_effect(ContinuousEffect::from_resolution(
        spell, alice, vec![spell], Modification::AddColors(crate::color::ColorSet::RED)));
    game.refresh_continuous_state().expect("spell modifications complete");
    assert_eq!(game.current_controller(spell), Some(alice));
    assert_eq!(game.current_colors(spell), Some(crate::color::ColorSet::RED));
    crate::game_loop::resolve_stack_entry(&mut game).expect("controlled permanent spell resolves");
    let permanent = game.find_object_by_stable_id(stable_id).unwrap();
    assert_ne!(permanent, spell);
    assert_eq!(game.object(permanent).unwrap().zone, Zone::Battlefield);
    assert_eq!(game.object(permanent).unwrap().owner, alice);
    assert_eq!(game.object(permanent).unwrap().initial_controller, bob);
    assert_eq!(game.current_controller(permanent), Some(alice));
    assert_eq!(game.current_colors(permanent), Some(crate::color::ColorSet::RED));
    game.effect_store.continuous_effects.remove_effect(control);
    game.refresh_continuous_state().expect("control removal completes");
    assert_eq!(game.current_controller(permanent), Some(bob));
    assert_eq!(game.current_colors(permanent), Some(crate::color::ColorSet::RED));
    let card = game.move_object(permanent, Zone::Hand, crate::events::cause::EventCause::from_effect(permanent, bob)).expect("permanent leaves");
    assert_eq!(game.object(card).unwrap().owner, alice);
    assert_eq!(game.object(card).unwrap().initial_controller, alice);
    assert_eq!(game.current_controller(card), Some(alice));
    assert_eq!(game.current_colors(card), Some(crate::color::ColorSet::COLORLESS));
}

#[test]
fn stolen_spell_resolves_for_effective_controller_without_changing_caster_fact() {
    for stolen in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let card = CardBuilder::new(CardId::new(), "Controlled spell resolution fixture")
            .card_types(vec![CardType::Sorcery]).build();
        let spell = game.create_object_from_card(&card, alice, Zone::Stack);
        game.object_mut(spell).unwrap().spell_effect = Some(
            crate::resolution::ResolutionProgram::from_effects(vec![crate::Effect::gain_life(1)]).into());
        let stable_id = game.object(spell).unwrap().stable_id;
        game.push_to_stack(crate::game_state::StackEntry::new(spell, bob));
        assert_eq!(game.object(spell).unwrap().initial_controller, bob);
        assert_eq!(game.current_controller(spell), Some(bob));
        let effective = if stolen { alice } else { bob };
        if stolen {
            game.effect_store.continuous_effects.add_effect(
                ContinuousEffect::gain_control(spell, alice, spell, alice)
                    .with_source_type(EffectSourceType::Resolution { locked_targets: vec![spell] }));
        }
        game.refresh_continuous_state().expect("spell control graph is finite");
        assert_eq!(game.current_controller(spell), Some(effective));
        assert_eq!(game.object(spell).unwrap().initial_controller, bob);
        crate::game_loop::resolve_stack_entry(&mut game).expect("controlled spell resolves");
        assert_eq!(game.player(effective).unwrap().life, 21,
            "the resolving spell's you refers to its current controller, stolen={stolen}");
        let other = if stolen { bob } else { alice };
        assert_eq!(game.player(other).unwrap().life, 20);
        assert!(game.stack.is_empty());
        let departed = game.find_object_by_stable_id(stable_id).expect("spell is a graveyard card");
        assert_eq!(game.object(departed).unwrap().zone, Zone::Graveyard);
        assert_eq!(game.object(departed).unwrap().owner, alice);
        assert_eq!(game.object(departed).unwrap().initial_controller, alice);
    }
}

#[test]
fn spell_stack_controller_is_a_base_fact_below_control_effects() {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = game.players[0].id;
    let bob = game.players[1].id;
    let card = CardBuilder::new(CardId::new(), "Nonowner-cast permanent spell")
        .card_types(vec![CardType::Artifact]).build();
    let spell = game.create_object_from_card(&card, alice, Zone::Stack);
    game.push_to_stack(crate::game_state::StackEntry::new(spell, bob));
    assert_eq!(game.stack.last().unwrap().controller, bob);
    assert_eq!(game.object(spell).unwrap().owner, alice);
    let query = game.continuous_query_snapshot().expect("spell control proposal is finite");
    assert_eq!(query.current_controller(spell), Some(bob),
        "putting another player's card on the stack establishes caster control, independently of ownership");
    assert_eq!(query.current_characteristics(spell).unwrap().controller, bob);
    assert_eq!(game.stack.last().unwrap().source_snapshot.as_ref().unwrap().controller, bob,
        "the cast snapshot must use the initial spell controller");
    game.effect_store.continuous_effects.add_effect(ContinuousEffect::new(
        spell, alice, EffectTarget::Specific(spell), Modification::ChangeController(alice)));
    let stolen = game.continuous_query_snapshot().expect("spell control change is finite");
    assert_eq!(stolen.current_controller(spell), Some(alice),
        "a later layer-two control effect applies above the initial caster fact");
    assert_eq!(stolen.object(spell).unwrap().owner, alice);
}

#[test]
fn physical_entry_controller_is_a_base_fact_below_existing_control_effects() {
    for entering_under_owner in [true, false] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let source_card = CardBuilder::new(CardId::new(), "Physical control source")
            .card_types(vec![CardType::Artifact]).build();
        let source = game.create_object_from_card(&source_card, alice, Zone::Battlefield);
        let land_card = CardBuilder::new(CardId::new(), "Physical entry recipient")
            .card_types(vec![CardType::Land]).build();
        let entrant = game.create_object_from_card(&land_card, alice, Zone::Hand);
        game.effect_store.continuous_effects.add_effect(ContinuousEffect::new(source, alice,
            EffectTarget::Filter(ObjectFilter::land()), Modification::ChangeController(alice)));
        game.try_all_continuous_effects().expect("original control world is finite");
        let mut context = crate::effects::EffectContext::new_default(source, alice);
        let options = crate::effects::zones::BattlefieldEntryOptions::specific(
            if entering_under_owner { alice } else { bob }, false);
        let receipt = crate::effects::zones::move_to_battlefield_with_options(
            &mut game, &mut context, entrant, options)
            .expect("physical entry is finite").expect("entry has a terminal receipt");
        let crate::effects::zones::BattlefieldEntryOutcome::Moved(new_id) = receipt.outcome
            else { panic!("entry must move the physical permanent") };
        assert_eq!(game.current_controller(new_id), Some(alice),
            "authored physical entry control is a base fact, so existing control-layer effects still apply");
        assert_eq!(game.current_characteristics(new_id).unwrap().controller, alice);
        assert_eq!(game.object(new_id).unwrap().owner, alice);
        assert_eq!(game.object(new_id).unwrap().zone, Zone::Battlefield);
        assert_eq!(game.current_controller(source), Some(alice));
        assert!(!context.decision_maker.awaiting_choice());
    }
}

#[test]
fn complete_static_control_discovery_rebinds_you_control_grants_before_entry_matching() {
    use crate::events::ReplacementMatcher;
    for static_control in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let artifact = CardBuilder::new(CardId::new(), "Controller-dependent grant source")
            .card_types(vec![CardType::Artifact]).build();
        let grantor = game.create_object_from_card(&artifact, bob, Zone::Battlefield);
        game.object_mut(grantor).unwrap().abilities_mut().push(Ability::static_ability(
            StaticAbility::grant_object_ability_for_filter(ObjectFilter::creature().you_control(),
                Ability::static_ability(StaticAbility::flying()), "Creatures you control have flying".into())));
        let control_source = game.create_object_from_card(&artifact, alice, Zone::Battlefield);
        if static_control {
            let object = game.object_mut(control_source).unwrap();
            object.attached_to = Some(crate::object::AttachmentTarget::Object(grantor));
            object.abilities_mut().push(Ability::static_ability(
                StaticAbility::control_attached_permanent("You control the attached permanent".into())));
        } else {
            game.effect_store.continuous_effects.add_effect(
                ContinuousEffect::gain_control(control_source, alice, grantor, alice));
        }
        let creature = CardBuilder::new(CardId::new(), "Controller-bound grant recipient")
            .card_types(vec![CardType::Creature]).build();
        let alice_recipient = game.create_object_from_card(&creature, alice, Zone::Battlefield);
        let bob_recipient = game.create_object_from_card(&creature, bob, Zone::Battlefield);
        let entrant = game.create_object_from_card(&creature, alice, Zone::Hand);
        let revision = game.effect_store.continuous_effects.revision();
        let query = game.continuous_query_snapshot().expect("static-control graph is finite");
        assert_eq!(query.current_controller(grantor), Some(alice));
        assert!(query.current_has_static_ability_id(alice_recipient, StaticAbilityId::Flying),
            "static_control={static_control}: the grant must use the source's derived controller");
        assert!(!query.current_has_static_ability_id(bob_recipient, StaticAbilityId::Flying),
            "static_control={static_control}: the former controller's creatures cannot retain the grant");
        let event = crate::events::EnterBattlefieldEvent::new(entrant, Zone::Hand);
        let matcher = crate::events::zones::matchers::WouldEnterBattlefieldMatcher::new(
            ObjectFilter::creature().with_static_ability(StaticAbilityId::Flying));
        let context = crate::events::EventContext::for_controller(alice, &game);
        assert!(matcher.matches_event(&event, &context).expect("complete entry query is finite"),
            "static_control={static_control}: replacement applicability must see the controller-bound grant");
        assert_eq!(game.effect_store.continuous_effects.revision(), revision);
        assert_eq!(game.object(entrant).unwrap().zone, Zone::Hand);
        assert_eq!(game.object(grantor).unwrap().owner, bob);
    }
}

#[test]
fn static_control_dependency_loop_uses_timestamp_order_and_live_source_control() {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = game.players[0].id;
    let bob = game.players[1].id;
    let aura = CardBuilder::new(CardId::new(), "Circular control source")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![crate::types::Subtype::Aura]).build();
    let older = game.create_object_from_card(&aura, alice, Zone::Battlefield);
    let newer = game.create_object_from_card(&aura, bob, Zone::Battlefield);
    let older_timestamp = game.effect_store.continuous_effects.get_object_timestamp(older)
        .expect("older permanent has an entry timestamp");
    let newer_timestamp = game.effect_store.continuous_effects.get_object_timestamp(newer)
        .expect("newer permanent has an entry timestamp");
    assert!(older_timestamp < newer_timestamp, "fixture establishes the dependency-loop timestamp order");
    for (source, attached_to) in [(older, newer), (newer, older)] {
        let object = game.object_mut(source).unwrap();
        object.attached_to = Some(crate::object::AttachmentTarget::Object(attached_to));
        object.attachments.push(attached_to);
        object.abilities_mut().push(Ability::static_ability(StaticAbility::enchant(crate::object::AuraAttachmentFilter::Object(ObjectFilter::permanent()))));
        object.abilities_mut().push(Ability::static_ability(
            StaticAbility::control_attached_permanent("You control the enchanted permanent".into())));
    }
    let revision = game.effect_store.continuous_effects.revision();
    let query = game.continuous_query_snapshot().expect("legal control-dependency loop must have a finite timestamp result");
    // CR 613.8b: the mutual dependency is ignored and timestamps decide order.
    // The older source first makes Alice control the newer one; the newer
    // source then uses its current Alice controller to control the older one.
    assert_eq!(query.current_controller(older), Some(alice));
    assert_eq!(query.current_controller(newer), Some(alice));
    assert_eq!(query.current_characteristics(older).unwrap().controller, alice);
    assert_eq!(query.current_characteristics(newer).unwrap().controller, alice);
    assert_eq!(query.object(older).unwrap().owner, alice);
    assert_eq!(query.object(newer).unwrap().owner, bob);
    assert_eq!(game.effect_store.continuous_effects.revision(), revision);
    assert_eq!(game.object(older).unwrap().owner, alice);
    assert_eq!(game.object(newer).unwrap().owner, bob);
}


#[test]
fn static_control_dependency_chain_reorders_older_source_after_newer_control() {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20);
    let alice = game.players[0].id;
    let bob = game.players[1].id;
    let charlie = game.players[2].id;
    let aura = CardBuilder::new(CardId::new(), "Control chain source")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![crate::types::Subtype::Aura]).build();
    let permanent = CardBuilder::new(CardId::new(), "Control chain recipient")
        .card_types(vec![CardType::Artifact]).build();
    let target = game.create_object_from_card(&permanent, charlie, Zone::Battlefield);
    let older = game.create_object_from_card(&aura, bob, Zone::Battlefield);
    let newer = game.create_object_from_card(&aura, alice, Zone::Battlefield);
    let older_timestamp = game.effect_store.continuous_effects.get_object_timestamp(older)
        .expect("older aura has an entry timestamp");
    let newer_timestamp = game.effect_store.continuous_effects.get_object_timestamp(newer)
        .expect("newer aura has an entry timestamp");
    assert!(older_timestamp < newer_timestamp);
    for (source, attached_to) in [(older, target), (newer, older)] {
        game.object_mut(source).unwrap().attached_to =
            Some(crate::object::AttachmentTarget::Object(attached_to));
        game.object_mut(attached_to).unwrap().attachments.push(source);
        let object = game.object_mut(source).unwrap();
        object.abilities_mut().push(Ability::static_ability(StaticAbility::enchant(
            crate::object::AuraAttachmentFilter::Object(ObjectFilter::permanent()))));
        object.abilities_mut().push(Ability::static_ability(
            StaticAbility::control_attached_permanent("You control the enchanted permanent".into())));
    }
    let revision = game.effect_store.continuous_effects.revision();
    let query = game.continuous_query_snapshot().expect("acyclic control dependency is finite");
    // Applying newer changes what older does, so older depends on newer despite timestamps.
    assert_eq!(query.current_controller(newer), Some(alice));
    assert_eq!(query.current_controller(older), Some(alice));
    assert_eq!(query.current_controller(target), Some(alice));
    assert_eq!(query.current_characteristics(target).unwrap().controller, alice);
    assert_eq!(query.object(target).unwrap().owner, charlie);
    assert_eq!(query.object(older).unwrap().owner, bob);
    assert_eq!(game.effect_store.continuous_effects.revision(), revision);
    assert_eq!(game.object(target).unwrap().owner, charlie);
    assert_eq!(game.object(older).unwrap().owner, bob);
}


#[test]
fn missing_static_controller_source_returns_typed_error_before_matching() {
    use crate::events::ReplacementMatcher;
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = game.players[0].id;
    let card = CardBuilder::new(CardId::new(), "Controller source error entrant")
        .card_types(vec![CardType::Creature]).build();
    let entrant = game.create_object_from_card(&card, alice, Zone::Hand);
    let missing_source = ObjectId::from_raw(999_999);
    game.effect_store.continuous_effects.add_effect(ContinuousEffect::new(
        missing_source, alice, EffectTarget::AllCreatures,
        Modification::AddAbility(StaticAbility::flying()))
        .with_originating_static_ability(StaticAbility::flying()));
    let revision = game.effect_store.continuous_effects.revision();
    let context = crate::events::EventContext::for_controller(alice, &game);
    let matcher = crate::events::zones::matchers::WouldEnterBattlefieldMatcher::new(
        ObjectFilter::creature().with_static_ability(StaticAbilityId::Flying));
    let event = crate::events::EnterBattlefieldEvent::new(entrant, Zone::Hand);
    assert_eq!(matcher.matches_event(&event, &context),
        Err(StaticEffectDiscoveryError::MissingControllerSource { source: missing_source }));
    assert_eq!(game.effect_store.continuous_effects.revision(), revision);
    assert_eq!(game.object(entrant).unwrap().zone, Zone::Hand);
}


#[test]
fn controller_bound_grant_prefix_preserves_control_dependency_order() {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20);
    let alice = game.players[0].id;
    let bob = game.players[1].id;
    let charlie = game.players[2].id;
    let artifact = CardBuilder::new(CardId::new(), "Control prefix grant source")
        .card_types(vec![CardType::Artifact]).build();
    let grantor = game.create_object_from_card(&artifact, charlie, Zone::Battlefield);
    game.object_mut(grantor).unwrap().abilities_mut().push(Ability::static_ability(
        StaticAbility::grant_object_ability_for_filter(ObjectFilter::creature().you_control(),
            Ability::static_ability(StaticAbility::flying()), "Creatures you control have flying".into())));
    let aura = CardBuilder::new(CardId::new(), "Control prefix chain source")
        .card_types(vec![CardType::Enchantment]).subtypes(vec![crate::types::Subtype::Aura]).build();
    let older = game.create_object_from_card(&aura, bob, Zone::Battlefield);
    let newer = game.create_object_from_card(&aura, alice, Zone::Battlefield);
    assert!(game.effect_store.continuous_effects.get_object_timestamp(older).unwrap()
        < game.effect_store.continuous_effects.get_object_timestamp(newer).unwrap());
    for (source, attached_to) in [(older, grantor), (newer, older)] {
        game.object_mut(source).unwrap().attached_to = Some(crate::object::AttachmentTarget::Object(attached_to));
        game.object_mut(attached_to).unwrap().attachments.push(source);
        let object = game.object_mut(source).unwrap();
        object.abilities_mut().push(Ability::static_ability(StaticAbility::enchant(
            crate::object::AuraAttachmentFilter::Object(ObjectFilter::permanent()))));
        object.abilities_mut().push(Ability::static_ability(
            StaticAbility::control_attached_permanent("You control the enchanted permanent".into())));
    }
    let creature = CardBuilder::new(CardId::new(), "Control prefix grant recipient")
        .card_types(vec![CardType::Creature]).build();
    let recipients = [alice, bob, charlie].map(|owner|
        game.create_object_from_card(&creature, owner, Zone::Battlefield));
    let revision = game.effect_store.continuous_effects.revision();
    let effects = try_get_all_continuous_effects(&game, StaticEffectDiscoveryLimits::default())
        .expect("prefix fixture static discovery is finite");
    let calculate = |id| crate::continuous::calculate_characteristics_with_effects(
        id, game.objects_map(), &effects, &game.battlefield, game.commander_objects(), &game)
        .expect("single-object prefix query is complete");
    assert_eq!(calculate(grantor).controller, alice, "full control layer must resolve the chain first");
    for (index, id) in recipients.iter().enumerate() {
        assert_eq!(calculate(*id).static_abilities.iter().any(|ability| ability.id() == StaticAbilityId::Flying),
            index == 0, "single-object grant must use the dependency-correct control prefix for recipient {index}");
    }
    let query = game.continuous_query_snapshot().expect("whole-world prefix query is finite");
    assert_eq!(query.current_controller(grantor), Some(alice));
    for (index, id) in recipients.iter().enumerate() {
        assert_eq!(query.current_has_static_ability_id(*id, StaticAbilityId::Flying), index == 0);
    }
    assert_eq!(game.effect_store.continuous_effects.revision(), revision);
    assert_eq!(game.object(grantor).unwrap().owner, charlie);
}

}


#[test]
fn typed_text_domain_failure_is_checked_without_a_power_toughness_effect() {
    use crate::{card::CardBuilder, ids::{CardId, PlayerId}, types::CardType};
    #[derive(Debug, Clone)]
    struct UnmodeledWords;
    impl crate::effects::EffectExecutor for UnmodeledWords {
        fn execute(&self, _: &mut GameState, _: &mut crate::effects::ExecutionContext)
            -> Result<crate::effect::EffectOutcome, crate::effects::ExecutionError>
        { Ok(crate::effect::EffectOutcome::resolved()) }
    }
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    let controller = PlayerId::from_index(0);
    let card = CardBuilder::new(CardId::new(), "Unmodeled text witness")
        .card_types(vec![CardType::Creature]).build();
    let id = game.create_object_from_card(&card, controller, Zone::Battlefield);
    game.object_mut(id).unwrap().abilities = std::sync::Arc::new(vec![crate::ability::Ability::activated(
        crate::cost::TotalCost::free(), vec![crate::effect::Effect::new(UnmodeledWords)])]);
    game.effect_store.continuous_effects.add_effect(ContinuousEffect::from_resolution(
        id, controller, vec![id], Modification::RewriteText(
            ironsmith_core::TextChange::color(crate::color::Color::Red, crate::color::Color::Blue).unwrap())));
    let revision = game.effect_store.continuous_effects.revision();
    assert!(matches!(try_generate_continuous_effects_from_static_abilities(&game, Default::default()),
        Err(StaticEffectDiscoveryError::TextChangeDomain(_))));
    assert!(matches!(game.update_static_ability_effects(), Err(StaticEffectDiscoveryError::TextChangeDomain(_))));
    assert!(matches!(game.continuous_query_snapshot(), Err(StaticEffectDiscoveryError::TextChangeDomain(_))));
    assert_eq!(game.effect_store.continuous_effects.revision(), revision);
    assert!(!game.continuous_state_is_clean_public());
}
