//! Dependency system for continuous effects.
//!
//! Per MTG Rule 613.8, continuous effects in the same layer/sublayer that would
//! depend on each other are applied in a specific order that handles the dependency,
//! regardless of their timestamps.
//!
//! A dependency exists if:
//! - The effects are in the same layer (or sublayer for Layer 7)
//! - Applying one effect would change whether the other applies (its condition
//!   or the existence of its source ability), what it applies to, or what it
//!   does to the things it applies to
//! - Neither effect is a characteristic-defining ability, or both are
//!
//! Dependencies are detected by simulating each candidate effect against a
//! board-wide baseline of characteristics computed from the earlier layers
//! (`sort_layer_effects_with_baseline_and_started_groups`). Where the
//! simulator cannot evaluate a value or condition it falls back to a
//! conservative structural answer ("could this modification change what that
//! value or condition reads?") rather than silently assuming independence.
//!
//! When `needs_baseline_dependency_sort` proves that no dependency can exist in
//! a group, `sort_layer_effects` applies the CR 613.2 order directly:
//! characteristic-defining effects first, then timestamp order.
//!
//! Example: Humility ("All creatures lose all abilities and have base power and
//! toughness 1/1") and an anthem from a creature. The anthem depends on Humility
//! because Humility removing abilities would stop the anthem from applying.

use std::collections::{HashMap, HashSet};

use crate::ability::{Ability, AbilityKind, ActivatedAbilityRuntimeExt as _};
use crate::continuous::{
    CalculatedCharacteristics, ContinuousEffect, ContinuousEffectGroupId, EffectSourceType,
    EffectTarget, Layer, Modification, PtSublayer, enforce_ability_gain_prohibitions,
    replace_card_types_and_prune_subtypes, replace_subtypes_for_set,
};
use crate::effect::Value;
use crate::filter::PlayerFilterExt;
use crate::game_state::{GameState, ObjectMap};
use crate::ids::{ObjectId, PlayerId};
use crate::target::ObjectFilter;

fn push_static_ability_once(
    chars: &mut CalculatedCharacteristics,
    ability: crate::static_abilities::StaticAbility,
) {
    let instance_id = ability.instance_id();
    if !chars.abilities.iter().any(|runtime_ability| {
        matches!(
            &runtime_ability.kind,
            AbilityKind::Static(existing) if existing.instance_id() == instance_id
        )
    }) {
        chars
            .abilities
            .push(Ability::static_ability(ability.clone()));
    }
    if !chars
        .static_abilities
        .iter()
        .any(|existing| existing.instance_id() == instance_id)
    {
        chars.static_abilities.push(ability);
    }
}

fn ability_is_mana_for_object(
    ability: &Ability,
    game: &GameState,
    object: &crate::object::Object,
) -> bool {
    let AbilityKind::Activated(activated) = &ability.kind else {
        return false;
    };
    activated.is_runtime_mana_ability(game, object.id, game.controller_of(object))
}

fn effect_depends_on_with_baseline_and_started_groups(
    a: &ContinuousEffect,
    b: &ContinuousEffect,
    baseline: &HashMap<ObjectId, CalculatedCharacteristics>,
    objects: &ObjectMap,
    game: &GameState,
    started_groups: &HashSet<ContinuousEffectGroupId>,
    representatives: Option<&[ObjectId]>,
) -> bool {
    // Static ability effects depend on any effect that would remove the
    // originating static ability from their source, unless this effect already
    // began applying in an earlier layer (CR 613.6).
    if !effect_group_has_started(a, started_groups)
        && a.originating_static_ability.is_some()
        && modification_can_remove_static_ability_presence(&b.modification)
        && {
            game.note_dependency_pair_probed();
            source_ability_presence_changed(a, b, baseline, objects, game)
        }
    {
        return true;
    }

    // An effect whose condition reads a characteristic that B changes may stop
    // or start existing once B applies (CR 613.8, "the existence of the first
    // effect"). Conditions are evaluated against the live game, not the
    // simulated baseline, so this is a structural, conservative answer: it
    // only fires when B actually applies to something in the baseline and the
    // condition could read what B writes. A group that already started applying
    // in an earlier layer keeps applying regardless (CR 613.6).
    if !effect_group_has_started(a, started_groups)
        && a.condition.as_ref().is_some_and(|condition| {
            condition_could_be_affected_by(condition, &b.modification)
        })
        && effect_applies_to_any_object(b, baseline, objects, game)
    {
        return true;
    }

    // Check if applying B would change what A applies to.
    if modification_can_affect_effect_target(&b.modification, &a.applies_to)
        && effect_applicability_changed_counted(a, b, baseline, objects, game, representatives)
    {
        return true;
    }

    // Check if applying B would change what A does to any objects it applies to.
    if modification_can_affect_dependency_output(&a.modification, &b.modification) && {
        game.note_dependency_pair_probed();
        effect_output_changed(a, b, baseline, objects, game)
    } {
        return true;
    }

    false
}

fn effect_applies_to_any_object(
    effect: &ContinuousEffect,
    baseline: &HashMap<ObjectId, CalculatedCharacteristics>,
    objects: &ObjectMap,
    game: &GameState,
) -> bool {
    objects.iter().any(|(id, object)| {
        baseline
            .get(id)
            .is_some_and(|chars| effect_applies_with_chars(effect, object, chars, game))
    })
}

fn is_characteristic_defining_effect(effect: &ContinuousEffect) -> bool {
    matches!(
        effect.source_type,
        EffectSourceType::CharacteristicDefining
    )
}

fn effect_group_has_started(
    effect: &ContinuousEffect,
    started_groups: &HashSet<ContinuousEffectGroupId>,
) -> bool {
    effect
        .group
        .is_some_and(|group| started_groups.contains(&group))
}

fn effect_applicability_changed_counted(
    a: &ContinuousEffect,
    b: &ContinuousEffect,
    baseline: &HashMap<ObjectId, CalculatedCharacteristics>,
    objects: &ObjectMap,
    game: &GameState,
    representatives: Option<&[ObjectId]>,
) -> bool {
    game.note_dependency_pair_probed();
    effect_applicability_changed(a, b, baseline, objects, game, representatives)
}

/// True when this target's applicability probe depends only on
/// characteristic-class dimensions (types, subtypes, supertypes, colors,
/// name, controller, static-ability ids) plus the per-class object flags
/// covered by [`chars_class_key`]. Object-identity-sensitive features
/// (specific ids, tagged constraints, counters, attachments, P/T reads,
/// combat/turn history, …) disqualify the probe from class dedup.
fn effect_target_supports_chars_class_dedup(target: &EffectTarget) -> bool {
    match target {
        EffectTarget::AllPermanents | EffectTarget::AllCreatures => true,
        EffectTarget::Specific(_) | EffectTarget::Source | EffectTarget::AttachedTo(_) => false,
        EffectTarget::Filter(filter) => filter_supports_chars_class_dedup(filter),
    }
}

fn filter_supports_chars_class_dedup(filter: &ObjectFilter) -> bool {
    !crate::continuous::filter_requires_layered_clone_fallback_for_dependency(filter)
        && filter.specific.is_none()
        && !filter.source
        && !filter.other
        && filter.tagged_constraints.is_empty()
        && filter.with_counter.is_none()
        && filter.without_counter.is_none()
        && filter.power_relative_to_source.is_none()
        && !filter.shares_creature_type_with_source
        && filter.no_shared_creature_types_with.is_empty()
        && filter.characteristic_relations.is_empty()
        && !filter.is_commander
        && !filter.noncommander
        && !filter.has_tap_activated_ability
        && !filter.no_abilities
        && filter.ability_markers.is_empty()
        && filter.excluded_ability_markers.is_empty()
        && !filter.uses_power_or_toughness_characteristics()
        && filter.attached_to_object.is_none()
        && filter.blocked_or_was_blocked_by_this_turn.is_none()
        && filter.attached_to_player.is_none()
        && filter.alternative_cast.is_none()
}

/// Bucket key covering every dimension a dedup-eligible probe can read.
fn chars_class_key(
    obj: &crate::object::Object,
    chars: &CalculatedCharacteristics,
    game: &GameState,
) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = rustc_hash::FxHasher::default();
    obj.zone.hash(&mut hasher);
    obj.owner.hash(&mut hasher);
    chars.controller.hash(&mut hasher);
    chars.name.as_str().hash(&mut hasher);
    obj.name.as_str().hash(&mut hasher);
    chars.card_types.as_slice().hash(&mut hasher);
    chars.subtypes.as_slice().hash(&mut hasher);
    chars.supertypes.as_slice().hash(&mut hasher);
    chars.colors.hash(&mut hasher);
    matches!(obj.kind, crate::object::ObjectKind::Token).hash(&mut hasher);
    game.is_tapped(obj.id).hash(&mut hasher);
    game.is_face_down(obj.id).hash(&mut hasher);
    for ability in chars.static_abilities.iter() {
        ability.id().hash(&mut hasher);
    }
    hasher.finish()
}

/// One representative object per characteristic class, in deterministic
/// battlefield order. Probe outcomes are identical within a class for
/// dedup-eligible filters, so probing representatives is exact.
pub(crate) fn chars_class_representatives(
    baseline: &HashMap<ObjectId, CalculatedCharacteristics>,
    objects: &ObjectMap,
    game: &GameState,
) -> Vec<ObjectId> {
    let mut seen: HashSet<u64> = HashSet::default();
    let mut representatives = Vec::new();
    let mut ids: Vec<ObjectId> = baseline.keys().copied().collect();
    ids.sort_unstable();
    for id in ids {
        let Some(obj) = objects.get(&id) else {
            continue;
        };
        let Some(chars) = baseline.get(&id) else {
            continue;
        };
        if seen.insert(chars_class_key(obj, chars, game)) {
            representatives.push(id);
        }
    }
    representatives
}

fn effect_applicability_changed(
    a: &ContinuousEffect,
    b: &ContinuousEffect,
    baseline: &HashMap<ObjectId, CalculatedCharacteristics>,
    objects: &ObjectMap,
    game: &GameState,
    representatives: Option<&[ObjectId]>,
) -> bool {
    let dedup = representatives.filter(|_| {
        effect_target_supports_chars_class_dedup(&a.applies_to)
            && effect_target_supports_chars_class_dedup(&b.applies_to)
    });

    let probe = |id: ObjectId, obj: &crate::object::Object| -> bool {
        let Some(chars) = baseline.get(&id) else {
            return false;
        };
        // If B does not apply, chars are unchanged and A's match cannot flip.
        if !effect_applies_with_chars(b, obj, chars, game) {
            return false;
        }
        let applies_before = effect_applies_with_chars(a, obj, chars, game);
        let mut chars_after = chars.clone();
        apply_modification_to_chars_for_dependency(&b.modification, &mut chars_after, obj, game);
        let applies_after = effect_applies_with_chars(a, obj, &chars_after, game);
        applies_before != applies_after
    };

    if let Some(representatives) = dedup {
        for &id in representatives {
            if let Some(obj) = objects.get(&id)
                && probe(id, obj)
            {
                return true;
            }
        }
        return false;
    }

    for (&id, obj) in objects {
        if probe(id, obj) {
            return true;
        }
    }

    false
}

fn source_ability_presence_changed(
    a: &ContinuousEffect,
    b: &ContinuousEffect,
    baseline: &HashMap<ObjectId, CalculatedCharacteristics>,
    objects: &ObjectMap,
    game: &GameState,
) -> bool {
    let Some(originating_static_ability) = &a.originating_static_ability else {
        return false;
    };
    let Some(source_obj) = objects.get(&a.source) else {
        return false;
    };
    let Some(source_chars_before) = baseline.get(&a.source) else {
        return false;
    };

    let has_before = source_chars_before
        .static_abilities
        .contains(originating_static_ability);

    let mut source_chars_after = source_chars_before.clone();
    if effect_applies_with_chars(b, source_obj, source_chars_before, game) {
        apply_modification_to_chars_for_dependency(
            &b.modification,
            &mut source_chars_after,
            source_obj,
            game,
        );
    }

    let has_after = source_chars_after
        .static_abilities
        .contains(originating_static_ability);

    has_before != has_after
}

fn effect_output_changed(
    a: &ContinuousEffect,
    b: &ContinuousEffect,
    baseline: &HashMap<ObjectId, CalculatedCharacteristics>,
    objects: &ObjectMap,
    game: &GameState,
) -> bool {
    if let Modification::CopyActivatedAbilities {
        filter,
        counter,
        include_mana,
        only_loyalty,
        exclude_source_name,
        exclude_source_id,
        ..
    } = &a.modification
    {
        let before = collect_activated_ability_signatures(
            filter,
            *counter,
            *include_mana,
            *only_loyalty,
            *exclude_source_name,
            *exclude_source_id,
            a,
            baseline,
            objects,
            game,
        );
        let baseline_after = apply_effect_to_baseline(b, baseline, objects, game);
        let after = collect_activated_ability_signatures(
            filter,
            *counter,
            *include_mana,
            *only_loyalty,
            *exclude_source_name,
            *exclude_source_id,
            a,
            &baseline_after,
            objects,
            game,
        );
        return before != after;
    }

    if let Modification::CopyStaticAbilityVariants {
        filter,
        selectors,
        exclude_source_id,
    } = &a.modification
    {
        let before = collect_static_ability_variant_signatures(
            filter,
            selectors,
            *exclude_source_id,
            a,
            baseline,
            objects,
            game,
        );
        let baseline_after = apply_effect_to_baseline(b, baseline, objects, game);
        let after = collect_static_ability_variant_signatures(
            filter,
            selectors,
            *exclude_source_id,
            a,
            &baseline_after,
            objects,
            game,
        );
        return before != after;
    }

    if let Modification::CopyTriggeredAbilities {
        filter,
        exclude_source_name,
        exclude_source_id,
    } = &a.modification
    {
        let before = collect_triggered_ability_signatures(
            filter,
            *exclude_source_name,
            *exclude_source_id,
            a,
            baseline,
            objects,
            game,
        );
        let baseline_after = apply_effect_to_baseline(b, baseline, objects, game);
        let after = collect_triggered_ability_signatures(
            filter,
            *exclude_source_name,
            *exclude_source_id,
            a,
            &baseline_after,
            objects,
            game,
        );
        return before != after;
    }

    let (a_power_value, a_toughness_value) = match &a.modification {
        Modification::SetPower { value, .. } => (Some(value), None),
        Modification::SetToughness { value, .. } => (None, Some(value)),
        Modification::SetPowerToughness {
            power, toughness, ..
        }
        | Modification::ModifyPowerToughnessValue { power, toughness } => {
            (Some(power), Some(toughness))
        }
        _ => return false,
    };

    let applies_before_any = objects.iter().any(|(&id, obj)| {
        baseline
            .get(&id)
            .is_some_and(|chars| effect_applies_with_chars(a, obj, chars, game))
    });
    if !applies_before_any {
        return false;
    }

    if !effect_applies_to_any_object(b, baseline, objects, game) {
        return false;
    }
    let baseline_after = apply_effect_to_baseline(b, baseline, objects, game);

    for value in [a_power_value, a_toughness_value].into_iter().flatten() {
        let before = evaluate_value(value, a.source, a.controller, baseline, objects, game);
        let after = evaluate_value(
            value,
            a.source,
            a.controller,
            &baseline_after,
            objects,
            game,
        );
        // A value the simulator cannot evaluate must not read as "unchanged":
        // fall back to whether it could read anything B writes.
        if matches!(before, ValueEval::Unknown) || matches!(after, ValueEval::Unknown) {
            if value_could_be_affected_by(value, &b.modification) {
                return true;
            }
            continue;
        }
        if before != after {
            return true;
        }
    }

    false
}

fn collect_triggered_ability_signatures(
    filter: &ObjectFilter,
    exclude_source_name: bool,
    exclude_source_id: bool,
    effect: &ContinuousEffect,
    baseline: &HashMap<ObjectId, CalculatedCharacteristics>,
    objects: &ObjectMap,
    game: &GameState,
) -> HashSet<String> {
    let mut signatures = HashSet::new();
    let source_name = objects
        .get(&effect.source)
        .map(|o| o.name.as_str())
        .unwrap_or("");
    for (&id, chars) in baseline {
        let Some(object) = objects.get(&id) else {
            continue;
        };
        if exclude_source_id && id == effect.source {
            continue;
        }
        if exclude_source_name && object.name == source_name {
            continue;
        }
        if !crate::continuous::filter_matches_with_characteristics(
            filter,
            object,
            chars,
            game,
            effect.controller,
            effect.source,
        ) {
            continue;
        }
        for ability in &chars.abilities {
            if matches!(ability.kind, AbilityKind::Triggered(_)) {
                signatures.insert(format!("{:?}", ability.kind));
            }
        }
    }
    signatures
}

fn evaluate_value(
    value: &Value,
    source: ObjectId,
    effect_controller: PlayerId,
    baseline: &HashMap<ObjectId, CalculatedCharacteristics>,
    objects: &ObjectMap,
    game: &GameState,
) -> ValueEval {
    let source_chars = baseline.get(&source);
    if let Some(scalar) = evaluate_source_scalar_value(
        value,
        source_chars.and_then(|chars| chars.power),
        source_chars.and_then(|chars| chars.toughness),
    ) {
        return ValueEval::Scalar(scalar);
    }

    match value {
        Value::Count(filter) => {
            let count = baseline
                .iter()
                .filter(|(id, chars)| {
                    let Some(obj) = objects.get(id) else {
                        return false;
                    };
                    object_matches_filter_with_chars(filter, obj, chars, game, effect_controller)
                })
                .count() as i32;
            ValueEval::Scalar(count)
        }
        Value::CountScaled(filter, multiplier) => {
            let count = baseline
                .iter()
                .filter(|(id, chars)| {
                    let Some(obj) = objects.get(id) else {
                        return false;
                    };
                    object_matches_filter_with_chars(filter, obj, chars, game, effect_controller)
                })
                .count() as i32;
            ValueEval::Scalar(count * *multiplier)
        }
        Value::GreatestSharedCreatureTypeCount(filter) => {
            let mut counts = HashMap::new();
            for (id, chars) in baseline {
                let Some(obj) = objects.get(id) else {
                    continue;
                };
                if !object_matches_filter_with_chars(filter, obj, chars, game, effect_controller) {
                    continue;
                }
                let controller_group = filter.controller.as_ref().map(|_| chars.controller);
                let mut types_on_object = HashSet::new();
                for subtype in &chars.subtypes {
                    if subtype.is_creature_type() && types_on_object.insert(*subtype) {
                        *counts.entry((controller_group, *subtype)).or_insert(0i32) += 1;
                    }
                }
            }
            ValueEval::Scalar(counts.into_values().max().unwrap_or(0))
        }
        Value::Min(left, right) => {
            let left = evaluate_value(left, source, effect_controller, baseline, objects, game);
            let right = evaluate_value(right, source, effect_controller, baseline, objects, game);
            match (left, right) {
                (ValueEval::Scalar(left), ValueEval::Scalar(right)) => {
                    ValueEval::Scalar(left.min(right))
                }
                _ => ValueEval::Unknown,
            }
        }
        Value::DividedRoundedDown(value, divisor) => {
            let value = evaluate_value(value, source, effect_controller, baseline, objects, game);
            match value {
                ValueEval::Scalar(value) if *divisor != 0 => {
                    ValueEval::Scalar(value.div_euclid(*divisor))
                }
                _ => ValueEval::Unknown,
            }
        }
        Value::CreatureTypesAmong(filter) => {
            let mut seen = std::collections::HashSet::new();
            for (id, chars) in baseline {
                let Some(obj) = objects.get(id) else {
                    continue;
                };
                if object_matches_filter_with_chars(filter, obj, chars, game, effect_controller) {
                    for subtype in &chars.subtypes {
                        if subtype.is_creature_type() {
                            seen.insert(*subtype);
                        }
                    }
                }
            }
            ValueEval::Scalar(seen.len() as i32)
        }
        Value::CardTypesAmong(filter) => {
            let mut seen = std::collections::HashSet::new();
            for (id, chars) in baseline {
                let Some(obj) = objects.get(id) else {
                    continue;
                };
                if object_matches_filter_with_chars(filter, obj, chars, game, effect_controller) {
                    for card_type in &chars.card_types {
                        seen.insert(*card_type);
                    }
                }
            }
            ValueEval::Scalar(seen.len() as i32)
        }
        Value::CreaturesDiedThisTurn => ValueEval::Scalar(
            game.turn_store
                .turn_history
                .total_creatures_died_this_turn() as i32,
        ),
        Value::CreaturesDiedThisTurnControlledBy(player_filter) => {
            let filter_ctx = crate::filter::FilterContext {
                you: Some(effect_controller),
                source: Some(source),
                source_snapshot: None,
                caster: None,
                prospective_cast: None,
                active_player: None,
                opponents: Vec::new(),
                teammates: Vec::new(),
                defending_player: None,
                defending_players: Vec::new(),
                attacking_player: None,
                attacking_players: Vec::new(),
                your_commanders: Vec::new(),
                iterated_player: None,
                x_value: None,
                chosen_player: None,
                target_players: Vec::new(),
                target_objects: Vec::new(),
                tagged_objects: std::collections::HashMap::new(),
                tagged_players: std::collections::HashMap::new(),
                effect_outcomes: std::collections::HashMap::new(),
                players_in_range: game.range_players_for_source(effect_controller, Some(source)),
            };
            let mut total = 0i32;
            for player in game.players.iter().filter(|p| p.is_in_game()) {
                if !player_filter.matches_player(player.id, &filter_ctx) {
                    continue;
                }
                total += game
                    .turn_store
                    .turn_history
                    .creatures_died_under_controller(player.id) as i32;
            }
            ValueEval::Scalar(total)
        }
        Value::LandsEnteredBattlefieldThisTurn(player_filter) => {
            let filter_ctx = crate::filter::FilterContext {
                you: Some(effect_controller),
                source: Some(source),
                source_snapshot: None,
                caster: None,
                prospective_cast: None,
                active_player: None,
                opponents: Vec::new(),
                teammates: Vec::new(),
                defending_player: None,
                defending_players: Vec::new(),
                attacking_player: None,
                attacking_players: Vec::new(),
                your_commanders: Vec::new(),
                iterated_player: None,
                x_value: None,
                chosen_player: None,
                target_players: Vec::new(),
                target_objects: Vec::new(),
                tagged_objects: std::collections::HashMap::new(),
                tagged_players: std::collections::HashMap::new(),
                effect_outcomes: std::collections::HashMap::new(),
                players_in_range: game.range_players_for_source(effect_controller, Some(source)),
            };
            let mut total = 0i32;
            for player in game.players.iter().filter(|p| p.is_in_game()) {
                if !player_filter.matches_player(player.id, &filter_ctx) {
                    continue;
                }
                total += game
                    .turn_store
                    .turn_history
                    .lands_entered_under_controller(player.id) as i32;
            }
            ValueEval::Scalar(total)
        }
        Value::PowerOf(target) | Value::ToughnessOf(target) => {
            use crate::target::ChooseSpec;
            let mut values = Vec::new();
            match target.as_ref() {
                ChooseSpec::SurfaceHinted { spec, .. } => {
                    if spec.as_ref() == &ChooseSpec::Source
                        && let Some(chars) = baseline.get(&source)
                    {
                        let v = match value {
                            Value::PowerOf(_) => chars.power,
                            Value::ToughnessOf(_) => chars.toughness,
                            _ => {
                                unreachable!("Value::PowerOf/ToughnessOf arm received non-PT value")
                            }
                        };
                        if let Some(v) = v {
                            values.push(v);
                        }
                    }
                }
                ChooseSpec::Source => {
                    if let Some(chars) = baseline.get(&source) {
                        let v = match value {
                            Value::PowerOf(_) => chars.power,
                            Value::ToughnessOf(_) => chars.toughness,
                            _ => {
                                unreachable!("Value::PowerOf/ToughnessOf arm received non-PT value")
                            }
                        };
                        if let Some(v) = v {
                            values.push(v);
                        }
                    }
                }
                ChooseSpec::Object(filter) | ChooseSpec::ObjectOrPlayer(filter, _) => {
                    for (&id, chars) in baseline {
                        let Some(obj) = objects.get(&id) else {
                            continue;
                        };
                        if !object_matches_filter_with_chars(
                            filter,
                            obj,
                            chars,
                            game,
                            effect_controller,
                        ) {
                            continue;
                        }
                        let v = match value {
                            Value::PowerOf(_) => chars.power,
                            Value::ToughnessOf(_) => chars.toughness,
                            _ => {
                                unreachable!("Value::PowerOf/ToughnessOf arm received non-PT value")
                            }
                        };
                        if let Some(v) = v {
                            values.push(v);
                        }
                    }
                }
                ChooseSpec::Target(_)
                | ChooseSpec::Player(_)
                | ChooseSpec::SpecificObject(_)
                | ChooseSpec::SpecificPlayer(_)
                | ChooseSpec::AnyTarget
                | ChooseSpec::AnyOtherTarget
                | ChooseSpec::PlayerOrPlaneswalker(_)
                | ChooseSpec::AttackedPlayerOrPlaneswalker
                | ChooseSpec::SourceController
                | ChooseSpec::SourceOwner
                | ChooseSpec::Tagged(_)
                | ChooseSpec::All(_)
                | ChooseSpec::EachPlayer(_)
                | ChooseSpec::Iterated
                | ChooseSpec::WithCount(_, _)
                | ChooseSpec::WithCountValue(_, _, _) => {}
            }
            values.sort();
            ValueEval::Set(values)
        }
        _ => ValueEval::Unknown,
    }
}

fn evaluate_source_scalar_value(
    value: &Value,
    source_power: Option<i32>,
    source_toughness: Option<i32>,
) -> Option<i32> {
    match value {
        Value::Fixed(n) => Some(*n),
        Value::Add(left, right) => Some(
            evaluate_source_scalar_value(left, source_power, source_toughness)?
                + evaluate_source_scalar_value(right, source_power, source_toughness)?,
        ),
        Value::DividedRoundedDown(value, divisor) if *divisor != 0 => Some(
            evaluate_source_scalar_value(value, source_power, source_toughness)?
                .div_euclid(*divisor),
        ),
        Value::HalfRoundedDown(value) => {
            Some(evaluate_source_scalar_value(value, source_power, source_toughness)?.div_euclid(2))
        }
        Value::SourcePower => source_power,
        Value::SourceToughness => source_toughness,
        _ => None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ValueEval {
    Scalar(i32),
    Set(Vec<i32>),
    Unknown,
}

fn effect_applies_with_chars(
    effect: &ContinuousEffect,
    object: &crate::object::Object,
    chars: &CalculatedCharacteristics,
    game: &GameState,
) -> bool {
    match &effect.applies_to {
        EffectTarget::Specific(id) => *id == object.id,
        EffectTarget::Source => effect.source == object.id,
        EffectTarget::AllPermanents => object.zone == crate::zone::Zone::Battlefield,
        EffectTarget::AllCreatures => {
            object.zone == crate::zone::Zone::Battlefield
                && chars.card_types.contains(&crate::types::CardType::Creature)
        }
        EffectTarget::Filter(filter) => {
            object_matches_filter_with_chars(filter, object, chars, game, effect.controller)
        }
        // Mirrors `effect_target_applies_to_direct`: attachment plus zone, with
        // no creature requirement, so auras on lands and other permanents are
        // visible to dependency detection.
        EffectTarget::AttachedTo(source_id) => {
            object.zone == crate::zone::Zone::Battlefield
                && objects_attached_to(source_id, object, game)
        }
    }
}

fn objects_attached_to(
    source_id: &ObjectId,
    object: &crate::object::Object,
    game: &GameState,
) -> bool {
    game.object(*source_id)
        .map(|source| {
            source.attached_to == Some(crate::object::AttachmentTarget::Object(object.id))
        })
        .unwrap_or(false)
}

fn object_matches_filter_with_chars(
    filter: &ObjectFilter,
    object: &crate::object::Object,
    chars: &CalculatedCharacteristics,
    game: &GameState,
    effect_controller: PlayerId,
) -> bool {
    // Dependency-layer matching has no tagged-object context. Fail closed for
    // tag-aware filters so they never broaden into "all objects" accidentally.
    if !filter.tagged_constraints.is_empty() {
        return false;
    }

    if let Some(zone) = filter.zone
        && object.zone != zone
    {
        return false;
    }

    if !filter.card_types.is_empty()
        && !filter
            .card_types
            .iter()
            .any(|t| chars.card_types.contains(t))
    {
        return false;
    }

    if filter
        .excluded_card_types
        .iter()
        .any(|t| chars.card_types.contains(t))
    {
        return false;
    }

    if !filter.subtypes.is_empty() && !filter.subtypes.iter().any(|t| chars.subtypes.contains(t)) {
        return false;
    }
    if filter
        .excluded_subtypes
        .iter()
        .any(|t| chars.subtypes.contains(t))
    {
        return false;
    }

    if !filter.supertypes.is_empty()
        && !filter
            .supertypes
            .iter()
            .any(|t| chars.supertypes.contains(t))
    {
        return false;
    }
    if filter
        .excluded_supertypes
        .iter()
        .any(|t| chars.supertypes.contains(t))
    {
        return false;
    }

    if let Some(ref controller_filter) = filter.controller {
        use crate::filter::PlayerFilter;
        match controller_filter {
            PlayerFilter::You => {
                if chars.controller != effect_controller {
                    return false;
                }
            }
            PlayerFilter::Opponent => {
                if chars.controller == effect_controller {
                    return false;
                }
            }
            PlayerFilter::Specific(player_id) => {
                if chars.controller != *player_id {
                    return false;
                }
            }
            PlayerFilter::Any => {}
            // Dependency-layer matching doesn't have enough context to resolve
            // these controller-relative player filters safely. Fail closed.
            PlayerFilter::NotYou
            | PlayerFilter::MostLifeTied
            | PlayerFilter::LowestLifeTied
            | PlayerFilter::MostCardsInHand
            | PlayerFilter::CardsInHandAtLeastMoreThanYou { .. }
            | PlayerFilter::HasMoreLifeThanYou { .. }
            | PlayerFilter::OpponentWithMoreControlledObjectsThan { .. }
            | PlayerFilter::ControlsMost { .. }
            | PlayerFilter::MaxSpeed { .. }
            | PlayerFilter::CastCardTypeThisTurn(_)
            | PlayerFilter::AttackedBySourceThisTurn
            | PlayerFilter::WasDealtDamageBySourceThisGame { .. }
            | PlayerFilter::WasDealtCombatDamageBySourcesThisGame { .. }
            | PlayerFilter::LostLifeThisTurn { .. }
            | PlayerFilter::WasDealtCombatDamageByDistinctSourcesThisTurn { .. }
            | PlayerFilter::Teammate
            | PlayerFilter::PlayerToYourLeft
            | PlayerFilter::PlayerToYourRight
            | PlayerFilter::Active
            | PlayerFilter::Defending
            | PlayerFilter::Attacking
            | PlayerFilter::DamagedPlayer
            | PlayerFilter::EffectController
            | PlayerFilter::ChosenPlayer
            | PlayerFilter::TaggedPlayer(_)
            | PlayerFilter::IteratedPlayer
            | PlayerFilter::TargetPlayerOrControllerOfTarget
            | PlayerFilter::Target(_)
            | PlayerFilter::AliasedTarget(_)
            | PlayerFilter::Excluding { .. }
            | PlayerFilter::ControllerOf(_)
            | PlayerFilter::OwnerOf(_)
            | PlayerFilter::AliasedOwnerOf(_)
            | PlayerFilter::AliasedControllerOf(_) => return false,
        }
    }

    if let Some(colors) = filter.colors
        && chars.colors.intersection(colors).is_empty()
    {
        return false;
    }

    if filter.colorless && !chars.colors.is_empty() {
        return false;
    }
    if filter.multicolored && chars.colors.count() < 2 {
        return false;
    }

    if filter.token && object.kind != crate::object::ObjectKind::Token {
        return false;
    }
    if filter.nontoken && object.kind == crate::object::ObjectKind::Token {
        return false;
    }
    if let Some(require_face_down) = filter.face_down
        && game.is_face_down(object.id) != require_face_down
    {
        return false;
    }
    if filter.foretold && !game.is_foretold(object.id) {
        return false;
    }

    let is_tapped = game.is_tapped(object.id);
    if filter.tapped && !is_tapped {
        return false;
    }
    if filter.untapped && is_tapped {
        return false;
    }

    if let Some(power_cmp) = &filter.power {
        if let Some(power) = chars.power {
            if !power_cmp.satisfies(power) {
                return false;
            }
        } else {
            return false;
        }
    }

    if let Some(toughness_cmp) = &filter.toughness {
        if let Some(toughness) = chars.toughness {
            if !toughness_cmp.satisfies(toughness) {
                return false;
            }
        } else {
            return false;
        }
    }

    if let Some(mv_cmp) = &filter.mana_value {
        let mv = object
            .mana_cost
            .as_ref()
            .map(|mc| mc.mana_value() as i32)
            .unwrap_or(0);
        if !mv_cmp.satisfies(mv) {
            return false;
        }
    }

    if let Some(required_cost) = &filter.exact_mana_cost
        && object.mana_cost.as_deref() != Some(required_cost)
    {
        return false;
    }

    if filter.has_mana_cost {
        match &object.mana_cost {
            Some(mc) if !mc.is_empty() => {}
            _ => return false,
        }
    }

    if filter.no_x_in_cost
        && let Some(mc) = &object.mana_cost
        && mc.has_x()
    {
        return false;
    }
    if filter.has_x_in_cost && !object.mana_cost.as_ref().is_some_and(|cost| cost.has_x()) {
        return false;
    }

    if let Some(required_name) = &filter.name {
        let object_name = object.name.trim().to_ascii_lowercase();
        let required_name = required_name.trim().to_ascii_lowercase();
        if object_name != required_name {
            return false;
        }
    }

    if filter.is_commander && !game.is_commander(object.id) {
        return false;
    }

    true
}

fn collect_activated_ability_signatures(
    filter: &ObjectFilter,
    counter: Option<crate::object::CounterType>,
    include_mana: bool,
    only_loyalty: bool,
    exclude_source_name: bool,
    exclude_source_id: bool,
    effect: &ContinuousEffect,
    baseline: &HashMap<ObjectId, CalculatedCharacteristics>,
    objects: &ObjectMap,
    game: &GameState,
) -> HashSet<String> {
    use crate::ability::AbilityKind;

    let mut signatures = HashSet::new();
    let source_name = objects
        .get(&effect.source)
        .map(|o| o.name.as_str())
        .unwrap_or("");

    for (&id, chars) in baseline {
        let Some(obj) = objects.get(&id) else {
            continue;
        };
        if exclude_source_id && id == effect.source {
            continue;
        }
        if exclude_source_name && obj.name == source_name {
            continue;
        }
        if let Some(counter_type) = counter
            && obj.counters.get(&counter_type).copied().unwrap_or(0) == 0
        {
            continue;
        }
        if !object_matches_filter_with_chars(filter, obj, chars, game, effect.controller) {
            continue;
        }
        for ability in &chars.abilities {
            let AbilityKind::Activated(activated) = &ability.kind else {
                continue;
            };
            if only_loyalty && !activated.is_loyalty_ability() {
                continue;
            }
            if ability_is_mana_for_object(ability, game, obj) && !include_mana {
                continue;
            }
            signatures.insert(format!("{:?}", ability.kind));
        }
    }

    signatures
}

fn collect_static_ability_variant_signatures(
    filter: &ObjectFilter,
    selectors: &[ironsmith_core::StaticAbilityVariantSelector],
    exclude_source_id: bool,
    effect: &ContinuousEffect,
    baseline: &HashMap<ObjectId, CalculatedCharacteristics>,
    objects: &ObjectMap,
    game: &GameState,
) -> HashSet<String> {
    let mut signatures = HashSet::new();
    for (&id, chars) in baseline {
        let Some(object) = objects.get(&id) else {
            continue;
        };
        if exclude_source_id && id == effect.source {
            continue;
        }
        if !crate::continuous::filter_matches_with_characteristics(
            filter,
            object,
            chars,
            game,
            effect.controller,
            effect.source,
        ) {
            continue;
        }
        for ability in &chars.static_abilities {
            if selectors.iter().copied().any(|selector| {
                crate::continuous::static_ability_matches_variant_selector(ability, selector)
            }) {
                signatures.insert(format!("{ability:?}"));
            }
        }
    }
    signatures
}

fn apply_effect_to_baseline(
    effect: &ContinuousEffect,
    baseline: &HashMap<ObjectId, CalculatedCharacteristics>,
    objects: &ObjectMap,
    game: &GameState,
) -> HashMap<ObjectId, CalculatedCharacteristics> {
    let mut after = baseline.clone();
    for (&id, obj) in objects {
        let Some(chars) = baseline.get(&id) else {
            continue;
        };
        if effect_applies_with_chars(effect, obj, chars, game) {
            let mut new_chars = chars.clone();
            apply_modification_to_chars_for_dependency(
                &effect.modification,
                &mut new_chars,
                obj,
                game,
            );
            after.insert(id, new_chars);
        }
    }
    after
}

pub(crate) fn apply_modification_to_chars_for_dependency(
    modification: &Modification,
    chars: &mut CalculatedCharacteristics,
    object: &crate::object::Object,
    game: &GameState,
) {
    match modification {
        Modification::CopyOf { .. } => {}
        Modification::ChangeController(new_controller) => {
            chars.controller = *new_controller;
        }
        Modification::AddCardTypes(types) => {
            for t in types {
                if !chars.card_types.contains(t) {
                    chars.card_types.push(*t);
                }
            }
        }
        Modification::RemoveCardTypes(types) => {
            chars.card_types.retain(|t| !types.contains(t));
        }
        Modification::SetCardTypes(types) => {
            replace_card_types_and_prune_subtypes(
                &mut chars.card_types,
                &mut chars.subtypes,
                types,
            );
        }
        Modification::AddSubtypes(types) => {
            for t in types {
                if !chars.subtypes.contains(t) {
                    chars.subtypes.push(*t);
                }
            }
        }
        Modification::RemoveSubtypes(types) => {
            chars.subtypes.retain(|t| !types.contains(t));
        }
        Modification::SetSubtypes(types) => {
            // CR 205.1a: setting subtypes replaces the subtypes of the same
            // family (land types replace land types, creature types replace
            // creature types) and leaves the other families alone.
            replace_subtypes_for_set(&mut chars.subtypes, types);

            // Setting a land's subtype to basic land types also replaces its
            // abilities with the corresponding intrinsic mana abilities. We
            // model that consequence here so dependency checks can see effects
            // like Blood Moon shutting off Urborg's static ability in layer 4.
            if chars.card_types.contains(&crate::types::CardType::Land)
                && !types.is_empty()
                && types.iter().all(|subtype| subtype.is_basic_land_type())
            {
                chars.abilities.clear();
                chars.static_abilities.clear();
                for subtype in types {
                    if let Some(ability) = Ability::basic_land_mana(*subtype)
                        && !chars.abilities.contains(&ability)
                    {
                        chars.abilities.push(ability);
                    }
                }
            }
        }
        Modification::AddSupertypes(types) => {
            for t in types {
                if !chars.supertypes.contains(t) {
                    chars.supertypes.push(*t);
                }
            }
        }
        Modification::RemoveSupertypes(types) => {
            chars.supertypes.retain(|t| !types.contains(t));
        }
        Modification::AddAllSubtypesOfFamily(family) => {
            for subtype in family.all_subtypes() {
                if !chars.subtypes.contains(subtype) {
                    chars.subtypes.push(*subtype);
                }
            }
        }
        Modification::RemoveAllSubtypesOfFamily(family) => {
            chars.subtypes.retain(|t| !t.belongs_to_family(*family));
        }
        Modification::RemoveAllCreatureTypes => {
            chars.subtypes.retain(|t| !t.is_creature_type());
        }
        Modification::SetAuraAttachmentFilter(_) => {}
        Modification::AddColors(colors) => {
            chars.colors = chars.colors.union(*colors);
        }
        Modification::RemoveColors(colors) => {
            use crate::color::Color;
            for color in [
                Color::White,
                Color::Blue,
                Color::Black,
                Color::Red,
                Color::Green,
            ] {
                if colors.contains(color) {
                    chars.colors = chars.colors.without(color);
                }
            }
        }
        Modification::SetColors(colors) => {
            chars.colors = *colors;
        }
        Modification::MakeColorless => {
            chars.colors = crate::color::ColorSet::COLORLESS;
        }
        Modification::SetPower { value, .. } => {
            if let ValueEval::Scalar(v) = evaluate_value_simple(value, chars) {
                chars.power = Some(v);
            }
        }
        Modification::SetToughness { value, .. } => {
            if let ValueEval::Scalar(v) = evaluate_value_simple(value, chars) {
                chars.toughness = Some(v);
            }
        }
        Modification::SetPowerToughness {
            power, toughness, ..
        } => {
            if let ValueEval::Scalar(v) = evaluate_value_simple(power, chars) {
                chars.power = Some(v);
            }
            if let ValueEval::Scalar(v) = evaluate_value_simple(toughness, chars) {
                chars.toughness = Some(v);
            }
        }
        Modification::AddAbility(ability) => {
            push_static_ability_once(chars, ability.clone());
        }
        Modification::AddAbilityGeneric(ability) => {
            if let crate::ability::AbilityKind::Static(static_ability) = &ability.kind {
                push_static_ability_once(chars, static_ability.clone());
            } else {
                chars.abilities.push(ability.clone());
            }
        }
        Modification::SetAbilities(abilities) => {
            chars.abilities = abilities.clone().into();
            chars.static_abilities = abilities
                .iter()
                .filter_map(|ability| {
                    if let crate::ability::AbilityKind::Static(static_ability) = &ability.kind {
                        Some(static_ability.clone())
                    } else {
                        None
                    }
                })
                .collect();
        }
        Modification::CopyActivatedAbilities { .. }
        | Modification::CopyStaticAbilityVariants { .. }
        | Modification::CopyTriggeredAbilities { .. } => {}
        Modification::AddCombatDamageDrawAbility => {
            chars.abilities.push(crate::ability::Ability::triggered(
                crate::triggers::Trigger::this_deals_combat_damage_to_player(
                    crate::target::PlayerFilter::Any,
                ),
                vec![crate::effect::Effect::draw(1)],
            ));
        }
        Modification::RemoveAbility(ability) => {
            chars.abilities.retain(|a| {
                if let crate::ability::AbilityKind::Static(ref sa) = a.kind {
                    sa != ability
                } else {
                    true
                }
            });
            chars.static_abilities.retain(|sa| sa != ability);
        }
        Modification::RemoveStaticAbilityFamily(id) => {
            chars.abilities.retain(|a| {
                !matches!(&a.kind, crate::ability::AbilityKind::Static(sa) if sa.id() == *id)
            });
            chars.static_abilities.retain(|sa| sa.id() != *id);
        }
        Modification::RemoveAbilityGeneric { ability, .. } => {
            chars.abilities.retain(|a| a != ability);
            if let crate::ability::AbilityKind::Static(static_ability) = &ability.kind {
                chars.static_abilities.retain(|sa| sa != static_ability);
            }
        }
        Modification::RemoveAllAbilities => {
            chars.abilities.clear();
            chars.static_abilities.clear();
        }
        Modification::RemoveAllAbilitiesExceptMana => {
            chars
                .abilities
                .retain(|ability| ability_is_mana_for_object(ability, game, object));
            chars.static_abilities.clear();
        }
        Modification::ModifyPower(delta) => {
            if let Some(ref mut p) = chars.power {
                *p += delta;
            }
        }
        Modification::ModifyToughness(delta) => {
            if let Some(ref mut t) = chars.toughness {
                *t += delta;
            }
        }
        Modification::ModifyPowerToughness { power, toughness } => {
            if let Some(ref mut p) = chars.power {
                *p += power;
            }
            if let Some(ref mut t) = chars.toughness {
                *t += toughness;
            }
        }
        Modification::ModifyPowerToughnessValue { power, toughness } => {
            let power_delta = evaluate_value_simple(power, chars);
            let toughness_delta = evaluate_value_simple(toughness, chars);
            if let (Some(p), ValueEval::Scalar(delta)) = (&mut chars.power, power_delta) {
                *p += delta;
            }
            if let (Some(t), ValueEval::Scalar(delta)) = (&mut chars.toughness, toughness_delta) {
                *t += delta;
            }
        }
        Modification::ModifyPowerToughnessByColorCount {
            power_multiplier,
            toughness_multiplier,
        } => {
            let color_count = chars.colors.count() as i32;
            if let Some(ref mut p) = chars.power {
                *p += power_multiplier * color_count;
            }
            if let Some(ref mut t) = chars.toughness {
                *t += toughness_multiplier * color_count;
            }
        }
        Modification::SwitchPowerToughness => {
            std::mem::swap(&mut chars.power, &mut chars.toughness);
        }
        Modification::ChangeText { .. }
        | Modification::SetTextBox(_)
        | Modification::SetName(_) | Modification::InsertNameWords { .. }
        | Modification::CantBeBlocked
        | Modification::CantAttack
        | Modification::CantBlock
        | Modification::DoesntUntap => {}
    }
    enforce_ability_gain_prohibitions(chars, modification);
}

fn evaluate_value_simple(value: &Value, chars: &CalculatedCharacteristics) -> ValueEval {
    evaluate_source_scalar_value(value, chars.power, chars.toughness)
        .map(ValueEval::Scalar)
        .unwrap_or(ValueEval::Unknown)
}

/// Check if a Value references power or toughness of objects.
///
/// This is used to determine if a P/T-setting effect could depend on
/// other P/T modifications. For example, if an effect sets a creature's
/// power equal to the number of creatures you control, it doesn't depend
/// on P/T modifications. But if it sets power equal to another creature's
/// power, it depends on effects that modify that creature's power.
fn value_references_pt(value: &Value) -> bool {
    match value {
        Value::AnnouncedTargetTotal(metric) => matches!(metric, ironsmith_core::ChoiceAggregateMetric::Power | ironsmith_core::ChoiceAggregateMetric::Toughness),
        Value::SurfaceHinted { value, .. } => value_references_pt(value),
        // These directly reference P/T of objects
        Value::SourcePower | Value::SourceToughness => true,
        Value::PowerOf(_) | Value::ToughnessOf(_) => true,
        Value::TotalPower(_)
        | Value::TotalToughness(_)
        | Value::GreatestPower(_)
        | Value::GreatestToughness(_)
        | Value::LeastPower(_)
        | Value::LeastToughness(_) => true,
        Value::Add(left, right) | Value::Min(left, right) => {
            value_references_pt(left) || value_references_pt(right)
        }
        Value::Scaled(value, _)
        | Value::DividedRoundedDown(value, _)
        | Value::HalfRoundedDown(value) => value_references_pt(value),

        // EffectValue could reference P/T from a prior effect
        Value::EffectValue(_) | Value::EffectValueOffset(_, _) => true,
        Value::EffectMetric { metric, .. }
        | Value::EffectMetricOffset { metric, .. }
        | Value::PendingEffectMetric { metric, .. }
        | Value::PendingEffectMetricOffset { metric, .. } => matches!(
            metric,
            crate::effect::EffectMetric::FirstPower
                | crate::effect::EffectMetric::FirstToughness
                | crate::effect::EffectMetric::TotalPower
                | crate::effect::EffectMetric::TotalToughness
                | crate::effect::EffectMetric::GreatestPower
                | crate::effect::EffectMetric::GreatestToughness
        ),
        Value::PriorEffectMetric { query, .. } | Value::PendingPriorEffectMetric(query) => {
            matches!(
                query.metric,
                crate::effect::EffectMetric::FirstPower
                    | crate::effect::EffectMetric::FirstToughness
                    | crate::effect::EffectMetric::TotalPower
                    | crate::effect::EffectMetric::TotalToughness
                    | crate::effect::EffectMetric::GreatestPower
                    | crate::effect::EffectMetric::GreatestToughness
            )
        }

        // These don't reference P/T
        Value::Fixed(_)
        | Value::X
        | Value::XTimes(_)
        | Value::VoteCount(_)
        | Value::PlayerVoteCount(_)
        | Value::Count(_)
        | Value::CountScaled(_, _)
        | Value::GreatestCount(_)
        | Value::GreatestSharedCreatureTypeCount(_)
        | Value::GreatestSharedNameCount(_)
        | Value::TotalManaValue(_)
        | Value::GreatestManaValue(_)
        | Value::LeastManaValue(_)
        | Value::BasicLandTypesAmong(_)
        | Value::CreatureTypesAmong(_)
        | Value::CardTypesAmong(_)
        | Value::StaticAbilitiesAmong { .. }
        | Value::ColorsAmong(_)
        | Value::ColorPairsAmong(_)
        | Value::DistinctCounterTypesAmong(_)
        | Value::DistinctNames(_)
        | Value::DistinctManaValues(_)
        | Value::DistinctPowers(_)
        | Value::TurnHistoryCount(_)
        | Value::CreaturesDiedThisTurn
        | Value::CreaturesDiedThisTurnControlledBy(_)
        | Value::PlayersBeingAttacked
        | Value::CountPlayers(_)
        | Value::CountPlayersWithCardsInHandAtLeast(_, _)
        | Value::PlayersWhoControlMoreThanYou { .. }
        | Value::PlayersWhoControlAtLeastMoreThanYou { .. }
        | Value::PartySize(_)
        | Value::Devotion { .. }
        | Value::DevotionToChosenColor(_)
        | Value::ManaSpentToCastThisSpell
        | Value::ManaSymbolSpentToCastThisSpell { .. }
        | Value::ManaFromSourceSpentToCastThisSpell { .. }
        | Value::ManaSpentToCast(_)
        | Value::ManaSpentToCastTriggeringObject
        | Value::UnspentMana(_)
        | Value::ColorsOfManaSpentToCastThisSpell
        | Value::ManaValueOf(_)
        | Value::ColorsOf(_)
        | Value::ManaSymbolsInManaCostOf { .. }
        | Value::NameStickerCharacterCountOnSource { .. }
        | Value::LifeTotal(_)
        | Value::LifeTotalAsTurnBegan(_)
        | Value::LifeTotalDifference(_)
        | Value::LastNotedLifeTotal
        | Value::Speed(_)
        | Value::StartingLifeTotal(_)
        | Value::HalfLifeTotalRoundedUp(_)
        | Value::HalfLifeTotalRoundedDown(_)
        | Value::HalfStartingLifeTotalRoundedUp(_)
        | Value::HalfStartingLifeTotalRoundedDown(_)
        | Value::CardsInHand(_)
        | Value::CardsInLibrary(_)
        | Value::LifeGainedThisTurn(_)
        | Value::LifeLostThisTurn(_)
        | Value::CardsDiscardedThisTurn(_)
        | Value::AttractionsVisitedThisTurn(_)
        | Value::DamageDealtToPlayersThisTurn(_)
        | Value::NoncombatDamageDealtToPlayersThisTurn(_)
        | Value::NoncombatDamageDealtBySourcesControlledThisTurn { .. }
        | Value::MaxCardsDrawnThisTurn(_)
        | Value::MaxDiceRolledThisTurn(_)
        | Value::LandsEnteredBattlefieldThisTurn(_)
        | Value::MaxCardsInHand(_)
        | Value::CardsInGraveyard(_)
        | Value::SpellsCastThisTurn(_)
        | Value::SpellsCastBeforeThisTurn(_)
        | Value::CommanderCastCount(_)
        | Value::CommanderColorIdentityColors(_)
        | Value::ThisAbilityResolvedThisTurnCount
        | Value::SourceRegeneratedThisTurnCount
        | Value::SourceMutationCount
        | Value::SpellsCastThisTurnMatching { .. }
        | Value::TotalManaValueOfSpellsCastThisTurnMatching { .. }
        | Value::DamageDealtThisTurnByTaggedSpellCast(_)
        | Value::CardTypesInGraveyard(_)
        | Value::WasKicked
        | Value::WasBoughtBack
        | Value::WasEntwined
        | Value::TimesPaidLabel(_)
        | Value::KickCount
        | Value::PlayerCounters(_, _)
        | Value::CountersOnSource(_)
        | Value::CountersOn(_, _)
        | Value::WasPaid(_)
        | Value::WasPaidLabel(_)
        | Value::TimesPaid(_)
        | Value::MagicGamesLostToOpponentsSinceLastWin
        | Value::DraftNotedHighestNumber { .. }
        | Value::TaggedCount
        | Value::EventValue(_)
        | Value::EventValueOffset(_, _)
        | Value::PendingComparisonLeft
        | Value::PendingComparisonRight
        | Value::PendingComparisonDifference => false,
    }
}

/// Order a group of effects that `needs_baseline_dependency_sort` has proven
/// free of dependencies.
///
/// CR 613.2: within a layer, effects from characteristic-defining abilities
/// apply first, then all other effects in timestamp order. Ties on timestamp
/// keep the caller's order, which is deterministic.
pub fn sort_with_dependencies<'a>(effects: &[&'a ContinuousEffect]) -> Vec<&'a ContinuousEffect> {
    let mut sorted = effects.to_vec();
    sorted.sort_by_key(|effect| (!is_characteristic_defining_effect(effect), effect.timestamp));
    sorted
}

/// Return true when full baseline simulation is required to sort this effect set safely.
///
/// This is a conservative gate: we only return `false` when we can prove that
/// dependency ordering cannot matter for the given set.
pub fn needs_baseline_dependency_sort(effects: &[&ContinuousEffect], game: &GameState) -> bool {
    if effects.len() <= 1 {
        return false;
    }

    let layer = effects[0].modification.layer();
    if effects
        .iter()
        .any(|effect| effect.modification.layer() != layer)
    {
        return true;
    }

    // For non-P/T layers we currently keep full baseline sorting when multiple
    // effects are present, except for simple object-isolated text effects whose
    // applicability cannot change based on other effects in the same layer.
    if layer != Layer::PowerToughness {
        return !non_pt_group_has_trivial_ordering(effects, game);
    }

    // Dependencies are only meaningful within the same P/T sublayer.
    let mut by_sublayer: HashMap<Option<PtSublayer>, Vec<&ContinuousEffect>> =
        HashMap::with_capacity(4);
    for &effect in effects {
        by_sublayer
            .entry(effect.modification.pt_sublayer())
            .or_default()
            .push(effect);
    }

    for sublayer_effects in by_sublayer.values() {
        if sublayer_effects.len() <= 1 {
            continue;
        }
        if !sublayer_group_has_trivial_ordering(sublayer_effects, game) {
            return true;
        }
    }

    false
}

fn non_pt_group_has_trivial_ordering(effects: &[&ContinuousEffect], game: &GameState) -> bool {
    if effects.iter().all(|effect| {
        effect.condition.is_none()
            && effect.originating_static_ability.is_none()
            && matches!(
                effect.source_type,
                EffectSourceType::Resolution { ref locked_targets } if locked_targets.len() <= 1
            )
            && matches!(effect.applies_to, EffectTarget::Specific(_))
            && matches!(
                effect.modification,
                Modification::ChangeText { .. }
                    | Modification::SetTextBox(_)
                    | Modification::SetName(_) | Modification::InsertNameWords { .. }
            )
    }) {
        return true;
    }

    if identical_unconditioned_additive_group(effects) {
        return true;
    }

    if attachment_scoped_effects_have_disjoint_scopes(effects, game) {
        return true;
    }

    non_pt_group_has_no_dynamic_dependencies(effects)
}

/// True when every effect in the group is an unconditioned copy of the same
/// additive set-union modification over the same target. Ordering provably
/// cannot matter: all effects share one filter, so an object either matches
/// every effect in the group at a given stage or none of them — an unmatched
/// object is never modified by any group member and can never start matching
/// mid-sequence, while a matched object receives the same idempotent union
/// regardless of order. Six copies of Mycosynth Lattice ("all permanents are
/// artifacts...") previously forced quadratic full-board baseline sorting on
/// every characteristics recompute through exactly this shape.
fn identical_unconditioned_additive_group(effects: &[&ContinuousEffect]) -> bool {
    let first = effects[0];
    if first.condition.is_some() {
        return false;
    }
    if !matches!(
        first.modification,
        Modification::AddCardTypes(_)
            | Modification::AddSubtypes(_)
            | Modification::AddSupertypes(_)
            | Modification::AddAllSubtypesOfFamily(_)
            | Modification::AddColors(_)
    ) {
        return false;
    }
    effects.iter().skip(1).all(|effect| {
        effect.condition.is_none()
            && effect.modification == first.modification
            && effect.applies_to == first.applies_to
    })
}

/// The single battlefield object this effect can ever apply to, when the
/// effect is scoped to its own source's attachment target ("enchanted …" /
/// "equipped …") and its condition, if any, only reads that same object.
fn attachment_scoped_effect_object(
    effect: &ContinuousEffect,
    game: &GameState,
) -> Option<ObjectId> {
    let source_scoped = match &effect.applies_to {
        EffectTarget::AttachedTo(source_id) => *source_id == effect.source,
        EffectTarget::Filter(filter) => {
            filter.tagged_constraints.len() == 1
                && filter.tagged_constraints.iter().all(|constraint| {
                    matches!(
                        constraint.relation,
                        crate::filter::TaggedOpbjectRelation::IsTaggedObject
                    ) && (constraint.tag == crate::tag::TagKey::from("enchanted")
                        || constraint.tag == crate::tag::TagKey::from("equipped"))
                })
        }
        _ => false,
    };
    if !source_scoped {
        return None;
    }
    if !effect
        .condition
        .as_ref()
        .is_none_or(condition_reads_only_source_attachment)
    {
        return None;
    }
    if !modification_reads_only_scope_object(&effect.modification) {
        return None;
    }
    game.object(effect.source)?
        .attached_to
        .and_then(|target| target.object_id())
}

/// True when applying this modification only reads and writes the object it
/// applies to, so its output cannot change based on any other object's
/// characteristics.
fn modification_reads_only_scope_object(modification: &Modification) -> bool {
    match modification {
        Modification::SetCardTypes(_)
        | Modification::AddCardTypes(_)
        | Modification::RemoveCardTypes(_)
        | Modification::AddSubtypes(_)
        | Modification::RemoveSubtypes(_)
        | Modification::SetSubtypes(_)
        | Modification::AddSupertypes(_)
        | Modification::RemoveSupertypes(_)
        | Modification::AddColors(_)
        | Modification::RemoveColors(_)
        | Modification::SetColors(_)
        | Modification::MakeColorless
        | Modification::ModifyPower(_)
        | Modification::ModifyToughness(_)
        | Modification::ModifyPowerToughness { .. } => true,
        Modification::SetPower { value, .. } | Modification::SetToughness { value, .. } => {
            value_reads_only_iterated_object(value)
        }
        Modification::SetPowerToughness {
            power, toughness, ..
        } => value_reads_only_iterated_object(power) && value_reads_only_iterated_object(toughness),
        _ => false,
    }
}

fn value_reads_only_iterated_object(value: &Value) -> bool {
    match value {
        Value::Fixed(_) => true,
        Value::ManaValueOf(spec) => {
            matches!(spec.as_ref(), crate::target::ChooseSpec::Iterated)
        }
        _ => false,
    }
}

/// True when the condition only inspects characteristic-class dimensions of
/// the source's own attachment target, so applying an effect to any other
/// object cannot change its outcome.
fn condition_reads_only_source_attachment(condition: &crate::ConditionExpr) -> bool {
    match condition {
        crate::ConditionExpr::AttachedToSourceMatches(filter) => {
            filter_supports_chars_class_dedup(filter)
        }
        crate::ConditionExpr::Not(inner) => condition_reads_only_source_attachment(inner),
        crate::ConditionExpr::And(left, right) | crate::ConditionExpr::Or(left, right) => {
            condition_reads_only_source_attachment(left)
                && condition_reads_only_source_attachment(right)
        }
        _ => false,
    }
}

/// True when every effect in the group is attachment-scoped (see
/// [`attachment_scoped_effect_object`]) and the scope objects are pairwise
/// distinct and never another effect's source. Such effects cannot form a CR
/// 613.8 dependency — applying one cannot change whether another applies,
/// what it applies to, or what it does — so baseline simulation would only
/// reproduce timestamp order at a quadratic full-board cost (several
/// independent animation auras previously made every characteristics
/// recompute rebuild layer baselines for the entire object map).
fn attachment_scoped_effects_have_disjoint_scopes(
    effects: &[&ContinuousEffect],
    game: &GameState,
) -> bool {
    let mut scopes: Vec<(ObjectId, ObjectId)> = Vec::with_capacity(effects.len());
    for effect in effects {
        let Some(scope) = attachment_scoped_effect_object(effect, game) else {
            return false;
        };
        scopes.push((scope, effect.source));
    }
    for (index, &(scope, _)) in scopes.iter().enumerate() {
        for (other_index, &(other_scope, other_source)) in scopes.iter().enumerate() {
            if index == other_index {
                continue;
            }
            if scope == other_scope || scope == other_source {
                return false;
            }
        }
    }
    true
}

fn non_pt_group_has_no_dynamic_dependencies(effects: &[&ContinuousEffect]) -> bool {
    for i in 0..effects.len() {
        for j in 0..effects.len() {
            if i == j {
                continue;
            }

            let a = effects[i];
            let b = effects[j];
            if a.originating_static_ability.is_some()
                && modification_can_remove_static_ability_presence(&b.modification)
            {
                return false;
            }
            if a
                .condition
                .as_ref()
                .is_some_and(|condition| condition_could_be_affected_by(condition, &b.modification))
            {
                return false;
            }
            if modification_can_affect_effect_target(&b.modification, &a.applies_to) {
                return false;
            }
            if modification_can_affect_dependency_output(&a.modification, &b.modification) {
                return false;
            }
        }
    }

    true
}

/// Could applying `modification` change the outcome of `condition`?
///
/// Conditions are evaluated against the live game rather than the simulated
/// baseline, so dependency detection answers structurally: a condition that
/// inspects objects through a filter depends on modifications that can change
/// what that filter reads; a condition that reads power or toughness depends on
/// layer 7 modifications; conditions about players, turn history, mana, votes,
/// counters, status and zones cannot be changed by any layered effect. Variants
/// the classifier does not recognise are treated as able to read any
/// characteristic, so unknown shapes stay conservative instead of silently
/// independent.
pub(crate) fn condition_could_be_affected_by(
    condition: &crate::ConditionExpr,
    modification: &Modification,
) -> bool {
    use crate::ConditionExpr as C;

    let filters_affected = |filters: &[&ObjectFilter]| {
        filters
            .iter()
            .any(|filter| modification_can_affect_filter(modification, filter))
    };
    let pt_affected = modification.layer() == Layer::PowerToughness;
    let types_affected = modification_can_change_type_characteristics(modification);
    let any_characteristic_affected = pt_affected
        || modification_can_change_abilities_or_matching_characteristics(modification);

    match condition {
        C::Not(inner) => condition_could_be_affected_by(inner, modification),
        C::And(left, right) | C::Or(left, right) => {
            condition_could_be_affected_by(left, modification)
                || condition_could_be_affected_by(right, modification)
        }

        // Object-inspecting conditions expressed through a filter.
        C::YouControl(filter)
        | C::OpponentControls(filter)
        | C::YouHaveCardInHandMatching(filter)
        | C::ObjectEnteredBattlefieldThisTurn(filter)
        | C::ObjectEnteredBattlefieldLastTurn(filter)
        | C::ObjectPutIntoGraveyardFromBattlefieldThisTurn(filter)
        | C::SourceMatches(filter)
        | C::AttachedToSourceMatches(filter)
        | C::TargetMatches(filter)
        | C::TaggedObjectMatches(_, filter)
        | C::TaggedObjectMatchedLastKnown(_, filter)
        | C::SourceSoulbondPartnerMatches(filter) => filters_affected(&[filter]),
        C::PlayerControls { filter, .. }
        | C::PlayerHasAtLeast { filter, .. }
        | C::PlayerControlsExactly { filter, .. }
        | C::PlayerControlsMost { filter, .. }
        | C::PlayerControlsMoreThanEachOtherPlayer { filter, .. }
        | C::PlayerControlsMoreThanYou { filter, .. }
        | C::AnOpponentControlsMoreThanPlayer { filter, .. }
        | C::AnOpponentHasFewerThanPlayer { filter, .. }
        | C::PlayerRemovedDraftCardMatching { filter, .. }
        | C::SourceCrewedByExactly { filter, .. }
        | C::PlayerTaggedObjectMatches { filter, .. }
        | C::SourceInGraveyardWithCardsAbove { filter, .. } => filters_affected(&[filter]),
        C::PlayerHasAtLeastWithDifferentPowers { filter, .. } => {
            pt_affected || filters_affected(&[filter])
        }
        C::CreatureDealtDamageBySourceDiedThisTurn { victim, .. } => {
            filters_affected(&[victim])
        }
        C::AttachmentCount { attachment, .. } => filters_affected(&[attachment]),
        C::CountComparison { count, .. } | C::CountParity { count, .. } => {
            anthem_count_could_be_affected_by(count, modification)
        }
        C::ValueComparison { left, right, .. } => {
            value_could_be_affected_by(left, modification)
                || value_could_be_affected_by(right, modification)
        }
        C::ValueIsPrime(value) => value_could_be_affected_by(value, modification),

        // Conditions that read types, colors or land types without a filter.
        C::PlayerControlsBasicLandTypesAmongLandsOrMore { .. }
        | C::EnchantedPermanentIsCreature
        | C::EnchantedPermanentIsLand
        | C::EnchantedPermanentIsEquipment
        | C::EnchantedPermanentIsVehicle
        | C::CardInYourGraveyard { .. }
        | C::PlayerHasCardTypesInGraveyardOrMore { .. }
        | C::PlayerWasDealtCombatDamageByCreatureSubtypeThisTurn { .. } => types_affected,
        C::TargetObjectsHaveDifferentColorSets => {
            matches!(modification.layer(), Layer::Color | Layer::Copy)
        }
        C::YouHaveFullParty => types_affected,
        C::YouControlMoreCreaturesThanTargetSpellController => types_affected,

        // Conditions that read power or toughness.
        C::SourcePowerAtLeast(_)
        | C::TargetHasGreatestPowerAmongCreatures
        | C::ControlCreaturesTotalPowerAtLeast(_) => pt_affected || types_affected,
        C::TargetManaValueLteColorsSpentToCastThisSpell => {
            matches!(modification.layer(), Layer::Copy)
        }

        // Player, turn, history, mana, vote, counter, status and zone facts:
        // no layered effect changes these.
        C::PlayerLifeAtMostHalfStartingLifeTotal { .. }
        | C::PlayerLifeLessThanHalfStartingLifeTotal { .. }
        | C::PlayerHasLessLifeThanYou { .. }
        | C::PlayerHasMoreLifeThanYou { .. }
        | C::PlayerHasNoOpponentWithMoreLifeThan { .. }
        | C::PlayerHasMoreLifeThanEachOtherPlayer { .. }
        | C::PlayerIsMonarch { .. }
        | C::PlayerHasInitiative { .. }
        | C::PlayerHasCitysBlessing { .. }
        | C::PlayerHasEnduringStory { .. }
        | C::SourceIsRingBearer { .. }
        | C::PlayerRingTemptedThisGameOrMore { .. }
        | C::PlayerCommittedCrimeThisTurn { .. }
        | C::PlayerRolledResultThisTurn { .. }
        | C::PlayerCompletedDungeon { .. }
        | C::LifeTotalOrLess(_)
        | C::LifeTotalOrGreater(_)
        | C::CardsInHandOrMore(_)
        | C::PlayerCardsInHandOrMore { .. }
        | C::PlayerCardsInHandOrFewer { .. }
        | C::PlayerCardsInHandAtTurnStartOrMore { .. }
        | C::PlayerCardsInHandAtTurnStartOrFewer { .. }
        | C::PlayerHasMoreCardsInHandThanYou { .. }
        | C::PlayerHasMoreCardsInHandThanEachOtherPlayer { .. }
        | C::PlayerHasPoisonCountersOrMore { .. }
        | C::PlayerHasCountersOrMore { .. }
        | C::YourTurn
        | C::CurrentTurnIsExtra
        | C::YourFirstTurnsOfTheGameOrFewer(_)
        | C::CreatureDiedThisTurn
        | C::CreatureDiedThisTurnOrMore(_)
        | C::CreatureCardPutIntoYourGraveyardThisTurn
        | C::CastSpellThisTurn
        | C::PlayerCastSpellsThisTurnOrMore { .. }
        | C::AttackedThisTurn
        | C::AttackedWithNOrMoreCreaturesThisTurn(_)
        | C::OpponentLostLifeThisTurn
        | C::AnyPlayerLostLifeThisTurnOrMore { .. }
        | C::OpponentWasDealtDamageThisTurn
        | C::OpponentWasDealtDamageThisTurnOrMore(_)
        | C::PermanentLeftBattlefieldThisTurn
        | C::NonlandPermanentLeftBattlefieldThisTurn
        | C::SpellWasWarpedThisTurn
        | C::PermanentLeftBattlefieldUnderYourControlThisTurn { .. }
        | C::SourceWasCast
        | C::ThisSpellWasCastAtSorceryTiming
        | C::ThisSpellEscaped
        | C::ThisSpellWasCastFromZone(_)
        | C::ThisSpellWasCastFromNonHand
        | C::PlayerTappedLandForManaThisTurn { .. }
        | C::PlayerGainedLifeThisTurnOrMore { .. }
        | C::PlayerHadLandEnterBattlefieldThisTurn { .. }
        | C::PlayerDescendedThisTurn { .. }
        | C::NoSpellsWereCastLastTurn
        | C::SpellsWereCastLastTurnOrMore(_)
        | C::TargetIsTapped
        | C::TargetIsAttacking
        | C::TargetIsBlocked
        | C::TargetWasKicked
        | C::ThisSpellWasKicked
        | C::ThisSpellPaidLabel(_)
        | C::TargetSpellCastOrderThisTurn(_)
        | C::TargetSpellControllerIsPoisoned
        | C::TargetSpellManaSpentToCastAtLeast { .. }
        | C::TriggeringSpellManaSpentToCastAtLeast { .. }
        | C::ColoredManaSpentToCastThisSpellAtLeast(_)
        | C::TriggeringSpellColoredManaSpentToCastAtLeast(_)
        | C::ItIsNight
        | C::FirstCombatPhaseOfTurn
        | C::SourceControllersMainPhase
        | C::SourceControllersCombatPhase
        | C::SourceControllersEndStep
        | C::SourceIsTapped
        | C::SourceIsSaddled
        | C::SourceDevouredCreaturesOrMore(_)
        | C::SourceIsHarnessed
        | C::SourceIsMonstrous
        | C::SourceIsRenowned
        | C::SourceIsFaceDown
        | C::SourceHasNoCounter(_)
        | C::SourceHasCounterAtLeast { .. }
        | C::SourceHasCountersAtLeast(_)
        | C::SourceDealtCombatDamageToPlayerThisTurn
        | C::ManaSpentToCastThisSpellAtLeast { .. }
        | C::SnowManaOfAnySpellColorSpentToCastThisSpell
        | C::TriggeringSpellSnowManaOfAnySpellColorSpentToCast
        | C::SameColorManaSpentToCastThisSpellAtLeast(_)
        | C::ColorsOfManaSpentToCastThisSpellOrMore(_)
        | C::YouControlCommander
        | C::TaggedObjectIsTopOfLibrary { .. }
        | C::StableObjectIsTopOfLibrary { .. }
        | C::TaggedObjectWasCast(_)
        | C::TaggedObjectIsSoulbondPaired(_)
        | C::EnchantedPermanentAttackedThisTurn
        | C::EnchantedPermanentAttackedOrBlockedSinceLastUpkeep
        | C::SourceBlockedOrBecameBlockedSinceLastUpkeep
        | C::TargetIsSoulbondPaired
        | C::PlayerTaggedObjectEnteredBattlefieldThisTurn { .. }
        | C::PlayerOwnsCardNamedInZones { .. }
        | C::ThisAbilityResolvedThisTurnExactly(_)
        | C::FirstTimeThisTurn
        | C::SourceFirstCrewedThisTurn
        | C::MaxTimesEachTurn(_)
        | C::DoThisMaxTimesEachTurn(_)
        | C::TriggeringObjectWasEnchanted
        | C::TriggeringObjectBecameTappedFirstTimeThisTurn
        | C::TriggeringObjectHadCountersPutFirstTimeThisTurn
        | C::TriggeringObjectHadToAttackThisCombat
        | C::TriggeringObjectHadCounters { .. }
        | C::SourceIsInZone(_)
        | C::ActivationTiming(_)
        | C::MaxActivationsPerTurn(_)
        | C::MaxActivationsPerObject(_)
        | C::SourceIsEquipped
        | C::SourceIsEnchanted
        | C::EquippedCreatureTapped
        | C::EquippedCreatureUntapped
        | C::EquippedCreatureAttacking
        | C::SourceChosenOption(_)
        | C::SecretChoicesMatch
        | C::VoteOptionGetsMoreVotes(_)
        | C::VoteOptionGetsMoreVotesOrTied(_)
        | C::OwnsCardExiledWithCounter(_)
        | C::SourceAttackedThisTurn
        | C::SourceAttackedBattleThisTurn
        | C::SourceSuspected
        | C::SourceCameUnderYourControlThisTurn
        | C::SourceAttackedOrBlockedThisTurn
        | C::SourceIsUntapped
        | C::SourceIsAttacking
        | C::SourceIsBlocking
        | C::SourceIsSoulbondPaired
        | C::TurnHistory(_)
        | C::PlayerGraveyardHasCardsAtLeast { .. }
        | C::XValueAtLeast(_)
        | C::AllTargetsStillLegal => false,

        // Opaque conditions may read anything.
        C::Custom(_) => any_characteristic_affected,

        #[allow(unreachable_patterns)]
        _ => any_characteristic_affected,
    }
}

fn anthem_count_could_be_affected_by(
    count: &ironsmith_core::AnthemCountExpression,
    modification: &Modification,
) -> bool {
    use ironsmith_core::AnthemCountExpression as A;
    match count {
        A::MatchingFilter(filter)
        | A::GreatestManaValueAmong(filter)
        | A::AttachedToSource(filter)
        | A::AttachedToAffected(filter) => modification_can_affect_filter(modification, filter),
        A::ColorsOfAffected => matches!(modification.layer(), Layer::Color | Layer::Copy),
        A::GraveyardsWithAtLeastCards { .. }
        | A::AffectedAttackedThisTurn
        | A::CountersOnSource(_)
        | A::CountersOnSourceWithSurface { .. }
        | A::CountersOnSourceWithPronoun { .. }
        | A::StickersOnSource { .. } => false,
        #[allow(unreachable_patterns)]
        _ => {
            modification.layer() == Layer::PowerToughness
                || modification_can_change_abilities_or_matching_characteristics(modification)
        }
    }
}

/// Could applying `modification` change what `value` evaluates to?
///
/// Fixed numbers and player-side quantities never change under layered
/// effects. Counts and aggregates over a filter change when the filter could
/// be affected, or when they aggregate power or toughness and the modification
/// is in layer 7. Anything else is treated conservatively.
fn value_could_be_affected_by(value: &Value, modification: &Modification) -> bool {
    let pt_affected = modification.layer() == Layer::PowerToughness;
    match value {
        Value::AnnouncedTargetTotal(_) => true,
        Value::SurfaceHinted { value, .. } => value_could_be_affected_by(value, modification),
        Value::Fixed(_)
        | Value::X
        | Value::XTimes(_)
        | Value::VoteCount(_)
        | Value::PlayerVoteCount(_)
        | Value::LifeTotal(_)
        | Value::LifeTotalAsTurnBegan(_)
        | Value::LifeTotalDifference(_)
        | Value::LastNotedLifeTotal
        | Value::Speed(_)
        | Value::StartingLifeTotal(_)
        | Value::HalfLifeTotalRoundedUp(_)
        | Value::HalfLifeTotalRoundedDown(_)
        | Value::HalfStartingLifeTotalRoundedUp(_)
        | Value::HalfStartingLifeTotalRoundedDown(_)
        | Value::CardsInHand(_)
        | Value::CardsInLibrary(_)
        | Value::LifeGainedThisTurn(_)
        | Value::LifeLostThisTurn(_)
        | Value::CardsDiscardedThisTurn(_)
        | Value::AttractionsVisitedThisTurn(_)
        | Value::DamageDealtToPlayersThisTurn(_)
        | Value::NoncombatDamageDealtToPlayersThisTurn(_)
        | Value::NoncombatDamageDealtBySourcesControlledThisTurn { .. }
        | Value::MaxCardsDrawnThisTurn(_)
        | Value::MaxDiceRolledThisTurn(_)
        | Value::LandsEnteredBattlefieldThisTurn(_)
        | Value::MaxCardsInHand(_)
        | Value::CardsInGraveyard(_)
        | Value::SpellsCastThisTurn(_)
        | Value::SpellsCastBeforeThisTurn(_)
        | Value::CommanderCastCount(_)
        | Value::ThisAbilityResolvedThisTurnCount
        | Value::SourceRegeneratedThisTurnCount
        | Value::SourceMutationCount
        | Value::CreaturesDiedThisTurn
        | Value::CreaturesDiedThisTurnControlledBy(_)
        | Value::PlayersBeingAttacked
        | Value::CountPlayers(_)
        | Value::CountPlayersWithCardsInHandAtLeast(_, _)
        | Value::ManaSpentToCastThisSpell
        | Value::ManaSymbolSpentToCastThisSpell { .. }
        | Value::ManaFromSourceSpentToCastThisSpell { .. }
        | Value::ManaSpentToCast(_)
        | Value::ManaSpentToCastTriggeringObject
        | Value::UnspentMana(_)
        | Value::ColorsOfManaSpentToCastThisSpell
        | Value::WasKicked
        | Value::WasBoughtBack
        | Value::WasEntwined
        | Value::TimesPaidLabel(_)
        | Value::KickCount
        | Value::PlayerCounters(_, _)
        | Value::CountersOnSource(_)
        | Value::CountersOn(_, _)
        | Value::WasPaid(_)
        | Value::WasPaidLabel(_)
        | Value::TimesPaid(_)
        | Value::MagicGamesLostToOpponentsSinceLastWin
        | Value::DraftNotedHighestNumber { .. }
        | Value::TaggedCount
        | Value::TurnHistoryCount(_)
        | Value::EventValue(_)
        | Value::EventValueOffset(_, _) => false,
        Value::Add(left, right) | Value::Min(left, right) => {
            value_could_be_affected_by(left, modification)
                || value_could_be_affected_by(right, modification)
        }
        Value::Scaled(value, _)
        | Value::DividedRoundedDown(value, _)
        | Value::HalfRoundedDown(value) => value_could_be_affected_by(value, modification),
        Value::SourcePower | Value::SourceToughness => pt_affected,
        Value::Count(filter)
        | Value::CountScaled(filter, _)
        | Value::GreatestCount(filter)
        | Value::GreatestSharedCreatureTypeCount(filter)
        | Value::GreatestSharedNameCount(filter)
        | Value::DistinctNames(filter)
        | Value::DistinctCounterTypesAmong(filter) => {
            modification_can_affect_filter(modification, filter)
                || modification_can_change_type_characteristics(modification)
                    && matches!(
                        value,
                        Value::GreatestSharedCreatureTypeCount(_)
                    )
                || matches!(modification, Modification::SetName(_) | Modification::InsertNameWords { .. })
                    && matches!(
                        value,
                        Value::GreatestSharedNameCount(_) | Value::DistinctNames(_)
                    )
        }
        Value::TotalPower(filter)
        | Value::TotalToughness(filter)
        | Value::GreatestPower(filter)
        | Value::GreatestToughness(filter)
        | Value::LeastPower(filter)
        | Value::LeastToughness(filter)
        | Value::DistinctPowers(filter) => {
            pt_affected || modification_can_affect_filter(modification, filter)
        }
        Value::TotalManaValue(filter)
        | Value::GreatestManaValue(filter)
        | Value::LeastManaValue(filter)
        | Value::DistinctManaValues(filter) => {
            matches!(modification.layer(), Layer::Copy)
                || modification_can_affect_filter(modification, filter)
        }
        Value::BasicLandTypesAmong(filter)
        | Value::CreatureTypesAmong(filter)
        | Value::CardTypesAmong(filter) => {
            modification_can_change_type_characteristics(modification)
                || modification_can_affect_filter(modification, filter)
        }
        Value::StaticAbilitiesAmong { .. } => {
            modification_can_change_abilities_or_matching_characteristics(modification)
        }
        Value::ColorsAmong(filter) | Value::ColorPairsAmong(filter) => {
            matches!(modification.layer(), Layer::Color | Layer::Copy)
                || modification_can_affect_filter(modification, filter)
        }
        Value::PowerOf(_) | Value::ToughnessOf(_) => pt_affected,
        Value::ManaValueOf(_) | Value::ManaSymbolsInManaCostOf { .. } => {
            matches!(modification.layer(), Layer::Copy)
        }
        Value::ColorsOf(_) => matches!(modification.layer(), Layer::Color | Layer::Copy),
        Value::Devotion { .. } | Value::DevotionToChosenColor(_) => {
            matches!(modification.layer(), Layer::Copy)
                || modification_can_change_type_characteristics(modification)
        }
        Value::PartySize(_) => modification_can_change_type_characteristics(modification),
        Value::NameStickerCharacterCountOnSource { .. } => {
            matches!(modification, Modification::SetName(_) | Modification::InsertNameWords { .. })
        }
        Value::SpellsCastThisTurnMatching { .. }
        | Value::TotalManaValueOfSpellsCastThisTurnMatching { .. }
        | Value::DamageDealtThisTurnByTaggedSpellCast(_)
        | Value::CardTypesInGraveyard(_)
        | Value::CommanderColorIdentityColors(_)
        | Value::PlayersWhoControlMoreThanYou { .. }
        | Value::PlayersWhoControlAtLeastMoreThanYou { .. } => {
            modification_can_change_abilities_or_matching_characteristics(modification)
        }
        Value::EffectValue(_)
        | Value::EffectValueOffset(_, _)
        | Value::EffectMetric { .. }
        | Value::EffectMetricOffset { .. }
        | Value::PendingEffectMetric { .. }
        | Value::PendingEffectMetricOffset { .. }
        | Value::PriorEffectMetric { .. }
        | Value::PendingComparisonLeft
        | Value::PendingComparisonRight
        | Value::PendingComparisonDifference
        | Value::PendingPriorEffectMetric(_) => {
            pt_affected || modification_can_change_abilities_or_matching_characteristics(modification)
        }
    }
}

fn modification_can_remove_static_ability_presence(modification: &Modification) -> bool {
    matches!(
        modification,
        Modification::CopyOf { .. }
            | Modification::SetTextBox(_)
            | Modification::SetSubtypes(_)
            | Modification::SetAbilities(_)
            | Modification::RemoveAbility(_)
            | Modification::RemoveStaticAbilityFamily(_)
            | Modification::RemoveAbilityGeneric { .. }
            | Modification::RemoveAllAbilities
            | Modification::RemoveAllAbilitiesExceptMana
    )
}

fn modification_can_affect_dependency_output(a: &Modification, b: &Modification) -> bool {
    match a {
        Modification::CopyActivatedAbilities { .. }
        | Modification::CopyStaticAbilityVariants { .. }
        | Modification::CopyTriggeredAbilities { .. } => {
            modification_can_change_abilities_or_matching_characteristics(b)
        }
        // A computed power or toughness may read characteristics the other
        // effect writes; only same-layer (layer 7) effects can reach here.
        Modification::SetPower { value, .. } | Modification::SetToughness { value, .. } => {
            value_could_be_affected_by(value, b)
        }
        Modification::SetPowerToughness {
            power, toughness, ..
        }
        | Modification::ModifyPowerToughnessValue { power, toughness } => {
            value_could_be_affected_by(power, b) || value_could_be_affected_by(toughness, b)
        }
        _ => false,
    }
}

fn modification_can_change_abilities_or_matching_characteristics(
    modification: &Modification,
) -> bool {
    matches!(
        modification,
        Modification::CopyOf { .. }
            | Modification::ChangeController(_)
            | Modification::SetTextBox(_)
            | Modification::SetName(_) | Modification::InsertNameWords { .. }
            | Modification::AddCardTypes(_)
            | Modification::RemoveCardTypes(_)
            | Modification::SetCardTypes(_)
            | Modification::AddSubtypes(_)
            | Modification::AddAllSubtypesOfFamily(_)
            | Modification::RemoveSubtypes(_)
            | Modification::RemoveAllSubtypesOfFamily(_)
            | Modification::SetSubtypes(_)
            | Modification::AddSupertypes(_)
            | Modification::RemoveSupertypes(_)
            | Modification::RemoveAllCreatureTypes
            | Modification::AddColors(_)
            | Modification::RemoveColors(_)
            | Modification::SetColors(_)
            | Modification::MakeColorless
            | Modification::AddAbility(_)
            | Modification::AddAbilityGeneric(_)
            | Modification::SetAbilities(_)
            | Modification::CopyActivatedAbilities { .. }
            | Modification::CopyStaticAbilityVariants { .. }
            | Modification::CopyTriggeredAbilities { .. }
            | Modification::AddCombatDamageDrawAbility
            | Modification::RemoveAbility(_)
            | Modification::RemoveStaticAbilityFamily(_)
            | Modification::RemoveAbilityGeneric { .. }
            | Modification::RemoveAllAbilities
            | Modification::RemoveAllAbilitiesExceptMana
    )
}

fn modification_can_affect_effect_target(
    modification: &Modification,
    target: &EffectTarget,
) -> bool {
    match target {
        // Attachment is a status, not a characteristic: no layered effect
        // changes what an aura or equipment is attached to.
        EffectTarget::Specific(_)
        | EffectTarget::Source
        | EffectTarget::AllPermanents
        | EffectTarget::AttachedTo(_) => false,
        EffectTarget::AllCreatures => modification_can_change_type_characteristics(modification),
        EffectTarget::Filter(filter) => modification_can_affect_filter(modification, filter),
    }
}

fn modification_can_affect_filter(modification: &Modification, filter: &ObjectFilter) -> bool {
    filter
        .any_of
        .iter()
        .any(|inner| modification_can_affect_filter(modification, inner))
        || filter
            .targets_object
            .as_deref()
            .is_some_and(|inner| modification_can_affect_filter(modification, inner))
        || filter
            .targets_only_object
            .as_deref()
            .is_some_and(|inner| modification_can_affect_filter(modification, inner))
        || filter
            .attached_to_object
            .as_deref()
            .is_some_and(|inner| modification_can_affect_filter(modification, inner))
        || match modification {
            Modification::CopyOf { .. } => filter.uses_non_pt_battlefield_characteristics(),
            Modification::ChangeController(_) => filter.controller.is_some(),
            Modification::ChangeText { .. } | Modification::SetTextBox(_) => {
                filter_uses_ability_characteristics(filter)
            }
            Modification::SetName(_) | Modification::InsertNameWords { .. } => {
                filter.name.is_some()
                    || filter.excluded_name.is_some()
                    || filter.name_originally_printed_in_set.is_some()
                    || filter.distinct_names
            }
            Modification::AddCardTypes(types) | Modification::RemoveCardTypes(types) => {
                filter_mentions_card_types(filter, types)
            }
            // Replacing card types also prunes subtypes that no longer have a
            // parent type, so every type-reading filter may change.
            Modification::SetCardTypes(_) => filter_uses_type_characteristics(filter),
            Modification::AddSubtypes(subtypes) | Modification::RemoveSubtypes(subtypes) => {
                filter_mentions_subtypes(filter, subtypes)
            }
            Modification::AddAllSubtypesOfFamily(family)
            | Modification::RemoveAllSubtypesOfFamily(family) => {
                filter_mentions_subtype_family(filter, *family)
            }
            Modification::RemoveAllCreatureTypes => {
                filter_mentions_subtype_family(filter, crate::types::SubtypeFamily::Creature)
            }
            // Setting subtypes replaces the same family (CR 205.1a); basic
            // land types also replace a land's rules text (CR 305.7), so
            // ability-reading filters change too.
            Modification::SetSubtypes(subtypes) => {
                crate::continuous::subtype_families_of(subtypes)
                    .into_iter()
                    .any(|family| filter_mentions_subtype_family(filter, family))
                    || (subtypes.iter().any(|subtype| subtype.is_basic_land_type())
                        && filter_uses_ability_characteristics(filter))
            }
            Modification::AddSupertypes(supertypes) | Modification::RemoveSupertypes(supertypes) => {
                filter_mentions_supertypes(filter, supertypes)
            }
            Modification::AddColors(_)
            | Modification::RemoveColors(_)
            | Modification::SetColors(_)
            | Modification::MakeColorless => filter_uses_color_characteristics(filter),
            Modification::AddAbility(_)
            | Modification::AddAbilityGeneric(_)
            | Modification::SetAbilities(_)
            | Modification::CopyActivatedAbilities { .. }
            | Modification::CopyStaticAbilityVariants { .. }
            | Modification::CopyTriggeredAbilities { .. }
            | Modification::AddCombatDamageDrawAbility
            | Modification::RemoveAbility(_)
            | Modification::RemoveStaticAbilityFamily(_)
            | Modification::RemoveAbilityGeneric { .. }
            | Modification::RemoveAllAbilities
            | Modification::RemoveAllAbilitiesExceptMana => {
                filter_uses_ability_characteristics(filter)
            }
            _ => false,
        }
}

fn filter_mentions_card_types(filter: &ObjectFilter, types: &[crate::types::CardType]) -> bool {
    filter.type_or_subtype_union
        || filter.one_per_card_type
        || types.iter().any(|card_type| {
            filter.card_types.contains(card_type)
                || filter.all_card_types.contains(card_type)
                || filter.excluded_card_types.contains(card_type)
                || ((filter.historic || filter.nonhistoric)
                    && *card_type == crate::types::CardType::Artifact)
        })
}

fn filter_mentions_subtypes(filter: &ObjectFilter, subtypes: &[crate::types::Subtype]) -> bool {
    filter.type_or_subtype_union
        || subtypes.iter().any(|subtype| {
            filter.subtypes.contains(subtype)
                || filter.excluded_subtypes.contains(subtype)
                || ((filter.historic || filter.nonhistoric)
                    && *subtype == crate::types::Subtype::Saga)
        })
}

fn filter_mentions_subtype_family(
    filter: &ObjectFilter,
    family: crate::types::SubtypeFamily,
) -> bool {
    filter.type_or_subtype_union
        || filter
            .subtypes
            .iter()
            .chain(filter.excluded_subtypes.iter())
            .any(|subtype| subtype.belongs_to_family(family))
        || ((filter.historic || filter.nonhistoric)
            && family == crate::types::SubtypeFamily::Enchantment)
}

fn filter_mentions_supertypes(
    filter: &ObjectFilter,
    supertypes: &[crate::types::Supertype],
) -> bool {
    supertypes.iter().any(|supertype| {
        filter.supertypes.contains(supertype)
            || filter.excluded_supertypes.contains(supertype)
            || ((filter.historic || filter.nonhistoric)
                && *supertype == crate::types::Supertype::Legendary)
    })
}

fn filter_uses_type_characteristics(filter: &ObjectFilter) -> bool {
    !filter.card_types.is_empty()
        || !filter.all_card_types.is_empty()
        || !filter.excluded_card_types.is_empty()
        || !filter.subtypes.is_empty()
        || filter.type_or_subtype_union
        || !filter.excluded_subtypes.is_empty()
        || !filter.supertypes.is_empty()
        || !filter.excluded_supertypes.is_empty()
        || filter.historic
        || filter.nonhistoric
        || filter.one_per_card_type
}

fn filter_uses_color_characteristics(filter: &ObjectFilter) -> bool {
    filter.colors.is_some()
        || filter.required_colors.is_some()
        || filter.chosen_color
        || !filter.excluded_colors.is_empty()
        || filter.colorless
        || filter.multicolored
        || filter.monocolored
        || filter.all_colors.is_some()
        || filter.exactly_two_colors.is_some()
        || filter.color_count.is_some()
}

fn filter_uses_ability_characteristics(filter: &ObjectFilter) -> bool {
    filter.has_tap_activated_ability
        || filter.no_abilities
        || !filter.static_abilities.is_empty()
        || !filter.excluded_static_abilities.is_empty()
        || !filter.ability_markers.is_empty()
        || !filter.excluded_ability_markers.is_empty()
        || filter.sticker.is_some()
}

fn modification_can_change_type_characteristics(modification: &Modification) -> bool {
    matches!(
        modification,
        Modification::CopyOf { .. }
            | Modification::AddCardTypes(_)
            | Modification::RemoveCardTypes(_)
            | Modification::SetCardTypes(_)
            | Modification::AddSubtypes(_)
            | Modification::AddAllSubtypesOfFamily(_)
            | Modification::RemoveSubtypes(_)
            | Modification::RemoveAllSubtypesOfFamily(_)
            | Modification::SetSubtypes(_)
            | Modification::AddSupertypes(_)
            | Modification::RemoveSupertypes(_)
            | Modification::RemoveAllCreatureTypes
    )
}

fn sublayer_group_has_trivial_ordering(effects: &[&ContinuousEffect], game: &GameState) -> bool {
    // P/T adders in 7c are commutative. Color-count modifiers also only read
    // layer-5 color characteristics, so other 7c modifiers cannot change their
    // output.
    if effects
        .iter()
        .all(|effect| is_commutative_pt_modifier(&effect.modification))
    {
        return true;
    }

    if sublayer_group_has_same_fixed_pt_setting(effects) {
        return true;
    }

    if attachment_scoped_effects_have_disjoint_scopes(effects, game) {
        return true;
    }

    // Characteristic-defining "count" effects that only use simple filter
    // counts are independent from each other.
    effects
        .iter()
        .all(|effect| is_independent_characteristic_defining_count_effect(effect))
}

fn is_commutative_pt_modifier(modification: &Modification) -> bool {
    match modification {
        Modification::ModifyPower(_)
        | Modification::ModifyToughness(_)
        | Modification::ModifyPowerToughness { .. }
        | Modification::ModifyPowerToughnessByColorCount { .. } => true,
        Modification::ModifyPowerToughnessValue { power, toughness } => {
            pt_modifier_value_is_commutative(power) && pt_modifier_value_is_commutative(toughness)
        }
        _ => false,
    }
}

fn pt_modifier_value_is_commutative(value: &Value) -> bool {
    match value {
        Value::SurfaceHinted { value, .. } => pt_modifier_value_is_commutative(value),
        Value::Fixed(_) => true,
        Value::Count(filter) | Value::CountScaled(filter, _) => {
            !filter.uses_power_or_toughness_characteristics()
        }
        _ => false,
    }
}

fn sublayer_group_has_same_fixed_pt_setting(effects: &[&ContinuousEffect]) -> bool {
    let mut fixed_setting: Option<(i32, i32)> = None;
    for effect in effects {
        let Modification::SetPowerToughness {
            power,
            toughness,
            sublayer,
        } = &effect.modification
        else {
            return false;
        };
        if *sublayer != PtSublayer::Setting {
            return false;
        }
        let (Value::Fixed(power), Value::Fixed(toughness)) = (power, toughness) else {
            return false;
        };
        let setting = (*power, *toughness);
        if let Some(existing) = fixed_setting {
            if existing != setting {
                return false;
            }
        } else {
            fixed_setting = Some(setting);
        }
    }

    fixed_setting.is_some()
}

fn is_independent_characteristic_defining_count_effect(effect: &ContinuousEffect) -> bool {
    if !matches!(effect.source_type, EffectSourceType::CharacteristicDefining) {
        return false;
    }

    if !matches!(
        effect.applies_to,
        EffectTarget::Specific(_) | EffectTarget::Source
    ) {
        return false;
    }

    match &effect.modification {
        Modification::SetPower { value, sublayer }
            if *sublayer == PtSublayer::CharacteristicDefining =>
        {
            value_is_independent_count_or_fixed(value)
        }
        Modification::SetToughness { value, sublayer }
            if *sublayer == PtSublayer::CharacteristicDefining =>
        {
            value_is_independent_count_or_fixed(value)
        }
        Modification::SetPowerToughness {
            power,
            toughness,
            sublayer,
        } if *sublayer == PtSublayer::CharacteristicDefining => {
            value_is_independent_count_or_fixed(power)
                && value_is_independent_count_or_fixed(toughness)
        }
        _ => false,
    }
}

fn value_is_independent_count_or_fixed(value: &Value) -> bool {
    match value {
        Value::Fixed(_) => true,
        Value::Count(filter) | Value::CountScaled(filter, _) => {
            filter_has_no_pt_constraints_for_fast_path(filter)
        }
        Value::Add(left, right) => {
            value_is_independent_count_or_fixed(left) && value_is_independent_count_or_fixed(right)
        }
        Value::Min(left, right) => {
            value_is_independent_count_or_fixed(left) && value_is_independent_count_or_fixed(right)
        }
        Value::DividedRoundedDown(value, _) | Value::HalfRoundedDown(value) => {
            value_is_independent_count_or_fixed(value)
        }
        _ => false,
    }
}

fn filter_has_no_pt_constraints_for_fast_path(filter: &ObjectFilter) -> bool {
    filter.power.is_none()
        && filter.toughness.is_none()
        && filter.power_relative_to_source.is_none()
        && filter.any_of.is_empty()
}

/// Groups that already started applying in a layer below `layer`, judged
/// board-wide against `baseline` (CR 613.6). The dependency sort must see one
/// answer per group for the whole layer, not the per-object answer the layer
/// driver keeps for application: otherwise two objects could compute the same
/// layer in different orders.
pub fn started_groups_for_sort<'a>(
    effects: impl IntoIterator<Item = &'a ContinuousEffect>,
    layer: Layer,
    baseline: &HashMap<ObjectId, CalculatedCharacteristics>,
    objects: &ObjectMap,
    game: &GameState,
) -> HashSet<ContinuousEffectGroupId> {
    let mut started = HashSet::new();
    for effect in effects {
        let Some(group) = effect.group else {
            continue;
        };
        if effect.modification.layer() >= layer || started.contains(&group) {
            continue;
        }
        if crate::continuous::continuous_effect_duration_and_condition_are_active(effect, game)
            && effect_applies_to_any_object(effect, baseline, objects, game)
        {
            started.insert(group);
        }
    }
    started
}

pub fn sort_layer_effects_with_baseline<'a>(
    effects: &[&'a ContinuousEffect],
    baseline: &HashMap<ObjectId, CalculatedCharacteristics>,
    objects: &ObjectMap,
    game: &GameState,
) -> Vec<&'a ContinuousEffect> {
    sort_layer_effects_with_baseline_and_started_groups(
        effects,
        baseline,
        objects,
        game,
        &HashSet::new(),
    )
}

pub fn sort_layer_effects_with_baseline_and_started_groups<'a>(
    effects: &[&'a ContinuousEffect],
    baseline: &HashMap<ObjectId, CalculatedCharacteristics>,
    objects: &ObjectMap,
    game: &GameState,
    started_groups: &HashSet<ContinuousEffectGroupId>,
) -> Vec<&'a ContinuousEffect> {
    if effects.is_empty() {
        return Vec::new();
    }

    let layer = effects[0].modification.layer();

    if layer == Layer::PowerToughness {
        // Group by sublayer
        let mut by_sublayer: HashMap<Option<PtSublayer>, Vec<&ContinuousEffect>> =
            HashMap::with_capacity(4);
        for &effect in effects {
            let sublayer = effect.modification.pt_sublayer();
            by_sublayer.entry(sublayer).or_default().push(effect);
        }

        let mut sublayers: Vec<_> = by_sublayer.keys().cloned().collect();
        sublayers.sort();

        // Each sublayer is sorted against the characteristics as they stand
        // after the earlier sublayers (CR 613.4): a 7b or 7c probe that reads
        // power or toughness must see the 7a and 7b results, not the values
        // from before layer 7.
        let mut current_baseline = baseline.clone();
        let mut current_started_groups = started_groups.clone();
        let mut result = Vec::with_capacity(effects.len());
        for (position, sublayer) in sublayers.iter().enumerate() {
            let sublayer_effects = &by_sublayer[sublayer];
            let sorted = sort_with_dependencies_with_baseline_and_started_groups(
                sublayer_effects,
                &current_baseline,
                objects,
                game,
                &current_started_groups,
            );
            if position + 1 < sublayers.len() {
                for effect in &sorted {
                    if let Some(group) = effect.group
                        && crate::continuous::continuous_effect_duration_and_condition_are_active(
                            effect, game,
                        )
                        && effect_applies_to_any_object(effect, &current_baseline, objects, game)
                    {
                        current_started_groups.insert(group);
                    }
                    current_baseline =
                        apply_effect_to_baseline(effect, &current_baseline, objects, game);
                }
            }
            result.extend(sorted);
        }

        result
    } else {
        sort_with_dependencies_with_baseline_and_started_groups(
            effects,
            baseline,
            objects,
            game,
            started_groups,
        )
    }
}

fn sort_with_dependencies_with_baseline_and_started_groups<'a>(
    effects: &[&'a ContinuousEffect],
    baseline: &HashMap<ObjectId, CalculatedCharacteristics>,
    objects: &ObjectMap,
    game: &GameState,
    started_groups: &HashSet<ContinuousEffectGroupId>,
) -> Vec<&'a ContinuousEffect> {
    if effects.len() <= 1 {
        return effects.to_vec();
    }
    game.note_dependency_sort();

    // CR 613.8c requires dependency information to be reevaluated after each
    // effect is applied. A previously independent effect can become dependent
    // because an earlier effect changed which objects it applies to.
    let mut remaining: Vec<usize> = (0..effects.len()).collect();
    let mut current_baseline = baseline.clone();
    let mut current_started_groups = started_groups.clone();
    let mut result = Vec::with_capacity(effects.len());

    while !remaining.is_empty() {
        let representatives = chars_class_representatives(&current_baseline, objects, game);
        let cda_pending = remaining.iter().any(|&index| {
            matches!(
                effects[index].source_type,
                EffectSourceType::CharacteristicDefining
            )
        });
        let eligible: Vec<usize> = remaining
            .iter()
            .copied()
            .filter(|&index| {
                !cda_pending
                    || matches!(
                        effects[index].source_type,
                        EffectSourceType::CharacteristicDefining
                    )
            })
            .collect();

        // depends_on[i] lists the positions (within `eligible`) that
        // eligible[i] must wait for.
        let depends_on: Vec<Vec<usize>> = eligible
            .iter()
            .map(|&index| {
                eligible
                    .iter()
                    .enumerate()
                    .filter(|&(_, &dependency_index)| {
                        dependency_index != index
                            && effect_depends_on_with_baseline_and_started_groups(
                                effects[index],
                                effects[dependency_index],
                                &current_baseline,
                                objects,
                                game,
                                &current_started_groups,
                                Some(&representatives),
                            )
                    })
                    .map(|(position, _)| position)
                    .collect()
            })
            .collect();

        let mut ready: Vec<usize> = (0..eligible.len())
            .filter(|&position| depends_on[position].is_empty())
            .map(|position| eligible[position])
            .collect();

        // CR 613.8a: effects that form a dependency loop ignore the
        // dependencies inside the loop and apply in timestamp order. Only the
        // loop members become candidates; an effect that merely depends on a
        // loop member keeps waiting for it. A loop that itself waits on another
        // loop is not a candidate either.
        if ready.is_empty() {
            ready = dependency_loop_candidates(&depends_on)
                .into_iter()
                .map(|position| eligible[position])
                .collect();
        }
        let next = ready
            .into_iter()
            .min_by_key(|&index| (effects[index].timestamp, index))
            .expect("at least one remaining effect must be eligible");
        let effect = effects[next];

        let starts_group = effect.group.is_some()
            && crate::continuous::continuous_effect_duration_and_condition_are_active(effect, game)
            && effect_applies_to_any_object(effect, &current_baseline, objects, game);
        current_baseline = apply_effect_to_baseline(effect, &current_baseline, objects, game);
        if starts_group && let Some(group) = effect.group {
            current_started_groups.insert(group);
        }
        result.push(effect);
        remaining.retain(|&index| index != next);
    }

    result
}

/// Positions in a dependency graph that belong to a strongly connected
/// component with no dependency on any other component. When nothing is
/// ready, these are exactly the members of the dependency loops that CR
/// 613.8a resolves by timestamp order.
fn dependency_loop_candidates(depends_on: &[Vec<usize>]) -> Vec<usize> {
    let component_of = strongly_connected_components(depends_on);
    let component_count = component_of.iter().copied().max().map_or(0, |max| max + 1);
    let mut waits_on_other_component = vec![false; component_count];
    for (node, dependencies) in depends_on.iter().enumerate() {
        for &dependency in dependencies {
            if component_of[dependency] != component_of[node] {
                waits_on_other_component[component_of[node]] = true;
            }
        }
    }
    (0..depends_on.len())
        .filter(|&node| !waits_on_other_component[component_of[node]])
        .collect()
}

/// Tarjan's algorithm over `depends_on`; returns each node's component id.
fn strongly_connected_components(depends_on: &[Vec<usize>]) -> Vec<usize> {
    struct State<'a> {
        depends_on: &'a [Vec<usize>],
        index: Vec<Option<usize>>,
        lowlink: Vec<usize>,
        on_stack: Vec<bool>,
        stack: Vec<usize>,
        component_of: Vec<usize>,
        next_index: usize,
        next_component: usize,
    }

    fn visit(state: &mut State<'_>, node: usize) {
        state.index[node] = Some(state.next_index);
        state.lowlink[node] = state.next_index;
        state.next_index += 1;
        state.stack.push(node);
        state.on_stack[node] = true;

        for position in 0..state.depends_on[node].len() {
            let next = state.depends_on[node][position];
            match state.index[next] {
                None => {
                    visit(state, next);
                    state.lowlink[node] = state.lowlink[node].min(state.lowlink[next]);
                }
                Some(next_index) if state.on_stack[next] => {
                    state.lowlink[node] = state.lowlink[node].min(next_index);
                }
                Some(_) => {}
            }
        }

        if state.index[node] == Some(state.lowlink[node]) {
            while let Some(member) = state.stack.pop() {
                state.on_stack[member] = false;
                state.component_of[member] = state.next_component;
                if member == node {
                    break;
                }
            }
            state.next_component += 1;
        }
    }

    let n = depends_on.len();
    let mut state = State {
        depends_on,
        index: vec![None; n],
        lowlink: vec![0; n],
        on_stack: vec![false; n],
        stack: Vec::with_capacity(n),
        component_of: vec![0; n],
        next_index: 0,
        next_component: 0,
    };
    for node in 0..n {
        if state.index[node].is_none() {
            visit(&mut state, node);
        }
    }
    state.component_of
}

/// Sort effects within a single layer, considering both sublayers and dependencies.
///
/// This is the main entry point for sorting Layer 7 effects which have sublayers.
pub fn sort_layer_effects<'a>(effects: &[&'a ContinuousEffect]) -> Vec<&'a ContinuousEffect> {
    if effects.is_empty() {
        return Vec::new();
    }

    let layer = effects[0].modification.layer();

    if layer == Layer::PowerToughness {
        // Group by sublayer
        let mut by_sublayer: HashMap<Option<PtSublayer>, Vec<&ContinuousEffect>> = HashMap::new();
        for &effect in effects {
            let sublayer = effect.modification.pt_sublayer();
            by_sublayer.entry(sublayer).or_default().push(effect);
        }

        // Sort sublayers
        let mut sublayers: Vec<_> = by_sublayer.keys().cloned().collect();
        sublayers.sort();

        // Process each sublayer
        let mut result = Vec::new();
        for sublayer in sublayers {
            let sublayer_effects = &by_sublayer[&sublayer];
            let sorted = sort_with_dependencies(sublayer_effects);
            result.extend(sorted);
        }

        result
    } else {
        // Non-Layer 7: just sort by dependencies
        sort_with_dependencies(effects)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::continuous::{EffectTarget, Modification};
    use crate::cost::TotalCost;
    use crate::effect::Effect;
    use crate::ids::PlayerId;
    use crate::mana::ManaSymbol;
    use crate::static_abilities::StaticAbility;
    use crate::target::ObjectFilter;
    use crate::types::{CardType, Subtype};
    use std::sync::Arc;

    fn create_test_effect(id: u64, timestamp: u64, modification: Modification) -> ContinuousEffect {
        ContinuousEffect {
            id: crate::continuous::ContinuousEffectId::new(id),
            source: ObjectId::from_raw(id),
            controller: PlayerId::from_index(0),
            applies_to: EffectTarget::AllPermanents,
            modification,
            timestamp,
            group: None,
            duration: crate::effect::Until::Forever,
            expires_end_of_turn: u32::MAX,
            condition: None,
            source_type: EffectSourceType::StaticAbility,
            originating_static_ability: None,
        }
    }

    #[test]
    fn test_timestamp_sorting_without_dependencies() {
        let e1 = create_test_effect(
            1,
            100,
            Modification::ModifyPowerToughness {
                power: 1,
                toughness: 1,
            },
        );
        let e2 = create_test_effect(
            2,
            50,
            Modification::ModifyPowerToughness {
                power: 2,
                toughness: 2,
            },
        );
        let e3 = create_test_effect(
            3,
            75,
            Modification::ModifyPowerToughness {
                power: 3,
                toughness: 3,
            },
        );

        let effects: Vec<&ContinuousEffect> = vec![&e1, &e2, &e3];
        let sorted = sort_with_dependencies(&effects);

        // Should be sorted by timestamp (oldest first)
        assert_eq!(sorted[0].id.0, 2); // timestamp 50
        assert_eq!(sorted[1].id.0, 3); // timestamp 75
        assert_eq!(sorted[2].id.0, 1); // timestamp 100
    }

    #[test]
    fn test_dependency_ordering() {
        // Gain/remove ability effects without a source/applicability change
        // apply in timestamp order.
        let anthem = create_test_effect(
            1,
            100, // Newer timestamp
            Modification::AddAbility(StaticAbility::flying()),
        );
        let humility = create_test_effect(
            2,
            50, // Older timestamp
            Modification::RemoveAllAbilities,
        );

        let effects: Vec<&ContinuousEffect> = vec![&anthem, &humility];
        let sorted = sort_with_dependencies(&effects);

        assert_eq!(sorted[0].id.0, 2); // humility, older timestamp
        assert_eq!(sorted[1].id.0, 1); // anthem, newer timestamp
    }

    #[test]
    fn test_filter_applicability_dependency_in_type_layer() {
        use crate::card::CardBuilder;
        use crate::ids::CardId;
        use crate::object::Object;
        use crate::zone::Zone;
        use std::collections::HashMap;

        let a = ContinuousEffect {
            id: crate::continuous::ContinuousEffectId::new(1),
            source: ObjectId::from_raw(1),
            controller: PlayerId::from_index(0),
            applies_to: EffectTarget::Filter(ObjectFilter::creature()),
            modification: Modification::SetSubtypes(vec![Subtype::Goblin]),
            timestamp: 50,
            group: None,
            duration: crate::effect::Until::Forever,
            expires_end_of_turn: u32::MAX,
            condition: None,
            source_type: EffectSourceType::StaticAbility,
            originating_static_ability: None,
        };

        let b = ContinuousEffect {
            id: crate::continuous::ContinuousEffectId::new(2),
            source: ObjectId::from_raw(2),
            controller: PlayerId::from_index(0),
            applies_to: EffectTarget::AllPermanents,
            modification: Modification::AddCardTypes(vec![CardType::Creature]),
            timestamp: 100,
            group: None,
            duration: crate::effect::Until::Forever,
            expires_end_of_turn: u32::MAX,
            condition: None,
            source_type: EffectSourceType::StaticAbility,
            originating_static_ability: None,
        };

        let card = CardBuilder::new(CardId(1), "Test Artifact")
            .card_types(vec![CardType::Artifact])
            .build();
        let object = Object::from_card(
            ObjectId::from_raw(10),
            &card,
            PlayerId::from_index(0),
            Zone::Battlefield,
        );
        let objects = [(object.id, Arc::new(object.clone()))]
            .into_iter()
            .collect::<crate::game_state::ObjectMap>();
        let baseline = HashMap::from([(
            object.id,
            CalculatedCharacteristics {
                name: object.name.clone(),
                mana_cost: object.mana_cost_owned(),
                compiled_card_text: object.compiled_card_text.clone(),
                ability_labels: object.ability_labels.clone(),
                power: object.base_power.as_ref().map(|p| p.base_value()),
                toughness: object.base_toughness.as_ref().map(|t| t.base_value()),
                card_types: object.card_types.clone(),
                subtypes: object.subtypes.clone(),
                supertypes: object.supertypes.clone(),
                world_supertype_since: None,
                colors: object.colors(),
                loyalty: object.base_loyalty,
                abilities: object.abilities.clone().into(),
                static_abilities: Vec::new().into(),
                ability_gain_prohibitions: Vec::new(),
                aura_attach_filter: object.aura_attach_filter_owned(),
                controller: object.owner,
            },
        )]);
        let game = GameState::new(vec!["Alice".to_string()], 20);

        assert!(effect_depends_on_with_baseline_and_started_groups(
            &a,
            &b,
            &baseline,
            &objects,
            &game,
            &HashSet::new(),
            None,
        ));
    }

    #[test]
    fn test_copy_activated_abilities_depends_on_granted_ability() {
        use crate::ability::Ability;
        use crate::card::CardBuilder;
        use crate::ids::CardId;
        use crate::object::Object;
        use crate::zone::Zone;
        use std::collections::HashMap;

        let land_card = CardBuilder::new(CardId(10), "Test Land")
            .card_types(vec![CardType::Land])
            .build();
        let land = Object::from_card(
            ObjectId::from_raw(10),
            &land_card,
            PlayerId::from_index(0),
            Zone::Battlefield,
        );

        let objects = [(land.id, Arc::new(land.clone()))]
            .into_iter()
            .collect::<crate::game_state::ObjectMap>();
        let baseline = HashMap::from([(
            land.id,
            CalculatedCharacteristics {
                name: land.name.clone(),
                mana_cost: land.mana_cost_owned(),
                compiled_card_text: land.compiled_card_text.clone(),
                ability_labels: land.ability_labels.clone(),
                power: land.base_power.as_ref().map(|p| p.base_value()),
                toughness: land.base_toughness.as_ref().map(|t| t.base_value()),
                card_types: land.card_types.clone(),
                subtypes: land.subtypes.clone(),
                supertypes: land.supertypes.clone(),
                world_supertype_since: None,
                colors: land.colors(),
                loyalty: land.base_loyalty,
                abilities: land.abilities.clone().into(),
                static_abilities: Vec::new().into(),
                ability_gain_prohibitions: Vec::new(),
                aura_attach_filter: land.aura_attach_filter_owned(),
                controller: land.owner,
            },
        )]);
        let game = GameState::new(vec!["Alice".to_string()], 20);

        let copy_effect = ContinuousEffect {
            id: crate::continuous::ContinuousEffectId::new(1),
            source: ObjectId::from_raw(1),
            controller: PlayerId::from_index(0),
            applies_to: EffectTarget::Source,
            modification: Modification::CopyActivatedAbilities {
                filter: ObjectFilter::land(),
                counter: None,
                include_mana: true,
                only_loyalty: false,
                exclude_source_name: false,
                exclude_source_id: false,
                force_once_each_turn: false,
            },
            timestamp: 1,
            group: None,
            duration: crate::effect::Until::Forever,
            expires_end_of_turn: u32::MAX,
            condition: None,
            source_type: EffectSourceType::StaticAbility,
            originating_static_ability: None,
        };

        let granted_ability = Ability::activated(
            TotalCost::mana(crate::mana::ManaCost::from_pips(vec![vec![
                ManaSymbol::Generic(1),
            ]])),
            vec![Effect::gain_life(1)],
        );
        let grant_effect = ContinuousEffect {
            id: crate::continuous::ContinuousEffectId::new(2),
            source: ObjectId::from_raw(2),
            controller: PlayerId::from_index(0),
            applies_to: EffectTarget::Specific(land.id),
            modification: Modification::AddAbilityGeneric(granted_ability),
            timestamp: 2,
            group: None,
            duration: crate::effect::Until::Forever,
            expires_end_of_turn: u32::MAX,
            condition: None,
            source_type: EffectSourceType::StaticAbility,
            originating_static_ability: None,
        };

        assert!(effect_depends_on_with_baseline_and_started_groups(
            &copy_effect,
            &grant_effect,
            &baseline,
            &objects,
            &game,
            &HashSet::new(),
            None,
        ));
    }

    #[test]
    fn test_copy_activated_abilities_depends_on_remove_all() {
        use crate::ability::{Ability, AbilityKind, ActivatedAbility, ActivationTiming};
        use crate::card::CardBuilder;
        use crate::ids::CardId;
        use crate::object::Object;
        use crate::zone::Zone;
        use std::collections::HashMap;

        let land_card = CardBuilder::new(CardId(11), "Test Land")
            .card_types(vec![CardType::Land])
            .build();
        let mut land = Object::from_card(
            ObjectId::from_raw(11),
            &land_card,
            PlayerId::from_index(0),
            Zone::Battlefield,
        );
        land.abilities_mut().push(Ability {
            kind: AbilityKind::Activated(ActivatedAbility {
                mana_cost: TotalCost::free(),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    Effect::gain_life(1),
                ]),
                choices: vec![],
                timing: ActivationTiming::AnyTime,
                additional_restrictions: vec![],
                activation_restrictions: vec![],
                mana_output: None,
                activation_condition: None,
                mana_usage_restrictions: vec![],
                is_loyalty_ability: false,
            }),
            functional_zones: vec![Zone::Battlefield],
        });

        let objects = [(land.id, Arc::new(land.clone()))]
            .into_iter()
            .collect::<crate::game_state::ObjectMap>();
        let baseline = HashMap::from([(
            land.id,
            CalculatedCharacteristics {
                name: land.name.clone(),
                mana_cost: land.mana_cost_owned(),
                compiled_card_text: land.compiled_card_text.clone(),
                ability_labels: land.ability_labels.clone(),
                power: land.base_power.as_ref().map(|p| p.base_value()),
                toughness: land.base_toughness.as_ref().map(|t| t.base_value()),
                card_types: land.card_types.clone(),
                subtypes: land.subtypes.clone(),
                supertypes: land.supertypes.clone(),
                world_supertype_since: None,
                colors: land.colors(),
                loyalty: land.base_loyalty,
                abilities: land.abilities.clone().into(),
                static_abilities: Vec::new().into(),
                ability_gain_prohibitions: Vec::new(),
                aura_attach_filter: land.aura_attach_filter_owned(),
                controller: land.owner,
            },
        )]);
        let game = GameState::new(vec!["Alice".to_string()], 20);

        let copy_effect = ContinuousEffect {
            id: crate::continuous::ContinuousEffectId::new(3),
            source: ObjectId::from_raw(3),
            controller: PlayerId::from_index(0),
            applies_to: EffectTarget::Source,
            modification: Modification::CopyActivatedAbilities {
                filter: ObjectFilter::land(),
                counter: None,
                include_mana: true,
                only_loyalty: false,
                exclude_source_name: false,
                exclude_source_id: false,
                force_once_each_turn: false,
            },
            timestamp: 1,
            group: None,
            duration: crate::effect::Until::Forever,
            expires_end_of_turn: u32::MAX,
            condition: None,
            source_type: EffectSourceType::StaticAbility,
            originating_static_ability: None,
        };

        let remove_effect = ContinuousEffect {
            id: crate::continuous::ContinuousEffectId::new(4),
            source: ObjectId::from_raw(4),
            controller: PlayerId::from_index(0),
            applies_to: EffectTarget::Specific(land.id),
            modification: Modification::RemoveAllAbilities,
            timestamp: 2,
            group: None,
            duration: crate::effect::Until::Forever,
            expires_end_of_turn: u32::MAX,
            condition: None,
            source_type: EffectSourceType::StaticAbility,
            originating_static_ability: None,
        };

        assert!(effect_depends_on_with_baseline_and_started_groups(
            &copy_effect,
            &remove_effect,
            &baseline,
            &objects,
            &game,
            &HashSet::new(),
            None,
        ));
    }

    #[test]
    fn test_topo_ready_queue_uses_oldest_timestamp_first() {
        // With no dependencies, the same ready-queue machinery should keep
        // timestamp order.
        let e1 = create_test_effect(1, 5, Modification::AddAbility(StaticAbility::flying()));
        let e2 = create_test_effect(2, 10, Modification::AddAbility(StaticAbility::haste()));
        let e3 = create_test_effect(3, 20, Modification::RemoveAllAbilities);

        let effects: Vec<&ContinuousEffect> = vec![&e1, &e2, &e3];
        let sorted = sort_with_dependencies(&effects);

        assert_eq!(sorted[0].id.0, 1);
        assert_eq!(sorted[1].id.0, 2);
        assert_eq!(sorted[2].id.0, 3);
    }

    #[test]
    fn test_value_references_pt() {
        use crate::effect::Value;
        use crate::target::ChooseSpec;

        // Values that reference P/T
        assert!(value_references_pt(&Value::SourcePower));
        assert!(value_references_pt(&Value::SourceToughness));
        assert!(value_references_pt(&Value::PowerOf(Box::new(
            ChooseSpec::Source
        ))));
        assert!(value_references_pt(&Value::ToughnessOf(Box::new(
            ChooseSpec::Source
        ))));
        assert!(value_references_pt(&Value::EffectValue(
            crate::effect::EffectId(1)
        )));

        // Values that don't reference P/T
        assert!(!value_references_pt(&Value::Fixed(5)));
        assert!(!value_references_pt(&Value::X));
        assert!(!value_references_pt(&Value::XTimes(2)));
        assert!(!value_references_pt(&Value::CardsInHand(
            crate::target::PlayerFilter::You
        )));
        assert!(!value_references_pt(&Value::WasKicked));
    }

    #[test]
    fn test_needs_baseline_dependency_sort_skips_independent_cda_counts() {
        use crate::effect::Value;

        let mut cda_a = create_test_effect(
            1,
            10,
            Modification::SetPowerToughness {
                power: Value::Count(ObjectFilter::creature().you_control()),
                toughness: Value::Count(ObjectFilter::creature().you_control()),
                sublayer: PtSublayer::CharacteristicDefining,
            },
        );
        cda_a.applies_to = EffectTarget::Specific(cda_a.source);
        cda_a.source_type = EffectSourceType::CharacteristicDefining;

        let mut cda_b = create_test_effect(
            2,
            20,
            Modification::SetPowerToughness {
                power: Value::Count(ObjectFilter::creature().you_control()),
                toughness: Value::Count(ObjectFilter::creature().you_control()),
                sublayer: PtSublayer::CharacteristicDefining,
            },
        );
        cda_b.applies_to = EffectTarget::Specific(cda_b.source);
        cda_b.source_type = EffectSourceType::CharacteristicDefining;

        let effects = vec![&cda_a, &cda_b];
        assert!(!needs_baseline_dependency_sort(
            &effects,
            &GameState::new(vec!["Test".to_string()], 20)
        ));
    }

    #[test]
    fn baseline_sort_rechecks_dependencies_after_each_effect() {
        use crate::card::CardBuilder;
        use crate::ids::CardId;
        use crate::object::Object;
        use crate::zone::Zone;
        use std::collections::HashMap;

        let card = CardBuilder::new(CardId::from_raw(40), "Dependency Seed")
            .card_types(vec![CardType::Artifact])
            .build();
        let object = Object::from_card(
            ObjectId::from_raw(40),
            &card,
            PlayerId::from_index(0),
            Zone::Battlefield,
        );
        let objects = [(object.id, Arc::new(object.clone()))]
            .into_iter()
            .collect::<crate::game_state::ObjectMap>();
        let baseline = HashMap::from([(
            object.id,
            CalculatedCharacteristics {
                name: object.name.clone(),
                mana_cost: object.mana_cost_owned(),
                compiled_card_text: object.compiled_card_text.clone(),
                ability_labels: object.ability_labels.clone(),
                power: object.base_power.as_ref().map(|power| power.base_value()),
                toughness: object
                    .base_toughness
                    .as_ref()
                    .map(|toughness| toughness.base_value()),
                card_types: object.card_types.clone(),
                subtypes: object.subtypes.clone(),
                supertypes: object.supertypes.clone(),
                world_supertype_since: None,
                colors: object.colors(),
                loyalty: object.base_loyalty,
                abilities: object.abilities.clone().into(),
                static_abilities: Vec::new().into(),
                ability_gain_prohibitions: Vec::new(),
                aura_attach_filter: object.aura_attach_filter_owned(),
                controller: object.owner,
            },
        )]);
        let game = GameState::new(vec!["Alice".to_string()], 20);

        let mut artifact_to_creature =
            create_test_effect(1, 1, Modification::AddCardTypes(vec![CardType::Creature]));
        artifact_to_creature.applies_to = EffectTarget::Filter(ObjectFilter::artifact());
        let mut land_to_enchantment = create_test_effect(
            2,
            2,
            Modification::AddCardTypes(vec![CardType::Enchantment]),
        );
        land_to_enchantment.applies_to = EffectTarget::Filter(ObjectFilter::land());
        let mut creature_to_land =
            create_test_effect(3, 3, Modification::AddCardTypes(vec![CardType::Land]));
        creature_to_land.applies_to = EffectTarget::Filter(ObjectFilter::creature());

        let sorted = sort_layer_effects_with_baseline(
            &[
                &artifact_to_creature,
                &land_to_enchantment,
                &creature_to_land,
            ],
            &baseline,
            &objects,
            &game,
        );
        let ids: Vec<_> = sorted.iter().map(|effect| effect.id).collect();
        assert_eq!(
            ids,
            vec![
                artifact_to_creature.id,
                creature_to_land.id,
                land_to_enchantment.id,
            ]
        );
    }

    #[test]
    fn test_needs_baseline_dependency_sort_for_dynamic_cda_non_count_values() {
        use crate::effect::Value;

        let mut cda_source_power = create_test_effect(
            1,
            10,
            Modification::SetPowerToughness {
                power: Value::SourcePower,
                toughness: Value::Fixed(1),
                sublayer: PtSublayer::CharacteristicDefining,
            },
        );
        cda_source_power.applies_to = EffectTarget::Specific(cda_source_power.source);
        cda_source_power.source_type = EffectSourceType::CharacteristicDefining;

        let mut cda_fixed = create_test_effect(
            2,
            20,
            Modification::SetPowerToughness {
                power: Value::Fixed(2),
                toughness: Value::Fixed(2),
                sublayer: PtSublayer::CharacteristicDefining,
            },
        );
        cda_fixed.applies_to = EffectTarget::Specific(cda_fixed.source);
        cda_fixed.source_type = EffectSourceType::CharacteristicDefining;

        let effects = vec![&cda_source_power, &cda_fixed];
        assert!(needs_baseline_dependency_sort(
            &effects,
            &GameState::new(vec!["Test".to_string()], 20)
        ));
    }

    #[test]
    fn test_needs_baseline_dependency_sort_skips_when_each_pt_sublayer_has_one_effect() {
        use crate::effect::Value;

        let cda = create_test_effect(
            1,
            10,
            Modification::SetPowerToughness {
                power: Value::Fixed(2),
                toughness: Value::Fixed(2),
                sublayer: PtSublayer::CharacteristicDefining,
            },
        );
        let modifier = create_test_effect(
            2,
            20,
            Modification::ModifyPowerToughness {
                power: 1,
                toughness: 1,
            },
        );

        let effects = vec![&cda, &modifier];
        assert!(!needs_baseline_dependency_sort(
            &effects,
            &GameState::new(vec!["Test".to_string()], 20)
        ));
    }

    #[test]
    fn test_needs_baseline_dependency_sort_skips_isolated_resolution_text_overlays() {
        let mut first = create_test_effect(
            1,
            10,
            Modification::SetTextBox(crate::continuous::TextBoxOverlay::new(
                "Flying".to_string(),
                Vec::new(),
            )),
        );
        first.applies_to = EffectTarget::Specific(ObjectId::from_raw(101));
        first.source_type = EffectSourceType::Resolution {
            locked_targets: vec![ObjectId::from_raw(101)],
        };

        let mut second = create_test_effect(
            2,
            20,
            Modification::SetTextBox(crate::continuous::TextBoxOverlay::new(
                "Deathtouch".to_string(),
                Vec::new(),
            )),
        );
        second.applies_to = EffectTarget::Specific(ObjectId::from_raw(202));
        second.source_type = EffectSourceType::Resolution {
            locked_targets: vec![ObjectId::from_raw(202)],
        };

        let effects = vec![&first, &second];
        assert!(!needs_baseline_dependency_sort(
            &effects,
            &GameState::new(vec!["Test".to_string()], 20)
        ));
    }

    #[test]
    fn delirium_independent_ability_grants_do_not_require_a_baseline() {
        let game = GameState::new(vec!["Test".to_string()], 20);
        let condition = crate::ConditionExpr::PlayerHasCardTypesInGraveyardOrMore {
            player: crate::target::PlayerFilter::You,
            count: 4,
        };
        let mut flying =
            create_test_effect(1, 10, Modification::AddAbility(StaticAbility::flying()))
                .with_condition(condition.clone());
        let must_attack = create_test_effect(
            2,
            20,
            Modification::AddAbility(StaticAbility::must_attack()),
        )
        .with_condition(condition.clone());
        assert!(!needs_baseline_dependency_sort(
            &[&flying, &must_attack],
            &game
        ));

        // Boolean composition is safe only if every leaf is invariant.
        flying.condition = Some(crate::ConditionExpr::And(
            Box::new(condition.clone()),
            Box::new(crate::ConditionExpr::Not(Box::new(condition))),
        ));
        assert!(!needs_baseline_dependency_sort(
            &[&flying, &must_attack],
            &game
        ));
        // A creature-count condition cannot be changed by ability grants, so
        // the group still proves trivial. A condition that reads abilities can.
        flying.condition = Some(crate::ConditionExpr::YouControl(ObjectFilter::creature()));
        assert!(!needs_baseline_dependency_sort(
            &[&flying, &must_attack],
            &game
        ));
        let mut flyers = ObjectFilter::creature();
        flyers
            .static_abilities
            .push(crate::static_abilities::StaticAbilityId::Flying);
        flying.condition = Some(crate::ConditionExpr::YouControl(flyers));
        assert!(needs_baseline_dependency_sort(
            &[&flying, &must_attack],
            &game
        ));

        // Layer 4 can change the counted types; it must retain simulation.
        let first_type =
            create_test_effect(3, 30, Modification::AddCardTypes(vec![CardType::Artifact]))
                .with_condition(must_attack.condition.clone().unwrap());
        let second_type =
            create_test_effect(4, 40, Modification::AddCardTypes(vec![CardType::Creature]));
        assert!(needs_baseline_dependency_sort(
            &[&first_type, &second_type],
            &game
        ));

        // Removing the originating static ability remains a dependency.
        let grant = must_attack.with_originating_static_ability(StaticAbility::flying());
        let removal = create_test_effect(5, 50, Modification::RemoveAllAbilities);
        assert!(needs_baseline_dependency_sort(&[&grant, &removal], &game));
    }

    fn chars_for(object: &crate::object::Object) -> CalculatedCharacteristics {
        CalculatedCharacteristics {
            name: object.name.clone(),
            mana_cost: object.mana_cost_owned(),
            compiled_card_text: object.compiled_card_text.clone(),
            ability_labels: object.ability_labels.clone(),
            power: object.base_power.as_ref().map(|power| power.base_value()),
            toughness: object
                .base_toughness
                .as_ref()
                .map(|toughness| toughness.base_value()),
            card_types: object.card_types.clone(),
            subtypes: object.subtypes.clone(),
            supertypes: object.supertypes.clone(),
            world_supertype_since: None,
            colors: object.colors(),
            loyalty: object.base_loyalty,
            abilities: object.abilities.clone().into(),
            static_abilities: Vec::new().into(),
            ability_gain_prohibitions: Vec::new(),
            aura_attach_filter: object.aura_attach_filter_owned(),
            controller: object.owner,
        }
    }

    fn battlefield_object(
        id: u32,
        name: &str,
        card_types: Vec<CardType>,
        pt: Option<(i32, i32)>,
    ) -> crate::object::Object {
        use crate::card::CardBuilder;
        use crate::ids::CardId;
        let mut builder = CardBuilder::new(CardId::from_raw(id), name).card_types(card_types);
        if let Some((power, toughness)) = pt {
            builder = builder.power_toughness(crate::card::PowerToughness::fixed(power, toughness));
        }
        crate::object::Object::from_card(
            ObjectId::from_raw(u64::from(id)),
            &builder.build(),
            PlayerId::from_index(0),
            crate::zone::Zone::Battlefield,
        )
    }

    fn board(
        objects: Vec<crate::object::Object>,
    ) -> (
        crate::game_state::ObjectMap,
        HashMap<ObjectId, CalculatedCharacteristics>,
    ) {
        let baseline = objects
            .iter()
            .map(|object| (object.id, chars_for(object)))
            .collect();
        let map = objects
            .into_iter()
            .map(|object| (object.id, Arc::new(object)))
            .collect();
        (map, baseline)
    }

    #[test]
    fn started_groups_for_sort_is_board_wide() {
        let (objects, baseline) = board(vec![
            battlefield_object(1, "Relic", vec![CardType::Artifact], None),
            battlefield_object(2, "Bear", vec![CardType::Creature], Some((2, 2))),
        ]);
        let game = GameState::new(vec!["Alice".to_string()], 20);

        let applied_group = ContinuousEffectGroupId::runtime(1);
        let mut applied =
            create_test_effect(1, 1, Modification::AddCardTypes(vec![CardType::Creature]));
        applied.applies_to = EffectTarget::Filter(ObjectFilter::artifact());
        applied.group = Some(applied_group);

        let idle_group = ContinuousEffectGroupId::runtime(2);
        let mut idle = create_test_effect(2, 2, Modification::AddCardTypes(vec![CardType::Land]));
        idle.applies_to = EffectTarget::Filter(ObjectFilter::enchantment());
        idle.group = Some(idle_group);

        let same_layer_group = ContinuousEffectGroupId::runtime(3);
        let mut same_layer =
            create_test_effect(3, 3, Modification::AddAbility(StaticAbility::flying()));
        same_layer.group = Some(same_layer_group);

        let started = started_groups_for_sort(
            [&applied, &idle, &same_layer],
            Layer::Ability,
            &baseline,
            &objects,
            &game,
        );
        assert!(started.contains(&applied_group));
        assert!(!started.contains(&idle_group));
        assert!(!started.contains(&same_layer_group));
    }

    #[test]
    fn loop_candidates_are_only_the_members_of_source_loops() {
        // 0 <-> 1 form a loop; 2 depends on 0; 3 <-> 4 form a loop that waits on 2.
        let depends_on = vec![vec![1], vec![0], vec![0], vec![4, 2], vec![3]];
        assert_eq!(dependency_loop_candidates(&depends_on), vec![0, 1]);
    }

    #[test]
    fn loop_fallback_keeps_dependents_waiting_for_the_loop() {
        let (objects, baseline) = board(vec![
            battlefield_object(1, "Relic", vec![CardType::Artifact], None),
            battlefield_object(2, "Bear", vec![CardType::Creature], Some((2, 2))),
        ]);
        let game = GameState::new(vec!["Alice".to_string()], 20);

        // A and B depend on each other; C depends only on A but is oldest.
        let mut artifacts_are_creatures =
            create_test_effect(1, 3, Modification::AddCardTypes(vec![CardType::Creature]));
        artifacts_are_creatures.applies_to = EffectTarget::Filter(ObjectFilter::artifact());
        let mut creatures_are_artifacts =
            create_test_effect(2, 2, Modification::AddCardTypes(vec![CardType::Artifact]));
        creatures_are_artifacts.applies_to = EffectTarget::Filter(ObjectFilter::creature());
        let mut creatures_are_lands =
            create_test_effect(3, 1, Modification::AddCardTypes(vec![CardType::Land]));
        creatures_are_lands.applies_to = EffectTarget::Filter(ObjectFilter::creature());

        let sorted = sort_layer_effects_with_baseline(
            &[
                &creatures_are_lands,
                &artifacts_are_creatures,
                &creatures_are_artifacts,
            ],
            &baseline,
            &objects,
            &game,
        );
        let ids: Vec<u64> = sorted.iter().map(|effect| effect.id.0).collect();
        // CR 613.8a: the loop resolves by timestamp (B then A); C, which
        // depends on A, applies after it despite the oldest timestamp.
        assert_eq!(ids, vec![2, 1, 3]);
    }

    #[test]
    fn fast_path_applies_characteristic_defining_effects_first() {
        let mut cda = create_test_effect(
            1,
            100,
            Modification::AddAllSubtypesOfFamily(crate::types::SubtypeFamily::Creature),
        );
        cda.source_type = EffectSourceType::CharacteristicDefining;
        let older = create_test_effect(2, 50, Modification::SetSubtypes(vec![Subtype::Goblin]));
        let sorted = sort_with_dependencies(&[&older, &cda]);
        let ids: Vec<u64> = sorted.iter().map(|effect| effect.id.0).collect();
        assert_eq!(ids, vec![1, 2]);
    }

    #[test]
    fn condition_classifier_tracks_what_a_condition_reads() {
        use crate::ConditionExpr as C;
        let controls_artifact = C::YouControl(ObjectFilter::artifact());
        assert!(condition_could_be_affected_by(
            &controls_artifact,
            &Modification::AddCardTypes(vec![CardType::Artifact])
        ));
        assert!(!condition_could_be_affected_by(
            &controls_artifact,
            &Modification::AddCardTypes(vec![CardType::Creature])
        ));
        assert!(!condition_could_be_affected_by(
            &controls_artifact,
            &Modification::AddAbility(StaticAbility::flying())
        ));
        assert!(!condition_could_be_affected_by(
            &C::YourTurn,
            &Modification::AddCardTypes(vec![CardType::Artifact])
        ));
        let power = C::SourcePowerAtLeast(3);
        assert!(condition_could_be_affected_by(
            &power,
            &Modification::ModifyPowerToughness {
                power: 1,
                toughness: 1
            }
        ));
        assert!(!condition_could_be_affected_by(
            &power,
            &Modification::AddColors(crate::color::ColorSet::from(crate::color::Color::Red))
        ));
        assert!(condition_could_be_affected_by(
            &C::Not(Box::new(C::And(
                Box::new(C::YourTurn),
                Box::new(controls_artifact)
            ))),
            &Modification::RemoveCardTypes(vec![CardType::Artifact])
        ));
    }

    #[test]
    fn conditioned_effect_depends_on_effect_its_condition_reads() {
        let (objects, baseline) = board(vec![battlefield_object(
            1,
            "Relic",
            vec![CardType::Artifact],
            None,
        )]);
        let game = GameState::new(vec!["Alice".to_string()], 20);

        let conditioned = create_test_effect(
            1,
            1,
            Modification::AddCardTypes(vec![CardType::Enchantment]),
        )
        .with_condition(crate::ConditionExpr::YouControl(ObjectFilter::creature()));
        let mut animate =
            create_test_effect(2, 2, Modification::AddCardTypes(vec![CardType::Creature]));
        animate.applies_to = EffectTarget::Filter(ObjectFilter::artifact());

        assert!(effect_depends_on_with_baseline_and_started_groups(
            &conditioned,
            &animate,
            &baseline,
            &objects,
            &game,
            &HashSet::new(),
            None,
        ));

        let unrelated = create_test_effect(
            1,
            1,
            Modification::AddCardTypes(vec![CardType::Enchantment]),
        )
        .with_condition(crate::ConditionExpr::YourTurn);
        assert!(!effect_depends_on_with_baseline_and_started_groups(
            &unrelated,
            &animate,
            &baseline,
            &objects,
            &game,
            &HashSet::new(),
            None,
        ));

        // An effect that applies to nothing cannot flip anyone's condition.
        let mut animate_lands =
            create_test_effect(3, 3, Modification::AddCardTypes(vec![CardType::Creature]));
        animate_lands.applies_to = EffectTarget::Filter(ObjectFilter::land());
        assert!(!effect_depends_on_with_baseline_and_started_groups(
            &conditioned,
            &animate_lands,
            &baseline,
            &objects,
            &game,
            &HashSet::new(),
            None,
        ));
    }

    #[test]
    fn attached_to_probe_does_not_require_a_creature() {
        use crate::card::CardBuilder;
        use crate::ids::CardId;
        use crate::zone::Zone;

        let mut game = GameState::new(vec!["Alice".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let land = game.create_object_from_card(
            &CardBuilder::new(CardId::from_raw(1), "Field")
                .card_types(vec![CardType::Land])
                .build(),
            alice,
            Zone::Battlefield,
        );
        let aura = game.create_object_from_card(
            &CardBuilder::new(CardId::from_raw(2), "Spreading Seas")
                .card_types(vec![CardType::Enchantment])
                .build(),
            alice,
            Zone::Battlefield,
        );
        game.object_mut(aura).unwrap().attached_to =
            Some(crate::object::AttachmentTarget::Object(land));

        let mut effect =
            create_test_effect(1, 1, Modification::SetSubtypes(vec![Subtype::Island]));
        effect.source = aura;
        effect.applies_to = EffectTarget::AttachedTo(aura);

        let land_object = game.object(land).unwrap();
        let chars = chars_for(land_object);
        assert!(effect_applies_with_chars(&effect, land_object, &chars, &game));
        assert!(!modification_can_affect_effect_target(
            &Modification::AddCardTypes(vec![CardType::Creature]),
            &effect.applies_to
        ));
    }

    #[test]
    fn later_sublayers_sort_against_earlier_sublayer_results() {
        use crate::target::ChooseSpec;
        let (objects, baseline) = board(vec![
            battlefield_object(1, "Goyf", vec![CardType::Creature], Some((2, 2))),
            battlefield_object(2, "Mimic", vec![CardType::Creature], Some((1, 1))),
        ]);
        let game = GameState::new(vec!["Alice".to_string()], 20);
        let goyf = ObjectId::from_raw(1);
        let mimic = ObjectId::from_raw(2);

        // 7a: Goyf's CDA makes it 5/5.
        let mut cda = create_test_effect(
            1,
            1,
            Modification::SetPowerToughness {
                power: Value::Fixed(5),
                toughness: Value::Fixed(5),
                sublayer: PtSublayer::CharacteristicDefining,
            },
        );
        cda.source = goyf;
        cda.applies_to = EffectTarget::Specific(goyf);
        cda.source_type = EffectSourceType::CharacteristicDefining;

        // 7b, newer: Goyf's base P/T becomes 5/5 (no change after 7a).
        let mut set_goyf = create_test_effect(
            2,
            20,
            Modification::SetPowerToughness {
                power: Value::Fixed(5),
                toughness: Value::Fixed(5),
                sublayer: PtSublayer::Setting,
            },
        );
        set_goyf.source = goyf;
        set_goyf.applies_to = EffectTarget::Specific(goyf);

        // 7b, older: Mimic's power becomes Goyf's power.
        let mut mirror = create_test_effect(
            3,
            10,
            Modification::SetPowerToughness {
                power: Value::PowerOf(Box::new(ChooseSpec::Source)),
                toughness: Value::Fixed(1),
                sublayer: PtSublayer::Setting,
            },
        );
        mirror.source = goyf;
        mirror.applies_to = EffectTarget::Specific(mimic);

        let sorted = sort_layer_effects_with_baseline(
            &[&set_goyf, &mirror, &cda],
            &baseline,
            &objects,
            &game,
        );
        let ids: Vec<u64> = sorted.iter().map(|effect| effect.id.0).collect();
        // Against the post-7a baseline the 7b setter changes nothing Mimic
        // reads, so 7b is plain timestamp order. A stale pre-layer-7 baseline
        // would have made Mimic wait for the setter.
        assert_eq!(ids, vec![1, 3, 2]);
    }

    #[test]
    fn computed_pt_setter_depends_on_setter_it_reads() {
        use crate::target::ChooseSpec;
        let (objects, baseline) = board(vec![
            battlefield_object(1, "Goyf", vec![CardType::Creature], Some((2, 2))),
            battlefield_object(2, "Mimic", vec![CardType::Creature], Some((1, 1))),
        ]);
        let game = GameState::new(vec!["Alice".to_string()], 20);
        let goyf = ObjectId::from_raw(1);
        let mimic = ObjectId::from_raw(2);

        let mut set_goyf = create_test_effect(
            1,
            20,
            Modification::SetPowerToughness {
                power: Value::Fixed(5),
                toughness: Value::Fixed(5),
                sublayer: PtSublayer::Setting,
            },
        );
        set_goyf.applies_to = EffectTarget::Specific(goyf);
        let mut mirror = create_test_effect(
            2,
            10,
            Modification::SetPowerToughness {
                power: Value::PowerOf(Box::new(ChooseSpec::Source)),
                toughness: Value::Fixed(1),
                sublayer: PtSublayer::Setting,
            },
        );
        mirror.source = goyf;
        mirror.applies_to = EffectTarget::Specific(mimic);

        assert!(effect_depends_on_with_baseline_and_started_groups(
            &mirror,
            &set_goyf,
            &baseline,
            &objects,
            &game,
            &HashSet::new(),
            None,
        ));
        assert!(!effect_depends_on_with_baseline_and_started_groups(
            &set_goyf,
            &mirror,
            &baseline,
            &objects,
            &game,
            &HashSet::new(),
            None,
        ));
        let sorted =
            sort_layer_effects_with_baseline(&[&mirror, &set_goyf], &baseline, &objects, &game);
        let ids: Vec<u64> = sorted.iter().map(|effect| effect.id.0).collect();
        assert_eq!(ids, vec![1, 2]);
    }

    #[test]
    fn value_based_modifier_depends_on_modifier_it_reads() {
        use crate::target::ChooseSpec;
        let (objects, baseline) = board(vec![
            battlefield_object(1, "Leader", vec![CardType::Creature], Some((2, 2))),
            battlefield_object(2, "Follower", vec![CardType::Creature], Some((1, 1))),
        ]);
        let game = GameState::new(vec!["Alice".to_string()], 20);
        let leader = ObjectId::from_raw(1);
        let follower = ObjectId::from_raw(2);

        let mut pump_leader = create_test_effect(
            1,
            20,
            Modification::ModifyPowerToughness {
                power: 2,
                toughness: 2,
            },
        );
        pump_leader.applies_to = EffectTarget::Specific(leader);
        let mut follow = create_test_effect(
            2,
            10,
            Modification::ModifyPowerToughnessValue {
                power: Value::PowerOf(Box::new(ChooseSpec::Source)),
                toughness: Value::Fixed(0),
            },
        );
        follow.source = leader;
        follow.applies_to = EffectTarget::Specific(follower);

        assert!(effect_depends_on_with_baseline_and_started_groups(
            &follow,
            &pump_leader,
            &baseline,
            &objects,
            &game,
            &HashSet::new(),
            None,
        ));
    }

    #[test]
    fn unevaluable_pt_value_falls_back_to_what_it_could_read() {
        let (objects, baseline) = board(vec![
            battlefield_object(1, "Big", vec![CardType::Creature], Some((4, 4))),
            battlefield_object(2, "Scaler", vec![CardType::Creature], Some((1, 1))),
        ]);
        let game = GameState::new(vec!["Alice".to_string()], 20);

        let mut greatest = create_test_effect(
            1,
            10,
            Modification::SetPowerToughness {
                power: Value::GreatestPower(ObjectFilter::creature()),
                toughness: Value::Fixed(1),
                sublayer: PtSublayer::Setting,
            },
        );
        greatest.applies_to = EffectTarget::Specific(ObjectId::from_raw(2));
        let mut set_big = create_test_effect(
            2,
            20,
            Modification::SetPowerToughness {
                power: Value::Fixed(7),
                toughness: Value::Fixed(7),
                sublayer: PtSublayer::Setting,
            },
        );
        set_big.applies_to = EffectTarget::Specific(ObjectId::from_raw(1));

        // GreatestPower is not evaluated by the simulator; the structural
        // fallback still sees that a layer 7 setter could change it.
        assert!(effect_depends_on_with_baseline_and_started_groups(
            &greatest,
            &set_big,
            &baseline,
            &objects,
            &game,
            &HashSet::new(),
            None,
        ));
    }

    #[test]
    fn copy_triggered_abilities_depends_on_triggered_ability_grant() {
        let (objects, baseline) = board(vec![
            battlefield_object(1, "Donor", vec![CardType::Creature], Some((2, 2))),
            battlefield_object(2, "Mirror", vec![CardType::Creature], Some((1, 1))),
        ]);
        let game = GameState::new(vec!["Alice".to_string()], 20);

        let mut copy = create_test_effect(
            1,
            10,
            Modification::CopyTriggeredAbilities {
                filter: ObjectFilter::creature(),
                exclude_source_name: false,
                exclude_source_id: true,
            },
        );
        copy.source = ObjectId::from_raw(2);
        copy.applies_to = EffectTarget::Specific(ObjectId::from_raw(2));
        let mut grant = create_test_effect(
            2,
            20,
            Modification::AddAbilityGeneric(Ability::triggered(
                crate::triggers::Trigger::this_deals_combat_damage_to_player(
                    crate::target::PlayerFilter::Any,
                ),
                vec![Effect::draw(1)],
            )),
        );
        grant.applies_to = EffectTarget::Specific(ObjectId::from_raw(1));

        assert!(effect_depends_on_with_baseline_and_started_groups(
            &copy,
            &grant,
            &baseline,
            &objects,
            &game,
            &HashSet::new(),
            None,
        ));
        let sorted =
            sort_layer_effects_with_baseline(&[&copy, &grant], &baseline, &objects, &game);
        let ids: Vec<u64> = sorted.iter().map(|effect| effect.id.0).collect();
        assert_eq!(ids, vec![2, 1]);
    }
}
