//! Dependency system for continuous effects.
//!
//! Per MTG Rule 613.8, continuous effects in the same layer/sublayer that would
//! depend on each other are applied in a specific order that handles the dependency,
//! regardless of their timestamps.
//!
//! A dependency exists if:
//! - The effects are in the same layer (or sublayer for Layer 7)
//! - Applying one effect would change whether the other applies, what it applies to,
//!   or what it does
//! - Neither effect is a characteristic-defining ability, or both are
//!
//! Example: Humility ("All creatures lose all abilities and have base power and
//! toughness 1/1") and an anthem from a creature. The anthem depends on Humility
//! because Humility removing abilities would stop the anthem from applying.

use std::collections::{HashMap, HashSet};

use crate::continuous::{
    CalculatedCharacteristics, ContinuousEffect, EffectSourceType, EffectTarget, Layer,
    Modification, PtSublayer,
};
use crate::effect::Value;
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::static_abilities::StaticAbility;
use crate::target::ObjectFilter;

/// Check if effect A depends on effect B.
///
/// Per Rule 613.8, A depends on B if:
/// 1. They apply in the same layer/sublayer
/// 2. Applying B first would change whether A applies, what A applies to,
///    or what A does to things it applies to
/// 3. Neither is a CDA, or both are CDAs
///
/// Returns true if A depends on B.
pub fn effect_depends_on(a: &ContinuousEffect, b: &ContinuousEffect) -> bool {
    // Rule 613.8: Must be in the same layer
    if a.modification.layer() != b.modification.layer() {
        return false;
    }

    // For Layer 7, must also be in the same sublayer
    if a.modification.layer() == Layer::PowerToughness {
        let sub_a = a.modification.pt_sublayer();
        let sub_b = b.modification.pt_sublayer();
        if sub_a != sub_b {
            return false;
        }
    }

    // Check CDA status - if one is CDA and the other isn't, no dependency
    let a_is_cda = matches!(a.source_type, EffectSourceType::CharacteristicDefining);
    let b_is_cda = matches!(b.source_type, EffectSourceType::CharacteristicDefining);
    if a_is_cda != b_is_cda {
        return false;
    }

    // Now check if B would affect A
    check_dependency_relationship(&a.modification, &b.modification, a.source, b.source)
}

fn effect_depends_on_with_baseline(
    a: &ContinuousEffect,
    b: &ContinuousEffect,
    baseline: &HashMap<ObjectId, CalculatedCharacteristics>,
    objects: &HashMap<ObjectId, crate::object::Object>,
    game: &GameState,
) -> bool {
    // First, check if applying B would change what A applies to.
    if effect_applicability_changed(a, b, baseline, objects, game) {
        return true;
    }

    // Then check if applying B would change what A does to any objects it applies to.
    if effect_output_changed(a, b, baseline, objects, game) {
        return true;
    }

    // Fall back to existing relationship checks for cases not covered by simulation.
    check_dependency_relationship(&a.modification, &b.modification, a.source, b.source)
}

/// Check if applying modification B would affect how modification A works.
fn check_dependency_relationship(
    a: &Modification,
    b: &Modification,
    _a_source: ObjectId,
    b_source: ObjectId,
) -> bool {
    match (a, b) {
        // ========================================
        // Layer 6 (Abilities) dependencies
        // ========================================

        // Removing all abilities depends on any effect that adds abilities.
        // This matches the rule that the remover should apply after the adders
        // when they are in the same layer.
        (Modification::RemoveAllAbilities, Modification::AddAbility(_))
        | (Modification::RemoveAllAbilities, Modification::AddAbilityGeneric(_))
        | (Modification::RemoveAllAbilities, Modification::CopyActivatedAbilities { .. })
        | (Modification::RemoveAllAbilities, Modification::AddCombatDamageDrawAbility) => true,

        // Granting abilities does not depend on a later removal.
        (Modification::AddAbility(_), Modification::RemoveAllAbilities)
        | (Modification::AddAbilityGeneric(_), Modification::RemoveAllAbilities)
        | (Modification::CopyActivatedAbilities { .. }, Modification::RemoveAllAbilities)
        | (Modification::AddCombatDamageDrawAbility, Modification::RemoveAllAbilities) => false,

        // If B removes specific abilities and A grants that ability
        (Modification::AddAbility(ability_a), Modification::RemoveAbility(ability_b)) => {
            // Check if they're the same ability
            static_abilities_match(ability_a, ability_b)
        }

        // Removing a specific static ability does not affect this granted trigger.
        (Modification::AddCombatDamageDrawAbility, Modification::RemoveAbility(_)) => false,

        // ========================================
        // Layer 7 (P/T) dependencies
        // ========================================

        // If B sets P/T and A modifies P/T, A doesn't depend on B
        // (setting happens first in sublayer ordering anyway)
        (Modification::ModifyPowerToughness { .. }, Modification::SetPowerToughness { .. }) => {
            false
        }

        // If B modifies P/T and A also modifies with fixed values,
        // no dependency (fixed modifiers are commutative)
        (Modification::ModifyPowerToughness { .. }, Modification::ModifyPowerToughness { .. }) => {
            // ModifyPowerToughness uses fixed i32 values, so no dependency
            false
        }

        // If A sets P/T using computed values and B modifies P/T,
        // A may depend on B if A's values reference P/T
        (
            Modification::SetPowerToughness {
                power: power_a,
                toughness: toughness_a,
                ..
            },
            Modification::ModifyPowerToughness { .. },
        ) => {
            // Check if A's values reference P/T that B could affect
            let a_refs_pt = value_references_pt(power_a) || value_references_pt(toughness_a);
            if !a_refs_pt {
                return false;
            }

            // B modifies P/T - check if it affects objects A references
            // ModifyPowerToughness affects whatever target it applies to
            // For now, conservatively assume B could affect any creature
            pt_value_depends_on_modification(power_a, b_source, true)
                || pt_value_depends_on_modification(toughness_a, b_source, true)
        }

        // If both A and B set P/T using computed values, check if A's values
        // depend on the object B is setting
        (
            Modification::SetPowerToughness {
                power: power_a,
                toughness: toughness_a,
                ..
            },
            Modification::SetPowerToughness { .. },
        ) => {
            // Check if A's values reference P/T
            let a_refs_pt = value_references_pt(power_a) || value_references_pt(toughness_a);
            if !a_refs_pt {
                return false;
            }

            // B sets P/T - check if it affects objects A references
            // SetPowerToughness affects a specific target
            pt_value_depends_on_modification(power_a, b_source, false)
                || pt_value_depends_on_modification(toughness_a, b_source, false)
        }

        // ModifyPower/ModifyToughness single variants
        (Modification::ModifyPower(_), Modification::ModifyPower(_))
        | (Modification::ModifyToughness(_), Modification::ModifyToughness(_))
        | (Modification::ModifyPower(_), Modification::ModifyToughness(_))
        | (Modification::ModifyToughness(_), Modification::ModifyPower(_)) => {
            // Fixed value modifiers, no dependency
            false
        }

        // SetPower/SetToughness with computed values
        (Modification::SetPower { value, .. }, Modification::ModifyPower(_))
        | (Modification::SetPower { value, .. }, Modification::ModifyPowerToughness { .. }) => {
            value_references_pt(value) && pt_value_depends_on_modification(value, b_source, true)
        }

        (Modification::SetToughness { value, .. }, Modification::ModifyToughness(_))
        | (Modification::SetToughness { value, .. }, Modification::ModifyPowerToughness { .. }) => {
            value_references_pt(value) && pt_value_depends_on_modification(value, b_source, true)
        }

        // Switch P/T interactions
        // If A switches P/T and B modifies P/T, the order matters
        (Modification::SwitchPowerToughness, Modification::ModifyPowerToughness { .. }) => true,
        (Modification::ModifyPowerToughness { .. }, Modification::SwitchPowerToughness) => true,

        // ========================================
        // Layer 4 (Type) dependencies
        // ========================================

        // If B changes card types and A is a type-dependent ability grant,
        // A may depend on B (e.g., effects that grant abilities to creatures
        // would be affected by something that removes the creature type)
        (Modification::AddAbility(_), Modification::SetCardTypes(_))
        | (Modification::AddAbility(_), Modification::AddCardTypes(_))
        | (Modification::AddAbility(_), Modification::RemoveCardTypes(_))
        | (Modification::AddCombatDamageDrawAbility, Modification::SetCardTypes(_))
        | (Modification::AddCombatDamageDrawAbility, Modification::AddCardTypes(_))
        | (Modification::AddCombatDamageDrawAbility, Modification::RemoveCardTypes(_)) => {
            // Conservative: assume dependency exists since we can't easily
            // determine if A's filter references card types
            // (Would need to inspect the filter, which requires more context)
            true
        }

        // If B changes subtypes and A is a subtype-dependent ability grant,
        // A may depend on B (e.g., "Elves get +1/+1" affected by type changes)
        (Modification::AddAbility(_), Modification::SetSubtypes(_))
        | (Modification::AddAbility(_), Modification::AddSubtypes(_))
        | (Modification::AddAbility(_), Modification::RemoveSubtypes(_))
        | (Modification::AddCombatDamageDrawAbility, Modification::SetSubtypes(_))
        | (Modification::AddCombatDamageDrawAbility, Modification::AddSubtypes(_))
        | (Modification::AddCombatDamageDrawAbility, Modification::RemoveSubtypes(_)) => {
            // Conservative: assume dependency exists
            true
        }

        // ========================================
        // Layer 5 (Color) dependencies
        // ========================================

        // If A grants protection from a color and B changes colors,
        // A depends on B (protection effectiveness changes based on colors)
        (Modification::AddAbility(ability), Modification::SetColors(_))
        | (Modification::AddAbility(ability), Modification::AddColors(_))
        | (Modification::AddAbility(ability), Modification::RemoveColors(_))
            if ability.has_protection()
                && ability
                    .protection_from()
                    .is_some_and(|p| matches!(p, crate::ability::ProtectionFrom::Color(_))) =>
        {
            true
        }

        // Default: no dependency
        _ => false,
    }
}

/// Check if two static abilities are functionally the same.
fn static_abilities_match(a: &StaticAbility, b: &StaticAbility) -> bool {
    // Simple equality check - could be more sophisticated
    a == b
}

fn effect_applicability_changed(
    a: &ContinuousEffect,
    b: &ContinuousEffect,
    baseline: &HashMap<ObjectId, CalculatedCharacteristics>,
    objects: &HashMap<ObjectId, crate::object::Object>,
    game: &GameState,
) -> bool {
    for (&id, obj) in objects {
        let Some(chars) = baseline.get(&id) else {
            continue;
        };
        let applies_before = effect_applies_with_chars(a, obj, chars, game);
        let mut chars_after = chars.clone();
        if effect_applies_with_chars(b, obj, chars, game) {
            apply_modification_to_chars_for_dependency(&b.modification, &mut chars_after, obj);
        }
        let applies_after = effect_applies_with_chars(a, obj, &chars_after, game);
        if applies_before != applies_after {
            return true;
        }
    }

    false
}

fn effect_output_changed(
    a: &ContinuousEffect,
    b: &ContinuousEffect,
    baseline: &HashMap<ObjectId, CalculatedCharacteristics>,
    objects: &HashMap<ObjectId, crate::object::Object>,
    game: &GameState,
) -> bool {
    if let Modification::CopyActivatedAbilities {
        filter,
        counter,
        include_mana,
        exclude_source_name,
        exclude_source_id,
    } = &a.modification
    {
        let before = collect_activated_ability_signatures(
            filter,
            *counter,
            *include_mana,
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
        } => (Some(power), Some(toughness)),
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

    let baseline_after = apply_effect_to_baseline(b, baseline, objects, game);

    if let Some(value) = a_power_value {
        let before = evaluate_value(value, a.source, a.controller, baseline, objects, game);
        let after = evaluate_value(
            value,
            a.source,
            a.controller,
            &baseline_after,
            objects,
            game,
        );
        if before != after {
            return true;
        }
    }
    if let Some(value) = a_toughness_value {
        let before = evaluate_value(value, a.source, a.controller, baseline, objects, game);
        let after = evaluate_value(
            value,
            a.source,
            a.controller,
            &baseline_after,
            objects,
            game,
        );
        if before != after {
            return true;
        }
    }

    false
}

fn evaluate_value(
    value: &Value,
    source: ObjectId,
    effect_controller: PlayerId,
    baseline: &HashMap<ObjectId, CalculatedCharacteristics>,
    objects: &HashMap<ObjectId, crate::object::Object>,
    game: &GameState,
) -> ValueEval {
    match value {
        Value::Fixed(n) => ValueEval::Scalar(*n),
        Value::Add(left, right) => {
            match (
                evaluate_value(left, source, effect_controller, baseline, objects, game),
                evaluate_value(right, source, effect_controller, baseline, objects, game),
            ) {
                (ValueEval::Scalar(a), ValueEval::Scalar(b)) => ValueEval::Scalar(a + b),
                _ => ValueEval::Unknown,
            }
        }
        Value::SourcePower => baseline
            .get(&source)
            .and_then(|c| c.power)
            .map(ValueEval::Scalar)
            .unwrap_or(ValueEval::Unknown),
        Value::SourceToughness => baseline
            .get(&source)
            .and_then(|c| c.toughness)
            .map(ValueEval::Scalar)
            .unwrap_or(ValueEval::Unknown),
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
        Value::CreaturesDiedThisTurn => ValueEval::Scalar(game.creatures_died_this_turn as i32),
        Value::PowerOf(target) | Value::ToughnessOf(target) => {
            use crate::target::ChooseSpec;
            let mut values = Vec::new();
            match target.as_ref() {
                ChooseSpec::Source => {
                    if let Some(chars) = baseline.get(&source) {
                        let v = match value {
                            Value::PowerOf(_) => chars.power,
                            Value::ToughnessOf(_) => chars.toughness,
                            _ => None,
                        };
                        if let Some(v) = v {
                            values.push(v);
                        }
                    }
                }
                ChooseSpec::Object(filter) => {
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
                            _ => None,
                        };
                        if let Some(v) = v {
                            values.push(v);
                        }
                    }
                }
                _ => {}
            }
            values.sort();
            ValueEval::Set(values)
        }
        _ => ValueEval::Unknown,
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
        EffectTarget::AttachedTo(source_id) => {
            object.zone == crate::zone::Zone::Battlefield
                && chars.card_types.contains(&crate::types::CardType::Creature)
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
        .map(|source| source.attached_to == Some(object.id))
        .unwrap_or(false)
}

fn object_matches_filter_with_chars(
    filter: &ObjectFilter,
    object: &crate::object::Object,
    chars: &CalculatedCharacteristics,
    game: &GameState,
    effect_controller: PlayerId,
) -> bool {
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
            _ => {}
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

    if let Some(required_name) = &filter.name
        && object.name != *required_name
    {
        return false;
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
    exclude_source_name: bool,
    exclude_source_id: bool,
    effect: &ContinuousEffect,
    baseline: &HashMap<ObjectId, CalculatedCharacteristics>,
    objects: &HashMap<ObjectId, crate::object::Object>,
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
            let is_activated = matches!(ability.kind, AbilityKind::Activated(_));
            let is_mana = matches!(ability.kind, AbilityKind::Mana(_));
            if !is_activated && !(include_mana && is_mana) {
                continue;
            }
            signatures.insert(format!("{:?}", ability.kind));
        }
    }

    signatures
}

fn apply_effect_to_baseline(
    effect: &ContinuousEffect,
    baseline: &HashMap<ObjectId, CalculatedCharacteristics>,
    objects: &HashMap<ObjectId, crate::object::Object>,
    game: &GameState,
) -> HashMap<ObjectId, CalculatedCharacteristics> {
    let mut after = baseline.clone();
    for (&id, obj) in objects {
        let Some(chars) = baseline.get(&id) else {
            continue;
        };
        if effect_applies_with_chars(effect, obj, chars, game) {
            let mut new_chars = chars.clone();
            apply_modification_to_chars_for_dependency(&effect.modification, &mut new_chars, obj);
            after.insert(id, new_chars);
        }
    }
    after
}

fn apply_modification_to_chars_for_dependency(
    modification: &Modification,
    chars: &mut CalculatedCharacteristics,
    _object: &crate::object::Object,
) {
    match modification {
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
            chars.card_types = types.clone();
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
            chars.subtypes = types.clone();
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
        Modification::RemoveAllCreatureTypes => {
            chars.subtypes.retain(|t| !t.is_creature_type());
        }
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
            chars
                .abilities
                .push(crate::ability::Ability::static_ability(ability.clone()));
        }
        Modification::AddAbilityGeneric(ability) => {
            chars.abilities.push(ability.clone());
        }
        Modification::AddCombatDamageDrawAbility => {
            chars.abilities.push(crate::ability::Ability::triggered(
                crate::triggers::Trigger::this_deals_combat_damage_to_player(),
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
        }
        Modification::RemoveAllAbilities => {
            chars.abilities.clear();
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
        Modification::SwitchPowerToughness => {
            std::mem::swap(&mut chars.power, &mut chars.toughness);
        }
        _ => {}
    }
}

fn evaluate_value_simple(value: &Value, chars: &CalculatedCharacteristics) -> ValueEval {
    match value {
        Value::Fixed(n) => ValueEval::Scalar(*n),
        Value::Add(left, right) => match (
            evaluate_value_simple(left, chars),
            evaluate_value_simple(right, chars),
        ) {
            (ValueEval::Scalar(a), ValueEval::Scalar(b)) => ValueEval::Scalar(a + b),
            _ => ValueEval::Unknown,
        },
        Value::SourcePower => chars
            .power
            .map(ValueEval::Scalar)
            .unwrap_or(ValueEval::Unknown),
        Value::SourceToughness => chars
            .toughness
            .map(ValueEval::Scalar)
            .unwrap_or(ValueEval::Unknown),
        _ => ValueEval::Unknown,
    }
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
        // These directly reference P/T of objects
        Value::SourcePower | Value::SourceToughness => true,
        Value::PowerOf(_) | Value::ToughnessOf(_) => true,
        Value::Add(left, right) => value_references_pt(left) || value_references_pt(right),

        // EffectValue could reference P/T from a prior effect
        Value::EffectValue(_) | Value::EffectValueOffset(_, _) => true,

        // These don't reference P/T
        Value::Fixed(_)
        | Value::X
        | Value::XTimes(_)
        | Value::Count(_)
        | Value::CountScaled(_, _)
        | Value::BasicLandTypesAmong(_)
        | Value::CreaturesDiedThisTurn
        | Value::CountPlayers(_)
        | Value::Devotion { .. }
        | Value::ColorsOfManaSpentToCastThisSpell
        | Value::ManaValueOf(_)
        | Value::LifeTotal(_)
        | Value::CardsInHand(_)
        | Value::CardsInGraveyard(_)
        | Value::SpellsCastThisTurn(_)
        | Value::SpellsCastBeforeThisTurn(_)
        | Value::CardTypesInGraveyard(_)
        | Value::WasKicked
        | Value::WasBoughtBack
        | Value::WasEntwined
        | Value::TimesPaidLabel(_)
        | Value::KickCount
        | Value::CountersOnSource(_)
        | Value::CountersOn(_, _)
        | Value::WasPaid(_)
        | Value::WasPaidLabel(_)
        | Value::TimesPaid(_)
        | Value::TaggedCount
        | Value::EventValue(_)
        | Value::EventValueOffset(_, _) => false,
    }
}

/// Check if effect B could affect the P/T values computed by effect A.
///
/// This handles the case where A sets P/T based on computed values that
/// B could modify. For example:
/// - A: "This creature's power is equal to the number of creatures you control"
///   - Doesn't depend on B modifying P/T
/// - A: "This creature's power is equal to another creature's power"
///   - Depends on B if B modifies that creature's P/T
fn pt_value_depends_on_modification(
    a_value: &Value,
    _b_source: ObjectId,
    b_affects_all: bool,
) -> bool {
    match a_value {
        // If A's value depends on source's P/T and B modifies source's P/T
        Value::SourcePower | Value::SourceToughness => {
            // A depends on its own source's P/T - if B modifies all creatures
            // or targets the same source, there's a potential dependency
            b_affects_all
        }

        // If A references a specific object's P/T
        Value::PowerOf(target) | Value::ToughnessOf(target) => {
            // Check if B could affect the referenced object
            // For ChooseSpec::Source, it means the source of effect A
            // For now, conservatively assume dependency if B affects all
            // or if we can't determine the specific object
            use crate::target::ChooseSpec;
            match target.as_ref() {
                ChooseSpec::Source => b_affects_all,
                ChooseSpec::Object(_filter) => {
                    // If B affects all, or if B's source matches the filter,
                    // there could be a dependency
                    b_affects_all
                }
                _ => b_affects_all,
            }
        }

        // EffectValue references a prior effect's result - could be anything
        Value::EffectValue(_) => {
            // Conservative: assume it could depend on P/T
            // In practice, this is rare in continuous effects
            b_affects_all
        }

        // These don't reference P/T, so no dependency
        _ => false,
    }
}

/// Sort effects considering dependencies.
///
/// Per Rule 613.8d, if dependencies would create a cycle, the effects are
/// applied in timestamp order as a fallback.
///
/// Returns effects sorted so that if A depends on B, B comes before A.
pub fn sort_with_dependencies<'a>(effects: &[&'a ContinuousEffect]) -> Vec<&'a ContinuousEffect> {
    if effects.len() <= 1 {
        return effects.to_vec();
    }

    // Build dependency graph: dependencies[i] contains effects that i depends on
    // If A depends on B, B must come before A in the result
    let mut depends_on: HashMap<usize, HashSet<usize>> = HashMap::new();
    for i in 0..effects.len() {
        depends_on.insert(i, HashSet::new());
    }

    let mut has_any_dependency = false;
    for i in 0..effects.len() {
        for j in 0..effects.len() {
            if i != j && effect_depends_on(effects[i], effects[j]) {
                // i depends on j, so j must come before i
                depends_on.get_mut(&i).unwrap().insert(j);
                has_any_dependency = true;
            }
        }
    }

    // If no dependencies, just sort by timestamp
    if !has_any_dependency {
        let mut sorted = effects.to_vec();
        sorted.sort_by_key(|e| e.timestamp);
        return sorted;
    }

    // Detect cycles
    if has_cycle(&depends_on, effects.len()) {
        // Fall back to timestamp ordering
        let mut sorted = effects.to_vec();
        sorted.sort_by_key(|e| e.timestamp);
        return sorted;
    }

    // Topological sort - effects with no dependencies come first
    // in_degree[i] = number of effects that i depends on (must come before i)
    let mut in_degree: Vec<usize> = vec![0; effects.len()];
    // in_degree is calculated from depends_on - no iteration needed
    for (i, deps) in &depends_on {
        in_degree[*i] = deps.len();
    }

    // Build reverse map: depended_by[j] = effects that depend on j
    let mut depended_by: HashMap<usize, Vec<usize>> = HashMap::new();
    for i in 0..effects.len() {
        depended_by.insert(i, Vec::new());
    }
    for (i, deps) in &depends_on {
        for &j in deps {
            depended_by.get_mut(&j).unwrap().push(*i);
        }
    }

    let mut result = Vec::new();
    let mut ready: Vec<usize> = (0..effects.len()).filter(|&i| in_degree[i] == 0).collect();

    // Sort ready queue so oldest timestamp is popped first.
    ready.sort_by_key(|&i| std::cmp::Reverse(effects[i].timestamp));

    while let Some(idx) = ready.pop() {
        result.push(effects[idx]);
        // Effects that depend on idx can now have their in_degree reduced
        for &dependent in &depended_by[&idx] {
            in_degree[dependent] -= 1;
            if in_degree[dependent] == 0 {
                ready.push(dependent);
            }
        }
        // Re-sort so oldest timestamp is popped first.
        ready.sort_by_key(|&i| std::cmp::Reverse(effects[i].timestamp));
    }

    result
}

pub fn sort_layer_effects_with_baseline<'a>(
    effects: &[&'a ContinuousEffect],
    baseline: &HashMap<ObjectId, CalculatedCharacteristics>,
    objects: &HashMap<ObjectId, crate::object::Object>,
    game: &GameState,
) -> Vec<&'a ContinuousEffect> {
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

        let mut sublayers: Vec<_> = by_sublayer.keys().cloned().collect();
        sublayers.sort();

        let mut result = Vec::new();
        for sublayer in sublayers {
            let sublayer_effects = &by_sublayer[&sublayer];
            let sorted =
                sort_with_dependencies_with_baseline(sublayer_effects, baseline, objects, game);
            result.extend(sorted);
        }

        result
    } else {
        sort_with_dependencies_with_baseline(effects, baseline, objects, game)
    }
}

fn sort_with_dependencies_with_baseline<'a>(
    effects: &[&'a ContinuousEffect],
    baseline: &HashMap<ObjectId, CalculatedCharacteristics>,
    objects: &HashMap<ObjectId, crate::object::Object>,
    game: &GameState,
) -> Vec<&'a ContinuousEffect> {
    if effects.len() <= 1 {
        return effects.to_vec();
    }

    let mut depends_on: HashMap<usize, HashSet<usize>> = HashMap::new();
    for i in 0..effects.len() {
        depends_on.insert(i, HashSet::new());
    }

    let mut has_any_dependency = false;
    for i in 0..effects.len() {
        for j in 0..effects.len() {
            if i != j
                && effect_depends_on_with_baseline(effects[i], effects[j], baseline, objects, game)
            {
                depends_on.get_mut(&i).unwrap().insert(j);
                has_any_dependency = true;
            }
        }
    }

    if !has_any_dependency {
        let mut sorted = effects.to_vec();
        sorted.sort_by_key(|e| e.timestamp);
        return sorted;
    }

    if has_cycle(&depends_on, effects.len()) {
        let mut sorted = effects.to_vec();
        sorted.sort_by_key(|e| e.timestamp);
        return sorted;
    }

    let mut in_degree: Vec<usize> = vec![0; effects.len()];
    for (i, deps) in &depends_on {
        in_degree[*i] = deps.len();
    }

    let mut depended_by: HashMap<usize, Vec<usize>> = HashMap::new();
    for i in 0..effects.len() {
        depended_by.insert(i, Vec::new());
    }
    for (i, deps) in &depends_on {
        for &j in deps {
            depended_by.get_mut(&j).unwrap().push(*i);
        }
    }

    let mut result = Vec::new();
    let mut ready: Vec<usize> = (0..effects.len()).filter(|&i| in_degree[i] == 0).collect();

    ready.sort_by_key(|&i| std::cmp::Reverse(effects[i].timestamp));

    while let Some(idx) = ready.pop() {
        result.push(effects[idx]);
        for &dependent in &depended_by[&idx] {
            in_degree[dependent] -= 1;
            if in_degree[dependent] == 0 {
                ready.push(dependent);
            }
        }
        ready.sort_by_key(|&i| std::cmp::Reverse(effects[i].timestamp));
    }

    result
}

/// Check if the dependency graph has a cycle.
fn has_cycle(dependencies: &HashMap<usize, HashSet<usize>>, n: usize) -> bool {
    let mut visited = vec![false; n];
    let mut in_stack = vec![false; n];

    fn dfs(
        node: usize,
        dependencies: &HashMap<usize, HashSet<usize>>,
        visited: &mut [bool],
        in_stack: &mut [bool],
    ) -> bool {
        visited[node] = true;
        in_stack[node] = true;

        if let Some(deps) = dependencies.get(&node) {
            for &dep in deps {
                if !visited[dep] {
                    if dfs(dep, dependencies, visited, in_stack) {
                        return true;
                    }
                } else if in_stack[dep] {
                    return true; // Cycle found
                }
            }
        }

        in_stack[node] = false;
        false
    }

    for i in 0..n {
        if !visited[i] && dfs(i, dependencies, &mut visited, &mut in_stack) {
            return true;
        }
    }

    false
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
    use crate::target::ObjectFilter;
    use crate::types::{CardType, Subtype};

    fn create_test_effect(id: u64, timestamp: u64, modification: Modification) -> ContinuousEffect {
        ContinuousEffect {
            id: crate::continuous::ContinuousEffectId::new(id),
            source: ObjectId::from_raw(id),
            controller: PlayerId::from_index(0),
            applies_to: EffectTarget::AllPermanents,
            modification,
            timestamp,
            duration: crate::effect::Until::Forever,
            condition: None,
            source_type: EffectSourceType::StaticAbility,
        }
    }

    #[test]
    fn test_no_dependency_different_layers() {
        let a = create_test_effect(
            1,
            100,
            Modification::ModifyPowerToughness {
                power: 1,
                toughness: 1,
            },
        );
        let b = create_test_effect(2, 50, Modification::AddAbility(StaticAbility::flying()));

        // Different layers - no dependency
        assert!(!effect_depends_on(&a, &b));
        assert!(!effect_depends_on(&b, &a));
    }

    #[test]
    fn test_remove_all_abilities_depends_on_add_ability() {
        let anthem = create_test_effect(1, 100, Modification::AddAbility(StaticAbility::flying()));
        let humility = create_test_effect(2, 50, Modification::RemoveAllAbilities);

        // Removing all abilities depends on the ability-adder.
        assert!(effect_depends_on(&humility, &anthem));
        // But not the other way around.
        assert!(!effect_depends_on(&anthem, &humility));
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
        // Create a scenario: humility depends on anthem
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

        // Anthem should come first because humility depends on it
        assert_eq!(sorted[0].id.0, 1); // anthem
        assert_eq!(sorted[1].id.0, 2); // humility
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
            duration: crate::effect::Until::Forever,
            condition: None,
            source_type: EffectSourceType::StaticAbility,
        };

        let b = ContinuousEffect {
            id: crate::continuous::ContinuousEffectId::new(2),
            source: ObjectId::from_raw(2),
            controller: PlayerId::from_index(0),
            applies_to: EffectTarget::AllPermanents,
            modification: Modification::AddCardTypes(vec![CardType::Creature]),
            timestamp: 100,
            duration: crate::effect::Until::Forever,
            condition: None,
            source_type: EffectSourceType::StaticAbility,
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
        let objects = HashMap::from([(object.id, object.clone())]);
        let baseline = HashMap::from([(
            object.id,
            CalculatedCharacteristics {
                name: object.name.clone(),
                power: object.base_power.as_ref().map(|p| p.base_value()),
                toughness: object.base_toughness.as_ref().map(|t| t.base_value()),
                card_types: object.card_types.clone(),
                subtypes: object.subtypes.clone(),
                supertypes: object.supertypes.clone(),
                colors: object.colors(),
                abilities: object.abilities.clone(),
                static_abilities: Vec::new(),
                controller: object.controller,
            },
        )]);
        let game = GameState::new(vec!["Alice".to_string()], 20);

        assert!(effect_depends_on_with_baseline(
            &a, &b, &baseline, &objects, &game
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

        let objects = HashMap::from([(land.id, land.clone())]);
        let baseline = HashMap::from([(
            land.id,
            CalculatedCharacteristics {
                name: land.name.clone(),
                power: land.base_power.as_ref().map(|p| p.base_value()),
                toughness: land.base_toughness.as_ref().map(|t| t.base_value()),
                card_types: land.card_types.clone(),
                subtypes: land.subtypes.clone(),
                supertypes: land.supertypes.clone(),
                colors: land.colors(),
                abilities: land.abilities.clone(),
                static_abilities: Vec::new(),
                controller: land.controller,
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
                exclude_source_name: false,
                exclude_source_id: false,
            },
            timestamp: 1,
            duration: crate::effect::Until::Forever,
            condition: None,
            source_type: EffectSourceType::StaticAbility,
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
            duration: crate::effect::Until::Forever,
            condition: None,
            source_type: EffectSourceType::StaticAbility,
        };

        assert!(effect_depends_on_with_baseline(
            &copy_effect,
            &grant_effect,
            &baseline,
            &objects,
            &game
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
        land.abilities.push(Ability {
            kind: AbilityKind::Activated(ActivatedAbility {
                mana_cost: TotalCost::free(),
                effects: vec![Effect::gain_life(1)],
                choices: vec![],
                timing: ActivationTiming::AnyTime,
                additional_restrictions: vec![],
            }),
            functional_zones: vec![Zone::Battlefield],
            text: None,
        });

        let objects = HashMap::from([(land.id, land.clone())]);
        let baseline = HashMap::from([(
            land.id,
            CalculatedCharacteristics {
                name: land.name.clone(),
                power: land.base_power.as_ref().map(|p| p.base_value()),
                toughness: land.base_toughness.as_ref().map(|t| t.base_value()),
                card_types: land.card_types.clone(),
                subtypes: land.subtypes.clone(),
                supertypes: land.supertypes.clone(),
                colors: land.colors(),
                abilities: land.abilities.clone(),
                static_abilities: Vec::new(),
                controller: land.controller,
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
                exclude_source_name: false,
                exclude_source_id: false,
            },
            timestamp: 1,
            duration: crate::effect::Until::Forever,
            condition: None,
            source_type: EffectSourceType::StaticAbility,
        };

        let remove_effect = ContinuousEffect {
            id: crate::continuous::ContinuousEffectId::new(4),
            source: ObjectId::from_raw(4),
            controller: PlayerId::from_index(0),
            applies_to: EffectTarget::Specific(land.id),
            modification: Modification::RemoveAllAbilities,
            timestamp: 2,
            duration: crate::effect::Until::Forever,
            condition: None,
            source_type: EffectSourceType::StaticAbility,
        };

        assert!(effect_depends_on_with_baseline(
            &copy_effect,
            &remove_effect,
            &baseline,
            &objects,
            &game
        ));
    }

    #[test]
    fn test_topo_ready_queue_uses_oldest_timestamp_first() {
        // e3 depends on e1, while e2 is independent.
        // Initial ready set is {e1, e2}; oldest (e1) should be applied first.
        let e1 = create_test_effect(1, 5, Modification::AddAbility(StaticAbility::flying()));
        let e2 = create_test_effect(2, 10, Modification::AddAbility(StaticAbility::haste()));
        let e3 = create_test_effect(3, 20, Modification::RemoveAllAbilities);

        let effects: Vec<&ContinuousEffect> = vec![&e1, &e2, &e3];
        let sorted = sort_with_dependencies(&effects);

        assert_eq!(sorted[0].id.0, 1); // oldest ready effect
        assert_eq!(sorted[1].id.0, 2); // next oldest ready effect
        assert_eq!(sorted[2].id.0, 3); // dependent effect
    }

    #[test]
    fn test_cycle_detection() {
        // Create a simple graph with a cycle
        let mut dependencies: HashMap<usize, HashSet<usize>> = HashMap::new();
        dependencies.insert(0, HashSet::from([1]));
        dependencies.insert(1, HashSet::from([2]));
        dependencies.insert(2, HashSet::from([0])); // Creates cycle: 0 -> 1 -> 2 -> 0

        assert!(has_cycle(&dependencies, 3));

        // Graph without cycle
        let mut no_cycle: HashMap<usize, HashSet<usize>> = HashMap::new();
        no_cycle.insert(0, HashSet::from([1]));
        no_cycle.insert(1, HashSet::from([2]));
        no_cycle.insert(2, HashSet::new());

        assert!(!has_cycle(&no_cycle, 3));
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
    fn test_fixed_pt_modifiers_no_dependency() {
        // Two fixed P/T modifiers should not depend on each other
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

        // Neither should depend on the other
        assert!(!effect_depends_on(&e1, &e2));
        assert!(!effect_depends_on(&e2, &e1));
    }

    #[test]
    fn test_set_pt_with_fixed_value_no_dependency() {
        use crate::effect::Value;

        // SetPowerToughness with fixed values doesn't depend on modifiers
        let setter = create_test_effect(
            1,
            100,
            Modification::SetPowerToughness {
                power: Value::Fixed(3),
                toughness: Value::Fixed(3),
                sublayer: PtSublayer::Setting,
            },
        );
        let modifier = create_test_effect(
            2,
            50,
            Modification::ModifyPowerToughness {
                power: 2,
                toughness: 2,
            },
        );

        // Setter with fixed values doesn't depend on modifier
        assert!(!effect_depends_on(&setter, &modifier));
    }

    #[test]
    fn test_set_pt_with_source_power_depends_on_other_setter() {
        use crate::effect::Value;

        // Two SetPowerToughness effects in the same sublayer where one uses SourcePower
        // should have a dependency if one's output affects the other's input.
        //
        // Example: Effect A sets P/T to creature count (no dependency)
        //          Effect B sets P/T equal to source's power (depends on anything affecting source P/T)
        //
        // However, within the same sublayer, if B references SourcePower and A modifies the source,
        // there's a dependency.
        let setter_a = create_test_effect(
            1,
            100,
            Modification::SetPowerToughness {
                power: Value::Fixed(5),
                toughness: Value::Fixed(5),
                sublayer: PtSublayer::Setting,
            },
        );
        let setter_b = create_test_effect(
            2,
            50,
            Modification::SetPowerToughness {
                power: Value::SourcePower, // References its own source's power
                toughness: Value::Fixed(3),
                sublayer: PtSublayer::Setting,
            },
        );

        // B uses SourcePower - if A set P/T on B's source, B would depend on A
        // The check_dependency_relationship sees that B's power uses SourcePower
        // and A could theoretically affect that (b_affects_all = false since A is SetPowerToughness)
        // Since b_affects_all is false and there's no direct source match, no dependency
        assert!(!effect_depends_on(&setter_b, &setter_a));

        // But if A is an anthem (affects all creatures), B would depend on A
        // This would be handled differently (through EffectTarget::AllCreatures filter)
    }

    #[test]
    fn test_set_pt_same_sublayer_with_computed_value() {
        use crate::effect::Value;
        use crate::target::{ChooseSpec, ObjectFilter};

        // SetPowerToughness with Value::PowerOf depends on another setter that targets that object
        let setter_b = create_test_effect(
            2,
            50,
            Modification::SetPowerToughness {
                power: Value::PowerOf(Box::new(ChooseSpec::Object(ObjectFilter::creature()))),
                toughness: Value::Fixed(3),
                sublayer: PtSublayer::Setting,
            },
        );
        let setter_a = create_test_effect(
            1,
            100,
            Modification::SetPowerToughness {
                power: Value::Fixed(5),
                toughness: Value::Fixed(5),
                sublayer: PtSublayer::Setting,
            },
        );

        // B references PowerOf(creature) - conservative approach assumes dependency
        // when we can't prove independence
        // Currently with b_affects_all=false, we return false (no dependency)
        // This is an area where more precise tracking could improve results
        assert!(!effect_depends_on(&setter_b, &setter_a));
    }

    #[test]
    fn test_set_pt_with_count_no_dependency() {
        use crate::effect::Value;
        use crate::target::ObjectFilter;

        // SetPowerToughness with Value::Count (creature count) doesn't depend on P/T modifiers
        let setter = create_test_effect(
            1,
            100,
            Modification::SetPowerToughness {
                power: Value::Count(ObjectFilter::creature()),
                toughness: Value::Count(ObjectFilter::creature()),
                sublayer: PtSublayer::Setting,
            },
        );
        let modifier = create_test_effect(
            2,
            50,
            Modification::ModifyPowerToughness {
                power: 2,
                toughness: 2,
            },
        );

        // Setter with creature count doesn't depend on P/T modifier
        assert!(!effect_depends_on(&setter, &modifier));
    }
}
