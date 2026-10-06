//! Independent CR 118.9 prices. A price never establishes an origin permission.
use super::{AlternativeCastingMethod, CastingMethod, GrantSelection};
use crate::effects::ExecutionError;
use crate::filter::ObjectFilterExt as _;
use crate::grant::{DerivedAlternativeCast, DerivedAlternativeCastRuntimeExt as _, Grantable};
use crate::grant_registry::{Grant, grant_usage_limit_allows};
use crate::object::{CastPriceReceipt, Object};
use crate::{GameState, PlayerId, Zone};

type PriceReceipt =
    CastPriceReceipt<crate::cost::TotalCost, crate::grant_registry::GrantPermissionIdentity>;

pub(crate) struct PriceRoute {
    pub price: Grant,
    pub origin: Option<Grant>,
    pub origin_alternative: Option<AlternativeCastingMethod>,
    pub price_riders: Vec<crate::static_abilities::StaticAbility>,
    pub origin_mana_surcharge: crate::mana::ManaCost,
    pub price_snapshot: Option<crate::snapshot::ObjectSnapshot>,
    pub origin_snapshot: Option<crate::snapshot::ObjectSnapshot>,
    pub prototype: Option<usize>,
}

fn grant_available(game: &GameState, player: PlayerId, grant: &Grant) -> bool {
    let limit = match &grant.grantable {
        Grantable::DerivedAlternativeCast(spec) => spec.usage_limit().or(grant.usage_limit),
        _ => grant.usage_limit,
    };
    grant_usage_limit_allows(game, player, grant.permission_identity.as_ref(), limit)
}

fn selection(grant: &Grant, index: usize) -> Option<GrantSelection> {
    Some(GrantSelection {
        identity: grant.permission_identity.clone()?,
        source: grant.source.source_id(),
        index,
    })
}

fn ordinary_origin(
    game: &GameState,
    player: PlayerId,
    spell: &Object,
    origin: &CastingMethod,
) -> bool {
    (matches!(origin, CastingMethod::Normal)
        && crate::decision::native_exile_normal_cast_origin(game, player, spell.id))
        || (spell.zone == Zone::Hand && spell.owner == player)
        || (spell.zone == Zone::Command
            && game.player(player).is_some_and(|p| {
                p.get_commanders()
                    .iter()
                    .any(|id| game.current_commander_object(*id) == Some(spell.id))
            }))
}

/// These origin rules impose additional costs, not a competing alternative.
/// CR 702.81a / 702.133a. Flashback, escape, morph and intrinsic FromZone
/// replacement prices are intentionally absent.
fn method_allows_separate_price(method: &AlternativeCastingMethod) -> bool {
    matches!(
        method,
        AlternativeCastingMethod::Retrace { .. }
            | AlternativeCastingMethod::JumpStart { .. }
            | AlternativeCastingMethod::FlashWithAdditionalCost { .. }
    )
}

fn grant_allows_separate_price(grant: &Grant) -> bool {
    matches!(
        &grant.grantable,
        Grantable::PlayFrom
            | Grantable::DerivedAlternativeCast(
                DerivedAlternativeCast::GraveyardCastFromCardManaCost { .. }
                    | DerivedAlternativeCast::RetraceFromCardManaCost
            )
    ) || matches!(&grant.grantable, Grantable::AlternativeCast(method) if method_allows_separate_price(method))
}

/// Install the proposed face for both source-relative filters and ordinary
/// characteristic lookups. The whole query stays fallible and side-effect free.
pub(crate) fn proposed_face(
    game: &GameState,
    spell: &Object,
    origin: &CastingMethod,
    prototype: Option<usize>,
) -> Option<Object> {
    let spell = game.object(spell.id).unwrap_or(spell);
    if spell.zone == Zone::Stack {
        return Some(spell.clone());
    }
    let mut face = match origin {
        CastingMethod::SplitOtherHalf | CastingMethod::SplitOtherHalfPlayFrom { .. } => {
            crate::decision::spell_view_for_split_other_half_cast(game, spell)?
        }
        CastingMethod::Fuse => crate::decision::spell_view_for_fused_split_cast(game, spell)?,
        _ => spell.clone(),
    };
    if !matches!(origin, CastingMethod::Fuse) {
        face.split_combined = None;
    }
    if let Some(index) = prototype {
        let method = face.alternative_casts.get(index)?;
        let (cost, power_toughness) = (
            method.mana_cost()?.clone(),
            method.prototype_power_toughness()?,
        );
        face.apply_prototype_cast_overlay(cost, power_toughness);
    }
    Some(face)
}

fn prototype_choices(
    game: &GameState,
    spell: &Object,
    origin: &CastingMethod,
) -> Vec<Option<usize>> {
    let mut choices = vec![None];
    if let Some(face) = proposed_face(game, spell, origin, None) {
        choices.extend(
            face.alternative_casts
                .iter()
                .enumerate()
                .filter_map(|(index, method)| {
                    method.prototype_power_toughness().map(|_| Some(index))
                }),
        );
    }
    choices
}

fn face_query(
    game: &GameState,
    spell: &Object,
    origin: &CastingMethod,
    prototype: Option<usize>,
) -> Result<Option<GameState>, ExecutionError> {
    if spell.zone == Zone::Stack {
        return Ok(None);
    }
    let Some(face) = proposed_face(game, spell, origin, prototype) else {
        return Ok(None);
    };
    let needs_overlay = prototype.is_some()
        || spell.split_combined.is_some()
        || matches!(
            origin,
            CastingMethod::SplitOtherHalf
                | CastingMethod::SplitOtherHalfPlayFrom { .. }
                | CastingMethod::Fuse
        );
    let checked = if needs_overlay {
        crate::grant_registry::proposed_card_face_query(game, &face)?
    } else {
        game.continuous_query_snapshot()
            .map_err(ExecutionError::ContinuousDiscovery)?
    };
    Ok(Some(checked))
}

/// Resolve both live providers before the card moves. No fallback to another
/// permission of the same source or to a new position in the grant list.
pub(crate) fn resolve_announcement(
    game: &GameState,
    player: PlayerId,
    spell: &Object,
    method: &CastingMethod,
) -> Result<Option<PriceRoute>, ExecutionError> {
    resolve_announcement_with_effect_authority(game, player, spell, method, false)
}

/// Only the existing resolving-instruction owner passes true. Public priority
/// methods cannot use this authority, even when their source/zone are public.
pub(crate) fn resolve_announcement_with_effect_authority(
    game: &GameState,
    player: PlayerId,
    spell: &Object,
    method: &CastingMethod,
    effect_authorized: bool,
) -> Result<Option<PriceRoute>, ExecutionError> {
    let CastingMethod::AlternativePrice {
        origin,
        origin_permission,
        price,
        prototype,
    } = method
    else {
        return Ok(None);
    };
    if matches!(origin.as_ref(), CastingMethod::AlternativePrice { .. } | CastingMethod::ExactPermission { .. })
        || spell.zone == Zone::Stack
    {
        return Ok(None);
    }
    let Some(query) = face_query(game, spell, origin, *prototype)? else {
        return Ok(None);
    };
    let game = &query;
    let face = game
        .object(spell.id)
        .ok_or(ExecutionError::ObjectNotFound(spell.id))?;
    let grants = game
        .effect_store
        .grant_registry
        .get_grants_for_card(game, spell.id, spell.zone, player);
    let selected = |key: &GrantSelection| {
        grants.get(key.index).filter(|grant| {
            grant.permission_identity.as_ref() == Some(&key.identity)
                && grant.source.source_id() == key.source
                && grant_available(game, player, grant)
        })
    };
    let Some(price_grant) =
        selected(price).filter(|g| matches!(g.grantable, Grantable::AlternativePrice { .. }))
    else {
        return Ok(None);
    };
    let origin_grant = match origin.as_ref() {
        CastingMethod::Normal | CastingMethod::SplitOtherHalf | CastingMethod::Fuse => {
            if origin_permission.is_some()
                || (!effect_authorized && !ordinary_origin(game, player, spell, origin))
                || (matches!(origin.as_ref(), CastingMethod::Fuse)
                    && (spell.zone != Zone::Hand || !spell.has_fuse))
            {
                return Ok(None);
            }
            None
        }
        CastingMethod::Alternative(index) => {
            if origin_permission.is_some()
                || spell.owner != player
                || !spell.alternative_casts.get(*index).is_some_and(|method| {
                    method.cast_from_zone() == spell.zone && method_allows_separate_price(method)
                })
            {
                return Ok(None);
            }
            None
        }
        CastingMethod::PlayFrom {
            source,
            zone,
            use_alternative,
        }
        | CastingMethod::SplitOtherHalfPlayFrom {
            source,
            zone,
            use_alternative,
        } => {
            if origin_permission.is_none()
                && effect_authorized
                && use_alternative.is_none()
                && *zone == spell.zone
            {
                None
            } else {
                let Some(key) = origin_permission.as_ref() else {
                    return Ok(None);
                };
                let Some(grant) = selected(key).filter(|grant| grant_allows_separate_price(grant))
                else {
                    return Ok(None);
                };
                if *source != key.source || *zone != spell.zone || grant.zone != spell.zone {
                    return Ok(None);
                }
                match (&grant.grantable, use_alternative) {
                    (_, None) => {}
                    (_, Some(index)) => {
                        if crate::decision::resolve_play_from_alternative_grant(
                            game, player, face, *zone, *index,
                        )
                        .is_none_or(|resolved| {
                            resolved.permission_identity.as_ref() != Some(&key.identity)
                        }) {
                            return Ok(None);
                        }
                    }
                }
                Some(grant.clone())
            }
        }
        _ => return Ok(None),
    };
    let origin_alternative = if let Some(grant) = &origin_grant {
        match &grant.grantable {
            Grantable::DerivedAlternativeCast(spec) => {
                // The permission is independently valid even for a card with
                // no mana cost. Only its price is replaced; no characteristic
                // query sees this local materialization helper.
                let mut priced_face = face.clone();
                if priced_face.mana_cost.is_none() {
                    priced_face.mana_cost = Some(crate::mana::ManaCost::new().into());
                }
                spec.materialize_for(&priced_face)
            }
            Grantable::AlternativeCast(method) => Some(method.clone()),
            _ => None,
        }
    } else if let CastingMethod::Alternative(index) = origin.as_ref() {
        spell.alternative_casts.get(*index).cloned()
    } else {
        None
    };
    if origin_grant
        .as_ref()
        .is_some_and(|grant| !matches!(grant.grantable, Grantable::PlayFrom))
        && origin_alternative.is_none()
    {
        return Ok(None);
    }
    let mut origin_mana_surcharge = crate::mana::ManaCost::new();
    if let Some(Grant {
        grantable:
            Grantable::DerivedAlternativeCast(DerivedAlternativeCast::GraveyardCastFromCardManaCost {
                additional_costs,
                ..
            }),
        ..
    }) = &origin_grant
    {
        for cost in additional_costs
            .iter()
            .filter_map(crate::costs::Cost::mana_cost_ref)
        {
            origin_mana_surcharge = crate::decision::add_mana_cost(&origin_mana_surcharge, cost);
        }
    } else if let Some(AlternativeCastingMethod::FlashWithAdditionalCost {
        additional_cost, ..
    }) = &origin_alternative
    {
        origin_mana_surcharge = additional_cost.clone();
    } else if let Some(AlternativeCastingMethod::JumpStart { additional_cost }) =
        &origin_alternative
    {
        for cost in additional_cost
            .costs()
            .iter()
            .filter_map(crate::costs::Cost::mana_cost_ref)
        {
            origin_mana_surcharge = crate::decision::add_mana_cost(&origin_mana_surcharge, cost);
        }
    }
    let context = game.filter_context_for(player, Some(price_grant.source.source_id()));
    let price_riders = if price_grant
        .cast_this_way_filter
        .as_ref()
        .is_none_or(|filter| filter.matches(face, &context, game))
    {
        price_grant.cast_this_way_grants.clone()
    } else {
        Vec::new()
    };
    let snapshot = |source| {
        game.object(source).map(|object| {
            crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                object, game,
            )
        })
    };
    let price_snapshot = snapshot(price_grant.source.source_id());
    let origin_snapshot = origin_grant
        .as_ref()
        .and_then(|grant| snapshot(grant.source.source_id()));
    Ok(Some(PriceRoute {
        price: price_grant.clone(),
        origin: origin_grant,
        origin_alternative,
        price_riders,
        origin_mana_surcharge,
        price_snapshot,
        origin_snapshot,
        prototype: *prototype,
    }))
}

pub(crate) fn price_receipt(
    game: &GameState,
    player: PlayerId,
    spell: &Object,
    method: &CastingMethod,
) -> Result<Option<PriceReceipt>, ExecutionError> {
    let CastingMethod::AlternativePrice {
        price,
        origin_permission,
        prototype,
        ..
    } = method
    else {
        return Ok(None);
    };
    if spell.zone == Zone::Stack {
        return Ok(spell
            .cast_price
            .as_deref()
            .filter(|receipt| {
                receipt.identity == price.identity
                    && receipt.source == price.source
                    && receipt.prototype == *prototype
                    && receipt.prototype.is_some() == spell.prototype_cast_state.is_some()
                    && spell.cast_grant_usage_identity.as_deref()
                        == origin_permission.as_ref().map(|origin| &origin.identity)
            })
            .cloned());
    }
    Ok(resolve_announcement(game, player, spell, method)?
        .and_then(|route| receipt_from_route(&route)))
}

pub(crate) fn receipt_from_route(route: &PriceRoute) -> Option<PriceReceipt> {
    let grant = &route.price;
    let Grantable::AlternativePrice { costs, .. } = &grant.grantable else {
        return None;
    };
    Some(CastPriceReceipt {
        identity: grant.permission_identity.clone()?,
        source: grant.source.source_id(),
        total_cost: crate::cost::TotalCost::from_costs(costs.clone()),
        origin_mana_surcharge: route.origin_mana_surcharge.clone(),
        prototype: route.prototype,
        constraints: grant.play_from_constraints.clone(),
    })
}

/// Boolean legacy query adapters preserve the original typed incomplete result
/// in their Result-bearing owner's latch instead of turning it into no plan.
pub(crate) fn receipt_or_latch(
    game: &GameState,
    player: PlayerId,
    spell: &Object,
    method: &CastingMethod,
) -> Option<PriceReceipt> {
    match price_receipt(game, player, spell, method) {
        Ok(receipt) => receipt,
        Err(error) => {
            game.record_token_resource_failure(&error);
            None
        }
    }
}

/// Candidate origins are collected before ordinary price/timing affordability:
/// a free price can enable an otherwise unaffordable spell, and the selected
/// price may itself supply flash. Full spell legality is checked by the caller.
pub(crate) fn candidates(
    game: &GameState,
    player: PlayerId,
    spell: &Object,
) -> Result<Vec<CastingMethod>, ExecutionError> {
    let mut result = Vec::new();
    for other_face in [false, true] {
        let face_method = if other_face {
            CastingMethod::SplitOtherHalf
        } else {
            CastingMethod::Normal
        };
        if other_face && !crate::decision::spell_has_castable_linked_other_half(game, spell) {
            continue;
        }
        for prototype in prototype_choices(game, spell, &face_method) {
            let Some(query) = face_query(game, spell, &face_method, prototype)? else {
                continue;
            };
            let grants = query
                .effect_store
                .grant_registry
                .get_grants_for_card(&query, spell.id, spell.zone, player);
            let prices = grants
                .iter()
                .enumerate()
                .filter(|(_, grant)| {
                    matches!(grant.grantable, Grantable::AlternativePrice { .. })
                        && grant_available(&query, player, grant)
                })
                .filter_map(|(index, grant)| selection(grant, index))
                .collect::<Vec<_>>();
            if prices.is_empty() {
                continue;
            }
            let mut origins = Vec::new();
            if ordinary_origin(game, player, spell, &face_method) {
                origins.push((face_method.clone(), None));
            }
            if !other_face && spell.owner == player {
                for (index, method) in spell.alternative_casts.iter().enumerate() {
                    if method.cast_from_zone() == spell.zone && method_allows_separate_price(method)
                    {
                        origins.push((CastingMethod::Alternative(index), None));
                    }
                }
            }
            for (index, grant) in grants.iter().enumerate() {
                if !grant_allows_separate_price(grant) || !grant_available(&query, player, grant) {
                    continue;
                }
                let Some(key) = selection(grant, index) else {
                    continue;
                };
                let origin = if other_face {
                    CastingMethod::SplitOtherHalfPlayFrom {
                        source: key.source,
                        zone: spell.zone,
                        use_alternative: None,
                    }
                } else {
                    CastingMethod::PlayFrom {
                        source: key.source,
                        zone: spell.zone,
                        use_alternative: None,
                    }
                };
                origins.push((origin, Some(key)));
            }
            for (origin, origin_permission) in origins {
                for price in &prices {
                    result.push(CastingMethod::AlternativePrice {
                        origin: Box::new(origin.clone()),
                        origin_permission: origin_permission.clone(),
                        price: price.clone(),
                        prototype,
                    });
                }
            }
        }
    }
    if ordinary_origin(game, player, spell, &CastingMethod::Fuse)
        && spell.zone == Zone::Hand
        && spell.has_fuse
    {
        let origin = CastingMethod::Fuse;
        if let Some(query) = face_query(game, spell, &origin, None)? {
            for (index, grant) in query
                .effect_store
                .grant_registry
                .get_grants_for_card(&query, spell.id, spell.zone, player)
                .iter()
                .enumerate()
            {
                if matches!(grant.grantable, Grantable::AlternativePrice { .. })
                    && grant_available(&query, player, grant)
                    && let Some(price) = selection(grant, index)
                {
                    result.push(CastingMethod::AlternativePrice {
                        origin: Box::new(origin.clone()),
                        origin_permission: None,
                        price,
                        prototype: None,
                    });
                }
            }
        }
    }
    Ok(result)
}

/// Origin semantics (including jump-start's departure replacement) remain
/// distinct from the price and never get synthesized as a different keyword.
pub(crate) fn origin_alternative(
    game: &GameState,
    player: PlayerId,
    spell: &Object,
    method: &CastingMethod,
) -> Option<AlternativeCastingMethod> {
    if spell.zone == Zone::Stack {
        return spell.cast_alternative_method_owned();
    }
    match resolve_announcement(game, player, spell, method) {
        Ok(route) => route.and_then(|route| route.origin_alternative),
        Err(error) => {
            game.record_token_resource_failure(&error);
            None
        }
    }
}

pub(crate) fn mana_cost(receipt: &PriceReceipt) -> crate::mana::ManaCost {
    receipt
        .total_cost
        .costs()
        .iter()
        .filter_map(crate::costs::Cost::mana_cost_ref)
        .fold(receipt.origin_mana_surcharge.clone(), |sum, cost| {
            crate::decision::add_mana_cost(&sum, cost)
        })
}

/// Price options for one exact spell/face already authorized by the resolving
/// instruction. They intentionally do not appear in priority enumeration.
pub(crate) fn effect_candidates(
    game: &GameState,
    player: PlayerId,
    spell: &Object,
    origin: &CastingMethod,
) -> Result<Vec<CastingMethod>, ExecutionError> {
    if !matches!(
        origin,
        CastingMethod::Normal
            | CastingMethod::SplitOtherHalf
            | CastingMethod::Fuse
            | CastingMethod::PlayFrom {
                use_alternative: None,
                ..
            }
            | CastingMethod::SplitOtherHalfPlayFrom {
                use_alternative: None,
                ..
            }
    ) {
        return Ok(Vec::new());
    }
    let mut result = Vec::new();
    for prototype in prototype_choices(game, spell, origin) {
        let Some(query) = face_query(game, spell, origin, prototype)? else {
            continue;
        };
        result.extend(
            query
                .effect_store
                .grant_registry
                .get_grants_for_card(&query, spell.id, spell.zone, player)
                .iter()
                .enumerate()
                .filter(|(_, grant)| {
                    matches!(grant.grantable, Grantable::AlternativePrice { .. })
                        && grant_available(&query, player, grant)
                })
                .filter_map(|(index, grant)| selection(grant, index))
                .map(|price| CastingMethod::AlternativePrice {
                    origin: Box::new(origin.clone()),
                    origin_permission: None,
                    price,
                    prototype,
                }),
        );
    }
    Ok(result)
}

pub(crate) fn origin_constraints_or_latch(
    game: &GameState,
    player: PlayerId,
    spell: &Object,
    method: &CastingMethod,
) -> Option<crate::grant_registry::PlayFromConstraints> {
    if spell.zone == Zone::Stack {
        return spell
            .cast_play_from_constraints
            .as_deref()
            .map(|(_, _, constraints)| constraints.clone());
    }
    match resolve_announcement(game, player, spell, method) {
        Ok(route) => route.and_then(|route| route.origin.map(|grant| grant.play_from_constraints)),
        Err(error) => {
            game.record_token_resource_failure(&error);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ordinary_effect_price_discovery_propagates_incomplete_continuous_state() {
        std::thread::Builder::new()
            .stack_size(128 * 1024 * 1024)
            .spawn(|| {
                let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
                let player = PlayerId(0);
                let mut model: crate::static_abilities::CompiledStaticAbility =
                    ironsmith_core::StaticAbility::haste();
                for _ in 0..140 {
                    model = ironsmith_core::StaticAbility::grant_object_ability_for_filter(
                        crate::target::ObjectFilter::source(),
                        ironsmith_core::Ability::static_ability(model),
                        "Finite child",
                    );
                }
                let card =
                    crate::card::CardBuilder::new(crate::ids::CardId::new(), "Discovery source")
                        .card_types(vec![crate::types::CardType::Artifact])
                        .build();
                let source = game.create_object_from_card(&card, player, Zone::Battlefield);
                game.object_mut(source).unwrap().abilities_mut().push(
                    crate::ability::Ability::static_ability(
                        crate::static_abilities::StaticAbility::from_model(model),
                    ),
                );
                let spell = game.create_object_from_card(&card, player, Zone::Hand);
                let before = game.player(player).unwrap().hand.clone();
                assert!(matches!(
                    effect_candidates(
                        &game,
                        player,
                        game.object(spell).unwrap(),
                        &CastingMethod::Normal
                    ),
                    Err(ExecutionError::ContinuousDiscovery(_))
                ));
                assert_eq!(game.player(player).unwrap().hand, before);
                assert_eq!(game.object(spell).unwrap().zone, Zone::Hand);
                assert!(game.turn_store.grant_cast_uses_this_turn.is_empty());
            })
            .unwrap()
            .join()
            .unwrap();
    }
}
