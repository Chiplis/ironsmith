use super::*;
use crate::ability::ActivatedAbilityRuntimeExt;
use crate::cards::CardDefinitionRuntimeExt;
use crate::filter::ObjectFilterExt as _;

fn resolve_modal_count_value(
    value: &crate::effect::Value,
    pending_x_value: Option<u32>,
    fallback: usize,
) -> usize {
    match value {
        crate::effect::Value::Fixed(n) => (*n).max(0) as usize,
        crate::effect::Value::X => pending_x_value.map(|x| x as usize).unwrap_or(fallback),
        crate::effect::Value::XTimes(multiplier) => pending_x_value
            .map(|x| ((x as i32) * *multiplier).max(0) as usize)
            .unwrap_or(fallback),
        _ => fallback,
    }
}

fn static_ability_is_granted_conspire_marker(
    ability: &crate::static_abilities::StaticAbility,
) -> bool {
    ability.id() == crate::static_abilities::StaticAbilityId::KeywordMarker
        && ability.display().eq_ignore_ascii_case("Conspire")
}

fn granted_conspire_count(game: &GameState, spell_id: ObjectId, caster: PlayerId) -> usize {
    let Some(object) = game.object(spell_id) else {
        return 0;
    };
    let attached_count = object
        .abilities
        .iter()
        .filter(|ability| ability.functions_in(&Zone::Stack))
        .filter_map(|ability| match &ability.kind {
            crate::ability::AbilityKind::Static(static_ability)
                if static_ability_is_granted_conspire_marker(static_ability) =>
            {
                Some(())
            }
            _ => None,
        })
        .count();
    let mut object_for_filter = object.clone();
    if let Some(chars) = game.current_characteristics(spell_id) {
        object_for_filter.name = chars.name.into();
        object_for_filter.card_types = chars.card_types.into();
        object_for_filter.subtypes = chars.subtypes.into();
        object_for_filter.supertypes = chars.supertypes.into();
        object_for_filter.color_override = Some(chars.colors);
    }

    let effect_count = game
        .all_continuous_effects()
        .into_iter()
        .filter(|effect| match &effect.modification {
            crate::continuous::Modification::AddAbility(ability) => {
                static_ability_is_granted_conspire_marker(ability)
            }
            _ => false,
        })
        .filter(|effect| match &effect.applies_to {
            crate::continuous::EffectTarget::Specific(id) => *id == spell_id,
            crate::continuous::EffectTarget::Source => effect.source == spell_id,
            crate::continuous::EffectTarget::Filter(filter) => {
                let filter_ctx = game
                    .filter_context_for(effect.controller, Some(effect.source))
                    .with_caster(Some(caster));
                filter.matches_non_recursive(&object_for_filter, &filter_ctx, game)
            }
            crate::continuous::EffectTarget::AllPermanents
            | crate::continuous::EffectTarget::AllCreatures
            | crate::continuous::EffectTarget::AttachedTo(_) => false,
        })
        .count();
    if attached_count + effect_count > 0 {
        return attached_count + effect_count;
    }

    if object.zone != Zone::Stack
        || game.controller_of(object) != caster
        || !(game.object_has_card_type(spell_id, crate::types::CardType::Instant)
            || game.object_has_card_type(spell_id, crate::types::CardType::Sorcery))
    {
        return 0;
    }

    let spell_colors = object_for_filter.colors();
    let is_red_or_green = spell_colors.contains(crate::color::Color::Red)
        || spell_colors.contains(crate::color::Color::Green);
    if !is_red_or_green {
        return 0;
    }

    game.battlefield
        .iter()
        .filter_map(|id| game.object(*id))
        .filter(|permanent| game.controller_of(permanent) == caster)
        .flat_map(|permanent| permanent.abilities.iter())
        .filter_map(|ability| match &ability.kind {
            crate::ability::AbilityKind::Static(static_ability)
                if ability.functions_in(&Zone::Battlefield) =>
            {
                Some(static_ability.display())
            }
            _ => None,
        })
        .filter(|display| {
            let normalized = display.to_ascii_lowercase();
            normalized.contains("instant")
                && normalized.contains("sorcery")
                && normalized.contains("you cast")
                && normalized.contains("have conspire")
        })
        .count()
}

fn ensure_granted_conspire_optional_costs(game: &mut GameState, pending: &mut PendingCast) -> bool {
    let conspire_count = granted_conspire_count(game, pending.spell_id, pending.caster);
    if conspire_count == 0 {
        return false;
    }

    let existing_count = game
        .object(pending.spell_id)
        .map(|spell| {
            spell
                .optional_costs
                .iter()
                .filter(|cost| cost.source_label == "Granted Conspire")
                .count()
        })
        .unwrap_or(0);
    let missing_count = conspire_count.saturating_sub(existing_count);
    if missing_count == 0 {
        return false;
    }
    let Some(spell) = game.object_mut(pending.spell_id) else {
        return false;
    };
    for _ in 0..missing_count {
        spell.optional_costs.push(crate::cost::OptionalCost::custom(
            "Granted Conspire",
            crate::cost::TotalCost::from_cost(crate::costs::Cost::effect(
                crate::effects::ConspireCostEffect::new(),
            )),
        ));
    }
    pending.optional_costs_paid = crate::cost::OptionalCostsPaid::from_costs(&spell.optional_costs);
    true
}

fn ensure_optional_life_cost_reduction_costs(
    game: &mut GameState,
    pending: &mut PendingCast,
) -> bool {
    let mut costs = crate::decision::optional_life_cost_reduction_costs_for_cast(
        game,
        pending.caster,
        pending.spell_id,
        &pending.casting_method,
        Some(pending.from_zone),
    );
    if costs.is_empty() {
        return false;
    }
    let Some(existing_spell) = game.object(pending.spell_id) else {
        return false;
    };
    costs.retain(|(source, optional)| {
        let label = crate::decision::optional_life_cost_reduction_label(optional, *source);
        !existing_spell
            .optional_costs
            .iter()
            .any(|existing| existing.source_label == label)
    });
    if costs.is_empty() {
        return false;
    }
    let Some(spell) = game.object_mut(pending.spell_id) else {
        return false;
    };
    for (source, optional) in costs {
        let label = crate::decision::optional_life_cost_reduction_label(&optional, source);
        spell.optional_costs.push(crate::cost::OptionalCost::custom(
            label,
            crate::cost::TotalCost::from_cost(crate::costs::Cost::life(optional.life_cost)),
        ));
    }
    pending.optional_costs_paid = crate::cost::OptionalCostsPaid::from_costs(&spell.optional_costs);
    true
}

/// Collect all available casting methods for a spell.
/// Returns a list of CastingMethodOption structs for each method that can be used.
pub(super) fn collect_available_casting_methods(
    game: &GameState,
    player: PlayerId,
    spell_id: ObjectId,
    from_zone: Zone,
) -> Vec<crate::decision::CastingMethodOption> {
    use crate::decision::{
        CastingMethodOption, can_cast_spell, can_cast_with_alternative_from_hand,
    };

    let mut methods = Vec::new();

    let Some(spell) = game.object(spell_id) else {
        return methods;
    };

    // Check normal casting method
    if from_zone == Zone::Hand && can_cast_spell(game, player, spell, &CastingMethod::Normal) {
        let cost_desc = spell
            .mana_cost
            .as_ref()
            .map(|cost| format_mana_cost_simple(cost))
            .unwrap_or_else(|| "0".to_string());
        let name = if spell.linked_face_layout == crate::card::LinkedFaceLayout::Split {
            spell.name.to_string()
        } else {
            "Normal".to_string()
        };
        methods.push(CastingMethodOption {
            method: CastingMethod::Normal,
            name,
            cost_description: cost_desc,
        });
    }

    // Check alternative casting methods from hand
    if from_zone == Zone::Hand {
        if can_cast_spell(game, player, spell, &CastingMethod::FaceDown) {
            methods.push(CastingMethodOption {
                method: CastingMethod::FaceDown,
                name: "Face down".to_string(),
                cost_description: "{3}".to_string(),
            });
        }

        let has_linked_other_half =
            crate::decision::spell_has_castable_linked_other_half(game, spell);
        if has_linked_other_half {
            if can_cast_spell(game, player, spell, &CastingMethod::SplitOtherHalf)
                && let Some(other_def) = game.linked_face_definition_by_name_or_id(
                    spell.other_face_name.as_deref(),
                    spell.other_face,
                )
            {
                let cost_desc = other_def
                    .card
                    .mana_cost
                    .as_ref()
                    .map(|cost| format_mana_cost_simple(cost))
                    .unwrap_or_else(|| "0".to_string());
                methods.push(CastingMethodOption {
                    method: CastingMethod::SplitOtherHalf,
                    name: other_def.card.name.to_string(),
                    cost_description: cost_desc,
                });
            }

            if spell.linked_face_layout == crate::card::LinkedFaceLayout::Split
                && spell.has_fuse
                && can_cast_spell(game, player, spell, &CastingMethod::Fuse)
            {
                let cost_desc = crate::decision::spell_mana_cost_for_cast(
                    game,
                    player,
                    spell,
                    &CastingMethod::Fuse,
                    from_zone,
                )
                .as_ref()
                .map(|cost| format_mana_cost_simple(cost))
                .unwrap_or_else(|| "0".to_string());
                methods.push(CastingMethodOption {
                    method: CastingMethod::Fuse,
                    name: "Fuse".to_string(),
                    cost_description: cost_desc,
                });
            }
        }

        for (idx, alt_cast) in spell.alternative_casts.iter().enumerate() {
            if alt_cast.cast_from_zone() == Zone::Hand
                && can_cast_with_alternative_from_hand(game, player, spell, spell_id, alt_cast)
            {
                let (name, cost_desc) = format_alternative_method(alt_cast, spell);
                methods.push(CastingMethodOption {
                    method: CastingMethod::Alternative(idx),
                    name,
                    cost_description: cost_desc,
                });
            }
        }

        let granted = game
            .effect_store
            .grant_registry
            .granted_alternative_casts_for_card(game, spell_id, Zone::Hand, player);
        let base_alt_idx = spell.alternative_casts.len();
        for (offset, grant) in granted.iter().enumerate() {
            if grant.method.cast_from_zone() != Zone::Hand
                || !can_cast_with_alternative_from_hand(
                    game,
                    player,
                    spell,
                    spell_id,
                    &grant.method,
                )
            {
                continue;
            }

            let (name, cost_desc) = format_alternative_method(&grant.method, spell);
            methods.push(CastingMethodOption {
                method: CastingMethod::PlayFrom {
                    source: grant.source_id,
                    zone: Zone::Hand,
                    use_alternative: Some(base_alt_idx + offset),
                },
                name,
                cost_description: cost_desc,
            });
        }
    }

    methods
}

pub(super) fn may_have_multiple_casting_methods(
    game: &GameState,
    player: PlayerId,
    spell_id: ObjectId,
    from_zone: Zone,
) -> bool {
    if from_zone != Zone::Hand {
        return false;
    }

    let Some(spell) = game.object(spell_id) else {
        return false;
    };

    if crate::decision::spell_can_be_cast_face_down(spell)
        || crate::decision::spell_has_castable_linked_other_half(game, spell)
        || spell.has_fuse
        || spell
            .alternative_casts
            .iter()
            .any(|method| method.cast_from_zone() == Zone::Hand)
    {
        return true;
    }

    game.effect_store
        .grant_registry
        .active_grants(game)
        .into_iter()
        .any(|grant| {
            grant.player == player
                && grant.zone == Zone::Hand
                && matches!(
                    grant.grantable,
                    crate::grant::Grantable::AlternativeCast(_)
                        | crate::grant::Grantable::DerivedAlternativeCast(_)
                )
        })
}

/// Format a mana cost in simple text form (e.g., "{3}{U}{U}").
pub(super) fn format_mana_cost_simple(cost: &crate::mana::ManaCost) -> String {
    use crate::mana::ManaSymbol;

    let mut parts = Vec::new();
    for pip in cost.pips() {
        if pip.len() == 1 {
            parts.push(match &pip[0] {
                ManaSymbol::Generic(n) => format!("{{{}}}", n),
                ManaSymbol::Colorless => "{C}".to_string(),
                ManaSymbol::White => "{W}".to_string(),
                ManaSymbol::Blue => "{U}".to_string(),
                ManaSymbol::Black => "{B}".to_string(),
                ManaSymbol::Red => "{R}".to_string(),
                ManaSymbol::Green => "{G}".to_string(),
                ManaSymbol::Snow => "{S}".to_string(),
                ManaSymbol::X => "{X}".to_string(),
                ManaSymbol::Life(n) => format!("{{{}/P}}", n),
            });
        } else {
            let alts: Vec<String> = pip
                .iter()
                .map(|s| match s {
                    ManaSymbol::Generic(n) => format!("{}", n),
                    ManaSymbol::Colorless => "C".to_string(),
                    ManaSymbol::White => "W".to_string(),
                    ManaSymbol::Blue => "U".to_string(),
                    ManaSymbol::Black => "B".to_string(),
                    ManaSymbol::Red => "R".to_string(),
                    ManaSymbol::Green => "G".to_string(),
                    ManaSymbol::Snow => "S".to_string(),
                    ManaSymbol::X => "X".to_string(),
                    ManaSymbol::Life(n) => format!("P/{}", n),
                })
                .collect();
            parts.push(format!("{{{}}}", alts.join("/")));
        }
    }
    if parts.is_empty() {
        "0".to_string()
    } else {
        parts.join("")
    }
}

pub(super) fn non_mana_costs_for_casting_method(
    game: &GameState,
    caster: PlayerId,
    spell: &crate::object::Object,
    casting_method: &CastingMethod,
) -> Vec<crate::costs::Cost> {
    match casting_method {
        CastingMethod::FaceDown => Vec::new(),
        CastingMethod::Alternative(idx) => spell
            .alternative_casts
            .get(*idx)
            .map(|method| method.non_mana_costs())
            .unwrap_or_default(),
        CastingMethod::PlayFrom {
            use_alternative: Some(idx),
            zone,
            ..
        } => {
            crate::decision::resolve_play_from_alternative_method(game, caster, spell, *zone, *idx)
                .or_else(|| spell.cast_alternative_method_owned())
                .map(|method| method.non_mana_costs())
                .unwrap_or_default()
        }
        _ => Vec::new(),
    }
}

pub(super) fn cost_references_x(cost: &crate::costs::Cost) -> bool {
    cost.effect_ref()
        .is_some_and(|effect| effect.references_cost_x())
}

pub(super) fn max_x_from_non_mana_costs(
    game: &GameState,
    caster: PlayerId,
    source: ObjectId,
    costs: &[crate::costs::Cost],
) -> Option<u32> {
    let mut max_x: Option<u32> = None;

    for cost in costs {
        let Some(effect) = cost.effect_ref() else {
            continue;
        };
        if let Some(matching) = effect.max_cost_x(game, source, caster) {
            max_x = Some(max_x.map_or(matching, |prev| prev.min(matching)));
        }
    }

    max_x
}

fn max_x_from_static_abilities(
    game: &GameState,
    caster: PlayerId,
    source: ObjectId,
) -> Option<u32> {
    let spell = game.object(source)?;
    let mut max_x = None;
    for ability in spell.abilities.iter() {
        if !ability.functional_zones.contains(&Zone::Stack) {
            continue;
        }
        let crate::ability::AbilityKind::Static(static_ability) = &ability.kind else {
            continue;
        };
        let Some(value) = static_ability.this_spell_x_maximum_value() else {
            continue;
        };
        let ctx = crate::effects::ExecutionContext::new_default(source, caster);
        let Ok(resolved) = crate::effects::helpers::resolve_value(game, &value, &ctx) else {
            continue;
        };
        let resolved = resolved.max(0) as u32;
        max_x = Some(max_x.map_or(resolved, |prev: u32| prev.min(resolved)));
    }
    max_x
}

fn min_x_from_static_abilities(
    game: &GameState,
    caster: PlayerId,
    source: ObjectId,
) -> Option<u32> {
    let spell = game.object(source)?;
    let mut min_x = None;
    for ability in spell.abilities.iter() {
        if !ability.functional_zones.contains(&Zone::Stack) {
            continue;
        }
        let crate::ability::AbilityKind::Static(static_ability) = &ability.kind else {
            continue;
        };
        let Some(value) = static_ability.this_spell_x_minimum_value() else {
            continue;
        };
        let ctx = crate::effects::ExecutionContext::new_default(source, caster);
        let Ok(resolved) = crate::effects::helpers::resolve_value(game, &value, &ctx) else {
            continue;
        };
        let resolved = resolved.max(0) as u32;
        min_x = Some(min_x.map_or(resolved, |prev: u32| prev.max(resolved)));
    }
    min_x
}

pub(super) fn activation_cost_steps_reference_x(steps: &[ActivationCostStep]) -> bool {
    steps.iter().any(|step| match step {
        ActivationCostStep::Cost(cost) => cost_references_x(cost),
        ActivationCostStep::Sacrifice { .. } | ActivationCostStep::CardChoice(_) => false,
    })
}

pub(super) fn max_x_from_activation_cost_steps(
    game: &GameState,
    caster: PlayerId,
    source: ObjectId,
    steps: &[ActivationCostStep],
) -> Option<u32> {
    let costs: Vec<_> = steps
        .iter()
        .filter_map(|step| match step {
            ActivationCostStep::Cost(cost) => Some(cost.clone()),
            ActivationCostStep::Sacrifice { .. } | ActivationCostStep::CardChoice(_) => None,
        })
        .collect();
    max_x_from_non_mana_costs(game, caster, source, &costs)
}

pub(super) fn compute_spell_cast_x_bounds(
    game: &GameState,
    caster: PlayerId,
    stack_id: ObjectId,
    casting_method: &CastingMethod,
    mana_cost_to_pay: Option<&crate::mana::ManaCost>,
) -> (bool, u32, u32) {
    let Some(spell) = game.object(stack_id) else {
        return (false, 0, 0);
    };

    let printed_has_x = spell.mana_cost.as_ref().is_some_and(|cost| cost.has_x());
    let pay_has_x = mana_cost_to_pay.is_some_and(|cost| cost.has_x());

    let mut non_mana_costs = non_mana_costs_for_casting_method(game, caster, spell, casting_method);
    non_mana_costs.extend(spell.additional_non_mana_costs());

    let costs_need_x = non_mana_costs.iter().any(cost_references_x);
    let needs_x = printed_has_x || pay_has_x || costs_need_x;
    if !needs_x {
        return (false, 0, 0);
    }

    let min_x = min_x_from_static_abilities(game, caster, stack_id).unwrap_or(0);
    let mut max_x = None;

    if pay_has_x && let Some(cost) = mana_cost_to_pay {
        let mana_spend_policy = game.mana_spend_policy(caster, Some(stack_id));
        let allow_black_life = crate::decision::mana_cost_has_black_symbol(cost)
            && game.player_can_pay_black_with_life_for_reason(
                caster,
                Some(stack_id),
                crate::costs::PaymentReason::CastSpell,
            );
        let caster_only_max = compute_potential_mana(game, caster)
            .max_x_for_cost_with_mana_spend_policy_and_black_life(
                cost,
                &mana_spend_policy,
                allow_black_life,
            );
        max_x = Some(
            crate::decision::max_x_payable_with_assist(game, caster, stack_id, cost)
                .unwrap_or(caster_only_max),
        );
    }

    if let Some(max_cost) = max_x_from_non_mana_costs(game, caster, stack_id, &non_mana_costs) {
        max_x = Some(max_x.map_or(max_cost, |prev| prev.min(max_cost)));
    }

    if let Some(max_static) = max_x_from_static_abilities(game, caster, stack_id) {
        max_x = Some(max_x.map_or(max_static, |prev| prev.min(max_static)));
    }

    (true, min_x, max_x.unwrap_or(0))
}

/// Format an alternative casting method's name and cost description.
pub(super) fn format_alternative_method(
    method: &crate::alternative_cast::AlternativeCastingMethod,
    spell: &crate::object::Object,
) -> (String, String) {
    use crate::alternative_cast::AlternativeCastingMethod;

    match method {
        AlternativeCastingMethod::Dash { cost } => {
            let cost_desc = format_mana_cost_simple(cost);
            ("Dash".to_string(), cost_desc)
        }
        AlternativeCastingMethod::Blitz { total_cost } => {
            let cost_desc = total_cost
                .mana_cost()
                .map(|cost| format_mana_cost_simple(cost))
                .unwrap_or_else(|| "0".to_string());
            ("Blitz".to_string(), cost_desc)
        }
        AlternativeCastingMethod::Warp { cost } => {
            let cost_desc = format_mana_cost_simple(cost);
            (
                "Warp".to_string(),
                format!("{cost_desc}, exile later and cast from exile"),
            )
        }
        AlternativeCastingMethod::Plot { cost } => {
            let cost_desc = format_mana_cost_simple(cost);
            (
                "Plot".to_string(),
                format!("{} to plot, free later", cost_desc),
            )
        }
        AlternativeCastingMethod::Suspend { cost, time } => {
            let cost_desc = format_mana_cost_simple(cost);
            (
                "Suspend".to_string(),
                format!("{cost_desc} with {time} time counters"),
            )
        }
        AlternativeCastingMethod::Disturb { cost } => {
            let cost_desc = format_mana_cost_simple(cost);
            ("Disturb".to_string(), format!("{cost_desc} from graveyard"))
        }
        AlternativeCastingMethod::Overload { cost, .. } => {
            let cost_desc = format_mana_cost_simple(cost);
            (
                "Overload".to_string(),
                format!("{cost_desc} with each-mode text"),
            )
        }
        AlternativeCastingMethod::Cleave { cost, .. } => {
            let cost_desc = format_mana_cost_simple(cost);
            (
                "Cleave".to_string(),
                format!("{cost_desc} with bracketed text removed"),
            )
        }
        AlternativeCastingMethod::Awaken { cost, .. } => {
            let cost_desc = format_mana_cost_simple(cost);
            ("Awaken".to_string(), cost_desc)
        }
        AlternativeCastingMethod::Flashback { .. } => {
            let cost_desc = method
                .mana_cost()
                .map(|cost| format_mana_cost_simple(cost))
                .unwrap_or_else(|| "0".to_string());
            ("Flashback".to_string(), cost_desc)
        }
        AlternativeCastingMethod::Harmonize { .. } => {
            let cost_desc = method
                .mana_cost()
                .map(|cost| format_mana_cost_simple(cost))
                .unwrap_or_else(|| "0".to_string());
            (
                "Harmonize".to_string(),
                format!("{cost_desc} from graveyard"),
            )
        }
        AlternativeCastingMethod::Retrace { .. } => {
            let mut parts = Vec::new();
            if let Some(mana) = method.mana_cost() {
                parts.push(format_mana_cost_simple(mana));
            }
            for cost in method.non_mana_costs() {
                let rendered = cost.display();
                if !rendered.trim().is_empty() {
                    parts.push(rendered);
                }
            }
            ("Retrace".to_string(), parts.join(", "))
        }
        AlternativeCastingMethod::JumpStart { .. } => {
            // Jump-start uses the spell's mana cost plus discarding a card
            let cost_desc = spell
                .mana_cost
                .as_ref()
                .map(|cost| format_mana_cost_simple(cost))
                .unwrap_or_else(|| "0".to_string());
            (
                "Jump-Start".to_string(),
                format!("{}, Discard a card", cost_desc),
            )
        }
        AlternativeCastingMethod::Escape {
            cost, exile_count, ..
        } => {
            let cost_desc = cost
                .as_ref()
                .map(|cost| format_mana_cost_simple(cost))
                .or_else(|| {
                    spell
                        .mana_cost
                        .as_ref()
                        .map(|cost| format_mana_cost_simple(cost))
                })
                .unwrap_or_else(|| "0".to_string());
            (
                "Escape".to_string(),
                format!("{}, Exile {} cards from graveyard", cost_desc, exile_count),
            )
        }
        AlternativeCastingMethod::Bestow { .. } => {
            let mut parts = Vec::new();
            if let Some(mana) = method.mana_cost() {
                parts.push(format_mana_cost_simple(mana));
            }
            for cost in method.non_mana_costs() {
                let rendered = cost.display();
                if !rendered.trim().is_empty() {
                    parts.push(rendered);
                }
            }
            ("Bestow".to_string(), parts.join(", "))
        }
        AlternativeCastingMethod::Mutate { cost } => {
            ("Mutate".to_string(), format_mana_cost_simple(cost))
        }
        AlternativeCastingMethod::Composed { .. } | AlternativeCastingMethod::FromZone { .. } => {
            let mana_cost = method.mana_cost();
            let name = method.name();
            let mut parts = Vec::new();
            if let Some(mana) = mana_cost {
                parts.push(format_mana_cost_simple(mana));
            }
            for cost in method.non_mana_costs() {
                let rendered = cost.display();
                if !rendered.trim().is_empty() {
                    parts.push(rendered);
                }
            }
            let cost_desc = if parts.is_empty() {
                "Free".to_string()
            } else {
                parts.join(", ")
            };
            (name.to_string(), cost_desc)
        }
        AlternativeCastingMethod::Trap {
            cost, condition, ..
        } => {
            let cost_desc = format_mana_cost_simple(cost);
            let condition_desc = match condition {
                crate::alternative_cast::TrapCondition::OpponentCastSpells { count } => {
                    format!("If opponent cast {}+ spells this turn", count)
                }
                crate::alternative_cast::TrapCondition::OpponentSearchedLibrary => {
                    "If opponent searched their library".to_string()
                }
                crate::alternative_cast::TrapCondition::OpponentCreatureEntered => {
                    "If opponent had a creature enter".to_string()
                }
                crate::alternative_cast::TrapCondition::CreatureDealtDamageToYou => {
                    "If a creature dealt damage to you".to_string()
                }
            };
            (
                "Trap".to_string(),
                format!("{} ({})", cost_desc, condition_desc),
            )
        }
        AlternativeCastingMethod::Madness { cost } => {
            let cost_desc = format_mana_cost_simple(cost);
            ("Madness".to_string(), cost_desc)
        }
        AlternativeCastingMethod::Miracle { cost } => {
            let cost_desc = format_mana_cost_simple(cost);
            ("Miracle".to_string(), cost_desc)
        }
        AlternativeCastingMethod::FlashWithAdditionalCost {
            additional_cost, ..
        } => (
            "Flash".to_string(),
            format!("{} more", format_mana_cost_simple(additional_cost)),
        ),
        AlternativeCastingMethod::Foretell { cost } => {
            let cost_desc = format_mana_cost_simple(cost);
            ("Foretell".to_string(), cost_desc)
        }
    }
}

/// Helper to extract modal spec from a spell's effects.
///
/// Searches through the spell's effects to find if it has a modal effect.
/// For compositional effects like ConditionalEffect, this evaluates conditions at cast time
/// to determine which branch's modal spec to use (e.g., Akroma's Will checking YouControlCommander).
/// Returns the modal specification if found.
pub(super) fn extract_modal_spec_from_spell(
    game: &GameState,
    spell_id: ObjectId,
    controller: PlayerId,
) -> Option<crate::effects::ModalSpec> {
    let obj = game.object(spell_id)?;

    // Check spell effects with context to handle conditional effects like Akroma's Will
    if let Some(ref effects) = obj.spell_effect {
        for effect in effects.all_effects() {
            if let Some(spec) = effect
                .0
                .get_modal_spec_with_context(game, controller, spell_id)
            {
                return Some(spec);
            }
        }
    }

    None
}

/// Helper to extract modal spec from a resolution program.
pub(super) fn extract_modal_spec_from_program(
    game: &GameState,
    effects: &crate::resolution::ResolutionProgram,
    controller: PlayerId,
    source: ObjectId,
) -> Option<crate::effects::ModalSpec> {
    for effect in effects.all_effects() {
        if let Some(spec) = effect
            .0
            .get_modal_spec_with_context(game, controller, source)
        {
            return Some(spec);
        }
    }

    None
}

/// Check for modal effects and either prompt for mode selection or continue to splice choices.
///
/// Per MTG rule 601.2b, modes must be chosen before targets.
/// This is called after the spell is proposed (moved to stack).
pub(super) fn check_modes_or_continue(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    pending: PendingCast,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    // Check if the spell has modal effects (with context for conditional effects like Akroma's Will)
    if let Some(modal_spec) = extract_modal_spec_from_spell(game, pending.spell_id, pending.caster)
    {
        let player = pending.caster;
        let source = pending.spell_id;
        let spell_effects = game
            .object(source)
            .map(|obj| {
                obj.spell_effect
                    .as_ref()
                    .map(|program| program.all_effects_owned())
                    .unwrap_or_default()
            })
            .unwrap_or_default();

        // Resolve min/max mode counts
        let base_max_modes = resolve_modal_count_value(
            &modal_spec.max_modes,
            pending.x_value,
            modal_spec.mode_descriptions.len().max(1),
        );
        let base_min_modes =
            resolve_modal_count_value(&modal_spec.min_modes, pending.x_value, base_max_modes);
        let conditional_range = conditional_mode_range_for_pending(game, &pending, &modal_spec);
        let (min_modes, max_modes) = conditional_range
            .map(|(_, conditional_min, conditional_max)| {
                (
                    base_min_modes.min(conditional_min),
                    base_max_modes.max(conditional_max),
                )
            })
            .unwrap_or((base_min_modes, base_max_modes));

        let spell_name = game
            .object(source)
            .map(|o| o.name.to_string())
            .unwrap_or_else(|| "spell".to_string());

        let base_has_legal_targets =
            spell_has_legal_targets(game, &spell_effects, player, Some(source));
        let conditional_has_legal_targets =
            conditional_range.is_some_and(|(optional_cost_index, _, _)| {
                let mut hypothetical = game.clone();
                if let Some(spell) = hypothetical.object_mut(source) {
                    spell.optional_costs_paid.pay_times(optional_cost_index, 1);
                }
                hypothetical.refresh_continuous_state();
                spell_has_legal_targets(&hypothetical, &spell_effects, player, Some(source))
            });
        if !base_has_legal_targets && !conditional_has_legal_targets {
            return Err(GameLoopError::InvalidState(
                "No legal mode/target combination available".to_string(),
            ));
        }

        let mode_options: Vec<crate::decisions::specs::ModeOption> = modal_spec
            .mode_descriptions
            .iter()
            .enumerate()
            .map(|(i, desc)| {
                let legal = spell_has_legal_targets_with_mode_preview(
                    game,
                    &spell_effects,
                    player,
                    Some(source),
                    &[i],
                );
                crate::decisions::specs::ModeOption::with_legality(i, desc.clone(), legal)
            })
            .collect();

        // Set up pending cast for modes stage
        let mut pending = pending;
        pending.stage = CastStage::ChoosingModes;
        state.pending_cast = Some(pending);

        Ok(GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::Modes(
                crate::decisions::context::ModesContext {
                    player,
                    source: Some(source),
                    spell_name,
                    spec: crate::decisions::ModesSpec::new(
                        source,
                        mode_options,
                        min_modes,
                        max_modes,
                        modal_spec.allow_repeated_modes,
                        modal_spec.mode_point_costs,
                    ),
                },
            ),
        ))
    } else {
        // No modal effects, continue to splice choices.
        check_splice_or_continue(game, trigger_queue, state, pending, decision_maker)
    }
}

fn splice_quality_matches_spell(
    game: &GameState,
    spell_id: ObjectId,
    quality: crate::static_abilities::SpliceQuality,
) -> bool {
    match quality {
        crate::static_abilities::SpliceQuality::Arcane => {
            game.current_has_subtype(spell_id, crate::types::Subtype::Arcane)
        }
        crate::static_abilities::SpliceQuality::InstantOrSorcery => {
            game.current_has_card_type(spell_id, crate::types::CardType::Instant)
                || game.current_has_card_type(spell_id, crate::types::CardType::Sorcery)
        }
    }
}

fn applicable_splice_spec(
    game: &GameState,
    card_id: ObjectId,
    spell_id: ObjectId,
) -> Option<crate::static_abilities::SpliceSpec<crate::costs::Cost>> {
    game.current_abilities(card_id)?
        .into_iter()
        .filter_map(|ability| match ability.kind {
            crate::ability::AbilityKind::Static(static_ability) => {
                static_ability.splice_spec().cloned()
            }
            _ => None,
        })
        .find(|spec| splice_quality_matches_spell(game, spell_id, spec.quality))
}

/// Continue CR 601.2b after modes by offering every applicable splice card in
/// the caster's hand. The order returned by the chooser is the order in which
/// the added programs resolve, after the main spell's own program.
pub(super) fn check_splice_or_continue(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    mut pending: PendingCast,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    let hand = game
        .player(pending.caster)
        .map(|player| player.hand.clone())
        .unwrap_or_default();
    let candidates = hand
        .into_iter()
        .filter(|card_id| applicable_splice_spec(game, *card_id, pending.spell_id).is_some())
        .map(|card_id| {
            let name = game
                .object(card_id)
                .map(|card| card.name.to_string())
                .unwrap_or_else(|| format!("Card #{}", card_id.0));
            crate::decisions::context::SelectableObject::new(card_id, name)
                .with_selection_identity(crate::decisions::context::SelectionIdentity::StableId)
                .with_reveal_policy(crate::decisions::context::SelectionRevealPolicy::Public)
        })
        .collect::<Vec<_>>();

    if candidates.is_empty() {
        return check_optional_costs_or_continue(
            game,
            trigger_queue,
            state,
            pending,
            decision_maker,
        );
    }

    let max = candidates.len();
    let spell_name = game
        .object(pending.spell_id)
        .map(|spell| spell.name.to_string())
        .unwrap_or_else(|| "spell".to_string());
    pending.stage = CastStage::ChoosingSplices;
    let player = pending.caster;
    let source = pending.spell_id;
    state.pending_cast = Some(pending);
    let context = crate::decisions::context::SelectObjectsContext::new(
        player,
        Some(source),
        format!("Reveal cards to splice onto {spell_name}, in resolution order"),
        candidates,
        0,
        Some(max),
    )
    .require_explicit_choice()
    .with_selection_identity(crate::decisions::context::SelectionIdentity::StableId)
    .with_reveal_policy(crate::decisions::context::SelectionRevealPolicy::Public);
    Ok(GameProgress::NeedsDecisionCtx(
        crate::decisions::context::DecisionContext::SelectObjects(context),
    ))
}

/// Apply the simultaneous splice reveal/order choice and extend the proposed
/// stack spell with copied resolution programs and additional costs.
pub(super) fn apply_splice_response(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    selected_cards: &[ObjectId],
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    let mut pending = state.pending_cast.take().ok_or_else(|| {
        GameLoopError::InvalidState("No pending cast for splice response".to_string())
    })?;
    if pending.stage != CastStage::ChoosingSplices {
        state.rollback_action(game);
        return Err(GameLoopError::InvalidState(
            "Splice response outside the splice announcement stage".to_string(),
        ));
    }

    let hand = game
        .player(pending.caster)
        .map(|player| player.hand.clone())
        .unwrap_or_default();
    let mut seen = std::collections::HashSet::new();
    let mut additions = Vec::with_capacity(selected_cards.len());
    for card_id in selected_cards {
        if !hand.contains(card_id) || !seen.insert(*card_id) {
            state.rollback_action(game);
            return Err(GameLoopError::ActionCancelled(
                "Each selected splice card must be a distinct card in the caster's hand"
                    .to_string(),
            ));
        }
        let Some(spec) = applicable_splice_spec(game, *card_id, pending.spell_id) else {
            state.rollback_action(game);
            return Err(GameLoopError::ActionCancelled(
                "Selected card has no splice ability applicable to this spell".to_string(),
            ));
        };
        let Some(card) = game.object(*card_id) else {
            state.rollback_action(game);
            return Err(GameLoopError::ActionCancelled(
                "Selected splice card no longer exists".to_string(),
            ));
        };
        additions.push((
            card.stable_id,
            spec.cost,
            card.spell_effect_owned().unwrap_or_default(),
        ));
    }

    if !additions.is_empty() {
        let viewers = game
            .players
            .iter()
            .filter(|player| player.is_in_game())
            .map(|player| player.id)
            .collect::<Vec<_>>();
        for viewer in viewers {
            let view_ctx = crate::decisions::context::ViewCardsContext::new(
                viewer,
                pending.caster,
                Some(pending.spell_id),
                Zone::Hand,
                "Reveal cards spliced onto a spell",
            )
            .with_public(true);
            decision_maker.view_cards(game, viewer, selected_cards, &view_ctx);
        }
        for card_id in selected_cards {
            let snapshot = game
                .object(*card_id)
                .map(|card| crate::snapshot::ObjectSnapshot::from_object(card, game));
            game.queue_trigger_event(
                pending.provenance,
                crate::triggers::TriggerEvent::new_with_provenance(
                    crate::events::CardRevealedEvent::new(
                        pending.caster,
                        *card_id,
                        Zone::Hand,
                        Some(pending.spell_id),
                        snapshot,
                    ),
                    pending.provenance,
                ),
            );
        }

        let Some(spell) = game.object_mut(pending.spell_id) else {
            state.rollback_action(game);
            return Err(GameLoopError::ActionCancelled(
                "Proposed spell no longer exists".to_string(),
            ));
        };
        spell.begin_splice_cast_overlay();
        let mut program = spell.spell_effect_owned().unwrap_or_default();
        for (stable_id, cost, added_program) in additions {
            pending.spliced_cards.push(stable_id);
            pending.splice_costs.push(cost);
            program.extend(added_program);
        }
        spell.spell_effect = Some(program.into());
        game.refresh_continuous_state();

        let legal = game
            .object(pending.spell_id)
            .and_then(|spell| spell.spell_effect.as_ref())
            .is_none_or(|program| {
                spell_program_has_legal_targets_with_modes(
                    game,
                    program,
                    pending.caster,
                    Some(pending.spell_id),
                    pending.chosen_modes.as_deref(),
                )
            });
        if !legal {
            state.rollback_action(game);
            return Err(GameLoopError::ActionCancelled(
                "Selected splice text has required choices with no legal completion".to_string(),
            ));
        }
        pending.remaining_requirements = game
            .object(pending.spell_id)
            .and_then(|spell| spell.spell_effect.as_ref())
            .map(|program| {
                extract_target_requirements_from_program_with_modes(
                    game,
                    program,
                    pending.caster,
                    Some(pending.spell_id),
                    pending.chosen_modes.as_deref(),
                )
            })
            .unwrap_or_default();
    }

    check_optional_costs_or_continue(game, trigger_queue, state, pending, decision_maker)
}

/// Cast a spell while another spell or ability is resolving, using the same
/// staged CR 601 transaction as an ordinary priority cast.
///
/// Returning `Ok(None)` means the decision maker surfaced an interactive
/// prompt or cancelled the proposal; in either case the game is restored to
/// the point immediately before the proposal. Internal transaction failures
/// remain errors so callers cannot mistake an incomplete CR 601 transaction
/// for a player cancellation.
#[allow(clippy::too_many_arguments)]
pub(crate) fn cast_spell_from_resolving_effect(
    game: &mut GameState,
    spell_id: ObjectId,
    from_zone: Zone,
    caster: PlayerId,
    casting_method: &CastingMethod,
    base_mana_cost_waived: bool,
    mana_cost_reduction: Option<&crate::mana::ManaCost>,
    provenance: ProvNodeId,
    decision_maker: &mut impl DecisionMaker,
) -> Result<Option<ObjectId>, GameLoopError> {
    cast_spell_from_resolving_effect_with_context(
        game,
        spell_id,
        from_zone,
        caster,
        casting_method,
        base_mana_cost_waived,
        mana_cost_reduction,
        None,
        ironsmith_core::value_model::ManaSpendMode::Normal,
        std::collections::HashMap::new(),
        provenance,
        decision_maker,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn cast_spell_from_resolving_effect_with_context(
    game: &mut GameState,
    spell_id: ObjectId,
    from_zone: Zone,
    caster: PlayerId,
    casting_method: &CastingMethod,
    base_mana_cost_waived: bool,
    mana_cost_reduction: Option<&crate::mana::ManaCost>,
    additional_mana_cost: Option<&crate::mana::ManaCost>,
    mana_spend_mode: ironsmith_core::value_model::ManaSpendMode,
    tagged_objects: std::collections::HashMap<crate::tag::TagKey, Vec<ObjectSnapshot>>,
    provenance: ProvNodeId,
    decision_maker: &mut impl DecisionMaker,
) -> Result<Option<ObjectId>, GameLoopError> {
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut trigger_queue = TriggerQueue::new();
    state.save_checkpoint(game);

    let stack_id = match propose_spell_cast(game, spell_id, from_zone, caster, casting_method) {
        Ok(stack_id) => stack_id,
        Err(error) => {
            state.rollback_action(game);
            return Err(error);
        }
    };
    let effects = game
        .object(stack_id)
        .map(|object| object.spell_effect_owned().unwrap_or_default())
        .unwrap_or_default();
    let optional_costs_paid = game
        .object(stack_id)
        .map(|object| object.optional_costs_paid.clone())
        .unwrap_or_default();
    let requirements = extract_target_requirements_from_program_with_modes(
        game,
        &effects,
        caster,
        Some(stack_id),
        None,
    );
    let mut pending = PendingCast::new(
        stack_id,
        from_zone,
        caster,
        provenance,
        CastStage::ChoosingModes,
        None,
        requirements,
        casting_method.clone(),
        optional_costs_paid,
        None,
        stack_id,
    );
    pending.base_mana_cost_waived = base_mana_cost_waived;
    pending.effect_mana_cost_reduction = mana_cost_reduction.cloned();
    pending.effect_additional_mana_cost = additional_mana_cost.cloned();
    pending.effect_mana_spend_mode = mana_spend_mode;
    pending.tagged_objects = tagged_objects;
    pending.effect_driven = true;

    let mut progress = match check_modes_or_continue(
        game,
        &mut trigger_queue,
        &mut state,
        pending,
        decision_maker,
    ) {
        Ok(progress) => progress,
        Err(error) => {
            state.rollback_action(game);
            if matches!(error, GameLoopError::ActionCancelled(_)) {
                return Ok(None);
            }
            return Err(error);
        }
    };

    loop {
        match progress {
            GameProgress::NeedsDecisionCtx(context) => {
                let next = apply_decision_context_with_dm(
                    game,
                    &mut trigger_queue,
                    &mut state,
                    &context,
                    decision_maker,
                );
                if decision_maker.awaiting_choice() {
                    state.rollback_action(game);
                    return Ok(None);
                }
                progress = match next {
                    Ok(progress) => progress,
                    Err(error) => {
                        state.rollback_action(game);
                        if matches!(error, GameLoopError::ActionCancelled(_)) {
                            return Ok(None);
                        }
                        return Err(error);
                    }
                };
            }
            GameProgress::Continue if state.pending_cast.is_none() => {
                if game
                    .stack
                    .iter()
                    .any(|entry| entry.object_id == stack_id && !entry.is_ability)
                {
                    // Triggers created while casting wait until the resolving
                    // parent finishes. Preserve the already-matched entries for
                    // that outer resolution boundary instead of discarding this
                    // transaction-local queue.
                    game.defer_trigger_entries(trigger_queue.take_all());
                    return Ok(Some(stack_id));
                }
                state.rollback_action(game);
                return Ok(None);
            }
            GameProgress::Continue => {
                state.rollback_action(game);
                return Err(GameLoopError::InvalidState(
                    "effect-driven cast stopped before completing its CR 601 transaction"
                        .to_string(),
                ));
            }
            GameProgress::StackResolved | GameProgress::GameOver(_) => {
                state.rollback_action(game);
                return Err(GameLoopError::InvalidState(
                    "effect-driven cast advanced the outer game loop during resolution".to_string(),
                ));
            }
        }
    }
}

pub(super) fn activation_stage_after_modes(pending: &PendingActivation) -> ActivationStage {
    if !pending.alternative_cost_branches.is_empty() && pending.selected_alternative_cost.is_none()
    {
        ActivationStage::ChoosingAlternativeCost
    } else if pending.activation_cost_has_x && pending.x_value.is_none() {
        ActivationStage::ChoosingX
    } else if pending.hybrid_choices.is_empty() && !pending.pending_hybrid_pips.is_empty() {
        ActivationStage::AnnouncingCost
    } else {
        activation_stage_after_announcements(pending)
    }
}

pub(super) fn assign_pending_activation_cost(
    game: &GameState,
    pending: &mut PendingActivation,
    cost: &crate::cost::TotalCost,
    decision_maker: &mut impl DecisionMaker,
) -> Result<(), GameLoopError> {
    let components = cost.as_all().ok_or_else(|| {
        GameLoopError::InvalidState(
            "an alternative activation cost must be selected before locking payment".to_string(),
        )
    })?;

    pending.mana_cost_to_pay = None;
    pending.remaining_cost_steps.clear();
    pending.display_mana_pips.clear();
    pending.hybrid_choices.clear();
    pending.pending_hybrid_pips.clear();
    append_activation_cost_steps_from_components(components, &mut pending.remaining_cost_steps);

    for component in components {
        if let Some(dynamic_mana) = component.dynamic_mana_cost_ref() {
            let mut execution_ctx =
                ExecutionContext::new(pending.source, pending.activator, &mut *decision_maker)
                    .with_provenance(pending.provenance);
            let resolved = crate::special_actions::resolve_dynamic_mana_cost(
                game,
                dynamic_mana,
                &mut execution_ctx,
            )
            .map_err(|err| {
                GameLoopError::InvalidState(format!(
                    "failed to resolve dynamic activation mana cost: {err:?}"
                ))
            })?;
            pending.mana_cost_to_pay = Some(game.adjust_mana_cost_for_payment_reason(
                pending.activator,
                Some(pending.source),
                &resolved,
                pending.payment_reason,
            ));
            continue;
        }
        if let crate::costs::CostProcessingMode::ManaPayment { cost } = component.processing_mode()
        {
            pending.mana_cost_to_pay = Some(cost);
        }
    }

    pending.activation_cost_has_tap = components.iter().any(|cost| cost.requires_tap());
    pending.activation_cost_has_x = pending
        .mana_cost_to_pay
        .as_ref()
        .is_some_and(crate::mana::ManaCost::has_x)
        || activation_cost_steps_reference_x(&pending.remaining_cost_steps);
    pending.pending_hybrid_pips = pending
        .mana_cost_to_pay
        .as_ref()
        .map(get_pips_requiring_announcement)
        .unwrap_or_default();
    Ok(())
}

/// Check for modal effects on an activated ability and prompt before targets.
///
/// Per MTG rule 602.2b, activated ability modes are announced during activation.
pub(super) fn check_activation_modes_or_continue(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    pending: PendingActivation,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    if pending.chosen_modes.is_none()
        && let Some(modal_spec) = extract_modal_spec_from_program(
            game,
            &pending.effects,
            pending.activator,
            pending.source,
        )
    {
        let player = pending.activator;
        let source = pending.source;
        let effects = pending.effects.all_effects_owned();
        let pending_x_value = pending.x_value.and_then(|x| u32::try_from(x).ok());

        let max_modes = resolve_modal_count_value(
            &modal_spec.max_modes,
            pending_x_value,
            modal_spec.mode_descriptions.len().max(1),
        );
        let min_modes =
            resolve_modal_count_value(&modal_spec.min_modes, pending_x_value, max_modes);

        if !spell_has_legal_targets(game, &effects, player, Some(source)) {
            return Err(GameLoopError::InvalidState(
                "No legal mode/target combination available".to_string(),
            ));
        }

        let mode_options: Vec<crate::decisions::specs::ModeOption> = modal_spec
            .mode_descriptions
            .iter()
            .enumerate()
            .map(|(i, desc)| {
                let legal = spell_has_legal_targets_with_mode_preview(
                    game,
                    &effects,
                    player,
                    Some(source),
                    &[i],
                );
                crate::decisions::specs::ModeOption::with_legality(i, desc.clone(), legal)
            })
            .collect();

        let mut pending = pending;
        pending.stage = ActivationStage::ChoosingModes;
        state.pending_activation = Some(pending);

        return Ok(GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::Modes(
                crate::decisions::context::ModesContext {
                    player,
                    source: Some(source),
                    spell_name: game
                        .object(source)
                        .map(|o| format!("{}'s ability", o.name))
                        .unwrap_or_else(|| "ability".to_string()),
                    spec: crate::decisions::ModesSpec::new(
                        source,
                        mode_options,
                        min_modes,
                        max_modes,
                        modal_spec.allow_repeated_modes,
                        modal_spec.mode_point_costs,
                    ),
                },
            ),
        ));
    }

    let mut pending = pending;
    pending.stage = activation_stage_after_modes(&pending);
    continue_activation(game, trigger_queue, state, pending, decision_maker)
}

fn optional_mana_cost_is_affordable_with_spell_modifiers(
    game: &GameState,
    pending: &PendingCast,
    optional_cost_index: usize,
) -> Option<bool> {
    let spell = game.object(pending.spell_id)?;
    let base_cost = if pending.base_mana_cost_waived {
        crate::mana::ManaCost::new()
    } else {
        crate::decision::spell_mana_cost_for_cast(
            game,
            pending.caster,
            spell,
            &pending.casting_method,
            pending.from_zone,
        )?
    };

    let mut optional_costs_paid = pending.optional_costs_paid.clone();
    optional_costs_paid.pay_times(optional_cost_index, 1);

    let mut hypothetical_spell = spell.clone();
    hypothetical_spell.optional_costs_paid = optional_costs_paid.clone();
    let combined_cost = mana_cost_with_paid_optional_and_splice_costs(
        &base_cost,
        &hypothetical_spell,
        &optional_costs_paid,
        &pending.splice_costs,
        pending.chosen_modes.as_deref(),
    );
    let combined_cost = mana_cost_with_effect_additional_cost(
        &combined_cost,
        pending.effect_additional_mana_cost.as_ref(),
    );
    let mut effective_cost =
        crate::decision::calculate_effective_mana_cost_for_payment_with_chosen_targets_for_casting_method_from_zone(
            game,
            pending.caster,
            &hypothetical_spell,
            &combined_cost,
            &pending.chosen_targets,
            &pending.casting_method,
            pending.from_zone,
        );
    if let Some(reduction) = pending.effect_mana_cost_reduction.as_ref() {
        effective_cost = crate::decision::reduce_mana_cost(&effective_cost, reduction);
    }

    Some(crate::decision::can_potentially_pay(
        game,
        pending.caster,
        &effective_cost,
        pending.x_value.unwrap_or(0),
    ))
}

fn optional_cost_is_affordable_for_pending(
    game: &GameState,
    pending: &PendingCast,
    optional_cost_index: usize,
) -> bool {
    let Some(optional_cost) = game
        .object(pending.spell_id)
        .and_then(|spell| spell.optional_costs.get(optional_cost_index))
    else {
        return false;
    };
    if let Some(mana_cost) = optional_cost.cost.mana_cost() {
        optional_mana_cost_is_affordable_with_spell_modifiers(game, pending, optional_cost_index)
            .unwrap_or_else(|| {
                let adjusted_cost = game.adjust_mana_cost_for_payment_reason(
                    pending.caster,
                    Some(pending.spell_id),
                    mana_cost,
                    crate::costs::PaymentReason::CastSpell,
                );
                crate::decision::can_potentially_pay(game, pending.caster, &adjusted_cost, 0)
            })
    } else {
        crate::cost::can_pay_cost_with_reason(
            game,
            pending.spell_id,
            pending.caster,
            &optional_cost.cost,
            crate::costs::PaymentReason::CastSpell,
        )
        .is_ok()
    }
}

fn conditional_mode_range_for_pending(
    game: &GameState,
    pending: &PendingCast,
    modal_spec: &crate::effects::ModalSpec,
) -> Option<(usize, usize, usize)> {
    let range = modal_spec.conditional_mode_range.as_ref()?;
    let optional_cost_index = game
        .object(pending.spell_id)?
        .optional_costs
        .iter()
        .position(|cost| cost.cost_ref().matches_query(&range.required_optional_cost))?;
    optional_cost_is_affordable_for_pending(game, pending, optional_cost_index).then(|| {
        let max_modes = resolve_modal_count_value(
            &range.max_modes,
            pending.x_value,
            modal_spec.mode_descriptions.len(),
        );
        let min_modes = resolve_modal_count_value(&range.min_modes, pending.x_value, max_modes);
        (optional_cost_index, min_modes, max_modes)
    })
}

fn mode_point_total(modal_spec: &crate::effects::ModalSpec, modes: &[usize]) -> Option<usize> {
    let mut seen = std::collections::HashSet::new();
    let mut total = 0usize;
    for mode in modes {
        if *mode >= modal_spec.mode_descriptions.len()
            || (!modal_spec.allow_repeated_modes && !seen.insert(*mode))
        {
            return None;
        }
        total = total.saturating_add(
            modal_spec
                .mode_point_costs
                .get(*mode)
                .copied()
                .unwrap_or(1)
                .max(1) as usize,
        );
    }
    Some(total)
}

/// Validate a mode selection against both its ordinary range and any CR 601.4
/// range enabled by a later optional cost. Returns that required cost's index.
pub(super) fn cast_mode_selection_required_optional_cost(
    game: &GameState,
    pending: &PendingCast,
    modes: &[usize],
) -> Result<Option<usize>, GameLoopError> {
    let modal_spec = extract_modal_spec_from_spell(game, pending.spell_id, pending.caster)
        .ok_or_else(|| GameLoopError::InvalidState("spell has no modal proposal".to_string()))?;
    let total = mode_point_total(&modal_spec, modes).ok_or_else(|| {
        GameLoopError::ActionCancelled(
            "mode selection contains an invalid or duplicate mode".to_string(),
        )
    })?;
    let base_max = resolve_modal_count_value(
        &modal_spec.max_modes,
        pending.x_value,
        modal_spec.mode_descriptions.len().max(1),
    );
    let base_min = resolve_modal_count_value(&modal_spec.min_modes, pending.x_value, base_max);
    if (base_min..=base_max).contains(&total) {
        return Ok(None);
    }
    if let Some((optional_cost_index, conditional_min, conditional_max)) =
        conditional_mode_range_for_pending(game, pending, &modal_spec)
        && (conditional_min..=conditional_max).contains(&total)
    {
        return Ok(Some(optional_cost_index));
    }
    Err(GameLoopError::ActionCancelled(
        "mode selection has no legal joint optional-cost proposal".to_string(),
    ))
}

/// Check for optional costs and either prompt for them or continue to targeting/finalization.
///
/// This is called after modes and before the value of X is chosen.
/// Returns the next decision needed or continues the cast.
pub(super) fn check_optional_costs_or_continue(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    mut pending: PendingCast,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    // X and other announcement metadata can mutate the stack object before
    // re-entering this stage. Start cast-time cost discovery from one clean
    // state so its first characteristics query uses the batched game cache.
    game.refresh_continuous_state();
    if ensure_granted_conspire_optional_costs(game, &mut pending) {
        // Conspire discovery mutates the stack object; optional-life discovery
        // immediately performs another derived-characteristics query.
        game.refresh_continuous_state();
    }
    if ensure_optional_life_cost_reduction_costs(game, &mut pending) {
        // Keep later affordability and target queries on the clean path, while
        // avoiding a refresh when no new optional costs were appended.
        game.refresh_continuous_state();
    }

    // Check if the spell has optional costs
    let optional_costs = if let Some(obj) = game.object(pending.spell_id) {
        obj.optional_costs.clone()
    } else {
        Vec::new().into()
    };

    if optional_costs.is_empty() {
        // CR 601.2b announces variable values only after modes and alternative/
        // additional costs have been chosen.
        check_x_or_continue(game, trigger_queue, state, pending, decision_maker)
    } else {
        // Build the optional cost options for the decision
        let player = pending.caster;
        let source = pending.spell_id;

        // Check which costs the player can afford (using potential mana)
        let mut options: Vec<OptionalCostOption> = optional_costs
            .iter()
            .enumerate()
            .map(|(index, opt_cost)| {
                let affordable = optional_cost_is_affordable_for_pending(game, &pending, index);

                // Format the cost description
                let cost_description = if let Some(mana) = opt_cost.cost.mana_cost() {
                    format!("{}", mana.mana_value())
                } else {
                    "special".to_string()
                };

                OptionalCostOption {
                    index,
                    label: opt_cost.display_label(),
                    repeatable: opt_cost.repeatable,
                    affordable,
                    cost_description,
                }
            })
            .collect();
        options.sort_by_key(|option| {
            !pending
                .required_optional_cost_indices
                .contains(&option.index)
        });

        // Set up pending cast for optional costs stage
        let mut pending = pending;
        let required_optional_cost_count = pending.required_optional_cost_indices.len();
        pending.stage = CastStage::ChoosingOptionalCosts;
        state.pending_cast = Some(pending);

        // Convert to SelectOptionsContext for optional cost selection
        let selectable_options: Vec<crate::decisions::context::SelectableOption> = options
            .iter()
            .map(|opt| {
                crate::decisions::context::SelectableOption::with_legality(
                    opt.index,
                    format!("{}: {}", opt.label, opt.cost_description),
                    opt.affordable,
                )
            })
            .collect();
        let spell_name = game
            .object(source)
            .map(|o| o.name.to_string())
            .unwrap_or_else(|| "spell".to_string());
        let ctx = crate::decisions::context::SelectOptionsContext::new(
            player,
            Some(source),
            format!("Choose optional costs for {}", spell_name),
            selectable_options,
            required_optional_cost_count,
            if options.iter().any(|opt| opt.repeatable) {
                64
            } else {
                options.len()
            },
        );
        Ok(GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::SelectOptions(ctx),
        ))
    }
}

/// Get the effective mana cost for a spell being cast.
///
/// This is called during casting to determine hybrid/Phyrexian pips.
pub(super) fn get_spell_mana_cost(
    game: &GameState,
    spell_id: ObjectId,
    caster: PlayerId,
    casting_method: &CastingMethod,
    from_zone: Zone,
) -> Option<crate::mana::ManaCost> {
    let obj = game.object(spell_id)?;
    crate::decision::spell_mana_cost_for_cast(game, caster, obj, casting_method, from_zone)
}

/// Get pips that require announcement (hybrid/Phyrexian pips with multiple options).
///
/// Returns a list of (pip_index, alternatives) for each pip that has multiple payment options.
/// Per MTG rule 601.2b, the player must announce how they will pay these during casting.
pub(super) fn get_pips_requiring_announcement(
    cost: &crate::mana::ManaCost,
) -> Vec<(usize, Vec<crate::mana::ManaSymbol>)> {
    cost.pips()
        .iter()
        .enumerate()
        .filter(|(_, pip)| pip.len() > 1) // Multiple options = needs choice
        .map(|(i, pip)| (i, pip.clone()))
        .collect()
}

/// Continue the CR 601.2b announcement sequence after modes and costs.
///
/// The previous cast flow prompted for X before modes and skipped modal
/// selection entirely on modal X spells. Keep X behind the earlier
/// announcements and preserve the pending proposal while the choice is made.
pub(super) fn check_x_or_continue(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    mut pending: PendingCast,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    if pending.base_mana_cost_waived {
        if game
            .object(pending.spell_id)
            .and_then(|spell| spell.mana_cost.as_ref())
            .is_some_and(|cost| cost.has_x())
        {
            pending.x_value = Some(0);
            if let Some(spell) = game.object_mut(pending.spell_id) {
                spell.x_value = Some(0);
            }
        }
        return continue_to_targeting_or_finalize(
            game,
            trigger_queue,
            state,
            pending,
            decision_maker,
        );
    }

    let mana_cost = get_spell_mana_cost(
        game,
        pending.spell_id,
        pending.caster,
        &pending.casting_method,
        pending.from_zone,
    );
    let (needs_x, min_x, max_x) = compute_spell_cast_x_bounds(
        game,
        pending.caster,
        pending.spell_id,
        &pending.casting_method,
        mana_cost.as_ref(),
    );

    if needs_x && pending.x_value.is_none() {
        pending.stage = CastStage::ChoosingX;
        let player = pending.caster;
        let source = pending.spell_id;
        state.pending_cast = Some(pending);
        return Ok(GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::Number(
                crate::decisions::context::NumberContext::x_value_with_min(
                    player, source, min_x, max_x,
                ),
            ),
        ));
    }

    continue_to_targeting_or_finalize(game, trigger_queue, state, pending, decision_maker)
}

/// Continue the casting process to targeting or mana payment.
///
/// Called when there are no optional costs or after optional costs are chosen.
/// Per MTG rule 601.2b, checks for hybrid/Phyrexian pips first.
pub(super) fn continue_to_targeting_or_finalize(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    pending: PendingCast,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    // Per MTG 601.2b: Check for hybrid/Phyrexian pips that need announcement BEFORE targets
    // Skip if we already have hybrid choices (coming back from AnnouncingCost stage)
    if pending.hybrid_choices.is_empty()
        && let Some(mana_cost) = announced_spell_mana_cost(game, &pending)
    {
        let pips_to_announce = get_pips_requiring_announcement(&mana_cost);
        if !pips_to_announce.is_empty() {
            // Need to announce hybrid/Phyrexian choices
            return check_hybrid_announcement_or_continue(
                game,
                trigger_queue,
                state,
                pending,
                pips_to_announce,
                decision_maker,
            );
        }
    }

    // No hybrid/Phyrexian pips (or already announced), continue to targets
    continue_to_targets_or_mana_payment(game, trigger_queue, state, pending, decision_maker)
}

/// Check for hybrid/Phyrexian pips and prompt for announcements.
///
/// Per MTG rule 601.2b, the player announces how they will pay hybrid/Phyrexian costs
/// before targets are chosen.
pub(super) fn check_hybrid_announcement_or_continue(
    game: &mut GameState,
    _trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    pending: PendingCast,
    pips_to_announce: Vec<(usize, Vec<crate::mana::ManaSymbol>)>,
    _decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    let mut pending = pending;
    pending.stage = CastStage::AnnouncingCost;
    pending.pending_hybrid_pips = pips_to_announce;

    // Prompt for the first pip
    prompt_for_next_hybrid_pip(game, state, pending)
}

/// Prompt the player for the next hybrid/Phyrexian pip choice.
pub(super) fn prompt_for_next_hybrid_pip(
    game: &GameState,
    state: &mut PriorityLoopState,
    pending: PendingCast,
) -> Result<GameProgress, GameLoopError> {
    // Get the next pip to announce
    if let Some((pip_idx, alternatives)) = pending.pending_hybrid_pips.first().cloned() {
        let player = pending.caster;
        let source = pending.spell_id;
        let spell_name = game
            .object(source)
            .map(|o| o.name.to_string())
            .unwrap_or_else(|| "spell".to_string());

        // Build hybrid options for each alternative
        let options: Vec<crate::decisions::context::HybridOption> = alternatives
            .iter()
            .enumerate()
            .map(|(i, sym)| crate::decisions::context::HybridOption {
                index: i,
                label: format_mana_symbol_for_choice(sym),
                symbol: *sym,
            })
            .collect();

        state.pending_cast = Some(pending);

        // Create a HybridChoice decision for this pip
        let ctx = crate::decisions::context::HybridChoiceContext::new(
            player,
            Some(source),
            spell_name,
            pip_idx + 1, // 1-based for display
            options,
        );
        Ok(GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::HybridChoice(ctx),
        ))
    } else {
        // No more pips to announce - this shouldn't happen, but handle gracefully
        state.pending_cast = Some(pending);
        Err(GameLoopError::InvalidState(
            "No pending hybrid pips to announce".to_string(),
        ))
    }
}

/// Format a mana symbol for display in hybrid/Phyrexian choice.
pub(super) fn format_mana_symbol_for_choice(sym: &crate::mana::ManaSymbol) -> String {
    use crate::mana::ManaSymbol;
    match sym {
        ManaSymbol::White => "{W} (White mana)".to_string(),
        ManaSymbol::Blue => "{U} (Blue mana)".to_string(),
        ManaSymbol::Black => "{B} (Black mana)".to_string(),
        ManaSymbol::Red => "{R} (Red mana)".to_string(),
        ManaSymbol::Green => "{G} (Green mana)".to_string(),
        ManaSymbol::Colorless => "{C} (Colorless mana)".to_string(),
        ManaSymbol::Generic(n) => format!("{{{}}} ({} generic mana)", n, n),
        ManaSymbol::Snow => "{S} (Snow mana)".to_string(),
        ManaSymbol::Life(n) => format!("{} life (Phyrexian)", n),
        ManaSymbol::X => "{X}".to_string(),
    }
}

/// Continue to target selection or mana payment.
///
/// Called after hybrid/Phyrexian choices are made (or when none are needed).
fn target_chooser_candidates(
    game: &GameState,
    controller: PlayerId,
    source: ObjectId,
    chooser: &crate::target::PlayerFilter,
) -> Vec<PlayerId> {
    let filter_ctx = game
        .filter_context_for(controller, Some(source))
        .with_active_player(game.turn.active_player);
    game.players
        .iter()
        .filter(|player| player.is_in_game())
        .filter_map(|player| {
            crate::filter::player_filter_matches_game(chooser, player.id, game, &filter_ctx)
                .then_some(player.id)
        })
        .collect()
}

fn target_chooser_context(
    game: &GameState,
    controller: PlayerId,
    source: ObjectId,
    subject: String,
    candidates: &[PlayerId],
) -> crate::decisions::context::SelectOptionsContext {
    let options = candidates
        .iter()
        .enumerate()
        .map(|(index, player)| {
            crate::decisions::context::SelectableOption::new(
                index,
                game.player(*player)
                    .map(|candidate| candidate.name.to_string())
                    .unwrap_or_else(|| format!("Player {}", player.0)),
            )
        })
        .collect();
    crate::decisions::context::SelectOptionsContext::new(
        controller,
        Some(source),
        format!("Choose a player to choose a target for {subject}"),
        options,
        1,
        1,
    )
}

fn resolved_next_target_chooser(
    game: &GameState,
    controller: PlayerId,
    source: ObjectId,
    requirement: &TargetRequirement,
) -> Result<Result<PlayerId, Vec<PlayerId>>, GameLoopError> {
    let Some(filter) = requirement.chooser.as_ref() else {
        return Ok(Ok(controller));
    };
    let candidates = target_chooser_candidates(game, controller, source, filter);
    match candidates.as_slice() {
        [chooser] => Ok(Ok(*chooser)),
        [] => Err(GameLoopError::InvalidState(
            "No eligible player can make the delegated target choice".to_string(),
        )),
        _ => Ok(Err(candidates)),
    }
}

fn specialize_target_requirement_for_chooser(
    game: &GameState,
    controller: PlayerId,
    source: ObjectId,
    chooser: PlayerId,
    requirement: &mut TargetRequirement,
) {
    requirement.spec =
        super::targeting::specialize_iterated_player_choose_spec(&requirement.spec, chooser);
    requirement.legal_targets =
        compute_legal_targets(game, &requirement.spec, controller, Some(source));
    requirement.legal_target_sets = crate::targeting::legal_target_sets_for_spec(
        game,
        &requirement.spec,
        &requirement.legal_targets,
    );
    requirement.aggregate_constraint = crate::targeting::resolved_target_aggregate_constraint(
        game,
        &requirement.spec,
        controller,
        Some(source),
        &requirement.legal_targets,
    );
}

pub(super) fn continue_to_targets_or_mana_payment(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    pending: PendingCast,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    // Validate that we can still pay the cost after hybrid choices
    // This is necessary because max_x was calculated assuming life payment for Phyrexian pips,
    // but the player may have chosen mana payment instead
    if let Some(ref cost) = pending.mana_cost_to_pay {
        let x_value = pending.x_value.unwrap_or(0);
        let expanded_pips =
            expand_mana_cost_to_pips(cost, x_value as usize, &pending.hybrid_choices);
        let potential = compute_potential_mana(game, pending.caster);

        // Check if we can pay all the expanded pips (excluding life payments)
        let total_mana_needed: usize = expanded_pips
            .iter()
            .filter(|pip| {
                !pip.iter()
                    .any(|s| matches!(s, crate::mana::ManaSymbol::Life(_)))
            })
            .count();

        if potential.total() < total_mana_needed as u32 {
            return Err(GameLoopError::InvalidState(format!(
                "Cannot afford spell: need {} mana but only have {} available. \
                Consider paying life for Phyrexian mana or choosing a lower X value.",
                total_mana_needed,
                potential.total()
            )));
        }
    }

    if pending.remaining_requirements.is_empty() {
        // No targets needed, go to mana payment
        continue_to_mana_payment(
            game,
            trigger_queue,
            state,
            pending,
            Vec::new(),
            decision_maker,
        )
    } else {
        // Need to select targets
        let mut pending = pending;
        let requirement = pending.remaining_requirements[0].clone();
        let player = pending.caster;
        let source = pending.spell_id;
        let context = game
            .object(source)
            .map(|o| o.name.to_string())
            .unwrap_or_else(|| "spell".to_string());

        let chooser = match resolved_next_target_chooser(game, player, source, &requirement)? {
            Ok(chooser) => chooser,
            Err(candidates) => {
                pending.stage = CastStage::ChoosingTargetChooser;
                pending.pending_target_chooser_candidates = candidates.clone();
                let ctx = target_chooser_context(game, player, source, context, &candidates);
                state.pending_cast = Some(pending);
                return Ok(GameProgress::NeedsDecisionCtx(
                    crate::decisions::context::DecisionContext::SelectOptions(ctx),
                ));
            }
        };
        let requirement_count = pending
            .remaining_requirements
            .iter()
            .take_while(|candidate| {
                matches!(
                    resolved_next_target_chooser(game, player, source, candidate),
                    Ok(Ok(candidate_chooser)) if candidate_chooser == chooser
                )
            })
            .count();
        for requirement in pending
            .remaining_requirements
            .iter_mut()
            .take(requirement_count)
        {
            specialize_target_requirement_for_chooser(game, player, source, chooser, requirement);
        }
        let requirements = pending.remaining_requirements[..requirement_count].to_vec();
        pending.stage = CastStage::ChoosingTargets;
        pending.active_target_requirement_count = requirements.len();

        state.pending_cast = Some(pending);

        // Convert to TargetsContext
        let ctx = crate::decisions::context::TargetsContext::new(
            chooser,
            source,
            context,
            requirements
                .into_iter()
                .map(|r| crate::decisions::context::TargetRequirementContext {
                    description: r.description,
                    legal_targets: r.legal_targets,
                    legal_target_sets: r.legal_target_sets,
                    aggregate_constraint: r.aggregate_constraint,
                    min_targets: r.min_targets,
                    max_targets: r.max_targets,
                    distinct_player_group: r.distinct_player_group,
                })
                .collect(),
        );
        Ok(GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::Targets(ctx),
        ))
    }
}

pub(super) fn finalize_pending_spell_cast(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    mut pending: PendingCast,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    // Announcement-time events (notably revealing splice cards) become
    // triggers only after the proposal survives legality and payment. Until
    // this point they remain in GameState so CR 729 rollback erases them with
    // the rest of an illegal proposal.
    drain_pending_trigger_events(game, trigger_queue);
    let effect_driven = pending.effect_driven;
    let base_mana_cost_waived = pending.base_mana_cost_waived;
    let mana_spent_to_cast = pending.mana_spent_to_cast.clone();
    let assist_mana_spent_to_cast = pending
        .assist_player
        .filter(|_| pending.assist_mana_spent_to_cast.total() > 0)
        .map(|player| (player, pending.assist_mana_spent_to_cast.clone()));
    for _ in pending
        .hybrid_choices
        .iter()
        .filter(|(_, symbol)| matches!(symbol, crate::mana::ManaSymbol::Life(_)))
    {
        pending
            .optional_costs_paid
            .mark_label_paid("CompleatedLifePaid");
    }
    if game.is_active_player(pending.caster)
        && matches!(
            game.turn.phase,
            crate::game_state::Phase::FirstMain | crate::game_state::Phase::NextMain
        )
    {
        pending
            .optional_costs_paid
            .mark_label_paid("CastDuringYourMainPhase");
    }
    let spell_cast_provenance =
        game.alloc_child_event_provenance(pending.provenance, crate::events::EventKind::SpellCast);
    let result = finalize_spell_cast(
        game,
        trigger_queue,
        state,
        pending.spell_id,
        pending.from_zone,
        pending.caster,
        pending.chosen_targets,
        pending.chosen_target_assignments,
        pending.target_distributions,
        pending.x_value,
        pending.casting_method,
        pending.optional_costs_paid,
        pending.chosen_modes,
        pending.spliced_cards,
        mana_spent_to_cast,
        assist_mana_spent_to_cast,
        pending.keyword_payment_contributions,
        pending.tagged_objects,
        pending.effect_outcomes,
        &mut pending.payment_trace,
        true,
        base_mana_cost_waived,
        pending.stack_id,
        spell_cast_provenance,
        &mut *decision_maker,
    )?;

    if effect_driven {
        // The resolving effect reports the SpellCast event through its own
        // EffectOutcome. The parent spell is still resolving, so do not reset
        // priority or advance the normal priority loop here.
        state.clear_checkpoint();
        return Ok(GameProgress::Continue);
    }

    let event = if let Some(obj) = game.object(result.new_id) {
        let snapshot = crate::snapshot::ObjectSnapshot::from_object(obj, game);
        TriggerEvent::new_with_provenance(
            SpellCastEvent::new_with_snapshot(
                result.new_id,
                result.caster,
                result.from_zone,
                snapshot,
            ),
            spell_cast_provenance,
        )
    } else {
        TriggerEvent::new_with_provenance(
            SpellCastEvent::new(result.new_id, result.caster, result.from_zone),
            spell_cast_provenance,
        )
    };
    queue_triggers_from_event(game, trigger_queue, event, false);

    state.clear_checkpoint();
    reset_priority(game, &mut state.tracker);
    advance_priority_with_dm(game, trigger_queue, decision_maker)
}

pub(super) fn continue_spell_next_cost_or_finalize(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    mut pending: PendingCast,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    if pending.mana_cost_to_pay.is_some() && pending.pending_mana_payment.is_none() {
        return begin_spell_mana_payment(game, trigger_queue, state, pending, decision_maker);
    }
    auto_pay_spell_tap_cost_steps(game, trigger_queue, &mut pending, decision_maker)?;
    pending.stage = spell_stage_after_targets(&pending);
    let option_count =
        usize::from(pending.mana_cost_to_pay.is_some()) + pending.remaining_cost_steps.len();

    if option_count == 1 {
        if pending.mana_cost_to_pay.is_some() {
            let payment = pending.pending_mana_payment.take().ok_or_else(|| {
                GameLoopError::InvalidState(
                    "spell mana sources were not prepared before cost payment".to_string(),
                )
            })?;
            return commit_prepared_spell_mana_payment(
                game,
                trigger_queue,
                state,
                pending,
                payment,
                decision_maker,
            );
        }

        pending.stage = CastStage::ProcessingCosts;
        return continue_spell_cost_payment(game, trigger_queue, state, pending, decision_maker);
    }

    match pending.stage {
        CastStage::ChoosingNextCost => {
            let source_name = game
                .object(pending.spell_id)
                .map(|o| o.name.to_string())
                .unwrap_or_else(|| "spell".to_string());
            let ctx = build_next_cost_context(
                pending.caster,
                pending.spell_id,
                source_name,
                pending.mana_cost_to_pay.as_ref(),
                pending.pending_mana_payment.is_some(),
                &pending.remaining_cost_steps,
            );
            state.pending_cast = Some(pending);
            Ok(GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::SelectOptions(ctx),
            ))
        }
        CastStage::ReadyToFinalize => {
            finalize_pending_spell_cast(game, trigger_queue, state, pending, decision_maker)
        }
        other => Err(GameLoopError::InvalidState(format!(
            "unexpected spell payment stage {other}"
        ))),
    }
}

/// Enter the mana component of CR 601.2h after the player selects it from the
/// remaining total-cost components. Assist setup is part of that component, so
/// choosing or paying a non-mana cost never commits mana early.
pub(super) fn begin_spell_mana_payment(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    pending: PendingCast,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    let can_assist = pending.mana_cost_to_pay.as_ref().is_some_and(|cost| {
        assist_payable_generic_total(cost, pending.x_value.unwrap_or(0)) > 0
            && game.current_has_static_ability_id(
                pending.spell_id,
                crate::static_abilities::StaticAbilityId::Assist,
            )
    });
    if can_assist && !pending.assist_player_choice_made {
        return prompt_spell_assist_player(game, state, pending);
    }
    if pending.assist_player.is_some() && !pending.assist_payment_complete {
        return prompt_spell_assist_contribution(game, state, pending);
    }
    prompt_spell_mana_ability_window(game, trigger_queue, state, pending, decision_maker)
}

pub(super) fn eligible_assist_players(game: &GameState, caster: PlayerId) -> Vec<PlayerId> {
    game.turn_store
        .turn_order
        .iter()
        .copied()
        .filter(|player| *player != caster && game.player(*player).is_some())
        .collect()
}

pub(super) fn assist_payable_generic_total(cost: &crate::mana::ManaCost, x_value: u32) -> u32 {
    let x_pips = cost
        .pips()
        .iter()
        .filter(|pip| {
            pip.iter()
                .any(|symbol| matches!(symbol, crate::mana::ManaSymbol::X))
        })
        .count() as u32;
    cost.generic_mana_total()
        .saturating_add(x_pips.saturating_mul(x_value))
}

pub(super) fn prompt_spell_assist_player(
    game: &GameState,
    state: &mut PriorityLoopState,
    mut pending: PendingCast,
) -> Result<GameProgress, GameLoopError> {
    let caster_can_pay = spell_mana_payment_request(game, &pending)
        .is_ok_and(|request| crate::mana_payment::plan_mana_payment(game, &request).is_ok());
    let mut options = vec![crate::decisions::context::SelectableOption::with_legality(
        0,
        "Do not choose a player to assist",
        caster_can_pay,
    )];
    for (offset, player) in eligible_assist_players(game, pending.caster)
        .into_iter()
        .enumerate()
    {
        let name = game
            .player(player)
            .map(|candidate| candidate.name.clone())
            .unwrap_or_else(|| format!("Player {}", player.0));
        let maximum = pending
            .mana_cost_to_pay
            .as_ref()
            .map(|cost| assist_payable_generic_total(cost, pending.x_value.unwrap_or(0)))
            .unwrap_or(0);
        let can_complete = (1..=maximum)
            .any(|amount| assist_generic_contribution_is_legal(game, &pending, player, amount));
        options.push(crate::decisions::context::SelectableOption::with_legality(
            offset + 1,
            format!("Choose {name} to assist"),
            can_complete,
        ));
    }

    pending.stage = CastStage::ChoosingAssistPlayer;
    let caster = pending.caster;
    let source = pending.spell_id;
    let spell_name = game
        .object(source)
        .map(|spell| spell.name.to_string())
        .unwrap_or_else(|| "spell".to_string());
    state.pending_cast = Some(pending);
    Ok(GameProgress::NeedsDecisionCtx(
        crate::decisions::context::DecisionContext::SelectOptions(
            crate::decisions::context::SelectOptionsContext::new(
                caster,
                Some(source),
                format!("Choose another player to assist with {spell_name}"),
                options,
                1,
                1,
            ),
        ),
    ))
}

pub(super) fn max_assist_generic_contribution(game: &GameState, pending: &PendingCast) -> u32 {
    let Some(assistant) = pending.assist_player else {
        return 0;
    };
    let maximum = pending
        .mana_cost_to_pay
        .as_ref()
        .map(|cost| assist_payable_generic_total(cost, pending.x_value.unwrap_or(0)))
        .unwrap_or(0);
    (1..=maximum)
        .rev()
        .find(|amount| assist_generic_contribution_is_legal(game, pending, assistant, *amount))
        .unwrap_or(0)
}

pub(super) fn assist_generic_contribution_is_legal(
    game: &GameState,
    pending: &PendingCast,
    assistant: PlayerId,
    amount: u32,
) -> bool {
    let maximum = pending
        .mana_cost_to_pay
        .as_ref()
        .map(|cost| assist_payable_generic_total(cost, pending.x_value.unwrap_or(0)))
        .unwrap_or(0);
    if amount > maximum {
        return false;
    }
    if amount > 0 {
        let assistant_request = crate::mana_payment::ManaPaymentRequest::new(
            assistant,
            pending.spell_id,
            crate::costs::PaymentReason::CastSpell,
            crate::mana::ManaCost::new().add_generic(amount),
        )
        .with_spend_policy(game.mana_spend_policy(assistant, Some(pending.spell_id)));
        if crate::mana_payment::plan_mana_payment(game, &assistant_request).is_err() {
            return false;
        }
    }
    let mut caster_pending = pending.clone();
    caster_pending.assist_generic_contribution = amount;
    spell_mana_payment_request(game, &caster_pending)
        .is_ok_and(|request| crate::mana_payment::plan_mana_payment(game, &request).is_ok())
}

pub(super) fn prompt_spell_assist_payment_plan(
    game: &mut GameState,
    state: &mut PriorityLoopState,
    mut pending: PendingCast,
) -> Result<GameProgress, GameLoopError> {
    let assistant = pending.assist_player.ok_or_else(|| {
        GameLoopError::InvalidState("Assist payment has no chosen player".to_string())
    })?;
    let cost = crate::mana::ManaCost::new().add_generic(pending.assist_generic_contribution);
    let mut request = crate::mana_payment::ManaPaymentRequest::new(
        assistant,
        pending.spell_id,
        crate::costs::PaymentReason::CastSpell,
        cost,
    )
    .with_spend_policy(game.mana_spend_policy(assistant, Some(pending.spell_id)));
    if let Some(existing) = pending.pending_mana_payment.as_ref() {
        request.preferences = existing.request.preferences.clone();
    }
    let plan = crate::mana_payment::plan_mana_payment(game, &request)
        .map_err(|failure| {
            state.rollback_action(game);
            GameLoopError::ActionCancelled(format!(
                "the announced Assist contribution has no legal payment plan: {failure:?}"
            ))
        })?
        .into_iter()
        .next()
        .ok_or_else(|| {
            GameLoopError::InvalidState("planner returned no Assist payment plan".to_string())
        })?;
    pending.display_mana_pips =
        expand_mana_cost_to_display_pips(&request.cost, request.x_value as usize);
    pending.pending_mana_payment = Some(crate::mana_payment::PendingManaPayment::new(
        request.clone(),
        plan.clone(),
    ));
    pending.stage = CastStage::PayingAssistMana;
    let subject = game
        .object(pending.spell_id)
        .map(|spell| format!("Assist payment for {}", spell.name))
        .unwrap_or_else(|| "Assist payment".to_string());
    state.pending_cast = Some(pending);
    Ok(GameProgress::NeedsDecisionCtx(
        crate::decisions::context::DecisionContext::ManaPayment(
            crate::decisions::context::ManaPaymentContext::new(
                assistant,
                request.source,
                subject,
                request,
                plan,
            ),
        ),
    ))
}

pub(super) fn prompt_spell_assist_contribution(
    game: &GameState,
    state: &mut PriorityLoopState,
    mut pending: PendingCast,
) -> Result<GameProgress, GameLoopError> {
    let assistant = pending.assist_player.ok_or_else(|| {
        GameLoopError::InvalidState("Assist contribution has no chosen player".to_string())
    })?;
    let maximum = pending
        .mana_cost_to_pay
        .as_ref()
        .map(|cost| assist_payable_generic_total(cost, pending.x_value.unwrap_or(0)))
        .unwrap_or(0);
    let options = (0..=maximum)
        .map(|amount| {
            crate::decisions::context::SelectableOption::with_legality(
                amount as usize,
                if amount == 0 {
                    "Pay no mana with assist".to_string()
                } else {
                    format!("Pay {amount} generic mana with assist")
                },
                assist_generic_contribution_is_legal(game, &pending, assistant, amount),
            )
        })
        .collect::<Vec<_>>();
    pending.stage = CastStage::ChoosingAssistContribution;
    let source = pending.spell_id;
    let subject = game
        .object(source)
        .map(|spell| spell.name.to_string())
        .unwrap_or_else(|| "spell".to_string());
    state.pending_cast = Some(pending);
    Ok(GameProgress::NeedsDecisionCtx(
        crate::decisions::context::DecisionContext::SelectOptions(
            crate::decisions::context::SelectOptionsContext::new(
                assistant,
                Some(source),
                format!("Choose how much generic mana to pay for {subject}"),
                options,
                1,
                1,
            ),
        ),
    ))
}

pub(super) fn spell_mana_payment_request(
    game: &GameState,
    pending: &PendingCast,
) -> Result<crate::mana_payment::ManaPaymentRequest, GameLoopError> {
    let cost = pending.mana_cost_to_pay.as_ref().ok_or_else(|| {
        GameLoopError::InvalidState("spell payment prompt has no mana cost".to_string())
    })?;
    let mut payment_pips = expand_mana_cost_to_pips(
        cost,
        pending.x_value.unwrap_or(0) as usize,
        &pending.hybrid_choices,
    );
    for _ in 0..pending.assist_generic_contribution {
        if let Some(index) = payment_pips
            .iter()
            .rposition(|pip| pip.as_slice() == [crate::mana::ManaSymbol::Generic(1)])
        {
            payment_pips.remove(index);
        }
    }
    let locked_cost = crate::mana::ManaCost::from_pips(payment_pips);
    let mut spend_policy = game.mana_spend_policy(pending.caster, Some(pending.spell_id));
    spend_policy.allow_mode(pending.effect_mana_spend_mode);
    let mut request = crate::mana_payment::ManaPaymentRequest::new(
        pending.caster,
        pending.spell_id,
        crate::costs::PaymentReason::CastSpell,
        locked_cost,
    )
    .with_spend_policy(spend_policy);
    request.allow_black_life = crate::decision::mana_cost_has_black_symbol(&request.cost)
        && game.player_can_pay_black_with_life_for_reason(
            pending.caster,
            Some(pending.spell_id),
            crate::costs::PaymentReason::CastSpell,
        );
    if let Some(existing) = pending.pending_mana_payment.as_ref() {
        request.preferences = existing.request.preferences.clone();
    }
    Ok(request)
}

pub(super) fn spell_mana_payment_is_legal(game: &GameState, pending: &PendingCast) -> bool {
    if spell_mana_payment_request(game, pending)
        .is_ok_and(|request| crate::mana_payment::plan_mana_payment(game, &request).is_ok())
    {
        return true;
    }
    let maximum = pending
        .mana_cost_to_pay
        .as_ref()
        .map(|cost| assist_payable_generic_total(cost, pending.x_value.unwrap_or(0)))
        .unwrap_or(0);
    if maximum == 0
        || !game.current_has_static_ability_id(
            pending.spell_id,
            crate::static_abilities::StaticAbilityId::Assist,
        )
    {
        return false;
    }
    let assistants = if pending.assist_player_choice_made {
        pending.assist_player.into_iter().collect::<Vec<_>>()
    } else {
        eligible_assist_players(game, pending.caster)
    };
    assistants.into_iter().any(|assistant| {
        (1..=maximum)
            .any(|amount| assist_generic_contribution_is_legal(game, pending, assistant, amount))
    })
}

pub(super) fn prompt_spell_mana_ability_window(
    game: &mut GameState,
    _trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    mut pending: PendingCast,
    _decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    let request = spell_mana_payment_request(game, &pending)?;
    let plan = crate::mana_payment::plan_mana_payment(game, &request)
        .map_err(|failure| {
            state.rollback_action(game);
            GameLoopError::ActionCancelled(format!(
                "no legal mana payment plan for the spell: {failure:?}"
            ))
        })?
        .into_iter()
        .next()
        .ok_or_else(|| GameLoopError::InvalidState("planner returned no spell plan".to_string()))?;

    pending.display_mana_pips =
        expand_mana_cost_to_display_pips(&request.cost, request.x_value as usize);
    pending.pending_mana_payment = Some(crate::mana_payment::PendingManaPayment::new(
        request.clone(),
        plan.clone(),
    ));
    pending.mana_ability_window_closed = true;
    pending.stage = CastStage::PayingMana;
    let player = pending.caster;
    let source = pending.spell_id;
    let subject = game
        .object(source)
        .map(|spell| spell.name.to_string())
        .unwrap_or_else(|| "spell".to_string());
    state.pending_cast = Some(pending);
    Ok(GameProgress::NeedsDecisionCtx(
        crate::decisions::context::DecisionContext::ManaPayment(
            crate::decisions::context::ManaPaymentContext::new(
                player, source, subject, request, plan,
            ),
        ),
    ))
}

pub(super) fn prompt_activation_mana_ability_window(
    game: &mut GameState,
    _trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    mut pending: PendingActivation,
    _decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    let request = activation_mana_payment_request(game, &pending)?;
    let cost = pending.mana_cost_to_pay.as_ref().ok_or_else(|| {
        GameLoopError::InvalidState("activation payment prompt has no mana cost".to_string())
    })?;
    let plan = crate::mana_payment::plan_mana_payment(game, &request)
        .map_err(|failure| {
            state.rollback_action(game);
            GameLoopError::ActionCancelled(format!(
                "no legal mana payment plan for the activation: {failure:?}"
            ))
        })?
        .into_iter()
        .next()
        .ok_or_else(|| {
            GameLoopError::InvalidState("planner returned no activation plan".to_string())
        })?;

    pending.display_mana_pips =
        expand_mana_cost_to_display_pips(cost, pending.x_value.unwrap_or(0));
    pending.pending_mana_payment = Some(crate::mana_payment::PendingManaPayment::new(
        request.clone(),
        plan.clone(),
    ));
    pending.mana_ability_window_closed = true;
    pending.stage = ActivationStage::PayingMana;
    let player = pending.activator;
    let source = pending.source;
    let subject = format!("{}'s ability", pending.source_name);
    state.pending_activation = Some(pending);
    Ok(GameProgress::NeedsDecisionCtx(
        crate::decisions::context::DecisionContext::ManaPayment(
            crate::decisions::context::ManaPaymentContext::new(
                player, source, subject, request, plan,
            ),
        ),
    ))
}

pub(super) fn activation_mana_payment_request(
    game: &GameState,
    pending: &PendingActivation,
) -> Result<crate::mana_payment::ManaPaymentRequest, GameLoopError> {
    let cost = pending.mana_cost_to_pay.as_ref().ok_or_else(|| {
        GameLoopError::InvalidState("activation payment prompt has no mana cost".to_string())
    })?;
    let locked_cost = crate::mana::ManaCost::from_pips(expand_mana_cost_to_pips(
        cost,
        pending.x_value.unwrap_or(0),
        &pending.hybrid_choices,
    ));
    let spend_policy = game.mana_spend_policy(pending.activator, Some(pending.source));
    let mut request = crate::mana_payment::ManaPaymentRequest::new(
        pending.activator,
        pending.source,
        pending.payment_reason,
        locked_cost,
    )
    .with_spend_policy(spend_policy);
    request.allow_black_life = crate::decision::mana_cost_has_black_symbol(&request.cost)
        && game.player_can_pay_black_with_life_for_reason(
            pending.activator,
            Some(pending.source),
            pending.payment_reason,
        );
    if let Some(existing) = pending.pending_mana_payment.as_ref() {
        request.preferences = existing.request.preferences.clone();
    }
    if pending.activation_cost_has_tap
        && !request
            .preferences
            .excluded_sources
            .contains(&pending.source)
    {
        request.preferences.excluded_sources.push(pending.source);
    }
    request.preferences.normalize();
    Ok(request)
}

pub(super) fn auto_pay_spell_tap_cost_steps(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    pending: &mut PendingCast,
    decision_maker: &mut impl DecisionMaker,
) -> Result<(), GameLoopError> {
    loop {
        let Some(index) = pending.remaining_cost_steps.iter().position(|step| {
            matches!(
                step,
                ActivationCostStep::Cost(cost) if cost.requires_tap() || cost.requires_untap()
            )
        }) else {
            return Ok(());
        };

        let ActivationCostStep::Cost(cost) = pending.remaining_cost_steps.remove(index) else {
            unreachable!("tap/untap auto-payment only matches cost steps");
        };

        let mut cost_ctx = CostContext::new(pending.spell_id, pending.caster, &mut *decision_maker)
            .with_provenance(pending.provenance);
        cost_ctx.tagged_objects = pending.tagged_objects.clone();
        cost_ctx.effect_outcomes = pending.effect_outcomes.clone();
        cost_ctx.x_value = pending.x_value;

        match cost.pay(game, &mut cost_ctx).map_err(|err| {
            GameLoopError::InvalidState(format!(
                "Failed to auto-pay spell tap cost {}: {err:?}",
                describe_cost_component(&cost)
            ))
        })? {
            crate::costs::CostPaymentResult::Paid => {
                record_immediate_cost_payment(&mut pending.payment_trace, &cost, pending.spell_id);
                pending.tagged_objects = cost_ctx.tagged_objects;
                pending.effect_outcomes = cost_ctx.effect_outcomes;
                drain_pending_trigger_events(game, trigger_queue);
            }
            crate::costs::CostPaymentResult::NeedsChoice(description) => {
                return Err(GameLoopError::InvalidState(format!(
                    "Spell tap cost unexpectedly requires choice: {} ({description})",
                    describe_cost_component(&cost)
                )));
            }
        }
    }
}

pub(super) fn continue_spell_cost_payment(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    mut pending: PendingCast,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    let Some(step) = pending.remaining_cost_steps.first().cloned() else {
        return continue_spell_next_cost_or_finalize(
            game,
            trigger_queue,
            state,
            pending,
            decision_maker,
        );
    };

    match step {
        ActivationCostStep::Cost(cost) => {
            let mut cost_ctx =
                CostContext::new(pending.spell_id, pending.caster, &mut *decision_maker)
                    .with_provenance(pending.provenance);
            cost_ctx.tagged_objects = pending.tagged_objects.clone();
            cost_ctx.effect_outcomes = pending.effect_outcomes.clone();
            cost_ctx.x_value = pending.x_value;

            let payment = cost.pay(game, &mut cost_ctx).map_err(|err| {
                GameLoopError::InvalidState(format!(
                    "Failed to pay deferred spell cost {}: {err:?}",
                    describe_cost_component(&cost)
                ))
            })?;
            if cost_ctx.decision_maker.awaiting_choice() {
                state.pending_cast = Some(pending);
                return Ok(GameProgress::Continue);
            }

            match payment {
                crate::costs::CostPaymentResult::Paid => {
                    record_immediate_cost_payment(
                        &mut pending.payment_trace,
                        &cost,
                        pending.spell_id,
                    );
                    pending.tagged_objects = cost_ctx.tagged_objects;
                    pending.effect_outcomes = cost_ctx.effect_outcomes;
                    pending.remaining_cost_steps.remove(0);
                    drain_pending_trigger_events(game, trigger_queue);
                    continue_spell_next_cost_or_finalize(
                        game,
                        trigger_queue,
                        state,
                        pending,
                        decision_maker,
                    )
                }
                crate::costs::CostPaymentResult::NeedsChoice(description) => {
                    Err(GameLoopError::InvalidState(format!(
                        "Deferred spell cost unexpectedly requires staged choice: {} ({})",
                        describe_cost_component(&cost),
                        description
                    )))
                }
            }
        }
        ActivationCostStep::Sacrifice {
            filter,
            description,
            ..
        } => {
            let legal_targets = get_legal_sacrifice_targets(
                game,
                pending.caster,
                pending.spell_id,
                &filter,
                crate::costs::PaymentReason::CastSpell,
            );
            if legal_targets.is_empty() {
                return Err(GameLoopError::InvalidState(
                    "No valid permanents available for spell sacrifice cost".to_string(),
                ));
            }

            let player = pending.caster;
            let source = pending.spell_id;
            pending.stage = CastStage::ChoosingSacrifice;
            state.pending_cast = Some(pending);

            let candidates: Vec<crate::decisions::context::SelectableObject> = legal_targets
                .iter()
                .map(|&id| {
                    let name = game
                        .object(id)
                        .map(|o| o.name.to_string())
                        .unwrap_or_else(|| format!("Object #{}", id.0));
                    crate::decisions::context::SelectableObject::new(id, name)
                })
                .collect();
            let ctx = crate::decisions::context::SelectObjectsContext::new(
                player,
                Some(source),
                description,
                candidates,
                1,
                Some(1),
            )
            .with_reveal_policy(crate::decisions::context::SelectionRevealPolicy::Public);
            Ok(GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::SelectObjects(ctx),
            ))
        }
        ActivationCostStep::CardChoice(card_choice_cost) => {
            let (description, legal_cards) = card_cost_choice_description_and_candidates(
                game,
                pending.caster,
                pending.spell_id,
                &card_choice_cost,
                &[],
            );
            if legal_cards.is_empty() {
                return Err(GameLoopError::InvalidState(
                    "No valid cards available for spell cost choice".to_string(),
                ));
            }

            let player = pending.caster;
            let source = pending.spell_id;
            pending.stage = CastStage::ChoosingCardCost;
            state.pending_cast = Some(pending);

            let candidates: Vec<crate::decisions::context::SelectableObject> = legal_cards
                .iter()
                .map(|&id| {
                    let name = game
                        .object(id)
                        .map(|o| o.name.to_string())
                        .unwrap_or_else(|| format!("Object #{}", id.0));
                    crate::decisions::context::SelectableObject::new(id, name)
                })
                .collect();
            let ctx = crate::decisions::context::SelectObjectsContext::new(
                player,
                Some(source),
                description,
                candidates,
                1,
                Some(1),
            )
            .with_reveal_policy(card_cost_choice_reveal_policy(&card_choice_cost));
            Ok(GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::SelectObjects(ctx),
            ))
        }
    }
}

/// Continue the casting process into selectable payment order.
///
/// Called after targets are chosen (or when no targets needed).
/// Computes the effective mana cost and remaining non-mana payment steps.
fn mana_cost_with_paid_optional_costs(
    base_cost: &crate::mana::ManaCost,
    spell: &crate::object::Object,
    optional_costs_paid: &OptionalCostsPaid,
) -> crate::mana::ManaCost {
    let mut pips = base_cost.pips().to_vec();
    for (index, optional_cost) in spell.optional_costs.iter().enumerate() {
        let times = optional_costs_paid.times_paid(index);
        let Some(mana_cost) = optional_cost.cost.mana_cost() else {
            continue;
        };
        for _ in 0..times {
            pips.extend(mana_cost.pips().iter().cloned());
        }
    }
    crate::mana::ManaCost::from_pips(pips)
}

fn mana_cost_with_paid_optional_and_splice_costs(
    base_cost: &crate::mana::ManaCost,
    spell: &crate::object::Object,
    optional_costs_paid: &OptionalCostsPaid,
    splice_costs: &[crate::cost::TotalCost],
    chosen_modes: Option<&[usize]>,
) -> crate::mana::ManaCost {
    let combined = mana_cost_with_paid_optional_costs(base_cost, spell, optional_costs_paid);
    let mut pips = combined.pips().to_vec();
    for splice_cost in splice_costs {
        if let Some(mana_cost) = splice_cost.mana_cost() {
            pips.extend(mana_cost.pips().iter().cloned());
        }
    }
    if let Some(chosen_modes) = chosen_modes
        && let Some(spree) = spell
            .spell_effect
            .as_deref()
            .and_then(|program| {
                program
                    .all_effects()
                    .into_iter()
                    .find_map(|effect| effect.modal_effect_spec())
            })
            .filter(|modal| modal.spree)
    {
        for mode in chosen_modes {
            if let Some(cost) = spree.mode_additional_mana_costs.get(*mode) {
                pips.extend(cost.pips().iter().cloned());
            }
        }
    }
    crate::mana::ManaCost::from_pips(pips)
}

fn spell_escalate_cost(spell: &crate::object::Object) -> Option<&crate::cost::TotalCost> {
    spell.abilities.iter().find_map(|ability| {
        let crate::ability::AbilityKind::Static(static_ability) = &ability.kind else {
            return None;
        };
        static_ability.escalate_spec().map(|spec| &spec.cost)
    })
}

fn mana_cost_with_escalate(
    base_cost: &crate::mana::ManaCost,
    spell: &crate::object::Object,
    chosen_modes: Option<&[usize]>,
) -> crate::mana::ManaCost {
    let times = chosen_modes
        .map(|modes| modes.len().saturating_sub(1))
        .unwrap_or(0);
    let Some(escalate_mana) = spell_escalate_cost(spell).and_then(|cost| cost.mana_cost()) else {
        return base_cost.clone();
    };

    let mut pips = base_cost.pips().to_vec();
    for _ in 0..times {
        pips.extend(escalate_mana.pips().iter().cloned());
    }
    crate::mana::ManaCost::from_pips(pips)
}

fn mana_cost_with_effect_additional_cost(
    base_cost: &crate::mana::ManaCost,
    additional_cost: Option<&crate::mana::ManaCost>,
) -> crate::mana::ManaCost {
    let Some(additional_cost) = additional_cost else {
        return base_cost.clone();
    };
    let mut pips = base_cost.pips().to_vec();
    pips.extend(additional_cost.pips().iter().cloned());
    crate::mana::ManaCost::from_pips(pips)
}

fn announced_spell_mana_cost(
    game: &GameState,
    pending: &PendingCast,
) -> Option<crate::mana::ManaCost> {
    let spell = game.object(pending.spell_id)?;
    let base = if pending.base_mana_cost_waived {
        crate::mana::ManaCost::new()
    } else {
        get_spell_mana_cost(
            game,
            pending.spell_id,
            pending.caster,
            &pending.casting_method,
            pending.from_zone,
        )?
    };
    let combined = mana_cost_with_paid_optional_and_splice_costs(
        &base,
        spell,
        &pending.optional_costs_paid,
        &pending.splice_costs,
        pending.chosen_modes.as_deref(),
    );
    let combined = mana_cost_with_escalate(&combined, spell, pending.chosen_modes.as_deref());
    Some(mana_cost_with_effect_additional_cost(
        &combined,
        pending.effect_additional_mana_cost.as_ref(),
    ))
}

pub(super) fn continue_to_mana_payment(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    pending: PendingCast,
    targets: Vec<Target>,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    use crate::decision::calculate_effective_mana_cost_for_payment_with_chosen_targets_for_casting_method_from_zone;

    let mut pending = pending;
    pending.chosen_targets = targets;

    // CR 601.2e validates the completed proposal after all announcements and
    // before total-cost calculation, the mana-ability window, or any payment.
    // A failed proposal is cancelled atomically under CR 601.6.
    let proposal_is_legal = game.object(pending.spell_id).is_some_and(|spell| {
        if pending.effect_driven {
            crate::decision::completed_effect_driven_cast_proposal_is_legal(
                game,
                pending.caster,
                spell,
                &pending.casting_method,
            )
        } else {
            crate::decision::completed_cast_proposal_is_legal(
                game,
                pending.caster,
                spell,
                &pending.casting_method,
            )
        }
    });
    if !proposal_is_legal {
        state.rollback_action(game);
        return Err(GameLoopError::ActionCancelled(
            "completed spell proposal is illegal under CR 601.2e".to_string(),
        ));
    }

    // Compute the effective mana cost for this spell
    let effective_cost = if let Some(obj) = game.object(pending.spell_id) {
        let base_cost = if pending.base_mana_cost_waived {
            Some(crate::mana::ManaCost::new())
        } else {
            crate::decision::spell_mana_cost_for_cast(
                game,
                pending.caster,
                obj,
                &pending.casting_method,
                pending.from_zone,
            )
        };

        // Apply cost reductions (affinity, delve, convoke, improvise)
        base_cost.map(|bc| {
            let bc = mana_cost_with_paid_optional_and_splice_costs(
                &bc,
                obj,
                &pending.optional_costs_paid,
                &pending.splice_costs,
                pending.chosen_modes.as_deref(),
            );
            let bc = mana_cost_with_escalate(&bc, obj, pending.chosen_modes.as_deref());
            let bc = mana_cost_with_effect_additional_cost(
                &bc,
                pending.effect_additional_mana_cost.as_ref(),
            );
            let effective = calculate_effective_mana_cost_for_payment_with_chosen_targets_for_casting_method_from_zone(
                game,
                pending.caster,
                obj,
                &bc,
                &pending.chosen_targets,
                &pending.casting_method,
                pending.from_zone,
            );
            pending
                .effect_mana_cost_reduction
                .as_ref()
                .map_or(effective.clone(), |reduction| {
                    crate::decision::reduce_mana_cost(&effective, reduction)
                })
        })
    } else {
        None
    };

    pending.mana_cost_to_pay = effective_cost.filter(|cost| !cost.is_empty());

    if pending.remaining_cost_steps.is_empty() {
        pending.remaining_cost_steps = collect_spell_cost_steps(
            game,
            pending.spell_id,
            pending.caster,
            &pending.casting_method,
            &pending.optional_costs_paid,
            &pending.splice_costs,
            pending.chosen_modes.as_deref(),
            pending.chosen_targets.len(),
            pending.from_zone,
        );
        if game
            .object(pending.spell_id)
            .is_some_and(crate::decision::has_delve)
            && pending
                .mana_cost_to_pay
                .as_ref()
                .is_some_and(|cost| cost.generic_mana_total() > 0)
            && game
                .player(pending.caster)
                .is_some_and(|player| !player.graveyard.is_empty())
        {
            pending.remaining_cost_steps.push(delve_cost_step());
        }
    }

    continue_spell_next_cost_or_finalize(game, trigger_queue, state, pending, decision_maker)
}

pub(super) fn get_available_mana_abilities(
    game: &GameState,
    player: PlayerId,
    decision_maker: &mut impl DecisionMaker,
) -> Vec<(ObjectId, usize, String)> {
    let _ = decision_maker;
    collect_available_mana_abilities(game, player, |_, _| true)
}

pub(crate) fn attack_mana_ability_window_context(
    game: &GameState,
    player: PlayerId,
    declaration_source: ObjectId,
) -> Option<crate::decisions::context::SelectOptionsContext> {
    declaration_mana_ability_window_context(
        game,
        player,
        declaration_source,
        "declaring attackers",
        false,
    )
}

pub(crate) fn blocker_mana_ability_window_context(
    game: &GameState,
    player: PlayerId,
    declaration_source: ObjectId,
) -> Option<crate::decisions::context::SelectOptionsContext> {
    declaration_mana_ability_window_context(
        game,
        player,
        declaration_source,
        "declaring blockers",
        true,
    )
}

fn declaration_mana_ability_window_context(
    game: &GameState,
    player: PlayerId,
    declaration_source: ObjectId,
    declaration_kind: &str,
    include_finish_only: bool,
) -> Option<crate::decisions::context::SelectOptionsContext> {
    let mut decision_maker = crate::decision::AutoPassDecisionMaker;
    let mana_abilities = get_available_mana_abilities(game, player, &mut decision_maker);
    if mana_abilities.is_empty() && !include_finish_only {
        return None;
    }

    let mut options = mana_abilities
        .iter()
        .enumerate()
        .map(|(index, (source, _, description))| {
            crate::decisions::context::SelectableOption::new(
                index,
                format!(
                    "Activate {}: {}",
                    describe_permanent(game, *source),
                    description
                ),
            )
            .with_object(*source)
        })
        .collect::<Vec<_>>();
    options.push(crate::decisions::context::SelectableOption::new(
        options.len(),
        "Finish activating mana abilities",
    ));

    Some(crate::decisions::context::SelectOptionsContext::new(
        player,
        Some(declaration_source),
        format!("Activate mana abilities before paying costs for {declaration_kind}"),
        options,
        1,
        1,
    ))
}

/// Apply one response in the CR 508 attack-cost mana-ability window.
/// Returns `true` when the player closes the window and costs may be paid.
pub(crate) fn apply_attack_mana_ability_window_response(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    player: PlayerId,
    choice: usize,
) -> Result<bool, GameLoopError> {
    apply_declaration_mana_ability_window_response(
        game,
        trigger_queue,
        player,
        choice,
        "attack declaration",
    )
}

pub(crate) fn apply_blocker_mana_ability_window_response(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    player: PlayerId,
    choice: usize,
) -> Result<bool, GameLoopError> {
    apply_declaration_mana_ability_window_response(
        game,
        trigger_queue,
        player,
        choice,
        "blocker declaration",
    )
}

fn apply_declaration_mana_ability_window_response(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    player: PlayerId,
    choice: usize,
    declaration_kind: &str,
) -> Result<bool, GameLoopError> {
    use crate::special_actions::{SpecialAction, perform};

    let mut decision_maker = crate::decision::AutoPassDecisionMaker;
    let mana_abilities = get_available_mana_abilities(game, player, &mut decision_maker);
    if choice > mana_abilities.len() {
        return Err(GameLoopError::InvalidState(format!(
            "Invalid {declaration_kind} mana-ability window choice: {choice} > {}",
            mana_abilities.len()
        )));
    }
    if choice == mana_abilities.len() {
        return Ok(true);
    }

    let (permanent_id, ability_index, _) = mana_abilities[choice];
    let activation_cost_has_tap = activated_ability_has_tap_cost(game, permanent_id, ability_index);
    perform(
        SpecialAction::ActivateManaAbility {
            permanent_id,
            ability_index,
        },
        game,
        player,
        &mut decision_maker,
    )
    .map_err(|err| {
        GameLoopError::InvalidState(format!(
            "Failed to activate mana ability during {declaration_kind}: {err}"
        ))
    })?;
    drain_pending_trigger_events(game, trigger_queue);
    queue_ability_activated_event(
        game,
        trigger_queue,
        &mut decision_maker,
        permanent_id,
        player,
        true,
        None,
        activation_cost_has_tap,
    );

    Ok(false)
}

fn collect_available_mana_abilities(
    game: &GameState,
    player: PlayerId,
    mut include: impl FnMut(ObjectId, &crate::ability::Ability) -> bool,
) -> Vec<(ObjectId, usize, String)> {
    use crate::special_actions::can_activate_mana_ability_check_with_view;

    let mut abilities = Vec::new();
    let view = crate::derived_view::DerivedGameView::new(game);
    let simple_mana_analysis = view.simple_battlefield_mana_analysis(player);

    for &perm_id in simple_mana_analysis.mana_source_ids() {
        let Some(perm) = game.object(perm_id) else {
            continue;
        };
        let cached_abilities = view.abilities_rc(perm_id);
        let current_abilities = cached_abilities.as_deref().unwrap_or(&perm.abilities);

        for &ability_index in simple_mana_analysis.mana_ability_indices_for(perm_id) {
            let Some(ability) = current_abilities.get(ability_index) else {
                continue;
            };
            if simple_mana_analysis
                .activatable_indices_for(perm_id)
                .contains(&ability_index)
                || can_activate_mana_ability_check_with_view(
                    game,
                    player,
                    perm_id,
                    ability_index,
                    ability,
                    &view,
                    None,
                )
                .is_ok()
            {
                if !include(perm_id, ability) {
                    continue;
                }
                let desc = describe_mana_ability(game, perm_id, player, &ability.kind);
                abilities.push((perm_id, ability_index, desc));
            }
        }
    }

    abilities
}

/// Describe a mana ability for display.
pub(super) fn describe_mana_ability(
    game: &GameState,
    source: ObjectId,
    controller: PlayerId,
    kind: &crate::ability::AbilityKind,
) -> String {
    use crate::ability::AbilityKind;
    use crate::mana::ManaSymbol;

    if let AbilityKind::Activated(mana_ability) = kind
        && mana_ability.is_runtime_mana_ability(game, source, controller)
    {
        let mana_strs: Vec<&str> = mana_ability
            .inferred_mana_symbols(game, source, controller)
            .iter()
            .map(|m| match m {
                ManaSymbol::White => "{W}",
                ManaSymbol::Blue => "{U}",
                ManaSymbol::Black => "{B}",
                ManaSymbol::Red => "{R}",
                ManaSymbol::Green => "{G}",
                ManaSymbol::Colorless => "{C}",
                _ => "mana",
            })
            .collect();
        if mana_strs.is_empty() {
            "Add mana".to_string()
        } else {
            format!("Add {}", mana_strs.join(""))
        }
    } else {
        "Add mana".to_string()
    }
}

/// Describe a permanent for display.
pub(super) fn describe_permanent(game: &GameState, id: ObjectId) -> String {
    game.object(id)
        .map(|obj| obj.name.to_string())
        .unwrap_or_else(|| "Unknown".to_string())
}

/// Get legal sacrifice targets for a filter.
pub(super) fn get_legal_sacrifice_targets(
    game: &GameState,
    player: PlayerId,
    source: ObjectId,
    filter: &ObjectFilter,
    reason: crate::costs::PaymentReason,
) -> Vec<ObjectId> {
    let ctx = game.filter_context_for(player, Some(source));
    game.battlefield
        .iter()
        .copied()
        .filter(|&id| {
            game.object(id).is_some_and(|obj| {
                filter.matches(obj, &ctx, game)
                    && game.can_be_sacrificed(id)
                    && (!reason.is_cast_or_ability_payment()
                        || !game.player_cant_sacrifice_nonland_to_cast_or_activate(player)
                        || obj.has_card_type(crate::types::CardType::Land))
            })
        })
        .collect()
}

/// Get legal cards in hand that can be discarded for a cost.
pub(super) fn get_legal_discard_cards(
    game: &GameState,
    player: PlayerId,
    source: ObjectId,
    card_types: &[crate::types::CardType],
) -> Vec<ObjectId> {
    game.player(player)
        .map(|p| {
            p.hand
                .iter()
                .copied()
                .filter(|&card_id| {
                    if card_id == source {
                        return false;
                    }
                    game.object(card_id).is_some_and(|obj| {
                        card_types.is_empty()
                            || card_types
                                .iter()
                                .any(|card_type| obj.card_types.contains(card_type))
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Get legal cards in hand that can be exiled for a cost.
pub(super) fn get_legal_exile_from_hand_cards(
    game: &GameState,
    player: PlayerId,
    source: ObjectId,
    color_filter: Option<crate::color::ColorSet>,
) -> Vec<ObjectId> {
    game.player(player)
        .map(|p| {
            p.hand
                .iter()
                .copied()
                .filter(|&card_id| {
                    if card_id == source {
                        return false;
                    }
                    game.object(card_id).is_some_and(|obj| {
                        if let Some(required_colors) = color_filter {
                            !obj.colors().intersection(required_colors).is_empty()
                        } else {
                            true
                        }
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Get legal cards in graveyard that can be exiled for a cost.
pub(super) fn get_legal_exile_from_graveyard_cards(
    game: &GameState,
    player: PlayerId,
    card_type: Option<crate::types::CardType>,
) -> Vec<ObjectId> {
    game.player(player)
        .map(|p| {
            p.graveyard
                .iter()
                .copied()
                .filter(|&card_id| {
                    if let Some(ct) = card_type {
                        game.object(card_id)
                            .is_some_and(|obj| obj.has_card_type(ct))
                    } else {
                        true
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Get legal cards in hand that can be revealed for a cost.
pub(super) fn get_legal_reveal_from_hand_cards(
    game: &GameState,
    player: PlayerId,
    source: ObjectId,
    card_type: Option<crate::types::CardType>,
    color_filter: Option<crate::color::ColorSet>,
) -> Vec<ObjectId> {
    game.player(player)
        .map(|p| {
            p.hand
                .iter()
                .copied()
                .filter(|&card_id| {
                    if card_id == source {
                        return false;
                    }
                    let Some(obj) = game.object(card_id) else {
                        return false;
                    };
                    if let Some(ct) = card_type
                        && !obj.has_card_type(ct)
                    {
                        return false;
                    }
                    if let Some(required_colors) = color_filter {
                        return game.current_colors(card_id).is_some_and(|colors| {
                            !colors.intersection(required_colors).is_empty()
                        });
                    }
                    true
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Get legal permanents that can be returned to hand for a cost.
pub(super) fn get_legal_return_to_hand_targets(
    game: &GameState,
    player: PlayerId,
    source: ObjectId,
    filter: &ObjectFilter,
) -> Vec<ObjectId> {
    let ctx = game.filter_context_for(player, Some(source));
    game.battlefield
        .iter()
        .copied()
        .filter(|&id| {
            game.object(id)
                .is_some_and(|obj| filter.matches(obj, &ctx, game))
        })
        .collect()
}

pub(super) fn get_legal_cost_choice_objects(
    game: &GameState,
    player: PlayerId,
    source: ObjectId,
    filter: &ObjectFilter,
    zone: Zone,
    top_only: bool,
) -> Vec<ObjectId> {
    let ctx = game.filter_context_for(player, Some(source));

    let ids: Vec<ObjectId> = match zone {
        Zone::Battlefield => game.battlefield.iter().copied().collect(),
        Zone::Hand => game
            .player(player)
            .map(|p| p.hand.iter().copied().collect())
            .unwrap_or_default(),
        Zone::Graveyard => game.player(player).map_or_else(Vec::new, |p| {
            if top_only {
                p.graveyard.iter().rev().copied().collect()
            } else {
                p.graveyard.to_vec()
            }
        }),
        Zone::Exile => game.exile.iter().copied().collect(),
        _ => Vec::new(),
    };

    let mut candidates = ids
        .into_iter()
        .filter(|&id| {
            game.object(id).is_some_and(|obj| {
                if filter.other && obj.id == source {
                    return false;
                }
                filter.matches(obj, &ctx, game)
            })
        })
        .collect::<Vec<_>>();
    if top_only {
        candidates.truncate(1);
    }
    candidates
}

pub(super) fn card_cost_choice_description_and_candidates(
    game: &GameState,
    player: PlayerId,
    source: ObjectId,
    card_choice_cost: &ActivationCardCostChoice,
    already_chosen: &[ObjectId],
) -> (String, Vec<ObjectId>) {
    let (description, mut candidates) = match card_choice_cost {
        ActivationCardCostChoice::Discard {
            card_types,
            description,
            ..
        } => (
            format!("Choose a card to discard: {}", description),
            get_legal_discard_cards(game, player, source, card_types),
        ),
        ActivationCardCostChoice::ExileFromHand {
            color_filter,
            description,
            ..
        } => (
            format!("Choose a card to exile: {}", description),
            get_legal_exile_from_hand_cards(game, player, source, *color_filter),
        ),
        ActivationCardCostChoice::ExileFromGraveyard {
            card_type,
            description,
            ..
        } => (
            format!(
                "Choose a card to exile from your graveyard: {}",
                description
            ),
            get_legal_exile_from_graveyard_cards(game, player, *card_type),
        ),
        ActivationCardCostChoice::ExileChosenObject {
            filter,
            zone,
            top_only,
            description,
            ..
        } => (
            format!("Choose an object to exile: {}", description),
            get_legal_cost_choice_objects(game, player, source, filter, *zone, *top_only),
        ),
        ActivationCardCostChoice::RevealFromHand {
            card_type,
            color_filter,
            description,
            ..
        } => (
            format!("Choose a card to reveal: {}", description),
            get_legal_reveal_from_hand_cards(game, player, source, *card_type, *color_filter),
        ),
        ActivationCardCostChoice::ReturnToHand {
            filter,
            description,
            ..
        } => (
            format!("Choose a permanent to return: {}", description),
            get_legal_return_to_hand_targets(game, player, source, filter),
        ),
        ActivationCardCostChoice::MoveChosenObjectToZone {
            filter,
            source_zone,
            destination_zone,
            description,
            ..
        } => (
            format!(
                "Choose an object to move to {}: {}",
                destination_zone, description
            ),
            get_legal_cost_choice_objects(game, player, source, filter, *source_zone, false),
        ),
    };
    candidates.retain(|id| !already_chosen.contains(id));
    (description, candidates)
}

fn card_cost_choice_reveal_policy(
    card_choice_cost: &ActivationCardCostChoice,
) -> crate::decisions::context::SelectionRevealPolicy {
    use crate::decisions::context::SelectionRevealPolicy;

    match card_choice_cost {
        ActivationCardCostChoice::Discard { .. }
        | ActivationCardCostChoice::ExileFromHand { .. }
        | ActivationCardCostChoice::ExileFromGraveyard { .. }
        | ActivationCardCostChoice::ExileChosenObject { .. }
        | ActivationCardCostChoice::RevealFromHand { .. } => SelectionRevealPolicy::Public,
        ActivationCardCostChoice::MoveChosenObjectToZone {
            destination_zone, ..
        } if !destination_zone.is_hidden() => SelectionRevealPolicy::Public,
        _ => SelectionRevealPolicy::None,
    }
}

fn filter_names_source_object(game: &GameState, source: ObjectId, filter: &ObjectFilter) -> bool {
    if filter.specific == Some(source) {
        return true;
    }

    let Some(source_name) = game.object(source).map(|object| object.name.as_str()) else {
        return false;
    };

    filter
        .name
        .as_deref()
        .is_some_and(|name| name.eq_ignore_ascii_case(source_name))
}

fn deterministic_named_source_cost(
    game: &GameState,
    source: ObjectId,
    filter: &ObjectFilter,
    description: &str,
    legal_objects: &[ObjectId],
) -> bool {
    if legal_objects.len() != 1 || legal_objects[0] != source {
        return false;
    }

    if filter_names_source_object(game, source, filter) {
        return true;
    }

    let Some(source_name) = game.object(source).map(|object| object.name.as_str()) else {
        return false;
    };
    description
        .to_ascii_lowercase()
        .contains(&source_name.to_ascii_lowercase())
}

fn deterministic_named_source_card_cost(
    game: &GameState,
    source: ObjectId,
    card_choice_cost: &ActivationCardCostChoice,
    legal_objects: &[ObjectId],
) -> bool {
    match card_choice_cost {
        ActivationCardCostChoice::ExileChosenObject {
            filter,
            description,
            ..
        }
        | ActivationCardCostChoice::ReturnToHand {
            filter,
            description,
            ..
        }
        | ActivationCardCostChoice::MoveChosenObjectToZone {
            filter,
            description,
            ..
        } => deterministic_named_source_cost(game, source, filter, description, legal_objects),
        ActivationCardCostChoice::Discard { .. }
        | ActivationCardCostChoice::ExileFromHand { .. }
        | ActivationCardCostChoice::ExileFromGraveyard { .. }
        | ActivationCardCostChoice::RevealFromHand { .. } => false,
    }
}

pub(super) fn collect_spell_cost_steps(
    game: &GameState,
    spell_id: ObjectId,
    caster: PlayerId,
    casting_method: &CastingMethod,
    optional_costs_paid: &OptionalCostsPaid,
    splice_costs: &[crate::cost::TotalCost],
    chosen_modes: Option<&[usize]>,
    chosen_target_count: usize,
    from_zone: Zone,
) -> Vec<ActivationCostStep> {
    let mut cost_steps = Vec::new();
    let extend_non_mana = |out: &mut Vec<ActivationCostStep>, total: &crate::cost::TotalCost| {
        let non_mana_components: Vec<_> = total
            .costs()
            .iter()
            .filter(|component| component.mana_cost_ref().is_none())
            .cloned()
            .collect();
        append_activation_cost_steps_from_components(&non_mana_components, out);
    };

    if let Some(obj) = game.object(spell_id) {
        let alternative_additional_cost = match casting_method {
            CastingMethod::Normal => obj
                .cast_alternative_method
                .as_ref()
                .and_then(|method| method.total_cost())
                .cloned()
                .unwrap_or_else(crate::cost::TotalCost::free),
            CastingMethod::FaceDown => crate::cost::TotalCost::free(),
            CastingMethod::SplitOtherHalf | CastingMethod::Fuse => crate::cost::TotalCost::free(),
            CastingMethod::Alternative(idx) => obj
                .alternative_casts
                .get(*idx)
                .and_then(|method| method.total_cost())
                .cloned()
                .unwrap_or_else(crate::cost::TotalCost::free),
            CastingMethod::GrantedEscape { .. } => crate::cost::TotalCost::free(),
            CastingMethod::GrantedFlashback => crate::cost::TotalCost::free(),
            CastingMethod::PlayFrom {
                use_alternative: None,
                ..
            } => crate::cost::TotalCost::free(),
            CastingMethod::PlayFrom {
                use_alternative: Some(idx),
                zone,
                ..
            }
            | CastingMethod::SplitOtherHalfPlayFrom {
                use_alternative: idx,
                zone,
                ..
            } => crate::decision::resolve_play_from_alternative_method(
                game, caster, obj, *zone, *idx,
            )
            .or_else(|| obj.cast_alternative_method_owned())
            .and_then(|method| method.total_cost().cloned())
            .unwrap_or_else(crate::cost::TotalCost::free),
        };

        let method_specific_additional_cost = match casting_method {
            CastingMethod::Normal => obj
                .cast_alternative_method
                .as_ref()
                .and_then(|method| method.additional_cost())
                .cloned()
                .unwrap_or_else(crate::cost::TotalCost::free),
            CastingMethod::Alternative(idx) => obj
                .alternative_casts
                .get(*idx)
                .or(obj.cast_alternative_method.as_deref())
                .and_then(|method| method.additional_cost())
                .cloned()
                .unwrap_or_else(crate::cost::TotalCost::free),
            CastingMethod::GrantedEscape { exile_count, .. } => crate::cost::TotalCost::from_cost(
                crate::costs::Cost::exile_from_graveyard(*exile_count, None),
            ),
            CastingMethod::PlayFrom {
                use_alternative: Some(idx),
                zone,
                ..
            }
            | CastingMethod::SplitOtherHalfPlayFrom {
                use_alternative: idx,
                zone,
                ..
            } => crate::decision::resolve_play_from_alternative_method(
                game, caster, obj, *zone, *idx,
            )
            .or_else(|| obj.cast_alternative_method_owned())
            .and_then(|method| method.additional_cost().cloned())
            .unwrap_or_else(crate::cost::TotalCost::free),
            CastingMethod::FaceDown
            | CastingMethod::SplitOtherHalf
            | CastingMethod::Fuse
            | CastingMethod::GrantedFlashback
            | CastingMethod::PlayFrom {
                use_alternative: None,
                ..
            } => crate::cost::TotalCost::free(),
        };

        extend_non_mana(&mut cost_steps, &alternative_additional_cost);
        extend_non_mana(&mut cost_steps, &method_specific_additional_cost);
        extend_non_mana(&mut cost_steps, &obj.additional_cost);
        for (idx, optional_cost) in obj.optional_costs.iter().enumerate() {
            let times = optional_costs_paid.times_paid(idx);
            for _ in 0..times {
                extend_non_mana(&mut cost_steps, &optional_cost.cost);
            }
        }
        for splice_cost in splice_costs {
            extend_non_mana(&mut cost_steps, splice_cost);
        }
        if let Some(escalate_cost) = spell_escalate_cost(obj) {
            for _ in 0..chosen_modes
                .map(|modes| modes.len().saturating_sub(1))
                .unwrap_or(0)
            {
                extend_non_mana(&mut cost_steps, escalate_cost);
            }
        }
        let life_per_target = obj
            .abilities
            .iter()
            .filter(|ability| ability.functions_in(&obj.zone))
            .filter_map(|ability| match &ability.kind {
                crate::ability::AbilityKind::Static(static_ability) => {
                    static_ability.additional_life_cost_per_target()
                }
                _ => None,
            })
            .fold(0u32, u32::saturating_add);
        let total_life = life_per_target.saturating_mul(chosen_target_count as u32);
        if total_life > 0 {
            append_activation_cost_steps_from_components(
                &[crate::costs::Cost::life(total_life)],
                &mut cost_steps,
            );
        }
        let commander_tax_life =
            crate::decision::commander_tax_life_payment_amount(game, obj, from_zone);
        if commander_tax_life > 0 {
            append_activation_cost_steps_from_components(
                &[crate::costs::Cost::life(commander_tax_life)],
                &mut cost_steps,
            );
        }
    }

    cost_steps
}

pub(super) fn describe_cost_component(cost: &crate::costs::Cost) -> String {
    if cost.requires_tap() {
        return "Tap this permanent".to_string();
    }
    if cost.requires_untap() {
        return "Untap this permanent".to_string();
    }

    let display = cost.display();
    if !display.trim().is_empty() {
        display
    } else {
        cost.processing_mode().display()
    }
}

pub(super) fn describe_pending_cost_step(step: &ActivationCostStep) -> String {
    match step {
        ActivationCostStep::Cost(cost) => describe_cost_component(cost),
        ActivationCostStep::Sacrifice { description, .. } => description.clone(),
        ActivationCostStep::CardChoice(choice) => match choice {
            ActivationCardCostChoice::Discard { description, .. }
            | ActivationCardCostChoice::ExileFromHand { description, .. }
            | ActivationCardCostChoice::ExileFromGraveyard { description, .. }
            | ActivationCardCostChoice::ExileChosenObject { description, .. }
            | ActivationCardCostChoice::RevealFromHand { description, .. }
            | ActivationCardCostChoice::ReturnToHand { description, .. }
            | ActivationCardCostChoice::MoveChosenObjectToZone { description, .. } => {
                description.clone()
            }
        },
    }
}

pub(super) fn delve_cost_step() -> ActivationCostStep {
    ActivationCostStep::CardChoice(ActivationCardCostChoice::ExileFromGraveyard {
        cost: crate::costs::Cost::exile_from_graveyard(1, None),
        card_type: None,
        description: "Delve — exile a card to pay {1}".to_string(),
        generic_mana_reduction: 1,
    })
}

pub(super) fn delve_generic_reduction(step: &ActivationCostStep) -> u32 {
    match step {
        ActivationCostStep::CardChoice(ActivationCardCostChoice::ExileFromGraveyard {
            generic_mana_reduction,
            ..
        }) => *generic_mana_reduction,
        _ => 0,
    }
}

pub(super) fn spell_stage_after_targets(pending: &PendingCast) -> CastStage {
    if !pending.remaining_cost_steps.is_empty() || pending.mana_cost_to_pay.is_some() {
        CastStage::ChoosingNextCost
    } else {
        CastStage::ReadyToFinalize
    }
}

pub(super) fn activation_stage_after_targets(pending: &PendingActivation) -> ActivationStage {
    if !pending.pending_target_distributions.is_empty() {
        ActivationStage::ChoosingDistribution
    } else if !pending.remaining_cost_steps.is_empty() || pending.mana_cost_to_pay.is_some() {
        ActivationStage::ChoosingNextCost
    } else {
        ActivationStage::ReadyToFinalize
    }
}

pub(super) fn append_target_distribution_requirements(
    game: &GameState,
    source: ObjectId,
    player: PlayerId,
    x_value: Option<u32>,
    all_targets: &[Target],
    all_assignments: &[crate::game_state::TargetAssignment],
    requirements: &[TargetRequirement],
    new_assignments: &[crate::game_state::TargetAssignment],
    pending: &mut std::collections::VecDeque<PendingTargetDistribution>,
) -> Result<(), GameLoopError> {
    let resolved_targets = all_targets
        .iter()
        .map(|target| match target {
            Target::Object(id) => crate::effects::ResolvedTarget::Object(*id),
            Target::Player(id) => crate::effects::ResolvedTarget::Player(*id),
        })
        .collect::<Vec<_>>();
    let mut decision_maker = crate::decision::SelectFirstDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(source, player, &mut decision_maker)
        .with_targets(resolved_targets)
        .with_target_assignments(all_assignments.to_vec());
    ctx.x_value = x_value;

    for (requirement, assignment) in requirements.iter().zip(new_assignments) {
        let Some(value) = requirement.distribution_value.as_ref() else {
            continue;
        };
        let targets = all_targets
            .get(assignment.range.clone())
            .ok_or_else(|| {
                GameLoopError::InvalidState(
                    "distribution target assignment is outside the chosen target list".to_string(),
                )
            })?
            .to_vec();
        if targets.is_empty() {
            continue;
        }
        let total = crate::effects::helpers::resolve_value(game, value, &ctx)
            .map_err(|error| {
                GameLoopError::InvalidState(format!(
                    "cannot resolve announced distribution amount: {error}"
                ))
            })?
            .max(0) as u32;
        let required_minimum = requirement
            .distribution_min_per_target
            .saturating_mul(targets.len() as u32);
        if total < required_minimum {
            return Err(GameLoopError::ActionCancelled(format!(
                "cannot divide {total} with at least {} assigned to each of {} targets",
                requirement.distribution_min_per_target,
                targets.len()
            )));
        }
        pending.push_back(PendingTargetDistribution {
            spec: requirement.spec.clone(),
            range: assignment.range.clone(),
            total,
            targets,
            min_per_target: requirement.distribution_min_per_target,
        });
    }
    Ok(())
}

pub(super) fn target_distribution_context(
    game: &GameState,
    player: PlayerId,
    source: ObjectId,
    pending: &PendingTargetDistribution,
) -> crate::decisions::context::DistributeContext {
    let targets = pending
        .targets
        .iter()
        .map(|target| crate::decisions::context::DistributeTarget {
            target: *target,
            name: match target {
                Target::Object(id) => game
                    .object(*id)
                    .map(|object| object.name.to_string())
                    .unwrap_or_else(|| format!("Object #{}", id.0)),
                Target::Player(id) => format!("Player {}", id.index() + 1),
            },
        })
        .collect();
    crate::decisions::context::DistributeContext::new(
        player,
        Some(source),
        format!("Divide {} among the chosen targets", pending.total),
        pending.total,
        targets,
        pending.min_per_target,
    )
}

fn normalized_target_distribution(
    requirement: &PendingTargetDistribution,
    response: &[(Target, u32)],
) -> Result<Vec<(Target, u32)>, GameLoopError> {
    let mut amounts = std::collections::HashMap::new();
    for (target, amount) in response {
        if !requirement.targets.contains(target)
            || *amount < requirement.min_per_target
            || amounts.insert(*target, *amount).is_some()
        {
            return Err(GameLoopError::ActionCancelled(
                "announced distribution contains an invalid target, amount, or duplicate"
                    .to_string(),
            ));
        }
    }
    if amounts.len() != requirement.targets.len()
        || amounts.values().copied().sum::<u32>() != requirement.total
    {
        return Err(GameLoopError::ActionCancelled(format!(
            "announced distribution must assign exactly {} among every chosen target",
            requirement.total
        )));
    }
    Ok(requirement
        .targets
        .iter()
        .map(|target| (*target, amounts[target]))
        .collect())
}

pub(super) fn continue_cast_target_distributions_or_mana_payment(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    mut pending: PendingCast,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    if let Some(requirement) = pending.pending_target_distributions.front() {
        pending.stage = CastStage::ChoosingDistribution;
        let ctx = target_distribution_context(game, pending.caster, pending.spell_id, requirement);
        state.pending_cast = Some(pending);
        return Ok(GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::Distribute(ctx),
        ));
    }
    let targets = pending.chosen_targets.clone();
    continue_to_mana_payment(game, trigger_queue, state, pending, targets, decision_maker)
}

pub(super) fn apply_target_distribution_response(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    response: &[(Target, u32)],
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    if let Some(mut pending) = state.pending_cast.take() {
        if pending.stage != CastStage::ChoosingDistribution {
            state.pending_cast = Some(pending);
        } else {
            let requirement = pending
                .pending_target_distributions
                .front()
                .cloned()
                .ok_or_else(|| {
                    GameLoopError::InvalidState(
                        "spell distribution stage has no pending requirement".to_string(),
                    )
                })?;
            let allocations = match normalized_target_distribution(&requirement, response) {
                Ok(allocations) => allocations,
                Err(error) => {
                    state.rollback_action(game);
                    return Err(error);
                }
            };
            pending.pending_target_distributions.pop_front();
            pending
                .target_distributions
                .push(crate::game_state::TargetDistribution {
                    spec: requirement.spec,
                    range: requirement.range,
                    allocations,
                });
            return continue_cast_target_distributions_or_mana_payment(
                game,
                trigger_queue,
                state,
                pending,
                decision_maker,
            );
        }
    }

    let mut pending = state.pending_activation.take().ok_or_else(|| {
        GameLoopError::InvalidState("No pending spell or activation distribution".to_string())
    })?;
    if pending.stage != ActivationStage::ChoosingDistribution {
        state.pending_activation = Some(pending);
        return Err(GameLoopError::InvalidState(
            "distribution response outside an announcement distribution stage".to_string(),
        ));
    }
    let requirement = pending
        .pending_target_distributions
        .front()
        .cloned()
        .ok_or_else(|| {
            GameLoopError::InvalidState(
                "activation distribution stage has no pending requirement".to_string(),
            )
        })?;
    let allocations = match normalized_target_distribution(&requirement, response) {
        Ok(allocations) => allocations,
        Err(error) => {
            state.rollback_action(game);
            return Err(error);
        }
    };
    pending.pending_target_distributions.pop_front();
    pending
        .target_distributions
        .push(crate::game_state::TargetDistribution {
            spec: requirement.spec,
            range: requirement.range,
            allocations,
        });
    pending.stage = activation_stage_after_targets(&pending);
    continue_activation(game, trigger_queue, state, pending, decision_maker)
}

pub(super) fn build_next_cost_context(
    player: PlayerId,
    source: ObjectId,
    source_name: String,
    mana_cost: Option<&crate::mana::ManaCost>,
    mana_option_legal: bool,
    remaining_cost_steps: &[ActivationCostStep],
) -> crate::decisions::context::SelectOptionsContext {
    let mut options = Vec::new();
    let mut next_index = 0usize;

    if let Some(cost) = mana_cost {
        options.push(crate::decisions::context::SelectableOption::with_legality(
            next_index,
            format!("Mana: {}", format_mana_cost_simple(cost)),
            mana_option_legal,
        ));
        next_index += 1;
    }

    for step in remaining_cost_steps {
        options.push(crate::decisions::context::SelectableOption::new(
            next_index,
            describe_pending_cost_step(step),
        ));
        next_index += 1;
    }

    crate::decisions::context::SelectOptionsContext::new(
        player,
        Some(source),
        format!("Choose the next cost to pay for {}", source_name),
        options,
        1,
        1,
    )
    .with_context_text(
        "Tapping resolves immediately. Other costs may open a follow-up payment prompt.",
    )
}

pub(super) fn activation_stage_after_announcements(pending: &PendingActivation) -> ActivationStage {
    if !pending.remaining_requirements.is_empty() {
        ActivationStage::ChoosingTargets
    } else {
        activation_stage_after_targets(pending)
    }
}

pub(super) fn remove_any_counters_among_effect(
    cost: &crate::costs::Cost,
) -> Option<&crate::effects::RemoveAnyCountersAmongEffect> {
    cost.effect_ref()?
        .downcast_ref::<crate::effects::RemoveAnyCountersAmongEffect>()
}

fn staged_remove_counters_among_allocations(
    game: &GameState,
    cost: &crate::effects::RemoveAnyCountersAmongEffect,
    source: ObjectId,
    payer: PlayerId,
    tagged_objects: &std::collections::HashMap<crate::tag::TagKey, Vec<ObjectSnapshot>>,
    distribution: Vec<(Target, u32)>,
) -> Result<std::collections::VecDeque<(ObjectId, u32)>, GameLoopError> {
    let valid_targets = crate::effects::remove_any_counters_among_valid_targets_with_tags(
        cost,
        game,
        source,
        payer,
        tagged_objects,
    );

    let mut allocations: std::collections::HashMap<ObjectId, u32> =
        std::collections::HashMap::new();
    for (target, amount) in distribution {
        if let Target::Object(object_id) = target {
            *allocations.entry(object_id).or_insert(0) += amount;
        }
    }

    let distributed_total: u32 = allocations.values().copied().sum();
    if distributed_total != cost.count {
        return Err(GameLoopError::InvalidState(format!(
            "counter distribution must assign exactly {} counters (got {})",
            cost.count, distributed_total
        )));
    }

    let mut ordered = std::collections::VecDeque::new();
    for object_id in valid_targets {
        let amount = allocations.remove(&object_id).unwrap_or(0);
        if amount > 0 {
            ordered.push_back((object_id, amount));
        }
    }
    Ok(ordered)
}

pub(super) fn continue_activation_remove_counters_among_payment(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    mut pending: PendingActivation,
    decision_maker: &mut impl DecisionMaker,
    provided_ctx: Option<&crate::decisions::context::DecisionContext>,
) -> Result<GameProgress, GameLoopError> {
    let pending_cost = pending
        .remaining_cost_steps
        .first()
        .and_then(|step| match step {
            ActivationCostStep::Cost(cost) => remove_any_counters_among_effect(cost).cloned(),
            _ => None,
        });
    let cost = if pending
        .pending_remove_counters_among
        .as_ref()
        .is_some_and(|staged| staged.distribution_ready)
    {
        pending
            .pending_remove_counters_among
            .as_ref()
            .map(|staged| staged.cost.clone())
            .expect("checked staged remove-counters-among cost")
    } else {
        pending_cost.ok_or_else(|| {
            GameLoopError::InvalidState(
                "No remove-counters-among activation cost is currently pending".to_string(),
            )
        })?
    };

    let requested_count = if cost.dynamic_count {
        let total_available = crate::effects::remove_any_counters_among_valid_targets_with_tags(
            &cost,
            game,
            pending.source,
            pending.activator,
            &pending.tagged_objects,
        )
        .into_iter()
        .filter_map(|id| game.object(id))
        .map(|object| {
            if let Some(counter_type) = cost.counter_type {
                object.counters.get(&counter_type).copied().unwrap_or(0)
            } else {
                object.counters.values().copied().sum::<u32>()
            }
        })
        .sum::<u32>();
        let max_count = cost.count.min(total_available);
        if pending.x_value.is_none() {
            if max_count < cost.min_count {
                return Err(GameLoopError::InvalidState(format!(
                    "not enough counters to pay remove-counters-among cost: need at least {}, have {}",
                    cost.min_count, max_count
                )));
            }
            let activator = pending.activator;
            let source = pending.source;
            state.pending_activation = Some(pending);
            let ctx = crate::decisions::context::NumberContext::new(
                activator,
                Some(source),
                cost.min_count,
                max_count,
                "Choose value for X",
            );
            return Ok(GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::Number(ctx),
            ));
        }
        pending
            .x_value
            .and_then(|value| u32::try_from(value).ok())
            .unwrap_or(cost.min_count)
            .clamp(cost.min_count, max_count)
    } else {
        cost.count
    };
    let mut cost = cost;
    cost.count = requested_count;

    let staged = pending
        .pending_remove_counters_among
        .get_or_insert_with(|| PendingRemoveCountersAmongChoice {
            cost: cost.clone(),
            distribution_ready: false,
            allocations: std::collections::VecDeque::new(),
            removed_total: 0,
        });

    if !staged.distribution_ready {
        let distribute_ctx =
            if let Some(crate::decisions::context::DecisionContext::Distribute(ctx)) = provided_ctx
            {
                ctx.clone()
            } else {
                let targets: Vec<Target> =
                    crate::effects::remove_any_counters_among_valid_targets_with_tags(
                        &cost,
                        game,
                        pending.source,
                        pending.activator,
                        &pending.tagged_objects,
                    )
                    .into_iter()
                    .map(Target::Object)
                    .collect();
                let spec = crate::decisions::specs::DistributeSpec::counters(
                    pending.source,
                    cost.count,
                    targets,
                );
                match crate::decisions::spec::DecisionSpec::build_context(
                    &spec,
                    pending.activator,
                    Some(pending.source),
                    game,
                ) {
                    crate::decisions::context::DecisionContext::Distribute(ctx) => ctx,
                    _ => {
                        unreachable!("counter distribution spec should build a distribute context")
                    }
                }
            };

        let distribution = decision_maker.decide_distribute(game, &distribute_ctx);
        if decision_maker.awaiting_choice() {
            state.pending_activation = Some(pending);
            return Ok(GameProgress::Continue);
        }

        staged.allocations = staged_remove_counters_among_allocations(
            game,
            &cost,
            pending.source,
            pending.activator,
            &pending.tagged_objects,
            distribution,
        )?;
        staged.distribution_ready = true;
    }

    let mut used_provided_counters_ctx = false;
    loop {
        let Some(staged) = pending.pending_remove_counters_among.as_mut() else {
            break;
        };
        let Some((object_id, amount_for_target)) = staged.allocations.front().copied() else {
            let removed_total = staged.removed_total;
            pending.pending_remove_counters_among = None;
            if removed_total != cost.count {
                return Err(GameLoopError::InvalidState(
                    "staged counter payment removed the wrong number of counters".to_string(),
                ));
            }
            let paid_cost = crate::costs::Cost::effect(cost.clone());
            record_immediate_cost_payment(&mut pending.payment_trace, &paid_cost, pending.source);
            pending.remaining_cost_steps.remove(0);
            drain_pending_trigger_events(game, trigger_queue);
            pending.stage = activation_stage_after_targets(&pending);
            return continue_activation(game, trigger_queue, state, pending, decision_maker);
        };

        if let Some(counter_type) = cost.counter_type {
            let removed = game
                .remove_counters(
                    object_id,
                    counter_type,
                    amount_for_target,
                    Some(pending.source),
                    Some(pending.activator),
                )
                .map(|(removed, event)| {
                    game.queue_trigger_event(pending.provenance, event);
                    removed
                })
                .unwrap_or(0);
            if removed != amount_for_target {
                return Err(GameLoopError::InvalidState(
                    "failed to remove the allocated counters".to_string(),
                ));
            }
            staged.removed_total += removed;
            staged.allocations.pop_front();
            continue;
        }

        let available_counters: Vec<(CounterType, u32)> = game
            .object(object_id)
            .map(|obj| {
                obj.counters
                    .iter()
                    .filter(|(_, count)| **count > 0)
                    .map(|(counter_type, count)| (*counter_type, *count))
                    .collect()
            })
            .unwrap_or_default();
        let available_total: u32 = available_counters.iter().map(|(_, count)| *count).sum();
        if available_total < amount_for_target {
            return Err(GameLoopError::InvalidState(
                "allocated target no longer has enough counters".to_string(),
            ));
        }

        let counters_ctx = if !used_provided_counters_ctx {
            if let Some(crate::decisions::context::DecisionContext::Counters(ctx)) = provided_ctx {
                if ctx.target == crate::game_state::Target::Object(object_id) {
                    used_provided_counters_ctx = true;
                    Some(ctx.clone())
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
        .unwrap_or_else(|| {
            let spec = crate::decisions::specs::CounterRemovalSpec::new(
                pending.source,
                object_id,
                amount_for_target,
                available_counters.clone(),
            );
            match crate::decisions::spec::DecisionSpec::build_context(
                &spec,
                pending.activator,
                Some(pending.source),
                game,
            ) {
                crate::decisions::context::DecisionContext::Counters(ctx) => ctx,
                _ => unreachable!("counter removal spec should build a counters context"),
            }
        });

        let selections = decision_maker.decide_counters(game, &counters_ctx);
        if decision_maker.awaiting_choice() {
            state.pending_activation = Some(pending);
            return Ok(GameProgress::Continue);
        }

        let mut removed_from_target = 0u32;
        for (counter_type, requested) in selections {
            if removed_from_target >= amount_for_target {
                break;
            }
            let remaining = amount_for_target - removed_from_target;
            let to_remove = requested.min(remaining);
            if to_remove == 0 {
                continue;
            }
            if let Some((removed, event)) = game.remove_counters(
                object_id,
                counter_type,
                to_remove,
                Some(pending.source),
                Some(pending.activator),
            ) {
                game.queue_trigger_event(pending.provenance, event);
                removed_from_target += removed;
            }
        }
        if removed_from_target != amount_for_target {
            return Err(GameLoopError::InvalidState(
                "failed to remove the requested counters".to_string(),
            ));
        }
        staged.removed_total += removed_from_target;
        staged.allocations.pop_front();
    }

    Err(GameLoopError::InvalidState(
        "remove-counters-among payment fell through unexpectedly".to_string(),
    ))
}

pub(super) fn continue_activation_cost_payment(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    mut pending: PendingActivation,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    let Some(step) = pending.remaining_cost_steps.first().cloned() else {
        pending.stage = activation_stage_after_targets(&pending);
        return continue_activation(game, trigger_queue, state, pending, decision_maker);
    };

    match step {
        ActivationCostStep::Cost(cost) => {
            if remove_any_counters_among_effect(&cost).is_none()
                && cost.display().to_ascii_lowercase().contains("from among")
            {
                return Err(GameLoopError::InvalidState(format!(
                    "remove-counters-among cost lost effect-backed staged type: {:?}",
                    cost
                )));
            }
            if pending.pending_remove_counters_among.is_some()
                || remove_any_counters_among_effect(&cost).is_some()
            {
                return continue_activation_remove_counters_among_payment(
                    game,
                    trigger_queue,
                    state,
                    pending,
                    decision_maker,
                    None,
                );
            }

            let mut cost_ctx =
                CostContext::new(pending.source, pending.activator, &mut *decision_maker)
                    .with_reason(pending.payment_reason)
                    .with_provenance(pending.provenance);
            cost_ctx.tagged_objects = pending.tagged_objects.clone();
            cost_ctx.x_value = pending.x_value.and_then(|x| u32::try_from(x).ok());
            let pre_cost_source_snapshot = game.object(pending.source).map(|obj| {
                crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                    obj, game,
                )
            });

            let payment = cost.pay(game, &mut cost_ctx).map_err(|err| {
                GameLoopError::InvalidState(format!(
                    "Failed to pay deferred activation cost {}: {err:?}",
                    cost.display()
                ))
            })?;
            if cost_ctx.decision_maker.awaiting_choice() {
                state.pending_activation = Some(pending);
                return Ok(GameProgress::Continue);
            }

            match payment {
                crate::costs::CostPaymentResult::Paid => {
                    record_immediate_cost_payment(
                        &mut pending.payment_trace,
                        &cost,
                        pending.source,
                    );
                    pending.source_snapshot = if let Some(obj) = game.object(pending.source) {
                        crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                            obj, game,
                        )
                    } else {
                        pre_cost_source_snapshot.unwrap_or(pending.source_snapshot)
                    };
                    if pending.x_value.is_none() {
                        pending.x_value = cost_ctx.x_value.map(|x| x as usize);
                    }
                    pending.tagged_objects = cost_ctx.tagged_objects;
                    pending.remaining_cost_steps.remove(0);
                    drain_pending_trigger_events(game, trigger_queue);
                    pending.stage = activation_stage_after_targets(&pending);
                    continue_activation(game, trigger_queue, state, pending, decision_maker)
                }
                crate::costs::CostPaymentResult::NeedsChoice(description) => {
                    Err(GameLoopError::InvalidState(format!(
                        "Deferred activation cost unexpectedly requires staged choice: {} ({})",
                        cost.display(),
                        description
                    )))
                }
            }
        }
        ActivationCostStep::Sacrifice {
            ref filter,
            ref description,
            ..
        } => {
            let legal_targets = get_legal_sacrifice_targets(
                game,
                pending.activator,
                pending.source,
                filter,
                pending.payment_reason,
            );

            if legal_targets.is_empty() {
                return Err(GameLoopError::InvalidState(
                    "No valid sacrifice targets".to_string(),
                ));
            }

            if deterministic_named_source_cost(
                game,
                pending.source,
                filter,
                description,
                &legal_targets,
            ) {
                pending.stage = ActivationStage::ChoosingSacrifice;
                state.pending_activation = Some(pending);
                return apply_sacrifice_target_response(
                    game,
                    trigger_queue,
                    state,
                    legal_targets[0],
                    decision_maker,
                );
            }

            let player = pending.activator;
            let source = pending.source;
            pending.stage = ActivationStage::ChoosingSacrifice;
            state.pending_activation = Some(pending);

            let candidates: Vec<crate::decisions::context::SelectableObject> = legal_targets
                .iter()
                .map(|&id| {
                    let name = game
                        .object(id)
                        .map(|o| o.name.to_string())
                        .unwrap_or_else(|| format!("Permanent #{}", id.0));
                    crate::decisions::context::SelectableObject::new(id, name)
                })
                .collect();
            let ctx = crate::decisions::context::SelectObjectsContext::new(
                player,
                Some(source),
                format!("Choose a creature to sacrifice: {}", description),
                candidates,
                1,
                Some(1),
            )
            .with_reveal_policy(crate::decisions::context::SelectionRevealPolicy::Public);
            Ok(GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::SelectObjects(ctx),
            ))
        }
        ActivationCostStep::CardChoice(card_choice_cost) => {
            let (description, legal_cards) = card_cost_choice_description_and_candidates(
                game,
                pending.activator,
                pending.source,
                &card_choice_cost,
                &[],
            );

            if legal_cards.is_empty() {
                return Err(GameLoopError::InvalidState(
                    "No valid cards available for activation cost choice".to_string(),
                ));
            }

            if deterministic_named_source_card_cost(
                game,
                pending.source,
                &card_choice_cost,
                &legal_cards,
            ) {
                pending.stage = ActivationStage::ChoosingCardCost;
                state.pending_activation = Some(pending);
                return apply_sacrifice_target_response(
                    game,
                    trigger_queue,
                    state,
                    legal_cards[0],
                    decision_maker,
                );
            }

            let player = pending.activator;
            let source = pending.source;
            pending.stage = ActivationStage::ChoosingCardCost;
            state.pending_activation = Some(pending);

            let candidates: Vec<crate::decisions::context::SelectableObject> = legal_cards
                .iter()
                .map(|&id| {
                    let name = game
                        .object(id)
                        .map(|o| o.name.to_string())
                        .unwrap_or_else(|| format!("Card #{}", id.0));
                    crate::decisions::context::SelectableObject::new(id, name)
                })
                .collect();
            let ctx = crate::decisions::context::SelectObjectsContext::new(
                player,
                Some(source),
                description,
                candidates,
                1,
                Some(1),
            )
            .with_reveal_policy(card_cost_choice_reveal_policy(&card_choice_cost));
            Ok(GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::SelectObjects(ctx),
            ))
        }
    }
}

/// Continue the activation process based on current stage.
pub(super) fn continue_activation(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    mut pending: PendingActivation,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    // Activation legality has already been checked and ability data is captured in
    // PendingActivation. Targets are chosen before costs are paid; once payment begins,
    // the player selects which remaining cost to satisfy next.

    loop {
        match pending.stage {
            ActivationStage::ChoosingModes => {
                return check_activation_modes_or_continue(
                    game,
                    trigger_queue,
                    state,
                    pending,
                    decision_maker,
                );
            }
            ActivationStage::ChoosingAlternativeCost => {
                let view = crate::derived_view::DerivedGameView::new(game);
                let options = pending
                    .alternative_cost_branches
                    .iter()
                    .enumerate()
                    .map(|(index, branch)| {
                        let legal =
                            crate::decision::activation_total_cost_branch_is_payable_with_view(
                                game,
                                pending.activator,
                                pending.source,
                                branch,
                                &view,
                            );
                        crate::decisions::context::SelectableOption::with_legality(
                            index,
                            branch.display(),
                            legal,
                        )
                    })
                    .collect();
                let ability_name = game
                    .object(pending.source)
                    .map(|object| format!("{}'s ability", object.name))
                    .unwrap_or_else(|| "ability".to_string());
                let context = crate::decisions::context::SelectOptionsContext::new(
                    pending.activator,
                    Some(pending.source),
                    format!("Choose an activation cost for {ability_name}"),
                    options,
                    1,
                    1,
                );
                state.pending_activation = Some(pending);
                return Ok(GameProgress::NeedsDecisionCtx(
                    crate::decisions::context::DecisionContext::SelectOptions(context),
                ));
            }
            ActivationStage::ChoosingX => {
                // Need to choose X value first
                let mut max_x = if let Some(ref cost) = pending.mana_cost_to_pay {
                    let mana_spend_policy =
                        game.mana_spend_policy(pending.activator, Some(pending.source));
                    let allow_black_life = crate::decision::mana_cost_has_black_symbol(cost)
                        && game.player_can_pay_black_with_life_for_reason(
                            pending.activator,
                            Some(pending.source),
                            pending.payment_reason,
                        );
                    compute_potential_mana(game, pending.activator)
                        .max_x_for_cost_with_mana_spend_policy_and_black_life(
                            cost,
                            &mana_spend_policy,
                            allow_black_life,
                        )
                        .into()
                } else {
                    None
                };
                if let Some(cost_max_x) = max_x_from_activation_cost_steps(
                    game,
                    pending.activator,
                    pending.source,
                    &pending.remaining_cost_steps,
                ) {
                    max_x = Some(max_x.map_or(cost_max_x, |mana_max| mana_max.min(cost_max_x)));
                }
                let max_x = max_x.unwrap_or(0);
                let min_x = game
                    .current_ability(pending.source, pending.ability_index)
                    .and_then(|ability| match &ability.kind {
                        crate::ability::AbilityKind::Activated(activated) => {
                            Some(activated.activation_x_minimum())
                        }
                        _ => None,
                    })
                    .unwrap_or(0);
                if min_x > max_x {
                    return Err(GameLoopError::InvalidState(format!(
                        "No legal X value between {min_x} and {max_x} for this activation"
                    )));
                }

                state.pending_activation = Some(pending.clone());

                let ctx = crate::decisions::context::NumberContext::x_value_with_min(
                    pending.activator,
                    pending.source,
                    min_x,
                    max_x,
                );
                return Ok(GameProgress::NeedsDecisionCtx(
                    crate::decisions::context::DecisionContext::Number(ctx),
                ));
            }
            ActivationStage::ProcessingCosts => {
                return continue_activation_cost_payment(
                    game,
                    trigger_queue,
                    state,
                    pending,
                    decision_maker,
                );
            }
            ActivationStage::ChoosingNextCost => {
                if pending.mana_cost_to_pay.is_some() && pending.pending_mana_payment.is_none() {
                    return prompt_activation_mana_ability_window(
                        game,
                        trigger_queue,
                        state,
                        pending,
                        decision_maker,
                    );
                }
                auto_pay_activation_tap_cost_steps(
                    game,
                    trigger_queue,
                    &mut pending,
                    decision_maker,
                )?;
                let option_count = usize::from(pending.mana_cost_to_pay.is_some())
                    + pending.remaining_cost_steps.len();
                if option_count == 0 {
                    pending.stage = ActivationStage::ReadyToFinalize;
                    continue;
                }
                if option_count == 1 {
                    if pending.mana_cost_to_pay.is_some() {
                        let payment = pending.pending_mana_payment.take().ok_or_else(|| {
                            GameLoopError::InvalidState(
                                "activation mana sources were not prepared before cost payment"
                                    .to_string(),
                            )
                        })?;
                        return commit_prepared_activation_mana_payment(
                            game,
                            trigger_queue,
                            state,
                            pending,
                            payment,
                            decision_maker,
                        );
                    } else {
                        pending.stage = ActivationStage::ProcessingCosts;
                    }
                    continue;
                }

                let ability_name = game
                    .object(pending.source)
                    .map(|o| format!("{}'s ability", o.name))
                    .unwrap_or_else(|| "ability".to_string());
                let ctx = build_next_cost_context(
                    pending.activator,
                    pending.source,
                    ability_name,
                    pending.mana_cost_to_pay.as_ref(),
                    pending.pending_mana_payment.is_some(),
                    &pending.remaining_cost_steps,
                );
                state.pending_activation = Some(pending);
                return Ok(GameProgress::NeedsDecisionCtx(
                    crate::decisions::context::DecisionContext::SelectOptions(ctx),
                ));
            }
            ActivationStage::ChoosingSacrifice | ActivationStage::ChoosingCardCost => {
                state.pending_activation = Some(pending);
                return Err(GameLoopError::InvalidState(
                    "Activation object-cost stage requires a SelectObjects response".to_string(),
                ));
            }
            ActivationStage::AnnouncingCost => {
                // Handle hybrid/Phyrexian mana announcement (per MTG rule 601.2b via 602.2b)
                if pending.pending_hybrid_pips.is_empty() {
                    // All hybrid pips announced - validate that we can still pay the cost
                    // This is necessary because max_x was calculated assuming life payment for Phyrexian pips,
                    // but the player may have chosen mana payment instead
                    if let Some(ref cost) = pending.mana_cost_to_pay {
                        let x_value = pending.x_value.unwrap_or(0);
                        let expanded_pips =
                            expand_mana_cost_to_pips(cost, x_value, &pending.hybrid_choices);
                        let potential = compute_potential_mana(game, pending.activator);

                        // Check if we can pay all the expanded pips
                        let total_mana_needed: usize = expanded_pips
                            .iter()
                            .filter(|pip| {
                                !pip.iter()
                                    .any(|s| matches!(s, crate::mana::ManaSymbol::Life(_)))
                            })
                            .count();

                        if potential.total() < total_mana_needed as u32 {
                            return Err(GameLoopError::InvalidState(format!(
                                "Cannot afford ability: need {} mana but only have {} available. \
                            Consider paying life for Phyrexian mana or choosing a lower X value.",
                                total_mana_needed,
                                potential.total()
                            )));
                        }
                    }

                    pending.stage = activation_stage_after_announcements(&pending);
                    continue;
                }

                // Prompt for the next hybrid pip
                let (pip_idx, alternatives) = pending.pending_hybrid_pips[0].clone();
                let player = pending.activator;
                let source = pending.source;
                let ability_name = game
                    .object(source)
                    .map(|o| format!("{}'s ability", o.name))
                    .unwrap_or_else(|| "ability".to_string());

                // Build hybrid options for each alternative
                let options: Vec<crate::decisions::context::HybridOption> = alternatives
                    .iter()
                    .enumerate()
                    .map(|(i, sym)| crate::decisions::context::HybridOption {
                        index: i,
                        label: format_mana_symbol_for_choice(sym),
                        symbol: *sym,
                    })
                    .collect();

                state.pending_activation = Some(pending);

                // Create a HybridChoice decision for this pip
                let ctx = crate::decisions::context::HybridChoiceContext::new(
                    player,
                    Some(source),
                    ability_name,
                    pip_idx + 1, // 1-based for display
                    options,
                );
                return Ok(GameProgress::NeedsDecisionCtx(
                    crate::decisions::context::DecisionContext::HybridChoice(ctx),
                ));
            }
            ActivationStage::ChoosingTargets => {
                if pending.remaining_requirements.is_empty() {
                    pending.stage = activation_stage_after_targets(&pending);
                    continue;
                } else {
                    let requirement = pending.remaining_requirements[0].clone();
                    let player = pending.activator;
                    let source = pending.source;
                    let context = game
                        .object(source)
                        .map(|o| format!("{}'s ability", o.name))
                        .unwrap_or_else(|| "ability".to_string());

                    let chooser =
                        match resolved_next_target_chooser(game, player, source, &requirement)? {
                            Ok(chooser) => chooser,
                            Err(candidates) => {
                                pending.stage = ActivationStage::ChoosingTargetChooser;
                                pending.pending_target_chooser_candidates = candidates.clone();
                                let ctx = target_chooser_context(
                                    game,
                                    player,
                                    source,
                                    context,
                                    &candidates,
                                );
                                state.pending_activation = Some(pending);
                                return Ok(GameProgress::NeedsDecisionCtx(
                                    crate::decisions::context::DecisionContext::SelectOptions(ctx),
                                ));
                            }
                        };
                    let requirement_count = pending
                        .remaining_requirements
                        .iter()
                        .take_while(|candidate| {
                            matches!(
                                resolved_next_target_chooser(game, player, source, candidate),
                                Ok(Ok(candidate_chooser)) if candidate_chooser == chooser
                            )
                        })
                        .count();
                    for requirement in pending
                        .remaining_requirements
                        .iter_mut()
                        .take(requirement_count)
                    {
                        specialize_target_requirement_for_chooser(
                            game,
                            player,
                            source,
                            chooser,
                            requirement,
                        );
                    }
                    let requirements = pending.remaining_requirements[..requirement_count].to_vec();
                    pending.stage = ActivationStage::ChoosingTargets;
                    pending.active_target_requirement_count = requirements.len();

                    state.pending_activation = Some(pending);

                    // Convert to TargetsContext
                    let ctx = crate::decisions::context::TargetsContext::new(
                        chooser,
                        source,
                        context,
                        requirements
                            .into_iter()
                            .map(|r| crate::decisions::context::TargetRequirementContext {
                                description: r.description,
                                legal_targets: r.legal_targets,
                                legal_target_sets: r.legal_target_sets,
                                aggregate_constraint: r.aggregate_constraint,
                                min_targets: r.min_targets,
                                max_targets: r.max_targets,
                                distinct_player_group: r.distinct_player_group,
                            })
                            .collect(),
                    );
                    return Ok(GameProgress::NeedsDecisionCtx(
                        crate::decisions::context::DecisionContext::Targets(ctx),
                    ));
                }
            }
            ActivationStage::ChoosingTargetChooser => {
                let context = game
                    .object(pending.source)
                    .map(|object| format!("{}'s ability", object.name))
                    .unwrap_or_else(|| "ability".to_string());
                let ctx = target_chooser_context(
                    game,
                    pending.activator,
                    pending.source,
                    context,
                    &pending.pending_target_chooser_candidates,
                );
                state.pending_activation = Some(pending);
                return Ok(GameProgress::NeedsDecisionCtx(
                    crate::decisions::context::DecisionContext::SelectOptions(ctx),
                ));
            }
            ActivationStage::ChoosingDistribution => {
                let requirement =
                    pending
                        .pending_target_distributions
                        .front()
                        .ok_or_else(|| {
                            GameLoopError::InvalidState(
                                "activation distribution stage has no pending requirement"
                                    .to_string(),
                            )
                        })?;
                let ctx = target_distribution_context(
                    game,
                    pending.activator,
                    pending.source,
                    requirement,
                );
                state.pending_activation = Some(pending);
                return Ok(GameProgress::NeedsDecisionCtx(
                    crate::decisions::context::DecisionContext::Distribute(ctx),
                ));
            }
            ActivationStage::PayingMana => {
                let payment = pending.pending_mana_payment.as_ref().ok_or_else(|| {
                    GameLoopError::InvalidState(
                        "activation is in the payment stage without an authoritative plan"
                            .to_string(),
                    )
                })?;
                let ctx = crate::decisions::context::ManaPaymentContext::new(
                    payment.request.payer,
                    payment.request.source,
                    format!("{}'s ability", pending.source_name),
                    payment.request.clone(),
                    payment.plan.clone(),
                );
                state.pending_activation = Some(pending);
                return Ok(GameProgress::NeedsDecisionCtx(
                    crate::decisions::context::DecisionContext::ManaPayment(ctx),
                ));
            }
            ActivationStage::ReadyToFinalize => {
                // Record activation for per-turn-limited abilities
                if pending.is_once_per_turn {
                    game.record_ability_activation(pending.source, pending.ability_index);
                }
                if pending.is_loyalty_ability {
                    game.record_loyalty_ability_activation(pending.source);
                }

                // Create ability stack entry with targets
                let mut entry =
                    StackEntry::ability(pending.source, pending.activator, pending.effects.clone())
                        .with_ability_index(pending.ability_index)
                        .with_activation_cost_has_x(pending.activation_cost_has_x)
                        .with_activation_cost_has_tap(pending.activation_cost_has_tap)
                        .with_mana_spent_on_activation(pending.mana_spent_on_activation.clone())
                        .with_provenance(pending.provenance)
                        .with_source_info(pending.source_stable_id, pending.source_name.clone())
                        .with_source_snapshot(pending.source_snapshot.clone())
                        .with_chosen_modes(pending.chosen_modes.clone())
                        .with_target_distributions(pending.target_distributions.clone())
                        .with_mana_usage_restrictions(
                            pending.mana_usage_restrictions.clone(),
                            pending.mana_source_chosen_creature_type,
                        )
                        .with_tagged_objects(pending.tagged_objects.clone());
                entry.targets = pending.chosen_targets.clone();
                entry.target_assignments = pending.chosen_target_assignments.clone();

                // Pass X value to stack entry so it's available during resolution
                if let Some(x) = pending.x_value {
                    entry = entry.with_x(x as u32);
                }

                game.push_to_stack(entry);
                queue_becomes_targeted_events(
                    game,
                    trigger_queue,
                    &pending.chosen_targets,
                    pending.source,
                    pending.activator,
                    true,
                    pending.provenance,
                );
                queue_ability_activated_event(
                    game,
                    trigger_queue,
                    &mut *decision_maker,
                    pending.source,
                    pending.activator,
                    false,
                    Some(pending.source_stable_id),
                    pending.activation_cost_has_tap,
                );

                // Clear pending state and checkpoint - action completed successfully
                state.pending_activation = None;
                state.clear_checkpoint();
                priority_after_player_action(game, &mut state.tracker, pending.activator);
                return advance_priority_with_dm(game, trigger_queue, decision_maker);
            }
        }
    }
}

pub(super) fn auto_pay_activation_tap_cost_steps(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    pending: &mut PendingActivation,
    decision_maker: &mut impl DecisionMaker,
) -> Result<(), GameLoopError> {
    loop {
        let Some(index) = pending.remaining_cost_steps.iter().position(|step| {
            matches!(
                step,
                ActivationCostStep::Cost(cost) if cost.requires_tap() || cost.requires_untap()
            )
        }) else {
            return Ok(());
        };

        let ActivationCostStep::Cost(cost) = pending.remaining_cost_steps.remove(index) else {
            unreachable!("tap/untap auto-payment only matches cost steps");
        };

        let mut cost_ctx =
            CostContext::new(pending.source, pending.activator, &mut *decision_maker)
                .with_provenance(pending.provenance);
        cost_ctx.tagged_objects = pending.tagged_objects.clone();
        cost_ctx.x_value = pending.x_value.and_then(|x| u32::try_from(x).ok());

        match cost.pay(game, &mut cost_ctx).map_err(|err| {
            GameLoopError::InvalidState(format!(
                "Failed to auto-pay activation tap cost {}: {err:?}",
                describe_cost_component(&cost)
            ))
        })? {
            crate::costs::CostPaymentResult::Paid => {
                record_immediate_cost_payment(&mut pending.payment_trace, &cost, pending.source);
                pending.tagged_objects = cost_ctx.tagged_objects;
                drain_pending_trigger_events(game, trigger_queue);
            }
            crate::costs::CostPaymentResult::NeedsChoice(description) => {
                return Err(GameLoopError::InvalidState(format!(
                    "Activation tap cost unexpectedly requires choice: {} ({description})",
                    describe_cost_component(&cost)
                )));
            }
        }
    }
}
