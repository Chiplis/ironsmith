use super::*;
use crate::ability::ActivatedAbilityRuntimeExt as _;
use crate::filter::{
    FilterContext, ObjectFilterExt as _, PlayerFilterExt as _, TaggedConstraintSubject as _,
};
use crate::types::CardType;

mod mechanics;

pub use mechanics::*;

pub(crate) fn mana_cost_has_black_symbol(cost: &crate::mana::ManaCost) -> bool {
    cost.pips()
        .iter()
        .any(|pip| pip.contains(&crate::mana::ManaSymbol::Black))
}

fn with_source_exiled_tagged_objects(
    game: &GameState,
    mut ctx: crate::filter::FilterContext,
    source: ObjectId,
) -> crate::filter::FilterContext {
    let source_exiled = game
        .get_exiled_with_source_links(source)
        .iter()
        .filter_map(|id| {
            game.object(*id)
                .map(|obj| crate::snapshot::ObjectSnapshot::from_object(obj, game))
        })
        .collect::<Vec<_>>();
    if !source_exiled.is_empty() {
        ctx.tagged_objects
            .insert(crate::tag::SOURCE_EXILED_TAG.into(), source_exiled);
    }
    ctx
}

fn count_basic_land_types_among_filter(
    game: &GameState,
    filter: &crate::target::ObjectFilter,
    filter_ctx: &crate::filter::FilterContext,
) -> u32 {
    let mut seen = std::collections::HashSet::new();
    for obj in game.objects_in_deterministic_order() {
        if !filter.matches(obj, filter_ctx, game) {
            continue;
        }
        for subtype in game.calculated_subtypes(obj.id) {
            if subtype.is_basic_land_type() {
                seen.insert(subtype);
            }
        }
    }
    seen.len() as u32
}

fn shared_spell_characteristic_count(
    game: &GameState,
    spell: &crate::object::Object,
    source: ObjectId,
    controller: PlayerId,
    intersection: &ironsmith_core::CostReductionCharacteristicIntersection,
) -> i32 {
    let filter_ctx = with_source_exiled_tagged_objects(
        game,
        FilterContext::new(controller)
            .with_source(source)
            .with_active_player(game.turn.active_player)
            .with_opponents(
                game.turn_store
                    .turn_order
                    .iter()
                    .copied()
                    .filter(|player| *player != controller)
                    .collect(),
            ),
        source,
    );
    let comparison_objects = game
        .objects_in_deterministic_order()
        .into_iter()
        .filter(|object| intersection.comparison.matches(object, &filter_ctx, game))
        .collect::<Vec<_>>();

    match intersection.characteristic {
        crate::ObjectCharacteristic::CardType => {
            let comparison = comparison_objects
                .iter()
                .flat_map(|object| {
                    game.current_card_types(object.id)
                        .unwrap_or_else(|| object.card_types.to_vec())
                })
                .collect::<std::collections::HashSet<_>>();
            game.current_card_types(spell.id)
                .unwrap_or_else(|| spell.card_types.to_vec())
                .into_iter()
                .filter(|card_type| comparison.contains(card_type))
                .count() as i32
        }
        crate::ObjectCharacteristic::PermanentType => {
            let is_permanent_type = |card_type: &CardType| {
                matches!(
                    card_type,
                    CardType::Land
                        | CardType::Creature
                        | CardType::Artifact
                        | CardType::Enchantment
                        | CardType::Planeswalker
                        | CardType::Battle
                )
            };
            let comparison = comparison_objects
                .iter()
                .flat_map(|object| {
                    game.current_card_types(object.id)
                        .unwrap_or_else(|| object.card_types.to_vec())
                })
                .filter(is_permanent_type)
                .collect::<std::collections::HashSet<_>>();
            game.current_card_types(spell.id)
                .unwrap_or_else(|| spell.card_types.to_vec())
                .into_iter()
                .filter(is_permanent_type)
                .filter(|card_type| comparison.contains(card_type))
                .count() as i32
        }
        crate::ObjectCharacteristic::Subtype(family) => {
            let comparison = comparison_objects
                .iter()
                .flat_map(|object| game.calculated_subtypes(object.id))
                .filter(|subtype| subtype.belongs_to_family(family))
                .collect::<std::collections::HashSet<_>>();
            game.calculated_subtypes(spell.id)
                .into_iter()
                .filter(|subtype| subtype.belongs_to_family(family))
                .filter(|subtype| comparison.contains(subtype))
                .count() as i32
        }
        crate::ObjectCharacteristic::Color => {
            let comparison = comparison_objects.iter().fold(
                crate::color::ColorSet::COLORLESS,
                |colors, object| {
                    colors.union(
                        game.current_colors(object.id)
                            .unwrap_or_else(|| object.colors()),
                    )
                },
            );
            game.current_colors(spell.id)
                .unwrap_or_else(|| spell.colors())
                .intersection(comparison)
                .count() as i32
        }
        crate::ObjectCharacteristic::ManaValue => comparison_objects
            .iter()
            .any(|object| object.subject_mana_value() == spell.subject_mana_value())
            as i32,
    }
}

fn resolve_cost_reduction_amount(
    game: &GameState,
    spell: &crate::object::Object,
    source: ObjectId,
    controller: PlayerId,
    reduction: &crate::static_abilities::CostReduction,
) -> i32 {
    let per_characteristic =
        resolve_cost_modifier_value_for_source(game, source, controller, &reduction.reduction);
    let Some(intersection) = &reduction.characteristic_intersection else {
        return per_characteristic;
    };
    per_characteristic.saturating_mul(shared_spell_characteristic_count(
        game,
        spell,
        source,
        controller,
        intersection,
    ))
}

fn alternative_method_is_emerge(
    method: &crate::alternative_cast::AlternativeCastingMethod,
) -> bool {
    method.name().eq_ignore_ascii_case("Emerge")
}

fn emerge_sacrifice_filter(
    method: &crate::alternative_cast::AlternativeCastingMethod,
) -> Option<crate::target::ObjectFilter> {
    method
        .non_mana_costs()
        .into_iter()
        .find_map(|cost| cost.sacrifice_filter().cloned())
}

fn maximum_emerge_reduction(
    game: &GameState,
    player: PlayerId,
    source: crate::ids::ObjectId,
    method: &crate::alternative_cast::AlternativeCastingMethod,
) -> u32 {
    if !alternative_method_is_emerge(method) {
        return 0;
    }
    let Some(filter) = emerge_sacrifice_filter(method) else {
        return 0;
    };

    let ctx = game.filter_context_for(player, Some(source));
    let lands_only = game.player_cant_sacrifice_nonland_to_cast_or_activate(player);
    game.battlefield
        .iter()
        .copied()
        .filter_map(|id| {
            let obj = game.object(id)?;
            if game.controller_of(obj) != player
                || !filter.matches(obj, &ctx, game)
                || !game.can_be_sacrificed(id)
                || (lands_only && !game.object_has_card_type(id, crate::types::CardType::Land))
            {
                return None;
            }
            Some(obj.mana_cost.as_ref().map_or(0, |cost| cost.mana_value()))
        })
        .max()
        .unwrap_or(0)
}

pub(crate) fn apply_emerge_reduction_to_alternative_mana_cost(
    game: &GameState,
    player: PlayerId,
    source: crate::ids::ObjectId,
    method: &crate::alternative_cast::AlternativeCastingMethod,
    base_cost: &crate::mana::ManaCost,
) -> crate::mana::ManaCost {
    let reduction = maximum_emerge_reduction(game, player, source, method);
    if reduction == 0 {
        base_cost.clone()
    } else {
        base_cost.reduce_generic(reduction)
    }
}

/// Calculate activated-ability cost after applying battlefield static cost modifiers.
pub fn calculate_effective_activation_total_cost(
    game: &GameState,
    activator: PlayerId,
    ability_source: ObjectId,
    cost: &crate::cost::TotalCost,
) -> crate::cost::TotalCost {
    calculate_effective_activation_total_cost_with_chosen_targets(
        game,
        activator,
        ability_source,
        cost,
        &[],
    )
}

pub fn calculate_effective_activation_total_cost_with_chosen_targets(
    game: &GameState,
    activator: PlayerId,
    ability_source: ObjectId,
    cost: &crate::cost::TotalCost,
    chosen_targets: &[Target],
) -> crate::cost::TotalCost {
    let view = DerivedGameView::new(game);
    calculate_effective_activation_total_cost_with_view(
        game,
        activator,
        ability_source,
        cost,
        chosen_targets,
        &view,
    )
}

pub(crate) fn calculate_effective_activation_total_cost_with_view(
    game: &GameState,
    activator: PlayerId,
    ability_source: ObjectId,
    cost: &crate::cost::TotalCost,
    chosen_targets: &[Target],
    view: &DerivedGameView<'_>,
) -> crate::cost::TotalCost {
    use crate::ability::AbilityKind;
    use crate::filter::{FilterContext, player_filter_matches_game};

    if let ironsmith_core::TotalCostKind::OneOf(branches) = cost.kind() {
        return crate::cost::TotalCost::one_of(
            branches
                .iter()
                .map(|branch| {
                    calculate_effective_activation_total_cost_with_view(
                        game,
                        activator,
                        ability_source,
                        branch,
                        chosen_targets,
                        view,
                    )
                })
                .collect(),
        );
    }

    fn opponents_of(game: &GameState, player: PlayerId) -> Vec<PlayerId> {
        game.turn_store
            .turn_order
            .iter()
            .copied()
            .filter(|p| *p != player)
            .collect()
    }

    let mut costs = Vec::with_capacity(cost.costs().len());
    for component in cost.costs() {
        if let Some(mana_cost) = component.mana_cost_ref() {
            let reduced = calculate_effective_activation_mana_cost_with_view(
                game,
                activator,
                ability_source,
                mana_cost,
                chosen_targets,
                view,
            );
            costs.push(crate::costs::Cost::mana(reduced));
        } else {
            costs.push(component.clone());
        }
    }

    let mut adjusted = crate::cost::TotalCost::from_costs(costs);
    let Some(ability_source_object) = game.object(ability_source) else {
        return adjusted;
    };

    let mut cost_modifier_sources = view.activated_ability_cost_modifier_sources();
    if ability_source_object.zone != Zone::Battlefield {
        cost_modifier_sources.push(ability_source);
    }

    for source_id in cost_modifier_sources {
        let Some(perm) = game.object(source_id) else {
            continue;
        };
        let controller = game.controller_of(perm);
        let filter_ctx = FilterContext::new(controller)
            .with_source(source_id)
            .with_active_player(game.turn.active_player)
            .with_opponents(opponents_of(game, controller));

        let static_abilities = if perm.zone == Zone::Battlefield {
            view.static_abilities_rc(source_id).unwrap_or_else(|| {
                Rc::new(
                    perm.abilities
                        .iter()
                        .filter_map(|a| match &a.kind {
                            AbilityKind::Static(sa) => Some(sa.clone()),
                            _ => None,
                        })
                        .collect(),
                )
            })
        } else {
            Rc::new(
                perm.abilities
                    .iter()
                    .filter_map(|a| match &a.kind {
                        AbilityKind::Static(sa) if a.functions_in(&perm.zone) => Some(sa.clone()),
                        _ => None,
                    })
                    .collect(),
            )
        };

        for static_ability in static_abilities.iter() {
            if !static_ability.is_active(game, source_id) {
                continue;
            }

            if let Some(increase) = static_ability.activated_ability_cost_increase() {
                if let Some(activator_filter) = &increase.activator
                    && !player_filter_matches_game(activator_filter, activator, game, &filter_ctx)
                {
                    continue;
                }
                if !increase
                    .filter
                    .matches(ability_source_object, &filter_ctx, game)
                {
                    continue;
                }

                let mut costs = adjusted.costs().to_vec();
                costs.extend(increase.increase.costs().iter().cloned());
                adjusted = crate::cost::TotalCost::from_costs(costs);
            }
        }
    }

    adjusted
}

/// Calculate the effective mana portion of an activated ability's cost.
pub fn calculate_effective_activation_mana_cost(
    game: &GameState,
    activator: PlayerId,
    ability_source: ObjectId,
    base_cost: &crate::mana::ManaCost,
) -> crate::mana::ManaCost {
    let view = DerivedGameView::new(game);
    calculate_effective_activation_mana_cost_with_view(
        game,
        activator,
        ability_source,
        base_cost,
        &[],
        &view,
    )
}

pub(crate) fn calculate_effective_activation_mana_cost_with_view(
    game: &GameState,
    activator: PlayerId,
    ability_source: ObjectId,
    base_cost: &crate::mana::ManaCost,
    chosen_targets: &[Target],
    view: &DerivedGameView<'_>,
) -> crate::mana::ManaCost {
    use crate::ability::AbilityKind;
    use crate::filter::FilterContext;

    fn opponents_of(game: &GameState, player: PlayerId) -> Vec<PlayerId> {
        game.turn_store
            .turn_order
            .iter()
            .copied()
            .filter(|p| *p != player)
            .collect()
    }

    let mut adjusted = base_cost.clone();
    let Some(ability_source_object) = game.object(ability_source) else {
        return adjusted;
    };

    let mut cost_modifier_sources = view.activated_ability_cost_modifier_sources();
    if ability_source_object.zone != Zone::Battlefield {
        cost_modifier_sources.push(ability_source);
    }

    for source_id in cost_modifier_sources {
        let Some(perm) = game.object(source_id) else {
            continue;
        };
        let controller = game.controller_of(perm);
        let filter_ctx = FilterContext::new(controller)
            .with_source(source_id)
            .with_active_player(game.turn.active_player)
            .with_opponents(opponents_of(game, controller));

        let static_abilities = if perm.zone == Zone::Battlefield {
            view.static_abilities_rc(source_id).unwrap_or_else(|| {
                Rc::new(
                    perm.abilities
                        .iter()
                        .filter_map(|a| match &a.kind {
                            AbilityKind::Static(sa) => Some(sa.clone()),
                            _ => None,
                        })
                        .collect(),
                )
            })
        } else {
            Rc::new(
                perm.abilities
                    .iter()
                    .filter_map(|a| match &a.kind {
                        AbilityKind::Static(sa) if a.functions_in(&perm.zone) => Some(sa.clone()),
                        _ => None,
                    })
                    .collect(),
            )
        };

        for static_ability in static_abilities.iter() {
            if !static_ability.is_active(game, source_id) {
                continue;
            }

            if let Some(reduction) = static_ability.activated_ability_cost_reduction() {
                if !reduction
                    .filter
                    .matches(ability_source_object, &filter_ctx, game)
                {
                    continue;
                }
                if let Some(condition) = &reduction.condition
                    && !crate::static_abilities::activated_ability_cost_condition_is_active_for_activation(
                        game,
                        ability_source,
                        condition,
                        chosen_targets,
                    )
                {
                    continue;
                }

                let multiplier = if let Some(per_filter) = &reduction.per_matching_objects {
                    game.objects_in_deterministic_order()
                        .into_iter()
                        .filter(|obj| per_filter.matches(obj, &filter_ctx, game))
                        .count() as u32
                } else if let Some(lands_filter) = &reduction.per_basic_land_types_among {
                    count_basic_land_types_among_filter(game, lands_filter, &filter_ctx)
                } else {
                    1
                };
                if multiplier == 0 {
                    continue;
                }

                if let Some(replacement) = &reduction.replacement_mana_cost {
                    adjusted = replacement.clone();
                    continue;
                }

                let before = adjusted.clone();
                adjusted = adjusted.reduce_generic(reduction.reduction.saturating_mul(multiplier));
                if let Some(minimum_total_mana) = reduction.minimum_total_mana
                    && before.mana_value() > 0
                    && adjusted.mana_value() < minimum_total_mana
                {
                    let missing = minimum_total_mana - adjusted.mana_value();
                    adjusted = add_generic_mana_cost(&adjusted, missing);
                }
            }
        }
    }

    apply_payment_reason_mana_adjustments(
        game,
        activator,
        Some(ability_source),
        &adjusted,
        crate::costs::PaymentReason::ActivateAbility,
    )
}

/// Resolve an alternative method index for `CastingMethod::PlayFrom`.
///
/// The index space is:
/// 1) Card intrinsic alternatives (`card.alternative_casts`)
/// 2) Granted alternatives for this card/zone/player (appended after intrinsic methods)
pub fn resolve_play_from_alternative_method(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    zone: Zone,
    idx: usize,
) -> Option<crate::alternative_cast::AlternativeCastingMethod> {
    if let Some(method) = spell.alternative_casts.get(idx) {
        return Some(method.clone());
    }

    let granted = game
        .effect_store
        .grant_registry
        .granted_alternative_casts_for_card(game, spell.id, zone, player);
    let granted_idx = idx.checked_sub(spell.alternative_casts.len())?;
    if let Some(entry) = granted.get(granted_idx) {
        return Some(entry.method.clone());
    }

    let adventure_idx = granted_idx.checked_sub(granted.len())?;
    let adventure_view = spell_view_for_split_other_half_cast(game, spell)?;
    let view = DerivedGameView::new(game);
    let adventure_granted =
        view.granted_alternative_casts_for_card_view(spell.id, &adventure_view, zone, player);
    adventure_granted
        .get(adventure_idx)
        .map(|entry| entry.method.clone())
}

pub(crate) fn alternative_cast_method_matches_kind(
    method: &crate::alternative_cast::AlternativeCastingMethod,
    kind: crate::filter::AlternativeCastKind,
) -> bool {
    use crate::alternative_cast::AlternativeCastingMethod;
    use crate::filter::AlternativeCastKind;

    match (kind, method) {
        (AlternativeCastKind::Blitz, AlternativeCastingMethod::Blitz { .. }) => true,
        (AlternativeCastKind::Dash, AlternativeCastingMethod::Dash { .. }) => true,
        (AlternativeCastKind::Flashback, AlternativeCastingMethod::Flashback { .. }) => true,
        (AlternativeCastKind::JumpStart, AlternativeCastingMethod::JumpStart { .. }) => true,
        (AlternativeCastKind::Escape, AlternativeCastingMethod::Escape { .. }) => true,
        (AlternativeCastKind::Madness, AlternativeCastingMethod::Madness { .. }) => true,
        (AlternativeCastKind::Miracle, AlternativeCastingMethod::Miracle { .. }) => true,
        (AlternativeCastKind::Suspend, AlternativeCastingMethod::Suspend { .. }) => true,
        _ => false,
    }
}

pub(crate) fn casting_method_matches_alternative_kind(
    game: &GameState,
    caster: PlayerId,
    spell: &crate::object::Object,
    casting_method: &CastingMethod,
    kind: crate::filter::AlternativeCastKind,
) -> bool {
    match casting_method {
        CastingMethod::Alternative(idx) => spell
            .alternative_casts
            .get(*idx)
            .is_some_and(|method| alternative_cast_method_matches_kind(method, kind)),
        CastingMethod::GrantedEscape { .. } => kind == crate::filter::AlternativeCastKind::Escape,
        CastingMethod::GrantedFlashback => kind == crate::filter::AlternativeCastKind::Flashback,
        CastingMethod::PlayFrom {
            use_alternative: Some(idx),
            zone,
            ..
        }
        | CastingMethod::SplitOtherHalfPlayFrom {
            use_alternative: idx,
            zone,
            ..
        } => resolve_play_from_alternative_method(game, caster, spell, *zone, *idx)
            .or_else(|| spell.cast_alternative_method_owned())
            .as_ref()
            .is_some_and(|method| alternative_cast_method_matches_kind(method, kind)),
        CastingMethod::Normal => spell
            .cast_alternative_method
            .as_ref()
            .is_some_and(|method| alternative_cast_method_matches_kind(method, kind)),
        CastingMethod::FaceDown
        | CastingMethod::SplitOtherHalf
        | CastingMethod::Fuse
        | CastingMethod::PlayFrom {
            use_alternative: None,
            ..
        } => false,
    }
}

fn casting_method_is_bestow(
    game: &GameState,
    caster: PlayerId,
    spell: &crate::object::Object,
    casting_method: &CastingMethod,
) -> bool {
    match casting_method {
        CastingMethod::Alternative(idx) => spell
            .alternative_casts
            .get(*idx)
            .is_some_and(|method| method.is_bestow()),
        CastingMethod::PlayFrom {
            use_alternative: Some(idx),
            zone,
            ..
        }
        | CastingMethod::SplitOtherHalfPlayFrom {
            use_alternative: idx,
            zone,
            ..
        } => resolve_play_from_alternative_method(game, caster, spell, *zone, *idx)
            .or_else(|| spell.cast_alternative_method_owned())
            .as_ref()
            .is_some_and(|method| method.is_bestow()),
        _ => false,
    }
}

fn casting_method_is_mutate(
    game: &GameState,
    caster: PlayerId,
    spell: &crate::object::Object,
    casting_method: &CastingMethod,
) -> bool {
    match casting_method {
        CastingMethod::Alternative(idx) => spell
            .alternative_casts
            .get(*idx)
            .is_some_and(|method| method.is_mutate()),
        CastingMethod::PlayFrom {
            use_alternative: Some(idx),
            zone,
            ..
        }
        | CastingMethod::SplitOtherHalfPlayFrom {
            use_alternative: idx,
            zone,
            ..
        } => resolve_play_from_alternative_method(game, caster, spell, *zone, *idx)
            .or_else(|| spell.cast_alternative_method_owned())
            .as_ref()
            .is_some_and(|method| method.is_mutate()),
        _ => false,
    }
}

fn inferred_cast_origin_zone_for_cost_filter(
    _game: &GameState,
    _caster: PlayerId,
    spell: &crate::object::Object,
    casting_method: &CastingMethod,
) -> Zone {
    match casting_method {
        CastingMethod::Normal => {
            if spell.zone == Zone::Stack {
                Zone::Hand
            } else {
                spell.zone
            }
        }
        CastingMethod::FaceDown | CastingMethod::SplitOtherHalf | CastingMethod::Fuse => Zone::Hand,
        CastingMethod::Alternative(idx) => spell
            .alternative_casts
            .get(*idx)
            .cloned()
            .or_else(|| spell.cast_alternative_method_owned())
            .map(|method| method.cast_from_zone())
            .unwrap_or_else(|| {
                if spell.zone == Zone::Stack {
                    Zone::Hand
                } else {
                    spell.zone
                }
            }),
        CastingMethod::GrantedEscape { .. } | CastingMethod::GrantedFlashback => Zone::Graveyard,
        CastingMethod::PlayFrom { zone, .. }
        | CastingMethod::SplitOtherHalfPlayFrom { zone, .. } => *zone,
    }
}

fn spell_view_for_cost_filter_match(
    game: &GameState,
    caster: PlayerId,
    spell: &crate::object::Object,
    casting_method: &CastingMethod,
    cast_from_zone: Option<Zone>,
) -> Option<crate::object::Object> {
    let mut view = spell.clone();
    let mut changed = false;

    if casting_method_is_bestow(game, caster, spell, casting_method) {
        view.apply_bestow_cast_overlay();
        changed = true;
    } else {
        let method = match casting_method {
            CastingMethod::Alternative(idx) => spell
                .alternative_casts
                .get(*idx)
                .cloned()
                .or_else(|| spell.cast_alternative_method_owned()),
            CastingMethod::PlayFrom {
                use_alternative: Some(idx),
                zone,
                ..
            }
            | CastingMethod::SplitOtherHalfPlayFrom {
                use_alternative: idx,
                zone,
                ..
            } => resolve_play_from_alternative_method(game, caster, spell, *zone, *idx)
                .or_else(|| spell.cast_alternative_method_owned()),
            _ => spell.cast_alternative_method_owned(),
        };

        if matches!(
            method,
            Some(crate::alternative_cast::AlternativeCastingMethod::Disturb { .. })
        ) {
            if let Some(disturb_view) = spell_view_for_disturb_cast(game, spell) {
                view = disturb_view;
                changed = true;
            }
        } else if let Some(method) = method.as_ref()
            && let Some(power_toughness) = method.prototype_power_toughness()
            && let Some(cost) = method.mana_cost()
            && view.apply_prototype_cast_overlay(cost.clone(), power_toughness)
        {
            changed = true;
        }
    }

    if cast_from_zone.is_some() || spell.zone == Zone::Stack {
        view.zone = cast_from_zone.unwrap_or_else(|| {
            inferred_cast_origin_zone_for_cost_filter(game, caster, spell, casting_method)
        });
        changed = true;
    }

    changed.then_some(view)
}

pub(crate) fn optional_life_cost_reduction_costs_for_cast(
    game: &GameState,
    caster: PlayerId,
    spell_id: ObjectId,
    casting_method: &CastingMethod,
    cast_from_zone: Option<Zone>,
) -> Vec<(ObjectId, ironsmith_core::OptionalLifeAdditionalCost)> {
    let Some(spell) = game.object(spell_id) else {
        return Vec::new();
    };
    let view = DerivedGameView::new(game);
    let modifier_sources = view.battlefield_spell_cost_modifier_sources();
    if modifier_sources.is_empty() {
        return Vec::new();
    }

    // A printed modifier source can be discovered without layered reads, but
    // inspecting its active abilities and the live stack spell can still need
    // them. Batch both classes before either lookup so a dirty cast state never
    // falls back to one full-board baseline per source.
    let mut prewarm_ids = modifier_sources.clone();
    if !prewarm_ids.contains(&spell_id) {
        prewarm_ids.push(spell_id);
    }
    view.prewarm_characteristics_forced(&prewarm_ids);

    let spell_for_filter =
        spell_view_for_cost_filter_match(game, caster, spell, casting_method, cast_from_zone)
            .unwrap_or_else(|| {
                let mut spell_for_filter = spell.clone();
                if let Some(chars) = view.current_characteristics_arc(spell_id) {
                    spell_for_filter.name = chars.name.clone().into();
                    spell_for_filter.card_types = chars.card_types.to_vec().into();
                    spell_for_filter.subtypes = chars.subtypes.to_vec().into();
                    spell_for_filter.supertypes = chars.supertypes.to_vec().into();
                    spell_for_filter.color_override = Some(chars.colors);
                }
                spell_for_filter
            });

    let mut costs = Vec::new();
    for perm_id in modifier_sources {
        let Some(perm) = game.object(perm_id) else {
            continue;
        };
        let controller = view
            .current_controller(perm_id)
            .unwrap_or_else(|| game.controller_of(perm));
        let filter_ctx = game
            .filter_context_for(controller, Some(perm_id))
            .with_caster(Some(caster));
        let Some(abilities) = view.static_abilities_rc(perm_id) else {
            continue;
        };
        for static_ability in abilities.iter() {
            if !static_ability.is_active(game, perm_id) {
                continue;
            }
            let Some(reduction) = static_ability.cost_reduction_mana_cost() else {
                continue;
            };
            let Some(optional) = &reduction.optional_life_additional_cost else {
                continue;
            };
            if reduction
                .filter
                .matches_non_recursive(&spell_for_filter, &filter_ctx, game)
            {
                costs.push((perm_id, optional.clone()));
            }
        }
    }

    costs
}

pub(crate) fn optional_life_cost_reduction_label(
    optional: &ironsmith_core::OptionalLifeAdditionalCost,
    source: ObjectId,
) -> String {
    format!("{} [source:{}]", optional.label, source.0)
}

fn spell_with_optional_life_cost_reduction_costs(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    casting_method: &CastingMethod,
) -> crate::object::Object {
    let mut spell_with_optional_costs = spell.clone();
    for (source, optional) in
        optional_life_cost_reduction_costs_for_cast(game, player, spell.id, casting_method, None)
    {
        let label = optional_life_cost_reduction_label(&optional, source);
        if spell_with_optional_costs
            .optional_costs
            .iter()
            .any(|existing| existing.source_label == label)
        {
            continue;
        }
        spell_with_optional_costs
            .optional_costs
            .push(crate::cost::OptionalCost::custom(
                label,
                crate::cost::TotalCost::from_cost(crate::costs::Cost::life(optional.life_cost)),
            ));
    }
    spell_with_optional_costs
}

fn disturb_linked_face_matches_cost_filter(
    game: &GameState,
    caster: PlayerId,
    spell: &crate::object::Object,
    filter: &crate::target::ObjectFilter,
    ctx: &crate::target::FilterContext,
) -> bool {
    if !filter.subtypes.contains(&crate::types::Subtype::Aura) {
        return false;
    }
    let Some(view) = spell_view_for_disturb_cast(game, spell) else {
        return false;
    };
    filter.matches_non_recursive(&view, &ctx.clone().with_caster(Some(caster)), game)
}

pub(crate) fn spell_matches_cast_filter(
    game: &GameState,
    spell: &crate::object::Object,
    spell_filter: &crate::target::ObjectFilter,
) -> bool {
    spell_filter.matches(spell, &crate::target::FilterContext::default(), game)
}

pub(crate) fn snapshot_matches_cast_filter(
    game: &GameState,
    snapshot: &crate::snapshot::ObjectSnapshot,
    spell_filter: &crate::target::ObjectFilter,
) -> bool {
    spell_filter.matches_snapshot(snapshot, &crate::target::FilterContext::default(), game)
}

pub(crate) fn spells_cast_this_turn_matching_filter(
    game: &GameState,
    player: PlayerId,
    spell_filter: &crate::target::ObjectFilter,
) -> u32 {
    if spell_filter == &crate::target::ObjectFilter::default() {
        return game.turn_store.turn_history.spells_cast_by_player(player);
    }

    game.turn_store
        .turn_history
        .spell_cast_snapshot_history()
        .iter()
        .filter(|snapshot| {
            snapshot.controller == player
                && snapshot_matches_cast_filter(game, snapshot, spell_filter)
        })
        .count() as u32
}

pub(crate) fn violates_cast_limit(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    spell_filter: &crate::target::ObjectFilter,
) -> bool {
    spell_matches_cast_filter(game, spell, spell_filter)
        && spells_cast_this_turn_matching_filter(game, player, spell_filter) >= 1
}

pub(crate) fn violates_any_cast_limit(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
) -> bool {
    game.effect_store
        .cant_effects
        .cast_limit_filters_for_player(player)
        .is_some_and(|filters| {
            filters
                .iter()
                .any(|spell_filter| violates_cast_limit(game, player, spell, spell_filter))
        })
}

pub(crate) fn violates_any_cant_cast_restriction(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
) -> bool {
    game.effect_store
        .cant_effects
        .cast_filters_for_player(player)
        .is_some_and(|filters| {
            filters.iter().any(|restriction| {
                let mut ctx = crate::target::FilterContext::default();
                if let Some(source) = restriction.source {
                    ctx = ctx.with_source(source);
                }
                restriction.filter.matches(spell, &ctx, game)
            })
        })
}

fn optional_cost_selection_subsets(
    optional_costs: &[crate::cost::OptionalCost],
) -> Vec<Vec<usize>> {
    fn visit(
        index: usize,
        len: usize,
        selected: &mut Vec<usize>,
        selections: &mut Vec<Vec<usize>>,
    ) {
        if index == len {
            selections.push(selected.clone());
            return;
        }

        visit(index + 1, len, selected, selections);
        selected.push(index);
        visit(index + 1, len, selected, selections);
        selected.pop();
    }

    let mut selections = Vec::new();
    visit(0, optional_costs.len(), &mut Vec::new(), &mut selections);
    selections
}

/// Visit complete CR 601.2 proposals formed by the joint yes/no choices for
/// optional costs. A repeatable cost needs only its zero- and one-payment
/// hypotheses here: the legality predicates tested before casting begins are
/// sensitive to whether that cost was paid, while the transaction records its
/// exact announced count later.
fn any_payable_optional_cost_proposal<F>(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    base_mana_cost: Option<&crate::mana::ManaCost>,
    casting_method: &CastingMethod,
    mut predicate: F,
) -> bool
where
    F: FnMut(
        &GameState,
        &crate::object::Object,
        Option<&crate::mana::ManaCost>,
        &DerivedGameView<'_>,
    ) -> bool,
{
    for selected in optional_cost_selection_subsets(&spell.optional_costs) {
        if selected.iter().any(|&index| {
            crate::cost::can_pay_cost_with_reason(
                game,
                spell.id,
                player,
                &spell.optional_costs[index].cost,
                crate::costs::PaymentReason::CastSpell,
            )
            .is_err()
        }) {
            continue;
        }

        let mut hypothetical_storage = (!selected.is_empty()).then(|| game.clone());
        if let Some(hypothetical) = hypothetical_storage.as_mut()
            && let Some(hypothetical_spell) = hypothetical.object_mut(spell.id)
        {
            hypothetical_spell.optional_costs_paid =
                crate::cost::OptionalCostsPaid::from_costs(&hypothetical_spell.optional_costs);
            for &index in &selected {
                hypothetical_spell.optional_costs_paid.pay_times(index, 1);
            }
        }
        let hypothetical = hypothetical_storage.as_ref().unwrap_or(game);

        let mut proposal = spell.clone();
        proposal.zone = Zone::Stack;
        proposal.optional_costs_paid =
            crate::cost::OptionalCostsPaid::from_costs(&proposal.optional_costs);
        for &index in &selected {
            proposal.optional_costs_paid.pay_times(index, 1);
        }

        let mut combined_pips = base_mana_cost
            .map(|cost| cost.pips().to_vec())
            .unwrap_or_default();
        for &index in &selected {
            if let Some(optional_mana_cost) = spell.optional_costs[index].cost.mana_cost() {
                combined_pips.extend(optional_mana_cost.pips().iter().cloned());
            }
        }
        let combined_cost =
            (!combined_pips.is_empty()).then(|| crate::mana::ManaCost::from_pips(combined_pips));
        let hypothetical_view = DerivedGameView::new(hypothetical);
        let effective_cost = combined_cost.as_ref().map(|cost| {
            calculate_effective_mana_cost_with_view_for_casting_method(
                hypothetical,
                player,
                &proposal,
                cost,
                casting_method,
                &hypothetical_view,
            )
        });
        let max_x = effective_cost
            .as_ref()
            .filter(|cost| cost.has_x())
            .map(|cost| {
                let potential = hypothetical_view.potential_mana(player);
                let mana_spend_policy = hypothetical.mana_spend_policy(player, Some(proposal.id));
                let allow_black_life = mana_cost_has_black_symbol(cost)
                    && hypothetical_view.player_can_pay_black_with_life_for_reason(
                        player,
                        crate::costs::PaymentReason::CastSpell,
                    );
                let caster_only = potential.max_x_for_cost_with_mana_spend_policy_and_black_life(
                    cost,
                    &mana_spend_policy,
                    allow_black_life,
                );
                max_x_payable_with_assist(hypothetical, player, proposal.id, cost)
                    .unwrap_or(caster_only)
            })
            .unwrap_or(0);

        for x_value in 0..=max_x {
            if effective_cost.as_ref().is_some_and(|cost| {
                !mana_cost_can_be_paid_by_caster_or_assist_with_view_at_x(
                    hypothetical,
                    player,
                    spell.id,
                    cost,
                    x_value,
                    &hypothetical_view,
                )
            }) {
                continue;
            }

            if !effective_cost.as_ref().is_some_and(|cost| cost.has_x()) {
                if predicate(
                    hypothetical,
                    &proposal,
                    effective_cost.as_ref(),
                    &hypothetical_view,
                ) {
                    return true;
                }
                continue;
            }

            let mut x_game = hypothetical.clone();
            let mut x_proposal = proposal.clone();
            x_proposal.x_value = Some(x_value);
            if let Some(game_spell) = x_game.object_mut(spell.id) {
                game_spell.x_value = Some(x_value);
            }
            let x_view = DerivedGameView::new(&x_game);
            if predicate(&x_game, &x_proposal, effective_cost.as_ref(), &x_view) {
                return true;
            }
        }
    }

    false
}

/// CR 601.3a allows casting to begin when any later announcement can make the
/// completed proposal escape a prohibition. Test optional costs jointly and X
/// at every payable value against the spell as it would exist on the stack.
fn every_payable_proposal_violates_cant_cast(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    mana_cost: Option<&crate::mana::ManaCost>,
    casting_method: &CastingMethod,
) -> bool {
    if !violates_any_cant_cast_restriction(game, player, spell) {
        return false;
    }

    !any_payable_optional_cost_proposal(
        game,
        player,
        spell,
        mana_cost,
        casting_method,
        |hypothetical, proposal, _effective_cost, _hypothetical_view| {
            !violates_any_cant_cast_restriction(hypothetical, player, proposal)
        },
    )
}

pub(crate) fn is_sorcery_speed_spell(spell: &crate::object::Object) -> bool {
    use crate::types::CardType;

    spell.has_card_type(CardType::Sorcery)
        || spell.has_card_type(CardType::Creature)
        || spell.has_card_type(CardType::Artifact)
        || spell.has_card_type(CardType::Enchantment)
        || spell.has_card_type(CardType::Planeswalker)
}

pub(crate) fn spell_has_active_flash_with_view(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    spell_id: ObjectId,
    view: &DerivedGameView<'_>,
) -> bool {
    spell.abilities.iter().any(|a| {
        if let crate::ability::AbilityKind::Static(s) = &a.kind {
            if s.has_flash() {
                return true;
            }
            if let Some(spec) = s.conditional_spell_keyword_spec()
                && spec.keyword == crate::static_abilities::ConditionalSpellKeywordKind::Flash
            {
                return crate::static_abilities::conditional_spell_keyword_active(
                    spec, game, player,
                );
            }
        }
        false
    }) || view.card_has_granted_static_ability_id(
        spell_id,
        Zone::Hand,
        player,
        crate::static_abilities::StaticAbilityId::Flash,
    ) || view.card_view_has_granted_static_ability_id(
        spell_id,
        spell,
        Zone::Hand,
        player,
        crate::static_abilities::StaticAbilityId::Flash,
    )
}

pub(crate) fn player_was_attacked_this_step(game: &GameState, player: PlayerId) -> bool {
    use crate::combat_state::AttackTarget;
    use crate::game_state::{Phase, Step};

    if !matches!(game.turn.phase, Phase::Combat) || game.turn.step != Some(Step::DeclareAttackers) {
        return false;
    }

    let Some(combat) = game.combat.as_ref() else {
        return false;
    };

    combat
        .attackers
        .iter()
        .any(|attacker| match attacker.target {
            AttackTarget::Player(defender) => defender == player,
            AttackTarget::Planeswalker(planeswalker_id) => game
                .object(planeswalker_id)
                .is_some_and(|planeswalker| game.controller_of(planeswalker) == player),
            AttackTarget::Battle(battle_id) => game.battle_protector(battle_id) == Some(player),
        })
}

pub(crate) fn this_spell_cast_restriction_allows(
    game: &GameState,
    player: PlayerId,
    kind: &crate::static_abilities::ThisSpellCastRestrictionKind,
) -> bool {
    let timing_allows = kind
        .timing
        .is_none_or(|timing| this_spell_cast_timing_allows(game, player, timing));
    if !timing_allows {
        return false;
    }
    kind.condition
        .as_ref()
        .is_none_or(|condition| this_spell_cast_condition_allows(game, player, condition))
}

pub(crate) fn this_spell_cast_timing_allows(
    game: &GameState,
    player: PlayerId,
    timing: crate::static_abilities::ThisSpellCastTiming,
) -> bool {
    use crate::game_state::{Phase, Step};
    use crate::static_abilities::ThisSpellCastTiming;

    match timing {
        ThisSpellCastTiming::DuringDeclareAttackersStep => {
            matches!(game.turn.phase, Phase::Combat)
                && game.turn.step == Some(Step::DeclareAttackers)
        }
        ThisSpellCastTiming::DuringCombat => matches!(game.turn.phase, Phase::Combat),
        ThisSpellCastTiming::DuringCombatBeforeBlockersAreDeclared => {
            matches!(game.turn.phase, Phase::Combat)
                && matches!(
                    game.turn.step,
                    Some(Step::BeginCombat | Step::DeclareAttackers)
                )
        }
        ThisSpellCastTiming::DuringCombatAfterBlockersAreDeclared => {
            matches!(game.turn.phase, Phase::Combat)
                && matches!(
                    game.turn.step,
                    Some(Step::DeclareBlockers | Step::CombatDamage | Step::EndCombat)
                )
        }
        ThisSpellCastTiming::DuringCombatOnYourTurnBeforeBlockersAreDeclared => {
            game.is_active_player(player)
                && matches!(game.turn.phase, Phase::Combat)
                && matches!(
                    game.turn.step,
                    Some(Step::BeginCombat | Step::DeclareAttackers)
                )
        }
        ThisSpellCastTiming::DuringCombatOnOpponentsTurn => {
            !game.is_active_player(player) && matches!(game.turn.phase, Phase::Combat)
        }
        ThisSpellCastTiming::BeforeAttackersAreDeclared => {
            matches!(game.turn.phase, Phase::Combat) && game.turn.step == Some(Step::BeginCombat)
        }
        ThisSpellCastTiming::BeforeCombatDamageStep => {
            matches!(game.turn.phase, Phase::Combat)
                && matches!(
                    game.turn.step,
                    Some(Step::BeginCombat | Step::DeclareAttackers | Step::DeclareBlockers)
                )
        }
        ThisSpellCastTiming::DuringOpponentsUpkeep => {
            !game.is_active_player(player)
                && matches!(game.turn.phase, Phase::Beginning)
                && game.turn.step == Some(Step::Upkeep)
        }
        ThisSpellCastTiming::DuringOpponentsTurnAfterUpkeep => {
            if game.is_active_player(player) {
                return false;
            }
            !matches!(
                (game.turn.phase, game.turn.step),
                (Phase::Beginning, Some(Step::Untap | Step::Upkeep))
            )
        }
        ThisSpellCastTiming::DuringYourEndStep => {
            game.is_active_player(player)
                && matches!(game.turn.phase, Phase::Ending)
                && game.turn.step == Some(Step::End)
        }
        ThisSpellCastTiming::AfterCombat => {
            matches!(game.turn.phase, Phase::NextMain | Phase::Ending)
        }
    }
}

pub(crate) fn players_matching_cast_restriction_filter(
    game: &GameState,
    player: PlayerId,
    filter: &crate::target::PlayerFilter,
) -> Vec<PlayerId> {
    let filter_ctx = game.filter_context_for(player, None);
    match filter {
        crate::target::PlayerFilter::You => vec![player],
        crate::target::PlayerFilter::Opponent => filter_ctx.opponents.clone(),
        crate::target::PlayerFilter::Teammate => filter_ctx.teammates.clone(),
        crate::target::PlayerFilter::Specific(id) => vec![*id],
        crate::target::PlayerFilter::Any => game
            .players
            .iter()
            .filter(|candidate| candidate.is_in_game())
            .map(|candidate| candidate.id)
            .collect(),
        crate::target::PlayerFilter::NotYou => game
            .players
            .iter()
            .filter_map(|candidate| {
                (candidate.is_in_game() && candidate.id != player).then_some(candidate.id)
            })
            .collect(),
        _ => Vec::new(),
    }
}

pub(crate) fn this_spell_cast_condition_allows(
    game: &GameState,
    player: PlayerId,
    condition: &crate::static_abilities::ThisSpellCastCondition,
) -> bool {
    match condition {
        crate::static_abilities::ThisSpellCastCondition::YouWereAttackedThisStep => {
            player_was_attacked_this_step(game, player)
        }
        crate::static_abilities::ThisSpellCastCondition::PlayerCastSpellThisTurnOrMore {
            player: player_filter,
            spell_filter,
            count,
        } => players_matching_cast_restriction_filter(game, player, player_filter)
            .into_iter()
            .map(|matched_player| {
                spells_cast_this_turn_matching_filter(game, matched_player, spell_filter)
            })
            .sum::<u32>()
            >= *count,
        crate::static_abilities::ThisSpellCastCondition::CreatureIsAttackingYou => {
            player_was_attacked_this_step(game, player)
                || game.combat.as_ref().is_some_and(|combat| {
                    combat.attackers.iter().any(|attacker| match attacker.target {
                        crate::combat_state::AttackTarget::Player(defender) => defender == player,
                        crate::combat_state::AttackTarget::Planeswalker(planeswalker_id) => game
                            .object(planeswalker_id)
                            .is_some_and(|planeswalker| game.controller_of(planeswalker) == player),
                        crate::combat_state::AttackTarget::Battle(battle_id) => {
                            game.battle_protector(battle_id) == Some(player)
                        }
                    })
                })
        }
        crate::static_abilities::ThisSpellCastCondition::NoPermanentsNamedOnBattlefield(name) => {
            !game.battlefield.iter().any(|&id| {
                game.object(id)
                    .is_some_and(|object| object.name.eq_ignore_ascii_case(name))
            })
        }
        crate::static_abilities::ThisSpellCastCondition::YouControlAtLeast { filter, count } => {
            let mut required_filter = filter.clone();
            required_filter.zone = Some(Zone::Battlefield);
            let filter_ctx = game.filter_context_for(player, None);
            game.battlefield
                .iter()
                .filter_map(|&id| game.object(id))
                .filter(|object| {
                    game.controller_of(object) == player
                        && required_filter.matches(object, &filter_ctx, game)
                })
                .count()
                >= *count as usize
        }
        crate::static_abilities::ThisSpellCastCondition::YouControlFewerCreaturesThanEachOpponent => {
            let your_creatures = game.creatures_controlled_by(player).len();
            game.players
                .iter()
                .filter(|opponent| opponent.is_in_game() && opponent.id != player)
                .all(|opponent| your_creatures < game.creatures_controlled_by(opponent.id).len())
        }
        crate::static_abilities::ThisSpellCastCondition::YouControlNameWordOrMore {
            word,
            count,
        } => game
            .permanents_controlled_by(player)
            .iter()
            .filter(|id| {
                game.object(**id).is_some_and(|object| {
                    object
                        .name
                        .to_ascii_lowercase()
                        .contains(&word.to_ascii_lowercase())
                })
            })
            .count()
            >= *count as usize,
    }
}

pub(crate) fn spell_cast_restrictions_allow(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
) -> bool {
    spell.abilities.iter().all(|ability| {
        let crate::ability::AbilityKind::Static(static_ability) = &ability.kind else {
            return true;
        };
        let Some(kind) = static_ability.this_spell_cast_restriction_kind() else {
            return true;
        };
        this_spell_cast_restriction_allows(game, player, &kind)
    })
}

pub(crate) fn has_valid_spell_timing(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    spell_id: ObjectId,
) -> bool {
    let view = DerivedGameView::new(game);
    has_valid_spell_timing_with_view(game, player, spell, spell_id, &view)
}

pub(crate) fn has_valid_spell_timing_with_view(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    spell_id: ObjectId,
    view: &DerivedGameView<'_>,
) -> bool {
    if game
        .effect_store
        .cant_effects
        .cast_spells_only_as_sorcery
        .contains(&player)
    {
        return game.is_active_player(player) && crate::turn::is_sorcery_timing(game);
    }

    if !is_sorcery_speed_spell(spell)
        || spell_has_active_flash_with_view(game, player, spell, spell_id, view)
    {
        return true;
    }

    // Sorcery-speed spells require: active player, main phase, empty stack.
    game.is_active_player(player) && crate::turn::is_sorcery_timing(game)
}

fn casting_method_grants_flash_timing(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    casting_method: &CastingMethod,
) -> bool {
    let method = match casting_method {
        CastingMethod::Alternative(idx) => spell.alternative_casts.get(*idx).cloned(),
        CastingMethod::PlayFrom {
            zone,
            use_alternative: Some(idx),
            ..
        }
        | CastingMethod::SplitOtherHalfPlayFrom {
            zone,
            use_alternative: idx,
            ..
        } => {
            crate::decision::resolve_play_from_alternative_method(game, player, spell, *zone, *idx)
                .or_else(|| spell.cast_alternative_method_owned())
        }
        _ => None,
    };
    matches!(
        method,
        Some(crate::alternative_cast::AlternativeCastingMethod::FlashWithAdditionalCost { .. })
    )
}

fn casting_method_grants_library_search_timing(
    game: &GameState,
    spell: &crate::object::Object,
    spell_id: ObjectId,
    casting_method: &CastingMethod,
) -> bool {
    matches!(
        casting_method,
        CastingMethod::PlayFrom {
            zone: Zone::Library,
            ..
        }
    ) && spell.zone == Zone::Library
        && game.current_has_static_ability_id(
            spell_id,
            crate::static_abilities::StaticAbilityId::CastThisCardFromLibraryWhileSearching,
        )
}

fn casting_method_grants_special_timing(
    ctx: &CastLegalityContext<'_>,
    spell: &crate::object::Object,
    spell_id: ObjectId,
    casting_method: &CastingMethod,
) -> bool {
    casting_method_grants_flash_timing(ctx.game, ctx.player, spell, casting_method)
        || casting_method_grants_sneak_timing(ctx.game, spell, casting_method)
        || (ctx.allow_library_search_cast_timing
            && casting_method_grants_library_search_timing(
                ctx.game,
                spell,
                spell_id,
                casting_method,
            ))
}

fn casting_method_grants_sneak_timing(
    game: &GameState,
    spell: &crate::object::Object,
    casting_method: &CastingMethod,
) -> bool {
    let method = match casting_method {
        CastingMethod::Alternative(idx) => spell.alternative_casts.get(*idx),
        _ => None,
    };
    let Some(method) = method else {
        return false;
    };
    method.name().eq_ignore_ascii_case("Sneak")
        && matches!(game.turn.phase, Phase::Combat)
        && game.turn.step == Some(Step::DeclareBlockers)
}

pub(crate) fn face_down_cast_mana_cost() -> crate::mana::ManaCost {
    crate::mana::ManaCost::from_pips(vec![vec![crate::mana::ManaSymbol::Generic(3)]])
}

pub(crate) fn spell_can_be_cast_face_down(spell: &crate::object::Object) -> bool {
    spell.abilities.iter().any(|ability| {
        matches!(
            &ability.kind,
            crate::ability::AbilityKind::Static(static_ability)
                if static_ability.turn_face_up_cost().is_some()
        )
    })
}

/// Resolve the mana cost for a spell cast from a specific zone and method.
pub fn spell_mana_cost_for_cast(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    casting_method: &CastingMethod,
    from_zone: Zone,
) -> Option<crate::mana::ManaCost> {
    let base_cost = match casting_method {
        CastingMethod::Normal => spell.mana_cost_owned(),
        CastingMethod::FaceDown => Some(face_down_cast_mana_cost()),
        CastingMethod::SplitOtherHalf => {
            if spell.zone == Zone::Stack {
                spell.mana_cost_owned()
            } else {
                linked_face_definition(game, spell).and_then(|def| def.card.mana_cost)
            }
        }
        CastingMethod::Fuse => {
            if spell.zone == Zone::Stack {
                spell.mana_cost_owned()
            } else {
                spell_view_for_fused_split_cast(game, spell).and_then(|view| view.mana_cost_owned())
            }
        }
        CastingMethod::Alternative(idx) => {
            if let Some(method) = spell
                .alternative_casts
                .get(*idx)
                .or(spell.cast_alternative_method.as_deref())
            {
                if matches!(
                    method,
                    crate::alternative_cast::AlternativeCastingMethod::Plot { .. }
                ) {
                    Some(crate::mana::ManaCost::new())
                } else if method.total_cost().is_some() {
                    Some(method.mana_cost().cloned().unwrap_or_default())
                } else {
                    method
                        .mana_cost()
                        .cloned()
                        .or_else(|| spell.mana_cost_owned())
                }
            } else {
                spell.mana_cost_owned()
            }
        }
        CastingMethod::GrantedEscape { .. } => spell.mana_cost_owned(),
        CastingMethod::GrantedFlashback => spell.mana_cost_owned(),
        CastingMethod::PlayFrom {
            use_alternative: None,
            ..
        } => spell.mana_cost_owned(),
        CastingMethod::PlayFrom {
            use_alternative: Some(idx),
            zone,
            ..
        }
        | CastingMethod::SplitOtherHalfPlayFrom {
            use_alternative: idx,
            zone,
            ..
        } => {
            if let Some(method) =
                resolve_play_from_alternative_method(game, player, spell, *zone, *idx)
                    .or_else(|| spell.cast_alternative_method_owned())
            {
                if matches!(
                    method,
                    crate::alternative_cast::AlternativeCastingMethod::Plot { .. }
                ) {
                    Some(crate::mana::ManaCost::new())
                } else if method.total_cost().is_some() {
                    Some(method.mana_cost().cloned().unwrap_or_default())
                } else {
                    method
                        .mana_cost()
                        .cloned()
                        .or_else(|| spell.mana_cost_owned())
                }
            } else {
                spell.mana_cost_owned()
            }
        }
    };

    let base_cost = if let Some(cost) = base_cost {
        if let Some(method) =
            alternative_method_for_casting_method(game, player, spell, casting_method)
        {
            Some(apply_emerge_reduction_to_alternative_mana_cost(
                game, player, spell.id, &method, &cost,
            ))
        } else {
            Some(cost)
        }
    } else {
        None
    };

    if from_zone == Zone::Command {
        let tax = game.commander_cast_count(spell.id).saturating_mul(2);
        base_cost.map(|cost| cost.add_generic(tax))
    } else {
        base_cost
    }
}

fn alternative_method_for_casting_method(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    casting_method: &CastingMethod,
) -> Option<crate::alternative_cast::AlternativeCastingMethod> {
    match casting_method {
        CastingMethod::Alternative(idx) => spell
            .alternative_casts
            .get(*idx)
            .or(spell.cast_alternative_method.as_deref())
            .cloned(),
        CastingMethod::PlayFrom {
            use_alternative: Some(idx),
            zone,
            ..
        }
        | CastingMethod::SplitOtherHalfPlayFrom {
            use_alternative: idx,
            zone,
            ..
        } => resolve_play_from_alternative_method(game, player, spell, *zone, *idx)
            .or_else(|| spell.cast_alternative_method_owned()),
        CastingMethod::Normal => spell.cast_alternative_method_owned(),
        CastingMethod::FaceDown
        | CastingMethod::SplitOtherHalf
        | CastingMethod::Fuse
        | CastingMethod::GrantedEscape { .. }
        | CastingMethod::GrantedFlashback
        | CastingMethod::PlayFrom {
            use_alternative: None,
            ..
        } => None,
    }
}

pub(crate) fn alternative_method_uses_printed_mana_cost(
    method: &crate::alternative_cast::AlternativeCastingMethod,
) -> bool {
    matches!(
        method,
        crate::alternative_cast::AlternativeCastingMethod::JumpStart { .. }
            | crate::alternative_cast::AlternativeCastingMethod::Escape { cost: None, .. }
    )
}

pub(crate) fn casting_method_requires_printed_mana_cost(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    casting_method: &CastingMethod,
) -> bool {
    match casting_method {
        CastingMethod::Normal
        | CastingMethod::GrantedEscape { .. }
        | CastingMethod::GrantedFlashback
        | CastingMethod::PlayFrom {
            use_alternative: None,
            ..
        } => true,
        CastingMethod::Alternative(idx) => spell
            .alternative_casts
            .get(*idx)
            .is_some_and(alternative_method_uses_printed_mana_cost),
        CastingMethod::PlayFrom {
            use_alternative: Some(idx),
            zone,
            ..
        }
        | CastingMethod::SplitOtherHalfPlayFrom {
            use_alternative: idx,
            zone,
            ..
        } => resolve_play_from_alternative_method(game, player, spell, *zone, *idx)
            .or_else(|| spell.cast_alternative_method_owned())
            .as_ref()
            .is_some_and(alternative_method_uses_printed_mana_cost),
        _ => false,
    }
}

/// Check if a spell can be cast by a player using the given casting method.
pub fn can_cast_spell(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    casting_method: &CastingMethod,
) -> bool {
    let view = DerivedGameView::new(game);
    can_cast_spell_with_view(game, player, spell, casting_method, &view)
}

/// Validate the fully announced CR 601.2 proposal without testing payment.
///
/// Unlike initial action discovery, this is deliberately strict: X, modes,
/// optional costs, targets, and cast overlays have already been committed to
/// the proposed stack object, so no further look-ahead is permitted here.
pub(crate) fn completed_cast_proposal_is_legal(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    casting_method: &CastingMethod,
) -> bool {
    completed_cast_proposal_is_legal_with_timing_permission(
        game,
        player,
        spell,
        casting_method,
        false,
    )
}

/// Validate a completed proposal initiated by a resolving effect.
///
/// The effect supplies the permission to cast at this time, but it does not
/// override prohibitions, cast limits, or restrictions printed on the spell.
pub(crate) fn completed_effect_driven_cast_proposal_is_legal(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    casting_method: &CastingMethod,
) -> bool {
    completed_cast_proposal_is_legal_with_timing_permission(
        game,
        player,
        spell,
        casting_method,
        true,
    )
}

fn completed_cast_proposal_is_legal_with_timing_permission(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    casting_method: &CastingMethod,
    timing_permission_from_effect: bool,
) -> bool {
    if violates_any_cant_cast_restriction(game, player, spell)
        || violates_any_cast_limit(game, player, spell)
        || spell.is_land()
        || !spell_cast_restrictions_allow(game, player, spell)
    {
        return false;
    }

    let view = DerivedGameView::new(game);
    let ctx = CastLegalityContext::new(game, player, &view);
    timing_permission_from_effect
        || has_valid_spell_timing_with_view(game, player, spell, spell.id, &view)
        || casting_method_grants_special_timing(&ctx, spell, spell.id, casting_method)
}

/// Check whether a player could begin casting a spell from hand for suspend.
///
/// This enforces cast prohibitions, cast limits, timing, and explicit
/// "cast this spell only ..." restrictions without requiring a printable mana
/// cost or legal targets yet.
pub fn can_begin_to_cast_from_hand_for_suspend(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
) -> bool {
    if violates_any_cant_cast_restriction(game, player, spell) {
        return false;
    }

    if violates_any_cast_limit(game, player, spell) {
        return false;
    }

    if spell.is_land() {
        return false;
    }

    if !has_valid_spell_timing(game, player, spell, spell.id) {
        return false;
    }

    spell_cast_restrictions_allow(game, player, spell)
}

pub(crate) fn spell_has_legal_targets_for_cast_with_view(
    game: &GameState,
    spell: &crate::object::Object,
    spell_id: ObjectId,
    program_override: Option<&crate::resolution::ResolutionProgram>,
    effects_override: Option<&[crate::effect::Effect]>,
    player: PlayerId,
    view: &DerivedGameView<'_>,
) -> bool {
    if let Some(effects) = effects_override {
        return effects.is_empty()
            || view.spell_has_legal_targets(effects, player, Some(spell_id), None);
    }

    if let Some(program) = program_override.or(spell.spell_effect.as_deref()) {
        return crate::game_loop::spell_program_has_legal_targets_with_modes_and_view(
            game,
            program,
            player,
            Some(spell_id),
            None,
            view,
        );
    }

    let synthesized_aura_effects = if spell.subtypes.contains(&crate::types::Subtype::Aura) {
        spell
            .aura_attach_filter
            .clone()
            .map(|filter| vec![crate::effect::Effect::attach_to(filter.target_spec())])
    } else {
        None
    };
    let effects = synthesized_aura_effects.as_deref().unwrap_or(&[]);
    effects.is_empty() || view.spell_has_legal_targets(effects, player, Some(spell_id), None)
}

fn spree_mode_selections(mode_count: usize, min: usize, max: usize) -> Vec<Vec<usize>> {
    fn visit(
        next: usize,
        mode_count: usize,
        min: usize,
        max: usize,
        selected: &mut Vec<usize>,
        out: &mut Vec<Vec<usize>>,
    ) {
        if selected.len() >= min {
            out.push(selected.clone());
        }
        if selected.len() == max {
            return;
        }
        for mode in next..mode_count {
            selected.push(mode);
            visit(mode + 1, mode_count, min, max, selected, out);
            selected.pop();
        }
    }

    let mut selections = Vec::new();
    visit(0, mode_count, min, max, &mut Vec::new(), &mut selections);
    selections
}

#[allow(clippy::too_many_arguments)]
fn has_payable_legal_spree_selection_with_view(
    game: &GameState,
    spell: &crate::object::Object,
    spell_id: ObjectId,
    program_override: Option<&crate::resolution::ResolutionProgram>,
    effects_override: Option<&[crate::effect::Effect]>,
    player: PlayerId,
    base_mana_cost: Option<&crate::mana::ManaCost>,
    casting_method: &CastingMethod,
    view: &DerivedGameView<'_>,
) -> Option<bool> {
    let program = program_override.or(spell.spell_effect.as_deref())?;
    let modal = program.all_effects().into_iter().find_map(|effect| {
        effect
            .0
            .get_modal_spec_with_context(game, player, spell_id)
            .filter(|modal| modal.spree)
    })?;
    let min = match modal.min_modes {
        crate::effect::Value::Fixed(value) => value.max(0) as usize,
        _ => return Some(false),
    };
    let max = match modal.max_modes {
        crate::effect::Value::Fixed(value) => {
            (value.max(0) as usize).min(modal.mode_descriptions.len())
        }
        _ => return Some(false),
    };

    for modes in spree_mode_selections(modal.mode_descriptions.len(), min, max) {
        let targets_are_legal = if let Some(effects) = effects_override {
            view.spell_has_legal_targets(effects, player, Some(spell_id), Some(&modes))
        } else {
            crate::game_loop::spell_program_has_legal_targets_with_modes_and_view(
                game,
                program,
                player,
                Some(spell_id),
                Some(&modes),
                view,
            )
        };
        if !targets_are_legal {
            continue;
        }

        let mut pips = base_mana_cost
            .map(|cost| cost.pips().to_vec())
            .unwrap_or_default();
        for (index, optional) in spell.optional_costs.iter().enumerate() {
            for _ in 0..spell.optional_costs_paid.times_paid(index) {
                if let Some(cost) = optional.cost.mana_cost() {
                    pips.extend(cost.pips().iter().cloned());
                }
            }
        }
        for mode in &modes {
            if let Some(cost) = modal.mode_additional_mana_costs.get(*mode) {
                pips.extend(cost.pips().iter().cloned());
            }
        }
        let combined = crate::mana::ManaCost::from_pips(pips);
        let effective = calculate_effective_mana_cost_with_view_for_casting_method(
            game,
            player,
            spell,
            &combined,
            casting_method,
            view,
        );
        if mana_cost_can_be_paid_by_caster_or_assist_with_view_at_x(
            game,
            player,
            spell_id,
            &effective,
            spell.x_value.unwrap_or(0),
            view,
        ) {
            return Some(true);
        }
    }

    Some(false)
}

fn spell_has_legal_targets_for_cast_or_payable_optional_cost_hypothesis_with_view(
    game: &GameState,
    spell: &crate::object::Object,
    spell_id: ObjectId,
    program_override: Option<&crate::resolution::ResolutionProgram>,
    effects_override: Option<&[crate::effect::Effect]>,
    player: PlayerId,
    base_mana_cost: Option<&crate::mana::ManaCost>,
    casting_method: &CastingMethod,
) -> bool {
    let spell_with_optional_costs =
        spell_with_optional_life_cost_reduction_costs(game, player, spell, casting_method);
    any_payable_optional_cost_proposal(
        game,
        player,
        &spell_with_optional_costs,
        base_mana_cost,
        casting_method,
        |hypothetical, proposal, _effective_cost, hypothetical_view| {
            if casting_method_is_mutate(hypothetical, player, proposal, casting_method) {
                let mutate_target =
                    crate::target::ChooseSpec::target(crate::target::ChooseSpec::Object(
                        crate::target::ObjectFilter::creature()
                            .owned_by(crate::target::PlayerFilter::Specific(proposal.owner))
                            .without_subtype(crate::types::Subtype::Human),
                    ));
                let requirement = crate::effect::Effect::new(
                    crate::effects::TargetOnlyEffect::new(mutate_target),
                );
                if !hypothetical_view.spell_has_legal_targets(
                    &[requirement],
                    player,
                    Some(spell_id),
                    None,
                ) {
                    return false;
                }
            }
            if let Some(legal) = has_payable_legal_spree_selection_with_view(
                hypothetical,
                proposal,
                spell_id,
                program_override,
                effects_override,
                player,
                base_mana_cost,
                casting_method,
                hypothetical_view,
            ) {
                return legal;
            }
            spell_has_legal_targets_for_cast_with_view(
                &hypothetical,
                proposal,
                spell_id,
                program_override,
                effects_override,
                player,
                hypothetical_view,
            )
        },
    )
}

fn mana_cost_can_be_paid_with_view_at_x(
    game: &GameState,
    player: PlayerId,
    spell_id: ObjectId,
    cost: &crate::mana::ManaCost,
    x_value: u32,
    view: &DerivedGameView<'_>,
) -> bool {
    let potential = view.potential_mana(player);
    let mana_spend_policy = game.mana_spend_policy(player, Some(spell_id));
    let allow_any_color_for_obvious = mana_spend_policy.has_any_color_spending()
        || game.has_source_filtered_mana_spend_permission(player, Some(spell_id));
    let allow_black_life = mana_cost_has_black_symbol(cost)
        && view.player_can_pay_black_with_life_for_reason(
            player,
            crate::costs::PaymentReason::CastSpell,
        );
    !mana_cost_is_obviously_unpayable(
        &potential,
        cost,
        allow_any_color_for_obvious,
        allow_black_life,
    ) && can_pay_mana_cost_with_available_sources(
        game,
        player,
        Some(spell_id),
        cost,
        x_value,
        crate::costs::PaymentReason::CastSpell,
        &mana_spend_policy,
        allow_black_life,
        view,
    )
}

fn mana_cost_with_locked_x_and_generic_reduction(
    cost: &crate::mana::ManaCost,
    x_value: u32,
    reduction: u32,
) -> crate::mana::ManaCost {
    let mut pips = Vec::new();
    let mut generic_from_x = 0u32;
    for pip in cost.pips() {
        if pip
            .iter()
            .any(|symbol| matches!(symbol, crate::mana::ManaSymbol::X))
        {
            generic_from_x = generic_from_x.saturating_add(x_value);
        } else {
            pips.push(pip.clone());
        }
    }
    crate::mana::ManaCost::from_pips(pips)
        .add_generic(generic_from_x)
        .reduce_generic(reduction)
}

fn mana_cost_can_be_paid_by_caster_or_assist_with_view_at_x(
    game: &GameState,
    caster: PlayerId,
    spell_id: ObjectId,
    cost: &crate::mana::ManaCost,
    x_value: u32,
    view: &DerivedGameView<'_>,
) -> bool {
    if mana_cost_can_be_paid_with_view_at_x(game, caster, spell_id, cost, x_value, view) {
        return true;
    }
    if !game
        .current_has_static_ability_id(spell_id, crate::static_abilities::StaticAbilityId::Assist)
    {
        return false;
    }

    let x_pips = cost
        .pips()
        .iter()
        .filter(|pip| {
            pip.iter()
                .any(|symbol| matches!(symbol, crate::mana::ManaSymbol::X))
        })
        .count() as u32;
    let generic_total = cost
        .generic_mana_total()
        .saturating_add(x_pips.saturating_mul(x_value));
    if generic_total == 0 {
        return false;
    }

    game.turn_store
        .turn_order
        .iter()
        .copied()
        .filter(|helper| *helper != caster && game.player(*helper).is_some())
        .any(|helper| {
            (1..=generic_total).any(|contribution| {
                let helper_cost = crate::mana::ManaCost::new().add_generic(contribution);
                if !mana_cost_can_be_paid_with_view_at_x(
                    game,
                    helper,
                    spell_id,
                    &helper_cost,
                    0,
                    view,
                ) {
                    return false;
                }
                let caster_cost =
                    mana_cost_with_locked_x_and_generic_reduction(cost, x_value, contribution);
                mana_cost_can_be_paid_with_view_at_x(game, caster, spell_id, &caster_cost, 0, view)
            })
        })
}

fn mana_cost_can_be_paid_by_caster_or_assist_with_view(
    game: &GameState,
    caster: PlayerId,
    spell_id: ObjectId,
    cost: &crate::mana::ManaCost,
    view: &DerivedGameView<'_>,
) -> bool {
    mana_cost_can_be_paid_by_caster_or_assist_with_view_at_x(game, caster, spell_id, cost, 0, view)
}

pub(crate) fn max_x_payable_with_assist(
    game: &GameState,
    caster: PlayerId,
    spell_id: ObjectId,
    cost: &crate::mana::ManaCost,
) -> Option<u32> {
    if !game
        .current_has_static_ability_id(spell_id, crate::static_abilities::StaticAbilityId::Assist)
        || !cost.has_x()
    {
        return None;
    }
    let view = DerivedGameView::new(game);
    let upper_bound = game
        .turn_store
        .turn_order
        .iter()
        .copied()
        .map(|player| view.potential_mana(player).total())
        .sum::<u32>();
    Some(
        (0..=upper_bound)
            .rev()
            .find(|x_value| {
                mana_cost_can_be_paid_by_caster_or_assist_with_view_at_x(
                    game, caster, spell_id, cost, *x_value, &view,
                )
            })
            .unwrap_or(0),
    )
}

fn effective_cost_with_affordable_optional_cost_hypothesis(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    base_cost: &crate::mana::ManaCost,
    casting_method: &CastingMethod,
) -> Option<crate::mana::ManaCost> {
    let spell_with_optional_costs =
        spell_with_optional_life_cost_reduction_costs(game, player, spell, casting_method);

    let mut adjusted = None;
    any_payable_optional_cost_proposal(
        game,
        player,
        &spell_with_optional_costs,
        Some(base_cost),
        casting_method,
        |_hypothetical, _proposal, effective_cost, _hypothetical_view| {
            if let Some(effective_cost) = effective_cost
                && effective_cost != base_cost
            {
                adjusted = Some(effective_cost.clone());
                return true;
            }
            false
        },
    );
    adjusted
}

pub(crate) fn can_cast_spell_with_context(
    spell: &crate::object::Object,
    casting_method: &CastingMethod,
    ctx: &CastLegalityContext<'_>,
) -> bool {
    let total_started_at = PerfTimer::start();
    let game = ctx.game;
    let player = ctx.player;
    let view = ctx.view;
    if game.is_planar_card(spell.id) {
        return false;
    }
    let cast_view = match casting_method {
        CastingMethod::FaceDown => {
            if !spell_can_be_cast_face_down(spell) {
                return false;
            }
            Some(spell_view_for_face_down_cast(spell))
        }
        CastingMethod::SplitOtherHalf | CastingMethod::SplitOtherHalfPlayFrom { .. } => {
            match spell_view_for_split_other_half_cast(game, spell) {
                Some(view) => Some(view),
                None => return false,
            }
        }
        CastingMethod::Fuse => match spell_view_for_fused_split_cast(game, spell) {
            Some(view) => Some(view),
            None => return false,
        },
        _ if casting_method_is_bestow(game, player, spell, casting_method) => {
            let mut view = spell.clone();
            view.apply_bestow_cast_overlay();
            Some(view)
        }
        _ => None,
    };
    let spell_for_checks = cast_view.as_ref().unwrap_or(spell);

    if let Some(method) = match casting_method {
        CastingMethod::Alternative(idx) => spell.alternative_casts.get(*idx).cloned(),
        CastingMethod::PlayFrom {
            use_alternative: Some(idx),
            zone,
            ..
        }
        | CastingMethod::SplitOtherHalfPlayFrom {
            use_alternative: idx,
            zone,
            ..
        } => resolve_play_from_alternative_method(game, player, spell, *zone, *idx)
            .or_else(|| spell.cast_alternative_method_owned()),
        _ => spell.cast_alternative_method_owned(),
    } && let Some(condition) = method.cast_condition()
        && !crate::static_abilities::this_spell_cost_condition_is_active_for_cast(
            game,
            spell.id,
            condition,
            &[],
        )
    {
        return false;
    }

    let restrictions_started_at = PerfTimer::start();
    let proposal_mana_cost = spell_mana_cost_for_cast(
        game,
        player,
        spell_for_checks,
        casting_method,
        spell_for_checks.zone,
    );
    if every_payable_proposal_violates_cant_cast(
        game,
        player,
        spell_for_checks,
        proposal_mana_cost.as_ref(),
        casting_method,
    ) {
        ctx.add_restrictions_ms(restrictions_started_at.elapsed_ms());
        ctx.add_total_ms(total_started_at.elapsed_ms());
        return false;
    }
    if violates_any_cast_limit(game, player, spell_for_checks) {
        ctx.add_restrictions_ms(restrictions_started_at.elapsed_ms());
        ctx.add_total_ms(total_started_at.elapsed_ms());
        return false;
    }
    if spell_for_checks.is_land() {
        ctx.add_restrictions_ms(restrictions_started_at.elapsed_ms());
        ctx.add_total_ms(total_started_at.elapsed_ms());
        return false;
    }
    if !spell_cast_restrictions_allow(game, player, spell_for_checks) {
        ctx.add_restrictions_ms(restrictions_started_at.elapsed_ms());
        ctx.add_total_ms(total_started_at.elapsed_ms());
        return false;
    }
    ctx.add_restrictions_ms(restrictions_started_at.elapsed_ms());

    let timing_started_at = PerfTimer::start();
    if !has_valid_spell_timing_with_view(game, player, spell_for_checks, spell.id, view)
        && !casting_method_grants_special_timing(ctx, spell_for_checks, spell.id, casting_method)
    {
        ctx.add_timing_ms(timing_started_at.elapsed_ms());
        ctx.add_total_ms(total_started_at.elapsed_ms());
        return false;
    }
    ctx.add_timing_ms(timing_started_at.elapsed_ms());

    let base_mana_cost = spell_mana_cost_for_cast(game, player, spell, casting_method, spell.zone);
    if base_mana_cost.is_none()
        && casting_method_requires_printed_mana_cost(game, player, spell, casting_method)
    {
        return false;
    }

    let target_started_at = PerfTimer::start();
    let program = cast_view
        .as_ref()
        .and_then(|view| view.spell_effect.as_deref())
        .or(spell.spell_effect.as_deref());
    let has_legal_targets =
        spell_has_legal_targets_for_cast_or_payable_optional_cost_hypothesis_with_view(
            game,
            spell_for_checks,
            spell.id,
            program,
            None,
            player,
            base_mana_cost.as_ref(),
            casting_method,
        );
    ctx.add_target_legality_ms(target_started_at.elapsed_ms());
    if !has_legal_targets {
        ctx.add_total_ms(total_started_at.elapsed_ms());
        return false;
    }

    if let Some(base_cost) = base_mana_cost.as_ref() {
        let cost_started_at = PerfTimer::start();
        let effective_cost = if ctx
            .can_use_printed_cost_directly(spell_has_intrinsic_cost_adjustments(spell_for_checks))
        {
            base_cost.clone()
        } else if ctx
            .spell_cost_needs_adjustment(spell_has_intrinsic_cost_adjustments(spell_for_checks))
        {
            calculate_effective_mana_cost_with_view_for_casting_method(
                game,
                player,
                spell_for_checks,
                base_cost,
                casting_method,
                view,
            )
        } else {
            apply_minimum_spell_total_mana_with_view(
                view,
                &apply_payment_reason_mana_adjustments(
                    game,
                    player,
                    Some(spell.id),
                    base_cost,
                    crate::costs::PaymentReason::CastSpell,
                ),
            )
        };
        ctx.add_cost_adjustment_ms(cost_started_at.elapsed_ms());

        let affordability_started_at = PerfTimer::start();
        let can_pay_effective = mana_cost_can_be_paid_by_caster_or_assist_with_view(
            game,
            player,
            spell.id,
            &effective_cost,
            view,
        );
        let can_pay_with_optional_reduction = !can_pay_effective
            && effective_cost_with_affordable_optional_cost_hypothesis(
                game,
                player,
                spell_for_checks,
                base_cost,
                casting_method,
            )
            .is_some_and(|cost| {
                mana_cost_can_be_paid_by_caster_or_assist_with_view(
                    game, player, spell.id, &cost, view,
                )
            });
        if !can_pay_effective && !can_pay_with_optional_reduction {
            ctx.add_affordability_ms(affordability_started_at.elapsed_ms());
            ctx.add_total_ms(total_started_at.elapsed_ms());
            return false;
        }
        ctx.add_affordability_ms(affordability_started_at.elapsed_ms());
    }

    ctx.add_total_ms(total_started_at.elapsed_ms());
    true
}

pub(crate) fn can_cast_spell_with_view(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    casting_method: &CastingMethod,
    view: &DerivedGameView<'_>,
) -> bool {
    let ctx = CastLegalityContext::new(game, player, view);
    can_cast_spell_with_context(spell, casting_method, &ctx)
}

// ============================================================================
// Unified Spell Casting Validation
// ============================================================================

/// Additional requirements for casting a spell beyond mana.
#[derive(Debug, Clone, Default)]
pub struct AdditionalCastRequirements {
    /// Cards that must be exiled from graveyard (excluding the spell itself).
    pub exile_from_graveyard: u32,
    /// Cards that must be discarded from hand.
    pub discard_from_hand: u32,
    /// A TotalCost that must be paid (for alternative costs like Force of Will).
    /// This is checked with spell exclusion (the spell being cast is excluded from hand).
    pub total_cost: Option<crate::cost::TotalCost>,
    /// If true, spell must be instant or sorcery only.
    pub must_be_instant_or_sorcery: bool,
}

pub(crate) fn can_cast_with_cost_with_view(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    spell_id: crate::ids::ObjectId,
    mana_cost: Option<&crate::mana::ManaCost>,
    effects_override: Option<&[crate::effect::Effect]>,
    requirements: &AdditionalCastRequirements,
    view: &DerivedGameView<'_>,
) -> bool {
    can_cast_with_cost_with_view_for_casting_method(
        game,
        player,
        spell,
        spell_id,
        mana_cost,
        effects_override,
        requirements,
        &CastingMethod::Normal,
        view,
    )
}

pub(crate) fn can_cast_with_cost_with_view_for_casting_method(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    spell_id: crate::ids::ObjectId,
    mana_cost: Option<&crate::mana::ManaCost>,
    effects_override: Option<&[crate::effect::Effect]>,
    requirements: &AdditionalCastRequirements,
    casting_method: &CastingMethod,
    view: &DerivedGameView<'_>,
) -> bool {
    let ctx = CastLegalityContext::new(game, player, view);
    can_cast_with_cost_with_context(
        spell,
        spell_id,
        mana_cost,
        effects_override,
        requirements,
        casting_method,
        &ctx,
    )
}

pub(crate) fn can_cast_with_cost_with_context(
    spell: &crate::object::Object,
    spell_id: crate::ids::ObjectId,
    mana_cost: Option<&crate::mana::ManaCost>,
    effects_override: Option<&[crate::effect::Effect]>,
    requirements: &AdditionalCastRequirements,
    casting_method: &CastingMethod,
    ctx: &CastLegalityContext<'_>,
) -> bool {
    use crate::types::CardType;
    let game = ctx.game;
    let player = ctx.player;
    let view = ctx.view;
    if game.is_planar_card(spell_id) {
        return false;
    }
    let cast_view = if casting_method_is_bestow(game, player, spell, casting_method) {
        let mut view = spell.clone();
        view.apply_bestow_cast_overlay();
        Some(view)
    } else {
        None
    };
    let spell_for_checks = cast_view.as_ref().unwrap_or(spell);

    if let Some(method) = match casting_method {
        CastingMethod::Alternative(idx) => spell.alternative_casts.get(*idx).cloned(),
        CastingMethod::PlayFrom {
            use_alternative: Some(idx),
            zone,
            ..
        }
        | CastingMethod::SplitOtherHalfPlayFrom {
            use_alternative: idx,
            zone,
            ..
        } => resolve_play_from_alternative_method(game, player, spell, *zone, *idx)
            .or_else(|| spell.cast_alternative_method_owned()),
        _ => spell.cast_alternative_method_owned(),
    } && let Some(condition) = method.cast_condition()
        && !crate::static_abilities::this_spell_cost_condition_is_active_for_cast(
            game,
            spell_id,
            condition,
            &[],
        )
    {
        return false;
    }

    let restrictions_started_at = PerfTimer::start();
    if every_payable_proposal_violates_cant_cast(
        game,
        player,
        spell_for_checks,
        mana_cost,
        casting_method,
    ) {
        ctx.add_restrictions_ms(restrictions_started_at.elapsed_ms());
        return false;
    }
    if violates_any_cast_limit(game, player, spell_for_checks) {
        ctx.add_restrictions_ms(restrictions_started_at.elapsed_ms());
        return false;
    }
    if spell_for_checks.is_land() {
        ctx.add_restrictions_ms(restrictions_started_at.elapsed_ms());
        return false;
    }
    if requirements.must_be_instant_or_sorcery
        && !spell.has_card_type(CardType::Instant)
        && !spell.has_card_type(CardType::Sorcery)
    {
        ctx.add_restrictions_ms(restrictions_started_at.elapsed_ms());
        return false;
    }
    if !spell_cast_restrictions_allow(game, player, spell_for_checks) {
        ctx.add_restrictions_ms(restrictions_started_at.elapsed_ms());
        return false;
    }
    ctx.add_restrictions_ms(restrictions_started_at.elapsed_ms());

    let timing_started_at = PerfTimer::start();
    if !has_valid_spell_timing_with_view(game, player, spell_for_checks, spell_id, view)
        && !casting_method_grants_special_timing(ctx, spell_for_checks, spell_id, casting_method)
    {
        ctx.add_timing_ms(timing_started_at.elapsed_ms());
        return false;
    }
    ctx.add_timing_ms(timing_started_at.elapsed_ms());

    let target_started_at = PerfTimer::start();
    let effects = effects_override;
    let program = if effects_override.is_some() {
        None
    } else {
        spell_for_checks.spell_effect.as_deref()
    };
    let has_legal_targets =
        spell_has_legal_targets_for_cast_or_payable_optional_cost_hypothesis_with_view(
            game,
            spell_for_checks,
            spell_id,
            program,
            effects,
            player,
            mana_cost,
            casting_method,
        );
    ctx.add_target_legality_ms(target_started_at.elapsed_ms());
    if !has_legal_targets {
        return false;
    }

    let Some(player_obj) = game.player(player) else {
        return false;
    };

    // Check exile from graveyard requirement
    if requirements.exile_from_graveyard > 0 {
        let other_cards_in_graveyard = player_obj
            .graveyard
            .iter()
            .filter(|&&id| id != spell_id)
            .count();
        if other_cards_in_graveyard < requirements.exile_from_graveyard as usize {
            return false;
        }
    }

    // Check discard from hand requirement
    // For Jump-Start, need at least discard_from_hand cards in hand
    if requirements.discard_from_hand > 0
        && (player_obj.hand.len() as u32) < requirements.discard_from_hand
    {
        return false;
    }

    // Check TotalCost requirement (for Force of Will style costs)
    if let Some(ref total_cost) = requirements.total_cost {
        for individual_cost in total_cost.costs() {
            if !can_pay_cost_with_spell_exclusion(game, player, individual_cost, Some(spell_id)) {
                return false;
            }
        }
    }

    if let Some(cost) = mana_cost {
        let cost_started_at = PerfTimer::start();
        let adjusted = if ctx
            .can_use_printed_cost_directly(spell_has_intrinsic_cost_adjustments(spell_for_checks))
        {
            cost.clone()
        } else if ctx
            .spell_cost_needs_adjustment(spell_has_intrinsic_cost_adjustments(spell_for_checks))
        {
            calculate_effective_mana_cost_with_view_for_casting_method(
                game,
                player,
                spell_for_checks,
                cost,
                casting_method,
                view,
            )
        } else {
            apply_minimum_spell_total_mana_with_view(
                view,
                &apply_payment_reason_mana_adjustments(
                    game,
                    player,
                    Some(spell_id),
                    cost,
                    crate::costs::PaymentReason::CastSpell,
                ),
            )
        };
        ctx.add_cost_adjustment_ms(cost_started_at.elapsed_ms());

        let affordability_started_at = PerfTimer::start();
        let can_pay_adjusted = mana_cost_can_be_paid_by_caster_or_assist_with_view(
            game, player, spell_id, &adjusted, view,
        );
        let can_pay_with_optional_reduction = !can_pay_adjusted
            && effective_cost_with_affordable_optional_cost_hypothesis(
                game,
                player,
                spell_for_checks,
                cost,
                casting_method,
            )
            .is_some_and(|optional_adjusted| {
                mana_cost_can_be_paid_by_caster_or_assist_with_view(
                    game,
                    player,
                    spell_id,
                    &optional_adjusted,
                    view,
                )
            });
        if !can_pay_adjusted && !can_pay_with_optional_reduction {
            ctx.add_affordability_ms(affordability_started_at.elapsed_ms());
            return false;
        }
        ctx.add_affordability_ms(affordability_started_at.elapsed_ms());
    }

    true
}

pub(crate) fn provisional_casting_method_for_alternative(
    spell: &crate::object::Object,
    method: &crate::alternative_cast::AlternativeCastingMethod,
) -> CastingMethod {
    if let Some(index) = spell
        .alternative_casts
        .iter()
        .position(|candidate| candidate == method)
    {
        return CastingMethod::Alternative(index);
    }

    match method {
        crate::alternative_cast::AlternativeCastingMethod::Escape { exile_count, .. } => {
            CastingMethod::GrantedEscape {
                source: spell.id,
                exile_count: *exile_count,
            }
        }
        crate::alternative_cast::AlternativeCastingMethod::Flashback { .. } => {
            CastingMethod::GrantedFlashback
        }
        _ => CastingMethod::Normal,
    }
}

/// Build additional cast requirements from an alternative casting method.
pub(crate) fn build_requirements_for_method(
    method: &crate::alternative_cast::AlternativeCastingMethod,
) -> AdditionalCastRequirements {
    let method_requirements = method.requirements();
    AdditionalCastRequirements {
        exile_from_graveyard: method_requirements.exile_from_graveyard,
        discard_from_hand: method_requirements.discard_from_hand,
        ..Default::default()
    }
}

/// Get the mana cost for an alternative casting method.
pub(crate) fn get_mana_cost_for_method<'a>(
    method: &'a crate::alternative_cast::AlternativeCastingMethod,
    spell: &'a crate::object::Object,
) -> Option<&'a crate::mana::ManaCost> {
    // Composed costs can intentionally represent "without paying its mana cost"
    // by omitting a mana component, so do not fall back to the card's printed cost.
    if method.is_composed_cost()
        || method.total_cost().is_some()
        || matches!(
            method,
            crate::alternative_cast::AlternativeCastingMethod::FromZone { .. }
        )
    {
        return method.mana_cost();
    }

    // Method's cost takes priority, fallback to spell's cost for methods that
    // explicitly say they reuse the spell's normal mana cost.
    method.mana_cost().or(spell.mana_cost.as_deref())
}

pub(crate) fn spell_view_for_disturb_cast(
    game: &GameState,
    spell: &crate::object::Object,
) -> Option<crate::object::Object> {
    let already_overlaid_disturb_spell = matches!(
        spell.cast_alternative_method.as_deref(),
        Some(crate::alternative_cast::AlternativeCastingMethod::Disturb { .. })
    ) && !spell.alternative_casts.iter().any(|method| {
        matches!(
            method,
            crate::alternative_cast::AlternativeCastingMethod::Disturb { .. }
        )
    });
    if already_overlaid_disturb_spell {
        let mut view = spell.clone();
        view.ensure_aura_cast_spell_effect();
        return Some(view);
    }

    let other_def = game
        .linked_face_definition_by_name_or_id(spell.other_face_name.as_deref(), spell.other_face)?;
    let front_colors = spell.colors();
    let mut view = spell.clone();
    view.apply_definition_face(&other_def);
    if view.mana_cost.is_none() && view.color_override.is_none() && !front_colors.is_empty() {
        view.color_override = Some(front_colors);
    }
    view.ensure_aura_cast_spell_effect();
    Some(view)
}

pub(crate) fn spell_view_for_face_down_cast(
    spell: &crate::object::Object,
) -> crate::object::Object {
    let mut view = spell.clone();
    view.apply_face_down_cast_overlay();
    view
}

pub(crate) fn linked_face_definition(
    game: &GameState,
    spell: &crate::object::Object,
) -> Option<crate::cards::CardDefinition> {
    game.linked_face_definition_by_name_or_id(spell.other_face_name.as_deref(), spell.other_face)
}

pub fn linked_other_face_land_definition(
    game: &GameState,
    spell: &crate::object::Object,
) -> Option<crate::cards::CardDefinition> {
    if spell.linked_face_layout != crate::card::LinkedFaceLayout::TransformLike
        || spell.has_card_type(crate::types::CardType::Land)
    {
        return None;
    }

    linked_face_definition(game, spell)
        .filter(|def| def.card.card_types.contains(&crate::types::CardType::Land))
}

pub(crate) fn spell_has_adventure_half(game: &GameState, spell: &crate::object::Object) -> bool {
    linked_face_definition(game, spell).is_some_and(|def| {
        def.card
            .subtypes
            .contains(&crate::types::Subtype::Adventure)
            && (def
                .card
                .card_types
                .contains(&crate::types::CardType::Instant)
                || def
                    .card
                    .card_types
                    .contains(&crate::types::CardType::Sorcery))
    })
}

pub(crate) fn spell_has_castable_linked_other_half(
    game: &GameState,
    spell: &crate::object::Object,
) -> bool {
    if spell.linked_face_layout == crate::card::LinkedFaceLayout::Split
        || spell_has_adventure_half(game, spell)
    {
        return true;
    }

    spell.linked_face_layout == crate::card::LinkedFaceLayout::TransformLike
        && linked_face_definition(game, spell).is_some_and(|def| def.card.mana_cost.is_some())
}

pub(crate) fn spell_view_for_split_other_half_cast(
    game: &GameState,
    spell: &crate::object::Object,
) -> Option<crate::object::Object> {
    if !spell_has_castable_linked_other_half(game, spell) {
        return None;
    }
    let other_def = linked_face_definition(game, spell)?;
    let mut view = spell.clone();
    view.apply_definition_face(&other_def);
    view.ensure_aura_cast_spell_effect();
    Some(view)
}

pub(crate) fn spell_view_for_fused_split_cast(
    game: &GameState,
    spell: &crate::object::Object,
) -> Option<crate::object::Object> {
    if spell.linked_face_layout != crate::card::LinkedFaceLayout::Split || !spell.has_fuse {
        return None;
    }
    let other_def = linked_face_definition(game, spell)?;
    let mut view = spell.clone();
    view.apply_fused_split_spell_overlay(&other_def);
    Some(view)
}

pub(crate) fn can_cast_with_alternative_with_view(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    method: &crate::alternative_cast::AlternativeCastingMethod,
    view: &DerivedGameView<'_>,
) -> bool {
    let ctx = CastLegalityContext::new(game, player, view);
    can_cast_with_alternative_with_context(spell, method, &ctx)
}

pub(crate) fn can_cast_with_alternative_with_context(
    spell: &crate::object::Object,
    method: &crate::alternative_cast::AlternativeCastingMethod,
    ctx: &CastLegalityContext<'_>,
) -> bool {
    use crate::alternative_cast::AlternativeCastingMethod;
    let game = ctx.game;
    let player = ctx.player;

    let disturbed_view = match method {
        AlternativeCastingMethod::Disturb { .. } => {
            match spell_view_for_disturb_cast(game, spell) {
                Some(view) => Some(view),
                None => return false,
            }
        }
        _ => None,
    };
    let base_spell_for_checks = disturbed_view.as_ref().unwrap_or(spell);
    let provisional_alternative_spell = (!base_spell_for_checks
        .alternative_casts
        .iter()
        .any(|candidate| candidate == method))
    .then(|| {
        let mut view = base_spell_for_checks.clone();
        view.cast_alternative_method = Some(Box::new(method.clone()));
        view
    });
    let spell_for_checks = provisional_alternative_spell
        .as_ref()
        .unwrap_or(base_spell_for_checks);
    let effects_override = method
        .overload_effects()
        .or_else(|| method.cleave_effects())
        .or_else(|| method.awaken_effects())
        .or_else(|| {
            disturbed_view
                .as_ref()
                .and_then(|view| view.spell_effect.as_deref())
                .map(|program| &**program)
        });
    let free_plot_cost = crate::mana::ManaCost::new();
    if let Some(condition) = method.cast_condition()
        && !crate::static_abilities::this_spell_cost_condition_is_active_for_cast(
            game,
            spell.id,
            condition,
            &[],
        )
    {
        return false;
    }
    let mana_cost = match method {
        AlternativeCastingMethod::Foretell { .. } => {
            if !game.is_foretold(spell.id) {
                return false;
            }
            get_mana_cost_for_method(method, spell_for_checks)
        }
        AlternativeCastingMethod::Plot { .. } => {
            if !game.is_plotted_by(spell.id, player) {
                return false;
            }
            let Some(plotted_turn) = game.plotted_turn(spell.id) else {
                return false;
            };
            if plotted_turn >= game.turn.turn_number {
                return false;
            }
            if !game.is_active_player(player) || !crate::turn::is_sorcery_timing(game) {
                return false;
            }
            Some(&free_plot_cost)
        }
        AlternativeCastingMethod::Suspend { .. } => return false,
        _ => get_mana_cost_for_method(method, spell_for_checks),
    };
    if mana_cost.is_none() && alternative_method_uses_printed_mana_cost(method) {
        return false;
    }
    let mana_cost = mana_cost.map(|cost| {
        apply_emerge_reduction_to_alternative_mana_cost(game, player, spell.id, method, cost)
    });

    let requirements = build_requirements_for_method(method);
    let casting_method = provisional_casting_method_for_alternative(spell, method);
    if !can_cast_with_cost_with_context(
        spell_for_checks,
        spell.id,
        mana_cost.as_ref(),
        effects_override,
        &requirements,
        &casting_method,
        ctx,
    ) {
        return false;
    }

    if !can_pay_non_mana_cost_sequence_for_cast(game, player, spell.id, method.non_mana_costs()) {
        return false;
    }

    true
}

fn choose_cost_tag(cost: &crate::costs::Cost) -> Option<crate::tag::TagKey> {
    cost.effect_ref()
        .and_then(|effect| effect.downcast_ref::<crate::effects::ChooseObjectsEffect>())
        .map(|choose| choose.tag.clone())
}

fn tagged_dependency_satisfied_by_prior_cost(
    cost: &crate::costs::Cost,
    available_tags: &[crate::tag::TagKey],
) -> bool {
    let Some(effect) = cost.effect_ref() else {
        return false;
    };
    let tagged_constraints = if let Some(sacrifice) =
        effect.downcast_ref::<crate::effects::SacrificeEffect>()
    {
        &sacrifice.filter.tagged_constraints
    } else if let Some(sacrifice) = effect.downcast_ref::<ironsmith_core::SacrificePlayerEffect>() {
        &sacrifice.filter.tagged_constraints
    } else {
        return false;
    };

    !tagged_constraints.is_empty()
        && tagged_constraints.iter().all(|constraint| {
            constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                && available_tags.iter().any(|tag| tag == &constraint.tag)
        })
}

pub(crate) fn can_pay_non_mana_cost_sequence_for_cast(
    game: &GameState,
    player: PlayerId,
    source: ObjectId,
    costs: Vec<crate::costs::Cost>,
) -> bool {
    let check_ctx = crate::costs::CostCheckContext::new(source, player)
        .with_reason(crate::costs::PaymentReason::CastSpell);
    let mut available_tags = Vec::new();

    for cost in costs {
        if game
            .validate_cost_for_payment_reason(player, source, &cost, check_ctx.reason)
            .is_err()
        {
            return false;
        }

        if crate::costs::can_pay_with_check_context(&*cost.0, game, &check_ctx).is_err()
            && !tagged_dependency_satisfied_by_prior_cost(&cost, &available_tags)
        {
            return false;
        }

        if let Some(tag) = choose_cost_tag(&cost)
            && !available_tags.iter().any(|available| available == &tag)
        {
            available_tags.push(tag);
        }
    }

    true
}

/// Check if a spell can be cast with an alternative cost from hand (e.g., Force of Will).
pub fn can_cast_with_alternative_from_hand(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    spell_id: crate::ids::ObjectId,
    method: &crate::alternative_cast::AlternativeCastingMethod,
) -> bool {
    let view = DerivedGameView::new(game);
    can_cast_with_alternative_from_hand_with_view(game, player, spell, spell_id, method, &view)
}

pub(crate) fn can_cast_with_alternative_from_hand_with_view(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    spell_id: crate::ids::ObjectId,
    method: &crate::alternative_cast::AlternativeCastingMethod,
    view: &DerivedGameView<'_>,
) -> bool {
    let ctx = CastLegalityContext::new(game, player, view);
    can_cast_with_alternative_from_hand_with_context(spell, spell_id, method, &ctx)
}

pub(crate) fn can_cast_with_alternative_from_hand_with_context(
    spell: &crate::object::Object,
    spell_id: crate::ids::ObjectId,
    method: &crate::alternative_cast::AlternativeCastingMethod,
    ctx: &CastLegalityContext<'_>,
) -> bool {
    use crate::alternative_cast::AlternativeCastingMethod;
    let game = ctx.game;
    let player = ctx.player;

    match method {
        method if method.is_composed_cost() => {
            let zero_cost = crate::mana::ManaCost::new();
            let casting_method = provisional_casting_method_for_alternative(spell, method);
            let mana_cost = method.mana_cost().or(Some(&zero_cost)).map(|cost| {
                apply_emerge_reduction_to_alternative_mana_cost(
                    game, player, spell_id, method, cost,
                )
            });
            if let Some(condition) = method.cast_condition()
                && !crate::static_abilities::this_spell_cost_condition_is_active_for_cast(
                    game,
                    spell_id,
                    condition,
                    &[],
                )
            {
                return false;
            }

            if !can_cast_with_cost_with_context(
                spell,
                spell_id,
                mana_cost.as_ref(),
                None,
                &AdditionalCastRequirements::default(),
                &casting_method,
                ctx,
            ) {
                return false;
            }

            can_pay_non_mana_cost_sequence_for_cast(game, player, spell_id, method.non_mana_costs())
        }
        AlternativeCastingMethod::Bestow { total_cost } => {
            let Some(cost) = total_cost.mana_cost() else {
                return false;
            };
            let casting_method = provisional_casting_method_for_alternative(spell, method);

            if !can_cast_with_cost_with_context(
                spell,
                spell_id,
                Some(cost),
                None,
                &AdditionalCastRequirements::default(),
                &casting_method,
                ctx,
            ) {
                return false;
            }

            if !can_pay_non_mana_cost_sequence_for_cast(
                game,
                player,
                spell_id,
                method.non_mana_costs(),
            ) {
                return false;
            }

            let bestow_spec = ChooseSpec::Object(crate::target::ObjectFilter::creature());
            let bestow_targets =
                crate::targeting::compute_legal_targets_with_tagged_objects_with_view(
                    game,
                    &bestow_spec,
                    player,
                    Some(spell_id),
                    None,
                    ctx.view,
                );
            !bestow_targets.is_empty()
        }
        AlternativeCastingMethod::Trap {
            cost, condition, ..
        } => {
            // Check if the trap condition is met
            if !is_trap_condition_met(game, player, condition) {
                return false;
            }
            // Check if player can pay the trap cost (usually {0})
            let casting_method = provisional_casting_method_for_alternative(spell, method);
            can_cast_with_cost_with_context(
                spell,
                spell_id,
                Some(cost),
                None,
                &AdditionalCastRequirements::default(),
                &casting_method,
                ctx,
            )
        }
        _ => can_cast_with_alternative_with_context(spell, method, ctx),
    }
}

/// Check if a trap condition is met for the given player.
pub(crate) fn is_trap_condition_met(
    game: &GameState,
    player: PlayerId,
    condition: &crate::alternative_cast::TrapCondition,
) -> bool {
    use crate::alternative_cast::TrapCondition;

    // Get all opponents
    let opponents: Vec<PlayerId> = game
        .players
        .iter()
        .filter(|p| p.id != player && p.is_in_game())
        .map(|p| p.id)
        .collect();

    match condition {
        TrapCondition::OpponentCastSpells { count } => {
            // Check if any opponent cast N or more spells this turn
            opponents
                .iter()
                .any(|&opp| game.turn_store.turn_history.spells_cast_by_player(opp) >= *count)
        }
        TrapCondition::OpponentSearchedLibrary => {
            // Check if any opponent searched their library this turn
            opponents.iter().any(|opp| {
                game.turn_store
                    .turn_history
                    .player_searched_library_this_turn(*opp)
            })
        }
        TrapCondition::OpponentCreatureEntered => {
            // Check if any opponent had a creature enter the battlefield this turn
            opponents.iter().any(|&opp| {
                game.turn_store
                    .turn_history
                    .player_had_creature_enter_battlefield_this_turn(opp)
            })
        }
        TrapCondition::CreatureDealtDamageToYou => {
            // Check if any creature dealt damage to the player this turn
            game.turn_store
                .turn_history
                .player_was_dealt_damage_by_creature_this_turn(player)
        }
    }
}

/// Check if a player can pay a specific cost, excluding a specific card from hand (the spell being cast).
pub(crate) fn can_pay_cost_with_spell_exclusion(
    game: &GameState,
    player: PlayerId,
    cost: &crate::costs::Cost,
    spell_to_exclude: Option<crate::ids::ObjectId>,
) -> bool {
    use crate::costs::CostProcessingMode;

    let source = spell_to_exclude.or_else(|| {
        game.player(player).and_then(|p| {
            p.hand
                .first()
                .copied()
                .or_else(|| p.graveyard.first().copied())
        })
    });
    let Some(source) = source else {
        return false;
    };

    let mut dm = crate::decision::CliDecisionMaker;
    let ctx = crate::costs::CostContext::new(source, player, &mut dm)
        .with_reason(crate::costs::PaymentReason::CastSpell);
    if game
        .validate_cost_for_payment_reason(player, source, cost, ctx.reason)
        .is_err()
    {
        return false;
    }

    match cost.processing_mode() {
        CostProcessingMode::ManaPayment { .. } => cost.can_potentially_pay(game, &ctx).is_ok(),
        CostProcessingMode::Immediate
        | CostProcessingMode::InlineWithTriggers
        | CostProcessingMode::SacrificeTarget { .. }
        | CostProcessingMode::DiscardCards { .. }
        | CostProcessingMode::ExileFromHand { .. }
        | CostProcessingMode::ExileFromGraveyard { .. }
        | CostProcessingMode::ExileObjects { .. }
        | CostProcessingMode::RevealFromHand { .. }
        | CostProcessingMode::ReturnToHandTarget { .. } => cost.can_pay(game, &ctx).is_ok(),
    }
}

pub(crate) fn apply_payment_reason_mana_adjustments(
    game: &GameState,
    payer: PlayerId,
    source: Option<ObjectId>,
    cost: &crate::mana::ManaCost,
    reason: crate::costs::PaymentReason,
) -> crate::mana::ManaCost {
    game.adjust_mana_cost_for_payment_reason(payer, source, cost, reason)
}

fn apply_minimum_spell_total_mana_with_view(
    view: &DerivedGameView<'_>,
    cost: &crate::mana::ManaCost,
) -> crate::mana::ManaCost {
    if let Some(minimum) = view.minimum_total_spell_mana_payment()
        && cost.mana_value() < minimum
    {
        return cost.add_generic(minimum - cost.mana_value());
    }

    cost.clone()
}

// ============================================================================
// Cost Modifier Helpers (Tier 9)
// ============================================================================

/// Calculate the effective mana cost after applying cost reduction abilities.
///
/// This handles abilities like:
/// - Affinity for artifacts: Reduce generic cost by 1 for each artifact you control
/// - Delve: Preview the maximum available generic reduction for action discovery
/// - Convoke: Tap creatures to pay for mana (colored or generic)
///
/// Returns the reduced mana cost.
pub fn calculate_effective_mana_cost(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    base_cost: &crate::mana::ManaCost,
) -> crate::mana::ManaCost {
    calculate_effective_mana_cost_for_casting_method(
        game,
        player,
        spell,
        base_cost,
        &CastingMethod::Normal,
    )
}

pub fn calculate_effective_mana_cost_for_casting_method(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    base_cost: &crate::mana::ManaCost,
    casting_method: &CastingMethod,
) -> crate::mana::ManaCost {
    let view = DerivedGameView::new(game);
    calculate_effective_mana_cost_with_targets_internal(
        game,
        player,
        spell,
        base_cost,
        1,
        &[],
        true,
        casting_method,
        None,
        &view,
    )
}

pub(crate) fn calculate_effective_mana_cost_with_view_for_casting_method(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    base_cost: &crate::mana::ManaCost,
    casting_method: &CastingMethod,
    view: &DerivedGameView<'_>,
) -> crate::mana::ManaCost {
    calculate_effective_mana_cost_with_targets_internal(
        game,
        player,
        spell,
        base_cost,
        1,
        &[],
        true,
        casting_method,
        None,
        view,
    )
}

/// Calculate the effective mana cost with explicit chosen target count.
pub fn calculate_effective_mana_cost_with_targets(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    base_cost: &crate::mana::ManaCost,
    chosen_target_count: usize,
) -> crate::mana::ManaCost {
    let view = DerivedGameView::new(game);
    calculate_effective_mana_cost_with_targets_internal(
        game,
        player,
        spell,
        base_cost,
        chosen_target_count,
        &[],
        true,
        &CastingMethod::Normal,
        None,
        &view,
    )
}

/// Calculate the effective mana cost using the exact chosen targets.
pub fn calculate_effective_mana_cost_with_chosen_targets(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    base_cost: &crate::mana::ManaCost,
    chosen_targets: &[Target],
) -> crate::mana::ManaCost {
    calculate_effective_mana_cost_with_chosen_targets_for_casting_method(
        game,
        player,
        spell,
        base_cost,
        chosen_targets,
        &CastingMethod::Normal,
    )
}

pub fn calculate_effective_mana_cost_with_chosen_targets_for_casting_method(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    base_cost: &crate::mana::ManaCost,
    chosen_targets: &[Target],
    casting_method: &CastingMethod,
) -> crate::mana::ManaCost {
    let view = DerivedGameView::new(game);
    calculate_effective_mana_cost_with_targets_internal(
        game,
        player,
        spell,
        base_cost,
        chosen_targets.len(),
        chosen_targets,
        true,
        casting_method,
        None,
        &view,
    )
}

pub(crate) fn calculate_effective_mana_cost_with_chosen_targets_for_casting_method_from_zone(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    base_cost: &crate::mana::ManaCost,
    chosen_targets: &[Target],
    casting_method: &CastingMethod,
    cast_from_zone: Zone,
) -> crate::mana::ManaCost {
    let view = DerivedGameView::new(game);
    calculate_effective_mana_cost_with_targets_internal(
        game,
        player,
        spell,
        base_cost,
        chosen_targets.len(),
        chosen_targets,
        true,
        casting_method,
        Some(cast_from_zone),
        &view,
    )
}

/// Calculate effective cost for payment stage where Convoke/Improvise are handled
/// as pip alternatives instead of up-front reductions.
pub fn calculate_effective_mana_cost_for_payment_with_targets(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    base_cost: &crate::mana::ManaCost,
    chosen_target_count: usize,
) -> crate::mana::ManaCost {
    calculate_effective_mana_cost_for_payment_with_targets_for_casting_method(
        game,
        player,
        spell,
        base_cost,
        chosen_target_count,
        &CastingMethod::Normal,
    )
}

pub fn calculate_effective_mana_cost_for_payment_with_targets_for_casting_method(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    base_cost: &crate::mana::ManaCost,
    chosen_target_count: usize,
    casting_method: &CastingMethod,
) -> crate::mana::ManaCost {
    let view = DerivedGameView::new(game);
    calculate_effective_mana_cost_with_targets_internal(
        game,
        player,
        spell,
        base_cost,
        chosen_target_count,
        &[],
        false,
        casting_method,
        None,
        &view,
    )
}

/// Calculate payment-stage effective cost using exact chosen targets.
pub fn calculate_effective_mana_cost_for_payment_with_chosen_targets(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    base_cost: &crate::mana::ManaCost,
    chosen_targets: &[Target],
) -> crate::mana::ManaCost {
    calculate_effective_mana_cost_for_payment_with_chosen_targets_for_casting_method(
        game,
        player,
        spell,
        base_cost,
        chosen_targets,
        &CastingMethod::Normal,
    )
}

pub fn calculate_effective_mana_cost_for_payment_with_chosen_targets_for_casting_method(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    base_cost: &crate::mana::ManaCost,
    chosen_targets: &[Target],
    casting_method: &CastingMethod,
) -> crate::mana::ManaCost {
    let view = DerivedGameView::new(game);
    calculate_effective_mana_cost_with_targets_internal(
        game,
        player,
        spell,
        base_cost,
        chosen_targets.len(),
        chosen_targets,
        false,
        casting_method,
        None,
        &view,
    )
}

pub(crate) fn calculate_effective_mana_cost_for_payment_with_chosen_targets_for_casting_method_from_zone(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    base_cost: &crate::mana::ManaCost,
    chosen_targets: &[Target],
    casting_method: &CastingMethod,
    cast_from_zone: Zone,
) -> crate::mana::ManaCost {
    let view = DerivedGameView::new(game);
    calculate_effective_mana_cost_with_targets_internal(
        game,
        player,
        spell,
        base_cost,
        chosen_targets.len(),
        chosen_targets,
        false,
        casting_method,
        Some(cast_from_zone),
        &view,
    )
}

pub(crate) fn calculate_effective_mana_cost_with_targets_internal(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    base_cost: &crate::mana::ManaCost,
    chosen_target_count: usize,
    chosen_targets: &[Target],
    include_convoke_improvise_reductions: bool,
    casting_method: &CastingMethod,
    cast_from_zone: Option<Zone>,
    view: &DerivedGameView<'_>,
) -> crate::mana::ManaCost {
    let mut current_cost = base_cost.clone();

    // Check for Affinity for artifacts
    if has_affinity_for_artifacts(spell) {
        // Count artifacts controlled by the player
        let artifact_count = count_artifacts_controlled_with_view(game, player, view);
        current_cost = current_cost.reduce_generic(artifact_count);
    }

    // Apply explicit cost reductions/increases on the spell itself.
    current_cost = apply_spell_cost_modifiers(
        game,
        player,
        spell,
        &current_cost,
        chosen_target_count,
        chosen_targets,
        casting_method,
        cast_from_zone,
    );

    // Apply global cost modifiers from battlefield permanents (Sphere of Resistance, leeches, etc.).
    current_cost = apply_battlefield_spell_cost_modifiers(
        game,
        player,
        spell,
        &current_cost,
        chosen_target_count,
        chosen_targets,
        casting_method,
        cast_from_zone,
        view,
    );

    // Action discovery previews maximum Delve usage. During the payment-stage
    // calculation (`include_convoke_improvise_reductions == false`), Delve is
    // instead an interactive repeatable payment in the CR 601 transaction.
    if include_convoke_improvise_reductions && has_delve(spell) {
        let graveyard_count = game
            .player(player)
            .map(|player| {
                player
                    .graveyard
                    .iter()
                    .filter(|&&card_id| card_id != spell.id)
                    .count() as u32
            })
            .unwrap_or(0);
        current_cost = current_cost.reduce_generic(graveyard_count);
    }

    if include_convoke_improvise_reductions {
        // Check for Convoke
        let has_convoke_ability = has_convoke(spell);
        if has_convoke_ability {
            // For Convoke, calculate the optimal creature tapping
            let (_, convoked_cost) = calculate_convoke_cost(game, player, &current_cost);
            current_cost = convoked_cost;
        }

        // Check for Improvise
        let has_improvise_ability = has_improvise(spell);
        if has_improvise_ability {
            // For Improvise, calculate the optimal artifact tapping
            let (_, improvised_cost) = calculate_improvise_cost(game, player, &current_cost);
            current_cost = improvised_cost;
        }
    }

    let current_cost = apply_payment_reason_mana_adjustments(
        game,
        player,
        Some(spell.id),
        &current_cost,
        crate::costs::PaymentReason::CastSpell,
    );

    apply_minimum_spell_total_mana_with_view(view, &current_cost)
}

fn chosen_targets_match_cost_filter(
    game: &GameState,
    filter: &crate::target::ObjectFilter,
    ctx: &crate::filter::FilterContext,
    chosen_targets: &[Target],
) -> bool {
    if let Some(count) = filter.target_count
        && (chosen_targets.len() < count.min
            || count
                .max
                .is_some_and(|maximum| chosen_targets.len() > maximum))
    {
        return false;
    }

    let requires_witnessed_target = filter.targets_player.is_some()
        || filter.targets_object.is_some()
        || filter.targets_only_player.is_some()
        || filter.targets_only_object.is_some();
    if !requires_witnessed_target {
        return true;
    }
    if chosen_targets.is_empty() {
        return false;
    }

    let matches_player = filter.targets_player.as_ref().is_none_or(|player_filter| {
        chosen_targets.iter().any(|target| match target {
            Target::Player(player) => player_filter.matches_player(*player, ctx),
            Target::Object(_) => false,
        })
    });
    let matches_object = filter.targets_object.as_ref().is_none_or(|object_filter| {
        chosen_targets.iter().any(|target| match target {
            Target::Object(object) => game
                .object(*object)
                .is_some_and(|object| object_filter.matches(object, ctx, game)),
            Target::Player(_) => false,
        })
    });
    if filter.targets_player.is_some() || filter.targets_object.is_some() {
        let matches = if filter.targets_any_of
            && filter.targets_player.is_some()
            && filter.targets_object.is_some()
        {
            matches_player || matches_object
        } else {
            matches_player && matches_object
        };
        if !matches {
            return false;
        }
    }

    if filter.targets_only_player.is_some() || filter.targets_only_object.is_some() {
        let every_target_matches = chosen_targets.iter().all(|target| {
            let matches_player = filter
                .targets_only_player
                .as_ref()
                .is_some_and(|player_filter| match target {
                    Target::Player(player) => player_filter.matches_player(*player, ctx),
                    Target::Object(_) => false,
                });
            let matches_object = filter
                .targets_only_object
                .as_ref()
                .is_some_and(|object_filter| match target {
                    Target::Object(object) => game
                        .object(*object)
                        .is_some_and(|object| object_filter.matches(object, ctx, game)),
                    Target::Player(_) => false,
                });
            if filter.targets_only_player.is_some() && filter.targets_only_object.is_some() {
                matches_player || matches_object
            } else if filter.targets_only_player.is_some() {
                matches_player
            } else {
                matches_object
            }
        });
        if !every_target_matches {
            return false;
        }
    }

    true
}

fn cost_modifier_target_repetitions(per_target: bool, chosen_target_count: usize) -> usize {
    if per_target { chosen_target_count } else { 1 }
}

pub(crate) fn apply_spell_cost_modifiers(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    cost: &crate::mana::ManaCost,
    chosen_target_count: usize,
    chosen_targets: &[Target],
    casting_method: &CastingMethod,
    cast_from_zone: Option<Zone>,
) -> crate::mana::ManaCost {
    use crate::ability::AbilityKind;
    use crate::filter::FilterContext;
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::target::ObjectFilter;

    fn opponents_of(game: &GameState, player: PlayerId) -> Vec<PlayerId> {
        game.turn_store
            .turn_order
            .iter()
            .copied()
            .filter(|p| *p != player)
            .collect()
    }

    fn spell_matches_filter(
        game: &GameState,
        spell: &crate::object::Object,
        caster: PlayerId,
        filter: &ObjectFilter,
        ctx: &FilterContext,
        casting_method: &CastingMethod,
        cast_from_zone: Option<Zone>,
        chosen_targets: &[Target],
    ) -> bool {
        let targets_match = chosen_targets_match_cost_filter(game, filter, ctx, chosen_targets);
        let mut cast_filter = filter.clone();
        let alternative_cast = cast_filter.alternative_cast;
        cast_filter.targets_player = None;
        cast_filter.targets_object = None;
        cast_filter.targets_any_of = false;
        cast_filter.targets_only_player = None;
        cast_filter.targets_only_object = None;
        cast_filter.targets_only_any_of = false;
        cast_filter.target_count = None;
        cast_filter.alternative_cast = None;
        let overlaid_spell =
            spell_view_for_cost_filter_match(game, caster, spell, casting_method, cast_from_zone);
        let spell_for_match = overlaid_spell.as_ref().unwrap_or(spell);
        let matches =
            cast_filter.matches_non_recursive(
                spell_for_match,
                &ctx.clone().with_caster(Some(caster)),
                game,
            ) || disturb_linked_face_matches_cost_filter(game, caster, spell, &cast_filter, ctx);
        targets_match
            && matches
            && alternative_cast.is_none_or(|kind| {
                casting_method_matches_alternative_kind(game, caster, spell, casting_method, kind)
            })
    }

    fn optional_life_reduction_was_paid(
        spell: &crate::object::Object,
        reduction: &crate::static_abilities::CostReductionManaCost,
        source: ObjectId,
    ) -> bool {
        match &reduction.optional_life_additional_cost {
            Some(optional) => spell
                .optional_costs_paid
                .was_paid_label(&optional_life_cost_reduction_label(optional, source)),
            None => true,
        }
    }

    let mut total_increase: i32 = 0;
    let mut total_reduction: i32 = 0;
    let mut increase_pips: Vec<Vec<ManaSymbol>> = Vec::new();
    let mut reduction_pips: Vec<Vec<ManaSymbol>> = Vec::new();
    if let CastingMethod::PlayFrom { source, zone, .. }
    | CastingMethod::SplitOtherHalfPlayFrom { source, zone, .. } = casting_method
    {
        let constraints = game
            .effect_store
            .grant_registry
            .play_from_constraints_for_card(game, spell.id, *zone, player, *source);
        if let Some(increase) = constraints.spell_cost_increase {
            increase_pips.extend(increase.pips().iter().cloned());
        }
    }
    let ctx = with_source_exiled_tagged_objects(
        game,
        FilterContext::new(player)
            .with_source(spell.id)
            .with_active_player(game.turn.active_player)
            .with_opponents(opponents_of(game, player)),
        spell.id,
    );

    for ability in spell.abilities.iter() {
        let AbilityKind::Static(static_ability) = &ability.kind else {
            continue;
        };
        let functions_in_current_zone = ability.functions_in(&spell.zone);
        if let Some(reduction) = static_ability.this_spell_cost_reduction() {
            let casting_method_matches = reduction.alternative_cast.is_none_or(|kind| {
                casting_method_matches_alternative_kind(game, player, spell, casting_method, kind)
            });
            if casting_method_matches
                && crate::static_abilities::this_spell_cost_condition_is_active_for_cast_with_optional_costs_paid(
                    game,
                    spell.id,
                    &reduction.condition,
                    chosen_targets,
                    Some(&spell.optional_costs_paid),
                )
            {
                let amount =
                    resolve_this_spell_cost_reduction_value(game, player, spell, reduction);
                if amount > 0 {
                    total_reduction = total_reduction.saturating_add(amount);
                }
            }
        }
        if let Some(reduction) = static_ability.this_spell_cost_reduction_mana_cost() {
            if crate::static_abilities::this_spell_cost_condition_is_active_for_cast_with_optional_costs_paid(
                game,
                spell.id,
                &reduction.condition,
                chosen_targets,
                Some(&spell.optional_costs_paid),
            ) {
                let repetitions = reduction
                    .repetitions
                    .as_ref()
                    .map(|value| resolve_cost_modifier_value(game, player, spell, value).max(0))
                    .unwrap_or(1);
                for _ in 0..repetitions {
                    reduction_pips.extend(reduction.reduction.pips().iter().cloned());
                }
            }
        }
        if !functions_in_current_zone {
            continue;
        }
        if !static_ability.is_active(game, spell.id) {
            continue;
        }
        if let Some(reduction) = static_ability.cost_reduction()
            && spell_matches_filter(
                game,
                spell,
                player,
                &reduction.filter,
                &ctx,
                casting_method,
                cast_from_zone,
                chosen_targets,
            )
        {
            let multiplier = if reduction.per_target {
                i32::try_from(chosen_target_count).unwrap_or(i32::MAX)
            } else {
                1
            };
            let amount = resolve_cost_reduction_amount(game, spell, spell.id, player, reduction)
                .saturating_mul(multiplier);
            if amount > 0 {
                total_reduction = total_reduction.saturating_add(amount);
            }
        }
        if let Some(increase) = static_ability.cost_increase()
            && spell_matches_filter(
                game,
                spell,
                player,
                &increase.filter,
                &ctx,
                casting_method,
                cast_from_zone,
                chosen_targets,
            )
        {
            let multiplier = if increase.per_target {
                i32::try_from(chosen_target_count).unwrap_or(i32::MAX)
            } else {
                1
            };
            let amount = resolve_cost_modifier_value(game, player, spell, &increase.increase)
                .saturating_mul(multiplier);
            if amount > 0 {
                total_increase = total_increase.saturating_add(amount);
            }
        }
        if let Some(increase) = static_ability.cost_increase_mana_cost()
            && spell_matches_filter(
                game,
                spell,
                player,
                &increase.filter,
                &ctx,
                casting_method,
                cast_from_zone,
                chosen_targets,
            )
        {
            for _ in 0..cost_modifier_target_repetitions(increase.per_target, chosen_target_count) {
                increase_pips.extend(increase.increase.pips().iter().cloned());
            }
        }
        if let Some(reduction) = static_ability.cost_reduction_mana_cost()
            && spell_matches_filter(
                game,
                spell,
                player,
                &reduction.filter,
                &ctx,
                casting_method,
                cast_from_zone,
                chosen_targets,
            )
            && optional_life_reduction_was_paid(spell, reduction, spell.id)
        {
            for _ in 0..cost_modifier_target_repetitions(reduction.per_target, chosen_target_count)
            {
                reduction_pips.extend(reduction.reduction.pips().iter().cloned());
            }
        }
        if let Some(per_target_amount) = static_ability.cost_increase_per_additional_target() {
            let additional_targets = chosen_target_count.saturating_sub(1);
            if additional_targets > 0 {
                let extra = (per_target_amount as i32).saturating_mul(additional_targets as i32);
                total_increase = total_increase.saturating_add(extra);
            }
        }
        if let Some(per_target_cost) =
            static_ability.cost_increase_mana_cost_per_additional_target()
        {
            let additional_targets = chosen_target_count.saturating_sub(1);
            for _ in 0..additional_targets {
                increase_pips.extend(per_target_cost.pips().iter().cloned());
            }
        }
    }

    for effect in &game.effect_store.temporary_spell_cost_reductions {
        if effect.player != player || effect.is_expired(game) {
            continue;
        }
        let temporary_ctx = with_source_exiled_tagged_objects(
            game,
            FilterContext::new(player)
                .with_source(effect.source)
                .with_active_player(game.turn.active_player)
                .with_opponents(opponents_of(game, player)),
            effect.source,
        );
        if spell_matches_filter(
            game,
            spell,
            player,
            &effect.filter,
            &temporary_ctx,
            casting_method,
            cast_from_zone,
            chosen_targets,
        ) {
            if let Some(generic_reduction) = &effect.generic_reduction {
                let amount = resolve_cost_modifier_value(game, player, spell, generic_reduction);
                if amount > 0 {
                    total_reduction = total_reduction.saturating_add(amount);
                }
            }
            reduction_pips.extend(effect.reduction.pips().iter().cloned());
        }
    }

    let mut adjusted = cost.clone();
    if !increase_pips.is_empty() {
        adjusted = add_mana_cost(&adjusted, &ManaCost::from_pips(increase_pips));
    }
    if total_increase > 0 {
        adjusted = add_generic_mana_cost(&adjusted, total_increase as u32);
    }
    if total_reduction > 0 {
        adjusted = adjusted.reduce_generic(total_reduction as u32);
    }
    if !reduction_pips.is_empty() {
        adjusted = reduce_mana_cost(&adjusted, &ManaCost::from_pips(reduction_pips));
    }
    adjusted
}

pub(crate) fn apply_battlefield_spell_cost_modifiers(
    game: &GameState,
    caster: PlayerId,
    spell: &crate::object::Object,
    cost: &crate::mana::ManaCost,
    chosen_target_count: usize,
    chosen_targets: &[Target],
    casting_method: &CastingMethod,
    cast_from_zone: Option<Zone>,
    view: &DerivedGameView<'_>,
) -> crate::mana::ManaCost {
    use crate::ability::AbilityKind;
    use crate::filter::FilterContext;
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::target::ObjectFilter;

    fn opponents_of(game: &GameState, player: PlayerId) -> Vec<PlayerId> {
        game.turn_store
            .turn_order
            .iter()
            .copied()
            .filter(|p| *p != player)
            .collect()
    }

    fn spell_matches_filter(
        game: &GameState,
        spell: &crate::object::Object,
        caster: PlayerId,
        filter: &ObjectFilter,
        ctx: &FilterContext,
        casting_method: &CastingMethod,
        cast_from_zone: Option<Zone>,
        chosen_targets: &[Target],
    ) -> bool {
        let targets_match = chosen_targets_match_cost_filter(game, filter, ctx, chosen_targets);
        let mut cast_filter = filter.clone();
        let alternative_cast = cast_filter.alternative_cast;
        cast_filter.targets_player = None;
        cast_filter.targets_object = None;
        cast_filter.targets_any_of = false;
        cast_filter.targets_only_player = None;
        cast_filter.targets_only_object = None;
        cast_filter.targets_only_any_of = false;
        cast_filter.target_count = None;
        cast_filter.alternative_cast = None;
        let overlaid_spell =
            spell_view_for_cost_filter_match(game, caster, spell, casting_method, cast_from_zone);
        let spell_for_match = overlaid_spell.as_ref().unwrap_or(spell);
        let matches =
            cast_filter.matches_non_recursive(
                spell_for_match,
                &ctx.clone().with_caster(Some(caster)),
                game,
            ) || disturb_linked_face_matches_cost_filter(game, caster, spell, &cast_filter, ctx);
        targets_match
            && matches
            && alternative_cast.is_none_or(|kind| {
                casting_method_matches_alternative_kind(game, caster, spell, casting_method, kind)
            })
    }

    fn optional_life_reduction_was_paid(
        spell: &crate::object::Object,
        reduction: &crate::static_abilities::CostReductionManaCost,
        source: ObjectId,
    ) -> bool {
        match &reduction.optional_life_additional_cost {
            Some(optional) => spell
                .optional_costs_paid
                .was_paid_label(&optional_life_cost_reduction_label(optional, source)),
            None => true,
        }
    }

    let mut total_increase: i32 = 0;
    let mut total_reduction: i32 = 0;
    let mut increase_pips: Vec<Vec<ManaSymbol>> = Vec::new();
    let mut reduction_pips: Vec<Vec<ManaSymbol>> = Vec::new();

    for perm_id in view.battlefield_spell_cost_modifier_sources() {
        let Some(perm) = game.object(perm_id) else {
            continue;
        };
        let controller = game.controller_of(perm);
        let ctx = with_source_exiled_tagged_objects(
            game,
            FilterContext::new(controller)
                .with_source(perm_id)
                .with_active_player(game.turn.active_player)
                .with_opponents(opponents_of(game, controller)),
            perm_id,
        );

        if let Some(static_abilities) = view.static_abilities_rc(perm_id) {
            for static_ability in static_abilities.iter() {
                if !static_ability.is_active(game, perm_id) {
                    continue;
                }
                if let Some(reduction) = static_ability.cost_reduction()
                    && spell_matches_filter(
                        game,
                        spell,
                        caster,
                        &reduction.filter,
                        &ctx,
                        casting_method,
                        cast_from_zone,
                        chosen_targets,
                    )
                {
                    let multiplier = if reduction.per_target {
                        i32::try_from(chosen_target_count).unwrap_or(i32::MAX)
                    } else {
                        1
                    };
                    let amount =
                        resolve_cost_reduction_amount(game, spell, perm_id, controller, reduction)
                            .saturating_mul(multiplier);
                    if amount > 0 {
                        total_reduction = total_reduction.saturating_add(amount);
                    }
                }
                if let Some(increase) = static_ability.cost_increase()
                    && spell_matches_filter(
                        game,
                        spell,
                        caster,
                        &increase.filter,
                        &ctx,
                        casting_method,
                        cast_from_zone,
                        chosen_targets,
                    )
                {
                    let multiplier = if increase.per_target {
                        i32::try_from(chosen_target_count).unwrap_or(i32::MAX)
                    } else {
                        1
                    };
                    let amount = resolve_cost_modifier_value_for_source(
                        game,
                        perm_id,
                        controller,
                        &increase.increase,
                    )
                    .saturating_mul(multiplier);
                    if amount > 0 {
                        total_increase = total_increase.saturating_add(amount);
                    }
                }
                if let Some(increase) = static_ability.cost_increase_mana_cost()
                    && spell_matches_filter(
                        game,
                        spell,
                        caster,
                        &increase.filter,
                        &ctx,
                        casting_method,
                        cast_from_zone,
                        chosen_targets,
                    )
                {
                    for _ in 0..cost_modifier_target_repetitions(
                        increase.per_target,
                        chosen_target_count,
                    ) {
                        increase_pips.extend(increase.increase.pips().iter().cloned());
                    }
                }
                if let Some(reduction) = static_ability.cost_reduction_mana_cost()
                    && spell_matches_filter(
                        game,
                        spell,
                        caster,
                        &reduction.filter,
                        &ctx,
                        casting_method,
                        cast_from_zone,
                        chosen_targets,
                    )
                    && optional_life_reduction_was_paid(spell, reduction, perm_id)
                {
                    for _ in 0..cost_modifier_target_repetitions(
                        reduction.per_target,
                        chosen_target_count,
                    ) {
                        reduction_pips.extend(reduction.reduction.pips().iter().cloned());
                    }
                }
                if let Some(per_target_amount) =
                    static_ability.cost_increase_per_additional_target()
                {
                    let additional_targets = chosen_target_count.saturating_sub(1);
                    if additional_targets > 0 {
                        let extra =
                            (per_target_amount as i32).saturating_mul(additional_targets as i32);
                        total_increase = total_increase.saturating_add(extra);
                    }
                }
                if let Some(per_target_cost) =
                    static_ability.cost_increase_mana_cost_per_additional_target()
                {
                    let additional_targets = chosen_target_count.saturating_sub(1);
                    for _ in 0..additional_targets {
                        increase_pips.extend(per_target_cost.pips().iter().cloned());
                    }
                }
            }
        } else {
            for static_ability in perm
                .abilities
                .iter()
                .filter_map(|ability| match &ability.kind {
                    AbilityKind::Static(static_ability) => Some(static_ability),
                    _ => None,
                })
            {
                if !static_ability.is_active(game, perm_id) {
                    continue;
                }
                if let Some(reduction) = static_ability.cost_reduction()
                    && spell_matches_filter(
                        game,
                        spell,
                        caster,
                        &reduction.filter,
                        &ctx,
                        casting_method,
                        cast_from_zone,
                        chosen_targets,
                    )
                {
                    let multiplier = if reduction.per_target {
                        i32::try_from(chosen_target_count).unwrap_or(i32::MAX)
                    } else {
                        1
                    };
                    let amount =
                        resolve_cost_reduction_amount(game, spell, perm_id, controller, reduction)
                            .saturating_mul(multiplier);
                    if amount > 0 {
                        total_reduction = total_reduction.saturating_add(amount);
                    }
                }
                if let Some(increase) = static_ability.cost_increase()
                    && spell_matches_filter(
                        game,
                        spell,
                        caster,
                        &increase.filter,
                        &ctx,
                        casting_method,
                        cast_from_zone,
                        chosen_targets,
                    )
                {
                    let multiplier = if increase.per_target {
                        i32::try_from(chosen_target_count).unwrap_or(i32::MAX)
                    } else {
                        1
                    };
                    let amount = resolve_cost_modifier_value_for_source(
                        game,
                        perm_id,
                        controller,
                        &increase.increase,
                    )
                    .saturating_mul(multiplier);
                    if amount > 0 {
                        total_increase = total_increase.saturating_add(amount);
                    }
                }
                if let Some(increase) = static_ability.cost_increase_mana_cost()
                    && spell_matches_filter(
                        game,
                        spell,
                        caster,
                        &increase.filter,
                        &ctx,
                        casting_method,
                        cast_from_zone,
                        chosen_targets,
                    )
                {
                    for _ in 0..cost_modifier_target_repetitions(
                        increase.per_target,
                        chosen_target_count,
                    ) {
                        increase_pips.extend(increase.increase.pips().iter().cloned());
                    }
                }
                if let Some(reduction) = static_ability.cost_reduction_mana_cost()
                    && spell_matches_filter(
                        game,
                        spell,
                        caster,
                        &reduction.filter,
                        &ctx,
                        casting_method,
                        cast_from_zone,
                        chosen_targets,
                    )
                    && optional_life_reduction_was_paid(spell, reduction, perm_id)
                {
                    for _ in 0..cost_modifier_target_repetitions(
                        reduction.per_target,
                        chosen_target_count,
                    ) {
                        reduction_pips.extend(reduction.reduction.pips().iter().cloned());
                    }
                }
                if let Some(per_target_amount) =
                    static_ability.cost_increase_per_additional_target()
                {
                    let additional_targets = chosen_target_count.saturating_sub(1);
                    if additional_targets > 0 {
                        let extra =
                            (per_target_amount as i32).saturating_mul(additional_targets as i32);
                        total_increase = total_increase.saturating_add(extra);
                    }
                }
                if let Some(per_target_cost) =
                    static_ability.cost_increase_mana_cost_per_additional_target()
                {
                    let additional_targets = chosen_target_count.saturating_sub(1);
                    for _ in 0..additional_targets {
                        increase_pips.extend(per_target_cost.pips().iter().cloned());
                    }
                }
            }
        }
    }

    let mut adjusted = cost.clone();
    if !increase_pips.is_empty() {
        adjusted = add_mana_cost(&adjusted, &ManaCost::from_pips(increase_pips));
    }
    if total_increase > 0 {
        adjusted = add_generic_mana_cost(&adjusted, total_increase as u32);
    }
    if total_reduction > 0 {
        adjusted = adjusted.reduce_generic(total_reduction as u32);
    }
    if !reduction_pips.is_empty() {
        adjusted = reduce_mana_cost(&adjusted, &ManaCost::from_pips(reduction_pips));
    }
    adjusted
}

pub(crate) fn resolve_this_spell_cost_reduction_value(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    reduction: &crate::static_abilities::ThisSpellCostReduction,
) -> i32 {
    if matches!(
        (&reduction.condition, &reduction.reduction),
        (
            crate::static_abilities::ThisSpellCostCondition::LifeTotalLessThanStarting,
            crate::effect::Value::X
        )
    ) {
        if let Some(player_state) = game.player(player) {
            return player_state
                .starting_life
                .saturating_sub(player_state.life)
                .max(0);
        }
    }

    resolve_cost_modifier_value(game, player, spell, &reduction.reduction)
}

pub(crate) fn add_generic_mana_cost(
    cost: &crate::mana::ManaCost,
    increase: u32,
) -> crate::mana::ManaCost {
    if increase == 0 {
        return cost.clone();
    }
    use crate::mana::ManaSymbol;

    let mut new_pips = cost.pips().to_vec();
    let mut remaining = increase;
    while remaining > 0 {
        let chunk = remaining.min(u8::MAX as u32) as u8;
        new_pips.push(vec![ManaSymbol::Generic(chunk)]);
        remaining -= chunk as u32;
    }

    crate::mana::ManaCost::from_pips(coalesce_plain_generic_pips(new_pips))
}

pub(crate) fn add_mana_cost(
    cost: &crate::mana::ManaCost,
    add: &crate::mana::ManaCost,
) -> crate::mana::ManaCost {
    if add.pips().is_empty() {
        return cost.clone();
    }
    let mut new_pips = cost.pips().to_vec();
    new_pips.extend(add.pips().iter().cloned());
    crate::mana::ManaCost::from_pips(coalesce_plain_generic_pips(new_pips))
}

fn coalesce_plain_generic_pips(
    pips: Vec<Vec<crate::mana::ManaSymbol>>,
) -> Vec<Vec<crate::mana::ManaSymbol>> {
    use crate::mana::ManaSymbol;

    let generic_total = pips
        .iter()
        .filter_map(|pip| match pip.as_slice() {
            [ManaSymbol::Generic(amount)] => Some(*amount as u32),
            _ => None,
        })
        .fold(0u32, u32::saturating_add);
    if generic_total == 0 {
        return pips;
    }

    let mut non_generic = pips
        .into_iter()
        .filter(|pip| !matches!(pip.as_slice(), [ManaSymbol::Generic(_)]))
        .collect::<Vec<_>>();
    // Canonical mana-cost order keeps leading X symbols ahead of the generic
    // numeral and the remaining colored/hybrid pips after it.
    let insert_at = non_generic
        .iter()
        .take_while(|pip| matches!(pip.as_slice(), [ManaSymbol::X]))
        .count();
    let mut generic_pips = Vec::new();
    let mut remaining = generic_total;
    while remaining > 0 {
        let chunk = remaining.min(u8::MAX as u32) as u8;
        generic_pips.push(vec![ManaSymbol::Generic(chunk)]);
        remaining -= chunk as u32;
    }
    non_generic.splice(insert_at..insert_at, generic_pips);
    non_generic
}

pub(crate) fn reduce_mana_cost(
    cost: &crate::mana::ManaCost,
    reduction: &crate::mana::ManaCost,
) -> crate::mana::ManaCost {
    use crate::mana::ManaSymbol;

    if reduction.pips().is_empty() {
        return cost.clone();
    }
    let mut pips = cost.pips().to_vec();
    let mut generic_reduction: u32 = 0;
    for red_pip in reduction.pips() {
        if red_pip.len() == 1
            && let ManaSymbol::Generic(amount) = red_pip[0]
        {
            generic_reduction = generic_reduction.saturating_add(amount as u32);
            continue;
        }
        if let Some(pos) = pips.iter().position(|pip| pip == red_pip) {
            pips.remove(pos);
        }
    }
    let reduced = crate::mana::ManaCost::from_pips(pips);
    if generic_reduction > 0 {
        reduced.reduce_generic(generic_reduction)
    } else {
        reduced
    }
}

pub(crate) fn resolve_cost_modifier_value(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    value: &crate::effect::Value,
) -> i32 {
    let mut dm = SelectFirstDecisionMaker;
    let ctx = ExecutionContext::new(spell.id, player, &mut dm);
    resolve_value(game, value, &ctx).unwrap_or(0)
}

pub(crate) fn resolve_cost_modifier_value_for_source(
    game: &GameState,
    source: ObjectId,
    controller: PlayerId,
    value: &crate::effect::Value,
) -> i32 {
    let mut dm = SelectFirstDecisionMaker;
    let ctx = ExecutionContext::new(source, controller, &mut dm);
    resolve_value(game, value, &ctx).unwrap_or(0)
}

/// Calculate the number of cards that need to be exiled for Delve.
///
/// Returns how many cards from graveyard should be exiled based on:
/// - The generic mana remaining in the cost after other reductions
/// - The player's available mana
/// - Cards available in graveyard
pub fn calculate_delve_exile_count(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    base_cost: &crate::mana::ManaCost,
) -> u32 {
    calculate_delve_exile_count_with_targets(game, player, spell, base_cost, 1)
}

/// Calculate the number of cards to exile for Delve with explicit target count.
pub fn calculate_delve_exile_count_with_targets(
    game: &GameState,
    player: PlayerId,
    spell: &crate::object::Object,
    base_cost: &crate::mana::ManaCost,
    chosen_target_count: usize,
) -> u32 {
    use crate::ability::AbilityKind;

    // Only calculate Delve if the spell actually has Delve
    let has_delve_ability = spell.abilities.iter().any(|a| {
        if let AbilityKind::Static(s) = &a.kind {
            s.has_delve()
        } else {
            false
        }
    });
    if !has_delve_ability {
        return 0;
    }

    // First apply other cost reductions (like Affinity)
    let mut cost_after_reductions = base_cost.clone();

    if has_affinity_for_artifacts(spell) {
        let artifact_count = count_artifacts_controlled(game, player);
        cost_after_reductions = cost_after_reductions.reduce_generic(artifact_count);
    }

    cost_after_reductions = apply_spell_cost_modifiers(
        game,
        player,
        spell,
        &cost_after_reductions,
        chosen_target_count,
        &[],
        &CastingMethod::Normal,
        None,
    );

    // Now calculate how much generic mana remains
    let generic_remaining = cost_after_reductions.generic_mana_total();

    // Get graveyard count and calculate exile amount
    let graveyard_count = game
        .player(player)
        .map(|player| {
            player
                .graveyard
                .iter()
                .filter(|&&card_id| card_id != spell.id)
                .count() as u32
        })
        .unwrap_or(0);

    // Exile up to the generic mana cost (maximum Delve)
    generic_remaining.min(graveyard_count)
}

/// Count the number of artifacts controlled by a player.
pub fn count_artifacts_controlled(game: &GameState, player: PlayerId) -> u32 {
    let view = DerivedGameView::new(game);
    count_artifacts_controlled_with_view(game, player, &view)
}

pub(crate) fn count_artifacts_controlled_with_view(
    game: &GameState,
    player: PlayerId,
    view: &DerivedGameView<'_>,
) -> u32 {
    game.battlefield
        .iter()
        .filter(|&&id| {
            if let Some(obj) = game.object(id) {
                game.controller_of(obj) == player
                    && view.object_has_card_type(id, crate::types::CardType::Artifact)
            } else {
                false
            }
        })
        .count() as u32
}

/// Check if a spell has the Delve ability.
pub fn has_delve(spell: &crate::object::Object) -> bool {
    use crate::ability::AbilityKind;
    spell.abilities.iter().any(|a| {
        if let AbilityKind::Static(s) = &a.kind {
            s.has_delve()
        } else {
            false
        }
    })
}

fn has_affinity_for_artifacts(spell: &crate::object::Object) -> bool {
    use crate::ability::AbilityKind;
    use ironsmith_core::StaticAbilityId;

    spell.abilities.iter().any(|a| {
        if let AbilityKind::Static(s) = &a.kind {
            s.id() == StaticAbilityId::AffinityForArtifacts
        } else {
            false
        }
    })
}

/// Count cards in a player's graveyard (for Delve calculation).
pub fn count_cards_in_graveyard(game: &GameState, player: PlayerId) -> u32 {
    game.player(player)
        .map(|p| p.graveyard.len() as u32)
        .unwrap_or(0)
}

/// Compute potential mana available to a player.
///
/// This includes:
/// - Current mana pool
/// - Mana from all untapped lands and mana sources that can be activated
///
/// Returns a ManaPool representing the maximum mana the player could produce.
pub fn compute_potential_mana(game: &GameState, player: PlayerId) -> crate::player::ManaPool {
    let view = DerivedGameView::new(game);
    compute_potential_mana_with_view(game, player, &view)
}

#[derive(Clone)]
struct AvailableManaSource {
    source_id: ObjectId,
    outputs: Vec<Vec<ManaSymbol>>,
    from_snow_source: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ManaPaymentSearchKey {
    pip_index: usize,
    white: u32,
    blue: u32,
    black: u32,
    red: u32,
    green: u32,
    colorless: u32,
    snow_white: u32,
    snow_blue: u32,
    snow_black: u32,
    snow_red: u32,
    snow_green: u32,
    snow_colorless: u32,
    life_to_pay: u32,
    used_sources_mask: u128,
}

impl ManaPaymentSearchKey {
    fn new(
        pip_index: usize,
        pool: &crate::player::ManaPool,
        snow_pool: &crate::player::ManaPool,
        life_to_pay: u32,
        used_sources_mask: u128,
    ) -> Self {
        Self {
            pip_index,
            white: pool.white,
            blue: pool.blue,
            black: pool.black,
            red: pool.red,
            green: pool.green,
            colorless: pool.colorless,
            snow_white: snow_pool.white,
            snow_blue: snow_pool.blue,
            snow_black: snow_pool.black,
            snow_red: snow_pool.red,
            snow_green: snow_pool.green,
            snow_colorless: snow_pool.colorless,
            life_to_pay,
            used_sources_mask,
        }
    }
}

pub(crate) fn can_pay_mana_cost_with_available_sources(
    game: &GameState,
    player: PlayerId,
    source: Option<ObjectId>,
    cost: &crate::mana::ManaCost,
    x_value: u32,
    reason: crate::costs::PaymentReason,
    mana_spend_policy: &crate::player::ManaSpendPolicy,
    allow_black_life: bool,
    view: &DerivedGameView<'_>,
) -> bool {
    let Some(player_obj) = game.player(player) else {
        return false;
    };

    let mut pips = expand_mana_cost_to_unit_pips(cost, x_value, allow_black_life);
    pips.sort_by_key(|pip| pip_payment_sort_key(pip));
    let has_life_payment_option = pips
        .iter()
        .flatten()
        .any(|symbol| matches!(symbol, ManaSymbol::Life(_)));
    let max_life_payment = if has_life_payment_option
        && game.can_lose_life(player)
        && (!reason.is_cast_or_ability_payment()
            || !game.player_cant_pay_life_to_cast_or_activate(player))
    {
        u32::try_from(player_obj.life).unwrap_or(0)
    } else {
        0
    };

    let sources = available_mana_sources_for_payment(game, player, view);
    let snow_pool = snow_mana_pool(game, player, source, reason);
    if sources.len() > 128 {
        return can_pay_expanded_pips_large_source_count(
            game,
            player,
            &pips,
            0,
            player_obj.mana_pool.clone(),
            snow_pool,
            &sources,
            &mut vec![false; sources.len()],
            0,
            max_life_payment,
            mana_spend_policy,
            source,
        );
    }

    let mut failed_states = std::collections::HashSet::new();
    can_pay_expanded_pips(
        game,
        player,
        &pips,
        0,
        player_obj.mana_pool.clone(),
        snow_pool,
        &sources,
        0,
        0,
        max_life_payment,
        mana_spend_policy,
        source,
        &mut failed_states,
    )
}

fn expand_mana_cost_to_unit_pips(
    cost: &crate::mana::ManaCost,
    x_value: u32,
    allow_black_life: bool,
) -> Vec<Vec<ManaSymbol>> {
    let mut pips = Vec::new();
    for pip in cost.pips() {
        if pip.len() == 1 {
            match pip[0] {
                ManaSymbol::Generic(n) => {
                    pips.extend((0..n).map(|_| vec![ManaSymbol::Generic(1)]));
                    continue;
                }
                ManaSymbol::X => {
                    pips.extend((0..x_value).map(|_| vec![ManaSymbol::Generic(1)]));
                    continue;
                }
                ManaSymbol::Black if allow_black_life => {
                    pips.push(vec![ManaSymbol::Black, ManaSymbol::Life(2)]);
                    continue;
                }
                _ => {}
            }
        }
        pips.push(pip.clone());
    }
    pips
}

fn pip_payment_sort_key(pip: &[ManaSymbol]) -> (u8, usize) {
    let has_generic = pip
        .iter()
        .any(|symbol| matches!(symbol, ManaSymbol::Generic(_) | ManaSymbol::X));
    let has_life_only = pip
        .iter()
        .all(|symbol| matches!(symbol, ManaSymbol::Life(_)));
    let has_colored = pip.iter().any(|symbol| {
        matches!(
            symbol,
            ManaSymbol::White
                | ManaSymbol::Blue
                | ManaSymbol::Black
                | ManaSymbol::Red
                | ManaSymbol::Green
        )
    });
    let has_colorless_or_snow = pip
        .iter()
        .any(|symbol| matches!(symbol, ManaSymbol::Colorless | ManaSymbol::Snow));

    let class = if has_colored && !has_generic {
        0
    } else if has_colorless_or_snow && !has_generic {
        1
    } else if has_colored {
        2
    } else if has_generic {
        3
    } else if has_life_only {
        4
    } else {
        5
    };
    (class, pip.len())
}

fn available_mana_sources_for_payment(
    game: &GameState,
    player: PlayerId,
    view: &DerivedGameView<'_>,
) -> Vec<AvailableManaSource> {
    use crate::ability::AbilityKind;

    let mut sources = Vec::new();
    let analysis = view.simple_battlefield_mana_analysis(player);

    for &perm_id in analysis.mana_source_ids() {
        let Some(object) = game.object(perm_id) else {
            continue;
        };
        let abilities = view
            .abilities_rc(perm_id)
            .unwrap_or_else(|| std::rc::Rc::new(object.abilities_vec()));
        let mut outputs_for_permanent = Vec::new();
        for &ability_index in analysis.mana_ability_indices_for(perm_id) {
            let Some(ability) = abilities.get(ability_index) else {
                continue;
            };
            let AbilityKind::Activated(mana_ability) = &ability.kind else {
                continue;
            };
            if analysis
                .activatable_indices_for(perm_id)
                .contains(&ability_index)
                || crate::special_actions::can_activate_mana_ability_check_with_view(
                    game,
                    player,
                    perm_id,
                    ability_index,
                    ability,
                    view,
                    None,
                )
                .is_ok()
            {
                let outputs = mana_ability_output_options(game, player, perm_id, mana_ability);
                for output in outputs {
                    if !outputs_for_permanent.contains(&output) {
                        outputs_for_permanent.push(output);
                    }
                }
            }
        }
        if !outputs_for_permanent.is_empty() {
            sources.push(AvailableManaSource {
                source_id: perm_id,
                outputs: outputs_for_permanent,
                from_snow_source: game
                    .current_has_supertype(perm_id, crate::types::Supertype::Snow),
            });
        }
    }

    sources
}

fn snow_mana_pool(
    game: &GameState,
    player: PlayerId,
    payment_source: Option<ObjectId>,
    reason: crate::costs::PaymentReason,
) -> crate::player::ManaPool {
    let Some(player_obj) = game.player(player) else {
        return crate::player::ManaPool::default();
    };
    let mut snow_pool = crate::player::ManaPool::default();
    let mut used_restricted = std::collections::HashSet::new();
    for unit in &player_obj.mana_source_provenance {
        let from_snow_source =
            unit.snapshot.as_ref().is_some_and(|snapshot| {
                snapshot.supertypes.contains(&crate::types::Supertype::Snow)
            }) || (unit.snapshot.is_none()
                && game.current_has_supertype(unit.source, crate::types::Supertype::Snow));
        if !from_snow_source
            || snow_pool.amount(unit.symbol) >= player_obj.mana_pool.amount(unit.symbol)
        {
            continue;
        }
        if unit.restricted {
            let Some(index) = player_obj
                .restricted_mana
                .iter()
                .enumerate()
                .find(|(index, restricted)| {
                    !used_restricted.contains(index)
                        && restricted.symbol == unit.symbol
                        && restricted.source == unit.source
                })
                .map(|(index, _)| index)
            else {
                continue;
            };
            used_restricted.insert(index);
            if !game.restricted_mana_unit_is_payable_for_reason(
                &player_obj.restricted_mana[index],
                payment_source,
                reason,
            ) {
                continue;
            }
        }
        snow_pool.add(unit.symbol, 1);
    }
    snow_pool
}

fn mana_ability_output_options(
    game: &GameState,
    player: PlayerId,
    source: ObjectId,
    mana_ability: &crate::ability::ActivatedAbility,
) -> Vec<Vec<ManaSymbol>> {
    use crate::effects::{
        AddColorlessManaEffect, AddManaEffect, AddManaOfAnyColorEffect, AddManaOfAnyOneColorEffect,
        AddScaledManaEffect,
    };

    if let Some(output) = mana_ability.mana_output.as_ref()
        && !output.is_empty()
    {
        return vec![output.clone()];
    }

    let resolve_amount = |value: &crate::effect::Value| -> usize {
        let mut dm = SelectFirstDecisionMaker;
        let ctx = ExecutionContext::new(source, player, &mut dm);
        resolve_value(game, value, &ctx).unwrap_or(0).max(0) as usize
    };

    let mut outputs = vec![Vec::new()];
    for effect in crate::ability::selected_resolution_effects_for_current_state(
        &mana_ability.effects,
        game,
        source,
        player,
    ) {
        let effect_outputs = if let Some(add_mana) = effect.downcast_ref::<AddManaEffect>() {
            vec![add_mana.mana.clone()]
        } else if let Some(add_colorless) = effect.downcast_ref::<AddColorlessManaEffect>() {
            vec![vec![
                ManaSymbol::Colorless;
                resolve_amount(&add_colorless.amount)
            ]]
        } else if let Some(add_scaled) = effect.downcast_ref::<AddScaledManaEffect>() {
            let repeats = resolve_amount(&add_scaled.amount);
            let mut output = Vec::new();
            for _ in 0..repeats {
                output.extend(add_scaled.mana.iter().copied());
            }
            vec![output]
        } else if let Some(add_any_color) = effect.downcast_ref::<AddManaOfAnyColorEffect>() {
            let colors = add_any_color.available_colors.as_deref().unwrap_or(&[
                crate::color::Color::White,
                crate::color::Color::Blue,
                crate::color::Color::Black,
                crate::color::Color::Red,
                crate::color::Color::Green,
            ]);
            any_color_output_options(colors, resolve_amount(&add_any_color.amount), false)
        } else if let Some(add_any_one_color) = effect.downcast_ref::<AddManaOfAnyOneColorEffect>()
        {
            any_color_output_options(
                &[
                    crate::color::Color::White,
                    crate::color::Color::Blue,
                    crate::color::Color::Black,
                    crate::color::Color::Red,
                    crate::color::Color::Green,
                ],
                resolve_amount(&add_any_one_color.amount),
                true,
            )
        } else if let Some(symbols) = effect.producible_mana_symbols(game, source, player) {
            symbols
                .into_iter()
                .filter(|symbol| is_payable_mana_symbol(*symbol))
                .map(|symbol| vec![symbol])
                .collect()
        } else {
            Vec::new()
        };

        if effect_outputs.is_empty() {
            continue;
        }
        outputs = combine_mana_output_options(&outputs, &effect_outputs);
    }

    let outputs = outputs
        .into_iter()
        .filter(|output| !output.is_empty())
        .collect::<Vec<_>>();
    if outputs.is_empty() {
        let inferred = mana_ability.inferred_mana_symbols(game, source, player);
        if inferred.is_empty() {
            Vec::new()
        } else {
            vec![inferred]
        }
    } else {
        outputs
    }
}

fn any_color_output_options(
    colors: &[crate::color::Color],
    amount: usize,
    same_color: bool,
) -> Vec<Vec<ManaSymbol>> {
    if amount == 0 {
        return vec![Vec::new()];
    }
    if same_color {
        return colors
            .iter()
            .map(|color| vec![ManaSymbol::from_color(*color); amount])
            .collect();
    }

    let mut outputs = vec![Vec::new()];
    for _ in 0..amount {
        let mut next = Vec::new();
        for output in &outputs {
            for color in colors {
                let mut candidate = output.clone();
                candidate.push(ManaSymbol::from_color(*color));
                next.push(candidate);
                if next.len() >= 128 {
                    return next;
                }
            }
        }
        outputs = next;
    }
    outputs
}

fn combine_mana_output_options(
    base: &[Vec<ManaSymbol>],
    next: &[Vec<ManaSymbol>],
) -> Vec<Vec<ManaSymbol>> {
    let mut combined = Vec::new();
    for left in base {
        for right in next {
            let mut output = left.clone();
            output.extend(right.iter().copied());
            combined.push(output);
            if combined.len() >= 128 {
                return combined;
            }
        }
    }
    combined
}

#[allow(clippy::too_many_arguments)]
fn can_pay_expanded_pips(
    game: &GameState,
    player: PlayerId,
    pips: &[Vec<ManaSymbol>],
    pip_index: usize,
    pool: crate::player::ManaPool,
    snow_pool: crate::player::ManaPool,
    sources: &[AvailableManaSource],
    used_sources_mask: u128,
    life_to_pay: u32,
    max_life_payment: u32,
    mana_spend_policy: &crate::player::ManaSpendPolicy,
    payment_source: Option<ObjectId>,
    failed_states: &mut std::collections::HashSet<ManaPaymentSearchKey>,
) -> bool {
    if pip_index >= pips.len() {
        return life_to_pay <= max_life_payment;
    }

    let key =
        ManaPaymentSearchKey::new(pip_index, &pool, &snow_pool, life_to_pay, used_sources_mask);
    if failed_states.contains(&key) {
        return false;
    }

    let pip = &pips[pip_index];
    for &symbol in pip {
        if let ManaSymbol::Life(amount) = symbol {
            let next_life = life_to_pay.saturating_add(amount as u32);
            if next_life <= max_life_payment
                && can_pay_expanded_pips(
                    game,
                    player,
                    pips,
                    pip_index + 1,
                    pool.clone(),
                    snow_pool.clone(),
                    sources,
                    used_sources_mask,
                    next_life,
                    max_life_payment,
                    mana_spend_policy,
                    payment_source,
                    failed_states,
                )
            {
                return true;
            }
            continue;
        }

        let mut pool_after = pool.clone();
        let mut snow_pool_after = snow_pool.clone();
        if remove_mana_for_pip(
            &mut pool_after,
            &mut snow_pool_after,
            symbol,
            mana_spend_policy,
        ) && can_pay_expanded_pips(
            game,
            player,
            pips,
            pip_index + 1,
            pool_after,
            snow_pool_after,
            sources,
            used_sources_mask,
            life_to_pay,
            max_life_payment,
            mana_spend_policy,
            payment_source,
            failed_states,
        ) {
            return true;
        }

        for (source_index, source) in sources.iter().enumerate() {
            let source_mask = 1u128 << source_index;
            if used_sources_mask & source_mask != 0 {
                continue;
            }
            let mut source_policy = mana_spend_policy.clone();
            if game.can_spend_mana_as_any_color_from_mana_source(
                player,
                payment_source,
                source.source_id,
            ) {
                source_policy.allow_mode(ironsmith_core::value_model::ManaSpendMode::AnyColor);
            }
            for output in &source.outputs {
                if let Some((pool_from_output, snow_pool_from_output)) =
                    consume_output_for_pip(output, symbol, &source_policy, source.from_snow_source)
                {
                    let mut combined_pool = pool.clone();
                    add_pool(&mut combined_pool, &pool_from_output);
                    let mut combined_snow_pool = snow_pool.clone();
                    add_pool(&mut combined_snow_pool, &snow_pool_from_output);
                    let can_pay_rest = can_pay_expanded_pips(
                        game,
                        player,
                        pips,
                        pip_index + 1,
                        combined_pool,
                        combined_snow_pool,
                        sources,
                        used_sources_mask | source_mask,
                        life_to_pay,
                        max_life_payment,
                        mana_spend_policy,
                        payment_source,
                        failed_states,
                    );
                    if can_pay_rest {
                        return true;
                    }
                }
            }
        }
    }

    failed_states.insert(key);
    false
}

#[allow(clippy::too_many_arguments)]
fn can_pay_expanded_pips_large_source_count(
    game: &GameState,
    player: PlayerId,
    pips: &[Vec<ManaSymbol>],
    pip_index: usize,
    pool: crate::player::ManaPool,
    snow_pool: crate::player::ManaPool,
    sources: &[AvailableManaSource],
    used_sources: &mut [bool],
    life_to_pay: u32,
    max_life_payment: u32,
    mana_spend_policy: &crate::player::ManaSpendPolicy,
    payment_source: Option<ObjectId>,
) -> bool {
    if pip_index >= pips.len() {
        return life_to_pay <= max_life_payment;
    }

    let pip = &pips[pip_index];
    for &symbol in pip {
        if let ManaSymbol::Life(amount) = symbol {
            let next_life = life_to_pay.saturating_add(amount as u32);
            if next_life <= max_life_payment
                && can_pay_expanded_pips_large_source_count(
                    game,
                    player,
                    pips,
                    pip_index + 1,
                    pool.clone(),
                    snow_pool.clone(),
                    sources,
                    used_sources,
                    next_life,
                    max_life_payment,
                    mana_spend_policy,
                    payment_source,
                )
            {
                return true;
            }
            continue;
        }

        let mut pool_after = pool.clone();
        let mut snow_pool_after = snow_pool.clone();
        if remove_mana_for_pip(
            &mut pool_after,
            &mut snow_pool_after,
            symbol,
            mana_spend_policy,
        ) && can_pay_expanded_pips_large_source_count(
            game,
            player,
            pips,
            pip_index + 1,
            pool_after,
            snow_pool_after,
            sources,
            used_sources,
            life_to_pay,
            max_life_payment,
            mana_spend_policy,
            payment_source,
        ) {
            return true;
        }

        for (source_index, source) in sources.iter().enumerate() {
            if used_sources[source_index] {
                continue;
            }
            let mut source_policy = mana_spend_policy.clone();
            if game.can_spend_mana_as_any_color_from_mana_source(
                player,
                payment_source,
                source.source_id,
            ) {
                source_policy.allow_mode(ironsmith_core::value_model::ManaSpendMode::AnyColor);
            }
            for output in &source.outputs {
                if let Some((pool_from_output, snow_pool_from_output)) =
                    consume_output_for_pip(output, symbol, &source_policy, source.from_snow_source)
                {
                    let mut combined_pool = pool.clone();
                    add_pool(&mut combined_pool, &pool_from_output);
                    let mut combined_snow_pool = snow_pool.clone();
                    add_pool(&mut combined_snow_pool, &snow_pool_from_output);
                    used_sources[source_index] = true;
                    let can_pay_rest = can_pay_expanded_pips_large_source_count(
                        game,
                        player,
                        pips,
                        pip_index + 1,
                        combined_pool,
                        combined_snow_pool,
                        sources,
                        used_sources,
                        life_to_pay,
                        max_life_payment,
                        mana_spend_policy,
                        payment_source,
                    );
                    used_sources[source_index] = false;
                    if can_pay_rest {
                        return true;
                    }
                }
            }
        }
    }

    false
}

fn consume_output_for_pip(
    output: &[ManaSymbol],
    pip: ManaSymbol,
    mana_spend_policy: &crate::player::ManaSpendPolicy,
    from_snow_source: bool,
) -> Option<(crate::player::ManaPool, crate::player::ManaPool)> {
    for (idx, &produced) in output.iter().enumerate() {
        if mana_symbol_can_pay_pip(produced, pip, mana_spend_policy, from_snow_source) {
            let mut remainder = crate::player::ManaPool::default();
            let mut snow_remainder = crate::player::ManaPool::default();
            for (other_idx, &symbol) in output.iter().enumerate() {
                if other_idx != idx && is_payable_mana_symbol(symbol) {
                    remainder.add(symbol, 1);
                    if from_snow_source {
                        snow_remainder.add(symbol, 1);
                    }
                }
            }
            return Some((remainder, snow_remainder));
        }
    }
    None
}

fn add_pool(pool: &mut crate::player::ManaPool, addition: &crate::player::ManaPool) {
    for symbol in PAYABLE_MANA_SYMBOLS {
        let amount = addition.amount(symbol);
        if amount > 0 {
            pool.add(symbol, amount);
        }
    }
}

const PAYABLE_MANA_SYMBOLS: [ManaSymbol; 6] = [
    ManaSymbol::White,
    ManaSymbol::Blue,
    ManaSymbol::Black,
    ManaSymbol::Red,
    ManaSymbol::Green,
    ManaSymbol::Colorless,
];

fn remove_mana_for_pip(
    pool: &mut crate::player::ManaPool,
    snow_pool: &mut crate::player::ManaPool,
    pip: ManaSymbol,
    mana_spend_policy: &crate::player::ManaSpendPolicy,
) -> bool {
    match pip {
        ManaSymbol::White
        | ManaSymbol::Blue
        | ManaSymbol::Black
        | ManaSymbol::Red
        | ManaSymbol::Green => {
            for symbol in PAYABLE_MANA_SYMBOLS {
                if mana_spend_policy.can_pay_symbol(symbol, pip)
                    && remove_pool_unit_preserving_snow(pool, snow_pool, symbol)
                {
                    return true;
                }
            }
            false
        }
        ManaSymbol::Colorless => {
            for symbol in PAYABLE_MANA_SYMBOLS {
                if mana_spend_policy.can_pay_symbol(symbol, ManaSymbol::Colorless)
                    && remove_pool_unit_preserving_snow(pool, snow_pool, symbol)
                {
                    return true;
                }
            }
            false
        }
        ManaSymbol::Generic(_) => remove_any_payable_mana(pool, snow_pool),
        ManaSymbol::Snow => remove_any_snow_mana(pool, snow_pool),
        ManaSymbol::Life(_) | ManaSymbol::X => false,
    }
}

fn remove_pool_unit_preserving_snow(
    pool: &mut crate::player::ManaPool,
    snow_pool: &mut crate::player::ManaPool,
    symbol: ManaSymbol,
) -> bool {
    if pool.amount(symbol) > snow_pool.amount(symbol) {
        return pool.remove(symbol, 1);
    }
    if pool.remove(symbol, 1) {
        snow_pool.remove(symbol, 1);
        return true;
    }
    false
}

fn remove_any_payable_mana(
    pool: &mut crate::player::ManaPool,
    snow_pool: &mut crate::player::ManaPool,
) -> bool {
    for symbol in PAYABLE_MANA_SYMBOLS {
        if remove_pool_unit_preserving_snow(pool, snow_pool, symbol) {
            return true;
        }
    }
    false
}

fn remove_any_snow_mana(
    pool: &mut crate::player::ManaPool,
    snow_pool: &mut crate::player::ManaPool,
) -> bool {
    for symbol in PAYABLE_MANA_SYMBOLS {
        if snow_pool.remove(symbol, 1) {
            return pool.remove(symbol, 1);
        }
    }
    false
}

fn mana_symbol_can_pay_pip(
    produced: ManaSymbol,
    pip: ManaSymbol,
    mana_spend_policy: &crate::player::ManaSpendPolicy,
    from_snow_source: bool,
) -> bool {
    match pip {
        ManaSymbol::Generic(_) => is_payable_mana_symbol(produced),
        ManaSymbol::White
        | ManaSymbol::Blue
        | ManaSymbol::Black
        | ManaSymbol::Red
        | ManaSymbol::Green
        | ManaSymbol::Colorless => mana_spend_policy.can_pay_symbol(produced, pip),
        ManaSymbol::Snow => from_snow_source && is_payable_mana_symbol(produced),
        ManaSymbol::Life(_) | ManaSymbol::X => false,
    }
}

fn is_payable_mana_symbol(symbol: ManaSymbol) -> bool {
    matches!(
        symbol,
        ManaSymbol::White
            | ManaSymbol::Blue
            | ManaSymbol::Black
            | ManaSymbol::Red
            | ManaSymbol::Green
            | ManaSymbol::Colorless
    )
}

pub(crate) fn compute_potential_mana_with_view(
    game: &GameState,
    player: PlayerId,
    view: &DerivedGameView<'_>,
) -> crate::player::ManaPool {
    use crate::ability::AbilityKind;
    use crate::costs::{CostCheckContext, can_pay_with_check_context};

    // Start with current mana pool
    let mut potential = game
        .player(player)
        .map(|p| p.mana_pool.clone())
        .unwrap_or_default();
    let simple_mana_analysis = view.simple_battlefield_mana_analysis(player);

    // Add mana from all available mana abilities.
    // The pass-local analysis already found the controlled mana sources.
    for &perm_id in simple_mana_analysis.mana_source_ids() {
        let Some(perm) = game.object(perm_id) else {
            continue;
        };

        let mana_ability_indices = simple_mana_analysis.mana_ability_indices_for(perm_id);
        if mana_ability_indices.len() == 1
            && let Some(symbols) = simple_mana_analysis.first_output_for(perm_id)
        {
            for mana in symbols {
                potential.add(*mana, 1);
            }
            continue;
        }

        let cached_abilities = view.abilities_rc(perm_id);
        let abilities = cached_abilities.as_deref().unwrap_or(&perm.abilities);
        for &ability_idx in mana_ability_indices {
            let Some(ability) = abilities.get(ability_idx) else {
                continue;
            };
            let AbilityKind::Activated(mana_ability) = &ability.kind else {
                continue;
            };
            if !mana_ability.is_runtime_mana_ability(game, perm_id, game.controller_of(perm)) {
                continue;
            }
            if mana_ability.has_tap_cost() && !game.can_activate_tap_abilities_of(perm_id) {
                continue;
            }
            // Do a simple non-recursive check for whether this mana ability
            // could be activated. We intentionally skip mana cost checks here
            // to avoid infinite recursion (mana ability with mana cost would
            // call compute_potential_mana again).
            let simple_taplike_costs_only = mana_ability.mana_cost.costs().iter().all(|cost| {
                cost.processing_mode().is_mana_payment()
                    || cost.requires_tap()
                    || cost.requires_untap()
            });

            let can_activate = if simple_taplike_costs_only {
                mana_ability.mana_cost.costs().iter().all(|cost| {
                    if cost.requires_tap() {
                        return !game.is_tapped(perm_id)
                            && (!view
                                .object_has_card_type(perm_id, crate::types::CardType::Creature)
                                || !game.is_summoning_sick(perm_id)
                                || view.object_has_static_ability_id(
                                    perm_id,
                                    crate::static_abilities::StaticAbilityId::Haste,
                                ));
                    }
                    if cost.requires_untap() {
                        return game.is_tapped(perm_id)
                            && (!view
                                .object_has_card_type(perm_id, crate::types::CardType::Creature)
                                || !game.is_summoning_sick(perm_id)
                                || view.object_has_static_ability_id(
                                    perm_id,
                                    crate::static_abilities::StaticAbilityId::Haste,
                                ));
                    }
                    true
                })
            } else {
                let ctx = CostCheckContext::new(perm_id, player)
                    .with_reason(crate::costs::PaymentReason::ActivateManaAbility);
                let components = mana_ability.mana_cost.costs();
                let mut idx = 0usize;
                let mut payable = true;
                while idx < components.len() {
                    let cost = if let Some(choose) =
                        components[idx].effect_ref().and_then(|effect| {
                            effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()
                        })
                        && let Some(next) = components.get(idx + 1)
                        && let Some(step) = crate::game_loop::choose_tagged_cost_step(choose, next)
                    {
                        idx += 2;
                        match step {
                            crate::game_loop::ActivationCostStep::Cost(cost)
                            | crate::game_loop::ActivationCostStep::Sacrifice { cost, .. } => cost,
                            crate::game_loop::ActivationCostStep::CardChoice(choice) => {
                                activation_card_cost_choice_cost(&choice).clone()
                            }
                        }
                    } else {
                        let cost = components[idx].clone();
                        idx += 1;
                        cost
                    };

                    // Skip mana cost check to avoid recursion - we only check
                    // non-mana costs like tap, life, sacrifice.
                    if cost.processing_mode().is_mana_payment() {
                        continue;
                    }

                    if game
                        .validate_cost_for_payment_reason(player, perm_id, &cost, ctx.reason)
                        .is_err()
                    {
                        payable = false;
                        break;
                    }
                    if can_pay_with_check_context(&*cost.0, game, &ctx).is_err() {
                        payable = false;
                        break;
                    }
                }
                payable
            };

            // Also check activation condition if present
            let condition_met = mana_ability
                .activation_condition
                .as_ref()
                .is_none_or(|cond| {
                    check_mana_ability_condition_for_potential(
                        game,
                        player,
                        perm_id,
                        ability_idx,
                        cond,
                    )
                });

            if can_activate && condition_met {
                // Add the mana this ability could produce, preserving
                // multiplicity for effects like Black Lotus.
                for mana in
                    inferred_potential_mana_symbols_for_ability(game, perm_id, player, mana_ability)
                {
                    potential.add(mana, 1);
                }
            }
        }
    }

    potential
}

fn activation_card_cost_choice_cost(
    choice: &crate::game_loop::ActivationCardCostChoice,
) -> &crate::costs::Cost {
    match choice {
        crate::game_loop::ActivationCardCostChoice::Discard { cost, .. }
        | crate::game_loop::ActivationCardCostChoice::ExileFromHand { cost, .. }
        | crate::game_loop::ActivationCardCostChoice::ExileFromGraveyard { cost, .. }
        | crate::game_loop::ActivationCardCostChoice::ExileChosenObject { cost, .. }
        | crate::game_loop::ActivationCardCostChoice::RevealFromHand { cost, .. }
        | crate::game_loop::ActivationCardCostChoice::ReturnToHand { cost, .. }
        | crate::game_loop::ActivationCardCostChoice::MoveChosenObjectToZone { cost, .. } => cost,
    }
}

pub(crate) fn simple_battlefield_mana_ability_output(
    game: &GameState,
    player: PlayerId,
    permanent_id: ObjectId,
    ability_index: usize,
    ability: &crate::ability::Ability,
    view: &DerivedGameView<'_>,
) -> Option<Vec<ManaSymbol>> {
    use crate::ability::AbilityKind;

    let object = game.object(permanent_id)?;
    if game.controller_of(object) != player
        || object.zone != Zone::Battlefield
        || !ability.functions_in(&object.zone)
    {
        return None;
    }

    let AbilityKind::Activated(mana_ability) = &ability.kind else {
        return None;
    };
    if !mana_ability.is_runtime_mana_ability(game, permanent_id, player)
        || !game.can_activate_abilities_of(permanent_id)
    {
        return None;
    }
    if mana_ability.has_tap_cost() && !game.can_activate_tap_abilities_of(permanent_id) {
        return None;
    }
    if !mana_ability
        .mana_cost
        .costs()
        .iter()
        .all(|cost| cost.requires_tap() || cost.requires_untap())
    {
        return None;
    }

    for cost in mana_ability.mana_cost.costs() {
        if cost.requires_tap() {
            if game.is_tapped(permanent_id) {
                return None;
            }
            if view.object_has_card_type(permanent_id, crate::types::CardType::Creature)
                && game.is_summoning_sick(permanent_id)
                && !view.object_has_static_ability_id(
                    permanent_id,
                    crate::static_abilities::StaticAbilityId::Haste,
                )
            {
                return None;
            }
        }
        if cost.requires_untap() && !game.is_tapped(permanent_id) {
            return None;
        }
        if cost.requires_untap()
            && view.object_has_card_type(permanent_id, crate::types::CardType::Creature)
            && game.is_summoning_sick(permanent_id)
            && !view.object_has_static_ability_id(
                permanent_id,
                crate::static_abilities::StaticAbilityId::Haste,
            )
        {
            return None;
        }
    }

    if let Some(condition) = &mana_ability.activation_condition
        && !check_mana_ability_condition_for_potential(
            game,
            player,
            permanent_id,
            ability_index,
            condition,
        )
    {
        return None;
    }

    Some(inferred_potential_mana_symbols_for_ability(
        game,
        permanent_id,
        player,
        mana_ability,
    ))
}

pub(crate) fn inferred_potential_mana_symbols_for_ability(
    game: &GameState,
    source: ObjectId,
    controller: PlayerId,
    mana_ability: &crate::ability::ActivatedAbility,
) -> Vec<ManaSymbol> {
    use crate::effects::{
        AddColorlessManaEffect, AddManaEffect, AddManaOfAnyColorEffect, AddManaOfAnyOneColorEffect,
        AddScaledManaEffect,
    };

    if let Some(mana_output) = mana_ability.mana_output.as_ref()
        && !mana_output.is_empty()
    {
        return mana_output.clone();
    }

    let resolve_amount = |value: &crate::effect::Value| -> usize {
        let mut dm = SelectFirstDecisionMaker;
        let ctx = ExecutionContext::new(source, controller, &mut dm);
        resolve_value(game, value, &ctx).unwrap_or(0).max(0) as usize
    };

    let mut inferred = Vec::new();
    for effect in crate::ability::selected_resolution_effects_for_current_state(
        &mana_ability.effects,
        game,
        source,
        controller,
    ) {
        if let Some(add_mana) = effect.downcast_ref::<AddManaEffect>() {
            inferred.extend(add_mana.mana.iter().copied());
            continue;
        }
        if let Some(add_colorless) = effect.downcast_ref::<AddColorlessManaEffect>() {
            inferred.extend(std::iter::repeat_n(
                ManaSymbol::Colorless,
                resolve_amount(&add_colorless.amount),
            ));
            continue;
        }
        if let Some(add_scaled) = effect.downcast_ref::<AddScaledManaEffect>() {
            let repeats = resolve_amount(&add_scaled.amount);
            for _ in 0..repeats {
                inferred.extend(add_scaled.mana.iter().copied());
            }
            continue;
        }
        if let Some(add_any_color) = effect.downcast_ref::<AddManaOfAnyColorEffect>() {
            let amount = resolve_amount(&add_any_color.amount);
            let colors = add_any_color.available_colors.as_deref().unwrap_or(&[
                crate::color::Color::White,
                crate::color::Color::Blue,
                crate::color::Color::Black,
                crate::color::Color::Red,
                crate::color::Color::Green,
            ]);
            for color in colors {
                inferred.extend(std::iter::repeat_n(ManaSymbol::from_color(*color), amount));
            }
            continue;
        }
        if let Some(add_any_one_color) = effect.downcast_ref::<AddManaOfAnyOneColorEffect>() {
            let amount = resolve_amount(&add_any_one_color.amount);
            for color in [
                crate::color::Color::White,
                crate::color::Color::Blue,
                crate::color::Color::Black,
                crate::color::Color::Red,
                crate::color::Color::Green,
            ] {
                inferred.extend(std::iter::repeat_n(ManaSymbol::from_color(color), amount));
            }
            continue;
        }

        if let Some(symbols) = effect.producible_mana_symbols(game, source, controller) {
            inferred.extend(symbols.into_iter().filter(|symbol| {
                matches!(
                    symbol,
                    ManaSymbol::White
                        | ManaSymbol::Blue
                        | ManaSymbol::Black
                        | ManaSymbol::Red
                        | ManaSymbol::Green
                        | ManaSymbol::Colorless
                )
            }));
        }
    }

    if inferred.is_empty() {
        mana_ability.inferred_mana_symbols(game, source, controller)
    } else {
        inferred
    }
}

/// Check mana ability condition for potential mana computation.
pub(crate) fn check_mana_ability_condition_for_potential(
    game: &GameState,
    player: PlayerId,
    source: ObjectId,
    ability_index: usize,
    condition: &crate::ConditionExpr,
) -> bool {
    let eval_ctx = crate::condition_eval::ExternalEvaluationContext {
        controller: player,
        source,
        defending_player: None,
        attacking_player: None,
        filter_source: Some(source),
        iterated_player: None,
        triggering_event: None,
        trigger_identity: None,
        ability_index: Some(ability_index),
        options: crate::condition_eval::ExternalEvaluationOptions::default(),
    };
    crate::condition_eval::evaluate_condition_external(game, condition, &eval_ctx)
}

#[cfg(test)]
mod tests;
