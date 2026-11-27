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
        // "One mana of any color among ..." (Mox Amber) and the imprinted
        // card's colors (Chrome Mox): at most one unit, of some color.
        ManaProduction::ColorsAmong { choose_one: true, .. } | ManaProduction::ImprintedColors => {
            units.push(ANY);
        }
        // One mana of each such type: at most one unit per type.
        ManaProduction::ColorsAmong { choose_one: false, .. } => {
            units.extend([ANY; 6]);
        }
        _ => return None,
    }
    Some(())
}

/// Whether paying for any of `payer`'s mana abilities puts a counter on
/// something, which could later pay a counter-removal cost.
fn a_mana_ability_cost_puts_counters(
    game: &GameState,
    view: &DerivedGameView<'_>,
    payer: crate::ids::PlayerId,
) -> bool {
    game.objects_map().values().any(|object| {
        game.controller_of(object) == payer
            && view.abilities_rc(object.id).is_none_or(|abilities| {
                abilities.iter().any(|ability| {
                    let AbilityKind::Activated(activated) = &ability.kind else {
                        return false;
                    };
                    activated.is_runtime_mana_ability(game, object.id, payer)
                        && activated.mana_cost.as_all().is_none_or(|costs| {
                            costs.iter().any(|cost| {
                                cost.effect_ref().is_some_and(|effect| {
                                    effect.downcast_ref::<crate::effects::PutCountersEffect>().is_some()
                                })
                            })
                        })
                })
            })
    })
}

/// Whether paying this ability's cost exiles the card it is printed on.
fn costs_exile_their_source(activated: &crate::ability::ActivatedAbility) -> bool {
    activated.mana_cost.as_all().is_some_and(|costs| {
        costs.iter().any(|cost| {
            cost.effect_ref().is_some_and(|effect| {
                effect
                    .downcast_ref::<crate::effects::ExileEffect>()
                    .is_some_and(|exile| matches!(exile.spec.base(), crate::target::ChooseSpec::Source))
            })
        })
    })
}

/// Side effects of a mana ability that cannot make or untap a mana source,
/// grant an ability, or put a card where a mana ability could use it. Any
/// state they change is covered by the stable-characteristics requirement.
fn is_reviewed_side_effect(effect: &crate::effect::Effect) -> bool {
    effect.downcast_ref::<crate::effects::ExileEffect>().is_some()
        || effect.downcast_ref::<crate::effects::LoseLifeEffect>().is_some()
        || effect.downcast_ref::<crate::effects::DealDamageEffect>().is_some()
}

/// Life loss, or damage, to the player activating the ability and to nothing
/// else.
fn changes_only_its_controllers_life(effect: &crate::effect::Effect) -> bool {
    use crate::target::{ChooseSpec, PlayerFilter};
    if let Some(lose) = effect.downcast_ref::<crate::effects::LoseLifeEffect>() {
        return matches!(
            lose.player.base(),
            ChooseSpec::SourceController | ChooseSpec::Player(PlayerFilter::You)
        );
    }
    effect
        .downcast_ref::<crate::effects::DealDamageEffect>()
        .is_some_and(|damage| {
            matches!(
                damage.target.base(),
                ChooseSpec::SourceController | ChooseSpec::Player(PlayerFilter::You)
            ) && damage.excess_to_controller.is_none()
        })
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
        // A free compatible unit ends the path at once; only otherwise search
        // for an augmenting path. This keeps large interchangeable pools
        // linear per pip instead of re-walking every owned unit.
        if let Some(unit) = (0..units.len())
            .find(|&unit| owner[unit].is_none() && !seen[unit] && pips[pip] & units[unit] != 0)
        {
            seen[unit] = true;
            owner[unit] = Some(pip);
            return true;
        }
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

/// Mana an ability certainly takes to activate: the units of its plain
/// generic, colored and colorless pips. A pip that could be paid another way
/// or whose size is not fixed counts for nothing, which only overestimates
/// what the ability nets.
fn certain_activation_mana(costs: &[crate::costs::Cost]) -> usize {
    costs
        .iter()
        .filter_map(|cost| cost.mana_cost_ref())
        .flat_map(|cost| cost.pips().iter())
        .map(|pip| match pip.as_slice() {
            [ManaSymbol::Generic(amount)] => usize::from(*amount),
            [symbol] if symbol_mask(*symbol).is_some() => 1,
            _ => 0,
        })
        .sum()
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

thread_local! {
    static LAST_BAIL: std::cell::Cell<&'static str> = const { std::cell::Cell::new("") };
}

thread_local! {
    static LAST_COUNTED: std::cell::RefCell<String> = const { std::cell::RefCell::new(String::new()) };
}

/// Diagnostics: the sources the bound last counted, with their unit counts.
pub(super) fn last_counted() -> String {
    LAST_COUNTED.with(|last| last.borrow().clone())
}

/// Diagnostics: why the bound last declined on this thread.
pub(super) fn last_bail() -> &'static str {
    LAST_BAIL.with(std::cell::Cell::get)
}

fn bail<T>(reason: &'static str) -> Option<T> {
    crate::perf::tier("tally.bail", reason);
    LAST_BAIL.with(|last| last.set(reason));
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
    LAST_BAIL.with(|last| last.set(""));
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
        // A matching uses at most one unit per demanded unit, so pool units
        // past the demand are inert; never materialize a saturated pool.
        let needed = usize::try_from(amount).unwrap_or(usize::MAX).min(demand.len());
        units.extend(std::iter::repeat_n(mask, needed));
    }
    if units.len() >= demand.len() && max_matching(&demand, &units) == demand.len() {
        return Some(false);
    }
    // Units counted below that their own abilities' mana costs take back.
    let mut activation_mana = 0usize;
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
            ("tap-sensitive", &|| game.mana_activation_may_change_derived_state()),
        ];
        for (reason, check) in checks {
            if check() {
                return bail(reason);
            }
        }
        // What a counted activation changes besides tapped state and mana:
        // only its controller's life total (a life payment, or life loss and
        // damage dealt to that player), or something else.
        let mut stateful = false;
        let mut changes_only_life = true;
        let mut counted: Vec<String> = Vec::new();
        let mut skipped_object_count_condition = false;
        let mut skipped_other_condition = false;
        for object in game.objects_map().values() {
            let Some(abilities) = view.abilities_rc(object.id) else { return bail("abilities"); };
            let controller = game.controller_of(object);
            let mut tap_group: Vec<u8> = Vec::new();
            // The most any one tap ability of this source nets after its own
            // mana cost.
            let mut tap_net = 0usize;
            let mut once_units: Vec<u8> = Vec::new();
            let mut once_activation_mana = 0usize;
            let mut has_tap_ability = false;
            // A tapped, reserved or summoning-sick source cannot pay {T}, and
            // nothing counted here untaps it: its tap abilities add nothing
            // and change nothing.
            let cannot_tap = object.zone == Zone::Battlefield
                && (game.is_tapped(object.id)
                    || request.reserved_tap_sources.contains(&object.id)
                    || (game.is_summoning_sick(object.id)
                        && view.object_has_card_type(object.id, crate::types::CardType::Creature)
                        && !view.object_has_haste_for_activation(object.id)));
            for ability in abilities.iter().filter(|ability| ability.functions_in(&object.zone)) {
                let AbilityKind::Activated(activated) = &ability.kind else {
                    continue;
                };
                if !activated.is_runtime_mana_ability(game, object.id, controller) {
                    continue;
                }
                // Never offered during this payment, whatever else happens:
                // a source the request excludes, and an ability usable only
                // when an instant could be cast (Lion's Eye Diamond), which
                // paying a cost never is.
                if controller == request.payer
                    && (request.preferences.excluded_sources.contains(&object.id)
                        || request.activation_excluded_sources.contains(&object.id)
                        || crate::special_actions::activation_restricted_to_instant_timing(activated))
                {
                    continue;
                }
                // A card that pays for its own mana ability by exiling itself
                // from its owner's hand (Elvish Spirit Guide) is one more
                // activation by that player, counted below. Any other mana
                // ability outside the battlefield is not modelled.
                let exiles_itself = costs_exile_their_source(activated);
                if object.zone != Zone::Battlefield
                    && !(object.zone == Zone::Hand && object.owner == controller && exiles_itself)
                {
                    crate::perf::tier_note("tally off-battlefield", || format!("{} [{:?}]", object.name, object.zone));
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
                // A condition that is false now (Mox Opal without metalcraft)
                // and reads nothing a mana activation writes stays false.
                // A sacrifice moves an object and a life payment changes a
                // life total, so which other sources are present decides
                // whether the skip stands (checked below).
                if controller == request.payer
                    && let Some(condition) = &activated.activation_condition
                    && !GameState::condition_can_change_with_mana_activations(condition)
                    && let Some(index) = abilities.iter().position(|candidate| std::ptr::eq(candidate, ability))
                    && !crate::special_actions::check_mana_ability_condition(
                        game, controller, object.id, index, condition,
                    )
                {
                    if GameState::condition_only_counts_objects(condition) {
                        skipped_object_count_condition = true;
                    } else {
                        skipped_other_condition = true;
                    }
                    continue;
                }
                let Some(costs) = activated.mana_cost.as_all() else { return bail("costs"); };
                // A counter this cost removes that is not there now (Hickory
                // Woodlot out of depletion counters) will not be there later
                // in the payment either: no counted activation puts one.
                if costs.iter().any(|cost| {
                    cost.is_remove_counters() && {
                        let mut chooser = crate::decision::SelectFirstDecisionMaker;
                        cost.can_pay(
                            game,
                            &crate::costs::CostContext::new(object.id, controller, &mut chooser),
                        )
                        .is_err()
                    }
                }) && !a_mana_ability_cost_puts_counters(game, view, request.payer)
                {
                    continue;
                }
                let taps = costs.iter().any(|cost| cost.requires_tap());
                if taps && cannot_tap && controller == request.payer {
                    continue;
                }
                // "Activate only once each turn" (Shire Scarecrow) also caps
                // the ability at one activation during this payment; counting
                // one even if it was already used this turn overestimates.
                let once = taps
                    || activated.is_exhaust_ability()
                    || activated.max_activations_per_turn().is_some_and(|cap| cap <= 1)
                    || costs.iter().any(|cost| cost.is_sacrifice_self())
                    || exiles_itself;
                if !once
                    || activated
                        .effects
                        .segments
                        .iter()
                        .any(|segment| !segment.self_replacements.is_empty())
                {
                    crate::perf::tier_note("tally repeatable", || object.name.to_string());
                    return bail("site6");
                }
                if !activated.choices.is_empty()
                    || costs.iter().any(|cost| {
                        !cost.requires_tap() && !cost.is_mana_cost() && !cost.is_life_cost()
                    })
                {
                    changes_only_life = false;
                }
                if !activated.choices.is_empty()
                    || costs.iter().any(|cost| !cost.requires_tap() && !cost.is_mana_cost())
                {
                    crate::perf::tier_note("tally stateful source", || format!(
                        "{}: choices={} costs=[{}]",
                        object.name,
                        activated.choices.len(),
                        costs.iter().map(|c| c.display()).collect::<Vec<_>>().join(","),
                    ));
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
                        // "Any color among ..." and the imprinted card's
                        // colors are read from the board as it stands.
                        // Nothing enters or changes color while mana
                        // abilities are activated, so the current set is
                        // the most the ability can offer; an empty set (Mox
                        // Amber with no legend, Chrome Mox with no imprint)
                        // makes no mana at all.
                        let read_now = match production {
                            ManaProduction::ColorsAmong { filter, choose_one, .. }
                                if !GameState::filter_reads_tapped_state_or_activation_history(filter, true) =>
                            {
                                Some(choose_one)
                            }
                            ManaProduction::ImprintedColors => Some(true),
                            _ => None,
                        };
                        if let Some(choose_one) = read_now {
                            let masks: Vec<u8> = effect
                                .producible_mana_symbols(game, object.id, controller)
                                .unwrap_or_default()
                                .into_iter()
                                .map(|symbol| symbol_mask(symbol).unwrap_or(ANY))
                                .collect();
                            if choose_one {
                                let union = masks.iter().fold(0u8, |mask, unit| mask | unit);
                                if union != 0 {
                                    produced.push(union);
                                }
                            } else {
                                produced.extend(masks);
                            }
                        } else if production_units(production, &mut produced).is_none() {
                            crate::perf::tier_note("tally production", || object.name.to_string());
                            return bail("production");
                        }
                    } else if is_reviewed_side_effect(effect) {
                        crate::perf::tier_note("tally stateful source", || format!("{}: side effect", object.name));
                        stateful = true;
                        changes_only_life &= changes_only_its_controllers_life(effect);
                    } else {
                        crate::perf::tier_note("tally side-effect", || object.name.to_string());
                        return bail("site7");
                    }
                }
                let spent = certain_activation_mana(costs).min(produced.len());
                if taps {
                    has_tap_ability = true;
                    tap_net = tap_net.max(produced.len() - spent);
                    // Tap abilities of one source exclude each other: keep the
                    // largest count, each unit any symbol one of them makes.
                    let union = produced.iter().fold(0u8, |mask, unit| mask | unit);
                    let union = tap_group.iter().fold(union, |mask, unit| mask | unit);
                    let count = tap_group.len().max(produced.len());
                    tap_group = vec![union; count];
                } else {
                    once_activation_mana += spent;
                    once_units.extend(produced);
                }
            }
            if has_tap_ability {
                if !cannot_tap {
                    activation_mana += tap_group.len() - tap_net;
                    if crate::perf::tiers_enabled() {
                        counted.push(format!("{}:{}", object.name, tap_group.len()));
                    }
                    units.extend(tap_group);
                }
            }
            activation_mana += once_activation_mana;
            if crate::perf::tiers_enabled() && !once_units.is_empty() {
                counted.push(format!("{}:{}(once)", object.name, once_units.len()));
            }
            units.extend(once_units);
        }
        // Life is the one such change whose readers are known exactly: a
        // derivation that reads a life total, or this turn's life and damage
        // records, records the read. With every object derived and no such
        // read, no characteristic, restriction or cost can differ at a lower
        // life total. A replacement could turn the life loss or damage into
        // something else, so none may apply to those events.
        let life_cannot_matter = || {
            changes_only_life
                && game.derive_every_object_once()
                && !game.derived_state_read_life()
                && !super::sources::has_replacements_for_events(
                    game,
                    &[
                        crate::events::EventKind::Damage,
                        crate::events::EventKind::LifeLoss,
                    ],
                )
        };
        if crate::perf::tiers_enabled() {
            LAST_COUNTED.with(|last| *last.borrow_mut() = counted.join(", "));
        }
        // A life total cannot change how many objects a filter matches.
        if stateful
            && (skipped_other_condition || (skipped_object_count_condition && !changes_only_life))
        {
            return bail("unmet-condition");
        }
        if stateful
            && !super::color_reachability::stable_characteristics(game, view)
            && !life_cannot_matter()
        {
            crate::perf::tier_note("tally stateful", || format!(
                "only-life={} life-read={} clean={}",
                changes_only_life,
                game.derived_state_read_life(),
                game.continuous_state_is_clean(),
            ));
            return bail("site8");
        }
    }
    // Whatever is activated, the mana left for this cost is what was made less
    // what the activations themselves took: at most each source's best net.
    let vetoed = units.len() - activation_mana.min(units.len()) < demand.len()
        || max_matching(&demand, &units) < demand.len();
    if debug_enabled() {
        eprintln!("TALLY result vetoed={vetoed} demand={demand:?} units={units:?}");
    }
    Some(vetoed)
}
