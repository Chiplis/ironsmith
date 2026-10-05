//! Announcement-time public identities needed to determine costs or targets.
//! This does not pay costs, reveal hidden cards, or manufacture unknown tags.
use crate::cost::{Cost, CostPaymentError, TotalCost};
use crate::effect::Effect;
use crate::effects::{ChooseObjectsEffect, ExecutionContext};
use crate::filter::ObjectFilterExt as _;
use crate::game_state::GameState;
use crate::snapshot::ObjectSnapshot;
use crate::tag::TagKey;
use crate::{ObjectId, PlayerId, Zone};
use ironsmith_core::tag::TagKeyWalk;
use std::collections::{HashMap, HashSet};

pub(crate) type CostReferenceBindings = HashMap<TagKey, Vec<ObjectSnapshot>>;

pub(crate) fn activation_reference_context(
    game: &GameState,
    source: ObjectId,
    ability_index: usize,
) -> CostReferenceBindings {
    let mut tags = HashMap::new();
    game.insert_granting_source_tag(source, ability_index, &mut tags);
    refresh_source_exiled_reference(game, source, &mut tags);
    tags
}

fn target_reference_tags(effect: &Effect, tags: &mut HashSet<TagKey>) {
    if let Some(spec) = effect.0.get_target_spec().filter(|spec| spec.is_target()) {
        spec.for_each_tag_key(&mut |tag| {
            tags.insert(tag.clone());
        });
    }
    effect.visit_child_effects(&mut |child| target_reference_tags(child, tags));
}

/// Only mandatory one-object choices from public zones can be announced
/// ahead of payment here. Optional or hidden selections retain their existing
/// payment/disclosure protocol rather than receiving speculative identities.
pub(crate) fn activation_reference_choices(
    cost: &TotalCost,
    effects: &[Effect],
) -> Result<Vec<ChooseObjectsEffect>, CostPaymentError> {
    let Some(components) = cost.as_all() else {
        return Ok(Vec::new());
    };
    let mut needed = HashSet::new();
    for effect in effects {
        target_reference_tags(effect, &mut needed);
    }
    for component in components {
        if let Some(reference) = component
            .dynamic_mana_cost_ref()
            .and_then(|cost| cost.mana_cost_of.as_deref())
        {
            reference.for_each_tag_key(&mut |tag| {
                needed.insert(tag.clone());
            });
        }
    }
    let mut result = Vec::new();
    for component in components {
        let Some(choose) = component
            .effect_ref()
            .and_then(|effect| effect.downcast_ref::<ChooseObjectsEffect>())
        else {
            continue;
        };
        if !needed.contains(&choose.tag) {
            continue;
        }
        let public_zone = matches!(
            choose.filter.zone.or(choose.zone),
            Some(Zone::Battlefield | Zone::Graveyard | Zone::Exile)
        );
        if !public_zone
            || choose.count != crate::ChoiceCount::exactly(1)
            || choose.count_value.is_some()
            || choose.aggregate_constraint.is_some()
            || choose.is_search
            || choose.top_only
            || choose.bottom_only
            || !choose.additional_zones.is_empty()
        {
            return Err(CostPaymentError::Other(
                "cost reference requires an unsupported announcement choice".into(),
            ));
        }
        result.push(choose.clone());
    }
    Ok(result)
}

pub(crate) fn public_reference_candidates(
    game: &GameState,
    source: ObjectId,
    payer: PlayerId,
    choice: &ChooseObjectsEffect,
    tags: &CostReferenceBindings,
    x_value: Option<u32>,
) -> Vec<ObjectId> {
    let Some(zone @ (Zone::Battlefield | Zone::Graveyard | Zone::Exile)) =
        choice.filter.zone.or(choice.zone)
    else {
        return Vec::new();
    };
    let mut ctx = ExecutionContext::new_default(source, payer).with_tagged_objects(tags.clone());
    ctx.x_value = x_value;
    let filter_ctx = ctx.filter_context(game);
    game.objects_in_zone(zone)
        .into_iter()
        .filter(|id| {
            game.object(*id).is_some_and(|object| {
                (zone == Zone::Battlefield || !game.hidden_identity_is_private(*id))
                    && (zone != Zone::Graveyard
                        || choice.filter.owner.is_some()
                        || object.owner == payer)
                    && choice.filter.matches(object, &filter_ctx, game)
            })
        })
        .collect()
}

/// Restrict each already-announced choice to its original current object.
/// Payment still checks its complete original filter and uses real movement
/// replacement processing. A stable-ID successor is never substituted.
pub(crate) fn lock_activation_reference_cost(
    game: &GameState,
    source: ObjectId,
    payer: PlayerId,
    cost: &TotalCost,
    tags: &CostReferenceBindings,
    selected: &CostReferenceBindings,
    x_value: Option<u32>,
) -> Result<TotalCost, CostPaymentError> {
    match cost.kind() {
        ironsmith_core::TotalCostKind::OneOf(branches) => Ok(TotalCost::one_of(
            branches
                .iter()
                .map(|branch| {
                    lock_activation_reference_cost(
                        game, source, payer, branch, tags, selected, x_value,
                    )
                })
                .collect::<Result<Vec<_>, _>>()?,
        )),
        ironsmith_core::TotalCostKind::All(components) => {
            let mut context =
                ExecutionContext::new_default(source, payer).with_tagged_objects(tags.clone());
            context.x_value = x_value;
            for (tag, snapshots) in selected {
                context
                    .tagged_objects
                    .insert(tag.clone(), snapshots.clone());
            }
            let mut locked = Vec::new();
            for component in components {
                if let Some(choose) = component
                    .effect_ref()
                    .and_then(|effect| effect.downcast_ref::<ChooseObjectsEffect>())
                    && let Some(snapshots) = selected.get(&choose.tag)
                {
                    let [snapshot] = snapshots.as_slice() else {
                        return Err(CostPaymentError::Other(
                            "cost reference needs one selected object".into(),
                        ));
                    };
                    if !public_reference_candidates(
                        game,
                        source,
                        payer,
                        choose,
                        &context.tagged_objects,
                        context.x_value,
                    )
                    .contains(&snapshot.object_id)
                    {
                        return Err(CostPaymentError::Other(
                            "announced cost object is no longer eligible".into(),
                        ));
                    }
                    let mut choose = choose.clone();
                    choose.filter.specific = Some(snapshot.object_id);
                    locked.push(Cost::validated_effect(Effect::new(choose)));
                } else if let Some(dynamic) = component
                    .dynamic_mana_cost_ref()
                    .filter(|cost| cost.mana_cost_of.is_some())
                {
                    locked.push(Cost::mana(
                        crate::special_actions::resolve_dynamic_mana_cost(
                            game,
                            dynamic,
                            &mut context,
                        )?,
                    ));
                } else {
                    locked.push(component.clone());
                }
            }
            Ok(TotalCost::from_costs(locked))
        }
    }
}

fn target_has_exact_mana_x(effect: &Effect) -> bool {
    fn spec_has_x(spec: &crate::ChooseSpec) -> bool {
        spec.is_target()
            && matches!(spec.base(), crate::ChooseSpec::Object(filter)
            if matches!(filter.mana_value.as_ref(), Some(crate::filter::Comparison::EqualExpr(value))
                if matches!(value.unhinted(), crate::Value::X)))
    }
    if effect.0.get_target_spec().is_some_and(spec_has_x) {
        return true;
    }
    let mut found = false;
    effect.visit_child_effects(&mut |child| {
        found |= target_has_exact_mana_x(child);
    });
    found
}

pub(crate) fn needs_activation_reference_context(cost: &TotalCost, effects: &[Effect]) -> bool {
    if effects.iter().any(target_has_exact_mana_x) {
        return true;
    }
    if let Some(branches) = cost.as_one_of() {
        return branches
            .iter()
            .any(|branch| needs_activation_reference_context(branch, effects));
    }
    let mut tags = HashSet::new();
    for effect in effects {
        target_reference_tags(effect, &mut tags);
    }
    for component in cost.costs() {
        if component
            .dynamic_mana_cost_ref()
            .is_some_and(|cost| cost.mana_cost_of.is_some())
        {
            return true;
        }
        if let Some(choose) = component
            .effect_ref()
            .and_then(|effect| effect.downcast_ref::<ChooseObjectsEffect>())
        {
            if tags.contains(&choose.tag) {
                return true;
            }
            choose.filter.for_each_tag_key(&mut |tag| {
                tags.insert(tag.clone());
            });
        }
    }
    tags.iter().any(|tag| {
        matches!(
            tag.as_str(),
            crate::tag::SOURCE_EXILED_TAG | crate::tag::GRANTING_SOURCE_TAG
        )
    })
}

/// Read-only existential preflight: every speculative public selection is
/// evaluated in its own reference context. No chosen identity escapes this
/// function and an unaffordable candidate cannot mask an affordable one.
pub(crate) fn activation_reference_preflight(
    game: &GameState,
    source: ObjectId,
    ability_index: usize,
    payer: PlayerId,
    activated: &crate::ability::ActivatedAbility,
) -> Option<bool> {
    let effects = activated.effects.flattened_default_effects();
    if !needs_activation_reference_context(&activated.mana_cost, effects) {
        return None;
    }
    if effects.iter().any(target_has_exact_mana_x)
        && game
            .object(source)
            .is_some_and(|object| object.x_value.is_none())
    {
        // Equality to a target's mana value has a finite complete set of X
        // witnesses. Price those announcements on isolated game copies; an
        // unresolved X is never accepted as a wildcard and the caller's
        // source/turn/hand state is never changed by availability checks.
        let mut values = std::collections::BTreeSet::from([0]);
        values.extend(
            game.objects_in_deterministic_order()
                .into_iter()
                .map(|object| crate::filter::object_current_mana_value(game, object.id)),
        );
        return Some(values.into_iter().filter(|x| *x >= activated.activation_x_minimum()).any(|x| {
            let mut preview = game.clone();
            let Some(object) = preview.object_mut(source) else { return false; };
            object.x_value = Some(x);
            let mut announced = activated.clone();
            announced.mana_cost = TotalCost::from_costs(activated.mana_cost.costs().iter().map(|component| {
                if let Some(mana) = component.mana_cost_ref() {
                    Cost::mana(crate::decision::mana_cost_with_locked_x_and_generic_reduction(mana, x, 0))
                } else { component.clone() }
            }).collect());
            activation_reference_preflight(&preview, source, ability_index, payer, &announced).unwrap_or(false)
        }));
    }
    let tags = activation_reference_context(game, source, ability_index);
    fn branch_payable(
        game: &GameState,
        source: ObjectId,
        payer: PlayerId,
        activated: &crate::ability::ActivatedAbility,
        cost: &TotalCost,
        tags: &CostReferenceBindings,
    ) -> bool {
        if let Some(branches) = cost.as_one_of() {
            return branches
                .iter()
                .any(|branch| branch_payable(game, source, payer, activated, branch, tags));
        }
        let Ok(choices) =
            activation_reference_choices(cost, activated.effects.flattened_default_effects())
        else {
            return false;
        };
        fn visit(
            game: &GameState,
            source: ObjectId,
            payer: PlayerId,
            activated: &crate::ability::ActivatedAbility,
            cost: &TotalCost,
            tags: &CostReferenceBindings,
            choices: &[ChooseObjectsEffect],
            selected: &CostReferenceBindings,
        ) -> bool {
            let mut context = tags.clone();
            context.extend(selected.clone());
            if let Some((choice, rest)) = choices.split_first() {
                return public_reference_candidates(game, source, payer, choice, &context, None)
                    .into_iter()
                    .any(|id| {
                        let Some(object) = game.object(id) else {
                            return false;
                        };
                        let mut selected = selected.clone();
                        selected.insert(
                            choice.tag.clone(),
                            vec![ObjectSnapshot::from_object_with_calculated_characteristics(
                                object, game,
                            )],
                        );
                        visit(game, source, payer, activated, cost, tags, rest, &selected)
                    });
            }
            let Ok(locked) = lock_activation_reference_cost(
                game,
                source,
                payer,
                cost,
                tags,
                selected,
                game.object(source).and_then(|object| object.x_value),
            ) else {
                return false;
            };
            let cost = crate::decision::calculate_effective_activation_total_cost_for_ability(
                game,
                payer,
                source,
                &locked,
                &[],
                Some(crate::decision::ActivationCostAbility::of(
                    game, payer, source, activated,
                )),
            );
            let Some(components) = cost.as_all() else {
                return false;
            };
            let view = crate::derived_view::DerivedGameView::new(game)
                .with_target_reference_bindings(context.clone());
            let reason = crate::costs::PaymentReason::ActivateAbility;
            let mut nonmana = Vec::new();
            let mut combined_mana = crate::mana::ManaCost::new();
            for component in components {
                if let Some(mana) = component.mana_cost_ref() {
                    combined_mana = crate::decision::add_mana_cost(&combined_mana, mana);
                } else {
                    nonmana.push(component.clone());
                }
            }
            // The ordinary price builder currently coalesces plain mana, but
            // preflight must not depend on that representation invariant:
            // two individually affordable components may share one resource.
            let mana = combined_mana;
            if !view.can_potentially_pay_with_reason(payer, Some(source), &mana, 0, reason) {
                return false;
            }
            let mut exec =
                ExecutionContext::new_default(source, payer).with_tagged_objects(context);
            exec.x_value = game.object(source).and_then(|object| object.x_value);
            crate::special_actions::can_pay_total_cost_with_reason_in_context(
                game,
                payer,
                source,
                &TotalCost::from_costs(nonmana),
                reason,
                &mut exec,
            )
            .is_ok()
                && view.spell_has_legal_targets(
                    activated.effects.flattened_default_effects(),
                    payer,
                    Some(source),
                    None,
                )
        }
        visit(
            game,
            source,
            payer,
            activated,
            cost,
            tags,
            &choices,
            &HashMap::new(),
        )
    }
    Some(branch_payable(
        game,
        source,
        payer,
        activated,
        &activated.mana_cost,
        &tags,
    ))
}

/// Pending announcements are outside compute_legal_actions' checked query.
/// Own a fresh result-bearing resource query for the whole branch check,
/// including the ordinary priced-branch fallback. Raw and priced branches
/// are distinct inputs: a captured raw branch is modified exactly once.
pub(crate) fn activation_branch_preflight_checked(
    game: &GameState,
    source: ObjectId,
    ability_index: usize,
    payer: PlayerId,
    raw_reference_branch: Option<&TotalCost>,
    priced_branch: &TotalCost,
) -> Result<bool, crate::effects::ExecutionError> {
    crate::decision::with_complete_legality_query(game, |checked| {
        let ability = checked.current_ability(source, ability_index);
        let activated = ability.as_ref().and_then(|ability| match &ability.kind {
            crate::ability::AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        });
        let effective;
        let branch = if let Some(raw) = raw_reference_branch {
            let Some(activated) = activated else {
                return Ok(false);
            };
            let mut reference_ability = activated.clone();
            reference_ability.mana_cost = raw.clone();
            if let Some(payable) = activation_reference_preflight(
                checked,
                source,
                ability_index,
                payer,
                &reference_ability,
            ) {
                return Ok(payable);
            }
            effective = crate::decision::calculate_effective_activation_total_cost_for_ability(
                checked,
                payer,
                source,
                raw,
                &[],
                Some(crate::decision::ActivationCostAbility::of(
                    checked, payer, source, activated,
                )),
            );
            &effective
        } else {
            priced_branch
        };
        let view = crate::derived_view::DerivedGameView::new(checked);
        Ok(
            crate::decision::activation_total_cost_branch_is_payable_with_view(
                checked, payer, source, branch, &view,
            ),
        )
    })
}

/// Linked-exile sets are live exact membership, unlike a chosen cost object's
/// saved LKI. Refresh only this well-defined source relation at payment entry.
pub(crate) fn refresh_source_exiled_reference(
    game: &GameState,
    source: ObjectId,
    tags: &mut CostReferenceBindings,
) {
    let exiled = game
        .get_exiled_with_source_links(source)
        .iter()
        .filter_map(|id| game.object(*id))
        .filter(|object| object.zone == Zone::Exile)
        .map(|object| ObjectSnapshot::from_object_with_calculated_characteristics(object, game))
        .collect::<Vec<_>>();
    tags.remove(crate::tag::SOURCE_EXILED_TAG);
    if !exiled.is_empty() {
        tags.insert(crate::tag::SOURCE_EXILED_TAG.into(), exiled);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter};
    use crate::{CardId, CardType};
    #[test]
    fn reference_price_retains_symbols_and_rejects_a_later_graveyard_incarnation_without_mutation()
    {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let payer = PlayerId(0);
        let definition = CardBuilder::new(CardId::new(), "Reference creature")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Blue, ManaSymbol::Green],
                vec![ManaSymbol::Black, ManaSymbol::Life(2)],
                vec![ManaSymbol::X],
            ]))
            .build();
        let object = game.create_object_from_card(&definition, payer, Zone::Graveyard);
        let source = game.create_object_from_card(&definition, payer, Zone::Battlefield);
        let tag: TagKey = "priced_cost_object".into();
        let choice = ChooseObjectsEffect::new(
            ObjectFilter::creature()
                .in_zone(Zone::Graveyard)
                .owned_by(PlayerFilter::You),
            1,
            PlayerFilter::You,
            tag.clone(),
        );
        let cost = TotalCost::from_costs(vec![
            Cost::validated_effect(Effect::new(choice)),
            Cost::validated_effect(Effect::exile(ChooseSpec::tagged(tag.clone()))),
            Cost::dynamic_mana(ironsmith_core::DynamicManaCost::from_object_mana_cost(
                ChooseSpec::tagged(tag.clone()),
            )),
        ]);
        let selected = HashMap::from([(
            tag,
            vec![ObjectSnapshot::from_object(
                game.object(object).unwrap(),
                &game,
            )],
        )]);
        let locked = lock_activation_reference_cost(
            &game,
            source,
            payer,
            &cost,
            &HashMap::new(),
            &selected,
            None,
        )
        .unwrap();
        let mana = locked
            .costs()
            .iter()
            .find_map(|cost| cost.mana_cost_ref())
            .unwrap();
        assert_eq!(
            mana.pips(),
            &[
                vec![ManaSymbol::Blue, ManaSymbol::Green],
                vec![ManaSymbol::Black, ManaSymbol::Life(2)]
            ]
        );
        let moved = game.move_object_by_effect(object, Zone::Hand).unwrap();
        let returned = game.move_object_by_effect(moved, Zone::Graveyard).unwrap();
        assert_ne!(returned, object);
        let graveyard = game.player(payer).unwrap().graveyard.clone();
        let life = game.player(payer).unwrap().life;
        let pool = game.player(payer).unwrap().mana_pool.clone();
        assert!(
            lock_activation_reference_cost(
                &game,
                source,
                payer,
                &cost,
                &HashMap::new(),
                &selected,
                None
            )
            .is_err()
        );
        assert_eq!(game.player(payer).unwrap().graveyard, graveyard);
        assert_eq!(game.player(payer).unwrap().life, life);
        assert_eq!(game.player(payer).unwrap().mana_pool, pool);
        let mut context = ExecutionContext::new_default(source, payer);
        assert!(
            crate::special_actions::resolve_dynamic_mana_cost(
                &game,
                &ironsmith_core::DynamicManaCost::from_object_mana_cost(ChooseSpec::tagged(
                    "missing"
                )),
                &mut context
            )
            .is_err()
        );
    }
    #[test]
    fn pending_branch_queries_propagate_resource_failures_for_reference_and_plain_branches() {
        for referenced in [false, true] {
            let mut game = crate::tests::test_helpers::setup_two_player_game();
            let payer = PlayerId(0);
            game.turn.active_player = payer;
            game.turn.priority_player = Some(payer);
            game.turn.phase = crate::game_state::Phase::FirstMain;
            game.turn.step = None;
            let artifact = CardBuilder::new(CardId::new(), "Branch source")
                .card_types(vec![CardType::Artifact])
                .build();
            let source = game.create_object_from_card(&artifact, payer, Zone::Battlefield);
            let card = CardBuilder::new(CardId::new(), "Cost reference")
                .card_types(vec![CardType::Creature])
                .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Green]))
                .build();
            let card = game.create_object_from_card(&card, payer, Zone::Graveyard);
            let branch = if referenced {
                TotalCost::from_cost(Cost::dynamic_mana(
                    ironsmith_core::DynamicManaCost::from_object_mana_cost(
                        ChooseSpec::SpecificObject(card),
                    ),
                ))
            } else {
                TotalCost::mana(ManaCost::from_symbols(vec![ManaSymbol::Green]))
            };
            game.object_mut(source).unwrap().abilities_mut().push(
                crate::ability::Ability::activated(branch.clone(), vec![Effect::gain_life(1)]),
            );
            let land = CardBuilder::new(CardId::new(), "Resource-limited mana")
                .card_types(vec![CardType::Land])
                .build();
            let land = game.create_object_from_card(&land, payer, Zone::Battlefield);
            game.object_mut(land)
                .unwrap()
                .abilities_mut()
                .push(crate::ability::Ability::mana(
                    TotalCost::from_cost(Cost::tap()),
                    vec![ManaSymbol::Green],
                ));
            game.effect_store.replacement_effects.add_resolution_effect(
                crate::replacement::ReplacementEffect::with_matcher(
                    land,
                    payer,
                    crate::events::mana::matchers::ManaProducedBySourceMatcher::new(
                        ObjectFilter::specific(land),
                    ),
                    crate::replacement::ReplacementAction::Additionally(vec![Effect::new(
                        crate::effects::CreateTokenEffect::you(
                            crate::cards::tokens::treasure_token_definition(),
                            2,
                        ),
                    )]),
                ),
            );
            game.set_token_creation_limits(crate::effects::tokens::TokenCreationLimits {
                max_created_tokens: 1,
                ..Default::default()
            });
            let objects = game.next_object_id_counter();
            let result = activation_branch_preflight_checked(
                &game,
                source,
                0,
                payer,
                referenced.then_some(&branch),
                &branch,
            );
            assert!(
                matches!(
                    result,
                    Err(crate::effects::ExecutionError::ResourceLimitExceeded { .. })
                ),
                "resource exhaustion must not become a disabled/unpayable option: {result:?}"
            );
            assert!(!game.is_tapped(land));
            assert_eq!(game.battlefield.len(), 2);
            assert_eq!(game.next_object_id_counter(), objects);
            assert_eq!(game.object(card).unwrap().zone, Zone::Graveyard);
            assert_eq!(game.player(payer).unwrap().mana_pool.total(), 0);
        }
    }
}
