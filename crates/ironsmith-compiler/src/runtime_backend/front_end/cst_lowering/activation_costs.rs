use crate::cards::builders::{CardTextError, ChoiceCount};
use crate::cost::TotalCost;
use crate::costs::Cost;
use crate::effect::Effect;
use crate::filter::ObjectFilter;
use crate::mana::{ManaCost, ManaSymbol};
use crate::object::CounterType;
use crate::target::PlayerFilter;
use crate::types::CardType;

use crate::runtime_backend::grammar::activation_costs::{
    ActivationCostCst, ActivationCostSegmentCst,
};

fn apply_activation_cost_default_battlefield_scope(filter: &mut ObjectFilter) {
    if !filter.any_of.is_empty() {
        for arm in &mut filter.any_of {
            apply_activation_cost_default_battlefield_scope(arm);
        }
        return;
    }
    if filter.controller.is_none() && filter.owner.is_none() {
        filter.controller = Some(PlayerFilter::You);
    }
    if filter.zone.is_none() {
        filter.zone = Some(crate::zone::Zone::Battlefield);
    }
}

pub(crate) fn lower_activation_cost_cst(
    cst: &ActivationCostCst,
) -> Result<TotalCost, CardTextError> {
    if !cst.alternative_branches.is_empty() {
        let mut modes = Vec::with_capacity(cst.alternative_branches.len());
        for branch in &cst.alternative_branches {
            let total = lower_activation_cost_cst(branch)?;
            modes.push(crate::effect::EffectMode::new(
                branch.raw.clone(),
                crate::costs::total_cost_to_payment_effects(&total),
            ));
        }
        return Ok(TotalCost::from_cost(Cost::validated_effect(
            Effect::choose_one(modes),
        )));
    }

    fn flush_pending_mana(costs: &mut Vec<Cost>, pending: &mut Vec<Vec<ManaSymbol>>) {
        if pending.is_empty() {
            return;
        }
        costs.push(Cost::mana(ManaCost::from_pips(std::mem::take(pending))));
    }

    let mut costs = Vec::new();
    let mut pending_mana_pips = Vec::new();
    let mut tap_tag_id = 0usize;
    let mut sacrifice_tag_id = 0usize;
    let mut exile_tag_id = 0usize;
    let mut return_tag_id = 0usize;
    for segment in &cst.segments {
        match segment {
            ActivationCostSegmentCst::Mana(cost) => {
                pending_mana_pips.extend(cost.pips().to_vec());
            }
            ActivationCostSegmentCst::Tap => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::tap());
            }
            ActivationCostSegmentCst::TapChosen { count, filter } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                let mut filter = filter.clone();
                apply_activation_cost_default_battlefield_scope(&mut filter);
                filter.untapped = true;
                let tag = format!("tap_cost_{tap_tag_id}");
                tap_tag_id += 1;
                costs.push(Cost::validated_effect(Effect::choose_objects(
                    filter,
                    ChoiceCount::exactly(*count as usize),
                    PlayerFilter::You,
                    tag.clone(),
                )));
                costs.push(Cost::validated_effect(Effect::tap(
                    crate::target::ChooseSpec::tagged(tag),
                )));
            }
            ActivationCostSegmentCst::Untap => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::untap());
            }
            ActivationCostSegmentCst::Life(amount) => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                if matches!(amount, crate::effect::Value::Fixed(_)) {
                    costs.push(Cost::life(amount.clone()));
                } else {
                    costs.push(Cost::validated_effect(Effect::lose_life_player(
                        amount.clone(),
                        PlayerFilter::You,
                    )));
                }
            }
            ActivationCostSegmentCst::Energy(amount) => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::energy(*amount));
            }
            ActivationCostSegmentCst::DiscardSource => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::discard_source());
            }
            ActivationCostSegmentCst::DiscardHand => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::discard_hand());
            }
            ActivationCostSegmentCst::DiscardCard(count) => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::discard(*count, None));
            }
            ActivationCostSegmentCst::DiscardFiltered {
                count,
                card_types,
                supertypes,
                filter,
                random,
                name,
                other,
            } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                if *random || name.is_some() || *other || filter.is_some() || !supertypes.is_empty()
                {
                    let card_filter = if let Some(filter) = filter {
                        Some(filter.clone())
                    } else if card_types.is_empty()
                        && supertypes.is_empty()
                        && name.is_none()
                        && !*other
                    {
                        None
                    } else {
                        let mut filter = ObjectFilter {
                            zone: Some(crate::zone::Zone::Hand),
                            card_types: card_types.clone(),
                            supertypes: supertypes.clone(),
                            ..Default::default()
                        };
                        if let Some(name) = name {
                            filter = filter.named(name.clone());
                        }
                        if *other {
                            filter.other = true;
                        }
                        Some(filter)
                    };
                    costs.push(Cost::validated_effect(Effect::discard_player_filtered(
                        *count as i32,
                        PlayerFilter::You,
                        *random,
                        card_filter,
                    )));
                } else if card_types.len() > 1 {
                    costs.push(Cost::discard_types(*count, card_types.clone()));
                } else if let Some(card_type) = card_types.first().copied() {
                    costs.push(Cost::discard(*count, Some(card_type)));
                } else {
                    costs.push(Cost::discard(*count, None));
                }
            }
            ActivationCostSegmentCst::Mill(count) => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::mill(*count));
            }
            ActivationCostSegmentCst::Behold { subtype, count } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::validated_effect(Effect::behold(*subtype, *count)));
            }
            ActivationCostSegmentCst::Blight { count } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                let tag = format!("blight_cost_{tap_tag_id}");
                tap_tag_id += 1;
                costs.push(Cost::validated_effect(Effect::choose_objects(
                    ObjectFilter::creature().you_control(),
                    ChoiceCount::exactly(1),
                    PlayerFilter::You,
                    tag.clone(),
                )));
                costs.push(Cost::validated_effect(Effect::put_counters(
                    CounterType::MinusOneMinusOne,
                    *count as i32,
                    crate::target::ChooseSpec::tagged(tag),
                )));
            }
            ActivationCostSegmentCst::SacrificeSelf => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::sacrifice_self());
            }
            ActivationCostSegmentCst::SacrificeCreature => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                let tag = format!("sacrifice_cost_{sacrifice_tag_id}");
                sacrifice_tag_id += 1;
                costs.push(Cost::validated_effect(Effect::choose_objects(
                    ObjectFilter::creature().you_control(),
                    ChoiceCount::exactly(1),
                    PlayerFilter::You,
                    tag.clone(),
                )));
                costs.push(Cost::validated_effect(Effect::sacrifice(
                    ObjectFilter::tagged(tag),
                    1,
                )));
            }
            ActivationCostSegmentCst::SacrificeChosen { count, filter } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                let mut filter = filter.clone();
                if filter.controller.is_none() {
                    filter.controller = Some(PlayerFilter::You);
                }
                let tag = format!("sacrifice_cost_{sacrifice_tag_id}");
                sacrifice_tag_id += 1;
                costs.push(Cost::validated_effect(Effect::choose_objects(
                    filter,
                    count.clone(),
                    PlayerFilter::You,
                    tag.clone(),
                )));
                let exact_count =
                    (!count.dynamic_x && count.max == Some(count.min)).then_some(count.min as u32);
                let sacrifice = if let Some(exact_count) = exact_count {
                    Effect::sacrifice(ObjectFilter::tagged(tag), exact_count)
                } else {
                    Effect::sacrifice_player(
                        ObjectFilter::tagged(tag.clone()),
                        crate::effect::Value::Count(ObjectFilter::tagged(tag)),
                        PlayerFilter::You,
                    )
                };
                costs.push(Cost::validated_effect(sacrifice));
            }
            ActivationCostSegmentCst::UnattachChosen { count, filter } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                let mut filter = filter.clone();
                if filter.zone.is_none() {
                    filter.zone = Some(crate::zone::Zone::Battlefield);
                }
                let tag = format!("unattach_cost_{return_tag_id}");
                return_tag_id += 1;
                costs.push(Cost::validated_effect(Effect::choose_objects(
                    filter,
                    ChoiceCount::exactly(*count as usize),
                    PlayerFilter::You,
                    tag.clone(),
                )));
                costs.push(Cost::validated_effect(Effect::unattach_objects(
                    crate::target::ChooseSpec::tagged(tag),
                )));
            }
            ActivationCostSegmentCst::ExileSelf => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::exile_self());
            }
            ActivationCostSegmentCst::ExileSelfFromGraveyard => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::exile_self());
            }
            ActivationCostSegmentCst::ExileFromHand {
                count,
                color_filter,
            } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::exile_from_hand(*count, *color_filter));
            }
            ActivationCostSegmentCst::ExileChosen {
                choice_count,
                filter,
            } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                let mut filter = filter.clone();
                if filter.zone.is_none() {
                    filter.zone = Some(crate::zone::Zone::Battlefield);
                }
                if filter.zone == Some(crate::zone::Zone::Battlefield)
                    && filter.controller.is_none()
                {
                    filter.controller = Some(PlayerFilter::You);
                }
                let tag = format!("exile_cost_{exile_tag_id}");
                exile_tag_id += 1;
                costs.push(Cost::validated_effect(Effect::choose_objects(
                    filter,
                    *choice_count,
                    PlayerFilter::You,
                    tag.clone(),
                )));
                costs.push(Cost::validated_effect(Effect::exile(
                    crate::target::ChooseSpec::tagged(tag),
                )));
            }
            ActivationCostSegmentCst::ExileSelfAndNamedArtifacts { names } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::exile_self());
                for name in names {
                    let tag = format!("exile_cost_{exile_tag_id}");
                    exile_tag_id += 1;
                    let mut filter = ObjectFilter {
                        zone: Some(crate::zone::Zone::Battlefield),
                        controller: Some(PlayerFilter::You),
                        card_types: vec![CardType::Artifact],
                        ..Default::default()
                    };
                    filter.name = Some(name.clone());
                    costs.push(Cost::validated_effect(Effect::choose_objects(
                        filter,
                        ChoiceCount::exactly(1),
                        PlayerFilter::You,
                        tag.clone(),
                    )));
                    costs.push(Cost::validated_effect(Effect::exile(
                        crate::target::ChooseSpec::tagged(tag),
                    )));
                }
            }
            ActivationCostSegmentCst::ExileTopLibrary { count } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                #[cfg(not(feature = "serialization"))]
                costs.push(Cost::validated_effect(Effect::exile_top_of_library_player(
                    *count as i32,
                    PlayerFilter::You,
                    crate::tag::TagKey::from("__cost_exiled_top__"),
                    None,
                )));
                #[cfg(feature = "serialization")]
                costs.push(Cost::validated_effect(Effect::exile_top_of_library_player(
                    *count as i32,
                    PlayerFilter::You,
                )));
            }
            ActivationCostSegmentCst::RevealSourceFromHand => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::effect(Effect::reveal_source_from_hand()));
            }
            ActivationCostSegmentCst::RevealFromHand {
                count,
                color_filter,
                card_type,
            } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::effect(Effect::reveal_from_hand(
                    count.clone(),
                    *card_type,
                    *color_filter,
                )));
            }
            ActivationCostSegmentCst::ReturnSelfToHand => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::return_self_to_hand());
            }
            ActivationCostSegmentCst::ReturnChosenToHand { count, filter } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                let mut filter = filter.clone();
                if filter.controller.is_none() {
                    filter.controller = Some(PlayerFilter::You);
                }
                if filter.zone.is_none() {
                    filter.zone = Some(crate::zone::Zone::Battlefield);
                }
                let tag = format!("return_cost_{return_tag_id}");
                return_tag_id += 1;
                costs.push(Cost::validated_effect(Effect::choose_objects(
                    filter,
                    ChoiceCount::exactly(*count as usize),
                    PlayerFilter::You,
                    tag.clone(),
                )));
                costs.push(Cost::validated_effect(Effect::return_to_hand(
                    ObjectFilter::tagged(tag),
                )));
            }
            ActivationCostSegmentCst::MoveOpponentOwnedExiledCardToGraveyard => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                let tag = format!("graveyard_cost_{return_tag_id}");
                return_tag_id += 1;
                let filter = ObjectFilter {
                    zone: Some(crate::zone::Zone::Exile),
                    owner: Some(PlayerFilter::Opponent),
                    ..Default::default()
                };
                costs.push(Cost::validated_effect(Effect::choose_objects(
                    filter,
                    ChoiceCount::exactly(1),
                    PlayerFilter::You,
                    tag.clone(),
                )));
                costs.push(Cost::validated_effect(Effect::move_to_zone(
                    crate::target::ChooseSpec::tagged(tag),
                    crate::zone::Zone::Graveyard,
                    false,
                )));
            }
            ActivationCostSegmentCst::ExertSelf { display_text } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::effect(crate::effects::ExertCostEffect::new(
                    display_text.clone(),
                )));
            }
            ActivationCostSegmentCst::PutCounters {
                counter_type,
                count,
            } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::add_counters(*counter_type, *count));
            }
            ActivationCostSegmentCst::PutCountersChosen {
                counter_type,
                count,
                filter,
                source_equivalent,
            } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                if *source_equivalent {
                    costs.push(Cost::add_counters(*counter_type, *count));
                    continue;
                }
                let mut filter = filter.clone();
                apply_activation_cost_default_battlefield_scope(&mut filter);
                if filter.source {
                    costs.push(Cost::add_counters(*counter_type, *count));
                    continue;
                }
                let tag = format!("put_counter_cost_{tap_tag_id}");
                tap_tag_id += 1;
                costs.push(Cost::validated_effect(Effect::choose_objects(
                    filter,
                    ChoiceCount::exactly(1),
                    PlayerFilter::You,
                    tag.clone(),
                )));
                costs.push(Cost::validated_effect(Effect::put_counters(
                    *counter_type,
                    *count as i32,
                    crate::target::ChooseSpec::tagged(tag),
                )));
            }
            ActivationCostSegmentCst::RemoveCounters {
                counter_type,
                count,
            } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::remove_counters(*counter_type, *count));
            }
            ActivationCostSegmentCst::RemoveCountersAmong {
                counter_type,
                count,
                filter,
                display_x,
                dynamic,
            } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                let mut filter = filter.clone();
                apply_activation_cost_default_battlefield_scope(&mut filter);
                let effect = if *dynamic {
                    Effect::remove_dynamic_counters_among(
                        *count,
                        u32::MAX / 4,
                        filter,
                        *counter_type,
                        *display_x,
                    )
                } else {
                    Effect::remove_any_counters_among(*count, filter, *counter_type)
                };
                costs.push(Cost::validated_effect(effect));
            }
            ActivationCostSegmentCst::RemoveCountersDynamic {
                counter_type,
                display_x,
                remove_all,
            } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                let cost = if *remove_all {
                    Cost::remove_all_counters_from_source(*counter_type)
                } else {
                    Cost::remove_any_counters_from_source(*counter_type, *display_x)
                };
                costs.push(cost);
            }
        }
    }
    flush_pending_mana(&mut costs, &mut pending_mana_pips);
    Ok(TotalCost::from_costs(costs))
}
