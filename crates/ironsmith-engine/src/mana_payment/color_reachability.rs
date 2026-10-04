//! A negative-only color proof for pure mana closures. Quantities, activation
//! costs, recipients and trigger conditions are deliberately overestimated.
//! Unknown mutations or dependencies retain the complete payment search.
use crate::{ability::{AbilityKind, ActivatedAbilityRuntimeExt as _},
    color::Color, continuous::{EffectTarget, Modification}, derived_view::DerivedGameView,
    effect::{Until, Value}, game_state::GameState, mana::ManaSymbol, target::PlayerFilter,
    zone::Zone};
use super::{ManaPaymentRequest, program::ManaProduction};

const COLORS: [Color; 5] = [Color::White, Color::Blue, Color::Black, Color::Red, Color::Green];
fn add(symbol: ManaSymbol, colors: &mut Vec<ManaSymbol>) -> Option<()> {
    if !matches!(symbol, ManaSymbol::White | ManaSymbol::Blue | ManaSymbol::Black |
        ManaSymbol::Red | ManaSymbol::Green | ManaSymbol::Colorless) { return None; }
    if !colors.contains(&symbol) { colors.push(symbol); }
    Some(())
}
fn production(game: &GameState, controller: crate::ids::PlayerId,
    value: ManaProduction<'_>, triggered: bool, colors: &mut Vec<ManaSymbol>) -> Option<()> {
    match value {
        ManaProduction::Fixed { symbols, .. } => {
            for &symbol in symbols { add(symbol, colors)?; }
        }
        ManaProduction::Repeated { symbols, amount: Value::Fixed(n), .. } => {
            if *n > 0 { for &symbol in symbols { add(symbol, colors)?; } }
        }
        ManaProduction::ChooseColors { available, amount: Value::Fixed(n), .. } => {
            if *n > 0 { for &color in available { add(ManaSymbol::from_color(color), colors)?; } }
        }
        ManaProduction::ChosenColor { amount, fixed_option, .. } => {
            // The fixed option is an *alternative* to the stored chosen color.
            // Quantity still reads that stored choice, not the output option.
            for color in COLORS {
                let positive = match amount {
                    Value::Fixed(n) => *n > 0,
                    Value::DevotionToChosenColor(PlayerFilter::You) => game.devotion_to_color(controller, color) > 0,
                    Value::DevotionToChosenColor(PlayerFilter::Specific(player)) => game.devotion_to_color(*player, color) > 0,
                    _ => return None,
                };
                if positive {
                    add(ManaSymbol::from_color(color), colors)?;
                    if let Some(fixed) = fixed_option { add(ManaSymbol::from_color(fixed), colors)?; }
                }
            }
        }
        // This instruction copies only colors already present in the triggering
        // event. Treating its quantity and recipients as unlimited is an upper bound.
        ManaProduction::LandProducedTypes { amount: Value::Fixed(_),
            source: crate::effects::ManaTypeSource::TriggeringEventProduced, .. } if triggered => {}
        _ => return None,
    }
    Some(())
}

fn stable_characteristics(game: &GameState, view: &DerivedGameView<'_>) -> bool {
    // Explicit object targets and fixed scalar modifications cannot change from
    // paying mana, tapping, or recording a chosen color. Unknown descriptors are
    // intentionally excluded, including duration predicates and copy/text roots.
    if game.effect_store.continuous_effects.effects().iter().any(|effect| {
        effect.condition.is_some()
            || !matches!(effect.applies_to, EffectTarget::Specific(_) | EffectTarget::Source)
            || !matches!(effect.duration, Until::Forever | Until::EndOfTurn)
            || !match &effect.modification {
                Modification::AddCardTypes(_) | Modification::RemoveCardTypes(_)
                | Modification::SetCardTypes(_) | Modification::AddSubtypes(_)
                | Modification::RemoveSubtypes(_) | Modification::SetSubtypes(_) => true,
                Modification::SetPowerToughness { power: Value::Fixed(_), toughness: Value::Fixed(_), .. } => true,
                Modification::AddAbility(ability) => ability.id() == crate::static_abilities::StaticAbilityId::Haste,
                _ => false,
            }
    }) { return false; }
    game.objects_map().values().all(|object| {
        let Some(abilities) = view.abilities_rc(object.id) else { return false; };
        object.abilities.iter().chain(abilities.iter()).all(|ability| {
            !ability.functions_in(&object.zone) || !matches!(&ability.kind,
                AbilityKind::Static(ability) if ability.may_generate_continuous_effects())
        })
    })
}

// A reduction bound to an inactive/non-mana ability of its own source cannot
// change this mana closure. Keep all unbound, cross-source and potentially
// mana-relevant modifiers conservative; increases may add state-changing costs.
fn has_relevant_cost_modifiers(game: &GameState, view: &DerivedGameView<'_>) -> bool {
    use crate::static_abilities::ActivatedAbilityCostCondition;
    view.activated_ability_cost_modifier_sources().into_iter().any(|source| {
        let Some(object) = game.object(source) else { return true; };
        view.static_abilities_rc(source).unwrap_or_default().iter().any(|ability| {
            if ability.activated_ability_cost_increase().is_some() { return true; }
            let Some(reduction) = ability.activated_ability_cost_reduction() else { return false; };
            if reduction.filter != crate::target::ObjectFilter::source() { return true; }
            let Some(ActivatedAbilityCostCondition::ThisAbility { ability_index: Some(index) })
                = &reduction.condition else { return true; };
            let Some(abilities) = view.abilities_rc(source) else { return true; };
            let Some(priced) = abilities.get(*index) else { return true; };
            priced.functions_in(&object.zone) && match &priced.kind {
                AbilityKind::Activated(activated) => activated.is_runtime_mana_ability(
                    game, source, game.controller_of(object)),
                _ => true,
            }
        })
    })
}

fn reachable_colors(game: &GameState, request: &ManaPaymentRequest,
    view: &DerivedGameView<'_>) -> Option<Vec<ManaSymbol>> {
    if has_relevant_cost_modifiers(game, view)
        || !stable_characteristics(game, view)
        || super::sources::has_mana_modifying_replacements(game)
        || !game.effect_store.mana_spend_effects.permissions.is_empty()
        || !game.effect_store.granted_mana_abilities.is_empty()
        || !game.effect_store.pending_reflexive_triggers.is_empty()
        || game.effect_store.delayed_triggers.iter().any(|trigger|
            trigger.effects.all_effects().iter().any(|effect| effect.contains_mana_production()))
        || game.effect_store.pending_trigger_entries.iter().any(|entry|
            entry.ability.effects.all_effects().iter().any(|effect| effect.contains_mana_production()))
    { return None; }
    let pool = &game.player(request.payer)?.mana_pool;
    let mut colors = [(ManaSymbol::White, pool.white), (ManaSymbol::Blue, pool.blue),
        (ManaSymbol::Black, pool.black), (ManaSymbol::Red, pool.red),
        (ManaSymbol::Green, pool.green), (ManaSymbol::Colorless, pool.colorless)]
        .into_iter().filter_map(|(symbol, amount)| (amount > 0).then_some(symbol)).collect::<Vec<_>>();
    if !request.allow_mana_abilities { return Some(colors); }
    // Scan every active ability, rather than only currently affordable ones:
    // another activation may fund an input cost. Other recipients and untapped
    // requirements are ignored to enlarge, never shrink, the reachable set.
    for object in game.objects_map().values() {
        let abilities = view.abilities_rc(object.id)?;
        for ability in abilities.iter().filter(|ability| ability.functions_in(&object.zone)) {
            match &ability.kind {
                AbilityKind::Activated(activated) if activated.is_runtime_mana_ability(
                    game, object.id, game.controller_of(object)) => {
                    if object.zone != Zone::Battlefield { return None; }
                    if game.controller_of(object) != request.payer {
                        if activated.allows_any_player_to_activate() { return None; }
                        continue;
                    }
                    let costs = activated.mana_cost.as_all()?;
                    if activated.is_exhaust_ability() || !activated.choices.is_empty()
                        || costs.iter().any(|cost| !cost.requires_tap() && !cost.is_mana_cost())
                        || activated.effects.segments.iter().any(|segment| !segment.self_replacements.is_empty())
                    { return None; }
                    if let Some(output) = &activated.mana_output {
                        for &symbol in output { add(symbol, &mut colors)?; }
                    }
                    for effect in activated.effects.iter() {
                        if effect.downcast_ref::<crate::effects::ChooseColorEffect>().is_some() { continue; }
                        production(game, game.controller_of(object), effect.mana_production()?, false, &mut colors)?;
                    }
                }
                AbilityKind::Triggered(trigger) if trigger.effects.all_effects().iter()
                    .any(|effect| effect.contains_mana_production()) => {
                    if !trigger.choices.is_empty() || trigger.effects.segments.iter()
                        .any(|segment| !segment.self_replacements.is_empty()) { return None; }
                    for effect in trigger.effects.iter() {
                        production(game, game.controller_of(object), effect.mana_production()?, true, &mut colors)?;
                    }
                }
                _ => {}
            }
        }
    }
    Some(colors)
}

pub(super) fn rules_out_payment(game: &GameState, request: &ManaPaymentRequest,
    view: &DerivedGameView<'_>) -> bool {
    if request.spend_policy != crate::player::ManaSpendPolicy::default() { return false; }
    let pips = super::planner::mana_payment_expanded_pips(game, request);
    let absent = |colors: &[ManaSymbol]| pips.iter().any(|pip| {
        !pip.is_empty() && pip.iter().all(|symbol| match symbol {
            ManaSymbol::White | ManaSymbol::Blue | ManaSymbol::Black | ManaSymbol::Red |
            ManaSymbol::Green | ManaSymbol::Colorless => !colors.contains(symbol),
            // Life, snow, generic and symbolic operands cannot be rejected by a
            // missing-color proof, regardless of their actual affordability.
            _ => false,
        })
    });
    let Some(player) = game.player(request.payer) else { return false; };
    let pool = &player.mana_pool;
    let existing = [(ManaSymbol::White, pool.white), (ManaSymbol::Blue, pool.blue),
        (ManaSymbol::Black, pool.black), (ManaSymbol::Red, pool.red),
        (ManaSymbol::Green, pool.green), (ManaSymbol::Colorless, pool.colorless)]
        .into_iter().filter_map(|(symbol, amount)| (amount > 0).then_some(symbol)).collect::<Vec<_>>();
    // Generic-only requests and already represented colors cannot benefit from
    // this proof. Avoid discovering the board for those common calls.
    if !absent(&existing) { return false; }
    reachable_colors(game, request, view).is_some_and(|colors| absent(&colors))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_scoped_inactive_cost_modifier_does_not_block_color_proof() {
        use crate::{ids::PlayerId, costs::PaymentReason, mana::ManaCost};
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let mut inactive = crate::Ability::mana(crate::TotalCost::free(), vec![ManaSymbol::Blue]);
        inactive.functional_zones = vec![Zone::Hand];
        let modifier = crate::Ability::static_ability(
            crate::static_abilities::StaticAbility::reduce_activated_ability_costs_if_targets(
                crate::target::ObjectFilter::source(), 1,
                crate::static_abilities::ActivatedAbilityCostCondition::ThisAbility {
                    ability_index: Some(1),
                }, None));
        let definition = crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Scoped source")
            .card_types(vec![crate::types::CardType::Land])
            .with_ability(crate::Ability::mana(crate::TotalCost::free(), vec![ManaSymbol::Green]))
            .with_ability(inactive).with_ability(modifier).build();
        let modifier_source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let source = modifier_source;
        game.refresh_continuous_state().unwrap();
        let request = ManaPaymentRequest::new(alice, source, PaymentReason::CastSpell,
            ManaCost::from_pips(vec![vec![ManaSymbol::White]]));
        let view = DerivedGameView::new(&game);
        assert!(view.has_activated_ability_cost_modifiers(), "reproduces old blanket guard");
        assert!(!has_relevant_cost_modifiers(&game, &view));
        assert!(rules_out_payment(&game, &request, &view));
        drop(view);

        // The same scoped modifier on a currently active mana ability must
        // retain fallback, as must a modifier without a source-local filter.
        let make_modifier = |index, filter| crate::Ability::static_ability(
            crate::static_abilities::StaticAbility::reduce_activated_ability_costs_if_targets(
                filter, 1, crate::static_abilities::ActivatedAbilityCostCondition::ThisAbility {
                    ability_index: Some(index),
                }, None));
        game.object_mut(modifier_source).unwrap().abilities_mut().push(
            make_modifier(0, crate::target::ObjectFilter::source()));
        game.refresh_continuous_state().unwrap();
        assert!(has_relevant_cost_modifiers(&game, &DerivedGameView::new(&game)));
        game.object_mut(modifier_source).unwrap().abilities_mut().pop();
        game.object_mut(modifier_source).unwrap().abilities_mut().push(
            make_modifier(1, crate::target::ObjectFilter::default()));
        game.refresh_continuous_state().unwrap();
        assert!(has_relevant_cost_modifiers(&game, &DerivedGameView::new(&game)));
        let positive = ManaPaymentRequest::new(alice, source, PaymentReason::CastSpell,
            ManaCost::from_pips(vec![vec![ManaSymbol::Green]]));
        assert!(!rules_out_payment(&game, &positive, &DerivedGameView::new(&game)));
    }
}
