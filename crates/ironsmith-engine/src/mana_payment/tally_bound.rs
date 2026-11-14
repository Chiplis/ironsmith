//! A negative-only quantity and color bound for mana payments.
//!
//! The bound overestimates everything the payer could make while paying: every
//! untapped source contributes the largest output any of its mana abilities
//! can make, each unit counted as any symbol that source could produce, and
//! abilities are assumed affordable. A cost that cannot be matched against
//! that supply cannot be paid. It answers in microseconds where proving the
//! same thing by search means simulating every activation order.
//!
//! The bound is sound only while nothing can raise production mid-payment, so
//! it declines (answers "maybe") whenever any of these could happen:
//! triggered mana abilities (CR 605.1b), mana or tap replacement effects, mana
//! abilities that are repeatable (no tap, sacrifice or exhaust cost), outputs
//! of unknown or variable size, side effects other than a short reviewed list,
//! mana abilities outside the battlefield or usable by other players, spending
//! permissions, and continuous effects that could change with a tapped
//! permanent or, when a counted ability has a non-tap cost or a side effect,
//! with any state change at all. Non-mana triggered abilities wait for the
//! stack (CR 605.3b), so they cannot help pay.
use crate::ability::{AbilityKind, ActivatedAbilityRuntimeExt as _};
use crate::derived_view::DerivedGameView;
use crate::effect::Value;
use crate::game_state::GameState;
use crate::mana::ManaSymbol;
use crate::zone::Zone;

use super::ManaPaymentRequest;
use super::program::ManaProduction;

const W: u8 = 1;
const U: u8 = 2;
const B: u8 = 4;
const R: u8 = 8;
const G: u8 = 16;
const C: u8 = 32;
const ANY_COLOR: u8 = W | U | B | R | G;
const ANY: u8 = ANY_COLOR | C;

fn symbol_mask(symbol: ManaSymbol) -> Option<u8> {
    Some(match symbol {
        ManaSymbol::White => W,
        ManaSymbol::Blue => U,
        ManaSymbol::Black => B,
        ManaSymbol::Red => R,
        ManaSymbol::Green => G,
        ManaSymbol::Colorless => C,
        _ => return None,
    })
}

fn color_mask(color: crate::color::Color) -> u8 {
    symbol_mask(ManaSymbol::from_color(color)).unwrap_or(ANY_COLOR)
}

fn fixed_amount(value: &Value) -> Option<u32> {
    match value {
        Value::Fixed(n) => Some((*n).max(0) as u32),
        _ => None,
    }
}

/// Units one activation can add, each as the set of symbols it could be.
fn production_units(production: ManaProduction<'_>, units: &mut Vec<u8>) -> Option<()> {
    match production {
        ManaProduction::Fixed { symbols, .. } => {
            for &symbol in symbols {
                units.push(symbol_mask(symbol)?);
            }
        }
        ManaProduction::Repeated { symbols, amount, .. } => {
            for _ in 0..fixed_amount(amount)? {
                for &symbol in symbols {
                    units.push(symbol_mask(symbol)?);
                }
            }
        }
        ManaProduction::ChooseColors { amount, available, .. } => {
            let mask = available.iter().fold(0u8, |mask, &color| mask | color_mask(color));
            for _ in 0..fixed_amount(amount)? {
                units.push(mask);
            }
        }
        ManaProduction::ChosenColor { amount, .. }
        | ManaProduction::CommanderIdentity { amount, .. }
        | ManaProduction::DraftChosenColors { amount, .. }
        | ManaProduction::DeckChosenColors { amount, .. } => {
            for _ in 0..fixed_amount(amount)? {
                units.push(ANY_COLOR);
            }
        }
        _ => return None,
    }
    Some(())
}

/// Side effects of a mana ability that cannot make or untap a mana source,
/// grant an ability, or put a card where a mana ability could use it. Any
/// state they change is covered by the stable-characteristics requirement.
fn is_reviewed_side_effect(effect: &crate::effect::Effect) -> bool {
    effect.downcast_ref::<crate::effects::ExileEffect>().is_some()
        || effect.downcast_ref::<crate::effects::LoseLifeEffect>().is_some()
        || effect.downcast_ref::<crate::effects::DealDamageEffect>().is_some()
}

/// Max matching of pips (acceptable-symbol masks) against supply units.
fn max_matching(pips: &[u8], units: &[u8]) -> usize {
    fn augment(
        pip: usize,
        pips: &[u8],
        units: &[u8],
        seen: &mut [bool],
        owner: &mut [Option<usize>],
    ) -> bool {
        for unit in 0..units.len() {
            if seen[unit] || pips[pip] & units[unit] == 0 {
                continue;
            }
            seen[unit] = true;
            if owner[unit].is_none_or(|other| augment(other, pips, units, seen, owner)) {
                owner[unit] = Some(pip);
                return true;
            }
        }
        false
    }
    let mut owner = vec![None; units.len()];
    let mut matched = 0;
    // Most constrained pips first keeps the augmenting paths short.
    let mut order: Vec<usize> = (0..pips.len()).collect();
    order.sort_by_key(|&pip| pips[pip].count_ones());
    for pip in order {
        let mut seen = vec![false; units.len()];
        if augment(pip, pips, units, &mut seen, &mut owner) {
            matched += 1;
        }
    }
    matched
}

/// Minimum demand of the cost as acceptable-symbol masks, one per mana unit.
/// `None` when a symbol is outside this model.
fn demand(game: &GameState, request: &ManaPaymentRequest) -> Option<Vec<u8>> {
    let mut demand = Vec::new();
    for pip in super::planner::mana_payment_expanded_pips(game, request) {
        if pip.is_empty() {
            continue;
        }
        let mut mask = 0u8;
        let mut units = u32::MAX;
        let mut free = false;
        for &symbol in &pip {
            match symbol {
                // A life alternative may make the pip free; demand nothing.
                ManaSymbol::Life(_) => free = true,
                ManaSymbol::Generic(amount) => {
                    mask = ANY;
                    units = units.min(u32::from(amount));
                }
                // Snow needs a snow source; any unit overestimates supply.
                ManaSymbol::Snow => {
                    mask = ANY;
                    units = units.min(1);
                }
                ManaSymbol::X => return None,
                other => {
                    mask |= symbol_mask(other)?;
                    units = units.min(1);
                }
            }
        }
        if free || units == 0 {
            continue;
        }
        if units == u32::MAX {
            return None;
        }
        for _ in 0..units {
            demand.push(mask);
        }
    }
    Some(demand)
}

fn debug_enabled() -> bool {
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ENABLED.get_or_init(|| std::env::var_os("IRONSMITH_TALLY_DEBUG").is_some())
}

fn bail<T>(reason: &str) -> Option<T> {
    if debug_enabled() {
        eprintln!("TALLY bail {reason}");
    }
    None
}

pub(super) fn rules_out_payment(
    game: &GameState,
    request: &ManaPaymentRequest,
    view: &DerivedGameView<'_>,
) -> bool {
    rules_out_payment_inner(game, request, view).unwrap_or(false)
}

fn rules_out_payment_inner(
    game: &GameState,
    request: &ManaPaymentRequest,
    view: &DerivedGameView<'_>,
) -> Option<bool> {
    if request.spend_policy != crate::player::ManaSpendPolicy::default()
        || request.allow_black_life
        || !request.cost.spending_restrictions().is_empty()
    {
        return bail("site1");
    }
    let Some(player) = game.player(request.payer) else { return bail("player"); };
    if !player.restricted_mana.is_empty() {
        return bail("site2");
    }
    let Some(demand) = demand(game, request) else { return bail("demand"); };
    if demand.is_empty() {
        return Some(false);
    }
    let pool = &player.mana_pool;
    let mut units = Vec::new();
    for (mask, amount) in [
        (W, pool.white),
        (U, pool.blue),
        (B, pool.black),
        (R, pool.red),
        (G, pool.green),
        (C, pool.colorless),
    ] {
        for _ in 0..amount {
            units.push(mask);
        }
    }
    if units.len() >= demand.len() && max_matching(&demand, &units) == demand.len() {
        return Some(false);
    }
    if request.allow_mana_abilities {
        let effects = &game.effect_store;
        let checks: [(&str, &dyn Fn() -> bool); 12] = [
            ("cost-modifiers", &|| super::color_reachability::has_relevant_cost_modifiers(game, view)),
            ("mana-replacements", &|| super::sources::has_mana_modifying_replacements(game)),
            ("mana-triggers", &|| super::sources::has_potential_mana_triggers(game, view)),
            ("event-replacements", &|| super::sources::has_replacements_for_events(
                game,
                &[
                    crate::events::EventKind::BecomeTapped,
                    crate::events::EventKind::ManaAdded,
                    crate::events::EventKind::AbilityActivated,
                ],
            )),
            ("activation-alternatives", &|| super::sources::has_live_activation_alternatives(game, None)),
            ("uncontrolled-lands", &|| game.any_uncontrolled_land_mana_permission()),
            ("spend-permissions", &|| !effects.mana_spend_effects.permissions.is_empty()),
            ("granted-mana", &|| !effects.granted_mana_abilities.is_empty()),
            ("reflexive", &|| !effects.pending_reflexive_triggers.is_empty()),
            ("delayed-mana", &|| effects.delayed_triggers.iter().any(|trigger| {
                trigger.effects.all_effects().iter().any(|effect| effect.contains_mana_production())
            })),
            ("pending-mana", &|| effects.pending_trigger_entries.iter().any(|entry| {
                entry.ability.effects.all_effects().iter().any(|effect| effect.contains_mana_production())
            })),
            ("tap-sensitive", &|| game.continuous_effects_are_tap_sensitive()),
        ];
        for (reason, check) in checks {
            if check() {
                return bail(reason);
            }
        }
        let mut stateful = false;
        for object in game.objects_map().values() {
            let Some(abilities) = view.abilities_rc(object.id) else { return bail("abilities"); };
            let controller = game.controller_of(object);
            let mut tap_group: Vec<u8> = Vec::new();
            let mut once_units: Vec<u8> = Vec::new();
            let mut has_tap_ability = false;
            for ability in abilities.iter().filter(|ability| ability.functions_in(&object.zone)) {
                let AbilityKind::Activated(activated) = &ability.kind else {
                    continue;
                };
                if !activated.is_runtime_mana_ability(game, object.id, controller) {
                    continue;
                }
                if object.zone != Zone::Battlefield {
                    return bail("site4");
                }
                if controller != request.payer {
                    if activated.allows_any_player_to_activate()
                        || game.may_tap_uncontrolled_land_for_mana(request.payer, object.id, activated)
                    {
                        return bail("site5");
                    }
                    continue;
                }
                let Some(costs) = activated.mana_cost.as_all() else { return bail("costs"); };
                let taps = costs.iter().any(|cost| cost.requires_tap());
                let once = taps
                    || activated.is_exhaust_ability()
                    || costs.iter().any(|cost| cost.is_sacrifice_self());
                if !once
                    || activated
                        .effects
                        .segments
                        .iter()
                        .any(|segment| !segment.self_replacements.is_empty())
                {
                    return bail("site6");
                }
                if !activated.choices.is_empty()
                    || costs.iter().any(|cost| !cost.requires_tap() && !cost.is_mana_cost())
                {
                    stateful = true;
                }
                let mut produced = Vec::new();
                if let Some(output) = &activated.mana_output {
                    for &symbol in output {
                        let Some(mask) = symbol_mask(symbol) else { return bail("output"); }; produced.push(mask);
                    }
                }
                for effect in activated.effects.iter() {
                    if effect.downcast_ref::<crate::effects::ChooseColorEffect>().is_some() {
                        continue;
                    }
                    if let Some(production) = effect.mana_production() {
                        if production_units(production, &mut produced).is_none() { return bail("production"); }
                    } else if is_reviewed_side_effect(effect) {
                        stateful = true;
                    } else {
                        return bail("site7");
                    }
                }
                if taps {
                    has_tap_ability = true;
                    // Tap abilities of one source exclude each other: keep the
                    // largest count, each unit any symbol one of them makes.
                    let union = produced.iter().fold(0u8, |mask, unit| mask | unit);
                    let union = tap_group.iter().fold(union, |mask, unit| mask | unit);
                    let count = tap_group.len().max(produced.len());
                    tap_group = vec![union; count];
                } else {
                    once_units.extend(produced);
                }
            }
            if has_tap_ability {
                let unusable = game.is_tapped(object.id)
                    || request.reserved_tap_sources.contains(&object.id)
                    || (game.is_summoning_sick(object.id)
                        && view.object_has_card_type(object.id, crate::types::CardType::Creature)
                        && !view.object_has_haste_for_activation(object.id));
                if !unusable {
                    units.extend(tap_group);
                }
            }
            units.extend(once_units);
        }
        if stateful && !super::color_reachability::stable_characteristics(game, view) {
            return bail("site8");
        }
    }
    let vetoed = units.len() < demand.len() || max_matching(&demand, &units) < demand.len();
    if debug_enabled() {
        eprintln!("TALLY result vetoed={vetoed} demand={demand:?} units={units:?}");
    }
    Some(vetoed)
}
