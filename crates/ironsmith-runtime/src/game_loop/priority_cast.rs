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
    let mut object_for_filter = object.clone();
    if let Some(chars) = game.current_characteristics(spell_id) {
        object_for_filter.name = chars.name;
        object_for_filter.card_types = chars.card_types;
        object_for_filter.subtypes = chars.subtypes;
        object_for_filter.supertypes = chars.supertypes;
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
    if effect_count > 0 {
        return effect_count;
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

fn ensure_granted_conspire_optional_costs(game: &mut GameState, pending: &mut PendingCast) {
    let conspire_count = granted_conspire_count(game, pending.spell_id, pending.caster);
    if conspire_count == 0 {
        return;
    }

    let Some(spell) = game.object_mut(pending.spell_id) else {
        return;
    };
    for _ in 0..conspire_count {
        spell.optional_costs.push(crate::cost::OptionalCost::custom(
            "Granted Conspire",
            crate::cost::TotalCost::from_cost(crate::costs::Cost::effect(
                crate::effects::ConspireCostEffect::new(),
            )),
        ));
    }
    pending.optional_costs_paid = crate::cost::OptionalCostsPaid::from_costs(&spell.optional_costs);
}

fn ensure_optional_life_cost_reduction_costs(game: &mut GameState, pending: &mut PendingCast) {
    let costs = crate::decision::optional_life_cost_reduction_costs_for_cast(
        game,
        pending.caster,
        pending.spell_id,
        &pending.casting_method,
    );
    if costs.is_empty() {
        return;
    }
    let Some(spell) = game.object_mut(pending.spell_id) else {
        return;
    };
    for (source, optional) in costs {
        let label = crate::decision::optional_life_cost_reduction_label(&optional, source);
        if spell
            .optional_costs
            .iter()
            .any(|existing| existing.label == label)
        {
            continue;
        }
        spell.optional_costs.push(crate::cost::OptionalCost::custom(
            label,
            crate::cost::TotalCost::from_cost(crate::costs::Cost::life(optional.life_cost)),
        ));
    }
    pending.optional_costs_paid = crate::cost::OptionalCostsPaid::from_costs(&spell.optional_costs);
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
            .map(format_mana_cost_simple)
            .unwrap_or_else(|| "0".to_string());
        let name = if spell.linked_face_layout == crate::card::LinkedFaceLayout::Split {
            spell.name.clone()
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
                    .map(format_mana_cost_simple)
                    .unwrap_or_else(|| "0".to_string());
                methods.push(CastingMethodOption {
                    method: CastingMethod::SplitOtherHalf,
                    name: other_def.card.name.clone(),
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
                .map(format_mana_cost_simple)
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
                .or_else(|| spell.cast_alternative_method.clone())
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
    for ability in &spell.abilities {
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
) -> (bool, u32) {
    let Some(spell) = game.object(stack_id) else {
        return (false, 0);
    };

    let printed_has_x = spell.mana_cost.as_ref().is_some_and(|cost| cost.has_x());
    let pay_has_x = mana_cost_to_pay.is_some_and(|cost| cost.has_x());

    let mut non_mana_costs = non_mana_costs_for_casting_method(game, caster, spell, casting_method);
    non_mana_costs.extend(spell.additional_non_mana_costs());

    let costs_need_x = non_mana_costs.iter().any(cost_references_x);
    let needs_x = printed_has_x || pay_has_x || costs_need_x;
    if !needs_x {
        return (false, 0);
    }

    let mut max_x = None;

    if pay_has_x && let Some(cost) = mana_cost_to_pay {
        let mana_spend_policy = game.mana_spend_policy(caster, Some(stack_id));
        let allow_black_life = game.player_can_pay_black_with_life_for_reason(
            caster,
            Some(stack_id),
            crate::costs::PaymentReason::CastSpell,
        );
        max_x = Some(
            compute_potential_mana(game, caster)
                .max_x_for_cost_with_mana_spend_policy_and_black_life(
                    cost,
                    &mana_spend_policy,
                    allow_black_life,
                ),
        );
    }

    if let Some(max_cost) = max_x_from_non_mana_costs(game, caster, stack_id, &non_mana_costs) {
        max_x = Some(max_x.map_or(max_cost, |prev| prev.min(max_cost)));
    }

    if let Some(max_static) = max_x_from_static_abilities(game, caster, stack_id) {
        max_x = Some(max_x.map_or(max_static, |prev| prev.min(max_static)));
    }

    (true, max_x.unwrap_or(0))
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
                .map(format_mana_cost_simple)
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
        AlternativeCastingMethod::Awaken { cost, .. } => {
            let cost_desc = format_mana_cost_simple(cost);
            ("Awaken".to_string(), cost_desc)
        }
        AlternativeCastingMethod::Flashback { .. } => {
            let cost_desc = method
                .mana_cost()
                .map(format_mana_cost_simple)
                .unwrap_or_else(|| "0".to_string());
            ("Flashback".to_string(), cost_desc)
        }
        AlternativeCastingMethod::Harmonize { .. } => {
            let cost_desc = method
                .mana_cost()
                .map(format_mana_cost_simple)
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
        AlternativeCastingMethod::JumpStart => {
            // Jump-start uses the spell's mana cost plus discarding a card
            let cost_desc = spell
                .mana_cost
                .as_ref()
                .map(format_mana_cost_simple)
                .unwrap_or_else(|| "0".to_string());
            (
                "Jump-Start".to_string(),
                format!("{}, Discard a card", cost_desc),
            )
        }
        AlternativeCastingMethod::Escape { cost, exile_count } => {
            let cost_desc = cost
                .as_ref()
                .map(format_mana_cost_simple)
                .or_else(|| spell.mana_cost.as_ref().map(format_mana_cost_simple))
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

/// Check for modal effects and either prompt for mode selection or continue to optional costs.
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
        let max_modes = resolve_modal_count_value(
            &modal_spec.max_modes,
            pending.x_value,
            modal_spec.mode_descriptions.len().max(1),
        );
        let min_modes =
            resolve_modal_count_value(&modal_spec.min_modes, pending.x_value, max_modes);

        let spell_name = game
            .object(source)
            .map(|o| o.name.clone())
            .unwrap_or_else(|| "spell".to_string());

        if !spell_has_legal_targets(game, &spell_effects, player, Some(source)) {
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
        // No modal effects, continue to optional costs
        check_optional_costs_or_continue(game, trigger_queue, state, pending, decision_maker)
    }
}

pub(super) fn activation_stage_after_modes(pending: &PendingActivation) -> ActivationStage {
    if pending.hybrid_choices.is_empty() && !pending.pending_hybrid_pips.is_empty() {
        ActivationStage::AnnouncingCost
    } else {
        activation_stage_after_announcements(pending)
    }
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
    let base_cost = crate::decision::spell_mana_cost_for_cast(
        game,
        pending.caster,
        spell,
        &pending.casting_method,
        pending.from_zone,
    )?;

    let mut optional_costs_paid = pending.optional_costs_paid.clone();
    optional_costs_paid.pay_times(optional_cost_index, 1);

    let mut hypothetical_spell = spell.clone();
    hypothetical_spell.optional_costs_paid = optional_costs_paid.clone();
    let combined_cost =
        mana_cost_with_paid_optional_costs(&base_cost, &hypothetical_spell, &optional_costs_paid);
    let effective_cost =
        crate::decision::calculate_effective_mana_cost_for_payment_with_chosen_targets_for_casting_method(
            game,
            pending.caster,
            &hypothetical_spell,
            &combined_cost,
            &pending.chosen_targets,
            &pending.casting_method,
        );

    Some(crate::decision::can_potentially_pay(
        game,
        pending.caster,
        &effective_cost,
        pending.x_value.unwrap_or(0),
    ))
}

/// Check for optional costs and either prompt for them or continue to targeting/finalization.
///
/// This is called after X value is chosen (or when there's no X cost).
/// Returns the next decision needed or continues the cast.
pub(super) fn check_optional_costs_or_continue(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    mut pending: PendingCast,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    ensure_granted_conspire_optional_costs(game, &mut pending);
    ensure_optional_life_cost_reduction_costs(game, &mut pending);

    // Check if the spell has optional costs
    let optional_costs = if let Some(obj) = game.object(pending.spell_id) {
        obj.optional_costs.clone()
    } else {
        Vec::new()
    };

    if optional_costs.is_empty() {
        // No optional costs, continue to targeting or finalization
        continue_to_targeting_or_finalize(game, trigger_queue, state, pending, decision_maker)
    } else {
        // Build the optional cost options for the decision
        let player = pending.caster;
        let source = pending.spell_id;

        // Check which costs the player can afford (using potential mana)
        let options: Vec<OptionalCostOption> = optional_costs
            .iter()
            .enumerate()
            .map(|(index, opt_cost)| {
                // Check if player can afford this cost with potential mana
                let affordable = if let Some(mana_cost) = opt_cost.cost.mana_cost() {
                    optional_mana_cost_is_affordable_with_spell_modifiers(game, &pending, index)
                        .unwrap_or_else(|| {
                            let adjusted_cost = game.adjust_mana_cost_for_payment_reason(
                                player,
                                Some(source),
                                mana_cost,
                                crate::costs::PaymentReason::CastSpell,
                            );
                            crate::decision::can_potentially_pay(game, player, &adjusted_cost, 0)
                        })
                } else {
                    // For non-mana costs, use the regular check
                    crate::cost::can_pay_cost_with_reason(
                        game,
                        source,
                        player,
                        &opt_cost.cost,
                        crate::costs::PaymentReason::CastSpell,
                    )
                    .is_ok()
                };

                // Format the cost description
                let cost_description = if let Some(mana) = opt_cost.cost.mana_cost() {
                    format!("{}", mana.mana_value())
                } else {
                    "special".to_string()
                };

                OptionalCostOption {
                    index,
                    label: opt_cost.label.clone(),
                    repeatable: opt_cost.repeatable,
                    affordable,
                    cost_description,
                }
            })
            .collect();

        // Set up pending cast for optional costs stage
        let mut pending = pending;
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
            .map(|o| o.name.clone())
            .unwrap_or_else(|| "spell".to_string());
        let ctx = crate::decisions::context::SelectOptionsContext::new(
            player,
            Some(source),
            format!("Choose optional costs for {}", spell_name),
            selectable_options,
            0, // min - optional costs are optional
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
        && let Some(mana_cost) = get_spell_mana_cost(
            game,
            pending.spell_id,
            pending.caster,
            &pending.casting_method,
            pending.from_zone,
        )
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
            .map(|o| o.name.clone())
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
        pending.stage = CastStage::ChoosingTargets;
        let requirements = pending.remaining_requirements.clone();
        let player = pending.caster;
        let source = pending.spell_id;
        let context = game
            .object(source)
            .map(|o| o.name.clone())
            .unwrap_or_else(|| "spell".to_string());

        state.pending_cast = Some(pending);

        // Convert to TargetsContext
        let ctx = crate::decisions::context::TargetsContext::new(
            player,
            source,
            context,
            requirements
                .into_iter()
                .map(|r| crate::decisions::context::TargetRequirementContext {
                    description: r.description,
                    legal_targets: r.legal_targets,
                    min_targets: r.min_targets,
                    max_targets: r.max_targets,
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
    let mana_spent_to_cast = pending.mana_spent_to_cast.clone();
    for _ in pending
        .hybrid_choices
        .iter()
        .filter(|(_, symbol)| matches!(symbol, crate::mana::ManaSymbol::Life(_)))
    {
        pending
            .optional_costs_paid
            .mark_label_paid("CompleatedLifePaid");
    }
    if pending.caster == game.turn.active_player
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
        pending.x_value,
        pending.casting_method,
        pending.optional_costs_paid,
        pending.chosen_modes,
        mana_spent_to_cast,
        pending.keyword_payment_contributions,
        pending.tagged_objects,
        pending.effect_outcomes,
        &mut pending.payment_trace,
        true,
        pending.stack_id,
        spell_cast_provenance,
        &mut *decision_maker,
    )?;

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
    auto_pay_spell_tap_cost_steps(game, trigger_queue, &mut pending, decision_maker)?;
    pending.stage = spell_stage_after_targets(&pending);
    let option_count =
        usize::from(pending.mana_cost_to_pay.is_some()) + pending.remaining_cost_steps.len();

    if option_count == 1 {
        if pending.mana_cost_to_pay.is_some() {
            pending.stage = CastStage::PayingMana;
            return continue_spell_cast_mana_payment(
                game,
                trigger_queue,
                state,
                pending,
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
                .map(|o| o.name.clone())
                .unwrap_or_else(|| "spell".to_string());
            let ctx = build_next_cost_context(
                pending.caster,
                pending.spell_id,
                source_name,
                pending.mana_cost_to_pay.as_ref(),
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
                        .map(|o| o.name.clone())
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
                        .map(|o| o.name.clone())
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

pub(super) fn continue_to_mana_payment(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    pending: PendingCast,
    targets: Vec<Target>,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    use crate::decision::calculate_effective_mana_cost_for_payment_with_chosen_targets_for_casting_method;

    let mut pending = pending;
    pending.chosen_targets = targets;

    // Compute the effective mana cost for this spell
    let effective_cost = if let Some(obj) = game.object(pending.spell_id) {
        let base_cost = crate::decision::spell_mana_cost_for_cast(
            game,
            pending.caster,
            obj,
            &pending.casting_method,
            pending.from_zone,
        );

        // Apply cost reductions (affinity, delve, convoke, improvise)
        base_cost.map(|bc| {
            let bc = mana_cost_with_paid_optional_costs(&bc, obj, &pending.optional_costs_paid);
            calculate_effective_mana_cost_for_payment_with_chosen_targets_for_casting_method(
                game,
                pending.caster,
                obj,
                &bc,
                &pending.chosen_targets,
                &pending.casting_method,
            )
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
        );
    }

    continue_spell_next_cost_or_finalize(game, trigger_queue, state, pending, decision_maker)
}

/// Continue processing spell cast mana payment pip-by-pip.
pub(super) fn continue_spell_cast_mana_payment(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    mut pending: PendingCast,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    let x_value = pending.x_value.unwrap_or(0);

    // Initialize remaining_mana_pips from mana_cost_to_pay if not already done
    // We use take() to clear mana_cost_to_pay so we don't re-populate on recursive calls
    if pending.remaining_mana_pips.is_empty()
        && let Some(cost) = pending.mana_cost_to_pay.take()
    {
        pending.display_mana_pips = expand_mana_cost_to_display_pips(&cost, x_value as usize);
        pending.remaining_mana_pips =
            expand_mana_cost_to_pips(&cost, x_value as usize, &pending.hybrid_choices);
    }

    // If no remaining pips, return to next-cost selection or finalize the spell.
    if pending.remaining_mana_pips.is_empty() {
        pending.current_pip_payment_options.clear();
        return continue_spell_next_cost_or_finalize(
            game,
            trigger_queue,
            state,
            pending,
            decision_maker,
        );
    }

    // Get the first pip to pay
    let pip = pending.remaining_mana_pips[0].clone();
    let remaining_count = pending.remaining_mana_pips.len();

    // Build payment options for this pip
    let player_id = pending.caster;
    let source = pending.spell_id;
    let context = game
        .object(source)
        .map(|o| o.name.clone())
        .unwrap_or_else(|| "spell".to_string());

    let mana_spend_policy = game.mana_spend_policy(player_id, Some(source));
    let allow_black_life = game.player_can_pay_black_with_life_for_reason(
        player_id,
        Some(source),
        crate::costs::PaymentReason::CastSpell,
    );
    let display_pip = current_display_pip(&pending.display_mana_pips, &pending.remaining_mana_pips);
    let options = build_pip_payment_options(
        game,
        player_id,
        &pip,
        display_pip,
        &mana_spend_policy,
        allow_black_life,
        Some(source),
        &mut *decision_maker,
    );

    // If no options available (shouldn't happen if we validated correctly), error
    if options.is_empty() {
        return Err(GameLoopError::InvalidState(
            "No payment options available for mana pip".to_string(),
        ));
    }

    // Auto-select deterministic pip choices when possible.
    if let Some(auto_choice) = preferred_auto_pip_choice(state, &options) {
        let action = options[auto_choice].action.clone();
        let pip_paid = execute_pip_payment_action(
            game,
            trigger_queue,
            player_id,
            Some(source),
            &pip,
            &mana_spend_policy,
            &action,
            &mut *decision_maker,
            &mut pending.payment_trace,
            Some(&mut pending.mana_spent_to_cast),
        )?;
        queue_mana_ability_event_for_action(
            game,
            trigger_queue,
            &mut *decision_maker,
            &action,
            player_id,
        );
        drain_pending_trigger_events(game, trigger_queue);
        if pip_paid {
            record_keyword_payment_contribution(
                &mut pending.keyword_payment_contributions,
                &action,
            );
            pending.remaining_mana_pips.remove(0);
        }
        return continue_spell_cast_mana_payment(
            game,
            trigger_queue,
            state,
            pending,
            decision_maker,
        );
    }

    let pip_description = format_pip(&pip);

    // Convert ManaPipPaymentOption to SelectableOption
    let selectable_options: Vec<crate::decisions::context::SelectableOption> = options
        .iter()
        .map(|opt| crate::decisions::context::SelectableOption::new(opt.index, &opt.description))
        .collect();

    pending.current_pip_payment_options = options;
    state.pending_cast = Some(pending);

    let ctx = crate::decisions::context::SelectOptionsContext::mana_pip_payment(
        player_id,
        source,
        context,
        pip_description,
        remaining_count,
        selectable_options,
    );
    Ok(GameProgress::NeedsDecisionCtx(
        crate::decisions::context::DecisionContext::SelectOptions(ctx),
    ))
}

/// Compute available mana payment options for a player during mana ability activation.
///
/// This returns options for:
/// - Available mana abilities that can be activated (excluding the one being paid for)
///   and that can help pay the remaining cost
/// - Option to pay (if enough mana is in pool)
pub(super) fn compute_mana_ability_payment_options(
    game: &GameState,
    player: PlayerId,
    pending: &PendingManaAbility,
    decision_maker: &mut impl DecisionMaker,
) -> Vec<ManaPaymentOption> {
    use crate::ability::AbilityKind;

    let mut options = Vec::new();

    // Get available mana abilities the player can activate
    // Exclude the mana ability we're trying to pay for
    let mana_abilities = get_available_mana_abilities(game, player, decision_maker);

    // Filter to only abilities that can help pay the cost
    let mut option_index = 0;
    for (perm_id, ability_index, description) in mana_abilities.iter() {
        // Skip mana abilities on the same source while paying this source's mana
        // activation cost. This avoids recursive "pay this ability with itself"
        // option loops (e.g., duplicated variable-output mana abilities).
        if *perm_id == pending.source {
            continue;
        }

        // Get the mana this ability produces and check if it can help pay the cost
        let mana_spend_policy = game.mana_spend_policy(player, Some(pending.source));
        let can_help = if game.object(*perm_id).is_some()
            && let Some(ability) = game.current_ability(*perm_id, *ability_index)
            && let AbilityKind::Activated(mana_ability) = &ability.kind
            && mana_ability.is_runtime_mana_ability(game, *perm_id, player)
        {
            let produced = mana_ability.inferred_mana_symbols(game, *perm_id, player);
            mana_can_help_pay_cost(
                &produced,
                &pending.mana_cost,
                game,
                player,
                &mana_spend_policy,
            )
        } else {
            // If we can't determine, include it
            true
        };

        if can_help {
            options.push(ManaPaymentOption {
                index: option_index,
                description: format!(
                    "Tap {}: {}",
                    describe_permanent(game, *perm_id),
                    description
                ),
            });
            option_index += 1;
        }
    }

    // Add option to pay if player has enough mana
    if game.can_pay_mana_cost_with_reason(
        player,
        Some(pending.source),
        &pending.mana_cost,
        0,
        crate::costs::PaymentReason::ActivateManaAbility,
    ) {
        options.push(ManaPaymentOption {
            index: options.len(),
            description: "Pay mana cost".to_string(),
        });
    }

    options
}

/// Check if mana produced by an ability can help pay a mana cost.
///
/// Returns true if any of the mana symbols can pay any pip in the cost,
/// considering the player's current mana pool.
pub(super) fn mana_can_help_pay_cost(
    mana_produced: &[crate::mana::ManaSymbol],
    cost: &crate::mana::ManaCost,
    game: &GameState,
    player: PlayerId,
    mana_spend_policy: &crate::player::ManaSpendPolicy,
) -> bool {
    use crate::mana::ManaSymbol;

    // Get current mana pool to see what's already available
    let pool = game.player(player).map(|p| &p.mana_pool);

    // Check each pip in the cost to see if the produced mana can help
    for pip in cost.pips() {
        for alternative in pip {
            match alternative {
                // Generic mana can be paid by any colored mana
                ManaSymbol::Generic(_) => {
                    // Any mana helps with generic costs
                    if !mana_produced.is_empty() {
                        return true;
                    }
                }
                // Colored mana must match
                ManaSymbol::White
                | ManaSymbol::Blue
                | ManaSymbol::Black
                | ManaSymbol::Red
                | ManaSymbol::Green => {
                    if mana_produced
                        .iter()
                        .any(|symbol| mana_spend_policy.can_pay_symbol(*symbol, *alternative))
                    {
                        return true;
                    }
                }
                // Colorless mana can only be paid by colorless
                ManaSymbol::Colorless => {
                    if mana_produced.iter().any(|symbol| {
                        mana_spend_policy.can_pay_symbol(*symbol, ManaSymbol::Colorless)
                    }) {
                        return true;
                    }
                }
                // Snow, life, X - less common, be permissive
                _ => return true,
            }
        }
    }

    // Also check if this mana could help after we pay some colored pips
    // (e.g., we might need {W}{W} and only have one white, so any mana helps with the first)
    // For simplicity, if the cost has any generic component that's not yet payable, any mana helps
    if pool.is_some() {
        let generic_needed = cost
            .pips()
            .iter()
            .filter(|pip| pip.iter().any(|s| matches!(s, ManaSymbol::Generic(_))))
            .count();

        // Very rough heuristic: if there are generic costs and the ability produces any mana
        if generic_needed > 0 && !mana_produced.is_empty() {
            return true;
        }
    }

    false
}

/// Get available mana abilities for a player that can be activated.
///
/// Returns a list of (permanent_id, ability_index, description) tuples.
pub(super) fn get_available_mana_abilities(
    game: &GameState,
    player: PlayerId,
    decision_maker: &mut impl DecisionMaker,
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
                let desc = describe_mana_ability(game, perm_id, player, &ability.kind);
                abilities.push((perm_id, ability_index, desc));
            }
        }
    }
    let _ = decision_maker;

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
        .map(|obj| obj.name.clone())
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
) -> Vec<ObjectId> {
    let ctx = game.filter_context_for(player, Some(source));

    let ids: Vec<ObjectId> = match zone {
        Zone::Battlefield => game.battlefield.iter().copied().collect(),
        Zone::Hand => game
            .player(player)
            .map(|p| p.hand.iter().copied().collect())
            .unwrap_or_default(),
        Zone::Graveyard => game
            .player(player)
            .map(|p| p.graveyard.iter().copied().collect())
            .unwrap_or_default(),
        Zone::Exile => game.exile.iter().copied().collect(),
        _ => Vec::new(),
    };

    ids.into_iter()
        .filter(|&id| {
            game.object(id).is_some_and(|obj| {
                if filter.other && obj.id == source {
                    return false;
                }
                filter.matches(obj, &ctx, game)
            })
        })
        .collect()
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
            description,
            ..
        } => (
            format!("Choose an object to exile: {}", description),
            get_legal_cost_choice_objects(game, player, source, filter, *zone),
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
            get_legal_cost_choice_objects(game, player, source, filter, *source_zone),
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

pub(super) fn collect_spell_cost_steps(
    game: &GameState,
    spell_id: ObjectId,
    caster: PlayerId,
    casting_method: &CastingMethod,
    optional_costs_paid: &OptionalCostsPaid,
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
            .or_else(|| obj.cast_alternative_method.clone())
            .and_then(|method| method.total_cost().cloned())
            .unwrap_or_else(crate::cost::TotalCost::free),
        };

        extend_non_mana(&mut cost_steps, &alternative_additional_cost);
        extend_non_mana(&mut cost_steps, &obj.additional_cost);
        for (idx, optional_cost) in obj.optional_costs.iter().enumerate() {
            let times = optional_costs_paid.times_paid(idx);
            for _ in 0..times {
                extend_non_mana(&mut cost_steps, &optional_cost.cost);
            }
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

pub(super) fn spell_stage_after_targets(pending: &PendingCast) -> CastStage {
    if !pending.remaining_cost_steps.is_empty()
        || pending.mana_cost_to_pay.is_some()
        || !pending.remaining_mana_pips.is_empty()
    {
        CastStage::ChoosingNextCost
    } else {
        CastStage::ReadyToFinalize
    }
}

pub(super) fn activation_stage_after_targets(pending: &PendingActivation) -> ActivationStage {
    if !pending.remaining_cost_steps.is_empty()
        || pending.mana_cost_to_pay.is_some()
        || !pending.remaining_mana_pips.is_empty()
    {
        ActivationStage::ChoosingNextCost
    } else {
        ActivationStage::ReadyToFinalize
    }
}

pub(super) fn build_next_cost_context(
    player: PlayerId,
    source: ObjectId,
    source_name: String,
    mana_cost: Option<&crate::mana::ManaCost>,
    remaining_cost_steps: &[ActivationCostStep],
) -> crate::decisions::context::SelectOptionsContext {
    let mut options = Vec::new();
    let mut next_index = 0usize;

    if let Some(cost) = mana_cost {
        options.push(crate::decisions::context::SelectableOption::new(
            next_index,
            format!("Mana: {}", format_mana_cost_simple(cost)),
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
                if ctx.target == object_id {
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
                crate::costs::PaymentReason::ActivateAbility,
            );

            if legal_targets.is_empty() {
                return Err(GameLoopError::InvalidState(
                    "No valid sacrifice targets".to_string(),
                ));
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
                        .map(|o| o.name.clone())
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

            let player = pending.activator;
            let source = pending.source;
            pending.stage = ActivationStage::ChoosingCardCost;
            state.pending_activation = Some(pending);

            let candidates: Vec<crate::decisions::context::SelectableObject> = legal_cards
                .iter()
                .map(|&id| {
                    let name = game
                        .object(id)
                        .map(|o| o.name.clone())
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

    match pending.stage {
        ActivationStage::ChoosingModes => {
            check_activation_modes_or_continue(game, trigger_queue, state, pending, decision_maker)
        }
        ActivationStage::ChoosingX => {
            // Need to choose X value first
            let mut max_x = if let Some(ref cost) = pending.mana_cost_to_pay {
                let mana_spend_policy =
                    game.mana_spend_policy(pending.activator, Some(pending.source));
                let allow_black_life = game.player_can_pay_black_with_life_for_reason(
                    pending.activator,
                    Some(pending.source),
                    crate::costs::PaymentReason::ActivateAbility,
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

            state.pending_activation = Some(pending.clone());

            let ctx = crate::decisions::context::NumberContext::x_value(
                pending.activator,
                pending.source,
                max_x,
            );
            Ok(GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::Number(ctx),
            ))
        }
        ActivationStage::ProcessingCosts => {
            continue_activation_cost_payment(game, trigger_queue, state, pending, decision_maker)
        }
        ActivationStage::ChoosingNextCost => {
            auto_pay_activation_tap_cost_steps(game, trigger_queue, &mut pending, decision_maker)?;
            let option_count = usize::from(pending.mana_cost_to_pay.is_some())
                + pending.remaining_cost_steps.len();
            if option_count == 0 {
                pending.stage = ActivationStage::ReadyToFinalize;
                return continue_activation(game, trigger_queue, state, pending, decision_maker);
            }
            if option_count == 1 {
                if pending.mana_cost_to_pay.is_some() {
                    pending.stage = ActivationStage::PayingMana;
                } else {
                    pending.stage = ActivationStage::ProcessingCosts;
                }
                return continue_activation(game, trigger_queue, state, pending, decision_maker);
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
                &pending.remaining_cost_steps,
            );
            state.pending_activation = Some(pending);
            Ok(GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::SelectOptions(ctx),
            ))
        }
        ActivationStage::ChoosingSacrifice | ActivationStage::ChoosingCardCost => {
            state.pending_activation = Some(pending);
            Err(GameLoopError::InvalidState(
                "Activation object-cost stage requires a SelectObjects response".to_string(),
            ))
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
                return continue_activation(game, trigger_queue, state, pending, decision_maker);
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
            Ok(GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::HybridChoice(ctx),
            ))
        }
        ActivationStage::ChoosingTargets => {
            if pending.remaining_requirements.is_empty() {
                pending.stage = activation_stage_after_targets(&pending);
                continue_activation(game, trigger_queue, state, pending, decision_maker)
            } else {
                let requirements = pending.remaining_requirements.clone();
                let player = pending.activator;
                let source = pending.source;
                let context = game
                    .object(source)
                    .map(|o| format!("{}'s ability", o.name))
                    .unwrap_or_else(|| "ability".to_string());

                state.pending_activation = Some(pending);

                // Convert to TargetsContext
                let ctx = crate::decisions::context::TargetsContext::new(
                    player,
                    source,
                    context,
                    requirements
                        .into_iter()
                        .map(|r| crate::decisions::context::TargetRequirementContext {
                            description: r.description,
                            legal_targets: r.legal_targets,
                            min_targets: r.min_targets,
                            max_targets: r.max_targets,
                        })
                        .collect(),
                );
                Ok(GameProgress::NeedsDecisionCtx(
                    crate::decisions::context::DecisionContext::Targets(ctx),
                ))
            }
        }
        ActivationStage::PayingMana => {
            let x_value = pending.x_value.unwrap_or(0);

            // Initialize remaining_mana_pips from mana_cost_to_pay if not already done
            // We use take() to clear mana_cost_to_pay so we don't re-populate on recursive calls
            if pending.remaining_mana_pips.is_empty()
                && let Some(cost) = pending.mana_cost_to_pay.take()
            {
                pending.display_mana_pips = expand_mana_cost_to_display_pips(&cost, x_value);
                pending.remaining_mana_pips =
                    expand_mana_cost_to_pips(&cost, x_value, &pending.hybrid_choices);
            }

            // If no remaining pips, return to next-cost selection or finalize.
            if pending.remaining_mana_pips.is_empty() {
                pending.stage = activation_stage_after_targets(&pending);
                return continue_activation(game, trigger_queue, state, pending, decision_maker);
            }

            // Get the first pip to pay
            let pip = pending.remaining_mana_pips[0].clone();
            let remaining_count = pending.remaining_mana_pips.len();

            // Build payment options for this pip
            let player_id = pending.activator;
            let source = pending.source;
            let context = game
                .object(source)
                .map(|o| format!("{}'s ability", o.name))
                .unwrap_or_else(|| "ability".to_string());

            let mana_spend_policy = game.mana_spend_policy(player_id, Some(source));
            let allow_black_life = game.player_can_pay_black_with_life_for_reason(
                player_id,
                Some(source),
                crate::costs::PaymentReason::ActivateAbility,
            );
            let display_pip =
                current_display_pip(&pending.display_mana_pips, &pending.remaining_mana_pips);
            let options = build_pip_payment_options(
                game,
                player_id,
                &pip,
                display_pip,
                &mana_spend_policy,
                allow_black_life,
                Some(source),
                &mut *decision_maker,
            );

            // If no options available (shouldn't happen if we validated correctly), error
            if options.is_empty() {
                return Err(GameLoopError::InvalidState(
                    "No payment options available for mana pip".to_string(),
                ));
            }

            // Auto-select deterministic pip choices when possible.
            if let Some(auto_choice) = preferred_auto_pip_choice(state, &options) {
                let action = options[auto_choice].action.clone();
                let pip_paid = execute_pip_payment_action(
                    game,
                    trigger_queue,
                    player_id,
                    Some(source),
                    &pip,
                    &mana_spend_policy,
                    &action,
                    &mut *decision_maker,
                    &mut pending.payment_trace,
                    None,
                )?;
                queue_mana_ability_event_for_action(
                    game,
                    trigger_queue,
                    &mut *decision_maker,
                    &action,
                    player_id,
                );
                drain_pending_trigger_events(game, trigger_queue);
                if pip_paid {
                    pending.remaining_mana_pips.remove(0);
                }
                return continue_activation(game, trigger_queue, state, pending, decision_maker);
            }

            let pip_description = format_pip(&pip);

            state.pending_activation = Some(pending);

            // Convert ManaPipPaymentOption to SelectableOption
            let selectable_options: Vec<crate::decisions::context::SelectableOption> = options
                .iter()
                .map(|opt| {
                    crate::decisions::context::SelectableOption::new(opt.index, &opt.description)
                })
                .collect();

            let ctx = crate::decisions::context::SelectOptionsContext::mana_pip_payment(
                player_id,
                source,
                context,
                pip_description,
                remaining_count,
                selectable_options,
            );
            Ok(GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::SelectOptions(ctx),
            ))
        }
        ActivationStage::ReadyToFinalize => {
            // Record activation for per-turn-limited abilities
            if pending.is_once_per_turn {
                game.record_ability_activation(pending.source, pending.ability_index);
            }

            // Create ability stack entry with targets
            let mut entry =
                StackEntry::ability(pending.source, pending.activator, pending.effects.clone())
                    .with_ability_index(pending.ability_index)
                    .with_provenance(pending.provenance)
                    .with_source_info(pending.source_stable_id, pending.source_name.clone())
                    .with_source_snapshot(pending.source_snapshot.clone())
                    .with_chosen_modes(pending.chosen_modes.clone())
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
            );

            // Clear pending state and checkpoint - action completed successfully
            state.pending_activation = None;
            state.clear_checkpoint();
            priority_after_player_action(game, &mut state.tracker, pending.activator);
            advance_priority_with_dm(game, trigger_queue, decision_maker)
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
