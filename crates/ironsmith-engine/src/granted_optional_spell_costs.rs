//! Source-owned optional additional casting payments. Acquisition precedes
//! payment; consequence timing is explicit and never inferred from display.
use crate::filter::ObjectFilterExt as _;
use crate::{
    game_state::GameState,
    ids::{ObjectId, PlayerId},
    zone::Zone,
};
use ironsmith_core::StaticAbilityPayload;
use ironsmith_core::static_ability_model::{GrantedOptionalCostPrice, GrantedOptionalCostTiming};
#[path = "granted_optional_spell_costs/recipe_references.rs"]
mod recipe_references;

pub(crate) fn acquire_for_cast(game: &mut GameState, spell: ObjectId, caster: PlayerId) {
    let Some(mut candidate) = game.object(spell).cloned() else {
        return;
    };
    if candidate.zone != Zone::Stack {
        return;
    }
    if let Some(chars) = game.current_characteristics(spell) {
        candidate.name = chars.name;
        candidate.card_types = chars.card_types;
        candidate.subtypes = chars.subtypes;
        candidate.supertypes = chars.supertypes;
        candidate.color_override = Some(chars.colors);
    }
    let mut grants = Vec::new();
    let mut used_ids = None;
    let view = crate::derived_view::DerivedGameView::new(game);
    // Derived static lists enforce each source's functional zones, including
    // command-zone emblems and recipient-local stack policies.
    for source in game
        .objects_in_deterministic_order()
        .into_iter()
        .map(|object| object.id)
        .filter(|source| !game.is_phased_out(*source))
    {
        let Some(object) = game.object(source) else {
            continue;
        };
        let Some(abilities) = view.static_abilities_rc(source) else {
            continue;
        };
        for (ordinal, ability) in abilities.iter().enumerate() {
            if !ability.is_active(game, source) {
                continue;
            }
            let Some(model) = ability.compiled_model() else {
                continue;
            };
            let payload = match &model.payload {
                StaticAbilityPayload::Conditional { ability, .. } => &ability.payload,
                payload => payload,
            };
            let StaticAbilityPayload::GrantOptionalSpellCost {
                filter,
                price,
                reference,
                effects,
                timing,
                generic_reduction,
                local_object_references,
                display,
            } = payload
            else {
                continue;
            };
            if reference
                .discriminator
                .as_deref()
                .is_some_and(|value| value.starts_with("granted-payment-"))
            {
                continue;
            }
            let context = game
                .filter_context_for(game.controller_of(object), Some(source))
                .with_caster(Some(caster))
                .with_prospective_cast(spell);
            let mut filter = filter.clone();
            filter.zone = None;
            if !filter.matches_non_recursive(&candidate, &context, game) {
                continue;
            }
            let mut reference = reference.clone();
            reference.discriminator = Some(format!(
                "granted-payment-{}-{ordinal}-{:?}-{}",
                object.stable_id.0.0,
                ability.instance_id(),
                reference.discriminator.as_deref().unwrap_or("")
            ));
            if used_ids.is_none() {
                used_ids = match recipe_references::used_effect_ids(game, &candidate) {
                    Ok(ids) => Some(ids),
                    Err(error) => {
                        game.record_token_resource_failure(
                            &crate::effects::ExecutionError::IncompleteEvidence(error),
                        );
                        return;
                    }
                };
            }
            let scope = reference
                .discriminator
                .as_deref()
                .expect("assigned occurrence identity");
            let (price, effects, reduction, locals) = match recipe_references::acquire(
                price,
                effects,
                generic_reduction,
                local_object_references,
                scope,
                used_ids
                    .as_mut()
                    .expect("initialized applicable recipe inventory"),
            ) {
                Ok(recipe) => recipe,
                Err(error) => {
                    game.record_token_resource_failure(
                        &crate::effects::ExecutionError::IncompleteEvidence(error),
                    );
                    return;
                }
            };
            let mut marker = crate::static_abilities::StaticAbility::grant_optional_spell_cost(
                crate::target::ObjectFilter::source(),
                price,
                reference,
                effects,
                *timing,
                display.clone(),
            )
            .with_optional_spell_cost_local_references(locals);
            if let Some(amount) = reduction {
                marker = marker.with_optional_spell_cost_reduction(amount);
            }
            grants.push(marker);
        }
    }
    drop(view);
    for grant in grants {
        let reduction = match grant.compiled_model().map(|model| &model.payload) {
            Some(StaticAbilityPayload::GrantOptionalSpellCost {
                generic_reduction, ..
            }) => generic_reduction.clone(),
            _ => None,
        };
        game.grant_incarnation_static_ability(spell, grant);
        if let Some(amount) = reduction {
            game.grant_incarnation_static_ability(
                spell,
                crate::static_abilities::StaticAbility::new(
                    crate::static_abilities::ThisSpellCostReduction::new(
                        amount,
                        crate::static_abilities::ThisSpellCostCondition::Always,
                    ),
                ),
            );
        }
    }
}

fn acquired(game: &GameState, spell: ObjectId) -> Vec<crate::static_abilities::StaticAbility> {
    let view = crate::derived_view::DerivedGameView::new(game);
    view.static_abilities_rc(spell).map(|abilities| abilities.iter().filter(|ability|
        matches!(ability.compiled_model().map(|model| &model.payload),
            Some(StaticAbilityPayload::GrantOptionalSpellCost { reference, .. })
                if reference.discriminator.as_deref().is_some_and(|value| value.starts_with("granted-payment-"))))
        .cloned().collect()).unwrap_or_default()
}

/// Announce ordinary optional costs, including X in a recipient mana-value
/// recipe. Keeping each original X pip makes the price follow the same X
/// announcement without reading an unannounced value as zero.
pub(crate) fn ensure_optional_costs(game: &mut GameState, spell: ObjectId) -> bool {
    let grants = acquired(game, spell);
    let Some(object) = game.object(spell) else {
        return false;
    };
    let mut additions = Vec::new();
    for grant in grants {
        let Some(model) = grant.compiled_model() else {
            continue;
        };
        let StaticAbilityPayload::GrantOptionalSpellCost {
            price,
            reference,
            effects,
            timing,
            display,
            ..
        } = &model.payload
        else {
            continue;
        };
        if object
            .optional_costs
            .iter()
            .any(|cost| cost.reference == *reference)
        {
            continue;
        }
        let price = match price {
            GrantedOptionalCostPrice::Fixed(price) => price.clone(),
            GrantedOptionalCostPrice::GenericManaValue => {
                let base = object.mana_cost.as_deref();
                let mut pips = Vec::new();
                if let Some(base) = base {
                    pips.extend(
                        base.pips()
                            .iter()
                            .filter(|pip| pip.as_slice() == [crate::mana::ManaSymbol::X])
                            .cloned(),
                    );
                }
                crate::cost::TotalCost::mana(
                    crate::mana::ManaCost::from_pips(pips)
                        .add_generic(base.map_or(0, |cost| cost.mana_value())),
                )
            }
        };
        let mut cost = crate::cost::OptionalCost::custom(reference.display_label(), price);
        cost.kind = reference.kind.clone();
        cost.reference = reference.clone();
        let trigger = (*timing == GrantedOptionalCostTiming::CastTrigger).then(|| {
            let ability = crate::ability::Ability {
                kind: crate::ability::AbilityKind::Triggered(crate::ability::TriggeredAbility {
                    trigger: crate::triggers::Trigger::you_cast_this_spell(),
                    effects: crate::resolution::ResolutionProgram::from_effects(effects.clone()),
                    choices: vec![],
                    intervening_if: Some(crate::ConditionExpr::ThisSpellPaidLabel(
                        reference.clone(),
                    )),
                    presentation_label: None,
                }),
                functional_zones: vec![Zone::Stack],
                // The acquired recipe's explicit source attribution is proof;
                // its display label/discriminator and current source keywords are not.
                designations: reference.custom_keyword.iter().cloned().collect(),
            };
            crate::static_abilities::StaticAbility::grant_object_ability_for_filter(
                crate::target::ObjectFilter::source(),
                ability,
                display.clone(),
            )
        });
        additions.push((cost, trigger));
    }
    let changed = !additions.is_empty();
    for (cost, trigger) in additions {
        if let Some(object) = game.object_mut(spell) {
            object.optional_costs.push(cost);
        }
        if let Some(trigger) = trigger {
            game.grant_incarnation_static_ability(spell, trigger);
        }
    }
    changed
}

pub(crate) fn casting_programs(
    game: &GameState,
    spell: ObjectId,
) -> Vec<(
    crate::cost::OptionalCostRef,
    crate::resolution::ResolutionProgram,
)> {
    acquired(game, spell)
        .into_iter()
        .filter_map(|ability| {
            let model = ability.compiled_model()?;
            let StaticAbilityPayload::GrantOptionalSpellCost {
                reference,
                effects,
                timing: GrantedOptionalCostTiming::DuringCasting,
                ..
            } = &model.payload
            else {
                return None;
            };
            Some((
                reference.clone(),
                crate::resolution::ResolutionProgram::from_effects(effects.clone()),
            ))
        })
        .collect()
}
/// Affordability previews acquire exactly the same source recipes as the
/// committing announcement. The hypothetical recipient is never published.
fn preview_for_cast(
    game: &GameState,
    spell: ObjectId,
    caster: PlayerId,
) -> Option<(GameState, crate::object::Object)> {
    let original = game.object(spell)?;
    if original.zone == Zone::Stack {
        return None;
    }
    let view = crate::derived_view::DerivedGameView::new(game);
    let has_provider = game
        .objects_in_deterministic_order()
        .into_iter()
        .map(|object| object.id)
        .filter(|source| !game.is_phased_out(*source))
        .any(|source| {
            view.static_abilities_rc(source).is_some_and(|abilities| {
                abilities.iter().any(|ability| {
                    let Some(model) = ability.compiled_model() else {
                        return false;
                    };
                    let payload = match &model.payload {
                        StaticAbilityPayload::Conditional { ability, .. } => &ability.payload,
                        payload => payload,
                    };
                    matches!(payload, StaticAbilityPayload::GrantOptionalSpellCost { .. })
                })
            })
        });
    if !has_provider {
        return None;
    }
    drop(view);
    let mut working = game.clone();
    {
        let candidate = working.object_mut(spell)?;
        candidate.zone = Zone::Stack;
        candidate.initial_controller = caster;
    }
    acquire_for_cast(&mut working, spell, caster);
    if !ensure_optional_costs(&mut working, spell) {
        return None;
    }
    if let Err(error) = working.refresh_continuous_state() {
        game.record_token_resource_failure(&crate::effects::ExecutionError::ContinuousDiscovery(
            error,
        ));
        return None;
    }
    let candidate = working.object(spell)?.clone();
    Some((working, candidate))
}

/// Evaluate the complete private proposal inside the isolated ID scope. The
/// projected state is dropped before the boolean escapes this boundary.
pub(crate) fn with_cast_preview(
    game: &GameState,
    spell: &crate::object::Object,
    caster: PlayerId,
    inspect: impl FnOnce(&GameState, &crate::object::Object) -> bool,
) -> bool {
    crate::static_abilities::with_static_ability_preview(|| {
        let projected = preview_for_cast(game, spell.id, caster);
        let game = projected.as_ref().map_or(game, |(game, _)| game);
        let spell = projected.as_ref().map_or(spell, |(_, spell)| spell);
        inspect(game, spell)
    })
}

#[cfg(test)]
mod preview_identity_tests {
    use super::*;
    use crate::{
        ability::Ability,
        cards::builders::CardDefinitionBuilder,
        ids::CardId,
        static_abilities::{StaticAbility, committed_static_allocations_on_thread},
        types::CardType,
    };
    // Test-only complete native definition diagnostics. The assertion reads
    // immutable payloads; it never reparses text or reconstructs execution.
    fn native_definition_snapshots(game: &GameState) -> Vec<(ObjectId, Zone, String)> {
        game.objects_in_deterministic_order().into_iter().map(|object|
            (object.id, object.zone, format!("{:?}", object.to_card_definition()))
        ).collect()
    }
    #[test]
    fn repeated_uncommitted_recipes_preserve_native_state_provider_identity_and_later_commit() {
        let caster = PlayerId(0);
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let mut reference =
            crate::cost::OptionalCostRef::new(crate::cost::OptionalCostKind::Additional);
        reference.custom_keyword = Some("independent price".into());
        let policy = StaticAbility::grant_optional_spell_cost(
            crate::target::ObjectFilter::enchantment().you_control(),
            GrantedOptionalCostPrice::Fixed(crate::cost::TotalCost::free()),
            reference,
            vec![],
            GrantedOptionalCostTiming::DuringCasting,
            "optional recipe",
        )
        .with_optional_spell_cost_reduction(crate::effect::Value::Fixed(2));
        let provider_identity = policy.instance_id();
        let provider = CardDefinitionBuilder::new(CardId::new(), "Provider")
            .card_types(vec![CardType::Enchantment])
            .with_ability(Ability::static_ability(policy))
            .build();
        let provider = game.create_object_from_definition(&provider, caster, Zone::Battlefield);
        let recipient = CardDefinitionBuilder::new(CardId::new(), "Recipient")
            .card_types(vec![CardType::Enchantment])
            .mana_cost(crate::mana::ManaCost::new().add_generic(2))
            .build();
        let spell = game.create_object_from_definition(&recipient, caster, Zone::Hand);
        let before = native_definition_snapshots(&game);
        let committed_before = committed_static_allocations_on_thread();
        let mut anticipated_references = None;
        for _ in 0..6 {
            assert!(with_cast_preview(
                &game,
                game.object(spell).unwrap(),
                caster,
                |projected, candidate| {
                    assert_eq!(candidate.optional_costs.len(), 1);
                    let references = candidate
                        .optional_costs
                        .iter()
                        .map(|cost| cost.reference.clone())
                        .collect::<Vec<_>>();
                    if let Some(expected) = &anticipated_references {
                        assert_eq!(expected, &references);
                    } else {
                        anticipated_references = Some(references);
                    }
                    assert!(with_cast_preview(
                        projected,
                        candidate,
                        caster,
                        |_, nested| {
                            assert_eq!(nested.optional_costs.len(), candidate.optional_costs.len());
                            true
                        }
                    ));
                    assert!(
                        candidate
                            .temporary_static_ability_grants
                            .iter()
                            .filter_map(|grant| grant.ability_payload.as_ref())
                            .all(|ability| ability.instance_id().is_preview())
                    );
                    true
                }
            ));
            assert_eq!(before, native_definition_snapshots(&game));
        }
        assert_eq!(committed_static_allocations_on_thread(), committed_before);
        assert_eq!(
            game.object(provider)
                .unwrap()
                .abilities
                .iter()
                .find_map(|ability| match &ability.kind {
                    crate::ability::AbilityKind::Static(ability) => Some(ability.instance_id()),
                    _ => None,
                }),
            Some(provider_identity)
        );
        game.object_mut(spell).unwrap().zone = Zone::Stack;
        acquire_for_cast(&mut game, spell, caster);
        assert!(ensure_optional_costs(&mut game, spell));
        let committed = game.object(spell).unwrap();
        assert_eq!(
            committed
                .optional_costs
                .iter()
                .map(|cost| cost.reference.clone())
                .collect::<Vec<_>>(),
            anticipated_references.unwrap()
        );
        assert!(
            committed
                .temporary_static_ability_grants
                .iter()
                .filter_map(|grant| grant.ability_payload.as_ref())
                .all(|ability| !ability.instance_id().is_preview())
        );
        assert!(committed_static_allocations_on_thread() > committed_before);
    }
}
